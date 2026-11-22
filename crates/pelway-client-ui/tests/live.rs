//! 通信基盤（Issue 333・PR-1）のワイヤ契約の結合テスト。`pelway-http` の
//! サーバーを `127.0.0.1:0` で実際に立て、`LiveSession` が GET（クエリ `actor`）
//! と POST（合成ヘッダー `x-pelway-member-id`・`x-pelway-member-kind`）で
//! サーバーと会話することを確かめる。
//! 表示用の行モデル（送信者名や `HH:MM` の組み立て）は PR-2 で足すため、ここでは
//! ワイヤ上の型（`Channel`・`StoredMessage`・`MemberId`）で検査する。

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::{Arc, Mutex};

use pelway_client_ui::live::{self, LiveError, LiveSession};
use pelway_client_ui::net::NetError;
use pelway_contract::ids::{ChannelId, MemberId, MessageId, WorkspaceId};
use pelway_contract::ledger::ApprovalLedger;
use pelway_contract::member::{ChannelMember, MemberKind};
use pelway_contract::message::Channel;
use pelway_contract::store::{DataStore, NewMessage, PurposeId, StoredMessage};
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
    let posted = session.post("CSV出力の件、案をお願いします").expect("post");
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

    // client_msg_id は投稿ごとに違う値（冪等の送信 ID。同じ値なら 409 を起こさず
    // 同じメッセージに収束するが、毎回同じでは再送以外も重複扱いになる）。
    let second = session.post("続けて二つ目").expect("second post");
    assert_ne!(posted.client_msg_id, second.client_msg_id);

    // ポーリングで他ユーザーの分（自分の投稿より前の seq）も取りこぼさず拾い、
    // seq 昇順のまま一覧になる。
    session.poll().expect("poll succeeds");
    let messages = session.messages();
    assert_eq!(messages.len(), 6, "seed 3 + 他ユーザー 1 + 自分 2");
    assert!(
        messages.windows(2).all(|w| w[0].seq < w[1].seq),
        "seq 昇順を保つ"
    );
    let other = &messages[3];
    assert_eq!(other.body, "案Aと案Bを用意しました");
    assert_eq!(other.author_id.as_str(), "m-hal");
    assert_eq!(other.member_kind, MemberKind::Bot);
    assert_eq!(messages[4].author_id.as_str(), "m-tanaka");
    assert_eq!(messages[5].author_id.as_str(), "m-tanaka");
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

// ページング（バイト境界）: 件数上限では応答サイズを抑えられないため、
// 1 ページが応答上限（1 MiB）を超える履歴では、同じカーソルから件数を
// 半減して再試行し、最後まで全件取得できる。起動時の `bootstrap` が縮退の
// 経路を通っても `connected` が立ったまま戻ることも確かめる（縮退の中で
// 立ち下がった `connected` が復帰しないと「つながっていません」から戻れない）。
#[test]
fn bootstrap_shrinks_pages_and_stays_connected() {
    let (addr, store) = spawn_demo_server();
    // 1 件あたり約 60 KiB の本文を持つ履歴（ASCII 60,000 バイト＝投稿の
    // 要求本文上限 64 KiB に収まる投稿）。MESSAGE_PAGE_LIMIT=200 の 1 ページ
    // 要求では応答が 1 MiB を優に超える（一覧の制限は件数だけ）。
    // limit を半減させる再試行が必要な配置。
    let extra = 20usize;
    {
        let mut store = store.lock().expect("store lock");
        for i in 0..extra {
            store
                .post_message(
                    NewMessage {
                        workspace_id: "ws-demo-01".into(),
                        channel_id: "ch-demo-anken-b".into(),
                        author_id: "m-hal".into(),
                        member_kind: MemberKind::Bot,
                        purpose_id: PurposeId::P01,
                        body: "a".repeat(60_000) + &i.to_string(),
                        thread_root: None,
                        client_msg_id: None,
                    },
                    3_000_000 + i as u64,
                )
                .expect("seed large message");
        }
    }

    let mut session = LiveSession::new(addr, "ws-demo-01".into());
    session
        .bootstrap()
        .expect("bootstrap succeeds by shrinking pages under the byte limit");
    assert!(
        session.connected(),
        "縮退を通った bootstrap でも接続状態に戻る"
    );
    let messages = session.messages();
    assert_eq!(
        messages.len(),
        extra + 3,
        "応答上限を超える履歴も全件取れる"
    );
    assert!(
        messages.windows(2).all(|w| w[0].seq < w[1].seq),
        "seq 昇順を保つ"
    );
}

