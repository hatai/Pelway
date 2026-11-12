//! HTTP 層の結合テスト（QA 用）。`127.0.0.1:0` で起動したサーバーに
//! `std::net::TcpStream` で実際に HTTP/1.1 要求を送り、状態コードと本文 JSON を確かめる。
//!
//! 接続先はループバック（127.0.0.1 / ::1）だけ。外部への接続は無い。

use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use pelway_contract::approval::{ApproveRequest, approve_path, comments_path};
use pelway_contract::ids::IdempotencyKey;
use pelway_contract::ledger::{ApprovalLedger, BeginApproval, NewCard};
use pelway_contract::snapshot::ActionPayload;
use pelway_contract::task::ActionKind;
use pelway_http::{
    CONNECTION_DEADLINE, MAX_BODY_BYTES, MAX_HEAD_BYTES, SharedLedger, serve,
    serve_with_connection_deadline,
};
use serde_json::{Value, json};

fn new_card(id: &str) -> NewCard {
    NewCard {
        workspace_id: "ws-synthetic".into(),
        channel_id: "ch-synthetic".into(),
        card_id: id.into(),
        payload: ActionPayload {
            action_kind: ActionKind::CustomerSend,
            recipient: "customer@example.invalid".to_owned(),
            body: format!("合成データの本文 {id}"),
            attachments: Vec::new(),
            account: "acct-cw-1".to_owned(),
            scope: Vec::new(),
        },
        requester: "m-human-http".into(),
        decider: "m-human-http".into(),
        deadline: None,
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
    // 経路は契約の定数（COMMENTS_PATH）から作る（Issue #91: 定数と経路の一致）。
    post_json(addr, &comments_path(&card.into()), &json, true)
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

// (a0) 契約の経路定数（COMMENTS_PATH / APPROVE_PATH の `{card_id}` 置換）で
// サーバーが受け付けること。旧・仮経路 `/v1/cards/...` はもう経路が無いこと。
// このテストは経路を文字列リテラルで書かず comments_path / approve_path で作る
// （定数とサーバーの経路の一致を構造的に確かめる）。
#[test]
fn approval_routes_match_contract_path_constants() {
    let ledger = shared_ledger(&["card-route"]);
    let addr = spawn_server(ledger.clone());

    // comments_path（定数 COMMENTS_PATH 由来）でコメント保存 → 200。
    let json = json!({"card_id": "card-route", "body": "送信前に確認"}).to_string();
    let (status, body) = post_json(addr, &comments_path(&"card-route".into()), &json, true);
    assert_eq!(status, 200);
    assert_eq!(body["card_id"], "card-route");
    assert_eq!(body["state"], "awaiting_send_approval");

    // approve_path（定数 APPROVE_PATH 由来）で承認 → 200。
    let json = approve_json_for(&ledger, "card-route", "a0-route-key-0000001");
    let (status, _) = post_json(addr, &approve_path(&"card-route".into()), &json, false);
    assert_eq!(status, 200);

    // 旧・仮経路 /v1/cards/... は 404（NotFound）。
    let json = json!({"card_id": "card-route", "body": "旧経路"}).to_string();
    let (status, json) = post_json(addr, "/v1/cards/card-route/comments", &json, true);
    assert_eq!(status, 404);
    assert_eq!(json["code"], "not_found");
}

// (a1) x-pelway-member-kind は仕様 04 §2 の 4 値を受け付ける。旧・デモの `ai` は 400。
#[test]
fn member_kind_header_accepts_four_spec_kinds_and_rejects_ai() {
    let ledger = shared_ledger(&["card-kind"]);
    let addr = spawn_server(ledger.clone());
    let path = comments_path(&"card-kind".into());
    for kind in ["human", "bot", "external_agent", "external_tool"] {
        let json = json!({"card_id": "card-kind", "body": "確認"}).to_string();
        let headers = [
            ("Content-Type", "application/json"),
            ("x-pelway-member-id", "m-human-http"),
            ("x-pelway-member-kind", kind),
        ];
        let (status, body) = http(addr, "POST", &path, &headers, json.as_bytes());
        assert_eq!(status, 200, "{kind}");
        let body: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
        assert_eq!(body["card_id"], "card-kind", "{kind}");
    }
    // 旧・デモの 2 値の `ai` は未知の値として 400（状態は変えない）。
    let json = json!({"card_id": "card-kind", "body": "旧デモの種別"}).to_string();
    let headers = [
        ("Content-Type", "application/json"),
        ("x-pelway-member-id", "m-human-http"),
        ("x-pelway-member-kind", "ai"),
    ];
    let (status, body) = http(addr, "POST", &path, &headers, json.as_bytes());
    assert_eq!(status, 400);
    let body: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
    assert_eq!(body["code"], "bad_request");
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
    let (status, json) = post_json(addr, &approve_path(&"card-inprog".into()), &body, false);
    assert_eq!(status, 409);
    assert_eq!(json["code"], "approval_in_progress");

    // 完了済みキーを別内容（版を 1 つ進めた）で使う: 422 idempotency_key_reused。
    let body = approve_json_for(&ledger, "card-reuse", "d-reuse-key-000000001");
    let (status, _) = post_json(addr, &approve_path(&"card-reuse".into()), &body, false);
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
    let (status, json) = post_json(addr, &approve_path(&"card-reuse".into()), &body, false);
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
            let (status, _) = post_json(addr, &approve_path(&"card-conc".into()), &body, false);
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
        &comments_path(&"card-f".into()),
        "{\"card_id\": \"card-f\", ",
        true,
    );
    assert_eq!(status, 400);
    assert_eq!(json["code"], "bad_request");

    let (status, body) = http(
        addr,
        "POST",
        &comments_path(&"card-f".into()),
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

// (g) 127.0.0.1 以外の待ち受けアドレスでの起動は拒否される。
// 127.0.0.2 など 127/8 のほかのアドレスも拒否（リード決定 2026-11-09 19:14）。
#[test]
fn serve_refuses_non_loopback_addresses() {
    let ledger = shared_ledger(&[]);
    for bad in [
        "0.0.0.0:0",
        "192.168.0.1:8080",
        "[::]:0",
        "[::1]:0",
        "127.0.0.2:0",
        "127.255.255.255:0",
    ] {
        let addr: SocketAddr = bad.parse().unwrap();
        assert!(serve(addr, Arc::clone(&ledger)).is_err(), "{bad}");
    }
}

// (h) ロックが poison したら 500。状態には触らない。本文 JSON は code 以外に
// 何も含まない（エラー文・ファイルパス・型名・panic の文を出さない）。
#[test]
fn poisoned_lock_returns_500_with_exact_body_and_keeps_state() {
    let ledger = shared_ledger(&["card-p"]);
    let addr = spawn_server(Arc::clone(&ledger));
    let before = card_state(&ledger, "card-p");

    // 別スレッドでロックを保持したまま panic させ、poison させる。
    let poison_ledger = Arc::clone(&ledger);
    let _ = thread::spawn(move || {
        let _guard = poison_ledger.lock().expect("lock before poison");
        panic!("poison the ledger mutex");
    })
    .join();
    assert!(ledger.is_poisoned());

    let (status, body) = http(
        addr,
        "POST",
        &comments_path(&"card-p".into()),
        &[
            ("Content-Type", "application/json"),
            ("x-pelway-member-id", "m-human-http"),
            ("x-pelway-member-kind", "human"),
        ],
        json!({"card_id": "card-p", "body": "poison 後のコメント"})
            .to_string()
            .as_bytes(),
    );
    assert_eq!(status, 500);
    // 本文のキーと値をすべて確かめる（余分なキーは無いこと）。
    let json: Value = serde_json::from_str(&body).expect("500 body is JSON");
    assert_eq!(json, json!({"code": "internal"}), "body: {body}");
    assert_eq!(body, "{\"code\":\"internal\"}");
    assert!(!body.contains("poison"), "body: {body}");
    assert!(!body.contains("panicked"), "body: {body}");
    assert!(!body.contains("ledger"), "body: {body}");

    // poison したロックは into_inner で読み、状態が変わっていないことを見る。
    let after = {
        let g = ledger
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let c = g.card(&"card-p".into()).expect("card");
        (c.snapshot_hash.clone(), c.comments.len(), c.card_version)
    };
    assert_eq!(after, before);
}

// (i) 経路の card_id と本文の card_id の食い違いは 422 card_id_mismatch。
#[test]
fn path_body_card_id_mismatch_returns_422() {
    let ledger = shared_ledger(&["card-await"]);
    let addr = spawn_server(ledger.clone());

    // comments: 本文の card_id が経路と食い違う。
    let json = json!({"card_id": "card-other", "body": "本文"}).to_string();
    let (status, json) = post_json(addr, &comments_path(&"card-await".into()), &json, true);
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
    let (status, json) = post_json(addr, &approve_path(&"card-await".into()), &json, false);
    assert_eq!(status, 422);
    assert_eq!(json["code"], "card_id_mismatch");
}

/// 完全な生の要求を書き、(状態コード, 応答本文の JSON) を読む。
fn http_raw(addr: SocketAddr, raw: &str) -> (u16, Value) {
    let mut stream = TcpStream::connect(addr).expect("connect");
    stream.write_all(raw.as_bytes()).expect("write raw");
    let _ = stream.shutdown(Shutdown::Write);
    let mut buf = String::new();
    stream.read_to_string(&mut buf).expect("read response");
    let status = buf
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("status line");
    let body = buf.split("\r\n\r\n").nth(1).unwrap_or("").to_owned();
    let json = serde_json::from_str(&body).unwrap_or(Value::Null);
    (status, json)
}

// (j) Content-Length が上限を超えたら、本文を読み込む前に 413 を返す。
#[test]
fn oversize_content_length_answers_413_before_reading_body() {
    let ledger = shared_ledger(&["card-j"]);
    let addr = spawn_server(ledger.clone());

    // 100000 バイトの本文を「これから送る」とだけ宣言し、本文も接続終了も送らない。
    let mut stream = TcpStream::connect(addr).expect("connect");
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(2)))
        .expect("client read timeout");
    stream
        .write_all(
            b"POST /v1/approvals/card-j/comments HTTP/1.1\r\nHost: demo.local\r\n\
               x-pelway-member-id: m-human-http\r\nx-pelway-member-kind: human\r\n\
               Content-Length: 100000\r\n\r\n",
        )
        .expect("write head");
    // 本文を送らない・接続を閉じないまま、応答を待つ。
    let mut buf = String::new();
    stream
        .read_to_string(&mut buf)
        .expect("413 must arrive without the body being sent");
    assert!(buf.starts_with("HTTP/1.1 413"), "got: {buf}");
    assert!(buf.contains("payload_too_large"), "got: {buf}");
}

// (k) Content-Length は ASCII 数字1つだけ。重複・非数字・負・+付き・TE は 400。
#[test]
fn content_length_must_be_single_decimal_and_te_rejected() {
    let ledger = shared_ledger(&["card-k"]);
    let addr = spawn_server(ledger.clone());

    let body = json!({"card_id": "card-k", "body": "本文"}).to_string();
    // '+' 付き（usize としては妥当でも）数字以外なので 400。正当な本文を送っても受理しない。
    let raw = format!(
        "POST /v1/approvals/card-k/comments HTTP/1.1\r\nHost: d\r\n\
         x-pelway-member-id: m-human-http\r\nx-pelway-member-kind: human\r\n\
         Content-Length: +{}\r\n\r\n{body}",
        body.len()
    );
    let (status, json) = http_raw(addr, &raw);
    assert_eq!(status, 400, "plus-prefixed length must be rejected");
    assert_eq!(json["code"], "bad_request");

    let head = "POST /v1/approvals/card-k/comments HTTP/1.1\r\nHost: d\r\n\
                x-pelway-member-id: m-human-http\r\nx-pelway-member-kind: human\r\n";
    for (label, raw) in [
        ("negative", format!("{head}Content-Length: -5\r\n\r\n")),
        ("non-numeric", format!("{head}Content-Length: abc\r\n\r\n")),
        (
            "duplicated",
            format!("{head}Content-Length: 10\r\nContent-Length: 20\r\n\r\n"),
        ),
        (
            "transfer-encoding",
            format!("{head}Content-Length: 10\r\nTransfer-Encoding: chunked\r\n\r\n"),
        ),
    ] {
        let (status, json) = http_raw(addr, &raw);
        assert_eq!(status, 400, "{label}");
        assert_eq!(json["code"], "bad_request", "{label}");
    }
}

// (l) 途中で止まった接続は READ_TIMEOUT で切断され、ほかの要求を妨げない。
#[test]
fn stalled_connection_times_out_and_does_not_block_others() {
    let ledger = shared_ledger(&["card-l"]);
    let addr = spawn_server(ledger.clone());

    // 接続 A: ヘッダが終わる前に書くのをやめて放置する。
    let mut stalled = TcpStream::connect(addr).expect("connect A");
    stalled
        .write_all(b"POST /v1/approvals/card-l/comments HTTP/1.1\r\nHost: d\r\n")
        .expect("write partial head");
    stalled
        .set_read_timeout(Some(std::time::Duration::from_secs(
            pelway_http::READ_TIMEOUT.as_secs() + 3,
        )))
        .expect("client read timeout");

    // 接続 B: 放置された A があっても、普通に処理される。
    let (status, json) = post_comments(addr, "card-l", "止まった接続があっても保存できる");
    assert_eq!(status, 200, "other connections must be served");
    assert_eq!(json["state"], "awaiting_send_approval");

    // A はサーバーから 408 で切断される。
    let mut buf = String::new();
    stalled
        .read_to_string(&mut buf)
        .expect("server must close the stalled connection");
    assert!(buf.starts_with("HTTP/1.1 408"), "got: {buf}");
    assert!(buf.contains("request_timeout"), "got: {buf}");
}

// (m) ヘッダーの大きさ・行数の上限を超えたら 431。
#[test]
fn header_size_and_line_limits_return_431() {
    let ledger = shared_ledger(&["card-m"]);
    let addr = spawn_server(ledger.clone());

    // 行数上限（junk ヘッダー 120 行 + 通常ヘッダー）。
    let junk: Vec<(&str, String)> = (0..120).map(|i| ("x-junk", format!("v{i}"))).collect();
    let owned: Vec<(&str, &str)> = junk.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let (status, json) = post_json_with_headers(
        addr,
        &comments_path(&"card-m".into()),
        &json!({"card_id": "card-m", "body": "本文"}).to_string(),
        &owned,
    );
    assert_eq!(status, 431, "too many header lines");
    assert_eq!(json["code"], "headers_too_large");

    // 大きさ上限（1 行 20 KiB のヘッダー）。
    let big = "x".repeat(20 * 1024);
    let (status, json) = post_json_with_headers(
        addr,
        &comments_path(&"card-m".into()),
        &json!({"card_id": "card-m", "body": "本文"}).to_string(),
        &[("x-big", big.as_str())],
    );
    assert_eq!(status, 431, "header too large");
    assert_eq!(json["code"], "headers_too_large");
}

/// ヘッダーを差し替えられる comments POST（(m) 用。post_json と同じ読み取り）。
fn post_json_with_headers(
    addr: SocketAddr,
    path: &str,
    body: &str,
    extra: &[(&str, &str)],
) -> (u16, Value) {
    let mut headers: Vec<(&str, &str)> = extra.to_vec();
    headers.push(("x-pelway-member-id", "m-human-http"));
    headers.push(("x-pelway-member-kind", "human"));
    let (status, body) = http(addr, "POST", path, &headers, body.as_bytes());
    let json = serde_json::from_str(&body).unwrap_or(Value::Null);
    (status, json)
}

/// 応答全体（状態行 + ヘッダー + 本文）から (状態コード, 本文の JSON) を取り出す。
fn parse_response(buf: &str) -> (u16, Value) {
    let status = buf
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("status line");
    let body = buf.split("\r\n\r\n").nth(1).unwrap_or("");
    (status, serde_json::from_str(body).unwrap_or(Value::Null))
}

/// ヘッダー全体（最後の空行まで）の合計バイト数が total になるよう、X-Pad の値で
/// 詰め物をした comments 要求のヘッダーを作る（(n) 用）。
fn head_padded_to(total: usize, card: &str, body_len: usize) -> String {
    let pre = format!(
        "POST /v1/approvals/{card}/comments HTTP/1.1\r\nHost: demo.local\r\n\
         Content-Type: application/json\r\n\
         x-pelway-member-id: m-human-http\r\nx-pelway-member-kind: human\r\n"
    );
    let post = format!("Content-Length: {body_len}\r\n\r\n");
    let pad = total - pre.len() - post.len() - "X-Pad: ".len() - "\r\n".len();
    assert!(pad > 0, "no room to pad to {total}");
    let head = format!("{pre}X-Pad: {}\r\n{post}", "p".repeat(pad));
    assert_eq!(head.len(), total);
    head
}

// (n) ヘッダーの合計バイト数は 16 KiB ちょうどまで受理、16 KiB+1 は 431。
// 16 KiB+1 のケースでは終端（空行）までのバイト数が上限を超えるため 431
// （ヘッダーの終端までの長さで判定するため。本文が同じ読み取りに混ざる
// ケースは (r)/(t) で確認する）。
#[test]
fn head_bytes_exactly_16kib_ok_and_one_more_431() {
    let ledger = shared_ledger(&["card-n"]);
    let addr = spawn_server(ledger.clone());
    let body = json!({"card_id": "card-n", "body": "本文"}).to_string();

    // 16 KiB ちょうど: ヘッダーだけを先に送って読み終えてもらい、それから本体を送る。
    let head = head_padded_to(MAX_HEAD_BYTES, "card-n", body.len());
    let mut stream = TcpStream::connect(addr).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("client read timeout");
    stream.write_all(head.as_bytes()).expect("write head");
    thread::sleep(Duration::from_millis(300));
    stream.write_all(body.as_bytes()).expect("write body");
    let _ = stream.shutdown(Shutdown::Write);
    let mut buf = String::new();
    stream.read_to_string(&mut buf).expect("read response");
    let (status, json) = parse_response(&buf);
    assert_eq!(status, 200, "16 KiB ちょうどは受理される: {buf}");
    assert_eq!(json["card_id"], "card-n");
    assert_eq!(json["state"], "awaiting_send_approval");

    // 16 KiB + 1 バイト: 終端（空行）までのバイト数が上限を超えるので 431。
    let head = head_padded_to(MAX_HEAD_BYTES + 1, "card-n", body.len());
    let mut stream = TcpStream::connect(addr).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("client read timeout");
    stream.write_all(head.as_bytes()).expect("write head");
    stream.write_all(body.as_bytes()).expect("write body");
    let _ = stream.shutdown(Shutdown::Write);
    let mut buf = String::new();
    stream.read_to_string(&mut buf).expect("read response");
    let (status, json) = parse_response(&buf);
    assert_eq!(status, 431, "16 KiB+1 は 431: {buf}");
    assert_eq!(json["code"], "headers_too_large");
}

// (o) リクエスト行の HTTP バージョンは HTTP/1.1 の形だけ。ヘッダー名は空・空白・
// 区切り文字入り（RFC 9110 の token でない）なら 400。有効な Content-Length と本文を
// 付けて送っても受理せず、カードの状態（hash・コメント数・版）も変わらない。
#[test]
fn bad_version_and_bad_header_names_return_400_and_keep_state() {
    let ledger = shared_ledger(&["card-o"]);
    let addr = spawn_server(ledger.clone());
    let before = card_state(&ledger, "card-o");
    let body = json!({"card_id": "card-o", "body": "本文"}).to_string();

    let raw_for = |version: &str, extra: &str| {
        format!(
            "POST /v1/approvals/card-o/comments {version}\r\nHost: demo.local\r\n\
             x-pelway-member-id: m-human-http\r\nx-pelway-member-kind: human\r\n\
             {extra}Content-Length: {}\r\n\r\n{body}",
            body.len()
        )
    };
    for (label, raw) in [
        ("HTTP/2.0", raw_for("HTTP/2.0", "")),
        ("HTTP/1.0", raw_for("HTTP/1.0", "")),
        ("HTTP バージョンでない", raw_for("FOO/1.1", "")),
        ("空のヘッダー名", raw_for("HTTP/1.1", ": no name\r\n")),
        (
            "空白を含むヘッダー名",
            raw_for("HTTP/1.1", "X-Bad Name: v\r\n"),
        ),
        ("コロンの前の空白", raw_for("HTTP/1.1", "X-Bad : v\r\n")),
        (
            "区切り文字を含むヘッダー名",
            raw_for("HTTP/1.1", "X-Bad(): v\r\n"),
        ),
    ] {
        let (status, json) = http_raw(addr, &raw);
        assert_eq!(status, 400, "{label}");
        assert_eq!(json["code"], "bad_request", "{label}");
    }
    assert_eq!(card_state(&ledger, "card-o"), before);
}

// (p) 全体期限: 3 秒未満の間隔で少しずつ送り続ける接続は、期限（本番 10 秒）で
// 408 を返して切断される。テストでは期限を短く差し替える。
#[test]
fn slow_trickle_head_hits_connection_deadline() {
    assert_eq!(
        CONNECTION_DEADLINE,
        Duration::from_secs(10),
        "本番の全体期限は 10 秒のまま"
    );
    let ledger = shared_ledger(&[]);
    let addr = serve_with_connection_deadline(
        "127.0.0.1:0".parse().unwrap(),
        ledger,
        Duration::from_millis(800),
    )
    .expect("serve with short deadline")
    .local_addr();

    let mut writer = TcpStream::connect(addr).expect("connect");
    let mut reader = writer.try_clone().expect("clone reader");
    let started = Instant::now();
    let reader_handle = thread::spawn(move || {
        reader
            .set_read_timeout(Some(Duration::from_secs(4)))
            .expect("reader timeout");
        let mut buf = String::new();
        let _ = reader.read_to_string(&mut buf);
        buf
    });
    writer
        .write_all(b"POST /v1/approvals/card-p/comments HTTP/1.1\r\nHost: d\r\n")
        .expect("write first line");
    // 3 秒未満の間隔で送り続けるので、1 回の読み取りタイムアウト（3 秒）には引っかからない。
    for _ in 0..40 {
        thread::sleep(Duration::from_millis(150));
        if writer.write_all(b"X-Trickle: 1\r\n").is_err() {
            break; // サーバーに切断された
        }
    }
    let buf = reader_handle.join().expect("join reader");
    let elapsed = started.elapsed();
    assert!(
        elapsed >= Duration::from_millis(600),
        "期限前に切断された: {elapsed:?}"
    );
    assert!(
        elapsed <= Duration::from_millis(2500),
        "期限前後で切断されない: {elapsed:?}"
    );
    assert!(buf.starts_with("HTTP/1.1 408"), "got: {buf}");
    assert!(buf.contains("request_timeout"), "got: {buf}");
}

/// 本文のバイト数が total ちょうどになる comments 用の JSON を作る
/// （(r)/(t) 用。Content-Length と実際の本文の長さを一致させる）。
fn json_body_padded_to(card: &str, total: usize) -> String {
    let pre = format!("{{\"card_id\": \"{card}\", \"body\": \"");
    let pad = total - pre.len() - "\"}".len();
    assert!(pad > 0, "no room to pad to {total}");
    format!("{pre}{}\"}}", "y".repeat(pad))
}

// (r) ちょうど 16 KiB のヘッダーと有効な本文を 1 回の write で続けて送っても受理される
// （431 にならない。レビュー5 指摘1）。
#[test]
fn head_16kib_and_body_in_one_write_accepted() {
    let ledger = shared_ledger(&["card-r"]);
    let addr = spawn_server(ledger.clone());
    let body = json_body_padded_to("card-r", 2048);
    let head = head_padded_to(MAX_HEAD_BYTES, "card-r", body.len());

    let mut stream = TcpStream::connect(addr).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("client read timeout");
    let mut raw = head.into_bytes();
    raw.extend_from_slice(body.as_bytes());
    stream
        .write_all(&raw)
        .expect("write head+body in one write");
    let _ = stream.shutdown(Shutdown::Write);
    let mut buf = String::new();
    stream.read_to_string(&mut buf).expect("read response");
    let (status, json) = parse_response(&buf);
    assert_eq!(
        status, 200,
        "16 KiB ちょうどのヘッダーと本文が同じ write でも受理: {buf}"
    );
    assert_eq!(json["card_id"], "card-r");
    assert_eq!(json["state"], "awaiting_send_approval");
}

// (s) Content-Length が 64 KiB を超える要求で、ヘッダー（16 KiB ちょうど）と
// 64 KiB を超える本文の全体を 1 回の write_all で送ると、431 ではなく 413 が返る
// （レビュー5 指摘1。レビュー6 指摘2 で「本文の先頭」から「本文の全体」に強化）。
#[test]
fn oversize_cl_head_16kib_whole_body_in_one_write_answers_413() {
    let ledger = shared_ledger(&["card-s"]);
    let addr = spawn_server(ledger.clone());
    let body_len = 100_000usize;
    let head = head_padded_to(MAX_HEAD_BYTES, "card-s", body_len);

    let mut stream = TcpStream::connect(addr).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("client read timeout");
    let mut raw = head.into_bytes();
    raw.extend_from_slice(&vec![b'z'; body_len]);
    stream
        .write_all(&raw)
        .expect("write head+whole body in one write");
    let _ = stream.shutdown(Shutdown::Write);
    let mut buf = String::new();
    stream.read_to_string(&mut buf).expect("read response");
    let (status, json) = parse_response(&buf);
    assert_eq!(
        status, 413,
        "64 KiB を超える本文の全体が同じ write に含まれていても 431 ではなく 413: {buf}"
    );
    assert_eq!(json["code"], "payload_too_large");
}

// (t) 16 KiB ちょうどのヘッダーの終端を含む読み取りに本文のバイトが混ざっても、
// 本文のバイトはヘッダーの上限に数えない（レビュー5 指摘1）。
// サーバーの読み取りの境界を確実にずらすため、ヘッダーを 13000 バイトまで先に送り、
// 残りのヘッダーと本文を 1 回の write で送る。
#[test]
fn body_in_final_head_read_not_counted_in_16kib_limit() {
    let ledger = shared_ledger(&["card-t"]);
    let addr = spawn_server(ledger.clone());
    let body = json_body_padded_to("card-t", 2048);
    let head = head_padded_to(MAX_HEAD_BYTES, "card-t", body.len());
    let (prefix, rest) = head.split_at(13000);

    let mut stream = TcpStream::connect(addr).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("client read timeout");
    stream
        .write_all(prefix.as_bytes())
        .expect("write head prefix");
    // この間にサーバーは 13000 バイトを読み終え、次の読み取りで待機する。
    thread::sleep(Duration::from_millis(300));
    let mut tail = rest.as_bytes().to_vec();
    tail.extend_from_slice(body.as_bytes());
    stream
        .write_all(&tail)
        .expect("write head rest + body in one write");
    let _ = stream.shutdown(Shutdown::Write);
    let mut buf = String::new();
    stream.read_to_string(&mut buf).expect("read response");
    let (status, json) = parse_response(&buf);
    assert_eq!(
        status, 200,
        "最後の読み取りに本文が混ざっても 16 KiB ちょうどのヘッダーは受理: {buf}"
    );
    assert_eq!(json["card_id"], "card-t");
}

// (u) Content-Length が 64 KiB を超える要求では、ヘッダーの終端を含む読み取りに
// 本文の先頭が混ざっていても 431 ではなく 413 を返す（レビュー5 指摘1）。
// (t) と同じく、ヘッダーを 13000 バイトまで先に送って読み取りの境界をずらす。
#[test]
fn oversize_cl_straddled_head_answers_413_not_431() {
    let ledger = shared_ledger(&["card-u"]);
    let addr = spawn_server(ledger.clone());
    let head = head_padded_to(MAX_HEAD_BYTES, "card-u", 100_000);
    let (prefix, rest) = head.split_at(13000);

    let mut stream = TcpStream::connect(addr).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("client read timeout");
    stream
        .write_all(prefix.as_bytes())
        .expect("write head prefix");
    thread::sleep(Duration::from_millis(300));
    let mut tail = rest.as_bytes().to_vec();
    tail.extend_from_slice(&vec![b'z'; 2048]);
    stream
        .write_all(&tail)
        .expect("write head rest + body start in one write");
    let _ = stream.shutdown(Shutdown::Write);
    let mut buf = String::new();
    stream.read_to_string(&mut buf).expect("read response");
    let (status, json) = parse_response(&buf);
    assert_eq!(status, 413, "431 ではなく 413: {buf}");
    assert_eq!(json["code"], "payload_too_large");
}

// (v) 1 回の読み取りのタイムアウトは「3 秒」と「全体期限までの残り」の短いほう。
// ヘッダー読み取り中、期限より前は 3 秒未満の間隔で少しずつ送り続け、期限の直前に
// 読み取り待ちに入ってから何も送らないと、3 秒を待たずに 408 で切る
// （レビュー5 指摘2。レビュー6 指摘3 で「期限の直前に読み取り待ちに入る」形に強化。
// 読み取り待ちの開始が期限から遠いと、全体期限そのものを 1 回のタイムアウトに
// 使う実装でも通ってしまい、実際の条件を検証できないため）。
// 期限 + 0.5 秒以内（QA の受入条件）。
#[test]
fn head_read_wait_is_cut_at_deadline_not_3s() {
    assert_eq!(
        pelway_http::READ_TIMEOUT,
        Duration::from_secs(3),
        "本番の読み取りごとのタイムアウトは 3 秒のまま"
    );
    let deadline = Duration::from_millis(2000);
    let ledger = shared_ledger(&[]);
    let addr = serve_with_connection_deadline("127.0.0.1:0".parse().unwrap(), ledger, deadline)
        .expect("serve with short deadline")
        .local_addr();

    let mut stream = TcpStream::connect(addr).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(6)))
        .expect("client read timeout");
    let started = Instant::now();
    stream
        .write_all(b"POST /v1/approvals/card-v/comments HTTP/1.1\r\nHost: d\r\n")
        .expect("write partial head");
    // 期限より前は 3 秒未満の間隔（300ms）でヘッダー行を少しずつ送り続ける
    // （1 回の読み取りタイムアウトの 3 秒には引っかからない）。最後の送信が
    // 期限の約 0.5 秒前になる時点でやめ、読み取り待ちに入ったまま何も送らない。
    while started.elapsed() + Duration::from_millis(500) < deadline {
        thread::sleep(Duration::from_millis(300));
        if stream.write_all(b"X-Trickle: 1\r\n").is_err() {
            break; // サーバーに切断された
        }
    }
    let mut buf = String::new();
    stream.read_to_string(&mut buf).expect("408 must arrive");
    let elapsed = started.elapsed();
    assert!(buf.starts_with("HTTP/1.1 408"), "got: {buf}");
    assert!(buf.contains("request_timeout"), "got: {buf}");
    // 上限は全体期限 + 0.5 秒（QA の受入条件）。下限は時計とスケジューリングの
    // ぶれのために 0.1 秒だけ許容する（期限前に切れないことの確認）。
    assert!(
        elapsed <= deadline + Duration::from_millis(500),
        "期限から 0.5 秒以内に切れていない: {elapsed:?}"
    );
    assert!(
        elapsed + Duration::from_millis(100) >= deadline,
        "期限前に切れている: {elapsed:?}"
    );
}

