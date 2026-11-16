//! チャンネルとメッセージの HTTP 経路（#327）の結合テスト。`127.0.0.1:0` で
//! 起動したサーバーに実際に HTTP/1.1 要求を送り、状態コードと本文 JSON を
//! 確かめる（接続先はループバックだけ）。経路は契約のテンプレート定数から
//! 作る（書き写さない。Issue #91）。

use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};

use pelway_contract::ids::{ChannelId, MemberId, WorkspaceId};
use pelway_contract::ledger::ApprovalLedger;
use pelway_contract::member::{ChannelMember, MemberKind};
use pelway_contract::message::{Channel, channel_messages_path, channel_path, channels_path};
use pelway_contract::store::DataStore;
use pelway_http::{SharedDataStore, SharedLedger, serve_with_store};
use serde_json::{Value, json};

const WS: &str = "ws-synthetic-channels";
const CH: &str = "ch-synthetic-talk";
const ME: &str = "m-human-poster";
const BOT: &str = "m-bot-runner";

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

/// 合成チャンネル 1 つ・過去メッセージ無しの保存部を持つサーバーを立てる。
fn spawn_server() -> (SocketAddr, SharedDataStore) {
    let mut s = DataStore::new();
    let ws = WorkspaceId::from(WS);
    s.ensure_workspace(&ws, 1_000);
    s.add_channel(Channel {
        workspace_id: ws,
        channel_id: ChannelId::from(CH),
        name: "合成チャンネル".to_owned(),
        members: vec![
            ChannelMember {
                member_id: MemberId::from(ME),
                display_name: "合成 一郎".to_owned(),
                member_kind: MemberKind::Human,
            },
            ChannelMember {
                member_id: MemberId::from(BOT),
                display_name: "合成ボット".to_owned(),
                member_kind: MemberKind::Bot,
            },
        ],
    });
    let store: SharedDataStore = Arc::new(Mutex::new(s));
    let ledger: SharedLedger = Arc::new(Mutex::new(ApprovalLedger::new()));
    let addr = serve_with_store("127.0.0.1:0".parse().unwrap(), ledger, Arc::clone(&store))
        .expect("serve on 127.0.0.1:0")
        .local_addr();
    (addr, store)
}

/// GET して (状態コード, JSON) を返す。path にクエリが既にあれば `&` で繋ぐ。
fn get(addr: SocketAddr, path: &str, actor: &str) -> (u16, Value) {
    let sep = if path.contains('?') { "&" } else { "?" };
    let (status, body) = http(addr, "GET", &format!("{path}{sep}actor={actor}"), &[], b"");
    (status, serde_json::from_str(&body).unwrap_or(Value::Null))
}

fn get_channels(addr: SocketAddr, workspace: &str, actor: &str) -> (u16, Value) {
    get(addr, &channels_path(&WorkspaceId::from(workspace)), actor)
}

fn get_channel(addr: SocketAddr, channel: &str, actor: &str) -> (u16, Value) {
    get(addr, &channel_path(&ChannelId::from(channel)), actor)
}

fn post_message(
    addr: SocketAddr,
    channel: &str,
    member: &str,
    kind: &str,
    body: &str,
) -> (u16, String) {
    http(
        addr,
        "POST",
        &channel_messages_path(&ChannelId::from(channel)),
        &[
            ("x-pelway-member-id", member),
            ("x-pelway-member-kind", kind),
        ],
        body.as_bytes(),
    )
}

fn message_body(client_msg_id: &str, body: &str) -> String {
    json!({"client_msg_id": client_msg_id, "body": body}).to_string()
}

fn get_messages(addr: SocketAddr, channel: &str, query: &str) -> (u16, Value) {
    let mut path = channel_messages_path(&ChannelId::from(channel));
    if !query.is_empty() {
        path = format!("{path}?{query}");
    }
    get(addr, &path, ME)
}

fn seqs(json: &Value) -> Vec<u64> {
    json.as_array()
        .expect("messages is an array")
        .iter()
        .map(|m| m["seq"].as_u64().expect("seq is a number"))
        .collect()
}

