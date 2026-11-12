//! #97 の受け入れ確認テスト。
//!
//! - passkey の登録とログインが axum の経路で完結する（SoftPasskey）。
//! - 顧客 IdP との Authorization Code Flow + PKCE がインメモリの偽 IdP 相手に
//!   完結し、`hd` / `tid` でテナントを制限できる。
//! - セッション発行・照会・取り消し（資格情報取消に追従）が動く。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Json;
use axum::routing::{get, post};
use serde_json::{Value, json};
use tower::ServiceExt;
use url::Url;
use webauthn_authenticator_rs::WebauthnAuthenticator;
use webauthn_authenticator_rs::softpasskey::SoftPasskey;
use webauthn_rs::prelude::{CreationChallengeResponse, RequestChallengeResponse};

use openidconnect::core::{
    CoreGenderClaim, CoreJsonWebKeySet, CoreJweContentEncryptionAlgorithm, CoreJwsSigningAlgorithm,
    CoreRsaPrivateSigningKey, CoreTokenType,
};
use openidconnect::{
    AccessToken, Audience, EmptyExtraTokenFields, HttpRequest, HttpResponse, IdToken,
    IdTokenClaims, IssuerUrl, JsonWebKeyId, Nonce, PrivateSigningKey, StandardClaims,
    SubjectIdentifier,
};
use pelway_auth::oidc::{Backchannel, BackchannelError, OidcStore, OidcTenant, TenantClaims};
use pelway_auth::{AuthState, auth_router};
use rsa::pkcs1::EncodeRsaPrivateKey;

const RP_ORIGIN: &str = "http://localhost";

/// テスト用の IdP。authorize の手続きは省略し、token endpoint が
/// 事前に控えた nonce / hd / tid を含む署名済み ID トークンを返す。
struct FakeIdp {
    key: CoreRsaPrivateSigningKey,
    issuer: String,
    client_id: String,
    subject: String,
    hd: Option<String>,
    tid: Option<String>,
    /// authorize URL から取り出した nonce（実 IdP が request で受け取るもの）。
    nonce: Mutex<Option<Nonce>>,
}

impl FakeIdp {
    fn new() -> Self {
        let rsa = rsa::RsaPrivateKey::new(&mut rand::thread_rng(), 2048)
            .expect("rsa keygen")
            .to_pkcs1_pem(rsa::pkcs1::LineEnding::LF)
            .expect("pem");
        Self {
            key: CoreRsaPrivateSigningKey::from_pem(
                rsa.as_str(),
                Some(JsonWebKeyId::new("k1".to_string())),
            )
            .expect("signing key"),
            issuer: "http://idp.test".to_string(),
            client_id: "pelway-rp".to_string(),
            subject: "idp-sub-1".to_string(),
            hd: Some("good.example.com".to_string()),
            tid: None,
            nonce: Mutex::new(None),
        }
    }

    fn router(idp: &Arc<Self>) -> Router {
        Router::new()
            .route(
                "/token",
                post({
                    let idp = idp.clone();
                    move || {
                        let idp = idp.clone();
                        async move {
                            let nonce = idp.nonce.lock().unwrap().take();
                            let claims = IdTokenClaims::new(
                                IssuerUrl::new(idp.issuer.clone()).unwrap(),
                                vec![Audience::new(idp.client_id.clone())],
                                chrono::Utc::now() + chrono::Duration::seconds(300),
                                chrono::Utc::now(),
                                StandardClaims::new(SubjectIdentifier::new(idp.subject.clone())),
                                TenantClaims {
                                    hd: idp.hd.clone(),
                                    tid: idp.tid.clone(),
                                },
                            )
                            .set_nonce(nonce);
                            let access = AccessToken::new("fake-access".to_string());
                            let id_token: IdToken<
                                _,
                                CoreGenderClaim,
                                CoreJweContentEncryptionAlgorithm,
                                CoreJwsSigningAlgorithm,
                            > = IdToken::new(
                                claims,
                                &idp.key,
                                CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256,
                                Some(&access),
                                None,
                            )
                            .unwrap();
                            let resp = pelway_auth::oidc::TenantTokenResponse::new(
                                access,
                                CoreTokenType::Bearer,
                                pelway_auth::oidc::TenantIdTokenFields::new(
                                    Some(id_token),
                                    EmptyExtraTokenFields {},
                                ),
                            );
                            Json(serde_json::to_value(resp).unwrap())
                        }
                    }
                }),
            )
            .route(
                "/jwks",
                get({
                    let idp = idp.clone();
                    move || {
                        let idp = idp.clone();
                        async move {
                            Json(
                                serde_json::to_value(CoreJsonWebKeySet::new(vec![
                                    idp.key.as_verification_key(),
                                ]))
                                .unwrap(),
                            )
                        }
                    }
                }),
            )
    }
}

