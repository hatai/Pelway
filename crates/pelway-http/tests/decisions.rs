//! A/B 決定（判断待ち）の HTTP 経路（#328）の結合テスト。`127.0.0.1:0` で起動した
//! サーバーに実際に HTTP/1.1 要求を送り、状態コードと本文 JSON を確かめる。
//! 接続先はループバックだけ。外部への接続は無い。
//!
//! 経路（契約のテンプレート定数から作る。経路を書き写さない。Issue #91）:
//! - `POST /v1/workspaces/{workspace_id}/decisions` … 判断待ちの作成
//! - `GET  /v1/workspaces/{workspace_id}/decisions?actor={member}` … 判断待ち一覧
//! - `POST /v1/decisions/{decision_id}/decide` … 決定の確定（操作者は合成ヘッダー）

use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};

use pelway_contract::decision::{decide_path, decisions_path};
use pelway_contract::ids::{DecisionId, WorkspaceId};
use pelway_contract::ledger::ApprovalLedger;
use pelway_http::{SharedLedger, serve};
use serde_json::{Value, json};

const WS: &str = "ws-synthetic-decision";
const CH: &str = "ch-synthetic-decision";
const REQUESTER: &str = "m-human-tanaka";
const DECIDER: &str = "m-human-yamamoto";
const OTHER: &str = "m-human-suzuki";

fn spawn_server(ledger: SharedLedger) -> SocketAddr {
    serve("127.0.0.1:0".parse().unwrap(), ledger)
        .expect("serve on 127.0.0.1:0")
        .local_addr()
}

/// 生の HTTP/1.1 要求を 1 回送り、(状態コード, 本文) を返す。
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
    let resp_body = buf.split("\r\n\r\n").nth(1).unwrap_or("").to_owned();
    (status, resp_body)
}

/// 合成の A/B 判断待ちの作成本文。`question` は呼び出し側で変えられる。
fn create_body(question: &str) -> Vec<u8> {
    json!({
        "channel_id": CH,
        "request_message_id": "msg-00000042",
        "requester": REQUESTER,
        "decider": DECIDER,
        "question": question,
        "options": [
            {"option_id": "a", "body": "案A: 納期を2週間延ばす"},
            {"option_id": "b", "body": "案B: 範囲を絞って納期を守る"}
        ]
    })
    .to_string()
    .into_bytes()
}

fn post_create(addr: SocketAddr, workspace: &str, body: &[u8]) -> (u16, String) {
    http(
        addr,
        "POST",
        &decisions_path(&WorkspaceId::from(workspace)),
        &[],
        body,
    )
}

fn post_decide(addr: SocketAddr, decision_id: &str, member: &str, body: &[u8]) -> (u16, String) {
    http(
        addr,
        "POST",
        &decide_path(&DecisionId::from(decision_id)),
        &[
            ("x-pelway-member-id", member),
            ("x-pelway-member-kind", "human"),
        ],
        body,
    )
}

fn decide_body(decision_id: &str, version: u64, option_id: &str) -> Vec<u8> {
    json!({
        "decision_id": decision_id,
        "version": version,
        "option_id": option_id
    })
    .to_string()
    .into_bytes()
}

fn get_decisions(addr: SocketAddr, workspace: &str, actor: &str) -> (u16, Value) {
    let (status, body) = http(
        addr,
        "GET",
        &format!(
            "{}?actor={actor}",
            decisions_path(&WorkspaceId::from(workspace))
        ),
        &[],
        b"",
    );
    (status, serde_json::from_str(&body).unwrap_or(Value::Null))
}

/// 決定者・合成ヘッダー付きで判断待ちを 1 件作り、`decision_id` を返す。
fn create_one(addr: SocketAddr, question: &str) -> String {
    let (status, body) = post_create(addr, WS, &create_body(question));
    assert_eq!(status, 200, "{body}");
    let v: Value = serde_json::from_str(&body).expect("create body is json");
    v["decision_id"].as_str().expect("decision_id").to_owned()
}

// ---- 受け入れ条件 1: A/B の選択肢つきの判断待ちが作成でき、
//      依頼メッセージへの参照と待ち開始時刻が記録される ----

