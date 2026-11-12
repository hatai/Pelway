//! 顧客 IdP（Google Workspace / Microsoft Entra ID）でのログインを
//! openidconnect-rs で axum に組み込む試作（#97）。Pelway は Relying Party。
//!
//! - Authorization Code Flow + PKCE S256。`state`（CSRF 対策）と `nonce`
//!   （ID トークン再生対策）は openidconnect に発行させ、サーバー側の
//!   メモリに pending として置き、callback で一度だけ取り出す。
//! - テナント制限: Google は `hd`（hosted domain）、Entra は `tid`
//!   （tenant id）claim を `allowed_hd` / `allowed_tid` と照合する。
//!   顧客の Workspace/テナント外のアカウントは拒否する。
//! - バックチャネル（token endpoint / JWKS 取得）は `AsyncHttpClient` を実装する
//!   呼び出し可能な値を外部から受け取る。スパイクではインメモリの偽 IdP を
//!   接続し、本番は reqwest などの HTTPS クライアントに差し替える（未確認）。
//! - JWKS は callback のたびに取り直す。本番はキャッシュと kid ローテーション
//!   追従が要る（未対応）。

use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Redirect;
use serde::Deserialize;
use webauthn_rs::prelude::Uuid;

use openidconnect::core::{
    CoreAuthDisplay, CoreAuthPrompt, CoreAuthenticationFlow, CoreErrorResponseType,
    CoreGenderClaim, CoreJsonWebKey, CoreJsonWebKeySet, CoreJweContentEncryptionAlgorithm,
    CoreJwsSigningAlgorithm, CoreProviderMetadata, CoreResponseType, CoreRevocableToken,
    CoreRevocationErrorResponse, CoreSubjectIdentifierType, CoreTokenIntrospectionResponse,
    CoreTokenType,
};
use openidconnect::{
    AdditionalClaims, AuthorizationCode, Client, ClientId, ClientSecret, CsrfToken,
    EmptyAdditionalProviderMetadata, EmptyExtraTokenFields, EndpointMaybeSet, EndpointNotSet,
    EndpointSet, HttpRequest, HttpResponse, IdTokenFields, IssuerUrl, JsonWebKeySetUrl, Nonce,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, ResponseTypes, Scope, StandardErrorResponse,
    StandardTokenResponse,
};
use openidconnect::{AuthUrl, TokenUrl};

use crate::session::AuthMethod;
use crate::{AuthState, ErrorBody, SessionCookie, err};

/// Google `hd` / Entra `tid` を受け取る追加 claim。
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, serde::Serialize)]
pub struct TenantClaims {
    pub hd: Option<String>,
    pub tid: Option<String>,
}
impl AdditionalClaims for TenantClaims {}

pub type TenantIdTokenFields = IdTokenFields<
    TenantClaims,
    EmptyExtraTokenFields,
    CoreGenderClaim,
    CoreJweContentEncryptionAlgorithm,
    CoreJwsSigningAlgorithm,
>;
pub type TenantTokenResponse = StandardTokenResponse<TenantIdTokenFields, CoreTokenType>;

/// `from_provider_metadata` が返す型（auth・token 両 endpoint 設定済み）。
pub type TenantClient = Client<
    TenantClaims,
    CoreAuthDisplay,
    CoreGenderClaim,
    CoreJweContentEncryptionAlgorithm,
    CoreJsonWebKey,
    CoreAuthPrompt,
    StandardErrorResponse<CoreErrorResponseType>,
    TenantTokenResponse,
    CoreTokenIntrospectionResponse,
    CoreRevocableToken,
    CoreRevocationErrorResponse,
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

/// 1 顧客テナントの RP 設定。
#[derive(Clone, Debug)]
pub struct OidcTenant {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub jwks_uri: String,
    /// Pelway 側の callback URL（IdP に登録する redirect_uri と一致させる）。
    pub redirect_uri: String,
    /// 許可する Google Workspace の hosted domain（空 = `hd` を検査しない）。
    pub allowed_hd: Vec<String>,
    /// 許可する Entra tenant id（空 = `tid` を検査しない）。
    pub allowed_tid: Vec<String>,
}