/// バックチャネルを偽 IdP の axum Router に橋渡しする（TCP も TLS も使わない）。
fn bridge(router: Router) -> Backchannel {
    Arc::new(move |req: HttpRequest| {
        let router = router.clone();
        Box::pin(async move {
            let (parts, body) = req.into_parts();
            let response = router
                .oneshot(Request::from_parts(parts, Body::from(body)))
                .await
                .map_err(|e| BackchannelError(format!("router: {e:?}")))?;
            let (parts, body) = response.into_parts();
            let bytes = axum::body::to_bytes(body, usize::MAX)
                .await
                .map_err(|e| BackchannelError(e.to_string()))?;
            Ok(HttpResponse::from_parts(parts, bytes.to_vec()))
        })
    })
}

struct TestApp {
    app: Router,
    idp: Arc<FakeIdp>,
}

fn test_app() -> TestApp {
    let idp = Arc::new(FakeIdp::new());
    let tenant = OidcTenant {
        client_id: idp.client_id.clone(),
        client_secret: Some("secret".to_string()),
        issuer: idp.issuer.clone(),
        authorization_endpoint: format!("{}/authorize", idp.issuer),
        token_endpoint: format!("{}/token", idp.issuer),
        jwks_uri: format!("{}/jwks", idp.issuer),
        redirect_uri: format!("{}/auth/oidc/callback", RP_ORIGIN),
        allowed_hd: vec!["good.example.com".to_string()],
        allowed_tid: Vec::new(),
    };
    let oidc = OidcStore::new(HashMap::from([("google".to_string(), tenant)])).unwrap();
    let state = Arc::new(
        AuthState::new("localhost", RP_ORIGIN, oidc, bridge(FakeIdp::router(&idp)))
            .expect("auth state"),
    );
    TestApp {
        app: auth_router(state),
        idp,
    }
}

async fn call(app: &Router, req: Request<Body>) -> (StatusCode, axum::http::HeaderMap, Value) {
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let headers = resp.headers().clone();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, headers, json)
}

