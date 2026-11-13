//! River（`riverqueue` 0.6）側の試験（Issue #99 の AC1・AC2 の比較相手）。

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

use common::TestDb;
use riverqueue::migrate::PostgresMigrator;
use riverqueue::{
    BoxError, Client, EventKind, InsertOpts, Job, JobArgs, JobState, QueueConfig, WorkCancelled,
    WorkContext, WorkError, WorkOutcome, Worker, Workers,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

/// AC1 の引数。業務行と同じトランザクションで積む。
#[derive(Clone, Debug, Deserialize, JobArgs, Serialize)]
#[river(kind = "qcmp_tx_probe")]
struct TxProbe {
    state_row_id: i64,
}

/// AC2 の引数。毎回失敗して River の再実行と打ち切りを観察する。
#[derive(Clone, Debug, Deserialize, JobArgs, Serialize)]
#[river(kind = "qcmp_flaky")]
struct Flaky {}

/// AC2 の引数。外部から停止されるまで待つ。
#[derive(Clone, Debug, Deserialize, JobArgs, Serialize)]
#[river(kind = "qcmp_sleepy")]
struct Sleepy {}

fn skip() -> bool {
    if std::env::var("PELWAY_DATABASE_URL").is_ok() {
        return false;
    }
    eprintln!("skip: PELWAY_DATABASE_URL 未設定");
    true
}

/// River のスキーマを当てた scratch database を返す。
async fn river_db() -> TestDb {
    let db = TestDb::new().await.unwrap();
    PostgresMigrator::new(db.pool.clone())
        .migrate_up()
        .await
        .unwrap();
    db
}

async fn river_count(pool: &PgPool, kind: &str) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM river_job WHERE kind = $1")
        .bind(kind)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// river_job.state は enum 型（river_job_state）なので text にキャストして受け取る。
async fn river_state(pool: &PgPool, id: i64) -> String {
    sqlx::query_scalar("SELECT state::text FROM river_job WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn state_row_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM devq_state_row")
        .fetch_one(pool)
        .await
        .unwrap()
}

/// AC1: River の `insert().tx()` で業務行とジョブを同じトランザクションに載せられる。
/// rollback すれば両方消え、commit すれば両方残る（自作キューと同じ性質）。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn river_insert_shares_transaction_with_business_rows() {
    if skip() {
        return;
    }
    let db = river_db().await;
    db.apply(common::business_schema()).await.unwrap();
    // worker を登録しない挿入専用クライアント（キューの設定は不要）。
    let client = Client::builder(db.pool.clone())
        .workers(Workers::new())
        .build()
        .unwrap();

    // rollback すれば river_job 行も業務行も残らない。
    let mut tx = db.pool.begin().await.unwrap();
    let state_id: i64 =
        sqlx::query_scalar("INSERT INTO devq_state_row (note) VALUES ($1) RETURNING id")
            .bind("river v1")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    client
        .insert(TxProbe {
            state_row_id: state_id,
        })
        .tx(&mut tx)
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    assert_eq!(river_count(&db.pool, "qcmp_tx_probe").await, 0);
    assert_eq!(state_row_count(&db.pool).await, 0);

    // commit すれば両方残る。
    let mut tx = db.pool.begin().await.unwrap();
    let state_id: i64 =
        sqlx::query_scalar("INSERT INTO devq_state_row (note) VALUES ($1) RETURNING id")
            .bind("river v2")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    client
        .insert(TxProbe {
            state_row_id: state_id,
        })
        .tx(&mut tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(river_count(&db.pool, "qcmp_tx_probe").await, 1);
    assert_eq!(state_row_count(&db.pool).await, 1);
    db.dispose().await;
}

/// 毎回失敗する worker。再試行の間隔だけ 50ms に詰める。
struct FlakyWorker {
    runs: Arc<AtomicU32>,
}

impl Worker<Flaky> for FlakyWorker {
    type Error = BoxError;

    fn next_retry(
        &self,
        _job: &Job<Flaky>,
        _error: &WorkError,
        _now: riverqueue::jiff::Timestamp,
    ) -> Option<Duration> {
        Some(Duration::from_millis(50))
    }

    async fn work(
        &self,
        _context: WorkContext,
        _job: Job<Flaky>,
    ) -> Result<WorkOutcome, Self::Error> {
        self.runs.fetch_add(1, Ordering::SeqCst);
        Err("synthetic failure".into())
    }
}

/// AC2: River の再実行は `max_attempts` で打ち切られ、1 回目の失敗は再実行待ち、
/// 2 回目の失敗で discarded になる（自作キューの Requeued / Dead に相当）。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn river_retries_then_discards_at_max_attempts() {
    if skip() {
        return;
    }
    let db = river_db().await;
    let runs = Arc::new(AtomicU32::new(0));
    let mut workers = Workers::new();
    workers
        .add(FlakyWorker {
            runs: Arc::clone(&runs),
        })
        .unwrap();
    let client = Client::builder(db.pool.clone())
        .workers(workers)
        .queue("default", QueueConfig::new(1))
        .build()
        .unwrap();
    let mut events = client.subscribe(&[EventKind::JobFailed]).unwrap();
    let mut run = client.start().unwrap();
    let inserted = client
        .insert(Flaky {})
        .opts(InsertOpts::default().with_max_attempts(2))
        .await
        .unwrap();

    // 1 回目の失敗は再実行待ち（retryable または予約付きの available/scheduled）。
    let mut saw_retryable = false;
    let outcome = tokio::time::timeout(Duration::from_secs(10), async {
        while let Ok(event) = events.recv().await {
            let Some(job_event) = event.as_job() else {
                continue;
            };
            if job_event.job.id != inserted.id() {
                continue;
            }
            match job_event.job.state {
                JobState::Retryable | JobState::Available | JobState::Scheduled => {
                    saw_retryable = true;
                }
                JobState::Discarded => return true,
                _ => {}
            }
        }
        false
    })
    .await
    .expect("再試行の後に discarded が届く");
    assert!(outcome, "max_attempts=2 の 2 回目の失敗で discarded になる");
    assert!(saw_retryable, "1 回目の失敗は再実行待ちになる");
    assert_eq!(runs.load(Ordering::SeqCst), 2, "試行は 2 回だけ");
    assert_eq!(river_state(&db.pool, inserted.id()).await, "discarded");
    run.stop().await.unwrap();
    db.dispose().await;
}