/// バックチャネル HTTP の失敗（試作では文字列だけ）。
#[derive(Debug)]
pub struct BackchannelError(pub String);

impl fmt::Display for BackchannelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for BackchannelError {}

/// 注入する非同期 HTTP クライアント。`openidconnect::AsyncHttpClient` が
/// `Fn(HttpRequest) -> Future` に実装されているため、その形で渡す。
pub type Backchannel = Arc<
    dyn Fn(
            HttpRequest,
        ) -> Pin<Box<dyn Future<Output = Result<HttpResponse, BackchannelError>> + Send>>
        + Send
        + Sync,
>;

/// `Backchannel` を `openidconnect::AsyncHttpClient` に適合させるラッパー。
/// oauth2 crate 自身の `reqwest::Client` 実装と同じく、`call(&'c self)` が
/// 返す Future を `+ 'c` にするため、クロージャへの blanket impl ではなく
/// 名前つきの実装にする（closure 版は `for<'c>` 境界を満たせない）。
pub struct BackchannelClient(pub Backchannel);

impl<'c> openidconnect::AsyncHttpClient<'c> for BackchannelClient {
    type Error = BackchannelError;
    type Future = Pin<Box<dyn Future<Output = Result<HttpResponse, BackchannelError>> + Send + 'c>>;

    fn call(&'c self, request: HttpRequest) -> Self::Future {
        let inner = self.0.clone();
        Box::pin(async move { inner(request).await })
    }
}

/// tenant 設定、進行中のログイン、(tenant, subject) -> member の対応表。
pub struct OidcStore {
    pub tenants: HashMap<String, OidcTenant>,
    /// CSRF `state` -> 進行中ログイン
    pending: Mutex<HashMap<String, OidcPending>>,
    /// (tenant 名, IdP subject) -> member_id。初回ログインで作る。
    members: Mutex<HashMap<(String, String), Uuid>>,
}

struct OidcPending {
    tenant: String,
    nonce: Nonce,
    pkce_verifier: PkceCodeVerifier,
}

impl OidcStore {
    pub fn new(tenants: HashMap<String, OidcTenant>) -> Self {
        Self {
            tenants,
            pending: Mutex::new(HashMap::new()),
            members: Mutex::new(HashMap::new()),
        }
    }
}

fn tenant_client(tenant: &OidcTenant, jwks: CoreJsonWebKeySet) -> Result<TenantClient, ErrorBody> {
    let bad = |_: openidconnect::url::ParseError| {
        err(StatusCode::INTERNAL_SERVER_ERROR, "bad_tenant_urls")
    };
    let metadata = CoreProviderMetadata::new(
        IssuerUrl::new(tenant.issuer.clone()).map_err(bad)?,
        AuthUrl::new(tenant.authorization_endpoint.clone()).map_err(bad)?,
        JsonWebKeySetUrl::new(tenant.jwks_uri.clone()).map_err(bad)?,
        vec![ResponseTypes::new(vec![CoreResponseType::Code])],
        vec![CoreSubjectIdentifierType::Public],
        vec![CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256],
        EmptyAdditionalProviderMetadata {},
    )
    .set_token_endpoint(Some(
        TokenUrl::new(tenant.token_endpoint.clone()).map_err(bad)?,
    ))
    .set_jwks(jwks);
    Ok(TenantClient::from_provider_metadata(
        metadata,
        ClientId::new(tenant.client_id.clone()),
        tenant.client_secret.clone().map(ClientSecret::new),
    )
    .set_redirect_uri(RedirectUrl::new(tenant.redirect_uri.clone()).map_err(bad)?))
}

