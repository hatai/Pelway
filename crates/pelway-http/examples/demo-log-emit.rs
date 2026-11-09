//! 合成データから「本文を残さないログ」を出す手順（GDPR-07）。
//!
//! 実行（ワークスペースルートから。`2>` の差し替え先のため初回は先に
//! `mkdir -p target/demo-logs` が必要。`target/demo-logs/` 自体はこの example
//! が最初のログ出力時に作る）:
//!
//! ```text
//! mkdir -p target/demo-logs
//! cargo run --offline -p pelway-http --example demo-log-emit 2> target/demo-logs/stderr.txt
//! ```
//!
//! - 待ち受けは `127.0.0.1:0` だけ。外部への接続・送信は無い。環境変数は読まない。
//! - 仮名化の salt は起動時に `/dev/urandom` から 32 バイト読む
//!   （`serve_with_logger_salt_from`）。読み取りに失敗したら起動しない。
//! - 合成データに 3 種の canary（本文・コメント用・秘密の代わり・`client_msg_id` 形）
//!   を埋めて要求を流す。ログ（`target/demo-logs/backend.jsonl`）・標準エラー・
//!   エラー応答の本文・panic の文言のどこにも canary が出ないことを、開発基盤の
//!   canary 走査（`rg`）で確かめるためのもの。
//! - 流す要求: コメント保存（200）→ 承認（200）→ 拒否（409・400）→ 内部エラー（500）。
//! - 終わったら止まる（サーバーのスレッドはプロセスと共に終わる。停止 API は無い）。

use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

use pelway_contract::ledger::{ApprovalLedger, NewCard};
use pelway_contract::task::ActionKind;
use pelway_http::log::{FileSink, LogSink, SALT_SOURCE};
use pelway_http::{SharedLedger, serve_with_logger_salt_from};

/// 本文・コメント用の合成 canary（実在の値ではない）。
const CANARY_BODY: &str = "PELWAY-CANARY-BODY-7Q2M9X";
/// 秘密の代わりの合成 canary（鍵や API キーではない。環境変数は読まない）。
const CANARY_SECRET: &str = "PELWAY-CANARY-SECRET-4K8T1Z";
/// `client_msg_id` 用の合成 canary（UUIDv4 の形の合成値）。
const CANARY_MSG_ID: &str = "3f6c2a9e-8b1d-4c7a-9e2f-5a1b7c3d9e04";

fn main() {
    let sink =
        Arc::new(FileSink::create("backend.jsonl").expect("create sink under target/demo-logs"));
    let ledger = synthetic_ledger();
    // salt は serve_with_logger_salt_from が /dev/urandom から 32 バイト読む。
    // 読み取りに失敗したら起動しない（フォールバックなし）。
    let addr = serve_with_logger_salt_from(
        "127.0.0.1:0".parse().expect("loopback addr"),
        Arc::clone(&ledger),
        Arc::clone(&sink) as Arc<dyn LogSink>,
        Path::new(SALT_SOURCE),
    )
    .expect("serve on 127.0.0.1:0")
    .local_addr();
    assert_eq!(addr.ip().to_string(), "127.0.0.1");

    // 1. コメント保存（canary 本文 + canary 秘密ヘッダー）→ 200。
    let body =
        format!("{{\"card_id\": \"card-gdpr07-01\", \"body\": \"確認しました {CANARY_BODY}\"}}");
    let r1 = post(
        addr,
        "/v1/cards/card-gdpr07-01/comments",
        &[
            ("Content-Type", "application/json"),
            ("x-pelway-member-id", "m-human-gdpr07"),
            ("x-pelway-member-kind", "human"),
            ("x-synthetic-secret", CANARY_SECRET),
        ],
        &body,
    );
    expect_status(&r1, 200, "コメント保存");

    // 2. 承認 → 200（承認した・模擬送信した の出来事も出る）。
    let approve = approve_json(&ledger, "card-gdpr07-01", "gdpr07-approve-key-000001", None);
    let r2 = post(
        addr,
        "/v1/cards/card-gdpr07-01/approve",
        &[("Content-Type", "application/json")],
        &approve,
    );
    expect_status(&r2, 200, "承認");

    // 3. 拒否（snapshot_hash に秘密 canary）→ 409。
    let wrong = approve_json(
        &ledger,
        "card-gdpr07-02",
        "gdpr07-wrong-key-0000001",
        Some(CANARY_SECRET),
    );
    let r3 = post(
        addr,
        "/v1/cards/card-gdpr07-02/approve",
        &[("Content-Type", "application/json")],
        &wrong,
    );
    expect_status(&r3, 409, "拒否（snapshot 不一致）");

    // 4. 拒否（未知フィールド client_msg_id canary）→ 400。
    let extra = format!(
        "{{\"card_id\": \"card-gdpr07-02\", \"body\": \"本文\", \"client_msg_id\": \"{CANARY_MSG_ID}\"}}"
    );
    let r4 = post(
        addr,
        "/v1/cards/card-gdpr07-02/comments",
        &[
            ("Content-Type", "application/json"),
            ("x-pelway-member-id", "m-human-gdpr07"),
            ("x-pelway-member-kind", "human"),
        ],
        &extra,
    );
    expect_status(&r4, 400, "拒否（未知フィールド）");

    // 5. 内部エラー（ロックを poison させてからコメント）→ 500。
    //    panic の文言に canary は含めない（標準エラーに既定のフックが出す）。
    poison(&ledger);
    let body = "{\"card_id\": \"card-gdpr07-02\", \"body\": \"内部エラー後のコメント\"}".to_owned();
    let r5 = post(
        addr,
        "/v1/cards/card-gdpr07-02/comments",
        &[
            ("Content-Type", "application/json"),
            ("x-pelway-member-id", "m-human-gdpr07"),
            ("x-pelway-member-kind", "human"),
        ],
        &body,
    );
    expect_status(&r5, 500, "内部エラー");

    println!(
        "demo-log-emit: statuses = [{}, {}, {}, {}, {}]",
        r1.0, r2.0, r3.0, r4.0, r5.0
    );
    println!("demo-log-emit: log file = {}", sink.path().display());
}