/// パスごとに決まった応答を返す最小スタブ（取得途中の失敗を再現するため）。
/// `respond` は要求ターゲットを受け `(status, body)` を返す。
fn spawn_stub_server(respond: impl Fn(&str) -> (u16, String) + Send + 'static) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind stub");
    let addr = listener.local_addr().expect("stub addr");
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buf = vec![0u8; 16 * 1024];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]);
            let target = request.split_whitespace().nth(1).unwrap_or("").to_owned();
            let (status, body) = respond(&target);
            let head = format!(
                "HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(body.as_bytes());
        }
    });
    addr
}

/// クロージャが返すバイト列をそのまま応答として書き出すスタブ
/// （HTTP として壊れた応答を作るため）。
fn spawn_raw_stub_server(respond: impl Fn(&str) -> Vec<u8> + Send + 'static) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind stub");
    let addr = listener.local_addr().expect("stub addr");
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buf = vec![0u8; 16 * 1024];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]);
            let target = request.split_whitespace().nth(1).unwrap_or("").to_owned();
            let _ = stream.write_all(&respond(&target));
        }
    });
    addr
}

// bootstrap の原子性: 再取得の途中（メッセージ取得）で失敗しても、
// 前回のチャンネル・メッセージ・表示ユーザーの組み合わせは壊れない
// （名義だけ先に書き換わり、別チャンネルへ別人名義で投稿する退化を防ぐ）。
#[test]
fn bootstrap_failure_mid_refetch_keeps_previous_channel_and_actor() {
    let (addr, _store) = spawn_demo_server();
    let mut session = LiveSession::new(addr, "ws-demo-01".into());
    session.bootstrap().expect("bootstrap succeeds");
    let channel_id = session.channel().unwrap().channel_id.as_str().to_owned();
    let actor = session.actor().as_str().to_owned();

    // 再接続先: チャンネル一覧と詳細は返すが、メッセージ取得だけ 500 にする。
    // 詳細のメンバーは現在の表示ユーザーとは別の人だけ（名義の切替を誘発する）。
    let other = Channel {
        workspace_id: WorkspaceId::from("ws-demo-01"),
        channel_id: ChannelId::from("ch-other"),
        name: "別チャンネル".to_owned(),
        members: vec![ChannelMember {
            member_id: MemberId::from("m-sato"),
            display_name: "佐藤".to_owned(),
            member_kind: MemberKind::Human,
        }],
    };
    let channels_json = serde_json::to_string(&vec![other.clone()]).unwrap();
    let channel_json = serde_json::to_string(&other).unwrap();
    let stub = spawn_stub_server(move |target| {
        if target.contains("/messages") {
            (500, "unavailable".to_owned())
        } else if target.contains("/channels/ch-") {
            (200, channel_json.clone())
        } else {
            (200, channels_json.clone())
        }
    });
    session.set_addr(stub);

    assert!(
        session.bootstrap().is_err(),
        "メッセージ取得が失敗するので bootstrap 自体は失敗"
    );
    assert_eq!(
        session.channel().unwrap().channel_id.as_str(),
        channel_id,
        "失敗しても前回のチャンネルを保持"
    );
    assert_eq!(
        session.actor().as_str(),
        actor,
        "失敗しても前回の表示ユーザーを保持"
    );
}

