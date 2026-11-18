//! 通信基盤（Issue 333・PR-1）のワイヤ契約の結合テスト。`pelway-http` の
//! サーバーを `127.0.0.1:0` で実際に立て、`LiveSession` が GET（クエリ `actor`）
//! と POST（合成ヘッダー `x-pelway-member-id`・`x-pelway-member-kind`）で
//! サーバーと会話することを確かめる。
//! 表示用の行モデル（送信者名や `HH:MM` の組み立て）は PR-2 で足すため、ここでは
//! ワイヤ上の型（`Channel`・`StoredMessage`・`MemberId`）で検査する。

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::{Arc, Mutex};

use pelway_client_core::approval::{Clock, KeySource};
use pelway_client_ui::live::{self, LiveSession};
use pelway_client_ui::realtime::{SystemClock, UuidKeySource};
use pelway_contract::ledger::ApprovalLedger;
use pelway_contract::member::MemberKind;
use pelway_contract::store::{DataStore, NewMessage, PurposeId};
use pelway_http::{SharedDataStore, SharedLedger, serve_with_store};

/// デモ seed（`DataStore::new_demo_seeded`）を持つサーバーと保存部の共有ハンドル。
fn spawn_demo_server() -> (SocketAddr, SharedDataStore) {
    let store: SharedDataStore = Arc::new(Mutex::new(DataStore::new_demo_seeded(1_000_000)));
    let ledger: SharedLedger = Arc::new(Mutex::new(ApprovalLedger::new()));
    let addr = serve_with_store("127.0.0.1:0".parse().unwrap(), ledger, Arc::clone(&store))
        .expect("serve on 127.0.0.1:0")
        .local_addr();
    (addr, store)
}

/// 使われていないポートを得る（bind してすぐ閉じる。接続は refused になる）。
fn dead_addr() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
    let addr = listener.local_addr().expect("local addr");
    drop(listener);
    addr
}

/// 1 接続だけ受けて、届いた要求（ヘッダー区切りまで）を記録してから決まった
/// 応答を返す最小スタブ。実サーバーでは見えない「ワイヤに何を書いたか」
/// （GET の `actor` クエリ等）と、HTTP の失敗系（5xx・壊れた JSON）を検査する。
fn stub_once(status: &str, body: &str) -> (SocketAddr, Arc<Mutex<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind stub");
    let addr = listener.local_addr().expect("stub addr");
    let captured = Arc::new(Mutex::new(String::new()));
    let slot = Arc::clone(&captured);
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut got = Vec::new();
        let mut buf = [0u8; 4096];
        while !got.windows(4).any(|w| w == b"\r\n\r\n") {
            let n = stream.read(&mut buf).expect("read request");
            if n == 0 {
                break;
            }
            got.extend_from_slice(&buf[..n]);
        }
        *slot.lock().unwrap() = String::from_utf8_lossy(&got).into_owned();
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });
    (addr, captured)
}

// 受け入れ条件1: 起動時にサーバーからチャンネルとメッセージが取れる（モックではない）。
#[test]
fn bootstrap_fetches_channels_and_messages() {
    let (addr, _store) = spawn_demo_server();
    let mut session = LiveSession::new(addr, "ws-demo-01".into());

    session.bootstrap().expect("bootstrap succeeds");

    assert!(session.connected(), "取得に成功したら接続中");
    let channel = session.channel().expect("channel selected");
    assert_eq!(channel.name, "案件B");
    // チャンネルのメンバー表がワイヤの型で取れている（人 3・bot 1 のデモ seed）。
    let humans: Vec<&str> = channel
        .members
        .iter()
        .filter(|m| m.member_kind == MemberKind::Human)
        .map(|m| m.member_id.as_str())
        .collect();
    assert_eq!(humans, ["m-tanaka", "m-yamamoto", "m-sato"]);
    assert!(
        channel
            .members
            .iter()
            .any(|m| m.member_kind == MemberKind::Bot),
        "AI メンバー（ハル）も取得される"
    );

    // seed の過去会話 3 件が取れている（実サーバー由来＝モックではない）。
    let messages = session.messages();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0].author_id.as_str(), "m-tanaka");
    assert_eq!(messages[2].author_id.as_str(), "m-yamamoto");
    assert!(messages.iter().all(|m| m.created_at > 0));
}

