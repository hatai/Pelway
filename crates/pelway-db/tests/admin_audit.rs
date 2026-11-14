//! Issue #149 の受け入れ条件ごとの結合テスト（管理ロールの利用の監査）。
//! AC1 監査の先行確定 / AC2 ロールバック後の監査残存 / AC3 fail-closed /
//! AC4 監査行の単一使用 / AC5 監査表の権限 / AC6 `prev_hash` 鎖の検証（各テストの
//! doc コメントに条件の全文がある）。「操作の前に」「別トランザクション」は
//! AC2 と AC3 の組み合わせで振る舞いとして確かめる（時刻の比較に頼らない）。

mod common;

use common::TestDb;
use pelway_audit_checkpoint::chain::{AuditEvent, GENESIS, StoredRow};
use pelway_db::admin::{AdminOp, AdminOpError, execute_admin_op};
use sha2::{Digest, Sha256};
use sqlx::FromRow;

async fn test_db() -> Option<TestDb> {
    TestDb::new().await
}

fn skip() {
    eprintln!("skip: PELWAY_DATABASE_URL が未設定（PostgreSQL が必要）");
}

/// チャンネル表で id に一致する行数を数える（操作の効果の確認用）。
async fn channels(owner: &sqlx::PgPool, id: &str) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM channels WHERE id = $1")
        .bind(id)
        .fetch_one(owner)
        .await
        .expect("チャンネル数の取得に失敗した")
}

/// 監査表の行（テスト用の読み取り表現）。
#[derive(Debug, FromRow)]
struct AuditRow {
    id: i64,
    workspace_id: String,
    at_ms: i64,
    actor: String,
    subject: String,
    reason: String,
    kind: String,
    result: String,
    snapshot_hash: Vec<u8>,
    policy_version: i64,
    prev_hash: Vec<u8>,
    hash: Vec<u8>,
    ref_id: Option<i64>,
}

impl AuditRow {
    async fn all(owner: &sqlx::PgPool) -> Vec<Self> {
        sqlx::query_as(
            "SELECT id, workspace_id, at_ms, actor, subject, reason, kind, result, \
             snapshot_hash, policy_version, prev_hash, hash, ref_id FROM admin_audit_events \
             ORDER BY id",
        )
        .fetch_all(owner)
        .await
        .expect("監査表の読み取りに失敗した")
    }

    fn to_stored(&self) -> StoredRow {
        let event = AuditEvent {
            workspace_id: self.workspace_id.clone(),
            at_ms: u64::try_from(self.at_ms).expect("at_ms は負でない"),
            actor: self.actor.clone(),
            subject: self.subject.clone(),
            reason: Some(self.reason.clone()),
            kind: self.kind.clone(),
            result: self.result.clone(),
            snapshot_hash: self.snapshot_hash.clone().try_into().expect("32 バイト"),
            policy_version: u64::try_from(self.policy_version).expect("負でない"),
            ref_id: self
                .ref_id
                .map(|r| u64::try_from(r).expect("id は負でない")),
        };
        StoredRow {
            event,
            prev_hash: self.prev_hash.clone().try_into().expect("32 バイト"),
            hash: self.hash.clone().try_into().expect("32 バイト"),
        }
    }
}

/// ワークスペース 1 つとチャンネル 1 つを管理ロールで登録する（1 回の複数文
/// 実行＝1 つの暗黙トランザクション。id は各テストの SQL と同じもの）。
async fn seed(admin: &sqlx::PgPool) {
    sqlx::raw_sql(concat!(
        "INSERT INTO workspaces (id, name) VALUES ('ws_admin_audit', 'ws_admin_audit'); ",
        "INSERT INTO channels (workspace_id, id, name) \
         VALUES ('ws_admin_audit', 'ch_seed', 'seed');"
    ))
    .execute(admin)
    .await
    .expect("シードの登録に失敗した");
}

fn sample_op(sql: &str) -> AdminOp {
    AdminOp {
        operator: "ops@example".into(),
        subject: "channels".into(),
        reason: "issue #149 の受け入れ条件確認".into(),
        sql: sql.into(),
    }
}