// 保持上限（スライディングウィンドウ）: 本文が空でも ID などの可変長
// フィールドが大きいメッセージを詰めた満杯のページを連続して返す接続先でも、
// 保持する履歴を最新側の予算内に収めながら末尾まで取り切れる
// （上限到達をもって失敗にすると、予算を超える履歴のあるチャンネルが
// 復帰できなくなるため）。本文だけ数えるとこの経路が上限をすり抜けるため、
// 可変長フィールドを含めて見積もる。
#[test]
fn poll_keeps_the_newest_messages_within_the_fetch_budget() {
    let (addr, _store) = spawn_demo_server();
    let mut session = LiveSession::new(addr, "ws-demo-01".into());
    session.bootstrap().expect("bootstrap succeeds");

    // 各ページは要求どおりちょうど MESSAGE_PAGE_LIMIT 件・1 MiB 未満で、
    // seq を進めながら 45 ページ返し、その後は空ページで取得を終わらせる。
    // 本文は空で、ID だけを大きくする（ページあたり約 860 KiB、合計で
    // 保持予算 32 MiB を超える配置）。
    let total: u64 = 45 * 200;
    let pad = "a".repeat(4_000);
    let next_seq = Arc::new(std::sync::atomic::AtomicU64::new(1));
    let stub = spawn_stub_server(move |_target| {
        let base = next_seq.fetch_add(200, std::sync::atomic::Ordering::SeqCst);
        if base > total {
            return (200, "[]".to_owned());
        }
        let msgs: Vec<StoredMessage> = (0..200_u64)
            .map(|i| StoredMessage {
                message_id: MessageId::from(format!("m-{}", base + i).as_str()),
                workspace_id: WorkspaceId::from("ws-demo-01"),
                channel_id: ChannelId::from("ch-demo-anken-b"),
                seq: base + i,
                author_id: MemberId::from(pad.as_str()),
                member_kind: MemberKind::Bot,
                purpose_id: PurposeId::P01,
                body: String::new(),
                thread_root: None,
                client_msg_id: None,
                created_at: 0,
            })
            .collect();
        (
            200,
            serde_json::to_string(&msgs).expect("serialize messages"),
        )
    });
    session.set_addr(stub);

    session
        .poll()
        .expect("poll succeeds by keeping only the newest within budget");
    assert!(session.connected(), "取得に成功したら接続状態");
    let messages = session.messages();
    assert!(
        messages.len() < total as usize,
        "予算を超える分の古い側は保持しない（{} 件中 {} 件）",
        total,
        messages.len()
    );
    assert_eq!(
        messages.last().map(|m| m.seq),
        Some(total),
        "最新のメッセージは残る"
    );
    assert!(
        messages.first().map(|m| m.seq).unwrap_or_default() > 1,
        "古い側は予算の分だけ捨てられる"
    );
    assert!(
        messages.windows(2).all(|w| w[0].seq < w[1].seq),
        "seq 昇順を保つ"
    );
}

