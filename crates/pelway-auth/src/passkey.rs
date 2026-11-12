//! passkey（webauthn-rs）の登録・ログインを axum に組み込む試作（#97）。
//!
//! - セレモニーの進行状態（`PasskeyRegistration` / `PasskeyAuthentication`）は
//!   サーバー側のメモリに置き、finish で一度だけ取り出す（再利用させない）。
//!   start 応答には進行 ID と challenge 公開オプションだけを返す。
//! - member レコードは register_start では作らず register_finish で初めて作る
//!   （start だけ呼ばれる幽霊 member を残さない）。
//! - `webauthn-rs` の判定（RP ID / Origin / challenge / user verification の
//!   検査）に乗る。自前で署名を検証しない。
//! - counter の更新（`Passkey::update_credential`）と clone 検出は本番で要対応。
//!   メモリのみの試作では行わない。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Deserialize;
use serde_json::{Value, json};
use webauthn_rs::prelude::*;

use crate::session::AuthMethod;
use crate::{AuthState, ErrorBody, SessionCookie, err};

/// member 名と passkey credential、進行中セレモニーのメモリ内保存。
#[derive(Default)]
pub struct PasskeyStore {
    /// member_id -> (member_name, credentials)
    members: Mutex<HashMap<Uuid, MemberRecord>>,
    /// member_name -> member_id
    names: Mutex<HashMap<String, Uuid>>,
    /// pending_id -> 進行中のセレモニー
    pending: Mutex<HashMap<Uuid, Pending>>,
}

struct MemberRecord {
    credentials: Vec<Passkey>,
}

/// 進行中セレモニーのサーバー側状態。finish で取り出したら消える（one-shot）。
enum Pending {
    Register {
        member_id: Uuid,
        member_name: String,
        state: PasskeyRegistration,
    },
    Authenticate {
        member_id: Uuid,
        state: PasskeyAuthentication,
    },
}

impl PasskeyStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// テスト・管理照会用: member の credential 数を返す。
    pub fn credential_count(&self, member_id: Uuid) -> usize {
        self.members
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(&member_id)
            .map(|m| m.credentials.len())
            .unwrap_or(0)
    }
}

#[derive(Deserialize)]
pub struct RegisterStartBody {
    member_name: String,
    display_name: String,
}

/// `POST /auth/passkey/register/start`: challenge を発行し、進行状態を保存する。
pub async fn register_start(
    State(state): State<Arc<AuthState>>,
    Json(body): Json<RegisterStartBody>,
) -> Result<Json<Value>, ErrorBody> {
    if state
        .passkeys
        .names
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .contains_key(&body.member_name)
    {
        return Err(err(StatusCode::CONFLICT, "name_taken"));
    }
    let member_id = Uuid::new_v4();
    let (ccr, reg_state) = state
        .webauthn
        .start_passkey_registration(member_id, &body.member_name, &body.display_name, None)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "registration_rejected"))?;
    let pending_id = Uuid::new_v4();
    state
        .passkeys
        .pending
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .insert(
            pending_id,
            Pending::Register {
                member_id,
                member_name: body.member_name,
                state: reg_state,
            },
        );
    Ok(Json(json!({ "pending_id": pending_id, "options": ccr })))
}

#[derive(Deserialize)]
pub struct RegisterFinishBody {
    pending_id: Uuid,
    credential: RegisterPublicKeyCredential,
}

/// `POST /auth/passkey/register/finish`: 登録を確定し、member を作って
/// セッションを発行する。
pub async fn register_finish(
    State(state): State<Arc<AuthState>>,
    Json(body): Json<RegisterFinishBody>,
) -> Result<SessionCookie, ErrorBody> {
    let pending = state
        .passkeys
        .pending
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .remove(&body.pending_id);
    let Pending::Register {
        member_id,
        member_name,
        state: reg_state,
    } = pending.ok_or_else(|| err(StatusCode::BAD_REQUEST, "pending_not_found"))?
    else {
        return Err(err(StatusCode::BAD_REQUEST, "pending_kind_mismatch"));
    };
    let passkey = state
        .webauthn
        .finish_passkey_registration(&body.credential, &reg_state)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "registration_failed"))?;
    let credential_id = passkey.cred_id().to_vec();
    {
        let mut members = state
            .passkeys
            .members
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        members
            .entry(member_id)
            .or_insert(MemberRecord {
                credentials: Vec::new(),
            })
            .credentials
            .push(passkey);
        state
            .passkeys
            .names
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(member_name, member_id);
    }
    let token = state
        .sessions
        .issue(member_id, AuthMethod::Passkey { credential_id });
    Ok(SessionCookie::new(token, member_id))
}

