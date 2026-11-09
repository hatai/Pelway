//! QA-GDPR-02 の仮経路（根拠記録・利用方針）の検証（QA 用）。
//!
//! - `127.0.0.1:0` で起動したサーバーに `std::net::TcpStream` で実際に HTTP/1.1
//!   要求を送る（接続先はループバックだけ。外部への接続は無い）。
//! - 仮の経路:
//!   - `POST /v1/workspaces/{workspace_id}/communication-basis-records`（作成）
//!   - `GET  /v1/workspaces/{workspace_id}/communication-basis-records?actor=...`（読取）
//!   - `POST /v1/workspaces/{workspace_id}/communication-basis-records/{basis_id}/withdraw`
//!     （撤回 = 取得と同じ1操作）
//!   - `POST /v1/workspaces/{workspace_id}/communication-use-policies`（作成）
//!   - `GET  /v1/workspaces/{workspace_id}/communication-use-policies?actor=...`（読取）
//! - 合成の利用者 ID（actor）は引数（要求本文・クエリ）で渡す（開発リードの仮の
//!   決め方 2026-11-10。ヘッダーや環境から推定しない）。
//! - ログ（07 の仕組み）に出るのは仮名化 ID・種別・数値だけ。本文・参照先の中身・
//!   `client_msg_id` はログに渡さない。

use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};

use pelway_contract::ids::WorkspaceId;
use pelway_contract::ledger::ApprovalLedger;
use pelway_contract::pseudonym::pseudonymize;
use pelway_contract::store::DataStore;
use serde_json::{Value, json};

use crate::log::{LogSink, Logger, MemorySink};
use crate::{SharedDataStore, SharedLedger, serve_with_store_and_logger};

/// 本文用の合成 canary（07 の検証と同じ値。実在の値ではない）。
const CANARY_BODY: &str = "PELWAY-CANARY-BODY-7Q2M9X";
/// `client_msg_id` 用の合成 canary（UUIDv4 の形の合成値）。
const CANARY_MSG_ID: &str = "3f6c2a9e-8b1d-4c7a-9e2f-5a1b7c3d9e04";
/// テストで使う固定 salt（作り方自体は log.rs の単体テスト (s3)(s4) で確認済み）。
const SALT: &[u8] = b"gdpr02-routes-test-salt";

const WS: &str = "ws-synthetic-gdpr02";
const ACTOR: &str = "m-human-gdpr02";

/// メモリ出力・既知 salt のロガーと保存部を持つサーバーを `127.0.0.1:0` で立てる。
fn spawn_logged(sink: &Arc<MemorySink>) -> (SocketAddr, SharedDataStore) {
    let ledger: SharedLedger = Arc::new(Mutex::new(ApprovalLedger::new()));
    let store: SharedDataStore = Arc::new(Mutex::new(DataStore::new()));
    let logger = Arc::new(Logger::with_sink(
        Arc::clone(sink) as Arc<dyn LogSink>,
        SALT.to_vec(),
    ));
    let addr = serve_with_store_and_logger(
        "127.0.0.1:0".parse().unwrap(),
        ledger,
        Arc::clone(&store),
        logger,
    )
    .expect("serve on 127.0.0.1:0")
    .local_addr();
    assert_eq!(addr.ip().to_string(), "127.0.0.1");
    (addr, store)
}