#[test]
fn create_returns_server_assigned_card_with_message_ref_and_waiting_since() {
    let addr = spawn_server(Arc::new(Mutex::new(ApprovalLedger::new())));
    let (status, body) = post_create(addr, WS, &create_body("納期延長の返事をどうするか"));
    assert_eq!(status, 200, "{body}");
    let v: Value = serde_json::from_str(&body).expect("json");
    // ID・版・待ち開始時刻はサーバーが付ける。
    assert_eq!(v["decision_id"], "dec-00000001");
    assert_eq!(v["version"], 1);
    assert!(v["waiting_since"].is_u64(), "waiting_since: {v}");
    // 依頼メッセージへの参照・チャンネル・依頼者・決定者・論点・選択肢。
    assert_eq!(v["request_message_id"], "msg-00000042");
    assert_eq!(v["channel_id"], CH);
    assert_eq!(v["requester"], REQUESTER);
    assert_eq!(v["decider"], DECIDER);
    assert_eq!(v["question"], "納期延長の返事をどうするか");
    assert_eq!(v["options"].as_array().expect("options").len(), 2);
    assert_eq!(v["options"][0]["option_id"], "a");
    // 作成直後はまだ決まっていない。
    assert!(v["outcome"].is_null(), "{v}");
}

// ---- 受け入れ条件 2: 決定すると選択内容・決定者・時刻・版が固定され、
//      確定後の変更が拒否される ----

#[test]
fn decide_fixes_outcome_and_rejects_later_changes() {
    let shared: SharedLedger = Arc::new(Mutex::new(ApprovalLedger::new()));
    let addr = spawn_server(Arc::clone(&shared));
    let decision_id = create_one(addr, "納期延長の返事をどうするか");

    // 確定: 選択内容・決定者・時刻・版が固定される。
    let (status, body) = post_decide(
        addr,
        &decision_id,
        DECIDER,
        &decide_body(&decision_id, 1, "a"),
    );
    assert_eq!(status, 200, "{body}");
    let v: Value = serde_json::from_str(&body).expect("outcome json");
    assert_eq!(v["decision_id"], decision_id);
    assert_eq!(v["version"], 2);
    assert_eq!(v["selected"]["option_id"], "a");
    assert_eq!(v["selected"]["body"], "案A: 納期を2週間延ばす");
    assert_eq!(v["decider"], DECIDER);
    assert!(v["decided_at"].is_u64(), "{v}");

    // 確定後に別の選択へ変える要求は拒否（状態は変わらない）。
    let (status, body) = post_decide(
        addr,
        &decision_id,
        DECIDER,
        &decide_body(&decision_id, 2, "b"),
    );
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("already_decided"), "{body}");

    // 同じ選択の再送は確定済みの結果を返す（冪等リプレイ）。
    let (status, body) = post_decide(
        addr,
        &decision_id,
        DECIDER,
        &decide_body(&decision_id, 2, "a"),
    );
    assert_eq!(status, 200, "{body}");
    let replay: Value = serde_json::from_str(&body).expect("replay json");
    assert_eq!(replay, v, "同じ選択の再送は同じ結果");

    // 正本（台帳）には決定記録が残り、版は 2 のまま。
    let ledger = shared.lock().expect("ledger lock");
    let card = ledger
        .decision(&DecisionId::from(decision_id.as_str()))
        .expect("decision");
    assert_eq!(card.version, 2);
    assert_eq!(
        card.outcome
            .as_ref()
            .expect("outcome")
            .selected
            .option_id
            .as_str(),
        "a"
    );
}

// ---- 受け入れ条件 3: 一覧は誰の・何の判断か、待ち時間、
//      対象チャンネルとメッセージへの参照を返す ----

