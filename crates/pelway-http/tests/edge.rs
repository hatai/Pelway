//! HTTP 層の未カバー経路の補強（#159）。公開入口・Display・プロトコルエラー・
//! ログ sink の失敗経路を、実ソケットと実ファイルで確かめる。
//!
//! 接続先はループバック（127.0.0.1）だけ。外部への接続は無い。

use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use pelway_contract::approval::{Decider, approve_path, comments_path};
use pelway_contract::handoff::handoffs_path;
use pelway_contract::ids::CardId;
use pelway_contract::ledger::{ApprovalLedger, NewCard};
use pelway_contract::pseudonym::pseudonymize;
use pelway_contract::snapshot::ActionPayload;
use pelway_contract::store::DataStore;
use pelway_contract::task::ActionKind;
use pelway_http::log::{FileSink, LogSink, Logger, MemorySink};
use pelway_http::{
    HttpError, MAX_HEADER_LINES, ServeError, SharedDataStore, SharedLedger, serve,
    serve_with_connection_deadline, serve_with_logger_urandom, serve_with_store,
    serve_with_store_and_logger_urandom,
};
use serde_json::json;

fn ledger() -> SharedLedger {
    Arc::new(Mutex::new(ApprovalLedger::new()))
}

fn store() -> SharedDataStore {
    Arc::new(Mutex::new(DataStore::new()))
}

fn new_card(id: &str) -> NewCard {
    NewCard {
        workspace_id: "ws-edge".into(),
        channel_id: "ch-edge".into(),
        card_id: id.into(),
        payload: ActionPayload {
            action_kind: ActionKind::CustomerSend,
            recipient: "customer@example.invalid".to_owned(),
            body: format!("合成データの本文 {id}"),
            attachments: Vec::new(),
            account: "acct-e-1".to_owned(),
            scope: Vec::new(),
        },
        requester: "m-edge-requester".into(),
        decider: Decider::Member("m-edge-decider".into()),
        deadline: None,
    }
}

/// 生の要求バイト列を送り、(状態コード, 応答全体) を返す。書き込み後は送信側を閉じる。
fn raw(addr: SocketAddr, request: &[u8]) -> (u16, String) {
    let mut stream = TcpStream::connect(addr).expect("connect");
    stream.write_all(request).expect("write");
    let _ = stream.shutdown(Shutdown::Write);
    let mut buf = String::new();
    stream.read_to_string(&mut buf).expect("read response");
    let status = buf
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("status line");
    (status, buf)
}

fn addr_of(server: &pelway_http::Server) -> SocketAddr {
    server.local_addr()
}

// ServeError の Display は両バリアントを文字列化できる。
#[test]
fn serve_error_display_formats_both_variants() {
    let bad: SocketAddr = "0.0.0.0:0".parse().unwrap();
    let err = serve(bad, ledger()).err().expect("非ループバックは Err");
    let text = err.to_string();
    assert!(text.contains("must be exactly 127.0.0.1"), "{text}");

    let io = ServeError::Io(std::io::Error::other("io-wired"));
    assert!(io.to_string().contains("io-wired"));
}

// HttpError の全バリアントが対応する状態コードを返す。
#[test]
fn http_status_maps_every_variant() {
    let cases = [
        (HttpError::BadRequest, 400),
        (HttpError::CardIdMismatch, 422),
        (HttpError::NotFound, 404),
        (HttpError::MethodNotAllowed, 405),
        (HttpError::PayloadTooLarge, 413),
        (HttpError::RequestTimeout, 408),
        (HttpError::HeadersTooLarge, 431),
        (HttpError::Internal, 500),
    ];
    for (e, want) in cases {
        assert_eq!(e.http_status(), want, "{e:?}");
    }
}

