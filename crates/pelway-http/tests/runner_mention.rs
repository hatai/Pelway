//! runner-lite（#329）の HTTP 経路の結合テスト。`POST /v1/channels/{id}/messages`
//! でハル宛てのメンションを投稿すると runner がタスクを起こし、bot の投稿と
//! 判断待ちまで進むことを確かめる。LLM はフェイク（実提供元には接続しない）。

use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use pelway_contract::decision::{decide_path, decisions_path};
use pelway_contract::ids::{ChannelId, MemberId, WorkspaceId};
use pelway_contract::ledger::ApprovalLedger;
use pelway_contract::member::MemberKind;
use pelway_contract::message::channel_messages_path;
use pelway_contract::store::{
    DEMO_CHANNEL_ID, DEMO_HAL_MEMBER_ID, DEMO_WORKSPACE_ID, DataStore, PurposeId,
};
use pelway_contract::task::TaskState;
use pelway_http::{SharedDataStore, SharedLedger, serve_with_store_and_runner};
use pelway_llm::{ChatReply, ChatRequest, EmbedReply, LlmError, LlmGateway, Usage};
use pelway_runner::{Runner, RunnerConfig, RunnerTaskId};
use serde_json::json;

fn now() -> u64 {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_millis(),
    )
    .unwrap_or(u64::MAX)
}

/// 要求スキーマに合う固定の応答を返すフェイクの gateway（下書き要求には
/// 下書き JSON、それ以外は A/B JSON）。
struct FakeGateway;

/// 下書きの応答スキーマか（required に `subject` を含むか）で応答を切り分ける。
fn is_draft_schema(req: &ChatRequest) -> bool {
    req.schema
        .as_ref()
        .and_then(|s| s.get("required"))
        .and_then(|r| r.as_array())
        .is_some_and(|a| a.iter().any(|v| v == "subject"))
}

impl LlmGateway for FakeGateway {
    async fn chat(&self, req: ChatRequest) -> Result<ChatReply, LlmError> {
        let text = if is_draft_schema(&req) {
            r#"{"to": "B社 ご担当者", "subject": "案件B 納期のご相談", "body": "お世話になっております。納期を2週間延ばす案で調整させていただけますでしょうか。"}"#
        } else {
            r#"{"option_a": "納期を2週間延ばす案", "option_b": "範囲を絞って納期を守る案"}"#
        };
        Ok(ChatReply {
            text: text.to_owned(),
            usage: Usage::default(),
        })
    }

    async fn embed(&self, _texts: Vec<String>) -> Result<EmbedReply, LlmError> {
        Err(LlmError::NotSent("embed は使わない".to_owned()))
    }
}

fn http(addr: SocketAddr, method: &str, path: &str, headers: &[(&str, &str)], body: &[u8]) -> u16 {
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
    buf.split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("status line")
}

fn wait_until(mut f: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if f() {
            return true;
        }
        if Instant::now() > deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn posted_mention_to_bot_runs_task_to_pending_decision() {
    let store: SharedDataStore = Arc::new(Mutex::new(DataStore::new_demo_seeded(now())));
    let ledger: SharedLedger = Arc::new(Mutex::new(ApprovalLedger::new()));
    let runner = Arc::new(Runner::new(
        RunnerConfig {
            bot_member_id: MemberId::from(DEMO_HAL_MEMBER_ID),
            decider: MemberId::from("m-yamamoto"),
            llm_basis: PurposeId::P06,
            llm_timeout: Duration::from_secs(30),
        },
        Arc::clone(&store),
        Arc::clone(&ledger),
        Arc::new(FakeGateway),
    ));
    let server = serve_with_store_and_runner(
        "127.0.0.1:0".parse().unwrap(),
        Arc::clone(&ledger),
        Arc::clone(&store),
        runner.clone(),
    )
    .expect("serve");
    let addr = server.local_addr();

    // 田中がハル宛てのメンションを投稿する。
    let status = http(
        addr,
        "POST",
        &channel_messages_path(&ChannelId::from(DEMO_CHANNEL_ID)),
        &[
            ("x-pelway-member-id", "m-tanaka"),
            ("x-pelway-member-kind", "human"),
        ],
        json!({"client_msg_id": "cm-mention-http-1", "body": "@ハル 納期延長の案を2つ作って", "thread_root": null})
            .to_string()
            .as_bytes(),
    );
    assert_eq!(status, 200);

    // メンションからタスクが作られ、bot の進捗・案の投稿、判断待ちまで進む
    // （最初のタスクの採番は task-00000001）。
    let task_id = RunnerTaskId("task-00000001".to_owned());
    assert!(wait_until(|| {
        runner
            .task(&task_id)
            .is_some_and(|t| t.state == TaskState::AwaitingDecision)
    }));
    let bot_posts: Vec<String> = store
        .lock()
        .expect("store")
        .channel_messages(&ChannelId::from(DEMO_CHANNEL_ID))
        .into_iter()
        .filter(|m| m.member_kind == MemberKind::Bot)
        .map(|m| m.body.clone())
        .collect();
    assert!(
        bot_posts
            .iter()
            .any(|b| b == "依頼を受け付けました。A/B 案を作成しています。"),
        "進捗の1行が出る: {bot_posts:?}"
    );
    assert!(
        bot_posts
            .iter()
            .any(|b| b.contains("案A: 納期を2週間延ばす案")
                && b.contains("案B: 範囲を絞って納期を守る案")),
        "A/B 案が投稿される: {bot_posts:?}"
    );
    let pending = ledger.lock().expect("ledger").pending_decisions(
        &WorkspaceId::from(DEMO_WORKSPACE_ID),
        &MemberId::from("m-yamamoto"),
        now(),
    );
    assert_eq!(pending.len(), 1, "判断待ちが T3a の仕組みで作られる");

    // 山本が案Aを決定すると（#330）、runner がメール下書きを生成して
    // AwaitingSendApproval まで進む。
    let item = &pending[0];
    let status = http(
        addr,
        "POST",
        &decide_path(&item.decision_id),
        &[
            ("x-pelway-member-id", "m-yamamoto"),
            ("x-pelway-member-kind", "human"),
        ],
        json!({
            "decision_id": item.decision_id.as_str(),
            "version": item.version,
            "option_id": "a"
        })
        .to_string()
        .as_bytes(),
    );
    assert_eq!(status, 200, "決定の確定");
    assert!(wait_until(|| {
        runner
            .task(&task_id)
            .is_some_and(|t| t.state == TaskState::AwaitingSendApproval)
    }));
    let task = runner.task(&task_id).expect("task");
    let draft = task.draft.as_ref().expect("下書きのデータ");
    assert_eq!(draft.to, "B社 ご担当者");
    assert!(draft.body.contains("納期を2週間延ばす案"));
    let bot_posts: Vec<String> = store
        .lock()
        .expect("store")
        .channel_messages(&ChannelId::from(DEMO_CHANNEL_ID))
        .into_iter()
        .filter(|m| m.member_kind == MemberKind::Bot)
        .map(|m| m.body.clone())
        .collect();
    assert!(
        bot_posts
            .iter()
            .any(|b| b.contains("メール下書き") && b.contains(&draft.subject)),
        "下書きが投稿される: {bot_posts:?}"
    );

    // 決定の一覧経路も残っていること（経路の退行を見る）。
    let status = http(
        addr,
        "GET",
        &format!(
            "{}?actor=m-yamamoto",
            decisions_path(&WorkspaceId::from(DEMO_WORKSPACE_ID))
        ),
        &[],
        &[],
    );
    assert_eq!(status, 200);
}