// 名義の切替と GET の actor: `bootstrap` がチャンネルのメンバーから
// 表示ユーザーを選び直したとき、履歴取得の GET はその新しい名義を送る
// （サーバーは GET の actor を記録するため、表示と取得で名義を食い違わせない）。
#[test]
fn bootstrap_fetches_history_with_the_switched_actor() {
    // 既定の表示ユーザー（m-tanaka）が居ないチャンネルを返すスタブ。
    let other = Channel {
        workspace_id: WorkspaceId::from("ws-demo-01"),
        channel_id: ChannelId::from("ch-other"),
        name: "別チャンネル".to_owned(),
        members: vec![ChannelMember {
            member_id: MemberId::from("m-sato"),
            display_name: "佐藤".to_owned(),
            member_kind: MemberKind::Human,
        }],
    };
    let channels_json = serde_json::to_string(&vec![other.clone()]).unwrap();
    let channel_json = serde_json::to_string(&other).unwrap();
    let seen = Arc::new(Mutex::new(Vec::<String>::new()));
    let seen_in_stub = seen.clone();
    let stub = spawn_stub_server(move |target| {
        seen_in_stub.lock().unwrap().push(target.to_owned());
        if target.contains("/messages") {
            (200, "[]".to_owned())
        } else if target.contains("/channels/ch-") {
            (200, channel_json.clone())
        } else {
            (200, channels_json.clone())
        }
    });

    let mut session = LiveSession::new(stub, "ws-demo-01".into());
    session.bootstrap().expect("bootstrap succeeds");
    assert_eq!(session.actor().as_str(), "m-sato", "先頭の人に切り替わる");
    let seen = seen.lock().unwrap();
    assert!(
        seen.iter()
            .any(|t| t.contains("/messages") && t.contains("actor=m-sato")),
        "履歴取得の GET は選び直した名義を送る: {seen:?}"
    );
}

// 冪等キー: 同じ論理投稿の再送は同じ `client_msg_id` で送り、
// サーバーは二重に記録しない。新しい投稿は別の ID で送る。
#[test]
fn repost_with_same_client_msg_id_is_idempotent() {
    let (addr, _store) = spawn_demo_server();
    let mut session = LiveSession::new(addr, "ws-demo-01".into());
    session.bootstrap().expect("bootstrap succeeds");
    let base = session.messages().len();

    let id = LiveSession::new_client_msg_id();
    let first = session
        .post_with_id("再送テスト", id.clone())
        .expect("first post");
    let second = session
        .post_with_id("再送テスト", id)
        .expect("repost with same id");
    assert_eq!(
        first.message_id, second.message_id,
        "同じ client_msg_id の再送は保存済みを返す"
    );
    assert_eq!(session.messages().len(), base + 1, "二重には記録されない");

    let third = session.post("別の投稿").expect("new post gets new id");
    assert_ne!(
        third.message_id, first.message_id,
        "新規の投稿は別の ID（別の保存済みメッセージ）"
    );
}

// 宛先の確定: チャンネルを選んでいない（`bootstrap` 未成功）セッションで
// `post` すると、通信を始める前に `NoChannel` を返す。`bootstrap` が
// 選び直したチャンネルへ、利用者が見ていない宛先・名義で届くことを防ぐ。
#[test]
fn post_without_selected_channel_returns_no_channel() {
    let (addr, _store) = spawn_demo_server();
    let mut session = LiveSession::new(addr, "ws-demo-01".into());
    assert!(
        matches!(session.post("hello"), Err(LiveError::NoChannel)),
        "宛先が未選択なら投稿せず NoChannel"
    );
}

// ページング: 1 ページの上限（MESSAGE_PAGE_LIMIT）を超える履歴でも、
// after_seq で続きを取って全件取得できる（応答上限超過での恒久的な
// 取得失敗を防ぐ回帰テスト）。
#[test]
fn poll_pages_through_history_over_the_limit() {
    let (addr, store) = spawn_demo_server();
    let mut session = LiveSession::new(addr, "ws-demo-01".into());
    session.bootstrap().expect("bootstrap succeeds");
    let base = session.messages().len();

    // 1 ページ分を超える履歴を保存部に足す（サーバーは limit 件までを返す
    // ため、クライアントが after_seq で続きを取らないと全件に届かない）。
    let extra = live::MESSAGE_PAGE_LIMIT as usize + 5;
    {
        let mut store = store.lock().expect("store lock");
        for i in 0..extra {
            store
                .post_message(
                    NewMessage {
                        workspace_id: "ws-demo-01".into(),
                        channel_id: "ch-demo-anken-b".into(),
                        author_id: "m-hal".into(),
                        member_kind: MemberKind::Bot,
                        purpose_id: PurposeId::P01,
                        body: format!("補足 {i}"),
                        thread_root: None,
                        client_msg_id: None,
                    },
                    3_000_000 + i as u64,
                )
                .expect("seed history message");
        }
    }

    session.poll().expect("poll succeeds over the limit");
    let messages = session.messages();
    assert_eq!(messages.len(), base + extra, "上限を超える履歴も全件取れる");
    assert!(
        messages.windows(2).all(|w| w[0].seq < w[1].seq),
        "seq 昇順を保つ"
    );
}

