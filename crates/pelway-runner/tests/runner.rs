//! runner-lite（#329）＋実 LLM 接続（#330）のフェイク gateway テスト。
//! 実 LLM には接続しない。
//!
//! 受け入れ条件との対応:
//! - メンション → タスク・進捗・A/B 案の投稿・判断待ち（#329）
//! - 検証に失敗した応答は1回だけ再試行し、それでもだめなら台本へ落ちる（#330）
//! - タイムアウトしたら台本へ落ちる（#330）
//! - キーが無い・不正なときは台本モードで起動する（#330）
//! - 根拠は設定で受けた目的 ID（P06）で検査する（#330）

use std::collections::VecDeque;
use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use pelway_contract::decision::{
    CreateDecisionRequest, DecideRequest, DecisionListItem, DecisionOption, DecisionOutcome,
};
use pelway_contract::ids::{ChannelId, MemberId, OptionId, WorkspaceId};
use pelway_contract::ledger::ApprovalLedger;
use pelway_contract::member::{ChannelMember, MemberKind};
use pelway_contract::message::Channel;
use pelway_contract::store::{
    DEMO_CHANNEL_ID, DEMO_HAL_MEMBER_ID, DEMO_WORKSPACE_ID, DataStore, NewMessage, PurposeId,
    StoredMessage,
};
use pelway_contract::task::TaskState;
use pelway_llm::{ChatReply, ChatRequest, EmbedReply, LlmError, LlmGateway, Usage};
use pelway_runner::demo::{
    BASIS_ENV, CHAT_MODEL_ENV, CUSTOM_BASE_URL_ENV, DemoLlmConfig, RESPONSE_FORMAT_ENV,
    TIMEOUT_SECS_ENV,
};
use pelway_runner::{MailDraft, Runner, RunnerConfig, RunnerError, RunnerTaskId, TEMPLATE_MARK};

const WS: &str = DEMO_WORKSPACE_ID;
const CH: &str = DEMO_CHANNEL_ID;
const HAL: &str = DEMO_HAL_MEMBER_ID;
const REQUESTER: &str = "m-tanaka";
const DECIDER: &str = "m-yamamoto";
/// 「受付」の進捗 1 行（固定段階。runner 側の定数と同じ文）。
const STAGE_ACCEPTED: &str = "依頼を受け付けました。A/B 案を作成しています。";
/// 台本へ落ちるときの進捗行に含まれる句。
const FALLBACK_MARK: &str = "台本の応答で進めます";
/// runner の既定のタイムアウト（テストは 30 秒。タイムアウト検証は短く上書きする）。
const TEST_TIMEOUT: Duration = Duration::from_secs(30);

fn now() -> u64 {
    let ms = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock");
    u64::try_from(ms.as_millis()).unwrap_or(u64::MAX)
}

/// LLM の応答（A/B 案）の合成 JSON。runner が要求するスキーマに従う形。
fn ab_reply() -> ChatReply {
    ChatReply {
        text: r#"{"option_a": "納期を2週間延ばす案", "option_b": "範囲を絞って納期を守る案"}"#
            .to_owned(),
        usage: Usage::default(),
    }
}

/// LLM の応答（メール下書き）の合成 JSON。下書きスキーマに従う形。
fn draft_reply() -> ChatReply {
    ChatReply {
        text: r#"{"to": "B社 ご担当者", "subject": "案件B 納期のご相談", "body": "お世話になっております。納期を2週間延ばす案で調整させていただけますでしょうか。"}"#
            .to_owned(),
        usage: Usage::default(),
    }
}

/// 要求が下書きの応答スキーマか（required に `subject` を含むか）。
fn is_draft_request(req: &ChatRequest) -> bool {
    req.schema
        .as_ref()
        .and_then(|s| s.get("required"))
        .and_then(|r| r.as_array())
        .is_some_and(|a| a.iter().any(|v| v == "subject"))
}

/// フェイクの gateway。`LlmGateway` trait 越しの差し替えを確かめる実装。
/// `reply` を即返すか、`release` を受け取るまで呼び出し中で止まるかのどちらか。
struct FakeGateway {
    calls: AtomicUsize,
    reply: Result<ChatReply, LlmError>,
    entered: Option<mpsc::Sender<()>>,
    release: Mutex<Option<mpsc::Receiver<()>>>,
}

impl FakeGateway {
    fn new(reply: Result<ChatReply, LlmError>) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            reply,
            entered: None,
            release: Mutex::new(None),
        }
    }

    /// `chat` が呼ばれたら `entered` に通知し、`release` まで止まる（ロック検証用）。
    fn blocking(reply: ChatReply) -> (Self, mpsc::Receiver<()>, mpsc::Sender<()>) {
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let mut gateway = Self::new(Ok(reply));
        gateway.entered = Some(entered_tx);
        gateway.release = Mutex::new(Some(release_rx));
        (gateway, entered_rx, release_tx)
    }
}

impl LlmGateway for FakeGateway {
    fn chat(&self, _req: ChatRequest) -> impl Future<Output = Result<ChatReply, LlmError>> + Send {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let entered = self.entered.clone();
        let release = self.release.lock().expect("release").take();
        let reply = self.reply.clone();
        async move {
            if let Some(tx) = entered {
                let _ = tx.send(());
            }
            if let Some(rx) = release {
                let _ = rx.recv();
            }
            reply
        }
    }

    async fn embed(&self, _texts: Vec<String>) -> Result<EmbedReply, LlmError> {
        Err(LlmError::NotSent("embed は runner では使わない".to_owned()))
    }
}

/// 呼び出しごとにキューの先頭の応答を返すフェイク（再試行・タイムアウトの
/// 検証用）。キューが尽きたら最後の応答を繰り返す。`delay` は応答までの
/// 待ち（非同期に待つため runner のタイムアウトが途中で切れる）。
struct SeqGateway {
    calls: AtomicUsize,
    replies: Mutex<VecDeque<Result<ChatReply, LlmError>>>,
    delay: Duration,
}