/// AC1: 管理操作の前に intent 監査行が別トランザクションで確定され、操作の後に
/// result 行が追記される。intent 行は鎖の起点（GENESIS）につながる。
#[tokio::test]
async fn audit_row_committed_before_admin_op() {
    let Some(db) = test_db().await else {
        skip();
        return;
    };
    seed(&db.admin).await;
    let sql =
        "INSERT INTO channels (workspace_id, id, name) VALUES ('ws_admin_audit', 'ch_ac1', 'ac1')";
    let out = execute_admin_op(&db.audit, &db.admin, sample_op(sql))
        .await
        .expect("管理操作の実行に失敗した");

    let rows = AuditRow::all(&db.owner).await;
    assert_eq!(rows.len(), 2, "intent 行と result 行の 2 行");
    let (intent, result) = (&rows[0], &rows[1]);
    assert_eq!(intent.id, out, "返り値のトークンは intent 行の id");
    assert_eq!(intent.workspace_id, "admin");
    assert_eq!(
        (
            intent.actor.as_str(),
            intent.subject.as_str(),
            intent.reason.as_str()
        ),
        ("ops@example", "channels", "issue #149 の受け入れ条件確認"),
        "監査行に実行者・対象・理由が記録される"
    );
    assert_eq!(intent.kind, "admin-op-intent");
    assert_eq!(intent.result, "pending");
    assert_eq!(
        intent.snapshot_hash,
        Sha256::digest(sql.as_bytes()).to_vec(),
        "snapshot_hash は実行する SQL の SHA-256"
    );
    assert_eq!(intent.policy_version, 0);
    assert_eq!(
        intent.prev_hash,
        GENESIS.to_vec(),
        "最初の行は鎖の起点につながる"
    );
    assert!(intent.ref_id.is_none());

    assert_eq!(result.kind, "admin-op-result");
    assert_eq!(result.result, "ok");
    assert_eq!(
        result.ref_id,
        Some(intent.id),
        "result 行は intent 行を参照する"
    );
    assert_eq!(
        result.prev_hash, intent.hash,
        "result 行は intent 行のハッシュにつながる"
    );
    assert!(
        result.id > intent.id,
        "result 行は intent 行の後ろに置かれる"
    );

    assert_eq!(
        channels(&db.owner, "ch_ac1").await,
        1,
        "管理操作の効果が見える"
    );
    db.dispose().await;
}

/// AC2: 管理操作が失敗してロールバックしても（複数文の暗黙トランザクションが
/// 全体として巻き戻っても）、監査行は別トランザクションで確定済みのため残る。
#[tokio::test]
async fn audit_row_survives_admin_op_rollback() {
    let Some(db) = test_db().await else {
        skip();
        return;
    };
    seed(&db.admin).await;
    // 1 文目は成功するが、2 文目の外部キー違反で暗黙トランザクション全体が
    // ロールバックされる（1 文目の効果も消える）。
    let sql = "INSERT INTO channels (workspace_id, id, name) VALUES ('ws_admin_audit', 'ch_rb', 'rb'); \
               INSERT INTO channels (workspace_id, id, name) VALUES ('missing_ws', 'ch_bad', 'bad')";
    let err = execute_admin_op(&db.audit, &db.admin, sample_op(sql))
        .await
        .expect_err("2 文目の外部キー違反で操作は失敗するべき");
    assert!(
        matches!(err, AdminOpError::Operation(_)),
        "操作自体の失敗であるべき: {err}"
    );

    assert_eq!(
        channels(&db.owner, "ch_rb").await,
        0,
        "ロールバックで 1 文目の効果も消えている"
    );

    let rows = AuditRow::all(&db.owner).await;
    assert_eq!(rows.len(), 2, "ロールバックしても監査行は 2 行残る");
    assert_eq!(rows[0].kind, "admin-op-intent");
    assert_eq!(rows[1].kind, "admin-op-result");
    assert!(
        rows[1].result.starts_with("error:"),
        "result 行は失敗を記録する: {}",
        rows[1].result
    );
    db.dispose().await;
}

/// AC3: intent 行の確定に失敗したとき、管理操作は実行されない（fail-closed）。
#[tokio::test]
async fn admin_op_fails_closed_when_audit_commit_fails() {
    let Some(db) = test_db().await else {
        skip();
        return;
    };
    seed(&db.admin).await;
    // 監査ロールから INSERT 権限を剥いで監査行の確定を失敗させる。
    sqlx::query("REVOKE INSERT ON admin_audit_events FROM pelway_audit")
        .execute(&db.owner)
        .await
        .expect("REVOKE に失敗した");

    let sql =
        "INSERT INTO channels (workspace_id, id, name) VALUES ('ws_admin_audit', 'ch_fc', 'fc')";
    let err = execute_admin_op(&db.audit, &db.admin, sample_op(sql))
        .await
        .expect_err("監査行が書けないときは失敗するべき");
    assert!(
        matches!(err, AdminOpError::AuditWrite(_)),
        "監査書き込みの失敗であるべき: {err}"
    );

    assert_eq!(
        channels(&db.owner, "ch_fc").await,
        0,
        "管理操作は実行されていない"
    );

    // 後続のために権限を戻す。
    sqlx::query("GRANT SELECT, INSERT ON admin_audit_events TO pelway_audit")
        .execute(&db.owner)
        .await
        .expect("GRANT に失敗した");
    db.dispose().await;
}