/// 停止要求を待つ worker。完了前に副作用の印を残す。
struct SleepyWorker {
    side_effect: Arc<AtomicBool>,
}

impl Worker<Sleepy> for SleepyWorker {
    type Error = WorkCancelled;

    async fn work(
        &self,
        context: WorkContext,
        _job: Job<Sleepy>,
    ) -> Result<WorkOutcome, Self::Error> {
        tokio::select! {
            _ = context.cancellation_token().cancelled() => Err(WorkCancelled),
            _ = tokio::time::sleep(Duration::from_secs(60)) => {
                self.side_effect.store(true, Ordering::SeqCst);
                Ok(WorkOutcome::Complete)
            }
        }
    }
}

/// AC2: 実行中のジョブを外から停止すると、worker の cancellation token が発火し、
/// 副作用の前に止まる（自作キューの request_stop / stop_requested に相当）。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn river_cancel_running_job_stops_before_side_effect() {
    if skip() {
        return;
    }
    let db = river_db().await;
    let side_effect = Arc::new(AtomicBool::new(false));
    let mut workers = Workers::new();
    workers
        .add(SleepyWorker {
            side_effect: Arc::clone(&side_effect),
        })
        .unwrap();
    let client = Client::builder(db.pool.clone())
        .workers(workers)
        .queue("default", QueueConfig::new(1))
        .build()
        .unwrap();
    let mut events = client.subscribe(&[EventKind::JobCancelled]).unwrap();
    let mut run = client.start().unwrap();
    let inserted = client.insert(Sleepy {}).await.unwrap();

    // ジョブが running になってから外から停止する。running 観測直後の即時 cancel は
    // 試行の登録窓との競合で稀に通知が落ち、フォールバックの定期検索（既定 1 分）まで
    // 止まらないことを観測したため、登録の後を狙って短く待つ。
    let started = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if river_state(&db.pool, inserted.id()).await == "running" {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    assert!(started.is_ok(), "ジョブが running にならない");
    tokio::time::sleep(Duration::from_millis(300)).await;
    client.jobs().cancel(inserted.id()).await.unwrap();

    let cancelled = tokio::time::timeout(Duration::from_secs(20), async {
        while let Ok(event) = events.recv().await {
            if let Some(job_event) = event.as_job()
                && job_event.job.id == inserted.id()
            {
                return matches!(job_event.job.state, JobState::Cancelled);
            }
        }
        false
    })
    .await
    .expect("停止が JobCancelled として届く");
    assert!(cancelled, "ジョブは cancelled になる");
    assert!(!side_effect.load(Ordering::SeqCst), "副作用の前に止まった");
    assert_eq!(river_state(&db.pool, inserted.id()).await, "cancelled");
    run.stop().await.unwrap();
    db.dispose().await;
}
