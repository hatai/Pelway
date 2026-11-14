//! 管理操作の専用経路（#149、ADR 0005 の「回避ロールの利用をすべて監査する」）。
//!
//! # 守ること（このモジュールが表す不変条件）
//!
//! - テナント表への管理操作（[`crate::ADMIN_ROLE`]（BYPASSRLS）で実行する読み書き）は
//!   この経路を通す。経路は操作の前に、実行者・対象・理由を記した intent 行を
//!   操作とは**別のトランザクション**で監査表（`admin_audit_events`）に確定する
//!   （確定できないときは操作を実行しない = fail-closed。Vault の監査デバイスと
//!   同じモデル）。操作の後には result 行（成功・失敗）を追記する。監査行は
//!   append-only で `prev_hash` 鎖（ADR 0007 の形式・単一鎖）につなぎ、削除・
//!   改竄は署名付きチェックポイント（ADR 0007）で検出する。
//! - 監査表への権限は [`AUDIT_ROLE`] の SELECT + INSERT だけ（[`crate::ADMIN_ROLE`] と
//!   [`crate::APP_ROLE`] は無権限のため、管理ロールで動く SQL で監査行を消したり
//!   書き換えたりできない）。intent 行は使い捨てで、操作は「その id の intent 行に
//!   result 行がまだ無い」ことを確認してから実行する（使い回しを許さない）。
//! - [`crate::admin_pool_options`] を使ってよいのはこの経路だけ（規範。資格情報と
//!   ネットワークの面での遮断は別 Issue。経路の外は防止できず検出のみ）。
//!
//! # テスト
//!
//! 結合テスト（`tests/admin_audit.rs`）には PostgreSQL が必要で、
//! `PELWAY_DATABASE_URL` が未設定の環境（CI を含む）では各テストは飛ぶ。

use pelway_audit_checkpoint::chain::{self, AuditEvent, GENESIS};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

/// 監査表への書き込みだけを行うロール（SELECT + INSERT のみ。UPDATE・DELETE は誰にも
/// 付与しない）。マイグレーション 0004 が作る。
pub const AUDIT_ROLE: &str = "pelway_audit";

/// 管理監査の鎖の単位（`workspace_id` の固定値。workspace ごとの鎖とは別の単一鎖）。
pub const AUDIT_CHAIN_ID: &str = "admin";

/// intent 行（操作の前に確定）の kind。
const KIND_INTENT: &str = "admin-op-intent";
/// result 行（操作の後に追記）の kind。
const KIND_RESULT: &str = "admin-op-result";
/// intent 行の result の値（結果はまだ出ていない）。
const RESULT_PENDING: &str = "pending";
/// 監査書き込みの直列化（鎖の分岐の防止）の advisory lock ID（0001_roles.sql と別）。
const AUDIT_WRITE_LOCK: i64 = 4210101;

/// [`AUDIT_ROLE`] で動くプールのオプション。専用経路の監査用接続（intent・result
/// 行の書き込みと鎖頭の読み取りだけ。接続ユーザーはメンバーであること）。
#[must_use]
pub fn audit_pool_options() -> PgPoolOptions {
    PgPoolOptions::new().after_connect(|conn, _| {
        Box::pin(async move {
            // SET ROLE はバインドパラメータを受け付けない（埋め込むのは定数だけ）。
            sqlx::query(sqlx::AssertSqlSafe(
                format!("SET ROLE {AUDIT_ROLE}").as_str(),
            ))
            .execute(conn)
            .await?;
            Ok(())
        })
    })
}

/// 管理操作の依頼。
#[derive(Debug, Clone)]
pub struct AdminOp {
    /// 実行者（監査行の actor）。呼び出し側が識別できる形で渡す。
    pub operator: String,
    /// 対象（監査行の subject）。
    pub subject: String,
    /// 理由（監査行の reason）。空ではならない（理由のない管理操作は拒否する）。
    pub reason: String,
    /// 実行する SQL。複数文可（1 つの暗黙トランザクション。失敗すれば全体が
    /// ロールバックされる）。
    pub sql: String,
}

/// 管理操作の実行結果（使い捨みトークン = intent 行の id。操作と 1:1 で対応する）。
pub type AdminOpResult = i64;

/// 専用経路の失敗。
#[derive(Debug)]
pub enum AdminOpError {
    /// 依頼の内容が不正（理由が空など）。操作も監査行も書いていない。
    Invalid(String),
    /// 監査行の確定に失敗した。操作は実行していない（fail-closed）。ただし操作の
    /// 成功後に result 行の追記が失敗したときも同様（その場合は効果が出ている）。
    AuditWrite(sqlx::Error),
    /// 管理操作の SQL が失敗した。intent・result の監査行は残っている。
    Operation(sqlx::Error),
}

impl std::fmt::Display for AdminOpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(msg) => write!(f, "管理操作の依頼が不正: {msg}"),
            Self::AuditWrite(err) => write!(f, "監査行の確定に失敗（操作は実行しない）: {err}"),
            Self::Operation(err) => write!(f, "管理操作の SQL が失敗（監査行は残っている）: {err}"),
        }
    }
}

