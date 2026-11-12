//! RLS 結合テストの共通セットアップ。`PELWAY_DATABASE_URL` が指す PostgreSQL に
//! 一意な名前の scratch database を作り、マイグレーションを当てて、ロール別の
//! プールを返す。未設定なら `None` を返し、呼び出し側のテストは飛ぶ（CI には
//! PostgreSQL が無いため）。設定があるのに接続や作成に失敗したときはパニックで
//! 落とす（黙って飛ばさない）。マイグレーションと `SET ROLE` のどちらも
//! スーパーユーザー権限を前提とする（docs/dev-environment.md の開発用 PostgreSQL）。

use std::str::FromStr;
use std::sync::atomic::{AtomicU32, Ordering};

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{PgPool, Pool, Postgres};

pub struct TestDb {
    /// scratch database の所有者（マイグレーション実行と破棄だけに使う）。
    pub owner: Pool<Postgres>,
    /// アプリ用ロール `pelway_app` で動くプール。物理接続を固定するため 1 接続。
    pub app: PgPool,
    /// 管理用ロール `pelway_admin` で動くプール。
    pub admin: PgPool,
    maintenance: PgPool,
    name: String,
}

impl TestDb {
    /// scratch database を作り、マイグレーションを当てて、ロール別のプールを返す。
    /// ロールはクラスタ共有だが advisory lock はデータベースローカルのため、接続先
    /// データベース上の advisory lock でマイグレーション全体を並行テストプロセスに対して
    /// 直列化する。
    ///
    /// # Panics
    ///
    /// `PELWAY_DATABASE_URL` が設定されているのに接続・作成に失敗したとき。
    pub async fn new() -> Option<Self> {
        let url = std::env::var("PELWAY_DATABASE_URL")
            .ok()
            .filter(|u| !u.is_empty())?;
        let maintenance = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .expect("PELWAY_DATABASE_URL への接続に失敗した");
        let mut guard = maintenance
            .begin()
            .await
            .expect("直列化用トランザクションの開始に失敗した");
        sqlx::query("SELECT pg_advisory_xact_lock(4210100)")
            .execute(&mut *guard)
            .await
            .expect("直列化用 advisory lock の取得に失敗した");
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let name = format!(
            "pelway_rls_{}_{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        // CREATE DATABASE はトランザクション内で実行できないため、guard とは別の
        // 接続（プール）で実行する。
        sqlx::query(sqlx::AssertSqlSafe(
            format!("CREATE DATABASE \"{name}\"").as_str(),
        ))
        .execute(&maintenance)
        .await
        .expect("scratch database の作成に失敗した");
        let base = || {
            PgConnectOptions::from_str(&url)
                .expect("PELWAY_DATABASE_URL を解析できない")
                .database(&name)
        };
        let owner = PgPoolOptions::new()
            .max_connections(2)
            .connect_with(base())
            .await
            .expect("scratch database への接続に失敗した");
        pelway_db::migrate(&owner)
            .await
            .expect("マイグレーションの適用に失敗した");
        let app = pelway_db::app_pool_options()
            .max_connections(1)
            .connect_with(base())
            .await
            .expect("アプリロールでの接続に失敗した");
        let admin = pelway_db::admin_pool_options()
            .max_connections(2)
            .connect_with(base())
            .await
            .expect("管理ロールでの接続に失敗した");
        guard
            .commit()
            .await
            .expect("直列化用トランザクションのコミットに失敗した");
        Some(Self {
            owner,
            app,
            admin,
            maintenance,
            name,
        })
    }

    /// scratch database を接続ごと削除する。テストの最後に呼ぶ。
    pub async fn dispose(self) {
        self.app.close().await;
        self.admin.close().await;
        self.owner.close().await;
        let drop = format!("DROP DATABASE IF EXISTS \"{}\" WITH (FORCE)", self.name);
        sqlx::query(sqlx::AssertSqlSafe(drop.as_str()))
            .execute(&self.maintenance)
            .await
            .expect("scratch database の削除に失敗した");
        self.maintenance.close().await;
    }
}