/// 生の HTTP/1.1 要求を 1 回送り、(状態コード, 本文) を返す。接続先はループバックだけ。
fn http(
    addr: SocketAddr,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> (u16, String) {
    let mut stream = TcpStream::connect(addr).expect("connect");
    let mut req = format!("{method} {path} HTTP/1.1\r\nHost: demo.local\r\nConnection: close\r\n");
    for (k, v) in headers {
        req.push_str(&format!("{k}: {v}\r\n"));
    }
    req.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
    stream.write_all(req.as_bytes()).expect("write head");
    stream.write_all(body).expect("write body");
    let _ = stream.shutdown(Shutdown::Write);
    let mut buf = String::new();
    stream.read_to_string(&mut buf).expect("read response");
    let status = buf
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("status line");
    let body = buf.split("\r\n\r\n").nth(1).unwrap_or("").to_owned();
    (status, body)
}

/// 根拠記録作成の要求 JSON（`overrides` はテストから差し替える項目）。
fn basis_request(overrides: Value) -> String {
    let mut v = json!({
        "actor": ACTOR,
        "basis_type": "synthetic_consent",
        "purpose_id": "p03",
        "scope": "ch-synthetic-02",
        "party_coverage_ref": "ref:party-coverage-1",
        "grantor_ref": "ref:grantor-1",
        "representative_authority_ref": null,
        "explanation_version": "expl-v1",
        "evidence_ref": "ref:evidence-1",
    });
    for (k, new) in overrides.as_object().into_iter().flatten() {
        v[k] = new.clone();
    }
    v.to_string()
}

fn post_basis(addr: SocketAddr, body: &str) -> (u16, String) {
    http(
        addr,
        "POST",
        &format!("/v1/workspaces/{WS}/communication-basis-records"),
        &[("Content-Type", "application/json")],
        body.as_bytes(),
    )
}

fn get_basis_list(addr: SocketAddr) -> (u16, String) {
    http(
        addr,
        "GET",
        &format!("/v1/workspaces/{WS}/communication-basis-records?actor={ACTOR}"),
        &[],
        b"",
    )
}

fn post_withdraw(addr: SocketAddr, basis_id: &str, body: &str) -> (u16, String) {
    http(
        addr,
        "POST",
        &format!("/v1/workspaces/{WS}/communication-basis-records/{basis_id}/withdraw"),
        &[("Content-Type", "application/json")],
        body.as_bytes(),
    )
}

/// ログ行を JSON にパースし、`kind`（出来事の種別）を返す。
fn kinds(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .map(|l| {
            let v: Value = serde_json::from_str(l).expect("each log line is JSON");
            assert!(v["ts_ms"].is_u64(), "ts_ms must be a number: {l}");
            v["kind"].as_str().expect("kind is a string").to_owned()
        })
        .collect()
}

// テスト1: 合成 workspace で、目的 P01〜P08 それぞれに根拠レコードが作れて読める。
#[test]
fn http_creates_and_lists_basis_records_for_all_purposes() {
    let sink = Arc::new(MemorySink::new());
    let (addr, store) = spawn_logged(&sink);

    for (i, purpose) in ["p01", "p02", "p03", "p04", "p05", "p06", "p07", "p08"]
        .into_iter()
        .enumerate()
    {
        let body = basis_request(json!({ "purpose_id": purpose }));
        let (status, resp) = post_basis(addr, &body);
        assert_eq!(status, 200, "{purpose}: {resp}");
        let v: Value = serde_json::from_str(&resp).expect("json");
        assert_eq!(v["purpose_id"], purpose);
        assert_eq!(
            v["basis_id"],
            format!("bas-{:08}", i + 1),
            "ID はサーバー採番"
        );
        assert_eq!(v["status"], "pending");
        assert_eq!(v["epoch"], 1);
    }

    // 読み取り（actor は引数 = クエリ）。
    let (status, resp) = get_basis_list(addr);
    assert_eq!(status, 200, "{resp}");
    let arr: Vec<Value> = serde_json::from_str(&resp).expect("json array");
    assert_eq!(arr.len(), 8);
    assert_eq!(arr[0]["basis_id"], "bas-00000001");
    assert_eq!(arr[7]["basis_id"], "bas-00000008");

    // 保存部にも同じ 8 件（正本はサーバー値）。
    let guard = store.lock().expect("store lock");
    assert_eq!(guard.basis_records(&WorkspaceId::from(WS)).len(), 8);
}

// テスト3: 偽の時刻・ID を送っても、保存された正本はサーバーの値になる。
#[test]
fn spoofed_ids_and_timestamps_are_rejected_and_server_values_win() {
    let sink = Arc::new(MemorySink::new());
    let (addr, store) = spawn_logged(&sink);

    // 偽の basis_id・版・時刻・status・epoch を送る → 拒否（未知フィールド）。
    for spoofed in [
        json!({ "basis_id": "bas-99999999" }),
        json!({ "basis_version": 42 }),
        json!({ "collected_at": 1 }),
        json!({ "effective_at": 1 }),
        json!({ "withdrawn_at": 1 }),
        json!({ "status": "active" }),
        json!({ "epoch": 99 }),
        json!({ "basis_id": "bas-99999999", "collected_at": 1, "epoch": 99 }),
    ] {
        let body = basis_request(spoofed);
        let (status, resp) = post_basis(addr, &body);
        assert_eq!(status, 400, "偽の ID・時刻が受理されている: {resp}");
    }
    // 偽の値は 1 件も保存されていない。
    {
        let guard = store.lock().expect("store lock");
        assert!(guard.basis_records(&WorkspaceId::from(WS)).is_empty());
    }

    // 通常の作成はサーバー値だけが正本になる。
    let before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("now")
        .as_millis() as u64;
    let (status, resp) = post_basis(addr, &basis_request(json!({})));
    let after = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("now")
        .as_millis() as u64;
    assert_eq!(status, 200, "{resp}");
    let v: Value = serde_json::from_str(&resp).expect("json");
    assert_eq!(v["basis_id"], "bas-00000001");
    assert_eq!(v["status"], "pending");
    assert_eq!(v["epoch"], 1);
    let collected = v["collected_at"].as_u64().expect("collected_at is u64");
    assert!(
        (before..=after).contains(&collected),
        "collected_at がサーバーの時刻でない: {collected} ({before}..={after})"
    );
}

// テスト4: 作成・読み取りのとき、07 のログに本文 canary と client_msg_id 用 canary が 0 件。
#[test]
fn route_logs_have_no_body_or_client_msg_id_canary() {
    let sink = Arc::new(MemorySink::new());
    let (addr, _store) = spawn_logged(&sink);

    // 作成: 自由文字列欄（scope・evidence_ref）に本文 canary を仕込む。
    // （保存された正本が応答に戻るので、応答本文にこの canary があるのは正しい。
    // ここで見るのは 07 のログに漏れないこと。）
    let body = basis_request(json!({
        "scope": format!("ch-synthetic-02 {CANARY_BODY}"),
        "evidence_ref": format!("ref:evidence-1 {CANARY_BODY}"),
    }));
    let (status, _resp) = post_basis(addr, &body);
    assert_eq!(status, 200);

    // 読み取り。
    let (status, _) = get_basis_list(addr);
    assert_eq!(status, 200);

    // client_msg_id canary を未知フィールドとして送る → 拒否。
    let with_msg_id = format!(
        "{{\"actor\": \"{ACTOR}\", \"basis_type\": \"synthetic_consent\", \
         \"purpose_id\": \"p03\", \"scope\": \"ch-synthetic-02\", \
         \"party_coverage_ref\": \"ref:1\", \"grantor_ref\": \"ref:2\", \
         \"explanation_version\": \"expl-v1\", \"evidence_ref\": \"ref:3\", \
         \"client_msg_id\": \"{CANARY_MSG_ID}\"}}"
    );
    let (status, resp) = post_basis(addr, &with_msg_id);
    assert_eq!(status, 400, "{resp}");

    // ログの検査: canary 0 件・出来事がある・生 ID が無く仮名がある。
    let lines = sink.lines();
    assert!(!lines.is_empty(), "ログが1件も出ていない");
    let joined = lines.join("\n");
    assert_eq!(
        joined.matches(CANARY_BODY).count(),
        0,
        "ログに本文 canary: {joined}"
    );
    assert_eq!(
        joined.matches(CANARY_MSG_ID).count(),
        0,
        "ログに client_msg_id canary: {joined}"
    );
    // 陽性対照: canary を含む文字列は同じ検査で見つかる。
    assert!(format!("x {CANARY_BODY}").matches(CANARY_BODY).count() == 1);
    assert!(format!("y {CANARY_MSG_ID}").matches(CANARY_MSG_ID).count() == 1);

    // 出来事: 作成・読取・拒否がそれぞれ出る。
    let ks = kinds(&lines);
    for want in ["basis_record_saved", "basis_records_listed", "rejected"] {
        assert!(ks.iter().any(|k| k == want), "{want} がない: {ks:?}");
    }
    // 生の ID（workspace・actor・basis）はログに無く、仮名がある。
    for raw in [WS, ACTOR, "bas-00000001"] {
        assert!(
            !joined.contains(raw),
            "生の ID {raw} がログにある: {joined}"
        );
    }
    assert!(
        joined.contains(&pseudonymize(SALT, WS)),
        "workspace の仮名がない: {joined}"
    );
    assert!(
        joined.contains(&pseudonymize(SALT, ACTOR)),
        "actor の仮名がない: {joined}"
    );
    assert!(
        joined.contains(&pseudonymize(SALT, "bas-00000001")),
        "basis_id の仮名がない: {joined}"
    );
}

// テスト6: 撤回が取得と同じ1操作（POST 1 回）で受け付けられ、epoch が進むこと。
#[test]
fn http_withdrawal_is_one_post_and_advances_epoch() {
    let sink = Arc::new(MemorySink::new());
    let (addr, _store) = spawn_logged(&sink);

    let (status, resp) = post_basis(addr, &basis_request(json!({})));
    assert_eq!(status, 200, "{resp}");
    let created: Value = serde_json::from_str(&resp).expect("json");
    let basis_id = created["basis_id"].as_str().expect("basis_id").to_owned();
    assert_eq!(created["epoch"], 1);

    // 撤回 = POST 1 回（取得と同じ操作の形）。
    let body =
        json!({ "actor": ACTOR, "withdrawal_method_ref": "ref:withdraw-method-1" }).to_string();
    let (status, resp) = post_withdraw(addr, &basis_id, &body);
    assert_eq!(status, 200, "{resp}");
    let withdrawn: Value = serde_json::from_str(&resp).expect("json");
    assert_eq!(withdrawn["epoch"], 2, "撤回で epoch が進む");
    assert_eq!(withdrawn["status"], "withdrawn");
    assert!(
        withdrawn["withdrawn_at"].is_u64(),
        "撤回時刻はサーバー付け: {resp}"
    );

    // 対象が無ければ 404。
    let (status, _) = post_withdraw(addr, "bas-99999999", &body);
    assert_eq!(status, 404);

    // ログに撤回の出来事（canary 無しは上のテストと同じ検査方針）。
    let ks = kinds(&sink.lines());
    assert!(
        ks.iter().any(|k| k == "basis_withdrawn"),
        "basis_withdrawn がない: {ks:?}"
    );
}

// テスト7: purpose_id の未設定・不正値の書き込みが拒否されること（HTTP）。
#[test]
fn http_rejects_invalid_or_missing_purpose_id() {
    let sink = Arc::new(MemorySink::new());
    let (addr, store) = spawn_logged(&sink);

    for bad in [
        json!("p09"),
        json!("p0"),
        json!("P03"),
        json!(""),
        json!(null),
        json!(3),
    ] {
        let (status, _) = post_basis(addr, &basis_request(json!({ "purpose_id": bad })));
        assert_eq!(status, 400, "根拠記録: {bad:?} が受理されている");
    }
    // 未設定（項目ごと無い）。
    let no_purpose = json!({
        "actor": ACTOR,
        "basis_type": "synthetic_consent",
        "scope": "ch-synthetic-02",
        "party_coverage_ref": "ref:1",
        "grantor_ref": "ref:2",
        "explanation_version": "expl-v1",
        "evidence_ref": "ref:3",
    })
    .to_string();
    let (status, _) = post_basis(addr, &no_purpose);
    assert_eq!(status, 400, "根拠記録: purpose_id 未設定が受理されている");

    // 利用方針も同じ検査。
    let policy_ok = json!({
        "actor": ACTOR,
        "purpose_id": "p04",
        "data_classes": ["message-body"],
        "source_scope": "ch-synthetic-02",
        "processors": ["processor-synthetic-1"],
        "allowed_outputs": ["summary"],
        "explanation_version": "expl-v1",
        "retention_policy_ref": "ref:retention-1",
    })
    .to_string();
    let (status, _) = http(
        addr,
        "POST",
        &format!("/v1/workspaces/{WS}/communication-use-policies"),
        &[("Content-Type", "application/json")],
        policy_ok.as_bytes(),
    );
    assert_eq!(status, 200);
    let mut policy_bad: Value = serde_json::from_str(&policy_ok).expect("json");
    policy_bad["purpose_id"] = json!("p99");
    let (status, _) = http(
        addr,
        "POST",
        &format!("/v1/workspaces/{WS}/communication-use-policies"),
        &[("Content-Type", "application/json")],
        policy_bad.to_string().as_bytes(),
    );
    assert_eq!(status, 400, "利用方針: p99 が受理されている");

    // 拒否された分は保存されていない（正本は無事的に 0 件の根拠記録）。
    let guard = store.lock().expect("store lock");
    assert!(guard.basis_records(&WorkspaceId::from(WS)).is_empty());
}

// 仮経路の補助: 利用方針の作成・読み取り・ログ。
#[test]
fn http_creates_and_lists_use_policies_with_versions() {
    let sink = Arc::new(MemorySink::new());
    let (addr, store) = spawn_logged(&sink);

    let v1 = json!({
        "actor": ACTOR,
        "purpose_id": "p04",
        "data_classes": ["message-body"],
        "source_scope": "ch-synthetic-02",
        "processors": ["processor-synthetic-1"],
        "allowed_outputs": ["summary"],
        "explanation_version": "expl-v1",
        "retention_policy_ref": "ref:retention-1",
    })
    .to_string();
    let (status, resp) = http(
        addr,
        "POST",
        &format!("/v1/workspaces/{WS}/communication-use-policies"),
        &[("Content-Type", "application/json")],
        v1.as_bytes(),
    );
    assert_eq!(status, 200, "{resp}");
    let p1: Value = serde_json::from_str(&resp).expect("json");
    assert_eq!(p1["version"], 1);

    // 変更（説明版を変える）→ 版が進み、前の版も残る。
    let v2 = json!({
        "actor": ACTOR,
        "purpose_id": "p04",
        "data_classes": ["message-body"],
        "source_scope": "ch-synthetic-02",
        "processors": ["processor-synthetic-1"],
        "allowed_outputs": ["summary", "draft"],
        "explanation_version": "expl-v2",
        "retention_policy_ref": "ref:retention-1",
    })
    .to_string();
    let (status, resp) = http(
        addr,
        "POST",
        &format!("/v1/workspaces/{WS}/communication-use-policies"),
        &[("Content-Type", "application/json")],
        v2.as_bytes(),
    );
    assert_eq!(status, 200, "{resp}");
    let p2: Value = serde_json::from_str(&resp).expect("json");
    assert_eq!(p2["version"], 2);

    // 読み取り（actor 必須）。
    let (status, resp) = http(
        addr,
        "GET",
        &format!("/v1/workspaces/{WS}/communication-use-policies?actor={ACTOR}"),
        &[],
        b"",
    );
    assert_eq!(status, 200, "{resp}");
    let arr: Vec<Value> = serde_json::from_str(&resp).expect("json array");
    assert_eq!(arr.len(), 2, "変更前の版も残る: {resp}");
    // actor が無い読み取りは拒否。
    let (status, _) = http(
        addr,
        "GET",
        &format!("/v1/workspaces/{WS}/communication-use-policies"),
        &[],
        b"",
    );
    assert_eq!(status, 400);

    // ログ: 保存・読取の出来事と仮名。
    let lines = sink.lines();
    let joined = lines.join("\n");
    let ks = kinds(&lines);
    for want in ["use_policy_saved", "use_policies_listed"] {
        assert!(ks.iter().any(|k| k == want), "{want} がない: {ks:?}");
    }
    assert!(joined.contains(&pseudonymize(SALT, WS)));
    assert!(joined.contains(&pseudonymize(SALT, ACTOR)));

    // 保存部にも同じ 2 件。
    let guard = store.lock().expect("store lock");
    assert_eq!(guard.use_policies(&WorkspaceId::from(WS)).len(), 2);
}