// serve_with_store: ループバックでは待ち受け、それ以外は断る。
#[test]
fn serve_with_store_binds_loopback_and_refuses_others() {
    let server = serve_with_store("127.0.0.1:0".parse().unwrap(), ledger(), store())
        .expect("serve on loopback");
    assert_eq!(addr_of(&server).ip().to_string(), "127.0.0.1");

    assert!(matches!(
        serve_with_store("0.0.0.0:0".parse().unwrap(), ledger(), store()),
        Err(ServeError::NotLoopback(_))
    ));
}

// serve_with_logger_urandom: salt を /dev/urandom から読み、ループバックで待ち受ける。
#[test]
fn serve_with_logger_urandom_binds_loopback() {
    let sink = Arc::new(MemorySink::new());
    let server = serve_with_logger_urandom(
        "127.0.0.1:0".parse().unwrap(),
        ledger(),
        sink as Arc<dyn LogSink>,
    )
    .expect("serve with urandom salt");
    assert_eq!(addr_of(&server).ip().to_string(), "127.0.0.1");
}

// serve_with_store_and_logger_urandom: 保存部つきでも同じく待ち受ける。
#[test]
fn serve_with_store_and_logger_urandom_binds_loopback() {
    let sink = Arc::new(MemorySink::new());
    let server = serve_with_store_and_logger_urandom(
        "127.0.0.1:0".parse().unwrap(),
        ledger(),
        store(),
        sink as Arc<dyn LogSink>,
    )
    .expect("serve with store and urandom salt");
    assert_eq!(addr_of(&server).ip().to_string(), "127.0.0.1");

    assert!(matches!(
        serve_with_store_and_logger_urandom(
            "0.0.0.0:0".parse().unwrap(),
            ledger(),
            store(),
            Arc::new(MemorySink::new()),
        ),
        Err(ServeError::NotLoopback(_))
    ));
}

// ヘッダー終端の前に接続が切れたら 400。
#[test]
fn eof_before_head_complete_is_400() {
    let server = serve("127.0.0.1:0".parse().unwrap(), ledger()).expect("serve");
    let (status, _) = raw(addr_of(&server), b"POST /v1/approvals/x/comments");
    assert_eq!(status, 400);
}

// ヘッダー行数が上限を超えたら 431。
#[test]
fn more_than_max_header_lines_is_431() {
    let server = serve("127.0.0.1:0".parse().unwrap(), ledger()).expect("serve");
    let mut req = String::from("POST /v1/approvals/x/comments HTTP/1.1\r\n");
    for i in 0..=MAX_HEADER_LINES {
        req.push_str(&format!("X-H{i}: v\r\n"));
    }
    req.push_str("Content-Length: 0\r\n\r\n");
    let (status, _) = raw(addr_of(&server), req.as_bytes());
    assert_eq!(status, 431);
}

// コロンを含まないヘッダー行は 400。
#[test]
fn header_line_without_colon_is_400() {
    let server = serve("127.0.0.1:0".parse().unwrap(), ledger()).expect("serve");
    let (status, _) = raw(
        addr_of(&server),
        b"POST /v1/approvals/x/comments HTTP/1.1\r\nNoColonHere\r\nContent-Length: 0\r\n\r\n",
    );
    assert_eq!(status, 400);
}

// Content-Length が無い要求は 400（ヘッダーのみで閉じる）。
#[test]
fn missing_content_length_is_400() {
    let server = serve("127.0.0.1:0".parse().unwrap(), ledger()).expect("serve");
    let (status, _) = raw(
        addr_of(&server),
        b"GET /v1/approvals/x/comments HTTP/1.1\r\nHost: demo.local\r\n\r\n",
    );
    assert_eq!(status, 400);
}

// 本文の途中で接続が切れたら 400。
#[test]
fn eof_during_body_is_400() {
    let server = serve("127.0.0.1:0".parse().unwrap(), ledger()).expect("serve");
    let (status, _) = raw(
        addr_of(&server),
        b"POST /v1/approvals/x/comments HTTP/1.1\r\nContent-Length: 10\r\n\r\n",
    );
    assert_eq!(status, 400);
}