/// 管理操作を監査付きで実行する。`audit` は [`audit_pool_options`] のプール
/// （監査用接続）、`admin` は [`crate::admin_pool_options`] のプール（管理用接続）。
///
/// 流れ: (1) 監査用接続で intent 行を操作とは別のトランザクションで確定する
/// （失敗なら [`AdminOpError::AuditWrite`] を返し、操作は実行しない）→
/// (2) intent 行が未使用であることを確認する → (3) 管理用接続で SQL を実行する →
/// (4) 監査用接続で result 行（ok または error:…）を追記する。
///
/// # Errors
///
/// 不正な依頼は [`AdminOpError::Invalid`]、監査書き込みの失敗は
/// [`AdminOpError::AuditWrite`]、操作の失敗は [`AdminOpError::Operation`]。
pub async fn execute_admin_op(
    audit: &PgPool,
    admin: &PgPool,
    op: AdminOp,
) -> Result<AdminOpResult, AdminOpError> {
    for (name, value) in [
        ("operator", &op.operator),
        ("subject", &op.subject),
        ("reason", &op.reason),
        ("sql", &op.sql),
    ] {
        if value.trim().is_empty() {
            return Err(AdminOpError::Invalid(format!(
                "{name} が空（理由のない管理操作は拒否する）"
            )));
        }
    }

    let snapshot = snapshot_hash(&op.sql);
    let intent_id = insert_event(
        audit,
        &event(&op, KIND_INTENT, RESULT_PENDING, snapshot, None),
        None,
    )
    .await
    .map_err(AdminOpError::AuditWrite)?;
    let intent_ref = u64::try_from(intent_id).expect("intent 行の id は正");

    // 使い捨ての確認: この intent 行を参照する result 行が既にあれば拒否する。
    let used: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM admin_audit_events WHERE ref_id = $1)")
            .bind(intent_id)
            .fetch_one(audit)
            .await
            .map_err(AdminOpError::AuditWrite)?;
    if used {
        return Err(AdminOpError::Invalid(format!(
            "intent 行 {intent_id} は既に使われている"
        )));
    }

    // 管理操作の SQL は動的な文字列（経路の目的が「理由を記録したうえでの任意
    // SQL の実行」）。外部からの入力は混ざらず、呼び出し側（運用ツール）が組んだ
    // SQL を実行し、snapshot_hash として監査行に記録する。
    let executed = sqlx::raw_sql(sqlx::AssertSqlSafe(op.sql.clone()))
        .execute(admin)
        .await;
    match executed {
        Ok(_) => {
            insert_event(
                audit,
                &event(&op, KIND_RESULT, "ok", snapshot, Some(intent_ref)),
                Some(intent_id),
            )
            .await
            .map_err(AdminOpError::AuditWrite)?;
            Ok(intent_id)
        }
        Err(err) => {
            let result_text = format!("error:{err}");
            insert_event(
                audit,
                &event(&op, KIND_RESULT, &result_text, snapshot, Some(intent_ref)),
                Some(intent_id),
            )
            .await
            .map_err(AdminOpError::AuditWrite)?;
            Err(AdminOpError::Operation(err))
        }
    }
}

/// 監査表に 1 行を書き、鎖の直近につなげる。advisory lock で書き込みを直列化
/// （鎖の分岐の防止）。トランザクションは呼び出しごとに独立（コミットして返す）。
async fn insert_event(pool: &PgPool, event: &AuditEvent, ref_id: Option<i64>) -> sqlx::Result<i64> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(AUDIT_WRITE_LOCK)
        .execute(&mut *tx)
        .await?;
    let head: Option<Vec<u8>> =
        sqlx::query_scalar("SELECT hash FROM admin_audit_events ORDER BY id DESC LIMIT 1")
            .fetch_optional(&mut *tx)
            .await?;
    let prev: [u8; 32] = match head {
        Some(bytes) => bytes.try_into().expect("監査行の hash は 32 バイト"),
        None => GENESIS,
    };
    let hash = chain::row_hash(event, &prev);
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO admin_audit_events \
         (workspace_id, at_ms, actor, subject, reason, kind, result, snapshot_hash, \
          policy_version, prev_hash, hash, ref_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) RETURNING id",
    )
    .bind(&event.workspace_id)
    .bind(i64::try_from(event.at_ms).expect("at_ms は i64 に収まる"))
    .bind(&event.actor)
    .bind(&event.subject)
    // 管理監査の行には必ず理由がある（reason 列は NOT NULL）。
    .bind(event.reason.as_deref().expect("管理監査行の reason は必須"))
    .bind(&event.kind)
    .bind(&event.result)
    .bind(event.snapshot_hash.as_slice())
    .bind(i64::try_from(event.policy_version).expect("policy_version は i64 に収まる"))
    .bind(prev.as_slice())
    .bind(hash.as_slice())
    .bind(ref_id)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(id)
}

/// 依頼から監査行の内容を作る。`policy_version` は常に 0（管理操作に承認の
/// policy は無い）。result 行の `ref_id` 参照もハッシュの対象（付け替えの検出）。
fn event(
    op: &AdminOp,
    kind: &str,
    result: &str,
    snapshot: [u8; 32],
    ref_id: Option<u64>,
) -> AuditEvent {
    AuditEvent {
        workspace_id: AUDIT_CHAIN_ID.to_owned(),
        // 監査行の日時（エポックからのミリ秒）。
        at_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("システム時刻はエポック以降")
            .as_millis() as u64,
        actor: op.operator.clone(),
        subject: op.subject.clone(),
        reason: Some(op.reason.clone()),
        kind: kind.to_owned(),
        result: result.to_owned(),
        snapshot_hash: snapshot,
        policy_version: 0,
        ref_id,
    }
}

/// 実行する SQL の snapshot hash（操作の中身の指紋。#87 と同じ考え方）。
fn snapshot_hash(sql: &str) -> [u8; 32] {
    Sha256::digest(sql.as_bytes()).into()
}
