//! Issue #99 の比較検証ハーネス。製品機能を含まない。
//!
//! 自作の `FOR UPDATE SKIP LOCKED` + lease による DB キューと River（`riverqueue`）を、
//! 同じ PostgreSQL に対する結合テスト（`tests/` 配下）で比べる。比較の結果と採否は
//! `docs/adr/0006-db-queue-choice.md` に記録する。
//!
//! テストの実行には PostgreSQL が必要。CI には PostgreSQL が無いため、
//! `PELWAY_DATABASE_URL` が未設定の環境では各テストは飛ぶ。手元での実行例:
//!
//! ```sh
//! podman run -d --name pelway-queue-compare-pg -p 127.0.0.1:55432:5432 \
//!   -e POSTGRES_USER=pelway -e POSTGRES_PASSWORD=pelway-dev -e POSTGRES_DB=pelway \
//!   docker.io/library/postgres:16-alpine
//! PELWAY_DATABASE_URL=postgres://pelway:pelway-dev@127.0.0.1:55432/pelway \
//!   cargo test -p pelway-dev-queue-compare -- --nocapture
//! ```
//!
//! 各テストは接続先に一意な名前の scratch database を作り、終わったら削除する。

use serde_json::Value as Json;
use sqlx::PgConnection;

/// worker を起こす合図に使う NOTIFY チャンネル名。正本はジョブ表の行。
pub const WAKE_CHANNEL: &str = "pelway_dev_queue_wake";

/// ジョブ表の DDL。呼び出し側のマイグレーションに組み込む前提の内容。
#[must_use]
pub fn create_table_sql() -> &'static str {
    "CREATE TABLE IF NOT EXISTS devq_queue_job (
        id               BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
        kind             TEXT        NOT NULL,
        payload          JSONB       NOT NULL DEFAULT '{}',
        state            TEXT        NOT NULL DEFAULT 'queued',
        attempts         INT         NOT NULL DEFAULT 0,
        max_attempts     INT         NOT NULL,
        run_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
        lease_until      TIMESTAMPTZ,
        cancel_requested BOOLEAN     NOT NULL DEFAULT false,
        created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
        updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
    );
    CREATE INDEX IF NOT EXISTS devq_queue_job_pick ON devq_queue_job (state, run_at, id);"
}

/// [`dequeue`] が引き当てたジョブ。
#[derive(Debug, PartialEq)]
pub struct ClaimedJob {
    pub id: i64,
    pub kind: String,
    pub payload: Json,
    pub attempts: i32,
    pub max_attempts: i32,
}

/// [`fail`] の結果。`attempts` は失敗後の試行回数。
#[derive(Debug, PartialEq, Eq)]
pub enum FailOutcome {
    /// `max_attempts` に達していなければ、バックオフ付きで再実行待ちに戻す。
    Requeued { attempts: i32 },
    /// `max_attempts` に達したのでそれ以上実行しない。
    Dead { attempts: i32 },
}

/// 呼び出し側のトランザクションの中でジョブを登録する。状態・監査・outbox と同じ
/// トランザクションで呼ぶことで、業務行とジョブ行が同時に見えるか同時に消えるかを
/// 保証する。通知は [`notify_wake_in_tx`] を別に呼ぶ。
///
/// # Errors
///
/// SQL の実行に失敗したとき [`sqlx::Error`] を返す。
pub async fn enqueue_in_tx(
    tx: &mut PgConnection,
    kind: &str,
    payload: &Json,
    max_attempts: i32,
) -> sqlx::Result<i64> {
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO devq_queue_job (kind, payload, max_attempts) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(kind)
    .bind(payload)
    .bind(max_attempts)
    .fetch_one(tx)
    .await?;
    Ok(id)
}

/// トランザクションの中で worker 起こしの合図（`pg_notify`）を送る。
/// commit と同時に届き、rollback すれば取り消される。
///
/// # Errors
///
/// SQL の実行に失敗したとき [`sqlx::Error`] を返す。
pub async fn notify_wake_in_tx(tx: &mut PgConnection) -> sqlx::Result<()> {
    sqlx::query("SELECT pg_notify($1, '')")
        .bind(WAKE_CHANNEL)
        .execute(tx)
        .await?;
    Ok(())
}

