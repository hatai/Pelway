//! #97（分割 1 本目）の受け入れ確認テスト。
//!
//! - passkey の登録とログインが axum の経路で完結する（SoftPasskey）。
//! - セッション発行・照会・取り消し（資格情報取消に追従）が動く。
//! - 顧客 IdP ログインの確認は 2 本目の PR のテストが担う。

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;
use url::Url;
use webauthn_authenticator_rs::WebauthnAuthenticator;
use webauthn_authenticator_rs::softpasskey::SoftPasskey;
use webauthn_rs::prelude::{CreationChallengeResponse, RequestChallengeResponse};

use pelway_auth::{AuthState, auth_router};

const RP_ORIGIN: &str = "http://localhost";

struct TestApp {
    app: Router,
}

fn test_app() -> TestApp {
    let state = Arc::new(AuthState::new("localhost", RP_ORIGIN).expect("auth state"));
    TestApp {
        app: auth_router(state),
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
