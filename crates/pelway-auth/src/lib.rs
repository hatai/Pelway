//! 認証スパイク（#97 の分割 1 本目）。passkey（`webauthn-rs`）の登録・
//! ログインと、不透明セッション ID の発行・照会・取り消しを axum の
//! 1 サービスに組み込めるか確かめるための下書き。確定スキーマではない。
//! 顧客 IdP（`openidconnect`）でのログインはこのブランチに積む 2 本目の
//! PR で `oidc` モジュールとして足す（`session::AuthMethod::Oidc` の
//! 変種はその受け口として先に置く）。
//!
//! # 12 章（通信秘密・目的別根拠・可視範囲・運営アクセス）との照合
//!
//! | 12 章の要件 | この試作での扱い |
//! |---|---|
//! | 認証通信と操作データを分ける（P06/P07、§7） | セッションは不透明 ID、member 情報はサーバー側だけ |
//! | 資格情報取消で通常の利用を停止（§3.2、P07） | credential 取消 → `SessionStore::revoke_member` |
//! | 外部の権限管理サービスを必須にしない（§7） | IdP 製品を足さず axum に RP だけを組み込む |
//! | server が平文を扱う信頼モデルを明示（§7） | secret・ID トークンはサーバーが平文で扱う |
//!
//! # 試作の確認結果（#97）
//!
//! - passkey の登録・ログインは `Webauthn` の start/finish 4 API を axum の
//!   handler に薄く包むだけで成立した（進行状態はサーバー側 one-shot 保存）。
//! - 残課題（本番時）: credential counter の clone 検出、
//!   pending/セッションの永続化と期限切れ、Cookie `Secure`、RP ID/Origin の
//!   デプロイ別設定。
//!
//! 実外部 I/O は無い。テストは SoftPasskey をインメモリで接続する。

#![forbid(unsafe_code)]

pub mod passkey;
pub mod session;

use std::sync::Arc;

use axum::Router;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, post};
use serde::Serialize;
use serde_json::json;
use webauthn_rs::prelude::Uuid;
use webauthn_rs::{Webauthn, WebauthnBuilder};

use passkey::PasskeyStore;
use session::{SessionStore, session_set_cookie};

/// エラー応答。pelway-http と同じく `{"error": "<code>"}` の JSON を返す。
#[derive(Debug)]
pub struct ErrorBody {
    status: StatusCode,
    code: &'static str,
}

/// `ErrorBody` を作る短い書き方。
pub fn err(status: StatusCode, code: &'static str) -> ErrorBody {
    ErrorBody { status, code }
}

impl IntoResponse for ErrorBody {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.code }))).into_response()
    }
}

/// セッション発行の応答。`Set-Cookie` でセッション トークンを渡し、
/// 本文には member_id だけを載せる（トークンは Cookie だけに置く）。
pub struct SessionCookie {
    token: Uuid,
    member_id: Uuid,
}

impl SessionCookie {
    pub fn new(token: Uuid, member_id: Uuid) -> Self {
        Self { token, member_id }
    }
}

impl IntoResponse for SessionCookie {
    fn into_response(self) -> Response {
        (
            [(header::SET_COOKIE, session_set_cookie(self.token))],
            Json(json!({ "member_id": self.member_id })),
        )
            .into_response()
    }
}

/// ルーター全体で共有する状態。各 store は内部に `Mutex` を持つ。
///
/// スパイクのためメモリのみ。`pelway-http` と同じく poison したロックは
/// 中身を取り出して使い続ける（500 で落とさない判断は本番で再検討）。
pub struct AuthState {
    pub webauthn: Webauthn,
    pub sessions: SessionStore,
    pub passkeys: PasskeyStore,
}

impl AuthState {
    /// `rp_id`/`rp_origin`（例: `localhost`, `http://localhost:3000`）で
    /// passkey の RP を固定する。本番はデプロイごとの設定値。
    pub fn new(rp_id: &str, rp_origin: &str) -> Result<Self, webauthn_rs::prelude::WebauthnError> {
        let webauthn = WebauthnBuilder::new(
            rp_id,
            &webauthn_rs::prelude::Url::parse(rp_origin)
                .map_err(|_| webauthn_rs::prelude::WebauthnError::Configuration)?,
        )?
        .rp_name("Pelway")
        .build()?;
        Ok(Self {
            webauthn,
            sessions: SessionStore::new(),
            passkeys: PasskeyStore::new(),
        })
    }
}

/// 認証まわりのルート一式を持つ `Router`。
///
/// - `POST /auth/passkey/register/start` / `register/finish`
/// - `POST /auth/passkey/login/start` / `login/finish`
/// - `POST /auth/passkey/credentials/revoke`
/// - `POST /auth/logout`（Cookie のセッションを取消）
/// - `GET  /auth/whoami`（セッション照会の試作経路）
pub fn auth_router(state: Arc<AuthState>) -> Router {
    Router::new()
        .route(
            "/auth/passkey/register/start",
            post(passkey::register_start),
        )
        .route(
            "/auth/passkey/register/finish",
            post(passkey::register_finish),
        )
        .route("/auth/passkey/login/start", post(passkey::login_start))
        .route("/auth/passkey/login/finish", post(passkey::login_finish))
        .route(
            "/auth/passkey/credentials/revoke",
            post(passkey::revoke_credential),
        )
        .route("/auth/logout", post(logout))
        .route("/auth/whoami", get(whoami))
        .with_state(state)
}

/// `POST /auth/logout`: Cookie のセッションを取り消す。204 を返す。
async fn logout(
    axum::extract::State(state): axum::extract::State<Arc<AuthState>>,
    headers: axum::http::HeaderMap,
) -> StatusCode {
    if let Some(token) = session::session_token(&headers) {
        state.sessions.revoke(&token);
    }
    StatusCode::NO_CONTENT
}

#[derive(Serialize)]
struct Whoami {
    member_id: Uuid,
    via: &'static str,
}

/// `GET /auth/whoami`: セッションが生きているかの確認用（試作経路）。
async fn whoami(
    axum::extract::State(state): axum::extract::State<Arc<AuthState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Whoami>, ErrorBody> {
    let token = session::session_token(&headers)
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "no_session"))?;
    let session = state
        .sessions
        .lookup(&token)
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "no_session"))?;
    let via = match session.via {
        session::AuthMethod::Passkey { .. } => "passkey",
        session::AuthMethod::Oidc { .. } => "oidc",
    };
    Ok(Json(Whoami {
        member_id: session.member_id,
        via,
    }))
}