/// 実行可能なジョブを 1 件引き当て、lease を取る。`FOR UPDATE SKIP LOCKED` により、
/// 他の worker が処理中の行は飛ぶ。`queued` で `run_at` を過ぎた行に加え、`running` の
/// まま lease が切れた行（worker が停止したとき）も回収する。該当がなければ `None`。
///
/// # Errors
///
/// SQL の実行に失敗したとき [`sqlx::Error`] を返す。
pub async fn dequeue<'e, E>(executor: E, lease_secs: i64) -> sqlx::Result<Option<ClaimedJob>>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let claimed = sqlx::query_as::<_, (i64, String, Json, i32, i32)>(
        "UPDATE devq_queue_job AS j
         SET state = 'running',
             attempts = j.attempts + 1,
             lease_until = now() + make_interval(secs => $1::double precision),
             updated_at = now()
         WHERE j.id IN (
             SELECT id FROM devq_queue_job
             WHERE (state = 'queued' AND run_at <= now())
                OR (state = 'running' AND lease_until IS NOT NULL AND lease_until < now())
             ORDER BY id
             LIMIT 1
             FOR UPDATE SKIP LOCKED
         )
         RETURNING j.id, j.kind, j.payload, j.attempts, j.max_attempts",
    )
    .bind(lease_secs)
    .fetch_optional(executor)
    .await?;
    Ok(
        claimed.map(|(id, kind, payload, attempts, max_attempts)| ClaimedJob {
            id,
            kind,
            payload,
            attempts,
            max_attempts,
        }),
    )
}

/// ジョブを完了にする。
///
/// # Errors
///
/// SQL の実行に失敗したとき [`sqlx::Error`] を返す。
pub async fn complete<'e, E>(executor: E, id: i64) -> sqlx::Result<()>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query("UPDATE devq_queue_job SET state = 'done', lease_until = NULL, updated_at = now() WHERE id = $1")
        .bind(id)
        .execute(executor)
        .await?;
    Ok(())
}

/// ジョブを失敗として処理する。`max_attempts` に達していなければ、試行回数の
/// 2 乗秒（上限 60 秒）のバックオフ付きで再実行待ちに戻す。
///
/// # Errors
///
/// SQL の実行に失敗したとき、または該当行が無いとき [`sqlx::Error`] を返す。
pub async fn fail<'e, E>(executor: E, id: i64) -> sqlx::Result<FailOutcome>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let outcome: Option<(String, i32)> = sqlx::query_as(
        "UPDATE devq_queue_job AS j
         SET state = CASE WHEN j.attempts >= j.max_attempts THEN 'dead' ELSE 'queued' END,
             run_at = now() + make_interval(secs => LEAST(j.attempts * j.attempts, 60)::double precision),
             lease_until = NULL,
             updated_at = now()
         WHERE id = $1
         RETURNING state, attempts",
    )
    .bind(id)
    .fetch_optional(executor)
    .await?;
    match outcome {
        Some((state, attempts)) if state == "dead" => Ok(FailOutcome::Dead { attempts }),
        Some((_state, attempts)) => Ok(FailOutcome::Requeued { attempts }),
        None => Err(sqlx::Error::RowNotFound),
    }
}

/// ジョブに停止を要求する。未実行（`queued`）なら即座に `cancelled` にする。
/// 実行中（`running`）なら `cancel_requested` を立てるだけなので、worker は副作用の
/// 前に [`stop_requested`] でこれを拾い、[`mark_stopped`] で止める。
///
/// # Errors
///
/// SQL の実行に失敗したとき [`sqlx::Error`] を返す。
pub async fn request_stop<'e, E>(executor: E, id: i64) -> sqlx::Result<()>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query(
        "UPDATE devq_queue_job
         SET state = CASE WHEN state = 'queued' THEN 'cancelled' ELSE state END,
             cancel_requested = true,
             updated_at = now()
         WHERE id = $1 AND state IN ('queued', 'running')",
    )
    .bind(id)
    .execute(executor)
    .await?;
    Ok(())
}

/// 実行中のジョブに停止要求が出ているかを確かめる。
///
/// # Errors
///
/// SQL の実行に失敗したとき [`sqlx::Error`] を返す。
pub async fn stop_requested<'e, E>(executor: E, id: i64) -> sqlx::Result<bool>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let requested: Option<bool> =
        sqlx::query_scalar("SELECT cancel_requested FROM devq_queue_job WHERE id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await?;
    Ok(requested.unwrap_or(false))
}

/// 停止要求を拾った worker がジョブを `cancelled` にする。
///
/// # Errors
///
/// SQL の実行に失敗したとき [`sqlx::Error`] を返す。
pub async fn mark_stopped<'e, E>(executor: E, id: i64) -> sqlx::Result<()>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query("UPDATE devq_queue_job SET state = 'cancelled', updated_at = now() WHERE id = $1 AND state = 'running'")
        .bind(id)
        .execute(executor)
        .await?;
    Ok(())
}