#[test]
fn list_returns_whose_what_wait_time_channel_and_message_ref() {
    let addr = spawn_server(Arc::new(Mutex::new(ApprovalLedger::new())));
    let decision_id = create_one(addr, "納期延長の返事をどうするか");
    // 他人の判断待ちと別ワークスペースの判断待ち（一覧に混ざらない）。
    let mut other = create_body("他人の判断");
    let mut v: Value = serde_json::from_slice(&other).expect("json");
    v["decider"] = json!(OTHER);
    other = v.to_string().into_bytes();
    let (status, body) = post_create(addr, WS, &other);
    assert_eq!(status, 200, "{body}");
    let (status, body) = post_create(addr, "ws-synthetic-other", &create_body("別ws"));
    assert_eq!(status, 200, "{body}");

    let (status, json) = get_decisions(addr, WS, DECIDER);
    assert_eq!(status, 200, "{json}");
    let items = json.as_array().expect("list is an array");
    assert_eq!(items.len(), 1, "自分が決定者の未決定だけ: {json}");
    let item = &items[0];
    assert_eq!(item["decision_id"], decision_id);
    // 誰の（決定者・依頼者）・何の（論点・選択肢）判断か。
    assert_eq!(item["decider"], DECIDER);
    assert_eq!(item["requester"], REQUESTER);
    assert_eq!(item["question"], "納期延長の返事をどうするか");
    assert_eq!(item["options"].as_array().expect("options").len(), 2);
    // 対象チャンネルと依頼メッセージへの参照。
    assert_eq!(item["channel_id"], CH);
    assert_eq!(item["request_message_id"], "msg-00000042");
    // 待ち時間は待ち開始時刻から（期限ではない）。直後なので小さい値。
    assert!(item["waiting_since"].is_u64(), "{item}");
    let wait_ms = item["wait_ms"].as_u64().expect("wait_ms is u64");
    assert!(wait_ms < 60_000, "wait_ms は現実的な小ささ: {wait_ms}");
    // 版（確定要求が書く値）。
    assert_eq!(item["version"], 1);

    // 別の人の一覧・別 workspace の一覧は空。
    let (status, json) = get_decisions(addr, WS, "m-human-stranger");
    assert_eq!(status, 200);
    assert_eq!(json.as_array().expect("array").len(), 0, "{json}");
    let (status, json) = get_decisions(addr, "ws-absent", DECIDER);
    assert_eq!(status, 200);
    assert_eq!(json.as_array().expect("array").len(), 0, "{json}");
}

// ---- 受け入れ条件 4: 決定済みの項目が一覧から消える ----

#[test]
fn confirmed_items_leave_the_pending_list() {
    let addr = spawn_server(Arc::new(Mutex::new(ApprovalLedger::new())));
    let decision_id = create_one(addr, "納期延長の返事をどうするか");

    let (status, json) = get_decisions(addr, WS, DECIDER);
    assert_eq!(status, 200);
    assert_eq!(json.as_array().expect("array").len(), 1);

    let (status, _) = post_decide(
        addr,
        &decision_id,
        DECIDER,
        &decide_body(&decision_id, 1, "a"),
    );
    assert_eq!(status, 200);

    let (status, json) = get_decisions(addr, WS, DECIDER);
    assert_eq!(status, 200, "{json}");
    assert_eq!(
        json.as_array().expect("array").len(),
        0,
        "確定した項目は一覧に出ない: {json}"
    );
}

// ---- 確定要求の検査（4xx。状態を変えない） ----

#[test]
fn decide_validates_decider_version_option_and_ids() {
    let addr = spawn_server(Arc::new(Mutex::new(ApprovalLedger::new())));
    let decision_id = create_one(addr, "納期延長の返事をどうするか");

    // 決定者でない人は決定できない。
    let (status, body) = post_decide(
        addr,
        &decision_id,
        OTHER,
        &decide_body(&decision_id, 1, "a"),
    );
    assert_eq!(status, 403, "{body}");
    assert!(body.contains("not_decider"), "{body}");

    // 画面の版が古い。
    let (status, body) = post_decide(
        addr,
        &decision_id,
        DECIDER,
        &decide_body(&decision_id, 99, "a"),
    );
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("version_conflict"), "{body}");

    // 選択肢に無い option_id。
    let (status, body) = post_decide(
        addr,
        &decision_id,
        DECIDER,
        &decide_body(&decision_id, 1, "z"),
    );
    assert_eq!(status, 422, "{body}");
    assert!(body.contains("unknown_option"), "{body}");

    // 存在しない判断待ち。
    let (status, body) = post_decide(
        addr,
        "dec-99999999",
        DECIDER,
        &decide_body("dec-99999999", 1, "a"),
    );
    assert_eq!(status, 404, "{body}");
    assert!(body.contains("decision_not_found"), "{body}");

    // 経路と本文の decision_id の不一致。
    let (status, body) = post_decide(
        addr,
        &decision_id,
        DECIDER,
        &decide_body("dec-99999999", 1, "a"),
    );
    assert_eq!(status, 422, "{body}");
    assert!(body.contains("decision_id_mismatch"), "{body}");

    // 合成ヘッダー（操作者）が無い・kind が不正。
    let (status, _) = http(
        addr,
        "POST",
        &decide_path(&DecisionId::from(decision_id.as_str())),
        &[],
        &decide_body(&decision_id, 1, "a"),
    );
    assert_eq!(status, 400, "member ヘッダー無しは 400");
    let (status, _) = http(
        addr,
        "POST",
        &decide_path(&DecisionId::from(decision_id.as_str())),
        &[("x-pelway-member-id", DECIDER)],
        &decide_body(&decision_id, 1, "a"),
    );
    assert_eq!(status, 400, "kind ヘッダー無しは 400");
    let (status, _) = http(
        addr,
        "POST",
        &decide_path(&DecisionId::from(decision_id.as_str())),
        &[
            ("x-pelway-member-id", DECIDER),
            ("x-pelway-member-kind", "not-a-kind"),
        ],
        &decide_body(&decision_id, 1, "a"),
    );
    assert_eq!(status, 400, "kind が不正なら 400");

    // 本文が JSON でない・必須欄が欠ける。
    let (status, _) = post_decide(addr, &decision_id, DECIDER, b"not-json");
    assert_eq!(status, 400, "不正 JSON は 400");
    let (status, _) = post_decide(addr, &decision_id, DECIDER, b"{}");
    assert_eq!(status, 400, "必須欄の欠落は 400");

    // 何も変わっていない（まだ未確定）。
    let (_, json) = get_decisions(addr, WS, DECIDER);
    assert_eq!(
        json.as_array().expect("array").len(),
        1,
        "失敗は状態を変えない"
    );
}