// 受け入れ条件2: 投稿がサーバーへ届き、ポーリングでほかのユーザーの変更が反映される。
#[test]
fn post_reaches_server_and_poll_picks_up_others_messages() {
    let (addr, store) = spawn_demo_server();
    let mut session = LiveSession::new(addr, "ws-demo-01".into());
    session.bootstrap().expect("bootstrap succeeds");

    // ほかのユーザーの変更（別クライアントの投稿相当）を、自分の投稿の直前に
    // 保存部へ足す。投稿応答の seq が他ユーザーの分を追い越す配置で、
    // 「投稿でポーリング位置が進み割り込み分を取りこぼす」退化を塞ぐ。
    {
        let mut store = store.lock().expect("store lock");
        store
            .post_message(
                NewMessage {
                    workspace_id: "ws-demo-01".into(),
                    channel_id: "ch-demo-anken-b".into(),
                    author_id: "m-hal".into(),
                    member_kind: MemberKind::Bot,
                    purpose_id: PurposeId::P01,
                    body: "案Aと案Bを用意しました".to_owned(),
                    thread_root: None,
                    client_msg_id: None,
                },
                2_000_000,
            )
            .expect("other user's message");
    }

    // 投稿: 表示ユーザー（既定は先頭の人＝田中）として送る。
    session.post("CSV出力の件、案をお願いします").expect("post");
    {
        let store = store.lock().expect("store lock");
        let messages = store.channel_messages(&"ch-demo-anken-b".into());
        let last = messages.last().expect("posted message stored");
        assert_eq!(last.author_id.as_str(), "m-tanaka");
        assert_eq!(last.member_kind, MemberKind::Human);
        assert_eq!(last.body, "CSV出力の件、案をお願いします");
        assert!(last.client_msg_id.is_some(), "冪等キー付きで保存");
    }
    // 自分の投稿はセッション側にも反映される。
    assert_eq!(
        session.messages().last().map(|m| m.body.as_str()),
        Some("CSV出力の件、案をお願いします")
    );

    // ポーリングで他ユーザーの分（自分の投稿より前の seq）も取りこぼさず拾い、
    // seq 昇順のまま一覧になる。
    session.poll().expect("poll succeeds");
    let messages = session.messages();
    assert_eq!(messages.len(), 5, "seed 3 + 他ユーザー 1 + 自分 1");
    assert!(
        messages.windows(2).all(|w| w[0].seq < w[1].seq),
        "seq 昇順を保つ"
    );
    let other = &messages[3];
    assert_eq!(other.body, "案Aと案Bを用意しました");
    assert_eq!(other.author_id.as_str(), "m-hal");
    assert_eq!(other.member_kind, MemberKind::Bot);
    assert_eq!(messages[4].author_id.as_str(), "m-tanaka");
}

// 受け入れ条件3: 表示ユーザーを田中／山本／佐藤で切り替えられる。
// 切替後の変更系要求は、そのユーザーの合成ヘッダーでサーバーに届く（Issue 327 の契約）。
#[test]
fn switching_actor_changes_who_the_server_accepts() {
    let (addr, store) = spawn_demo_server();
    let mut session = LiveSession::new(addr, "ws-demo-01".into());
    session.bootstrap().expect("bootstrap succeeds");

    assert_eq!(session.actor().as_str(), "m-tanaka");

    // 山本に切り替えて投稿すると、サーバーは山本として受け付ける。
    assert!(session.switch_actor("m-yamamoto"), "人のメンバーなら切替可");
    assert_eq!(session.actor().as_str(), "m-yamamoto");
    session.post("案Aで進めましょう").expect("post as yamamoto");
    {
        let store = store.lock().expect("store lock");
        let last = store
            .channel_messages(&"ch-demo-anken-b".into())
            .pop()
            .expect("posted");
        assert_eq!(last.author_id.as_str(), "m-yamamoto");
    }

    // 佐藤にも切り替えられる。bot・未知のメンバーには切り替えない。
    assert!(session.switch_actor("m-sato"));
    assert!(!session.switch_actor("m-hal"), "bot は切替対象にしない");
    assert!(
        !session.switch_actor("m-unknown"),
        "未知のメンバーは切替えない"
    );
    assert_eq!(session.actor().as_str(), "m-sato");
}

// 受け入れ条件3 のワイヤ側: 切替後の GET は新しい actor のクエリで出る（Issue 327）。
#[test]
fn get_after_switch_sends_new_actor_query() {
    let (addr, _store) = spawn_demo_server();
    let mut session = LiveSession::new(addr, "ws-demo-01".into());
    session.bootstrap().expect("bootstrap succeeds");
    session.switch_actor("m-yamamoto");

    let (stub, captured) = stub_once("200 OK", "[]");
    session.set_addr(stub);
    session.poll().expect("poll against stub");

    let request = captured.lock().unwrap().clone();
    assert!(
        request.starts_with("GET ") && request.contains("actor=m-yamamoto"),
        "切替後の GET は新しい actor のクエリ: {request}"
    );
}