// 知らない経路は 404、POST 以外のメソッドは 405。
#[test]
fn unknown_route_is_404_and_wrong_method_is_405() {
    let server = serve("127.0.0.1:0".parse().unwrap(), ledger()).expect("serve");
    let addr = addr_of(&server);

    let (status, _) = raw(
        addr,
        b"POST /v1/no-such-route HTTP/1.1\r\nContent-Length: 0\r\n\r\n",
    );
    assert_eq!(status, 404);

    let comments = comments_path(&CardId::from("card-edge"));
    let req = format!("GET {comments} HTTP/1.1\r\nContent-Length: 0\r\n\r\n");
    let (status, _) = raw(addr, req.as_bytes());
    assert_eq!(status, 405);

    // workspaces 配下でも経路が確定しなければ 404。
    let (status, _) = raw(
        addr,
        b"GET /v1/workspaces/ws-edge/bogus-segment HTTP/1.1\r\nContent-Length: 0\r\n\r\n",
    );
    assert_eq!(status, 404);

    // 経路の {card_id} が空なら照合しない（404）。
    let (status, _) = raw(
        addr,
        b"POST /v1/approvals//comments HTTP/1.1\r\nContent-Length: 0\r\n\r\n",
    );
    assert_eq!(status, 404);
}

// 接続が読み取り途中で閉じてもサーバーは他の接続を続けてさばける。
#[test]
fn dropped_connection_does_not_stop_server() {
    let server = serve("127.0.0.1:0".parse().unwrap(), ledger()).expect("serve");
    let mut s = TcpStream::connect(addr_of(&server)).expect("connect");
    s.write_all(b"POST /v1").expect("partial write");
    drop(s);
    let (status, _) = raw(
        addr_of(&server),
        b"POST /nope HTTP/1.1\r\nContent-Length: 0\r\n\r\n",
    );
    assert_eq!(status, 404);
}

// 1 接続の全体期限を極端に短くすると、最初の読み取りの前に期限切れで 408。
#[test]
fn elapsed_connection_deadline_is_408() {
    let server =
        serve_with_connection_deadline("127.0.0.1:0".parse().unwrap(), ledger(), Duration::ZERO)
            .expect("serve with short deadline");
    let stream = TcpStream::connect(addr_of(&server)).expect("connect");
    let mut buf = String::new();
    let mut s = stream;
    s.read_to_string(&mut buf).expect("read response");
    let status = buf
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok());
    assert_eq!(status, Some(408), "{buf:?}");
}

// Content-Length が本文上限を超えたら 413 を返し、読み捨てる残りを drain する。
#[test]
fn oversized_body_is_413_and_drained() {
    let server = serve("127.0.0.1:0".parse().unwrap(), ledger()).expect("serve");
    let mut req =
        String::from("POST /v1/approvals/x/comments HTTP/1.1\r\nContent-Length: 70000\r\n\r\n");
    req.push_str(&"x".repeat(500));
    let (status, _) = raw(addr_of(&server), req.as_bytes());
    assert_eq!(status, 413);
}