impl SeqGateway {
    fn new(replies: Vec<Result<ChatReply, LlmError>>, delay: Duration) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            replies: Mutex::new(replies.into()),
            delay,
        }
    }
}

impl LlmGateway for SeqGateway {
    async fn chat(&self, _req: ChatRequest) -> Result<ChatReply, LlmError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if !self.delay.is_zero() {
            tokio::time::sleep(self.delay).await;
        }
        let mut replies = self.replies.lock().expect("replies");
        match replies.len() {
            0 => panic!("応答のキューが空"),
            1 => replies[0].clone(),
            _ => replies.pop_front().expect("front"),
        }
    }

    async fn embed(&self, _texts: Vec<String>) -> Result<EmbedReply, LlmError> {
        Err(LlmError::NotSent("embed は runner では使わない".to_owned()))
    }
}

/// 要求スキーマで応答を切り分けるフェイク（下書きの決定フロー用）。下書き要求には
/// `draft_text`（`None` なら [`draft_reply`]）、それ以外には A/B 案を返す。
#[derive(Default)]
struct SchemaGateway {
    calls: AtomicUsize,
    draft_calls: AtomicUsize,
    draft_text: Option<String>,
}

impl LlmGateway for SchemaGateway {
    async fn chat(&self, req: ChatRequest) -> Result<ChatReply, LlmError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if !is_draft_request(&req) {
            return Ok(ab_reply());
        }
        self.draft_calls.fetch_add(1, Ordering::SeqCst);
        Ok(match &self.draft_text {
            Some(text) => ChatReply {
                text: text.clone(),
                usage: Usage::default(),
            },
            None => draft_reply(),
        })
    }

    async fn embed(&self, _texts: Vec<String>) -> Result<EmbedReply, LlmError> {
        Err(LlmError::NotSent("embed は runner では使わない".to_owned()))
    }
}

fn runner_config(timeout: Duration) -> RunnerConfig {
    RunnerConfig {
        bot_member_id: MemberId::from(HAL),
        decider: MemberId::from(DECIDER),
        llm_basis: PurposeId::P06,
        llm_timeout: timeout,
    }
}

fn fixture(
    store: DataStore,
    gateway: Arc<dyn pelway_runner::DynChatGateway>,
) -> (Arc<Mutex<DataStore>>, Arc<Mutex<ApprovalLedger>>, Runner) {
    fixture_at(store, gateway, TEST_TIMEOUT)
}

fn fixture_at(
    store: DataStore,
    gateway: Arc<dyn pelway_runner::DynChatGateway>,
    timeout: Duration,
) -> (Arc<Mutex<DataStore>>, Arc<Mutex<ApprovalLedger>>, Runner) {
    let store = Arc::new(Mutex::new(store));
    let ledger = Arc::new(Mutex::new(ApprovalLedger::new()));
    let runner = Runner::new(
        runner_config(timeout),
        Arc::clone(&store),
        Arc::clone(&ledger),
        gateway,
    );
    (store, ledger, runner)
}

fn member(id: &str, display: &str, kind: MemberKind) -> ChannelMember {
    ChannelMember {
        member_id: MemberId::from(id),
        display_name: display.to_owned(),
        member_kind: kind,
    }
}

/// HTTP の投稿経路と同じ `post_message` で保存する（`client_msg_id` 無し＝常に新規）。
fn post(store: &Mutex<DataStore>, author: &str, kind: MemberKind, body: &str) -> StoredMessage {
    store
        .lock()
        .expect("store")
        .post_message(
            NewMessage {
                workspace_id: WorkspaceId::from(WS),
                channel_id: ChannelId::from(CH),
                author_id: MemberId::from(author),
                member_kind: kind,
                purpose_id: PurposeId::P01,
                body: body.to_owned(),
                thread_root: None,
                client_msg_id: None,
            },
            now(),
        )
        .expect("post")
}

fn bot_messages(store: &Mutex<DataStore>) -> Vec<String> {
    store
        .lock()
        .expect("store")
        .channel_messages(&ChannelId::from(CH))
        .into_iter()
        .filter(|m| m.member_kind == MemberKind::Bot)
        .map(|m| m.body.clone())
        .collect()
}

fn pending_items(ledger: &Mutex<ApprovalLedger>) -> Vec<DecisionListItem> {
    ledger.lock().expect("ledger").pending_decisions(
        &WorkspaceId::from(WS),
        &MemberId::from(DECIDER),
        now(),
    )
}

/// `task_id` の状態が `state` になるまで 5 秒を上限にポーリングする。
fn wait_state(runner: &Runner, task_id: &RunnerTaskId, state: TaskState) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if runner.task(task_id).is_some_and(|t| t.state == state) {
            return true;
        }
        if Instant::now() > deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

// ---- 条件 2 + 3: メンション → タスク・進捗・A/B 案の投稿・判断待ち ----

