//! セッションの発行・照会・取り消し（#97 スパイク。メモリのみ、確定スキーマではない）。
//!
//! 12 章との照合:
//! - 認証通信と操作データの分離（12 §7、P06/P07 の行）: セッション トークンは
//!   不透明なランダム ID で、member の情報はサーバー側の `Session` にだけ置く。
//!   Cookie・JSON に平文の認証情報を埋めない。
//! - 資格情報取消で通常の利用を停止（12 §3.2、P07 の行）: `revoke_member` は
//!   その member のセッションをすべて消す。passkey の credential 削除や
//!   IdP 連携の解除と同じ扱いにする。
//! - Cookie は `HttpOnly; SameSite=Lax; Path=/`。`Secure` はループバック試作では
//!   付けない（本番では必須。未確認: https 化の時点で付ける）。

use std::collections::HashMap;
use std::sync::Mutex;

use axum::http::HeaderMap;
use webauthn_rs::prelude::Uuid;

/// Cookie 名。値は不透明なセッション トークン（`Uuid`）。
pub const SESSION_COOKIE: &str = "pelway_session";

/// ログインの経路。セッションがどの認証で作られたかを残す（監査の手がかり）。
#[derive(Clone, Debug)]
pub enum AuthMethod {
    /// passkey 認証。どの credential（=端末）でログインしたかを持つ。
    Passkey { credential_id: Vec<u8> },
    /// 顧客 IdP（OIDC）。tenant 名と IdP 側の subject を持つ。
    Oidc { tenant: String, subject: String },
}

/// 1 セッション。`revoked` フィールドは持たない（取り消し = 行の削除）。
#[derive(Clone, Debug)]
pub struct Session {
    pub member_id: Uuid,
    pub via: AuthMethod,
}

/// セッション トークンから `Session` へのメモリ内対応表。
///
/// `pelway-http` と同じく 1 つの `Mutex` で包む。1 要求の中で発行・照会・
/// 取り消しをロックを持ったまま行い、途中で別要求が割り込まない。
#[derive(Default)]
pub struct SessionStore {
    sessions: Mutex<HashMap<Uuid, Session>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// 新しいセッションを発行し、トークンを返す。
    pub fn issue(&self, member_id: Uuid, via: AuthMethod) -> Uuid {
        let token = Uuid::new_v4();
        self.sessions
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(token, Session { member_id, via });
        token
    }

    /// トークンに対応するセッションを返す。失効済み・未知なら `None`。
    pub fn lookup(&self, token: &Uuid) -> Option<Session> {
        self.sessions
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(token)
            .cloned()
    }

    /// 1 セッションを取り消す（ログアウト）。存在したかを返す。
    pub fn revoke(&self, token: &Uuid) -> bool {
        self.sessions
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(token)
            .is_some()
    }

    /// member の全セッションを取り消す（資格情報の取消に追従）。
    /// 消したセッション数を返す。
    pub fn revoke_member(&self, member_id: Uuid) -> usize {
        let mut sessions = self.sessions.lock().unwrap_or_else(|p| p.into_inner());
        let tokens: Vec<Uuid> = sessions
            .iter()
            .filter(|(_, s)| s.member_id == member_id)
            .map(|(t, _)| *t)
            .collect();
        for token in &tokens {
            sessions.remove(token);
        }
        tokens.len()
    }
}

/// `Set-Cookie` ヘッダー値を組み立てる。
pub fn session_set_cookie(token: Uuid) -> String {
    format!("{SESSION_COOKIE}={token}; HttpOnly; SameSite=Lax; Path=/")
}

/// 要求の `Cookie` ヘッダーからセッション トークンを取り出す。
/// 複数ヘッダー・前後の空白は受ける。値が `Uuid` でなければ `None`。
pub fn session_token(headers: &HeaderMap) -> Option<Uuid> {
    headers
        .get_all(axum::http::header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .find_map(|pair| {
            let (name, value) = pair.trim().split_once('=')?;
            (name.trim() == SESSION_COOKIE)
                .then(|| Uuid::parse_str(value.trim()).ok())
                .flatten()
        })
}
