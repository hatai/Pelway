//! 自作 `FOR UPDATE SKIP LOCKED` + lease キューの試験（Issue #99 の AC1〜AC3）。

mod common;

use std::sync::{Arc, Mutex};

use common::TestDb;
use pelway_dev_queue_compare::{
    FailOutcome, WAKE_CHANNEL, complete, dequeue, enqueue_in_tx, fail, mark_stopped,
    notify_wake_in_tx, request_stop, stop_requested,
};
use serde_json::json;
use sqlx::PgPool;
use sqlx::postgres::PgListener;
use sqlx::{Postgres, Transaction};

fn skip() -> bool {
    if std::env::var("PELWAY_DATABASE_URL").is_ok() {
        return false;
    }
    eprintln!("skip: PELWAY_DATABASE_URL 未設定");
    true
}

async fn count(pool: &PgPool, sql: &'static str) -> i64 {
    sqlx::query_scalar(sql).fetch_one(pool).await.unwrap()
}

async fn state(pool: &PgPool, id: i64) -> String {
    sqlx::query_scalar("SELECT state FROM devq_queue_job WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// ジョブを 1 件だけ登録して commit する。
async fn enqueue(pool: &PgPool, kind: &str, max_attempts: i32) -> i64 {
    let mut tx = pool.begin().await.unwrap();
    let id = enqueue_in_tx(&mut tx, kind, &json!({}), max_attempts)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    id
}

/// 業務行（状態・監査・outbox）とジョブを 1 トランザクションに載せる。
async fn write_all(pool: &PgPool) -> (i64, i64, Transaction<'_, Postgres>) {
    let mut tx = pool.begin().await.unwrap();
    let state_id: i64 =
        sqlx::query_scalar("INSERT INTO devq_state_row (note) VALUES ($1) RETURNING id")
            .bind("v1")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    sqlx::query("INSERT INTO devq_audit_log (state_row_id, summary) VALUES ($1, $2)")
        .bind(state_id)
        .bind("state created")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO devq_outbox_event (state_row_id, topic) VALUES ($1, $2)")
        .bind(state_id)
        .bind("state.changed")
        .execute(&mut *tx)
        .await
        .unwrap();
    let job_id = enqueue_in_tx(
        &mut tx,
        "deliver_state",
        &json!({"state_row_id": state_id}),
        5,
    )
    .await
    .unwrap();
    notify_wake_in_tx(&mut tx).await.unwrap();
    (state_id, job_id, tx)
}

/// AC1: 状態・監査・outbox とジョブが同じトランザクションで atomically 見える。
/// commit 前はどの接続からもジョブも業務行も見えず、rollback すれば両方消える。
#[tokio::test]
async fn enqueue_shares_transaction_with_state_audit_outbox() {
    if skip() {
        return;
    }
    let db = TestDb::new().await.unwrap();
    db.apply(common::business_schema()).await.unwrap();

    // rollback すればジョブも業務行も残らない。
    let (_state_id, job_id, tx) = write_all(&db.pool).await;
    assert!(job_id > 0);
    assert_eq!(
        dequeue(&db.pool, 60).await.unwrap(),
        None,
        "commit 前のジョブは他の接続から見えない"
    );
    assert_eq!(
        count(&db.pool, "SELECT count(*) FROM devq_state_row").await,
        0,
        "commit 前の業務行は他の接続から見えない"
    );
    tx.rollback().await.unwrap();
    assert_eq!(
        count(&db.pool, "SELECT count(*) FROM devq_queue_job").await,
        0
    );
    assert_eq!(
        count(&db.pool, "SELECT count(*) FROM devq_outbox_event").await,
        0
    );

    // commit すればジョブが worker に引き渡せる。
    let (state_id, job_id, tx) = write_all(&db.pool).await;
    tx.commit().await.unwrap();
    assert_eq!(
        count(&db.pool, "SELECT count(*) FROM devq_state_row").await,
        1
    );
    let claimed = dequeue(&db.pool, 60)
        .await
        .unwrap()
        .expect("commit 後は取得できる");
    assert_eq!(claimed.id, job_id);
    assert_eq!(claimed.kind, "deliver_state");
    assert_eq!(claimed.payload, json!({"state_row_id": state_id}));
    assert_eq!((claimed.attempts, claimed.max_attempts), (1, 5));
    complete(&db.pool, claimed.id).await.unwrap();
    assert_eq!(state(&db.pool, claimed.id).await, "done");
    db.dispose().await;
}

/// AC2: SKIP LOCKED により、並行する 2 worker が各ジョブをちょうど 1 回ずつ処理する。
#[tokio::test]
async fn skip_locked_two_workers_claim_each_job_exactly_once() {
    if skip() {
        return;
    }
    let db = TestDb::new().await.unwrap();
    for i in 0..8 {
        let mut tx = db.pool.begin().await.unwrap();
        enqueue_in_tx(&mut tx, "tick", &json!({"i": i}), 5)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    let claimed: Arc<Mutex<Vec<i64>>> = Arc::new(Mutex::new(Vec::new()));
    let work = |pool: PgPool, claimed: Arc<Mutex<Vec<i64>>>| async move {
        loop {
            match dequeue(&pool, 30).await.unwrap() {
                Some(job) => {
                    claimed.lock().unwrap().push(job.id);
                    complete(&pool, job.id).await.unwrap();
                }
                None => return,
            }
        }
    };
    tokio::join!(
        work(db.pool.clone(), Arc::clone(&claimed)),
        work(db.pool.clone(), Arc::clone(&claimed)),
    );
    let all = Arc::try_unwrap(claimed)
        .unwrap()
        .into_inner()
        .unwrap_or_default();
    assert_eq!(all.len(), 8, "8 件がちょうど 1 回ずつ処理される");
    let mut unique = all.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), all.len(), "同じジョブを 2 回引き当てていない");
    db.dispose().await;
}

/// AC2: lease 期限切れの回収、再実行上限での打ち切り、停止要求の扱い。
#[tokio::test]
async fn lease_expiry_retry_cap_and_stop_requests() {
    if skip() {
        return;
    }
    let db = TestDb::new().await.unwrap();

    // lease が切れた running 行は、停止した worker の代わりに再取得できる。
    let id = enqueue(&db.pool, "reclaim", 2).await;
    let first = dequeue(&db.pool, 60)
        .await
        .unwrap()
        .expect("最初の引き当て");
    assert_eq!((first.id, first.attempts), (id, 1));
    // worker が停止したことにする: lease を過去に戻す（検証だけが行う操作）。
    sqlx::query(
        "UPDATE devq_queue_job SET lease_until = now() - interval '1 second' WHERE id = $1",
    )
    .bind(id)
    .execute(&db.pool)
    .await
    .unwrap();
    let second = dequeue(&db.pool, 60)
        .await
        .unwrap()
        .expect("lease 期限切れの行は回収できる");
    assert_eq!((second.id, second.attempts), (id, 2));

    // 再実行上限に達したジョブは dead になり、それ以上引き当てない。
    assert_eq!(
        fail(&db.pool, id).await.unwrap(),
        FailOutcome::Dead { attempts: 2 },
        "max_attempts=2 の 2 回目の失敗は dead"
    );
    assert_eq!(dequeue(&db.pool, 60).await.unwrap(), None);
    assert_eq!(state(&db.pool, id).await, "dead");

    // 上限前の失敗はバックオフ付きで再実行待ちに戻る。
    let retry_id = enqueue(&db.pool, "retry", 3).await;
    let claimed = dequeue(&db.pool, 60).await.unwrap().unwrap();
    assert_eq!(claimed.id, retry_id);
    assert_eq!(
        fail(&db.pool, retry_id).await.unwrap(),
        FailOutcome::Requeued { attempts: 1 },
        "max_attempts=3 の 1 回目の失敗は再実行待ち"
    );
    assert_eq!(state(&db.pool, retry_id).await, "queued");

    // 未実行ジョブの停止は即座に cancelled になる。
    let queued_id = enqueue(&db.pool, "stop_queued", 3).await;
    request_stop(&db.pool, queued_id).await.unwrap();
    assert_eq!(state(&db.pool, queued_id).await, "cancelled");
    assert_eq!(dequeue(&db.pool, 60).await.unwrap(), None);

    // 実行中ジョブへの停止要求は、worker が副作用の前に拾える。
    let running_id = enqueue(&db.pool, "stop_running", 3).await;
    let claimed = dequeue(&db.pool, 60).await.unwrap().unwrap();
    assert_eq!(claimed.id, running_id);
    request_stop(&db.pool, running_id).await.unwrap();
    assert!(
        stop_requested(&db.pool, running_id).await.unwrap(),
        "実行中ジョブには停止要求フラグが立つ"
    );
    mark_stopped(&db.pool, running_id).await.unwrap();
    assert_eq!(state(&db.pool, running_id).await, "cancelled");
    db.dispose().await;
}

/// AC3: NOTIFY は起こす合図として commit と同時に届き、取りこぼしても
/// 定期の追いつき検索（dequeue の呼び出し）だけでジョブを処理できる。
#[tokio::test]
async fn missed_notify_is_covered_by_periodic_catchup_polling() {
    if skip() {
        return;
    }
    let db = TestDb::new().await.unwrap();

    // (a) commit と同時に起こす合図が LISTEN している接続に届く。
    let mut listener = PgListener::connect_with(&db.pool).await.unwrap();
    listener.listen(WAKE_CHANNEL).await.unwrap();
    let signalled_id = {
        let mut tx = db.pool.begin().await.unwrap();
        let id = enqueue_in_tx(&mut tx, "wake", &json!({}), 3).await.unwrap();
        notify_wake_in_tx(&mut tx).await.unwrap();
        tx.commit().await.unwrap();
        id
    };
    let notification = tokio::time::timeout(std::time::Duration::from_secs(5), listener.recv())
        .await
        .expect("commit と同時に合図が届く")
        .unwrap();
    assert_eq!(notification.channel(), WAKE_CHANNEL);
    complete(&db.pool, signalled_id).await.unwrap();

    // (b) 合図を取りこぼす（LISTEN している者を消す）。
    drop(listener);
    let missed_id = {
        let mut tx = db.pool.begin().await.unwrap();
        let id = enqueue_in_tx(&mut tx, "missed", &json!({}), 3)
            .await
            .unwrap();
        notify_wake_in_tx(&mut tx).await.unwrap();
        tx.commit().await.unwrap();
        id
    };
    // 定期の追いつき検索だけを行う worker: 合図に頼らずポーリングで拾う。
    let claimed = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if let Some(job) = dequeue(&db.pool, 30).await.unwrap() {
                return job;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("合図を取りこぼしても定期検索で追いつく");
    assert_eq!(claimed.id, missed_id);
    complete(&db.pool, claimed.id).await.unwrap();
    db.dispose().await;
}