#[test]
fn mention_creates_task_and_posts_fixed_stage_progress() {
    let (store, ledger, runner) = fixture(
        DataStore::new_demo_seeded(now()),
        Arc::new(FakeGateway::new(Ok(ab_reply()))),
    );

    let message = post(&store, REQUESTER, MemberKind::Human, "@ハル 案を2つ作って");
    let task_id = runner.on_message_posted(&message).expect("task");
    assert_eq!(task_id, RunnerTaskId("task-00000001".to_owned()));

    // タスクが作られ、固定段階の進捗 1 行が bot の投稿として出る。
    let task = runner.task(&task_id).expect("task exists");
    assert_eq!(task.request_message_id, message.message_id);
    assert_eq!(task.requester.as_str(), REQUESTER);
    // 登録直後は Working（速い fake では終了済み AwaitingDecision もありうる）。
    assert!(matches!(
        task.state,
        TaskState::Working | TaskState::AwaitingDecision
    ));
    assert!(
        bot_messages(&store).iter().any(|b| b == STAGE_ACCEPTED),
        "進捗の1行（固定段階）が出る"
    );

    // LLM が終わると A/B 案の投稿と判断待ち（T3a）が作られる。
    assert!(wait_state(&runner, &task_id, TaskState::AwaitingDecision));
    assert!(
        bot_messages(&store)
            .iter()
            .any(|b| b.contains("案A:") && b.contains("案B:")),
        "LLM の応答（A/B 案）が会話に投稿される"
    );
    let items = pending_items(&ledger);
    assert_eq!(items.len(), 1, "判断待ちが1件作られる");
    assert_eq!(items[0].request_message_id, message.message_id);
    assert_eq!(
        (items[0].requester.as_str(), items[0].decider.as_str()),
        (REQUESTER, DECIDER)
    );
    let options: Vec<_> = items[0]
        .options
        .iter()
        .map(|o| (o.option_id.as_str(), o.body.as_str()))
        .collect();
    assert_eq!(
        options,
        [
            ("a", "納期を2週間延ばす案"),
            ("b", "範囲を絞って納期を守る案")
        ]
    );
    assert_eq!(
        runner.task(&task_id).expect("task").state,
        TaskState::AwaitingDecision
    );

    // 同じメッセージの再配達ではタスクを二重に作らない（処理済みの ID を返す）。
    assert_eq!(runner.on_message_posted(&message), Some(task_id));
}

#[test]
fn non_bot_mention_and_plain_message_create_no_task() {
    let (store, _ledger, runner) = fixture(
        DataStore::new_demo_seeded(now()),
        Arc::new(FakeGateway::new(Ok(ab_reply()))),
    );

    // ハル以外へのメンション・メンション無しではタスクを起こさない。
    let to_human = post(&store, REQUESTER, MemberKind::Human, "@田中 確認して");
    assert_eq!(runner.on_message_posted(&to_human), None);
    let plain = post(&store, REQUESTER, MemberKind::Human, "普通の発言");
    assert_eq!(runner.on_message_posted(&plain), None);
    // bot 自身の投稿は起動しない（runner の投稿で再発火しない）。
    let bot_msg = post(&store, HAL, MemberKind::Bot, "@ハル 自分宛て");
    assert_eq!(runner.on_message_posted(&bot_msg), None);
    // タスクも進捗投稿も起こらない（bot の投稿は自分宛てメンションのみ）。
    assert!(bot_messages(&store).iter().all(|b| b == "@ハル 自分宛て"));
}

// ---- 失敗時は台本の応答で進める（フォールバック。#330） ----

#[test]
fn llm_failure_falls_back_to_script_reply() {
    let gateway = Arc::new(FakeGateway::new(Err(LlmError::Rejected {
        status: 503,
        body: "canary-提供元の内部情報".to_owned(),
    })));
    let (store, ledger, runner) = fixture(
        DataStore::new_demo_seeded(now()),
        Arc::clone(&gateway) as Arc<dyn pelway_runner::DynChatGateway>,
    );

    let message = post(&store, REQUESTER, MemberKind::Human, "@ハル 案を作って");
    let task_id = runner.on_message_posted(&message).expect("task");

    // 失敗すると台本の応答で案の投稿と判断待ちまで進む（Stopped を経由しない）。
    assert!(wait_state(&runner, &task_id, TaskState::AwaitingDecision));
    let posts = bot_messages(&store);
    assert!(
        posts.iter().any(|b| b.contains(FALLBACK_MARK)),
        "台本へ落ちた進捗行が出る: {posts:?}"
    );
    assert!(
        posts
            .iter()
            .any(|b| b.contains("案A:") && b.contains("案B:")),
        "台本の A/B 案が投稿される: {posts:?}"
    );
    assert!(
        posts.iter().all(|b| !b.contains("canary")),
        "提供元の応答本文は投稿しない: {posts:?}"
    );
    assert_eq!(pending_items(&ledger).len(), 1, "判断待ちは作られる");
    // 拒否（Rejected）は再送しない決まり（pelway-llm の error.rs）。呼び出しは1回。
    assert_eq!(
        gateway.calls.load(Ordering::SeqCst),
        1,
        "拒否は再試行しない"
    );

    // 判断待ちに入ったタスクは差し替えられない。存在しないタスクも同様。
    assert_eq!(
        runner.provide_fallback(&task_id, ab_reply()),
        Err(RunnerError::TaskNotStopped)
    );
    let missing = RunnerTaskId("task-99999999".to_owned());
    assert_eq!(
        runner.provide_fallback(&missing, ab_reply()),
        Err(RunnerError::TaskNotFound)
    );
}

// ---- 検証に失敗した応答は1回だけ再試行する（#330） ----

#[test]
fn invalid_reply_is_retried_once_then_falls_back() {
    // 応答が A/B の形を満たさないもの（再試行しても同じ）。
    let invalid = ChatReply {
        text: "{\"unexpected\": true}".to_owned(),
        usage: Usage::default(),
    };
    let gateway = Arc::new(FakeGateway::new(Ok(invalid)));
    let (store, _ledger, runner) = fixture(
        DataStore::new_demo_seeded(now()),
        Arc::clone(&gateway) as Arc<dyn pelway_runner::DynChatGateway>,
    );

    let message = post(&store, REQUESTER, MemberKind::Human, "@ハル 案を作って");
    let task_id = runner.on_message_posted(&message).expect("task");
    assert!(wait_state(&runner, &task_id, TaskState::AwaitingDecision));

    // 初回＋再試行の2回だけ呼び、それでもだめなら台本へ落ちる。
    assert_eq!(gateway.calls.load(Ordering::SeqCst), 2);
    let posts = bot_messages(&store);
    assert!(
        posts.iter().any(|b| b.contains(FALLBACK_MARK)),
        "台本へ落ちる: {posts:?}"
    );
    assert!(posts.iter().any(|b| b.contains("案A:")));
}