// サーバー URL は環境変数から読む値（Issue 390 の固定ポートと同じ値を渡す前提）。
// パースは IPv4 の 127.0.0.1 だけを受け付ける（サーバーの serve_inner と同じ
// 判定。`::1` や `127.0.0.2` も拒否）。
#[test]
fn parse_server_addr_accepts_ipv4_loopback_only() {
    let port_8787: SocketAddr = "127.0.0.1:8787".parse().unwrap();
    assert_eq!(
        live::parse_server_addr("http://127.0.0.1:8787").unwrap(),
        port_8787
    );
    assert_eq!(
        live::parse_server_addr("127.0.0.1:8787").unwrap(),
        port_8787
    );

    // 127.0.0.1 以外（127/8 の他・IPv6 を含む）・不正な形は拒否
    // （localhost の名前解決もしない）。
    for bad in [
        "http://127.0.0.2:8787",
        "http://[::1]:8787",
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
    // 既定値はデモ用の暫定値（Issue 390 の固定ポートが確定したら揃える）。
    assert!(live::DEFAULT_SERVER_URL.starts_with("http://127.0.0.1:"));
    assert!(live::parse_server_addr(live::DEFAULT_SERVER_URL).is_ok());
}

// 受信側の検査（net.rs）: エラーの種類ごとに呼び出し側の動作が変わるため、
// それぞれを直接確かめる（`TooLarge` は縮退の再試行、`Malformed`/`Io`/
// `NonLoopback` は接続失敗として扱う）。
#[test]
fn net_rejects_non_loopback_target() {
    // parse_server_addr を通らない経路（アドレスの直接指定）でも拒否する。
    let mut session = LiveSession::new("192.0.2.1:80".parse().unwrap(), "ws".into());
    assert!(matches!(
        session.bootstrap(),
        Err(LiveError::Net(NetError::NonLoopback))
    ));
}

#[test]
fn net_rejects_malformed_http_response() {
    let stub = spawn_raw_stub_server(|_| b"this is not an http response".to_vec());
    let mut session = LiveSession::new(stub, "ws".into());
    assert!(matches!(
        session.bootstrap(),
        Err(LiveError::Net(NetError::Malformed))
    ));
}

#[test]
fn net_rejects_content_length_mismatch() {
    let stub = spawn_raw_stub_server(|_| {
        b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\n[]".to_vec()
    });
    let mut session = LiveSession::new(stub, "ws".into());
    assert!(
        matches!(
            session.bootstrap(),
            Err(LiveError::Net(NetError::Malformed))
        ),
        "宣言と本文の長さが違う応答は受け付けない"
    );
}

#[test]
fn net_rejects_oversized_response() {
    // 1 MiB を超える応答は読み取りの途中で打ち切る。
    let stub = spawn_stub_server(|_| (200, "x".repeat(2 * 1024 * 1024)));
    let mut session = LiveSession::new(stub, "ws".into());
    assert!(matches!(
        session.bootstrap(),
        Err(LiveError::Net(NetError::TooLarge))
    ));
}

#[test]
fn net_times_out_on_a_silent_server() {
    // 接続を受けても応答を返さない接続先は、読み取り期限で失敗する。
    let stub = spawn_raw_stub_server(|_| {
        std::thread::sleep(std::time::Duration::from_secs(10));
        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n[]".to_vec()
    });
    let mut session = LiveSession::new(stub, "ws".into());
    assert!(matches!(
        session.bootstrap(),
        Err(LiveError::Net(NetError::Io(_)))
    ));
}