// comments: 要求者ヘッダー・種別ヘッダーが無いと 400。
#[test]
fn comments_without_member_headers_is_400() {
    let server = serve("127.0.0.1:0".parse().unwrap(), ledger()).expect("serve");
    let path = comments_path(&CardId::from("card-edge"));
    let body = json!({"card_id": "card-edge", "body": "hi"}).to_string();

    // どちらも無い。
    let req = format!(
        "POST {path} HTTP/1.1\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let (status, _) = raw(addr_of(&server), req.as_bytes());
    assert_eq!(status, 400);

    // member-id だけでは足りない（kind も必須）。
    let req = format!(
        "POST {path} HTTP/1.1\r\nx-pelway-member-id: m-edge\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let (status, _) = raw(addr_of(&server), req.as_bytes());
    assert_eq!(status, 400);

    // kind が仕様外でも 400。
    let req = format!(
        "POST {path} HTTP/1.1\r\nx-pelway-member-id: m-edge\r\nx-pelway-member-kind: bogus\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let (status, _) = raw(addr_of(&server), req.as_bytes());
    assert_eq!(status, 400);
}

// comments: 存在しないカードへのコメントは card_not_found（拒否ログの結果コード経路）。
#[test]
fn comment_on_missing_card_is_404() {
    let server = serve("127.0.0.1:0".parse().unwrap(), ledger()).expect("serve");
    let path = comments_path(&CardId::from("card-absent"));
    let body = json!({"card_id": "card-absent", "body": "hi"}).to_string();
    let req = format!(
        "POST {path} HTTP/1.1\r\nx-pelway-member-id: m-edge\r\nx-pelway-member-kind: human\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let (status, _) = raw(addr_of(&server), req.as_bytes());
    assert_eq!(status, 404);
}

// approve: 要求者ヘッダーが無いと 400、JSON が壊れていても 400。
#[test]
fn approve_missing_headers_or_bad_json_is_400() {
    let server = serve("127.0.0.1:0".parse().unwrap(), ledger()).expect("serve");
    let path = approve_path(&CardId::from("card-edge"));
    let body = json!({
        "card_id": "card-edge",
        "card_version": 1,
        "snapshot_hash": "h",
        "idempotency_key": "k-0000000000000001",
    })
    .to_string();

    // ヘッダー無し。
    let req = format!(
        "POST {path} HTTP/1.1\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let (status, _) = raw(addr_of(&server), req.as_bytes());
    assert_eq!(status, 400);

    // member-id だけでは足りない（kind も必須）。
    let req = format!(
        "POST {path} HTTP/1.1\r\nx-pelway-member-id: m-edge-decider\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let (status, _) = raw(addr_of(&server), req.as_bytes());
    assert_eq!(status, 400);

    // ヘッダーあり・JSON 壊れ。
    let req = format!(
        "POST {path} HTTP/1.1\r\nx-pelway-member-id: m-edge-decider\r\nx-pelway-member-kind: human\r\nContent-Length: 7\r\n\r\nnotjson"
    );
    let (status, _) = raw(addr_of(&server), req.as_bytes());
    assert_eq!(status, 400);
}

// approve: 存在しないカードは card_not_found、承認待ちでないカードはその旨のエラー。
#[test]
fn approve_error_codes_for_missing_and_not_awaiting_cards() {
    let mut ledger = ApprovalLedger::new();
    ledger.create_card(new_card("card-idle")).expect("card");
    let server =
        serve("127.0.0.1:0".parse().unwrap(), Arc::new(Mutex::new(ledger))).expect("serve");
    let addr = addr_of(&server);

    let approve = |card: &str| {
        let path = approve_path(&CardId::from(card));
        let body = json!({
            "card_id": card,
            "card_version": 1,
            "snapshot_hash": "h",
            "idempotency_key": "k-0000000000000001",
        })
        .to_string();
        let req = format!(
            "POST {path} HTTP/1.1\r\nx-pelway-member-id: m-edge-decider\r\nx-pelway-member-kind: human\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        raw(addr, req.as_bytes()).0
    };

    assert_eq!(approve("card-absent"), 404);
    // 承認待ちでないカードの approve は 404 以外の 4xx（契約エラーの結果コード経路）。
    let status = approve("card-idle");
    assert!((400..500).contains(&status), "{status}");
}

// approve: 完了済みのカードへの再 approve は not_awaiting_approval の結果コード経路。
#[test]
fn approve_finished_card_is_rejected_with_code() {
    let shared = ledger();
    {
        let mut g = shared.lock().expect("ledger lock");
        g.create_card(new_card("card-done")).expect("card");
        let (version, hash) = {
            let c = g.card(&"card-done".into()).expect("card");
            (c.card_version, c.snapshot_hash.clone())
        };
        let req = pelway_contract::approval::ApproveRequest {
            card_id: "card-done".into(),
            card_version: version,
            snapshot_hash: hash,
            idempotency_key: pelway_contract::ids::IdempotencyKey::new("k-0000000000000001")
                .expect("valid key"),
        };
        let _ = g.begin_approve(&req, &"m-edge-decider".into());
        let _ = g.approve(req, &"m-edge-decider".into());
    }
    let server = serve("127.0.0.1:0".parse().unwrap(), shared.clone()).expect("serve");

    let (version, hash) = {
        let g = shared.lock().expect("ledger lock");
        let c = g.card(&"card-done".into()).expect("card");
        (c.card_version, c.snapshot_hash.clone())
    };
    let path = approve_path(&CardId::from("card-done"));
    let body = json!({
        "card_id": "card-done",
        "card_version": version,
        "snapshot_hash": hash,
        "idempotency_key": "k-0000000000000002",
    })
    .to_string();
    let req = format!(
        "POST {path} HTTP/1.1\r\nx-pelway-member-id: m-edge-decider\r\nx-pelway-member-kind: human\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let (status, _) = raw(addr_of(&server), req.as_bytes());
    assert!((400..500).contains(&status), "{status}");
}

// handoffs: JSON が壊れている・欄が足りない要求は 400。
#[test]
fn handoffs_bad_body_is_400() {
    let server = serve("127.0.0.1:0".parse().unwrap(), ledger()).expect("serve");
    let path = handoffs_path(&CardId::from("card-edge"));
    let req = format!("POST {path} HTTP/1.1\r\nContent-Length: 2\r\n\r\n{{}}");
    let (status, _) = raw(addr_of(&server), req.as_bytes());
    assert_eq!(status, 400);
}

// 根拠記録の一覧: actor クエリが無いと 400。
#[test]
fn list_basis_records_without_actor_is_400() {
    let server = serve("127.0.0.1:0".parse().unwrap(), ledger()).expect("serve");
    let req = "GET /v1/workspaces/ws-edge/communication-basis-records HTTP/1.1\r\nContent-Length: 0\r\n\r\n";
    let (status, _) = raw(addr_of(&server), req.as_bytes());
    assert_eq!(status, 400);
}

// 根拠記録の撤回: JSON が壊れていると 400。
#[test]
fn withdraw_basis_record_bad_json_is_400() {
    let server = serve("127.0.0.1:0".parse().unwrap(), ledger()).expect("serve");
    let req = "POST /v1/workspaces/ws-edge/communication-basis-records/b-1/withdraw HTTP/1.1\r\nContent-Length: 7\r\n\r\nnotjson";
    let (status, _) = raw(addr_of(&server), req.as_bytes());
    assert_eq!(status, 400);
}

// FileSink の Debug は経路を出しつつ詳細を省略する。
#[test]
fn file_sink_debug_is_non_exhaustive() {
    let sink = FileSink::create("edge-debug.jsonl").expect("create");
    let text = format!("{sink:?}");
    assert!(text.contains("FileSink"), "{text}");
}

// 出力ファイルを開けないとき append は黙って諦める（接続処理を止めない）。
#[test]
fn file_sink_append_is_noop_when_open_fails() {
    let sink = FileSink::create("edge-blocked").expect("create");
    let dir = sink.path();
    std::fs::create_dir_all(&dir).expect("block path with a directory");
    sink.append("should be dropped");
    assert!(dir.is_dir());
    std::fs::remove_dir(&dir).expect("cleanup");
}

// Pseudonym の Display は中身の仮名文字列をそのまま書く。
#[test]
fn pseudonym_display_writes_inner() {
    let logger = Logger::disabled();
    // disabled の salt は空。仮名は pseudonymize の結果と一致する。
    let id = logger.id("raw-edge");
    assert_eq!(id.to_string(), pseudonymize(&[], "raw-edge"));
}

// Logger の Debug は salt を出さない非網羅表示。
#[test]
fn logger_debug_is_non_exhaustive() {
    let text = format!("{:?}", Logger::disabled());
    assert!(text.contains("Logger"), "{text}");
}