// (w) 413 のあとの読み捨て中も同じ: 期限より前は 3 秒未満の間隔で少しずつ送り続け、
// 期限の直前に読み取り待ちに入ってから何も送らないと、期限で接続を閉じる
// （413 は応答済みなので追記は無い。レビュー5 指摘2。レビュー6 指摘3 で
// 「期限の直前に読み取り待ちに入る」形に強化）。
// 期限 + 0.3 秒の時点で接続が既に切れていることを、書き込みのエラーで確かめる。
#[test]
fn drain_read_wait_after_413_is_cut_at_deadline() {
    let deadline = Duration::from_millis(2000);
    let ledger = shared_ledger(&["card-w"]);
    let addr = serve_with_connection_deadline("127.0.0.1:0".parse().unwrap(), ledger, deadline)
        .expect("serve with short deadline")
        .local_addr();

    let mut stream = TcpStream::connect(addr).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .expect("client read timeout");
    let started = Instant::now();
    stream
        .write_all(
            b"POST /v1/approvals/card-w/comments HTTP/1.1\r\nHost: d\r\n\
               Content-Length: 100000\r\n\r\n",
        )
        .expect("write head");
    let mut buf = String::new();
    stream.read_to_string(&mut buf).expect("413 arrives first");
    assert!(buf.starts_with("HTTP/1.1 413"), "got: {buf}");

    // 読み捨て中、期限より前は 3 秒未満の間隔（300ms）で少しずつ送り続ける
    // （1 回の読み取りタイムアウトの 3 秒には引っかからない）。最後の送信が
    // 期限の約 0.5 秒前になる時点でやめ、読み取り待ちに入ったまま何も送らない。
    let mut trickle_failed_at = None;
    while started.elapsed() + Duration::from_millis(500) < deadline {
        thread::sleep(Duration::from_millis(300));
        if stream.write_all(b"trickle").is_err() {
            trickle_failed_at = Some(started.elapsed());
            break; // サーバーに切断された
        }
    }
    // 期限前の切断は失敗: 順当なら最後の trickle は期限の約 0.5 秒前に成功している。
    // 時計とスケジューリングのぶれには 0.1 秒だけ許容する。
    if let Some(t) = trickle_failed_at {
        assert!(
            t + Duration::from_millis(100) >= deadline,
            "期限前に切断された: {t:?}"
        );
    }
    // 期限 + 0.3 秒まで待つ。
    while started.elapsed() < deadline + Duration::from_millis(300) {
        thread::sleep(Duration::from_millis(20));
    }
    // 1 バイト目を送り、少し待ってから 2 回目の書き込みが失敗すれば、1 バイト目を
    // 送った時点で接続が既に閉じられていたことになる（サーバーがまだ読み取り待ちで
    // 残り時間を無視するなら 1 バイト目は読み捨てに消費され、2 回目も成功する。
    // 閉じた後の最初の書き込みに対する RST が戻るのに少し時間が要るため、間を空ける）。
    let first = stream.write_all(b"x");
    thread::sleep(Duration::from_millis(100));
    let second = stream.write_all(b"x");
    assert!(
        second.is_err(),
        "期限の時点で接続が閉じられているべき: first={first:?} second={second:?}"
    );
    assert!(
        started.elapsed() <= deadline + Duration::from_millis(500),
        "期限から 0.5 秒以内に切れていない: {:?}",
        started.elapsed()
    );
}