// 受け入れ条件1: チャンネルの一覧と情報が取得できる。
#[test]
fn channel_list_and_info_are_served() {
    let (addr, _store) = spawn_server();

    // 一覧: workspace のチャンネルが 1 件。
    let (status, list) = get_channels(addr, WS, ME);
    assert_eq!(status, 200, "{list}");
    let channels = list.as_array().expect("channels is an array");
    assert_eq!(channels.len(), 1, "{list}");
    assert_eq!(channels[0]["channel_id"], CH);
    assert_eq!(channels[0]["name"], "合成チャンネル");
    assert_eq!(channels[0]["workspace_id"], WS);

    // 情報: メンバーと種別まで取れる。
    let (status, info) = get_channel(addr, CH, ME);
    assert_eq!(status, 200, "{info}");
    assert_eq!(info["channel_id"], CH);
    assert_eq!(info["name"], "合成チャンネル");
    let members = info["members"].as_array().expect("members is an array");
    assert_eq!(members.len(), 2, "{info}");
    assert_eq!(members[0]["member_id"], ME);
    assert_eq!(members[0]["member_kind"], "human");
    assert_eq!(members[1]["member_kind"], "bot");

    // 未知のチャンネルは 404、未知の workspace の一覧は空。
    let (status, _) = get_channel(addr, "ch-absent", ME);
    assert_eq!(status, 404);
    let (status, list) = get_channels(addr, "ws-absent", ME);
    assert_eq!(status, 200, "{list}");
    assert_eq!(list.as_array().expect("array").len(), 0, "{list}");
}

// 受け入れ条件2: メッセージを投稿すると seq 付きで保存され、取得に現れる。
#[test]
fn posted_message_is_stored_with_seq_and_listed() {
    let (addr, _store) = spawn_server();

    let (status, resp) = post_message(
        addr,
        CH,
        ME,
        "human",
        &message_body("cm-post-1", "依頼: 納期延長の返信案を作って"),
    );
    assert_eq!(status, 200, "{resp}");
    let posted: Value = serde_json::from_str(&resp).expect("posted json");
    assert_eq!(posted["seq"], 1);
    assert_eq!(posted["channel_id"], CH);
    assert_eq!(posted["workspace_id"], WS);
    assert_eq!(posted["author_id"], ME);
    assert_eq!(posted["member_kind"], "human");
    assert_eq!(posted["body"], "依頼: 納期延長の返信案を作って");
    assert_eq!(posted["client_msg_id"], "cm-post-1");
    assert!(
        posted["message_id"]
            .as_str()
            .expect("id")
            .starts_with("msg-")
    );

    // 2 件目は runner（bot）から。seq はチャンネル内で進む。
    let (status, resp) = post_message(
        addr,
        CH,
        BOT,
        "bot",
        &message_body("cm-post-2", "案Aと案Bを用意しました"),
    );
    assert_eq!(status, 200, "{resp}");
    let posted: Value = serde_json::from_str(&resp).expect("posted json");
    assert_eq!(posted["seq"], 2);
    assert_eq!(posted["member_kind"], "bot");

    // 取得に 2 件が seq 順で現れる。
    let (status, list) = get_messages(addr, CH, "");
    assert_eq!(status, 200, "{list}");
    assert_eq!(seqs(&list), [1, 2], "{list}");
    assert_eq!(list[0]["body"], "依頼: 納期延長の返信案を作って");
    assert_eq!(list[1]["author_id"], BOT);
}

// 受け入れ条件3: after_seq で差分だけが取得できる（ポーリング用）。
#[test]
fn after_seq_returns_only_newer_messages() {
    let (addr, _store) = spawn_server();
    for i in 1..=3 {
        let (status, resp) = post_message(
            addr,
            CH,
            ME,
            "human",
            &message_body(&format!("cm-diff-{i}"), &format!("発言 {i}")),
        );
        assert_eq!(status, 200, "{resp}");
    }

    // 全部 → 差分 → 最後尾。
    let (status, all) = get_messages(addr, CH, "");
    assert_eq!(status, 200);
    assert_eq!(seqs(&all), [1, 2, 3]);

    let (status, diff) = get_messages(addr, CH, "after_seq=1");
    assert_eq!(status, 200, "{diff}");
    assert_eq!(seqs(&diff), [2, 3], "{diff}");

    let (status, tail) = get_messages(addr, CH, "after_seq=3");
    assert_eq!(status, 200, "{tail}");
    assert_eq!(seqs(&tail), Vec::<u64>::new(), "{tail}");
}