fn post_json(path: &str, body: Value) -> Request<Body> {
    Request::post(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn post_json_auth(path: &str, body: Value, cookie: &str) -> Request<Body> {
    Request::post(path)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, cookie)
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn set_cookie(headers: &axum::http::HeaderMap) -> String {
    headers
        .get(header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next().map(str::to_string))
        .expect("Set-Cookie")
}

async fn whoami_status(app: &Router, cookie: &str) -> StatusCode {
    app.clone()
        .oneshot(
            Request::get("/auth/whoami")
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn passkey_register_and_login_issues_session() {
    let t = test_app();
    let mut device = SoftPasskey::new(true);
    let origin = Url::parse(RP_ORIGIN).unwrap();

    // 登録
    let (status, _, body) = call(
        &t.app,
        post_json(
            "/auth/passkey/register/start",
            json!({"member_name": "akari", "display_name": "Akari"}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "register/start: {body}");
    let pending_id = body["pending_id"].as_str().unwrap().to_string();
    let ccr: CreationChallengeResponse = serde_json::from_value(body["options"].clone()).unwrap();
    let credential = device.do_registration(origin.clone(), ccr).unwrap();

    let (status, headers, body) = call(
        &t.app,
        post_json(
            "/auth/passkey/register/finish",
            json!({"pending_id": pending_id, "credential": credential}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "register/finish: {body}");
    let cookie = set_cookie(&headers);
    assert_eq!(whoami_status(&t.app, &cookie).await, StatusCode::OK);

    // ログアウトするとセッションは死ぬ
    let (status, _, _) = call(
        &t.app,
        Request::post("/auth/logout")
            .header(header::COOKIE, &cookie)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(
        whoami_status(&t.app, &cookie).await,
        StatusCode::UNAUTHORIZED
    );

    // ログイン
    let (status, _, body) = call(
        &t.app,
        post_json("/auth/passkey/login/start", json!({"member_name": "akari"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "login/start: {body}");
    let pending_id = body["pending_id"].as_str().unwrap().to_string();
    let rcr: RequestChallengeResponse = serde_json::from_value(body["options"].clone()).unwrap();
    let assertion = device.do_authentication(origin, rcr).unwrap();

    let (status, headers, body) = call(
        &t.app,
        post_json(
            "/auth/passkey/login/finish",
            json!({"pending_id": pending_id, "credential": assertion}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "login/finish: {body}");
    assert_eq!(
        whoami_status(&t.app, &set_cookie(&headers)).await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn credential_revocation_stops_sessions() {
    let t = test_app();
    let mut device = SoftPasskey::new(true);
    let origin = Url::parse(RP_ORIGIN).unwrap();

    let (_, _, body) = call(
        &t.app,
        post_json(
            "/auth/passkey/register/start",
            json!({"member_name": "baku", "display_name": "Baku"}),
        ),
    )
    .await;
    let ccr: CreationChallengeResponse = serde_json::from_value(body["options"].clone()).unwrap();
    let credential = device.do_registration(origin, ccr).unwrap();
    let cred_id = credential.raw_id.clone();
    let pending_id = body["pending_id"].as_str().unwrap().to_string();

    let (_, headers, _) = call(
        &t.app,
        post_json(
            "/auth/passkey/register/finish",
            json!({"pending_id": pending_id, "credential": credential}),
        ),
    )
    .await;
    let cookie = set_cookie(&headers);

    // 未認証の呼び出しは 401（member はセッション Cookie から特定する）
    let (status, _, _) = call(
        &t.app,
        post_json(
            "/auth/passkey/credentials/revoke",
            json!({"credential_id": cred_id}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // 本人のセッションで credential を取消すとセッションが止まる
    // （12 章: 資格情報取消で通常の利用を停止）
    let (status, _, _) = call(
        &t.app,
        post_json_auth(
            "/auth/passkey/credentials/revoke",
            json!({"credential_id": cred_id}),
            &cookie,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        whoami_status(&t.app, &cookie).await,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn oidc_login_with_allowed_domain_issues_session() {
    let t = test_app();

    // /start は IdP へ 302 する
    let resp = t
        .app
        .clone()
        .oneshot(
            Request::get("/auth/oidc/google/start")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::TEMPORARY_REDIRECT);
    let location = Url::parse(resp.headers()[header::LOCATION].to_str().unwrap()).unwrap();
    let params: HashMap<_, _> = location.query_pairs().into_owned().collect();
    assert_eq!(params.get("client_id").unwrap(), "pelway-rp");
    assert_eq!(params.get("code_challenge_method").unwrap(), "S256");

    // 実 IdP は authorize request の nonce を ID トークンへ入れる
    *t.idp.nonce.lock().unwrap() = Some(Nonce::new(params["nonce"].clone()));
    let state = params["state"].clone();

    let (status, headers, body) = call(
        &t.app,
        Request::get(format!("/auth/oidc/callback?code=fake-code&state={state}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "callback: {body}");
    assert_eq!(
        whoami_status(&t.app, &set_cookie(&headers)).await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn oidc_callback_rejects_replayed_state_and_bad_domain() {
    let mut t = test_app();
    t.idp = Arc::new({
        let mut idp = FakeIdp::new();
        idp.hd = Some("evil.example.com".to_string());
        idp
    });
    // 偽 IdP を作り直したので bridge も張り替える
    let tenant = t_oidc_tenant(&t.idp);
    let oidc = OidcStore::new(HashMap::from([("google".to_string(), tenant)])).unwrap();
    let state = Arc::new(
        AuthState::new(
            "localhost",
            RP_ORIGIN,
            oidc,
            bridge(FakeIdp::router(&t.idp)),
        )
        .expect("auth state"),
    );
    t.app = auth_router(state);

    let resp = t
        .app
        .clone()
        .oneshot(
            Request::get("/auth/oidc/google/start")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let location = Url::parse(resp.headers()[header::LOCATION].to_str().unwrap()).unwrap();
    let params: HashMap<_, _> = location.query_pairs().into_owned().collect();
    *t.idp.nonce.lock().unwrap() = Some(Nonce::new(params["nonce"].clone()));
    let state = params["state"].clone();

    // 許可していない hosted domain は 403
    let (status, _, body) = call(
        &t.app,
        Request::get(format!("/auth/oidc/callback?code=fake-code&state={state}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "callback: {body}");
    assert_eq!(body["error"], "hosted_domain_denied");

    // 使い回した state は one-shot 消費で 400
    let (status, _, _) = call(
        &t.app,
        Request::get(format!("/auth/oidc/callback?code=fake-code&state={state}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[test]
fn oidc_store_rejects_tenant_without_allow_lists() {
    let mut tenant = t_oidc_tenant(&Arc::new(FakeIdp::new()));
    tenant.allowed_hd = Vec::new();
    tenant.allowed_tid = Vec::new();
    assert!(
        OidcStore::new(HashMap::from([("google".to_string(), tenant)])).is_err(),
        "empty allow lists must be a configuration error"
    );
}

fn t_oidc_tenant(idp: &Arc<FakeIdp>) -> OidcTenant {
    OidcTenant {
        client_id: idp.client_id.clone(),
        client_secret: Some("secret".to_string()),
        issuer: idp.issuer.clone(),
        authorization_endpoint: format!("{}/authorize", idp.issuer),
        token_endpoint: format!("{}/token", idp.issuer),
        jwks_uri: format!("{}/jwks", idp.issuer),
        redirect_uri: format!("{}/auth/oidc/callback", RP_ORIGIN),
        allowed_hd: vec!["good.example.com".to_string()],
        allowed_tid: Vec::new(),
    }
}