// (x) 全体期限は本文の読み取りにもかかる（リード決定 2026-11-09 20:36）。
// 正しい大きさの本文を 3 秒未満の間隔で少しずつ送り続けると、全体期限前後で
// 408 と切断が来る（1 回の読み取りごとの 3 秒には引っかからない）。
#[test]
fn slow_trickle_body_hits_connection_deadline() {
    assert_eq!(
        CONNECTION_DEADLINE,
        Duration::from_secs(10),
        "本番の全体期限は 10 秒のまま"
    );
    let deadline = Duration::from_millis(800);
    let ledger = shared_ledger(&["card-x"]);
    let addr = serve_with_connection_deadline("127.0.0.1:0".parse().unwrap(), ledger, deadline)
        .expect("serve with short deadline")
        .local_addr();

    let body = json_body_padded_to("card-x", 240);
    let head = format!(
        "POST /v1/approvals/card-x/comments HTTP/1.1\r\nHost: demo.local\r\n\
         x-pelway-member-id: m-human-http\r\nx-pelway-member-kind: human\r\n\
         Content-Length: {}\r\n\r\n",
        body.len()
    );
    let mut writer = TcpStream::connect(addr).expect("connect");
    let mut reader = writer.try_clone().expect("clone reader");
    let started = Instant::now();
    let reader_handle = thread::spawn(move || {
        reader
            .set_read_timeout(Some(Duration::from_secs(4)))
            .expect("reader timeout");
        let mut buf = String::new();
        let _ = reader.read_to_string(&mut buf);
        buf
    });
    writer.write_all(head.as_bytes()).expect("write head");
    // 3 秒未満の間隔で少しずつ送り続ける（4 バイト × 60 回 = 本文 240 バイト）。
    // 1 回の読み取りタイムアウト（3 秒）には引っかからない。
    for _ in 0..60 {
        thread::sleep(Duration::from_millis(150));
        if writer.write_all(b"yyyy").is_err() {
            break; // サーバーに切断された
        }
    }
    let buf = reader_handle.join().expect("join reader");
    let elapsed = started.elapsed();
    let (status, json) = parse_response(&buf);
    assert_eq!(status, 408, "got: {buf}");
    assert_eq!(json["code"], "request_timeout", "got: {buf}");
    assert!(
        elapsed + Duration::from_millis(100) >= deadline,
        "期限前に切断された: {elapsed:?}"
    );
    assert!(
        elapsed <= deadline + Duration::from_millis(500),
        "期限前後で切断されない: {elapsed:?}"
    );
}