/// `GET /auth/oidc/{tenant}/start`: IdP の認可 endpoint へ 302 する。
/// state / nonce / PKCE verifier を pending に置く。
pub async fn login_start(
    State(state): State<Arc<AuthState>>,
    Path(tenant): Path<String>,
) -> Result<Redirect, ErrorBody> {
    let tenant_cfg = state
        .oidc
        .tenants
        .get(&tenant)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "unknown_tenant"))?
        .clone();
    let client = tenant_client(&tenant_cfg, CoreJsonWebKeySet::new(Vec::new()))?;
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let (url, csrf, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("email".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();
    state
        .oidc
        .pending
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .insert(
            csrf.secret().to_string(),
            OidcPending {
                tenant,
                nonce,
                pkce_verifier,
            },
        );
    Ok(Redirect::temporary(url.as_str()))
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: String,
    state: String,
}

/// `GET /auth/oidc/callback?code&state`: code を token に換え、ID トークンを
/// 検証（署名・iss・aud・exp・nonce）し、テナント制限を照合してセッションを発行。
pub async fn callback(
    State(state): State<Arc<AuthState>>,
    Query(query): Query<CallbackQuery>,
) -> Result<SessionCookie, ErrorBody> {
    let pending = state
        .oidc
        .pending
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .remove(&query.state)
        .ok_or_else(|| err(StatusCode::BAD_REQUEST, "state_not_found"))?;
    let tenant_cfg = state
        .oidc
        .tenants
        .get(&pending.tenant)
        .ok_or_else(|| err(StatusCode::INTERNAL_SERVER_ERROR, "tenant_removed"))?
        .clone();
    let http = BackchannelClient(state.oidc_http.clone());
    let jwks = CoreJsonWebKeySet::fetch_async(
        &JsonWebKeySetUrl::new(tenant_cfg.jwks_uri.clone())
            .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "bad_tenant_urls"))?,
        &http,
    )
    .await
    .map_err(|_| err(StatusCode::BAD_GATEWAY, "jwks_fetch_failed"))?;
    let client = tenant_client(&tenant_cfg, jwks)?;
    let token_response = client
        .exchange_code(AuthorizationCode::new(query.code))
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "no_token_endpoint"))?
        .set_pkce_verifier(pending.pkce_verifier)
        .request_async(&http)
        .await
        .map_err(|_| err(StatusCode::UNAUTHORIZED, "code_exchange_failed"))?;
    let id_token = token_response
        .extra_fields()
        .id_token()
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "no_id_token"))?;
    let claims = id_token
        .claims(&client.id_token_verifier(), &pending.nonce)
        .map_err(|_| err(StatusCode::UNAUTHORIZED, "id_token_rejected"))?;
    let extra = claims.additional_claims();
    if !tenant_cfg.allowed_hd.is_empty()
        && !extra
            .hd
            .as_ref()
            .is_some_and(|hd| tenant_cfg.allowed_hd.contains(hd))
    {
        return Err(err(StatusCode::FORBIDDEN, "hosted_domain_denied"));
    }
    if !tenant_cfg.allowed_tid.is_empty()
        && !extra
            .tid
            .as_ref()
            .is_some_and(|tid| tenant_cfg.allowed_tid.contains(tid))
    {
        return Err(err(StatusCode::FORBIDDEN, "tenant_id_denied"));
    }
    let subject = claims.subject().to_string();
    let member_id = {
        let mut members = state.oidc.members.lock().unwrap_or_else(|p| p.into_inner());
        *members
            .entry((pending.tenant.clone(), subject.clone()))
            .or_insert_with(Uuid::new_v4)
    };
    let token = state.sessions.issue(
        member_id,
        AuthMethod::Oidc {
            tenant: pending.tenant,
            subject,
        },
    );
    Ok(SessionCookie::new(token, member_id))
}

/// デバッグ・管理照合用: テナント名の一覧（順序は不定）。
pub fn tenant_names(store: &OidcStore) -> Vec<String> {
    store.tenants.keys().cloned().collect()
}
