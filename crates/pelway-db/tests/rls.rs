//! Issue #100 の受け入れ条件ごとの結合テスト。
//!
//! - AC1: トランザクションごとに現在の `workspace_id` を設定し、各テーブルの
//!   ポリシーで絞り込む（読みも書きも自ワークスペースだけ）。
//! - AC2: 接続プールで `workspace_id` の設定が次の利用者に持ち越されない。
//! - AC3: ポリシーを回避する管理用のロールを分け、その利用を監査の対象にする。
//! - AC4: アプリが絞り込みを書き忘れても、別のワークスペースの行が返らない。
//!
//! ポリシーの定義（pg_policies）ではなく、実際に返る行で確かめる。カタログの
//! 検査は通っても実行時に漏れていることはあり得るため。

mod common;

use common::TestDb;
use pelway_contract::ids::WorkspaceId;
use pelway_db::{WORKSPACE_SETTING, begin_admin_tx, begin_workspace_tx};

const WS_A: &str = "ws_rls_alpha";
const WS_B: &str = "ws_rls_beta";

async fn test_db() -> Option<TestDb> {
    TestDb::new().await
}

fn skip() {
    eprintln!("skip: PELWAY_DATABASE_URL が未設定（PostgreSQL が必要）");
}

/// 両ワークスペースの行を、表の所有者（マイグレーションを実行したユーザー）で
/// 登録する。チャンネルは A に 2 つ、B に 1 つ。テストの対象はアプリロールと
/// 管理ロールの経路なので、シード自体はどちらの経路も使わない。
async fn seed(owner: &sqlx::PgPool) {
    let mut tx = owner
        .begin()
        .await
        .expect("シード用トランザクションの開始に失敗した");
    sqlx::query("INSERT INTO workspaces (id, name) VALUES ($1, $1), ($2, $2)")
        .bind(WS_A)
        .bind(WS_B)
        .execute(&mut *tx)
        .await
        .expect("ワークスペースの登録に失敗した");
    sqlx::query(
        "INSERT INTO channels (workspace_id, id, name) VALUES \
         ($1, 'ch_a1', 'a1'), ($1, 'ch_a2', 'a2'), ($2, 'ch_b1', 'b1')",
    )
    .bind(WS_A)
    .bind(WS_B)
    .execute(&mut *tx)
    .await
    .expect("チャンネルの登録に失敗した");
    tx.commit().await.expect("シードのコミットに失敗した");
}

/// AC1: workspace トランザクションの中では、現在の `workspace_id` の行だけが
/// 読め、書ける。他ワークスペースへの書き込みはポリシーの WITH CHECK で拒否される。
#[tokio::test]
async fn workspace_tx_reads_and_writes_only_current_workspace() {
    let Some(db) = test_db().await else {
        skip();
        return;
    };
    seed(&db.owner).await;

    let mut tx = begin_workspace_tx(&db.app, &WorkspaceId::from(WS_A))
        .await
        .expect("workspace トランザクションの開始に失敗した");
    let ids: Vec<String> = sqlx::query_scalar("SELECT id FROM channels ORDER BY id")
        .fetch_all(&mut *tx)
        .await
        .expect("チャンネル一覧の取得に失敗した");
    assert_eq!(ids, ["ch_a1", "ch_a2"], "自ワークスペースの行だけ返る");

    sqlx::query("INSERT INTO channels (workspace_id, id, name) VALUES ($1, 'ch_a3', 'a3')")
        .bind(WS_A)
        .execute(&mut *tx)
        .await
        .expect("自ワークスペースへの挿入は受け付けられる");

    let err = sqlx::query("INSERT INTO channels (workspace_id, id, name) VALUES ($1, 'b9', 'b9')")
        .bind(WS_B)
        .execute(&mut *tx)
        .await
        .expect_err("他ワークスペースへの挿入は拒否されるべき");
    assert!(
        err.to_string().contains("row-level security"),
        "RLS の拒否であるべき: {err}"
    );

    tx.commit().await.expect("コミットに失敗した");
    db.dispose().await;
}