// (y) 本文の読み取り中も同じ: 期限の直前に読み取り待ちに入ってから何も送らず、
// 接続が切れるまでの時間が全体期限 + 0.5 秒以内に収まる（リード決定 20:36、
// リード補足 20:43）。本文を少し送って読み取り待ちにしてから待つ。
#[test]
fn body_read_wait_is_cut_at_deadline_not_3s() {
    let deadline = Duration::from_millis(2000);
    let ledger = shared_ledger(&["card-y"]);
    let addr = serve_with_connection_deadline("127.0.0.1:0".parse().unwrap(), ledger, deadline)
        .expect("serve with short deadline")
        .local_addr();

    let mut stream = TcpStream::connect(addr).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(6)))
        .expect("client read timeout");
    let started = Instant::now();
    let head = "POST /v1/approvals/card-y/comments HTTP/1.1\r\nHost: d\r\n\
                x-pelway-member-id: m-human-http\r\nx-pelway-member-kind: human\r\n\
                Content-Length: 100\r\n\r\n";
    stream.write_all(head.as_bytes()).expect("write head");
    // 本文の一部だけを少しの間隔で送り、期限の直前（約 0.5 秒前）に読み取り待ちに
    // 入ったまま何も送らない。1 回の読み取りタイムアウト（3 秒）には引っかからない。
    for _ in 0..5 {
        stream.write_all(&[b'z'; 10]).expect("write partial body");
        thread::sleep(Duration::from_millis(300));
    }
    let mut buf = String::new();
    stream.read_to_string(&mut buf).expect("408 must arrive");
    let elapsed = started.elapsed();
    assert!(buf.starts_with("HTTP/1.1 408"), "got: {buf}");
    assert!(buf.contains("request_timeout"), "got: {buf}");
    // 上限は全体期限 + 0.5 秒（QA の受入条件）。下限は時計とスケジューリングの
    // ぶれのために 0.1 秒だけ許容する（期限前に切れないことの確認）。
    assert!(
        elapsed <= deadline + Duration::from_millis(500),
        "期限から 0.5 秒以内に切れていない: {elapsed:?}"
    );
    assert!(
        elapsed + Duration::from_millis(100) >= deadline,
        "期限前に切れている: {elapsed:?}"
    );
}

