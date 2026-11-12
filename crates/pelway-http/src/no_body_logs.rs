//! GDPR-07「本文を残さないログ」の検証（QA 用）。
//!
//! もともと `tests/no_body_logs.rs` の結合テストだったが、r3 で
//! `Logger::with_sink` をクレート外に公開しない形（`pub(crate)`）にしたのに伴い、
//! src 内の `#[cfg(test)]` テストとして移した（検証内容は無変更）。
//!
//! `127.0.0.1:0` で起動したサーバーに `std::net::TcpStream` で実際に HTTP/1.1
//! 要求を送り、メモリ出力（[`MemorySink`]）のログに canary（合成本文・合成秘密・
//! 合成 `client_msg_id`）と生の ID が出ないことを確かめる。
//!
//! 接続先はループバック（127.0.0.1）だけ。外部への接続は無い。

use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use pelway_contract::ledger::{ApprovalLedger, NewCard};
use pelway_contract::pseudonym::pseudonymize;
use pelway_contract::snapshot::ActionPayload;
use pelway_contract::task::ActionKind;
use serde_json::{Value, json};

use crate::log::{FileSink, LogSink, Logger, MemorySink};
use crate::{SharedLedger, serve_with_logger};

/// 本文・コメント用の合成 canary（実在の値ではない）。
const CANARY_BODY: &str = "PELWAY-CANARY-BODY-7Q2M9X";
/// 秘密の代わりの合成 canary（鍵や API キーではない）。
const CANARY_SECRET: &str = "PELWAY-CANARY-SECRET-4K8T1Z";
/// `client_msg_id` 用の合成 canary（UUIDv4 の形の合成値）。
const CANARY_MSG_ID: &str = "3f6c2a9e-8b1d-4c7a-9e2f-5a1b7c3d9e04";

/// テストで使う固定 salt（salt の作り方自体は実装側の `process_salt` の単体テストで確認）。
const SALT: &[u8] = b"no-body-logs-test-salt";

/// テキスト中の canary の出現回数（3 種の合計）。
fn canary_hits(text: &str) -> usize {
    [CANARY_BODY, CANARY_SECRET, CANARY_MSG_ID]
        .into_iter()
        .map(|c| text.matches(c).count())
        .sum()
}

fn assert_no_canary(text: &str, label: &str) {
    assert_eq!(canary_hits(text), 0, "{label} に canary がある: {text}");
}

fn new_card(id: &str) -> NewCard {
    NewCard {
        workspace_id: "ws-synthetic-gdpr07".into(),
        channel_id: "ch-synthetic-gdpr07".into(),
        card_id: id.into(),
        payload: ActionPayload {
            action_kind: ActionKind::CustomerSend,
            recipient: "customer@example.invalid".to_owned(),
            body: format!("合成データの本文 {CANARY_BODY}"),
            attachments: Vec::new(),
            account: "acct-cw-1".to_owned(),
            scope: Vec::new(),
        },
        requester: "m-human-gdpr07".into(),
        decider: "m-human-gdpr07".into(),
        deadline: None,
    }
}

/// メモリ出力のロガー付きサーバーを `127.0.0.1:0` で立てる。
fn spawn_logged(cards: &[&str], sink: &Arc<MemorySink>) -> (SocketAddr, SharedLedger) {
    let mut ledger = ApprovalLedger::new();
    for id in cards {
        ledger.create_card(new_card(id)).expect("new card");
    }
    let shared: SharedLedger = Arc::new(Mutex::new(ledger));
    let logger = Arc::new(Logger::with_sink(
        Arc::clone(sink) as Arc<dyn LogSink>,
        SALT.to_vec(),
    ));
    let addr = serve_with_logger("127.0.0.1:0".parse().unwrap(), Arc::clone(&shared), logger)
        .expect("serve on 127.0.0.1:0")
        .local_addr();
    assert_eq!(addr.ip().to_string(), "127.0.0.1");
    (addr, shared)
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

fn comment_headers() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Content-Type", "application/json"),
        ("x-pelway-member-id", "m-human-gdpr07"),
        ("x-pelway-member-kind", "human"),
    ]
}

fn post_comment(addr: SocketAddr, card: &str, body: &str) -> (u16, String) {
    let json = json!({"card_id": card, "body": body}).to_string();
    http(
        addr,
        "POST",
        &pelway_contract::approval::comments_path(&card.into()),
        &comment_headers(),
        json.as_bytes(),
    )
}

/// 台帳を直接読んで、カードの現在の (card_version, snapshot_hash) で承認 JSON を作る。
fn approve_json(
    ledger: &SharedLedger,
    card: &str,
    key: &str,
    hash_override: Option<&str>,
) -> String {
    let g = ledger.lock().expect("ledger lock");
    let c = g.card(&card.into()).expect("card");
    json!({
        "card_id": card,
        "card_version": c.card_version,
        "snapshot_hash": hash_override.unwrap_or(&c.snapshot_hash),
        "idempotency_key": key,
    })
    .to_string()
}