/// AC2: トランザクションで設定した `workspace_id` は、コミット後に同じ物理接続を
/// 借り直した次の利用者に持ち越されない。未設定のまま読めばどの行も返らない。
#[tokio::test]
async fn pooled_connection_does_not_carry_workspace_setting_to_next_user() {
    let Some(db) = test_db().await else {
        skip();
        return;
    };
    seed(&db.owner).await;

    // アプリプールは物理接続 1 本。最初の利用者のトランザクション。
    {
        let mut tx = begin_workspace_tx(&db.app, &WorkspaceId::from(WS_A))
            .await
            .expect("workspace トランザクションの開始に失敗した");
        let n: i64 = sqlx::query_scalar("SELECT count(*) FROM channels")
            .fetch_one(&mut *tx)
            .await
            .expect("チャンネル数の取得に失敗した");
        assert_eq!(n, 2);
        tx.commit().await.expect("コミットに失敗した");
    }

    // 次の利用者: 同じ物理接続をプールから借り直し、workspace_id を設定せずに読む。
    let mut conn = db.app.acquire().await.expect("接続の借用に失敗した");
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM channels")
        .fetch_one(&mut *conn)
        .await
        .expect("チャンネル数の取得に失敗した");
    assert_eq!(
        n, 0,
        "次の利用者に workspace_id の設定が持ち越されてはならない"
    );
    let setting: Option<String> = sqlx::query_scalar("SELECT current_setting($1, true)")
        .bind(WORKSPACE_SETTING)
        .fetch_one(&mut *conn)
        .await
        .expect("現在値の取得に失敗した");
    assert!(
        setting.as_deref().unwrap_or("").is_empty(),
        "設定はトランザクションの外へ漏れてはならない（PostgreSQL のプレースホルダ\
         GUC は戻るとき空文字列になることがある）: {setting:?}"
    );
    drop(conn);

    // 別ワークスペースの利用者が同じ接続を使っても、前の利用者の設定は残っていない。
    let mut tx = begin_workspace_tx(&db.app, &WorkspaceId::from(WS_B))
        .await
        .expect("workspace トランザクションの開始に失敗した");
    let ids: Vec<String> = sqlx::query_scalar("SELECT id FROM channels ORDER BY id")
        .fetch_all(&mut *tx)
        .await
        .expect("チャンネル一覧の取得に失敗した");
    assert_eq!(ids, ["ch_b1"]);
    tx.commit().await.expect("コミットに失敗した");
    db.dispose().await;
}

/// AC3: 管理ロール（BYPASSRLS）は全ワークスペースの行を見られるが、その利用は
/// `admin_access_log` に記録される。アプリロールは監査記録を書けない。
#[tokio::test]
async fn admin_role_bypasses_policies_and_its_usage_is_audited() {
    let Some(db) = test_db().await else {
        skip();
        return;
    };
    // このテストのシードと集計は管理ロールの経路で行う。横断の読み書きが
    // ポリシーなしで通ることと、その利用が監査記録に残ることを同時に確かめる。
    let mut tx = begin_admin_tx(&db.admin, "seed and count channels across workspaces")
        .await
        .expect("管理トランザクションの開始に失敗した");
    sqlx::query("INSERT INTO workspaces (id, name) VALUES ($1, $1), ($2, $2)")
        .bind(WS_A)
        .bind(WS_B)
        .execute(&mut *tx)
        .await
        .expect("ワークスペースの登録に失敗した");
    sqlx::query(
        "INSERT INTO channels (workspace_id, id, name) VALUES \
         ($1, 'ch_a1', 'a1'), ($1, 'ch_a2', 'a2'), ($2, 'ch_b1', 'b1')",
    )
    .bind(WS_A)
    .bind(WS_B)
    .execute(&mut *tx)
    .await
    .expect("チャンネルの登録に失敗した");
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM channels")
        .fetch_one(&mut *tx)
        .await
        .expect("チャンネル数の取得に失敗した");
    assert_eq!(n, 3, "管理ロールは全ワークスペースの行を見られる");
    tx.commit().await.expect("コミットに失敗した");

    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT db_user, reason FROM admin_access_log ORDER BY id")
            .fetch_all(&db.owner)
            .await
            .expect("監査記録の取得に失敗した");
    assert!(
        rows.iter()
            .any(|(user, reason)| user == "pelway_admin"
                && reason.contains("seed and count channels")),
        "管理ロールの利用が監査記録に残っているべき: {rows:?}"
    );

    let err = sqlx::query("INSERT INTO admin_access_log (db_user, reason) VALUES ('forged', 'x')")
        .execute(&db.app)
        .await
        .expect_err("アプリロールによる監査記録の捏造は拒否されるべき");
    assert!(
        err.to_string().contains("permission denied"),
        "権限の拒否であるべき: {err}"
    );
    db.dispose().await;
}

/// AC4: WHERE 句で workspace_id を絞り込まない書き方をしても、ポリシーが他の
/// ワークスペースの行を返さない。UPDATE・DELETE も他社の行には届かない。
#[tokio::test]
async fn missing_workspace_filter_does_not_return_other_workspaces_rows() {
    let Some(db) = test_db().await else {
        skip();
        return;
    };
    seed(&db.owner).await;

    let mut tx = begin_workspace_tx(&db.app, &WorkspaceId::from(WS_B))
        .await
        .expect("workspace トランザクションの開始に失敗した");
    let ids: Vec<String> = sqlx::query_scalar("SELECT id FROM channels ORDER BY id")
        .fetch_all(&mut *tx)
        .await
        .expect("チャンネル一覧の取得に失敗した");
    assert_eq!(
        ids,
        ["ch_b1"],
        "絞り込みを書き忘れても自ワークスペースの行だけが返る"
    );

    let deleted = sqlx::query("DELETE FROM channels")
        .execute(&mut *tx)
        .await
        .expect("削除の実行に失敗した");
    assert_eq!(deleted.rows_affected(), 1, "他社の行は削除できていない");

    tx.commit().await.expect("コミットに失敗した");

    // 削除されたのは B の 1 行だけ。A の行は無事。
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM channels")
        .fetch_one(&db.owner)
        .await
        .expect("チャンネル数の取得に失敗した");
    assert_eq!(n, 2, "ワークスペース A の行は残っているべき");
    db.dispose().await;
}