// (q) 413 のあとの読み捨ても全体期限で打ち切る。3 秒未満の間隔で送り続けると、
// 期限で接続を閉じられる（読み捨ての 1 MiB 上限はそのまま）。
#[test]
fn slow_trickle_after_413_stops_draining_at_deadline() {
    let ledger = shared_ledger(&["card-q"]);
    let addr = serve_with_connection_deadline(
        "127.0.0.1:0".parse().unwrap(),
        ledger,
        Duration::from_millis(800),
    )
    .expect("serve with short deadline")
    .local_addr();

    let mut stream = TcpStream::connect(addr).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .expect("client read timeout");
    stream
        .write_all(
            b"POST /v1/approvals/card-q/comments HTTP/1.1\r\nHost: d\r\n\
               Content-Length: 100000\r\n\r\n",
        )
        .expect("write head");
    let mut buf = String::new();
    stream.read_to_string(&mut buf).expect("413 arrives first");
    assert!(buf.starts_with("HTTP/1.1 413"), "got: {buf}");

    // 応答のあと、3 秒未満の間隔で少しずつ送り続ける。
    let started = Instant::now();
    let mut closed_at = None;
    for _ in 0..20 {
        thread::sleep(Duration::from_millis(150));
        if stream.write_all(b"trickle").is_err() {
            closed_at = Some(started.elapsed());
            break; // サーバーに切断された
        }
    }
    let closed_at = closed_at.unwrap_or_else(|| started.elapsed());
    assert!(
        closed_at >= Duration::from_millis(600),
        "期限前に切断された: {closed_at:?}"
    );
    assert!(
        closed_at <= Duration::from_millis(2500),
        "期限前後で切断されない: {closed_at:?}"
    );
}

// (z) 全体期限に Duration::MAX を渡しても panic せず、通常の要求に正しく応える
// （レビュー6 指摘1）。あふれる指定は扱える上限に丸めるため、通常の 200 が返る。
// 接続スレッドで panic するとこの要求への応答が来ないまま接続が切れるので、
// 200 が返ること自体が panic していないことの証明になる。
#[test]
fn duration_max_deadline_does_not_panic_and_serves() {
    let ledger = shared_ledger(&["card-z"]);
    let addr =
        serve_with_connection_deadline("127.0.0.1:0".parse().unwrap(), ledger, Duration::MAX)
            .expect("serve with Duration::MAX deadline")
            .local_addr();

    let (status, json) = post_comments(addr, "card-z", "巨大な期限でも通常どおり保存できる");
    assert_eq!(
        status, 200,
        "Duration::MAX でも panic せず通常の要求に応える"
    );
    assert_eq!(json["card_id"], "card-z");
    assert_eq!(json["state"], "awaiting_send_approval");
}
