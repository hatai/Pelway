//! Pelway の PostgreSQL 層。テナント分離の Row Level Security (ADR 0005)。
//!
//! # 守ること（この crate が表す不変条件）
//!
//! - すべての tenant 表は `workspace_id` を持ち、RLS ポリシーが
//!   [`WORKSPACE_SETTING`] の現在値に一致する行だけを読み書きさせる
//!   （アプリの絞り込みは二重の守りの一方で、書き忘れても DB が返さない）。
//! - 現在の `workspace_id` はトランザクションの中でのみ設定する
//!   （`set_config(..., true)`）。コミット・ロールバックで自動的に戻るため、
//!   接続プールで次の利用者に持ち越されない。
//! - 設定がないトランザクションはどの行も返さない（fail-closed）。
//! - アプリは [`APP_ROLE`] で動き、ポリシーの適用を受ける。管理用の処理は
//!   [`ADMIN_ROLE`]（BYPASSRLS）だけが行え、テナント表への直接の DML 権限を持つ。
//!   このロールの利用の監査は #149 で設計する（ADR 0005 の Consequences の
//!   うち監査の部分。この crate は監査を保証しない）。
//!
//! マイグレーション（`migrate`）は表の所有者（開発ではスーパーユーザー）で
//! 実行すること。`pelway_admin` への BYPASSRLS 付与にはスーパーユーザー権限が
//! 要るため。
//!
//! # テスト
//!
//! 結合テスト（`tests/`）には PostgreSQL が必要で、`PELWAY_DATABASE_URL` が
//! 未設定の環境（CI を含む）では各テストは飛ぶ。手元での実行例:
//!
//! ```sh
//! PELWAY_DATABASE_URL=postgres://pelway:pelway-dev@127.0.0.1:55432/pelway \
//!   cargo test -p pelway-db -- --nocapture
//! ```

#![forbid(unsafe_code)]

use pelway_contract::ids::WorkspaceId;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Acquire, PgPool, Postgres, Transaction};

/// RLS ポリシーが比較する現在の `workspace_id` を保持する GUC の名前。
/// `set_config(name, value, true)` でトランザクションの中だけで設定する。
pub const WORKSPACE_SETTING: &str = "app.workspace_id";

/// アプリ用のロール。RLS ポリシーの適用対象（表の所有者でもスーパーユーザーでもない）。
pub const APP_ROLE: &str = "pelway_app";

/// 管理用のロール。`BYPASSRLS` でポリシーを回避する。移行・バックアップ・横断の
/// 運営作業だけに使う。このロールの利用の監査は #149 で設計する。
pub const ADMIN_ROLE: &str = "pelway_admin";

/// 埋め込みのマイグレーションを実行する。表の所有者（開発ではスーパーユーザー）の
/// プールに対して呼ぶこと。
///
/// # Errors
///
/// マイグレーションの適用に失敗したとき [`sqlx::migrate::MigrateError`] を返す。
pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

/// [`APP_ROLE`] で動くプールのオプション。接続ごとに `SET ROLE` するので、
/// このプールからのすべての文が RLS ポリシーの適用を受ける。
///
/// 接続ユーザーは `pelway_app` のメンバー（開発のスーパーユーザーを含む）である
/// こと。ロールへの降格は接続時に一度だけ行われ、アプリコードはロールを切り替えない。
#[must_use]
pub fn app_pool_options() -> PgPoolOptions {
    PgPoolOptions::new().after_connect(|conn, _| {
        Box::pin(async move {
            // SET ROLE はバインドパラメータを受け付けない。埋め込むのはこの
            // crate の定数だけで、外部からの入力は混ざらない。
            sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE {APP_ROLE}").as_str()))
                .execute(conn)
                .await?;
            Ok(())
        })
    })
}

/// [`ADMIN_ROLE`] で動くプールのオプション。ポリシーを回避できるため、
/// 作る場所は移行・バックアップ・横断の運営作業に限定する。
#[must_use]
pub fn admin_pool_options() -> PgPoolOptions {
    PgPoolOptions::new().after_connect(|conn, _| {
        Box::pin(async move {
            sqlx::query(sqlx::AssertSqlSafe(
                format!("SET ROLE {ADMIN_ROLE}").as_str(),
            ))
            .execute(conn)
            .await?;
            Ok(())
        })
    })
}

/// トランザクションを開き、その中だけで有効な現在の `workspace_id` を設定する。
/// 設定はコミット・ロールバックで自動的に戻る（プールの持ち越しが起きない）。
/// トランザクションの外では `current_setting` は NULL になり、ポリシーはどの行も
/// 返さない。
///
/// # Errors
///
/// トランザクションの開始または `set_config` に失敗したとき [`sqlx::Error`] を返す。
pub async fn begin_workspace_tx<'c, A>(
    acquire: A,
    workspace_id: &WorkspaceId,
) -> sqlx::Result<Transaction<'c, Postgres>>
where
    A: Acquire<'c, Database = Postgres>,
{
    let mut tx = acquire.begin().await?;
    sqlx::query("SELECT set_config($1, $2, true)")
        .bind(WORKSPACE_SETTING)
        .bind(workspace_id.as_str())
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}
