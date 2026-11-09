//! HTTP 層の結合テスト（QA 用）。`127.0.0.1:0` で起動したサーバーに
//! `std::net::TcpStream` で実際に HTTP/1.1 要求を送り、状態コードと本文 JSON を確かめる。
//!
//! 接続先はループバック（127.0.0.1 / ::1）だけ。外部への接続は無い。

use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::{Arc, Barrier, Mutex};
use std::thread;

use pelway_contract::approval::ApproveRequest;
use pelway_contract::ids::IdempotencyKey;
use pelway_contract::ledger::{ApprovalLedger, BeginApproval, NewCard};
use pelway_contract::task::ActionKind;
use pelway_http::{MAX_BODY_BYTES, SharedLedger, serve};
use serde_json::{Value, json};

fn new_card(id: &str) -> NewCard {
    NewCard {
        workspace_id: "ws-synthetic".into(),
        channel_id: "ch-synthetic".into(),
        card_id: id.into(),
        payload_hash: "sha256:synthetic-payload-v1".to_owned(),
        action_kind: ActionKind::CustomerSend,
        recipient: "customer@example.invalid".to_owned(),
        body: "合成データの本文".to_owned(),
    }
}

fn shared_ledger(cards: &[&str]) -> SharedLedger {
    let mut ledger = ApprovalLedger::new();
    for id in cards {
        ledger.create_card(new_card(id)).expect("new card");
    }
    Arc::new(Mutex::new(ledger))
}

