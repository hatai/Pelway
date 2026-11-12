//! 受け渡し（#89）の HTTP 経路の結合テスト。`127.0.0.1:0` で起動したサーバーに
//! 実際に HTTP/1.1 要求を送り、状態コードと本文 JSON を確かめる。
//! 段数の上限・行き来・通知の中身などのシナリオは `pelway-contract` の単体テスト
//! （`handoff.rs`）で確かめる。ここは経路・状態コード・応答の形だけを検証する。
//! 接続先はループバックだけ。外部への接続は無い。

use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};

use pelway_contract::approval::Decider;
use pelway_contract::handoff::handoffs_path;
use pelway_contract::ids::{CardId, MemberId};
use pelway_contract::ledger::{ApprovalLedger, NewCard};
use pelway_contract::snapshot::ActionPayload;
use pelway_contract::task::ActionKind;
use pelway_http::{SharedLedger, serve};
use serde_json::{Value, json};

/// 受け渡し要求の本文（5 項目すべてを 1 件ずつ持つ）。
fn handoff_body(from: &str, to: &str) -> String {
    json!({
        "card_id": "card-handoff",
        "from": from,
        "to": to,
        "context": {
            "decisions": [
                {"body": "案Aで送る", "decider": "m-human-handoff", "decided_at": 900}
            ],
            "sent": [
                {
                    "recipient": "customer@example.invalid",
                    "body": "合成データの本文",
                    "attachments": ["引合書.png"],
                    "sent_at": 950
                }
            ],
            "current_state": "working",
            "remaining_work": ["返信を確認する"],
            "awaiting": to
        }
    })
    .to_string()
}

/// 担当が m-bot-1 の AgentHandoff カード 1 枚を持つ台帳。
fn shared_ledger() -> SharedLedger {
    let mut ledger = ApprovalLedger::new();
    ledger
        .create_card(NewCard {
            workspace_id: "ws-synthetic-handoff".into(),
            channel_id: "ch-synthetic-handoff".into(),
            card_id: "card-handoff".into(),
            payload: ActionPayload {
                action_kind: ActionKind::AgentHandoff,
                recipient: "m-bot-2".to_owned(),
                body: "合成データの本文".to_owned(),
                attachments: vec!["sha256:att-1".to_owned()],
                account: "acct-cw-1".to_owned(),
                scope: vec!["tickets:write".to_owned()],
            },
            requester: "m-human-handoff".into(),
            decider: Decider::Member("m-bot-1".into()),
            deadline: None,
        })
        .expect("new card");
    Arc::new(Mutex::new(ledger))
}

/// 生の HTTP/1.1 要求を 1 回送り、(状態コード, 本文) を返す。
fn http(addr: SocketAddr, method: &str, path: &str, body: &[u8]) -> (u16, String) {
    let mut stream = TcpStream::connect(addr).expect("connect");
    let req = format!(
        "{method} {path} HTTP/1.1\r\nHost: demo.local\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
        body.len()
    );
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

fn post_handoff(addr: SocketAddr, from: &str, to: &str) -> (u16, Value) {
    let path = handoffs_path(&CardId::from("card-handoff"));
    let (status, body) = http(addr, "POST", &path, handoff_body(from, to).as_bytes());
    (status, serde_json::from_str(&body).unwrap_or(Value::Null))
}

#[test]
fn handoff_post_returns_200_with_accepted_and_server_assigned_depth() {
    let addr = serve("127.0.0.1:0".parse().unwrap(), shared_ledger())
        .expect("serve")
        .local_addr();
    let (status, json) = post_handoff(addr, "m-bot-1", "m-bot-2");
    assert_eq!(status, 200, "{json}");
    assert_eq!(json["result"], "accepted");
    // 段数・記録 ID はサーバーが決めた値（契約の単体テストと同じ採番）。
    assert_eq!(json["record"]["depth"], 1);
    assert_eq!(json["record"]["handoff_id"], "hnd-00000001");
}

#[test]
fn handoff_refusal_is_200_with_refused_not_4xx() {
    let shared = shared_ledger();
    let addr = serve("127.0.0.1:0".parse().unwrap(), Arc::clone(&shared))
        .expect("serve")
        .local_addr();
    // 3 段まで受理（チェーンを作る。シナリオの詳細は契約の単体テスト）。
    for (from, to) in [
        ("m-bot-1", "m-bot-2"),
        ("m-bot-2", "m-bot-3"),
        ("m-bot-3", "m-bot-4"),
    ] {
        let (status, json) = post_handoff(addr, from, to);
        assert_eq!(status, 200, "{json}");
    }
    // 4 段目は HTTP の失敗ではなく、拒否の業務結果（通知・担当戻しを含む）。
    let (status, json) = post_handoff(addr, "m-bot-4", "m-bot-5");
    assert_eq!(status, 200, "{json}");
    assert_eq!(json["result"], "refused");
    assert_eq!(json["refusal"]["reason"]["code"], "depth_limit_exceeded");
    assert_eq!(
        json["refusal"]["notification"]["channel_id"],
        "ch-synthetic-handoff"
    );
    assert_eq!(json["refusal"]["returned_to"], "m-human-handoff");
    // サーバーを通して、タスクは依頼者に戻っている。
    let decider = shared
        .lock()
        .unwrap()
        .card(&CardId::from("card-handoff"))
        .unwrap()
        .decider
        .clone();
    assert_eq!(decider, Decider::Member(MemberId::from("m-human-handoff")));
}

#[test]
fn handoff_errors_map_to_4xx_and_unknown_path_is_404() {
    let shared = shared_ledger();
    let addr = serve("127.0.0.1:0".parse().unwrap(), Arc::clone(&shared))
        .expect("serve")
        .local_addr();
    // 現在の担当（m-bot-1）でないメンバーからの要求は 409。
    let (status, json) = post_handoff(addr, "m-bot-9", "m-bot-2");
    assert_eq!(status, 409, "{json}");
    assert_eq!(json["code"], "not_current_decider");
    // 自己受け渡しは 422。
    let (status, json) = post_handoff(addr, "m-bot-1", "m-bot-1");
    assert_eq!(status, 422, "{json}");
    assert_eq!(json["code"], "self_handoff");
    // 経路と本文の card_id の不一致は 422。
    let mut body: Value = serde_json::from_str(&handoff_body("m-bot-1", "m-bot-2")).unwrap();
    body["card_id"] = json!("card-other");
    let (status, _) = http(
        addr,
        "POST",
        &handoffs_path(&CardId::from("card-handoff")),
        body.to_string().as_bytes(),
    );
    assert_eq!(status, 422);
    // 無いカードは 404（本文の card_id を経路に合わせる）。未知の経路も 404
    // （経路定数と過不足なく一致）。
    let mut absent: Value = serde_json::from_str(&handoff_body("m-bot-1", "m-bot-2")).unwrap();
    absent["card_id"] = json!("card-absent");
    let path = handoffs_path(&CardId::from("card-absent"));
    let (status, _) = http(addr, "POST", &path, absent.to_string().as_bytes());
    assert_eq!(status, 404);
    let (status, _) = http(
        addr,
        "POST",
        "/v1/approvals/card-handoff/handoff",
        handoff_body("m-bot-1", "m-bot-2").as_bytes(),
    );
    assert_eq!(status, 404);
}