// HTTP の失敗系（5xx・壊れた JSON）でも「つながっていません」になり、
// 復帰（= 断線からのフル再取得）で通常の状態に戻る。
#[test]
fn status_and_json_errors_mark_offline_and_recover() {
    let (addr, store) = spawn_demo_server();
    let mut session = LiveSession::new(addr, "ws-demo-01".into());
    session.bootstrap().expect("bootstrap succeeds");

    // 5xx は Status の失敗として非接続になる。
    let (stub, _captured) = stub_once("500 Internal Server Error", "{}");
    session.set_addr(stub);
    let err = session.poll().expect_err("5xx は失敗する");
    assert!(matches!(err, live::LiveError::Status(500)), "{err:?}");
    assert!(!session.connected(), "失敗したら非接続状態へ");

    // 落ちている間に他ユーザーの分が積まれた想定。復帰のフル再取得で拾う。
    {
        let mut store = store.lock().expect("store lock");
        store
            .post_message(
                NewMessage {
                    workspace_id: "ws-demo-01".into(),
                    channel_id: "ch-demo-anken-b".into(),
                    author_id: "m-sato".into(),
                    member_kind: MemberKind::Human,
                    purpose_id: PurposeId::P01,
                    body: "停電中に進めておきました".to_owned(),
                    thread_root: None,
                    client_msg_id: None,
                },
                3_000_000,
            )
            .expect("queued during outage");
    }
    session.set_addr(addr);
    session.poll().expect("poll after recovery");
    assert!(session.connected(), "復帰したら接続状態へ");
    assert_eq!(
        session.messages().last().map(|m| m.body.as_str()),
        Some("停電中に進めておきました"),
        "復帰の再取得で履歴ごと新しい分を拾う"
    );

    // 200 でも JSON が読めなければ Json の失敗として非接続になる。
    let (stub, _captured) = stub_once("200 OK", "not json");
    session.set_addr(stub);
    let err = session.poll().expect_err("壊れた JSON は失敗する");
    assert!(matches!(err, live::LiveError::Json(_)), "{err:?}");
    assert!(!session.connected());
    session.set_addr(addr);
    session.poll().expect("poll after recovery");
    assert!(session.connected());
    assert_eq!(session.messages().len(), 4, "seed 3 + 停電中の 1 件");
}

// チーム点検の追加条件: サーバーが落ちた・ポーリングが失敗したとき
// 「つながっていません」の状態になり、復帰したら通常表示に戻る。
#[test]
fn connection_failure_marks_offline_and_recovery_restores() {
    let (addr, _store) = spawn_demo_server();
    let mut session = LiveSession::new(addr, "ws-demo-01".into());
    session.bootstrap().expect("bootstrap succeeds");
    assert!(session.connected());

    // 接続先が死ぬと poll は失敗し、セッションは「つながっていない」状態になる。
    session.set_addr(dead_addr());
    assert!(session.poll().is_err());
    assert!(!session.connected(), "失敗したら非接続状態へ");

    // 復帰（接続先が戻る）したら通常の状態に戻る。
    session.set_addr(addr);
    session.poll().expect("poll after recovery");
    assert!(session.connected(), "復帰したら接続状態へ");
}

// サーバー URL は環境変数から読む値（Issue 390 の固定ポートと同じ値を渡す前提）。
// パースはループバックだけを受け付ける（デモの接続先は 127.0.0.1）。
#[test]
fn parse_server_addr_accepts_loopback_only() {
    let port_8787: SocketAddr = "127.0.0.1:8787".parse().unwrap();
    assert_eq!(
        live::parse_server_addr("http://127.0.0.1:8787").unwrap(),
        port_8787
    );
    assert_eq!(
        live::parse_server_addr("127.0.0.1:8787").unwrap(),
        port_8787
    );
    assert_eq!(
        live::parse_server_addr("http://[::1]:8787").unwrap(),
        "[::1]:8787".parse().unwrap()
    );

    // ループバック以外・不正な形は拒否（localhost の名前解決もしない）。
    for bad in [
        "http://192.168.0.10:8787",
        "http://localhost:8787",
        "http://example.com",
        "not-an-addr",
        "",
    ] {
        assert!(
            live::parse_server_addr(bad).is_err(),
            "{bad} は拒否すること"
        );
    }
    // 既定値はデモ準備キットの固定ポート（Issue 390）と同じ形のループバック。
    assert!(live::DEFAULT_SERVER_URL.starts_with("http://127.0.0.1:"));
    assert!(live::parse_server_addr(live::DEFAULT_SERVER_URL).is_ok());
}

// 受け入れ条件4: 時刻とキーが実物（FixedClock / FixedKeySource の置き換え）。
#[test]
fn real_clock_and_real_keys() {
    // 実時計は UNIX エポックからのミリ秒で、呼ぶたびに進む。
    let clock = SystemClock;
    let t1 = clock.now_ms();
    let t2 = clock.now_ms();
    assert!(t1 > 0 && t2 >= t1, "実時刻のミリ秒: {t1} -> {t2}");

    // キーは冪等キーの書式（16〜128 文字の [A-Za-z0-9_-]）で、毎回違う値。
    let mut keys = UuidKeySource::new();
    let a = keys.fresh_key();
    let b = keys.fresh_key();
    assert_ne!(a, b, "毎回違うキー");
    assert_eq!(a.as_str().len(), 36, "UUID 形式（36 文字）");

    // client_msg_id も毎回違う（冪等の送信 ID。同じ値なら 409 を起こさず同じ
    // メッセージに収束するが、毎回同じでは再送以外も重複扱いになる）。
    let (addr, _store) = spawn_demo_server();
    let mut session = LiveSession::new(addr, "ws-demo-01".into());
    session.bootstrap().unwrap();
    let first = session.post("一つ目").unwrap();
    let second = session.post("二つ目").unwrap();
    assert_ne!(
        first.client_msg_id, second.client_msg_id,
        "投稿ごとに違う client_msg_id"
    );
}