fn spawn_server(ledger: SharedLedger) -> SocketAddr {
    let addr = serve("127.0.0.1:0".parse().unwrap(), ledger)
        .expect("serve on 127.0.0.1:0")
        .local_addr();
    assert_eq!(addr.ip().to_string(), "127.0.0.1");
    addr
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

fn post_json(addr: SocketAddr, path: &str, body: &str, member_headers: bool) -> (u16, Value) {
    let mut headers = vec![("Content-Type", "application/json")];
    if member_headers {
        headers.push(("x-pelway-member-id", "m-human-http"));
        headers.push(("x-pelway-member-kind", "human"));
    }
    let (status, body) = http(addr, "POST", path, &headers, body.as_bytes());
    let json = serde_json::from_str(&body).unwrap_or(Value::Null);
    (status, json)
}

fn post_comments(addr: SocketAddr, card: &str, body: &str) -> (u16, Value) {
    let json = json!({"card_id": card, "body": body}).to_string();
    post_json(addr, &format!("/v1/cards/{card}/comments"), &json, true)
}

/// カードの (snapshot_hash, コメント数, card_version)。状態が変わっていないことの確認用。
fn card_state(ledger: &SharedLedger, card: &str) -> (String, usize, u64) {
    let g = ledger.lock().expect("ledger lock");
    let c = g.card(&card.into()).expect("card");
    (c.snapshot_hash.clone(), c.comments.len(), c.card_version)
}

fn approve_json_for(ledger: &SharedLedger, card: &str, key: &str) -> String {
    let g = ledger.lock().expect("ledger lock");
    let c = g.card(&card.into()).expect("card");
    json!({
        "card_id": card,
        "card_version": c.card_version,
        "snapshot_hash": c.snapshot_hash,
        "idempotency_key": key,
    })
    .to_string()
}

/// 台帳を直接触って begin_approve だけ行い、カードを Sending（キーは処理中）のままにする。
fn reserve_directly(ledger: &SharedLedger, card: &str, key: &str) {
    let mut g = ledger.lock().expect("ledger lock");
    let (version, hash) = {
        let c = g.card(&card.into()).expect("card");
        (c.card_version, c.snapshot_hash.clone())
    };
    let req = ApproveRequest {
        card_id: card.into(),
        card_version: version,
        snapshot_hash: hash,
        idempotency_key: IdempotencyKey::new(key).expect("valid key"),
    };
    assert!(matches!(
        g.begin_approve(&req),
        Ok(BeginApproval::Reserved(_))
    ));
}

/// 台帳を直接触って approve まで行い、カードを SentAwaitingReply にする。
fn send_directly(ledger: &SharedLedger, card: &str, key: &str) {
    let mut g = ledger.lock().expect("ledger lock");
    let (version, hash) = {
        let c = g.card(&card.into()).expect("card");
        (c.card_version, c.snapshot_hash.clone())
    };
    let req = ApproveRequest {
        card_id: card.into(),
        card_version: version,
        snapshot_hash: hash,
        idempotency_key: IdempotencyKey::new(key).expect("valid key"),
    };
    g.approve(req).expect("approve");
}

// (a) 承認待ちでないカードへのコメントは 409 not_awaiting_approval。状態も変わらない。
#[test]
fn comments_on_non_awaiting_cards_return_409_and_keep_state() {
    let ledger = shared_ledger(&["card-sending", "card-sent"]);
    reserve_directly(&ledger, "card-sending", "a-sending-key-000001");
    send_directly(&ledger, "card-sent", "a-sent-key-000000001");
    let addr = spawn_server(ledger.clone());
    let sending_before = card_state(&ledger, "card-sending");
    let sent_before = card_state(&ledger, "card-sent");

    for (card, state) in [
        ("card-sending", "sending"),
        ("card-sent", "sent_awaiting_reply"),
    ] {
        for body in [" \n ".to_owned(), "あ".repeat(4001)] {
            let (status, json) = post_comments(addr, card, &body);
            assert_eq!(status, 409, "{card} {body:?}");
            assert_eq!(json["code"], "not_awaiting_approval", "{card} {body:?}");
            assert_eq!(json["state"], state, "{card} {body:?}");
        }
    }
    assert_eq!(card_state(&ledger, "card-sending"), sending_before);
    assert_eq!(card_state(&ledger, "card-sent"), sent_before);
}

// (b) 承認待ちのカードに同じ本文を送ると 422。
#[test]
fn comments_on_awaiting_card_with_same_bodies_return_422() {
    let ledger = shared_ledger(&["card-await"]);
    let addr = spawn_server(ledger.clone());

    let (status, json) = post_comments(addr, "card-await", " \n ");
    assert_eq!(status, 422);
    assert_eq!(json["code"], "empty_body");

    let (status, json) = post_comments(addr, "card-await", &"あ".repeat(4001));
    assert_eq!(status, 422);
    assert_eq!(json["code"], "too_long");
    assert_eq!(json["max_chars"], 4000);
}

// (c) 国旗絵文字でちょうど 4000 コードポイントは受理、4001 は 422 too_long。
#[test]
fn flag_emoji_4000_code_points_ok_and_4001_too_long() {
    let ledger = shared_ledger(&["card-flag"]);
    let addr = spawn_server(ledger.clone());

    let flag = "\u{1F1EF}\u{1F1F5}";
    let exact = flag.repeat(2000);
    let (status, json) = post_comments(addr, "card-flag", &exact);
    assert_eq!(status, 200);
    assert_eq!(json["card_id"], "card-flag");
    assert_eq!(json["card_version"], 1);
    assert_eq!(json["state"], "awaiting_send_approval");

    let (status, json) = post_comments(addr, "card-flag", &format!("{exact}\u{1F1EF}"));
    assert_eq!(status, 422);
    assert_eq!(json["code"], "too_long");
    assert_eq!(json["max_chars"], 4000);
}

// (d) approve の approval_in_progress（409）と idempotency_key_reused（422）。
#[test]
fn approve_progress_and_key_reuse_map_to_409_and_422() {
    let ledger = shared_ledger(&["card-inprog", "card-reuse"]);
    reserve_directly(&ledger, "card-inprog", "d-inprog-key-0000001");
    let addr = spawn_server(ledger.clone());

    // 処理中のキー・同じ内容の再送: 409 approval_in_progress。
    let body = approve_json_for(&ledger, "card-inprog", "d-inprog-key-0000001");
    let (status, json) = post_json(addr, "/v1/cards/card-inprog/approve", &body, false);
    assert_eq!(status, 409);
    assert_eq!(json["code"], "approval_in_progress");

    // 完了済みキーを別内容（版を 1 つ進めた）で使う: 422 idempotency_key_reused。
    let body = approve_json_for(&ledger, "card-reuse", "d-reuse-key-000000001");
    let (status, _) = post_json(addr, "/v1/cards/card-reuse/approve", &body, false);
    assert_eq!(status, 200);
    let (_, _, version) = card_state(&ledger, "card-reuse");
    let hash = card_state(&ledger, "card-reuse").0;
    let body = json!({
        "card_id": "card-reuse",
        "card_version": version + 1,
        "snapshot_hash": hash,
        "idempotency_key": "d-reuse-key-000000001",
    })
    .to_string();
    let (status, json) = post_json(addr, "/v1/cards/card-reuse/approve", &body, false);
    assert_eq!(status, 422);
    assert_eq!(json["code"], "idempotency_key_reused");
}

// (e) 同じカードへの approve を複数スレッドから同時に送っても、模擬送信は 1 件だけ。
#[test]
fn concurrent_approves_send_exactly_once() {
    let ledger = shared_ledger(&["card-conc"]);
    let addr = spawn_server(ledger.clone());
    let n = 8;
    let barrier = Arc::new(Barrier::new(n));
    let (tx, rx) = std::sync::mpsc::channel();

    let mut handles = Vec::new();
    for i in 0..n {
        let ledger = Arc::clone(&ledger);
        let barrier = Arc::clone(&barrier);
        let tx = tx.clone();
        handles.push(thread::spawn(move || {
            // 全スレッド同じ内容（版 1・予約前の hash）、鍵だけ別。
            let body = approve_json_for(&ledger, "card-conc", &format!("e-conc-{i:02}-key-00001"));
            barrier.wait();
            let (status, _) = post_json(addr, "/v1/cards/card-conc/approve", &body, false);
            tx.send(status).expect("send status");
        }));
    }
    drop(tx);
    for h in handles {
        h.join().expect("join");
    }
    let statuses: Vec<u16> = rx.iter().collect();
    assert_eq!(statuses.len(), n);
    assert_eq!(statuses.iter().filter(|s| **s == 200).count(), 1);
    assert!(statuses.iter().all(|s| *s == 200 || *s == 409));
    assert_eq!(ledger.lock().expect("lock").send_log().len(), 1);
}

// (f) 不正な JSON は 400、上限超えは 413。どちらも状態を変えない。
#[test]
fn malformed_json_400_and_oversized_413_keep_state() {
    let ledger = shared_ledger(&["card-f"]);
    let addr = spawn_server(ledger.clone());
    let before = card_state(&ledger, "card-f");

    let (status, json) = post_json(
        addr,
        "/v1/cards/card-f/comments",
        "{\"card_id\": \"card-f\", ",
        true,
    );
    assert_eq!(status, 400);
    assert_eq!(json["code"], "bad_request");

    let (status, body) = http(
        addr,
        "POST",
        "/v1/cards/card-f/comments",
        &[
            ("Content-Type", "application/json"),
            ("x-pelway-member-id", "m-human-http"),
            ("x-pelway-member-kind", "human"),
        ],
        &vec![b'x'; MAX_BODY_BYTES + 1],
    );
    assert_eq!(status, 413);
    let json: Value = serde_json::from_str(&body).expect("413 json body");
    assert_eq!(json["code"], "payload_too_large");

    assert_eq!(card_state(&ledger, "card-f"), before);
}

// (g) ループバック以外の待ち受けアドレスでの起動は拒否される。
#[test]
fn serve_refuses_non_loopback_addresses() {
    let ledger = shared_ledger(&[]);
    for bad in ["0.0.0.0:0", "192.168.0.1:8080", "[::]:0"] {
        let addr: SocketAddr = bad.parse().unwrap();
        assert!(serve(addr, Arc::clone(&ledger)).is_err(), "{bad}");
    }
    // ループバック（::1）は受理する。
    let ok = serve("[::1]:0".parse().unwrap(), ledger).expect("::1 is loopback");
    assert!(ok.local_addr().ip().is_loopback());
}

// (h) ロックが poison したら 500。状態には触らない。
#[test]
fn poisoned_lock_returns_500() {
    let ledger = shared_ledger(&["card-p"]);
    let addr = spawn_server(Arc::clone(&ledger));

    // 別スレッドでロックを保持したまま panic させ、poison させる。
    let poison_ledger = Arc::clone(&ledger);
    let _ = thread::spawn(move || {
        let _guard = poison_ledger.lock().expect("lock before poison");
        panic!("poison the ledger mutex");
    })
    .join();
    assert!(ledger.is_poisoned());

    let (status, json) = post_comments(addr, "card-p", "poison 後のコメント");
    assert_eq!(status, 500);
    assert_eq!(json["code"], "internal");
}

// (i) 経路の card_id と本文の card_id の食い違いは 422 card_id_mismatch。
#[test]
fn path_body_card_id_mismatch_returns_422() {
    let ledger = shared_ledger(&["card-await"]);
    let addr = spawn_server(ledger.clone());

    // comments: 本文の card_id が経路と食い違う。
    let json = json!({"card_id": "card-other", "body": "本文"}).to_string();
    let (status, json) = post_json(addr, "/v1/cards/card-await/comments", &json, true);
    assert_eq!(status, 422);
    assert_eq!(json["code"], "card_id_mismatch");

    // approve: 既存の check_path に従う。
    let (_, _, version) = card_state(&ledger, "card-await");
    let hash = card_state(&ledger, "card-await").0;
    let json = json!({
        "card_id": "card-other",
        "card_version": version,
        "snapshot_hash": hash,
        "idempotency_key": "i-mismatch-key-0000001",
    })
    .to_string();
    let (status, json) = post_json(addr, "/v1/cards/card-await/approve", &json, false);
    assert_eq!(status, 422);
    assert_eq!(json["code"], "card_id_mismatch");
}