/// AC4: 同じ実行者・対象・理由・SQL でも、操作ごとに新しい intent 行が伴う
/// （1 行の監査行を使い回して 2 度の操作はできない）。
#[tokio::test]
async fn audit_row_is_single_use() {
    let Some(db) = test_db().await else {
        skip();
        return;
    };
    seed(&db.admin).await;
    // 同じ SQL で 2 回実行できるよう、冪等な書き方にする（2 回目は 0 行影響）。
    let sql = "INSERT INTO channels (workspace_id, id, name) VALUES ('ws_admin_audit', 'ch_su', 'su') ON CONFLICT DO NOTHING";
    let first = execute_admin_op(&db.audit, &db.admin, sample_op(sql))
        .await
        .expect("1 回目の管理操作に失敗した");
    let second = execute_admin_op(&db.audit, &db.admin, sample_op(sql))
        .await
        .expect("2 回目の管理操作に失敗した");

    assert_ne!(
        first, second,
        "操作ごとに別々の intent 行（トークン）が発行される"
    );

    let rows = AuditRow::all(&db.owner).await;
    for (kind, want) in [("admin-op-intent", 2), ("admin-op-result", 2)] {
        let n = rows.iter().filter(|r| r.kind == kind).count();
        assert_eq!(n, want, "操作 2 回に対して {kind} は {want} 行");
    }
    let results: Vec<&AuditRow> = rows
        .iter()
        .filter(|r| r.kind == "admin-op-result")
        .collect();
    assert_eq!(
        results.iter().filter(|r| r.ref_id == Some(first)).count(),
        1,
        "1 行の intent 行に結果行は 1 行だけ（使い回しの痕跡がない）"
    );
    db.dispose().await;
}

/// AC5: `pelway_admin` は監査表に無権限。書き込みは `pelway_audit` の
/// SELECT + INSERT だけで、UPDATE・DELETE はどのロールにも無い。
#[tokio::test]
async fn admin_role_has_no_grant_on_audit_table() {
    let Some(db) = test_db().await else {
        skip();
        return;
    };

    async fn privilege(owner: &sqlx::PgPool, role: &str, perm: &str) -> bool {
        sqlx::query_scalar("SELECT has_table_privilege($1, 'admin_audit_events', $2)")
            .bind(role)
            .bind(perm)
            .fetch_one(owner)
            .await
            .expect("権限の問い合わせに失敗した")
    }

    // (ロール, 権限, あるべき値)。監査表は pelway_audit の SELECT + INSERT だけ。
    let expected = [
        ("pelway_admin", "select", false),
        ("pelway_admin", "insert", false),
        ("pelway_admin", "update", false),
        ("pelway_admin", "delete", false),
        ("pelway_app", "select", false),
        ("pelway_app", "insert", false),
        ("pelway_app", "update", false),
        ("pelway_app", "delete", false),
        ("pelway_audit", "select", true),  // 鎖頭の読み取り
        ("pelway_audit", "insert", true),  // 監査行の書き込み
        ("pelway_audit", "update", false), // append-only
        ("pelway_audit", "delete", false), // append-only
    ];
    for (role, perm, want) in expected {
        assert_eq!(
            privilege(&db.owner, role, perm).await,
            want,
            "{role} の {perm}"
        );
    }
    db.dispose().await;
}

/// AC6: 監査表の行は `prev_hash` 鎖でつながり、`pelway-audit-checkpoint` の
/// 歩査で検証できる（成功と失敗を混ぜた操作のあと）。
#[tokio::test]
async fn audit_rows_form_prev_hash_chain() {
    let Some(db) = test_db().await else {
        skip();
        return;
    };
    seed(&db.admin).await;
    let ok_sql =
        "INSERT INTO channels (workspace_id, id, name) VALUES ('ws_admin_audit', 'ch_c1', 'c1')";
    let fail_sql =
        "INSERT INTO channels (workspace_id, id, name) VALUES ('missing_ws', 'ch_c2', 'c2')";
    let ok_sql_2 =
        "INSERT INTO channels (workspace_id, id, name) VALUES ('ws_admin_audit', 'ch_c3', 'c3')";
    execute_admin_op(&db.audit, &db.admin, sample_op(ok_sql))
        .await
        .expect("1 回目（成功）に失敗した");
    let _ = execute_admin_op(&db.audit, &db.admin, sample_op(fail_sql)).await;
    execute_admin_op(&db.audit, &db.admin, sample_op(ok_sql_2))
        .await
        .expect("3 回目（成功）に失敗した");

    let rows = AuditRow::all(&db.owner).await;
    assert_eq!(rows.len(), 6, "操作 3 回で intent + result の 6 行");
    assert_eq!(
        rows[0].prev_hash,
        GENESIS.to_vec(),
        "最初の行は起点（GENESIS）から始まる"
    );
    let stored: Vec<StoredRow> = rows.iter().map(AuditRow::to_stored).collect();
    let heads = pelway_audit_checkpoint::chain::walk(&stored)
        .expect("鎖の歩査に失敗した（prev_hash の付き合わせか hash の再計算）");
    assert_eq!(heads.len(), stored.len());
    db.dispose().await;
}
