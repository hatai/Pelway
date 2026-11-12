//! 結合テストの共通セットアップ。`PELWAY_DATABASE_URL` が指す PostgreSQL に
//! 一意な名前の scratch database を作り、テストごとに完全に分離する。

use std::str::FromStr;
use std::sync::atomic::{AtomicU32, Ordering};

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{PgPool, Pool, Postgres};

/// 業務側のダミー表（状態・監査・outbox の代わり）。AC1 のトランザクション試験で使う。
pub fn business_schema() -> &'static str {
    "CREATE TABLE IF NOT EXISTS devq_state_row (
        id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
        note TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS devq_audit_log (
        id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
        state_row_id BIGINT NOT NULL REFERENCES devq_state_row (id),
        summary TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS devq_outbox_event (
        id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
        state_row_id BIGINT NOT NULL REFERENCES devq_state_row (id),
        topic TEXT NOT NULL
    );"
}

/// テスト専用の scratch database。`new` は `PELWAY_DATABASE_URL` が未設定のとき
/// `None` を返し、呼び出し側のテストは飛ばす（CI には PostgreSQL が無いため）。
/// 設定があるのに接続や作成に失敗したときはパニックで落とす（黙って飛ばさない）。
pub struct TestDb {
    pub pool: Pool<Postgres>,
    admin: PgPool,
    name: String,
}

impl TestDb {
    /// scratch database を作り、自作キューのスキーマを当てる。
    ///
    /// # Panics
    ///
    /// `PELWAY_DATABASE_URL` が設定されているのに接続・作成に失敗したとき。
    pub async fn new() -> Option<Self> {
        let url = std::env::var("PELWAY_DATABASE_URL")
            .ok()
            .filter(|u| !u.is_empty())?;
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .expect("PELWAY_DATABASE_URL への接続に失敗した");
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let name = format!(
            "pelway_qcmp_{}_{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        sqlx::query(sqlx::AssertSqlSafe(
            format!("CREATE DATABASE \"{name}\"").as_str(),
        ))
        .execute(&admin)
        .await
        .expect("scratch database の作成に失敗した");
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect_with(
                PgConnectOptions::from_str(&url)
                    .expect("PELWAY_DATABASE_URL を解析できない")
                    .database(&name),
            )
            .await
            .expect("scratch database への接続に失敗した");
        sqlx::raw_sql(pelway_dev_queue_compare::create_table_sql())
            .execute(&pool)
            .await
            .expect("自作キューのスキーマ適用に失敗した");
        Some(Self { pool, admin, name })
    }

    /// 追加の DDL を当てる（リテラル SQL に限る）。
    ///
    /// # Errors
    ///
    /// SQL の実行に失敗したとき [`sqlx::Error`] を返す。
    pub async fn apply(&self, sql: &'static str) -> sqlx::Result<()> {
        sqlx::raw_sql(sql).execute(&self.pool).await.map(|_| ())
    }

    /// scratch database を接続ごと削除する。テストの最後に呼ぶ。
    pub async fn dispose(self) {
        self.pool.close().await;
        let drop = format!("DROP DATABASE IF EXISTS \"{}\" WITH (FORCE)", self.name);
        sqlx::query(sqlx::AssertSqlSafe(drop.as_str()))
            .execute(&self.admin)
            .await
            .expect("scratch database の削除に失敗した");
        self.admin.close().await;
    }
}
