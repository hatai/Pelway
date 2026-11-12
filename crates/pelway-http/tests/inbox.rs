//! 判断待ち一覧の HTTP 経路（#88）の結合テスト。`127.0.0.1:0` で起動した
//! サーバーに実際に HTTP/1.1 要求を送り、状態コードと本文 JSON を確かめる。
//! 接続先はループバックだけ。外部への接続は無い。

use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};

use pelway_contract::approval::Decider;
use pelway_contract::ids::{CardId, MemberId, WorkspaceId};
use pelway_contract::ledger::{ApprovalLedger, NewCard};
use pelway_contract::snapshot::ActionPayload;
use pelway_contract::store::TsMs;
use pelway_contract::task::{ActionKind, TaskState};
use pelway_http::{SharedLedger, serve};
use serde_json::Value;

const WS: &str = "ws-synthetic-inbox";
const ME: &str = "m-human-inbox";
const OTHER: &str = "m-human-other";

/// テストカードの組み立て。`decider`・`requester`・期限・種類は呼び出し側で変える。
fn new_card(id: &str, decider: &str, requester: &str, deadline: Option<TsMs>) -> NewCard {
    NewCard {
        workspace_id: WorkspaceId::from(WS),
        channel_id: "ch-synthetic-inbox".into(),
        card_id: CardId::from(id),
        payload: ActionPayload {
            action_kind: ActionKind::CustomerSend,
            recipient: "customer@example.invalid".to_owned(),
            body: format!("合成データの本文 {id}"),
            attachments: Vec::new(),
            account: "acct-cw-1".to_owned(),
            scope: Vec::new(),
        },
        requester: MemberId::from(requester),
        decider: Decider::Member(MemberId::from(decider)),
        deadline,
    }
}

fn spawn_server(ledger: SharedLedger) -> SocketAddr {
    serve("127.0.0.1:0".parse().unwrap(), ledger)
        .expect("serve on 127.0.0.1:0")
        .local_addr()
}