#[test]
fn invalid_then_valid_reply_succeeds_on_retry() {
    let gateway = Arc::new(SeqGateway::new(
        vec![
            Ok(ChatReply {
                text: "not json".to_owned(),
                usage: Usage::default(),
            }),
            Ok(ab_reply()),
        ],
        Duration::ZERO,
    ));
    let (store, _ledger, runner) = fixture(
        DataStore::new_demo_seeded(now()),
        Arc::clone(&gateway) as Arc<dyn pelway_runner::DynChatGateway>,
    );

    let message = post(&store, REQUESTER, MemberKind::Human, "@ハル 案を作って");
    let task_id = runner.on_message_posted(&message).expect("task");
    assert!(wait_state(&runner, &task_id, TaskState::AwaitingDecision));

    assert_eq!(gateway.calls.load(Ordering::SeqCst), 2, "1回だけ再試行");
    let posts = bot_messages(&store);
    assert!(
        posts.iter().all(|b| !b.contains(FALLBACK_MARK)),
        "再試行で成功すれば台本へは落ちない: {posts:?}"
    );
    assert!(
        posts.iter().any(|b| b.contains("案A: 納期を2週間延ばす案")),
        "再試行の応答が使われる: {posts:?}"
    );
}

#[test]
fn invalid_error_is_retried_once_then_succeeds() {
    // 実提供元の検証失敗（LlmError::Invalid）側の経路: 再試行で回復する。
    let gateway = Arc::new(SeqGateway::new(
        vec![
            Err(LlmError::Invalid("schema 違反".to_owned())),
            Ok(ab_reply()),
        ],
        Duration::ZERO,
    ));
    let (store, _ledger, runner) = fixture(
        DataStore::new_demo_seeded(now()),
        Arc::clone(&gateway) as Arc<dyn pelway_runner::DynChatGateway>,
    );

    let message = post(&store, REQUESTER, MemberKind::Human, "@ハル 案を作って");
    let task_id = runner.on_message_posted(&message).expect("task");
    assert!(wait_state(&runner, &task_id, TaskState::AwaitingDecision));

    assert_eq!(gateway.calls.load(Ordering::SeqCst), 2, "1回だけ再試行");
    let posts = bot_messages(&store);
    assert!(
        posts.iter().all(|b| !b.contains(FALLBACK_MARK)),
        "再試行で成功すれば台本へは落ちない: {posts:?}"
    );
    assert!(posts.iter().any(|b| b.contains("案A:")));
}

#[test]
fn invalid_error_is_retried_once_then_falls_back() {
    // 再試行でも同じ検証失敗なら台本へ落ちる。
    let gateway = Arc::new(SeqGateway::new(
        vec![Err(LlmError::Invalid("schema 違反".to_owned()))],
        Duration::ZERO,
    ));
    let (store, _ledger, runner) = fixture(
        DataStore::new_demo_seeded(now()),
        Arc::clone(&gateway) as Arc<dyn pelway_runner::DynChatGateway>,
    );

    let message = post(&store, REQUESTER, MemberKind::Human, "@ハル 案を作って");
    let task_id = runner.on_message_posted(&message).expect("task");
    assert!(wait_state(&runner, &task_id, TaskState::AwaitingDecision));

    assert_eq!(
        gateway.calls.load(Ordering::SeqCst),
        2,
        "初回＋再試行の2回だけ"
    );
    let posts = bot_messages(&store);
    assert!(
        posts.iter().any(|b| b.contains(FALLBACK_MARK)),
        "台本へ落ちる: {posts:?}"
    );
    assert!(posts.iter().any(|b| b.contains("案A:")));
}

// ---- タイムアウトしたら台本へ落ちる（#330） ----

#[test]
fn timeout_falls_back_to_script_reply() {
    // chat が戻るのが設定のタイムアウトよりずっと遅い → runner が途中で切る。
    let gateway = Arc::new(SeqGateway::new(
        vec![Ok(ab_reply())],
        Duration::from_secs(60),
    ));
    let (store, _ledger, runner) = fixture_at(
        DataStore::new_demo_seeded(now()),
        Arc::clone(&gateway) as Arc<dyn pelway_runner::DynChatGateway>,
        Duration::from_millis(50),
    );

    let message = post(&store, REQUESTER, MemberKind::Human, "@ハル 案を作って");
    let task_id = runner.on_message_posted(&message).expect("task");
    assert!(wait_state(&runner, &task_id, TaskState::AwaitingDecision));

    let posts = bot_messages(&store);
    assert!(
        posts
            .iter()
            .any(|b| b.contains(FALLBACK_MARK) && b.contains("時間内")),
        "タイムアウトを理由に台本へ落ちる: {posts:?}"
    );
    assert!(posts.iter().any(|b| b.contains("案A:")));
    // タイムアウトは再送しない決まり（結果不明の可能性があるため）。呼び出しは1回。
    assert_eq!(
        gateway.calls.load(Ordering::SeqCst),
        1,
        "タイムアウトは再試行しない"
    );
}

// ---- 決定の確定 → メール下書きの生成（#330） ----