#[test]
fn create_validates_body() {
    let addr = spawn_server(Arc::new(Mutex::new(ApprovalLedger::new())));
    let cases: Vec<(Value, &str)> = vec![
        (json!({"question": "  "}), "empty_question"),
        (
            json!({"options": [{"option_id": "a", "body": "案A"}]}),
            "too_few_options",
        ),
        (
            json!({"options": [
                {"option_id": "a", "body": "案A"},
                {"option_id": "a", "body": "案A2"}
            ]}),
            "invalid_option_id",
        ),
        (
            json!({"options": [
                {"option_id": "a", "body": "案A"},
                {"option_id": "b", "body": " "}
            ]}),
            "empty_option_body",
        ),
    ];
    for (patch, code) in cases {
        let mut v: Value = serde_json::from_slice(&create_body("q")).expect("json");
        for (k, val) in patch.as_object().expect("patch obj") {
            v[k] = val.clone();
        }
        let (status, body) = post_create(addr, WS, &v.to_string().into_bytes());
        assert_eq!(status, 422, "{code}: {body}");
        assert!(body.contains(code), "{code}: {body}");
    }
    // サーバーが付ける欄を要求に入れると未知フィールドで 400。
    for smuggled in ["decision_id", "version", "waiting_since", "outcome"] {
        let mut v: Value = serde_json::from_slice(&create_body("q")).expect("json");
        v[smuggled] = json!("x");
        let (status, _) = post_create(addr, WS, &v.to_string().into_bytes());
        assert_eq!(status, 400, "{smuggled} 混入は 400");
    }
    // 本文が JSON でない。
    let (status, _) = post_create(addr, WS, b"not-json");
    assert_eq!(status, 400);
}

// ---- メソッド・一覧の引数 ----

#[test]
fn list_requires_actor_and_methods_are_checked() {
    let addr = spawn_server(Arc::new(Mutex::new(ApprovalLedger::new())));
    // actor が無い・空なら 400。
    let (status, _) = http(
        addr,
        "GET",
        &decisions_path(&WorkspaceId::from(WS)),
        &[],
        b"",
    );
    assert_eq!(status, 400, "actor 無しは 400");
    let (status, _) = http(
        addr,
        "GET",
        &format!("{}?actor=", decisions_path(&WorkspaceId::from(WS))),
        &[],
        b"",
    );
    assert_eq!(status, 400, "actor 空は 400");
    // 確定経路に GET は 405。
    let (status, body) = http(
        addr,
        "GET",
        &decide_path(&DecisionId::from("dec-00000001")),
        &[],
        b"",
    );
    assert_eq!(status, 405, "{body}");
    assert!(body.contains("method_not_allowed"), "{body}");
}

// ---- 台帳が空でも 200 で空の一覧 ----

#[test]
fn empty_ledger_returns_empty_list() {
    let addr = spawn_server(Arc::new(Mutex::new(ApprovalLedger::new())));
    let (status, json) = get_decisions(addr, WS, DECIDER);
    assert_eq!(status, 200, "{json}");
    assert_eq!(json, json!([]));
}