#[derive(Deserialize)]
pub struct LoginStartBody {
    member_name: String,
}

/// `POST /auth/passkey/login/start`: member の credential に対する
/// 認証 challenge を発行する。
pub async fn login_start(
    State(state): State<Arc<AuthState>>,
    Json(body): Json<LoginStartBody>,
) -> Result<Json<Value>, ErrorBody> {
    let member_id = *state
        .passkeys
        .names
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .get(&body.member_name)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "unknown_member"))?;
    let credentials = state
        .passkeys
        .members
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .get(&member_id)
        .map(|m| m.credentials.clone())
        .unwrap_or_default();
    if credentials.is_empty() {
        return Err(err(StatusCode::NOT_FOUND, "no_credentials"));
    }
    let (rcr, auth_state) = state
        .webauthn
        .start_passkey_authentication(&credentials)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "authentication_rejected"))?;
    let pending_id = Uuid::new_v4();
    state
        .passkeys
        .pending
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .insert(
            pending_id,
            Pending::Authenticate {
                member_id,
                state: auth_state,
            },
        );
    Ok(Json(json!({ "pending_id": pending_id, "options": rcr })))
}

#[derive(Deserialize)]
pub struct LoginFinishBody {
    pending_id: Uuid,
    credential: PublicKeyCredential,
}

/// `POST /auth/passkey/login/finish`: assertion を検証し、セッションを発行する。
pub async fn login_finish(
    State(state): State<Arc<AuthState>>,
    Json(body): Json<LoginFinishBody>,
) -> Result<SessionCookie, ErrorBody> {
    let pending = state
        .passkeys
        .pending
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .remove(&body.pending_id);
    let Pending::Authenticate {
        member_id,
        state: auth_state,
    } = pending.ok_or_else(|| err(StatusCode::BAD_REQUEST, "pending_not_found"))?
    else {
        return Err(err(StatusCode::BAD_REQUEST, "pending_kind_mismatch"));
    };
    state
        .webauthn
        .finish_passkey_authentication(&body.credential, &auth_state)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "authentication_failed"))?;
    let token = state.sessions.issue(
        member_id,
        AuthMethod::Passkey {
            credential_id: body.credential.raw_id.to_vec(),
        },
    );
    Ok(SessionCookie::new(token, member_id))
}

#[derive(Deserialize)]
pub struct RevokeCredentialBody {
    member_id: Uuid,
    credential_id: Vec<u8>,
}

/// `POST /auth/passkey/credentials/revoke`: credential を取り消す。
/// 12 章「資格情報取消で通常の利用を停止」に合わせ、member のセッションを
/// すべて消す（対象 credential のセッションだけに絞る選択肢は本番で検討）。
pub async fn revoke_credential(
    State(state): State<Arc<AuthState>>,
    Json(body): Json<RevokeCredentialBody>,
) -> Result<Json<Value>, ErrorBody> {
    let removed = {
        let mut members = state
            .passkeys
            .members
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let record = members
            .get_mut(&body.member_id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "unknown_member"))?;
        let before = record.credentials.len();
        record
            .credentials
            .retain(|p| p.cred_id().as_slice() != body.credential_id.as_slice());
        record.credentials.len() != before
    };
    if !removed {
        return Err(err(StatusCode::NOT_FOUND, "unknown_credential"));
    }
    let revoked_sessions = state.sessions.revoke_member(body.member_id);
    Ok(Json(
        json!({ "member_id": body.member_id, "revoked_sessions": revoked_sessions }),
    ))
}