// 投稿は冪等: 同じ client_msg_id・同じ内容の再送は同じメッセージで
// 二重に記録されない。内容が違う再投稿は 409（04 §2）。
#[test]
fn repost_is_idempotent_and_conflicting_repost_is_409() {
    let (addr, _store) = spawn_server();
    let body = message_body("cm-idem-1", "冪等の合成データ");
    let (status, first) = post_message(addr, CH, ME, "human", &body);
    assert_eq!(status, 200, "{first}");
    let (status, second) = post_message(addr, CH, ME, "human", &body);
    assert_eq!(status, 200, "{second}");
    let a: Value = serde_json::from_str(&first).expect("first json");
    let b: Value = serde_json::from_str(&second).expect("second json");
    assert_eq!(
        a["message_id"], b["message_id"],
        "再送は同じメッセージを返す"
    );

    // 同じ client_msg_id で内容が違う再投稿は 409。
    let (status, resp) = post_message(
        addr,
        CH,
        ME,
        "human",
        &message_body("cm-idem-1", "内容が違う再投稿"),
    );
    assert_eq!(status, 409, "{resp}");

    // 記録は 1 件のまま。
    let (status, list) = get_messages(addr, CH, "");
    assert_eq!(status, 200);
    assert_eq!(seqs(&list), [1], "{list}");
}

// 経路・入力の検査: 未知チャンネルへの投稿は 404、GET 系は actor が必須、
// 数値でない after_seq は 400、許さないメソッドは 405。
#[test]
fn route_and_input_validation() {
    let (addr, _store) = spawn_server();

    // 未知チャンネルへの投稿は 404 で保存しない。
    let (status, _) = post_message(
        addr,
        "ch-absent",
        ME,
        "human",
        &message_body("cm-x-1", "宛先のない投稿"),
    );
    assert_eq!(status, 404);
    // 未知チャンネルの取得も 404。
    let (status, _) = get_messages(addr, "ch-absent", "");
    assert_eq!(status, 404);

    // 投稿者ヘッダーが欠けた投稿は 400。
    let (status, _) = http(
        addr,
        "POST",
        &channel_messages_path(&ChannelId::from(CH)),
        &[],
        message_body("cm-x-2", "ヘッダー無し").as_bytes(),
    );
    assert_eq!(status, 400);
    let (status, _) = post_message(
        addr,
        CH,
        ME,
        "ai", // 旧・デモの値は受理しない
        &message_body("cm-x-3", "kind が不正"),
    );
    assert_eq!(status, 400);
    // client_msg_id の無い本文は 400（契約では必須）。
    let (status, _) = post_message(
        addr,
        CH,
        ME,
        "human",
        &json!({"body": "id 無し"}).to_string(),
    );
    assert_eq!(status, 400);

    // GET 系は actor が必須（空も 400）。
    for path in [
        channels_path(&WorkspaceId::from(WS)),
        channel_messages_path(&ChannelId::from(CH)),
    ] {
        let (status, _) = http(addr, "GET", &path, &[], b"");
        assert_eq!(status, 400, "{path}");
    }

    // after_seq が数値でなければ 400。
    let (status, _) = get_messages(addr, CH, "after_seq=abc");
    assert_eq!(status, 400);

    // メソッドの検査: 一覧への POST・情報への POST・チャンネル経路への PUT は 405。
    for (method, path) in [
        ("POST", channels_path(&WorkspaceId::from(WS))),
        ("POST", channel_path(&ChannelId::from(CH))),
        ("PUT", channel_messages_path(&ChannelId::from(CH))),
    ] {
        let (status, body) = http(addr, method, &path, &[], b"{}");
        assert_eq!(status, 405, "{method} {path}");
        assert!(
            body.contains("method_not_allowed"),
            "{method} {path}: {body}"
        );
    }
}