/// 生の HTTP/1.1 要求を 1 回送り、(状態コード, 本文) を返す。
fn http(addr: SocketAddr, method: &str, path: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(addr).expect("connect");
    let req = format!(
        "{method} {path} HTTP/1.1\r\nHost: demo.local\r\nConnection: close\r\nContent-Length: 0\r\n\r\n"
    );
    stream.write_all(req.as_bytes()).expect("write head");
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

fn get_inbox(addr: SocketAddr, actor: &str) -> (u16, Value) {
    let (status, body) = http(
        addr,
        "GET",
        &format!("/v1/workspaces/{WS}/inbox?actor={actor}"),
    );
    (status, serde_json::from_str(&body).unwrap_or(Value::Null))
}

fn card_ids(v: &Value, tab: &str) -> Vec<String> {
    v[tab]
        .as_array()
        .expect("tab is an array")
        .iter()
        .map(|item| item["card_id"].as_str().expect("card_id").to_owned())
        .collect()
}

/// 台帳を組み立てる: 自分が承認する項目（期限 30_000）・自分が担当する引き継ぎ
/// （期限 10_000）・自分が依頼して相手が判断する項目（期限 20_000）・
/// まったく関係ない他人の項目・ほかの workspace の項目。
fn ledger() -> SharedLedger {
    let mut ledger = ApprovalLedger::new();
    ledger
        .create_card(new_card("card-approve-me", ME, ME, Some(30_000)))
        .expect("new card");
    let mut handoff = new_card("card-handoff-me", ME, OTHER, Some(10_000));
    handoff.payload.action_kind = ActionKind::AgentHandoff;
    ledger.create_card(handoff).expect("new card");
    ledger
        .create_card(new_card("card-my-request", OTHER, ME, Some(20_000)))
        .expect("new card");
    ledger
        .create_card(new_card("card-untouched", OTHER, OTHER, None))
        .expect("new card");
    ledger
        .create_card(NewCard {
            workspace_id: WorkspaceId::from("ws-synthetic-other"),
            ..new_card("card-other-ws", ME, ME, None)
        })
        .expect("new card");
    Arc::new(Mutex::new(ledger))
}

// 受け入れ条件1・3: 「自分の判断待ち」タブに承認項目と引き継ぎが入り、
// 期限の近い順に並ぶ。他人の項目・ほかの workspace の項目は入らない。
#[test]
fn needs_my_decision_tab_lists_approvals_and_handoffs_by_deadline() {
    let addr = spawn_server(ledger());
    let (status, json) = get_inbox(addr, ME);
    assert_eq!(status, 200, "{json}");
    assert_eq!(
        card_ids(&json, "needs_my_decision"),
        ["card-handoff-me", "card-approve-me"],
        "期限 10_000 の引き継ぎが先、30_000 の承認が後: {json}"
    );
    let handoff = &json["needs_my_decision"][0];
    assert_eq!(handoff["reason"], "handoff");
    assert_eq!(handoff["state"], "awaiting_send_approval");
    assert_eq!(handoff["deadline"], 10_000);
    let approval = &json["needs_my_decision"][1];
    assert_eq!(approval["reason"], "approval");
    assert_eq!(approval["channel_id"], "ch-synthetic-inbox");
}

// 受け入れ条件2: 「自分の依頼」タブに自分の依頼が進み具合（現在の状態）付きで並ぶ。
#[test]
fn my_requests_tab_lists_my_requests_with_progress() {
    let addr = spawn_server(ledger());
    let (status, json) = get_inbox(addr, ME);
    assert_eq!(status, 200, "{json}");
    assert_eq!(
        card_ids(&json, "my_requests"),
        ["card-my-request", "card-approve-me"],
        "期限 20_000 が先、30_000 が後: {json}"
    );
    let requested = &json["my_requests"][0];
    assert_eq!(requested["state"], "awaiting_send_approval");
    assert_eq!(requested["requester"], ME);
    assert_eq!(requested["decider"], OTHER);
}

// 受け入れ条件4: 一覧は台帳の状態を検索して作る。承認で状態が変われば一覧も変わる。
#[test]
fn inbox_reflects_state_changes_in_the_ledger() {
    let shared = ledger();
    let addr = spawn_server(Arc::clone(&shared));

    let (status, before) = get_inbox(addr, ME);
    assert_eq!(status, 200);
    assert_eq!(card_ids(&before, "needs_my_decision").len(), 2);

    // card-approve-me を承認する（担当は自分）。
    {
        let mut ledger = shared.lock().expect("ledger lock");
        let c = ledger.card(&CardId::from("card-approve-me")).expect("card");
        let req = pelway_contract::approval::ApproveRequest {
            card_id: CardId::from("card-approve-me"),
            card_version: c.card_version,
            snapshot_hash: c.snapshot_hash.clone(),
            idempotency_key: pelway_contract::ids::IdempotencyKey::new("test-key-http-inbox-0001")
                .expect("valid key"),
        };
        ledger.approve(req, &MemberId::from(ME)).expect("approve");
    }

    let (status, after) = get_inbox(addr, ME);
    assert_eq!(status, 200, "{after}");
    assert_eq!(
        card_ids(&after, "needs_my_decision"),
        ["card-handoff-me"],
        "承認済みの項目は判断待ちに残らない: {after}"
    );
    let mine = card_ids(&after, "my_requests");
    assert!(mine.contains(&"card-approve-me".to_owned()), "{after}");
    let sent = after["my_requests"]
        .as_array()
        .expect("array")
        .iter()
        .find(|i| i["card_id"] == "card-approve-me")
        .expect("card-approve-me");
    assert_eq!(sent["state"], "sent_awaiting_reply", "進み具合が変わる");
}

// actor が無い・空なら 400、GET 以外は 405。
#[test]
fn inbox_requires_actor_and_get_method() {
    let addr = spawn_server(ledger());
    let (status, _) = http(addr, "GET", &format!("/v1/workspaces/{WS}/inbox"));
    assert_eq!(status, 400, "actor 無しは 400");
    let (status, _) = http(addr, "GET", &format!("/v1/workspaces/{WS}/inbox?actor="));
    assert_eq!(status, 400, "actor 空は 400");
    let (status, body) = http(
        addr,
        "POST",
        &format!("/v1/workspaces/{WS}/inbox?actor={ME}"),
    );
    assert_eq!(status, 405, "POST は 405: {body}");
    assert!(body.contains("method_not_allowed"), "{body}");
}

// 未知の workspace でも 200 で空の一覧（存在検査ではなく検索結果を返す）。
#[test]
fn unknown_workspace_returns_empty_inbox() {
    let addr = spawn_server(ledger());
    let (status, json) = http(
        addr,
        "GET",
        &format!("/v1/workspaces/ws-absent/inbox?actor={ME}"),
    );
    assert_eq!(status, 200);
    let json: Value = serde_json::from_str(&json).unwrap_or(Value::Null);
    assert_eq!(card_ids(&json, "needs_my_decision"), Vec::<String>::new());
    assert_eq!(card_ids(&json, "my_requests"), Vec::<String>::new());
}

// 台帳が空でも 200 で空の一覧。
#[test]
fn empty_ledger_returns_empty_tabs() {
    let addr = spawn_server(Arc::new(Mutex::new(ApprovalLedger::new())));
    let (status, json) = get_inbox(addr, ME);
    assert_eq!(status, 200, "{json}");
    assert_eq!(card_ids(&json, "needs_my_decision"), Vec::<String>::new());
    assert_eq!(card_ids(&json, "my_requests"), Vec::<String>::new());
    let _ = TaskState::Done; // 状態名の直列化はほかのテストで確認済み（未使用警告回避）。
}