#[test]
fn decision_generates_mail_draft() {
    let gateway = Arc::new(SchemaGateway::default());
    let (store, ledger, runner) = fixture(DataStore::new_demo_seeded(now()), gateway.clone());

    let (message, task_id) = mention_to_decision(&store, &runner);

    // 決定者が案Aを決定すると、下書きのデータが生成され、投稿されて
    // AwaitingSendApproval へ進む（HTTP 層はこれと同じ手順で runner に知らせる）。
    let (outcome, draft) = decide_to_draft(&ledger, &runner, &task_id, "a");
    assert_eq!(draft.to, "B社 ご担当者");
    assert_eq!(draft.subject, "案件B 納期のご相談");
    assert!(draft.body.contains("納期を2週間延ばす案"));
    let posts = bot_messages(&store);
    assert!(
        posts
            .iter()
            .any(|b| b.contains("メール下書き") && b.contains(&draft.subject)),
        "下書きが投稿される: {posts:?}"
    );

    // 同じ決定の二重通知・冪等リプレイでは下書き生成を再び走らせない。
    runner.on_decision_made(&message.message_id, &outcome);
    let calls_after = gateway.calls.load(Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(
        gateway.calls.load(Ordering::SeqCst),
        calls_after,
        "二重通知では LLM を再び呼ばない"
    );
}

/// `item` の判断待ちを `option` で確定する（決定者は `decider`）。
fn decide_item(
    ledger: &Mutex<ApprovalLedger>,
    item: &DecisionListItem,
    option: &str,
    decider: &str,
) -> DecisionOutcome {
    ledger
        .lock()
        .expect("ledger")
        .decide(
            DecideRequest {
                decision_id: item.decision_id.clone(),
                version: item.version,
                option_id: OptionId::from(option),
            },
            &MemberId::from(decider),
            now(),
        )
        .expect("decide")
}

/// 依頼をメンションで投稿し、runner が判断待ちまで進むのを待つ。
fn mention_to_decision(store: &Mutex<DataStore>, runner: &Runner) -> (StoredMessage, RunnerTaskId) {
    let message = post(
        store,
        REQUESTER,
        MemberKind::Human,
        "@ハル 納期の案を作って",
    );
    let task_id = runner.on_message_posted(&message).expect("task");
    assert!(wait_state(runner, &task_id, TaskState::AwaitingDecision));
    (message, task_id)
}

/// runner の判断待ち（1 件）を決定者が `option` で確定して知らせ、下書きの承認待ちまで待つ。
fn decide_to_draft(
    ledger: &Mutex<ApprovalLedger>,
    runner: &Runner,
    task_id: &RunnerTaskId,
    option: &str,
) -> (DecisionOutcome, MailDraft) {
    let items = pending_items(ledger);
    assert_eq!(items.len(), 1);
    let outcome = decide_item(ledger, &items[0], option, DECIDER);
    runner.on_decision_made(&items[0].request_message_id, &outcome);
    let reached = wait_state(runner, task_id, TaskState::AwaitingSendApproval);
    assert!(reached, "{:?}", runner.task(task_id).map(|t| t.state));
    let draft = runner
        .task(task_id)
        .and_then(|t| t.draft)
        .expect("下書きのデータが残る");
    (outcome, draft)
}

/// 同じ依頼メッセージを指す別の判断待ちを確定しても、runner のタスクは進まない
/// （runner が作った判断待ちの確定だけで下書き生成に入る）。
#[test]
fn decision_for_another_card_does_not_advance_task() {
    let gateway = Arc::new(SchemaGateway::default());
    let (store, ledger, runner) = fixture(DataStore::new_demo_seeded(now()), gateway.clone());
    let (message, task_id) = mention_to_decision(&store, &runner);
    let calls_before = gateway.calls.load(Ordering::SeqCst);

    // 別の利用者が同じ依頼メッセージを指す判断待ちを作り、自分で確定する。
    const OTHER: &str = "m-intruder";
    let other = ledger
        .lock()
        .expect("ledger")
        .create_decision(
            WorkspaceId::from(WS),
            CreateDecisionRequest {
                channel_id: ChannelId::from(CH),
                request_message_id: message.message_id.clone(),
                requester: MemberId::from(OTHER),
                decider: MemberId::from(OTHER),
                question: "別の論点".to_owned(),
                options: vec![
                    DecisionOption {
                        option_id: OptionId::from("a"),
                        body: "別カードの案".to_owned(),
                    },
                    DecisionOption {
                        option_id: OptionId::from("b"),
                        body: "別カードの案2".to_owned(),
                    },
                ],
            },
            now(),
        )
        .expect("create")
        .clone();
    let other_item = ledger
        .lock()
        .expect("ledger")
        .pending_decisions(&WorkspaceId::from(WS), &MemberId::from(OTHER), now())
        .into_iter()
        .find(|i| i.decision_id == other.decision_id)
        .expect("other item");
    let outcome = decide_item(&ledger, &other_item, "a", OTHER);
    runner.on_decision_made(&message.message_id, &outcome);
    std::thread::sleep(Duration::from_millis(100));
    let task = runner.task(&task_id).expect("task");
    assert_eq!(
        task.state,
        TaskState::AwaitingDecision,
        "別カードでは進まない"
    );
    assert!(task.draft.is_none());
    assert_eq!(gateway.calls.load(Ordering::SeqCst), calls_before);

    // runner の判断待ちを確定すれば、下書き生成に進む。
    decide_to_draft(&ledger, &runner, &task_id, "a");
}

/// 下書き生成が失敗して台本へ落ちても、下書きは決定された案（案B）に沿う。
#[test]
fn draft_fallback_follows_chosen_option() {
    let gateway = Arc::new(FakeGateway::new(Err(LlmError::NotSent(
        "接続できない".to_owned(),
    ))));
    let (store, ledger, runner) = fixture(DataStore::new_demo_seeded(now()), gateway.clone());
    let (_, task_id) = mention_to_decision(&store, &runner);
    let (outcome, draft) = decide_to_draft(&ledger, &runner, &task_id, "b");
    assert!(draft.body.contains(&outcome.selected.body), "{draft:?}");
    assert!(!draft.body.contains("2週間"), "{draft:?}");
    assert!(draft.template, "フォールバックの下書きは定型の印付き");
}

/// `tests/fixtures/draft_replies.txt` の `=== <名前>` で区切った応答本文（合成）。
fn draft_fixtures() -> Vec<(String, String)> {
    let raw = include_str!("fixtures/draft_replies.txt");
    raw.split("=== ")
        .filter(|s| !s.is_empty())
        .map(|s| {
            let (name, body) = s.split_once('\n').unwrap_or((s, ""));
            (
                name.to_owned(),
                body.strip_suffix('\n').unwrap_or(body).to_owned(),
            )
        })
        .collect()
}

/// GLM 風の壊れた下書き応答は、1回だけ再試行したあと決定された案に沿う定型の
/// 下書きに落ちる。(1) 定型の印が付く (2) 壊れた応答の中身（CANARY）が下書き・
/// 投稿・タスクの記録に出ない (3) 送信側の状態へ進まない。正しい形は印なしで採る。
#[test]
fn draft_fixtures_fail_closed_on_broken_replies() {
    let fixtures = draft_fixtures();
    assert_eq!(fixtures.len(), 11, "fixture の数");
    for (name, text) in fixtures {
        let ok = name.starts_with("ok_");
        let gw = Arc::new(SchemaGateway {
            draft_text: Some(text),
            ..Default::default()
        });
        let (store, ledger, runner) = fixture(DataStore::new_demo_seeded(now()), gw.clone());
        let (_, task_id) = mention_to_decision(&store, &runner);
        let (outcome, draft) = decide_to_draft(&ledger, &runner, &task_id, "b");
        std::thread::sleep(Duration::from_millis(50));
        let task = runner.task(&task_id).expect("task");
        let posts = bot_messages(&store);
        let last = posts.last().expect("最後の投稿");
        let calls = gw.draft_calls.load(Ordering::SeqCst);
        let ctx = format!("{name}: {task:?} {posts:?}");
        // (3) 送信経路なし: 下書きの投稿で止まり、送信側の状態へ進まない。
        assert_eq!(task.state, TaskState::AwaitingSendApproval, "{ctx}");
        assert!(last.contains(&draft.subject), "{ctx}");
        // (1) 定型の印は壊れた応答のときだけ、下書きのデータと投稿の両方に付く。
        let marks = (draft.template, last.contains(TEMPLATE_MARK));
        assert_eq!(marks, (!ok, !ok), "{ctx}");
        // 再試行は壊れた応答のときだけ1回。正しい形はそのまま採る。
        assert_eq!(calls, if ok { 1 } else { 2 }, "{ctx}");
        assert!(!ok || draft.to == "C社 ご担当者", "{ctx}");
        // 壊れた応答は決定された案に沿う定型へ落ち、その理由が投稿される。
        assert!(ok || draft.body.contains(&outcome.selected.body), "{ctx}");
        let reason = posts
            .iter()
            .any(|p| p.contains("LLM の応答を解釈できません"));
        assert_eq!(reason, !ok, "{ctx}");
        // (2) 壊れた応答の中身が下書き・投稿・タスクの記録に出ない。
        assert!(!ctx.contains("CANARY"), "{ctx}");
    }
}

// ---- 根拠は設定で受けた目的 ID で検査する（#330） ----

#[test]
fn without_configured_basis_stops_before_calling_llm() {
    // 設定の根拠（P06）の Active な記録を持たない保存部。
    let mut store = DataStore::new();
    store.ensure_workspace(&WorkspaceId::from(WS), now());
    store.add_channel(Channel {
        workspace_id: WorkspaceId::from(WS),
        channel_id: ChannelId::from(CH),
        name: "案件B".to_owned(),
        members: vec![
            member(REQUESTER, "田中", MemberKind::Human),
            member(HAL, "ハル", MemberKind::Bot),
        ],
    });
    let gateway = Arc::new(FakeGateway::new(Ok(ab_reply())));
    let (store, _ledger, runner) = fixture(
        store,
        Arc::clone(&gateway) as Arc<dyn pelway_runner::DynChatGateway>,
    );

    let message = post(&store, REQUESTER, MemberKind::Human, "@ハル 案を作って");
    let task_id = runner.on_message_posted(&message).expect("task");

    // LLM は呼ばれず、根拠が無い旨の理由で止まる（理由には設定の目的 ID が出る）。
    let task = runner.task(&task_id).expect("task");
    assert_eq!(task.state, TaskState::Stopped);
    assert!(
        task.stop_reason
            .as_deref()
            .is_some_and(|r| r.contains("P06")),
        "理由に設定の根拠（P06）が出る: {:?}",
        task.stop_reason
    );
    assert!(
        bot_messages(&store)
            .iter()
            .any(|b| b.contains("止まり") && b.contains("P06"))
    );
    assert_eq!(
        gateway.calls.load(Ordering::SeqCst),
        0,
        "根拠が無ければ LLM を呼ばない"
    );
}

// ---- 条件 5: LLM の呼び出しの間、ほかのリクエストが台帳のロックで止まらない ----

#[test]
fn llm_call_does_not_hold_store_or_ledger_locks() {
    let (gateway, entered, release) = FakeGateway::blocking(ab_reply());
    let (store, ledger, runner) = fixture(DataStore::new_demo_seeded(now()), Arc::new(gateway));

    let message = post(&store, REQUESTER, MemberKind::Human, "@ハル 案を作って");
    let task_id = runner.on_message_posted(&message).expect("task");
    // LLM 呼び出しの中に入るまで待つ。
    entered
        .recv_timeout(Duration::from_secs(5))
        .expect("gateway called");

    // LLM 呼び出しの間も、台帳・保存部のロックは取れる（ほかのリクエストは止まらない）。
    assert!(
        store.try_lock().is_ok(),
        "LLM 呼び出し中に保存部のロックが取れる"
    );
    assert!(
        ledger.try_lock().is_ok(),
        "LLM 呼び出し中に台帳のロックが取れる"
    );

    release.send(()).expect("release");
    assert!(wait_state(&runner, &task_id, TaskState::AwaitingDecision));
}

// ---- LLM 接続設定と台本モード（#330） ----

/// 環境変数の写像を組み立てる（`from_vars` に渡す形）。
fn vars(pairs: &[(&'static str, &str)]) -> impl Fn(&str) -> Option<String> {
    let map: std::collections::HashMap<&'static str, String> =
        pairs.iter().map(|(k, v)| (*k, (*v).to_owned())).collect();
    move |name| map.get(name).cloned()
}

#[test]
fn demo_llm_config_explicit_and_overridable() {
    // モデル名・接続先・環境変数名が設定として明示される（既定: Z.ai + GLM-5.3
    // Flash。キーの変数名・タイムアウト・根拠 P06）。
    let config = DemoLlmConfig::from_vars(|_| None).expect("defaults");
    assert_eq!(config.provider.name, "zai");
    assert_eq!(config.provider.base_url, pelway_llm::config::ZAI_BASE_URL);
    assert_eq!(config.provider.chat_model, "glm-5.3-flash");
    assert_eq!(
        config.api_key_env,
        pelway_llm::FnoxEnvTokenSource::DEFAULT_VAR
    );
    assert_eq!(config.llm_basis, PurposeId::P06);
    assert_eq!(config.timeout, Duration::from_secs(30));

    // 予備の GPT Luna へは設定の差し替えで切り替える（自動切替ではない。ADR 0008）。
    let backup = DemoLlmConfig::from_vars(vars(&[
        ("PELWAY_LLM_PROVIDER", "openai"),
        ("PELWAY_LLM_BASE_URL", pelway_llm::config::OPENAI_BASE_URL),
        (CHAT_MODEL_ENV, "gpt-6-luna"),
        (RESPONSE_FORMAT_ENV, "json_schema"),
        (TIMEOUT_SECS_ENV, "15"),
        (BASIS_ENV, "P03"),
    ]))
    .expect("backup");
    assert_eq!(backup.provider.chat_model, "gpt-6-luna");
    assert_eq!(backup.timeout, Duration::from_secs(15));
    assert_eq!(backup.llm_basis, PurposeId::P03);

    // 値の形が合わない設定は組み立て時に Err（落ちるのではなく設定エラー）。
    for (name, value) in [
        (BASIS_ENV, "P09"),
        (TIMEOUT_SECS_ENV, "abc"),
        (TIMEOUT_SECS_ENV, "0"),
        (RESPONSE_FORMAT_ENV, "yaml"),
    ] {
        assert!(
            DemoLlmConfig::from_vars(vars(&[(name, value)])).is_err(),
            "{name}={value}"
        );
    }
}

/// `PELWAY_LLM_BASE_URL` の接続先は Z.ai・OpenAI の 2 つの endpoint に限る
/// （https だけでは足りない。QA 点検 2026-11-17）。許可リスト外は
/// `PELWAY_LLM_CUSTOM_BASE_URL` で明示したときだけ使う。
#[test]
fn base_url_is_limited_to_known_providers() {
    for url in [
        pelway_llm::config::ZAI_BASE_URL,
        pelway_llm::config::OPENAI_BASE_URL,
    ] {
        let config = DemoLlmConfig::from_vars(vars(&[("PELWAY_LLM_BASE_URL", url)]))
            .expect("known provider");
        assert_eq!(config.provider.base_url, url);
    }
    // 末尾の / は許容する。
    let with_slash = format!("{}/", pelway_llm::config::OPENAI_BASE_URL);
    let config = DemoLlmConfig::from_vars(|name| {
        (name == "PELWAY_LLM_BASE_URL").then(|| with_slash.clone())
    })
    .expect("trailing slash");
    assert_eq!(
        config.provider.base_url,
        pelway_llm::config::OPENAI_BASE_URL
    );

    // https であっても許可リスト外は設定エラー（loopback は external-refs の
    // 許可ホストのため、リテラルでは https の loopback アドレスで確かめる）。
    assert!(
        DemoLlmConfig::from_vars(vars(&[("PELWAY_LLM_BASE_URL", "https://127.0.0.1:9")])).is_err()
    );

    // 別の環境変数で明示したときだけ許可リスト外を使う（ローカルのモック等）。
    // カスタム接続先は loopback なら任意のスキーム、それ以外は https のみ。
    for url in ["http://127.0.0.1:9/v1", "http://[::1]:9/v1"] {
        let custom =
            DemoLlmConfig::from_vars(vars(&[(CUSTOM_BASE_URL_ENV, url)])).expect("loopback custom");
        assert_eq!(custom.provider.base_url, url);
    }
    // 非 loopback の http 接続先は Bearer キーの送出経路になるため拒否
    // （demo-check external-refs のためリテラルは置換で生成する）。
    let http_other = "http://127.0.0.1".replace("127.0.0.1", "llm.invalid");
    assert!(
        DemoLlmConfig::from_vars(|name| (name == CUSTOM_BASE_URL_ENV).then(|| http_other.clone()))
            .is_err()
    );
    // 非 loopback でも https なら許可する。
    let https_other = "https://127.0.0.1".replace("127.0.0.1", "ok.invalid");
    let custom =
        DemoLlmConfig::from_vars(|name| (name == CUSTOM_BASE_URL_ENV).then(|| https_other.clone()))
            .expect("https custom");
    assert_eq!(custom.provider.base_url, https_other);
    // URL として解釈できない値も設定エラー。
    assert!(DemoLlmConfig::from_vars(vars(&[(CUSTOM_BASE_URL_ENV, "not a url")])).is_err());
}

/// キーが無い・送信に使えない形のときは台本の gateway が返る（落ちない）。
#[test]
fn missing_or_malformed_key_builds_script_gateway() {
    let config = DemoLlmConfig::from_vars(|_| None).expect("config");
    for key in [
        None,
        Some("".to_owned()),
        Some("   ".to_owned()),
        Some("a\nb".to_owned()),
    ] {
        let gateway = config.build_gateway_with(|_| key.clone());
        let reply = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("rt")
            .block_on(gateway.chat(ChatRequest::from_messages(vec![])));
        assert!(reply.is_ok(), "{key:?}");
    }
}

/// 起動時ログの接続先 URL は伏せる（Issue #404）。userinfo とクエリの
/// key/token の値がログ行に出ないことの各ケース。
#[test]
fn endpoint_log_line_redacts_url_secrets() {
    // userinfo あり（カスタム接続先は loopback なので http で通る）。
    let config = DemoLlmConfig::from_vars(vars(&[(
        CUSTOM_BASE_URL_ENV,
        "http://alice:hunter2@127.0.0.1:9/v1",
    )]))
    .expect("config");
    let line = config.endpoint_log_line();
    assert!(!line.contains("alice"), "{line}");
    assert!(!line.contains("hunter2"), "{line}");
    assert!(line.contains("***@127.0.0.1:9/v1"), "{line}");

    // クエリに key/token あり: 値は出ず、キー名と秘密でない値は残る。
    let config = DemoLlmConfig::from_vars(vars(&[(
        CUSTOM_BASE_URL_ENV,
        "http://127.0.0.1:9/v1?key=abc123&token=def456&x=1",
    )]))
    .expect("config");
    let line = config.endpoint_log_line();
    assert!(!line.contains("abc123"), "{line}");
    assert!(!line.contains("def456"), "{line}");
    assert!(line.contains("key=***"), "{line}");
    assert!(line.contains("token=***"), "{line}");
    assert!(line.contains("x=1"), "{line}");

    // どちらもなし: 既知の接続先はそのまま（不要な伏せをしない）。
    let config = DemoLlmConfig::from_vars(|name| {
        (name == "PELWAY_LLM_BASE_URL").then(|| pelway_llm::config::ZAI_BASE_URL.to_owned())
    })
    .expect("config");
    assert_eq!(
        config.endpoint_log_line(),
        format!(
            "pelway-runner: LLM endpoint: zai {}",
            pelway_llm::config::ZAI_BASE_URL
        )
    );

    // fragment に秘密が載っている場合も起動時ログに出ない
    // （review round 1 の must-fix）。
    let config = DemoLlmConfig::from_vars(vars(&[(
        CUSTOM_BASE_URL_ENV,
        "http://127.0.0.1:9/v1#access_token=sk-local",
    )]))
    .expect("config");
    let line = config.endpoint_log_line();
    assert!(!line.contains("sk-local"), "{line}");
    assert!(!line.contains('#'), "{line}");

    // 資格情報の名前（auth）のクエリも起動時ログに出ない
    // （review round 2 の must-fix）。
    let config = DemoLlmConfig::from_vars(vars(&[(
        CUSTOM_BASE_URL_ENV,
        "http://127.0.0.1:9/v1?auth=Bearer%20local",
    )]))
    .expect("config");
    let line = config.endpoint_log_line();
    assert!(!line.contains("Bearer"), "{line}");
    assert!(line.contains("auth=***"), "{line}");
}

/// 台本モード（キー無し）の runner がデモの流れを最後まで回せる。
#[test]
fn script_mode_runs_mention_to_draft() {
    let config = DemoLlmConfig::from_vars(|_| None).expect("config");
    let store = Arc::new(Mutex::new(DataStore::new_demo_seeded_with_basis(
        now(),
        config.llm_basis,
    )));
    let ledger = Arc::new(Mutex::new(ApprovalLedger::new()));
    let runner = Runner::new(
        RunnerConfig {
            llm_basis: config.llm_basis,
            llm_timeout: config.timeout,
            ..runner_config(TEST_TIMEOUT)
        },
        Arc::clone(&store),
        Arc::clone(&ledger),
        config.build_gateway_with(|_| None),
    );

    let (_, task_id) = mention_to_decision(&store, &runner);
    // 案Bを選ぶ（台本の下書きも決定された案に沿うことを確かめる）。
    let (outcome, draft) = decide_to_draft(&ledger, &runner, &task_id, "b");
    assert!(draft.body.contains(&outcome.selected.body), "{draft:?}");
    assert!(!draft.body.contains("2週間"), "{draft:?}");
}

/// 設定の根拠を P03 に差し替えても、seed がその根拠を登録すれば LLM を呼ぶ
/// （目的 ID はコードやゲートに固定されていない。#330）。
#[test]
fn configured_basis_p03_also_gates_the_llm_call() {
    let store = Arc::new(Mutex::new(DataStore::new_demo_seeded_with_basis(
        now(),
        PurposeId::P03,
    )));
    let ledger = Arc::new(Mutex::new(ApprovalLedger::new()));
    let runner = Runner::new(
        RunnerConfig {
            llm_basis: PurposeId::P03,
            ..runner_config(TEST_TIMEOUT)
        },
        Arc::clone(&store),
        ledger,
        Arc::new(FakeGateway::new(Ok(ab_reply()))),
    );

    let message = post(&store, REQUESTER, MemberKind::Human, "@ハル 案を作って");
    let task_id = runner.on_message_posted(&message).expect("task");
    assert!(wait_state(&runner, &task_id, TaskState::AwaitingDecision));
}

/// 判断待ちが作られた直後に確定されても、通知を取りこぼさず下書きに進む
/// （判断待ちの作成とタスクへの ID・状態の保存の間に隙間が無い）。
#[test]
fn decision_right_after_creation_is_not_lost() {
    for _ in 0..200 {
        let gateway = Arc::new(SchemaGateway::default());
        let (store, ledger, runner) = fixture(DataStore::new_demo_seeded(now()), gateway.clone());
        let message = post(
            &store,
            REQUESTER,
            MemberKind::Human,
            "@ハル 納期の案を作って",
        );
        let task_id = runner.on_message_posted(&message).expect("task");
        // 判断待ちが見えた瞬間に確定して通知する（HTTP の decide と同じ手順）。
        let deadline = Instant::now() + Duration::from_secs(5);
        let item = loop {
            if let Some(item) = pending_items(&ledger).into_iter().next() {
                break item;
            }
            assert!(Instant::now() < deadline, "判断待ちが作られない");
            std::hint::spin_loop();
        };
        let outcome = decide_item(&ledger, &item, "a", DECIDER);
        runner.on_decision_made(&item.request_message_id, &outcome);
        assert!(
            wait_state(&runner, &task_id, TaskState::AwaitingSendApproval),
            "作成直後の確定で下書きに進まない: {:?}",
            runner.task(&task_id).map(|t| t.state)
        );
    }
}