/// 生の HTTP/1.1 POST を 1 回送り、(状態コード, 本文) を返す。接続先はループバックだけ。
fn post(addr: SocketAddr, path: &str, headers: &[(&str, &str)], body: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(addr).expect("connect 127.0.0.1");
    let mut req = format!("POST {path} HTTP/1.1\r\nHost: demo.local\r\nConnection: close\r\n");
    for (k, v) in headers {
        req.push_str(&format!("{k}: {v}\r\n"));
    }
    req.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
    stream.write_all(req.as_bytes()).expect("write head");
    stream.write_all(body.as_bytes()).expect("write body");
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

fn expect_status(resp: &(u16, String), want: u16, label: &str) {
    assert_eq!(
        resp.0, want,
        "{label}: 予期した状態コードでない: {}",
        resp.1
    );
}

/// カード 2 枚の合成台帳（カード作成相当。本文に canary を含む）。
fn synthetic_ledger() -> SharedLedger {
    let mut ledger = ApprovalLedger::new();
    for id in ["card-gdpr07-01", "card-gdpr07-02"] {
        ledger
            .create_card(NewCard {
                workspace_id: "ws-synthetic-gdpr07".into(),
                channel_id: "ch-synthetic-gdpr07".into(),
                card_id: id.into(),
                payload_hash: "sha256:synthetic-payload-gdpr07".to_owned(),
                action_kind: ActionKind::CustomerSend,
                recipient: "customer@example.invalid".to_owned(),
                body: format!("合成データの本文（{id}）{CANARY_BODY}"),
            })
            .expect("create synthetic card");
    }
    Arc::new(Mutex::new(ledger))
}

/// 台帳の現在の (card_version, snapshot_hash) で承認 JSON を作る。
/// `hash_override` は拒否の再現用（canary 秘密を snapshot_hash に入れる）。
fn approve_json(
    ledger: &SharedLedger,
    card: &str,
    key: &str,
    hash_override: Option<&str>,
) -> String {
    let g = ledger.lock().expect("ledger lock");
    let c = g.card(&card.into()).expect("synthetic card");
    format!(
        "{{\"card_id\": \"{card}\", \"card_version\": {}, \"snapshot_hash\": \"{}\", \"idempotency_key\": \"{key}\"}}",
        c.card_version,
        hash_override.unwrap_or(&c.snapshot_hash),
    )
}

/// 別スレッドでロックを保持したまま panic させ、台帳を poison させる。
/// panic の文言は canary を含まない固定語句だけにする。
fn poison(ledger: &SharedLedger) {
    let target = Arc::clone(ledger);
    let _ = thread::spawn(move || {
        let _guard = target.lock().expect("lock before poison");
        panic!("poison the ledger mutex for the demo");
    })
    .join();
    assert!(ledger.is_poisoned());
}