fn post_approve(addr: SocketAddr, body: &str) -> (u16, String) {
    http(
        addr,
        "POST",
        &pelway_contract::approval::approve_path(&"card-flow-1".into()),
        &[("Content-Type", "application/json")],
        body.as_bytes(),
    )
}

/// 別スレッドでロックを保持したまま panic させ、台帳を poison させる。
fn poison(ledger: &SharedLedger) {
    let target = Arc::clone(ledger);
    let _ = thread::spawn(move || {
        let _guard = target.lock().expect("lock before poison");
        panic!("poison the ledger mutex");
    })
    .join();
    assert!(ledger.is_poisoned());
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

// (a) canary を含む合成要求を流し、ログに canary が 0 件で、出来事が 1 件以上あること。
#[test]
fn logs_have_no_canary_and_at_least_one_event() {
    let sink = Arc::new(MemorySink::new());
    let (addr, _ledger) = spawn_logged(&["card-log-a"], &sink);

    let body =
        json!({"card_id": "card-log-a", "body": format!("確認しました {CANARY_BODY}")}).to_string();
    let mut headers = comment_headers();
    headers.push(("x-synthetic-secret", CANARY_SECRET));
    let (status, resp_body) = http(
        addr,
        "POST",
        &pelway_contract::approval::comments_path(&"card-log-a".into()),
        &headers,
        body.as_bytes(),
    );
    assert_eq!(status, 200);
    assert_no_canary(&resp_body, "応答本文");

    let lines = sink.lines();
    assert!(!lines.is_empty(), "ログが1件も出ていない");
    assert_no_canary(&lines.join("\n"), "ログ");
    // 各行が JSON Lines として妥当なことは kinds() のパースで確認する。
    assert_eq!(kinds(&lines).len(), lines.len());
}

// (b) 生の合成 ID がログに 0 件で、仮名化した ID があること。
#[test]
fn raw_ids_absent_and_pseudonyms_present() {
    let sink = Arc::new(MemorySink::new());
    let (addr, _ledger) = spawn_logged(&["card-log-b"], &sink);

    let (status, _) = post_comment(addr, "card-log-b", "承認前の確認コメント");
    assert_eq!(status, 200);

    let joined = sink.lines().join("\n");
    assert!(!joined.contains("card-log-b"), "生の card_id がログにある");
    assert!(
        !joined.contains("m-human-gdpr07"),
        "生の member_id がログにある"
    );
    assert!(
        joined.contains(&pseudonymize(SALT, "card-log-b")),
        "仮名化した card_id がない: {joined}"
    );
    assert!(
        joined.contains(&pseudonymize(SALT, "m-human-gdpr07")),
        "仮名化した member_id がない: {joined}"
    );
    assert_no_canary(&joined, "ログ");
}

// (c) 拒否・内部エラーで、応答本文とログのどちらにも canary が 0 件であること。
#[test]
fn rejected_and_internal_error_have_no_canary_anywhere() {
    let sink = Arc::new(MemorySink::new());
    let (addr, ledger) = spawn_logged(&["card-rej"], &sink);

    // 409: snapshot_hash に秘密 canary を入れた承認（拒否）。
    let body = approve_json(
        &ledger,
        "card-rej",
        "c-409-key-000000001",
        Some(CANARY_SECRET),
    );
    let (status, resp) = http(
        addr,
        "POST",
        &pelway_contract::approval::approve_path(&"card-rej".into()),
        &[("Content-Type", "application/json")],
        body.as_bytes(),
    );
    assert_eq!(status, 409);
    assert!(resp.contains("snapshot_mismatch"), "{resp}");
    assert_no_canary(&resp, "409 応答本文");

    // 400: 未知フィールド client_msg_id canary 入りのコメント（拒否）。
    let body = json!({
        "card_id": "card-rej",
        "body": "本文",
        "client_msg_id": CANARY_MSG_ID,
    })
    .to_string();
    let (status, resp) = http(
        addr,
        "POST",
        &pelway_contract::approval::comments_path(&"card-rej".into()),
        &comment_headers(),
        body.as_bytes(),
    );
    assert_eq!(status, 400);
    assert_no_canary(&resp, "400 応答本文");

    // 500: ロックが poison した内部エラー。本文は {"code":"internal"} だけ。
    poison(&ledger);
    let (status, resp) = post_comment(addr, "card-rej", "poison 後のコメント");
    assert_eq!(status, 500);
    assert_eq!(resp, "{\"code\":\"internal\"}");
    assert_no_canary(&resp, "500 応答本文");

    let lines = sink.lines();
    assert!(!lines.is_empty(), "拒否・内部エラーでもログが出ていない");
    assert_no_canary(&lines.join("\n"), "ログ");
    let ks = kinds(&lines);
    assert!(
        ks.iter().any(|k| k == "rejected"),
        "rejected がない: {ks:?}"
    );
    assert!(
        ks.iter().any(|k| k == "internal_error"),
        "internal_error がない: {ks:?}"
    );
}

// (d) 陽性対照: canary を含む文字列を検査にかけると検出されること。
#[test]
fn canary_scan_detects_known_canaries() {
    let all_three = format!("x {CANARY_BODY} y {CANARY_SECRET} z {CANARY_MSG_ID}");
    assert_eq!(canary_hits(&all_three), 3);
    assert_eq!(canary_hits(CANARY_BODY), 1);
    assert_eq!(canary_hits(CANARY_SECRET), 1);
    assert_eq!(canary_hits(CANARY_MSG_ID), 1);
    assert_eq!(canary_hits("canary を含まない合成データ 追記: 1"), 0);
    assert_eq!(
        canary_hits("PELWAY-CANARY-BODY-7Q2M9"),
        0,
        "部分一致では拾わない"
    );
}

// (e) ファイル出力の先が target/demo-logs/ の下であること（worktree の外を指さないこと）。
#[test]
fn file_sink_path_is_under_target_demo_logs() {
    let sink = FileSink::create("backend.jsonl").expect("create sink");
    let path = sink.path();
    assert_eq!(
        path,
        std::path::Path::new("target/demo-logs").join("backend.jsonl")
    );
    assert!(
        !path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    );
    // ディレクトリを含む名前・トラバーサルは拒否。
    for bad in ["", ".", "..", "../escape.jsonl", "a/b.jsonl", "a\\b.jsonl"] {
        assert!(FileSink::create(bad).is_err(), "{bad:?}");
    }
}

// (f) 一連の流れで、受けた・応答した・承認した・模擬送信した・拒否した・内部エラーが
// それぞれ 1 件以上出ること。模擬送信 ID も仮名化されていること。
#[test]
fn full_flow_emits_all_event_kinds_with_pseudonyms_only() {
    let sink = Arc::new(MemorySink::new());
    let (addr, ledger) = spawn_logged(&["card-flow-1"], &sink);

    let (status, _) = post_comment(addr, "card-flow-1", "送信前の最終確認");
    assert_eq!(status, 200);

    let body = approve_json(&ledger, "card-flow-1", "f-approve-key-0000001", None);
    let (status, _) = post_approve(addr, &body);
    assert_eq!(status, 200);

    let (status, _) = post_comment(addr, "card-flow-1", "送信済みへのコメント");
    assert_eq!(status, 409);

    poison(&ledger);
    let (status, _) = post_comment(addr, "card-flow-1", "poison 後");
    assert_eq!(status, 500);

    let lines = sink.lines();
    let joined = lines.join("\n");
    assert_no_canary(&joined, "ログ");
    for id in ["card-flow-1", "m-human-gdpr07", "sim-00000001"] {
        assert!(!joined.contains(id), "生の ID {id} がログにある");
    }
    let ks = kinds(&lines);
    for want in [
        "request_received",
        "responded",
        "comment_saved",
        "approved",
        "simulated_send",
        "rejected",
        "internal_error",
    ] {
        assert!(ks.iter().any(|k| k == want), "{want} がない: {ks:?}");
    }
    assert_eq!(ks.iter().filter(|k| *k == "simulated_send").count(), 1);

    // approved と simulated_send の card は同じ仮名で、send も仮名（pid: 始まり）。
    let mut approved_card = None;
    let mut send_card = None;
    let mut send_value = None;
    for line in &lines {
        let v: Value = serde_json::from_str(line).expect("json");
        match v["kind"].as_str() {
            Some("approved") => approved_card = v["card"].as_str().map(str::to_owned),
            Some("simulated_send") => {
                send_card = v["card"].as_str().map(str::to_owned);
                send_value = v["send"].as_str().map(str::to_owned);
            }
            _ => {}
        }
    }
    let approved_card = approved_card.expect("approved event");
    let send_card = send_card.expect("simulated_send event");
    let send_value = send_value.expect("simulated_send event");
    assert_eq!(approved_card, send_card);
    assert!(approved_card.starts_with("pid:"), "{approved_card}");
    assert!(send_value.starts_with("pid:"), "{send_value}");
    assert_ne!(send_value, "sim-00000001");
}

// (g) に相当する salt 読み取り失敗の注入テスト (s2) は、salt のパスを受ける入口を
// クレート外に公開しない形にしたため、`src/lib.rs` の単体テストに移した
// （`serve_with_failing_salt_source_returns_err_without_listening`）。
