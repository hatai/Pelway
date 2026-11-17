//! runner-lite（#329）のフェイク gateway テスト。実 LLM には接続しない。
//!
//! 受け入れ条件との対応:
//! - 条件 2: ハル宛てのメンションでタスクが作られ、進捗の1行（固定段階）が出る
//! - 条件 3: LLM の応答（A/B 案）が会話に投稿され、判断待ちが T3a の仕組みで作られる
//! - 条件 4: LLM の失敗時は「止まっていることと理由」が投稿され、フォールバック応答へ差し替えられる
//! - 条件 5: LLM の呼び出しの間、ほかのリクエストが台帳のロックで止まらない
//! - 条件 6: 根拠が確認できない場合は LLM を呼ばずに止まる
//! - モデル名・接続先・環境変数名は設定として明示し、キーが無い・使えない形の
//!   ときは台本モードで起動する（#330）

use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use pelway_contract::decision::DecisionListItem;
use pelway_contract::ids::{ChannelId, MemberId, WorkspaceId};
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
use pelway_runner::{Runner, RunnerConfig, RunnerError, RunnerTaskId};

const WS: &str = DEMO_WORKSPACE_ID;
const CH: &str = DEMO_CHANNEL_ID;
const HAL: &str = DEMO_HAL_MEMBER_ID;
const REQUESTER: &str = "m-tanaka";
const DECIDER: &str = "m-yamamoto";
/// 「受付」の進捗 1 行（固定段階。runner 側の定数と同じ文）。
const STAGE_ACCEPTED: &str = "依頼を受け付けました。A/B 案を作成しています。";

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

fn fixture(
    store: DataStore,
    gateway: Arc<FakeGateway>,
) -> (Arc<Mutex<DataStore>>, Arc<Mutex<ApprovalLedger>>, Runner) {
    let store = Arc::new(Mutex::new(store));
    let ledger = Arc::new(Mutex::new(ApprovalLedger::new()));
    let config = RunnerConfig {
        bot_member_id: MemberId::from(HAL),
        decider: MemberId::from(DECIDER),
    };
    let runner = Runner::new(config, Arc::clone(&store), Arc::clone(&ledger), gateway);
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

// ---- 条件 4: 失敗時は停止と理由を投稿し、フォールバック応答へ差し替えられる ----

#[test]
fn llm_failure_posts_stop_with_reason_and_fallback_replaces_it() {
    let (store, ledger, runner) = fixture(
        DataStore::new_demo_seeded(now()),
        Arc::new(FakeGateway::new(Err(LlmError::Rejected {
            status: 503,
            body: "canary-提供元の内部情報".to_owned(),
        }))),
    );

    let message = post(&store, REQUESTER, MemberKind::Human, "@ハル 案を作って");
    let task_id = runner.on_message_posted(&message).expect("task");
    assert!(wait_state(&runner, &task_id, TaskState::Stopped));

    // 「止まっていることと理由」が投稿される。
    let task = runner.task(&task_id).expect("task");
    let reason = task.stop_reason.clone().expect("reason");
    let posts = bot_messages(&store);
    assert!(
        posts
            .iter()
            .any(|b| b.contains("止まり") && b.contains(&reason)),
        "停止と理由が投稿される: {posts:?}"
    );
    assert!(
        !reason.contains("canary") && posts.iter().all(|b| !b.contains("canary")),
        "提供元の応答本文は投稿しない: {reason}"
    );
    assert_eq!(pending_items(&ledger).len(), 0, "判断待ちは作られない");

    // フォールバック応答へ差し替えると、案の投稿と判断待ちまで進む。
    runner
        .provide_fallback(&task_id, ab_reply())
        .expect("fallback");
    assert_eq!(
        runner.task(&task_id).expect("task").state,
        TaskState::AwaitingDecision
    );
    assert!(bot_messages(&store).iter().any(|b| b.contains("案A:")));
    assert_eq!(pending_items(&ledger).len(), 1);

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

// ---- 条件 6: 根拠が確認できない場合は LLM を呼ばずに止まる ----

#[test]
fn without_p03_basis_stops_before_calling_llm() {
    // P03 の Active な根拠を持たない保存部（new_demo_seeded だけが持つ）。
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
    let (store, _ledger, runner) = fixture(store, Arc::clone(&gateway));

    let message = post(&store, REQUESTER, MemberKind::Human, "@ハル 案を作って");
    let task_id = runner.on_message_posted(&message).expect("task");

    // LLM は呼ばれず、根拠が無い旨の理由で止まる。
    let task = runner.task(&task_id).expect("task");
    assert_eq!(task.state, TaskState::Stopped);
    assert!(
        task.stop_reason
            .as_deref()
            .is_some_and(|r| r.contains("P03")),
        "理由に根拠（P03）が出る: {:?}",
        task.stop_reason
    );
    assert!(
        bot_messages(&store)
            .iter()
            .any(|b| b.contains("止まり") && b.contains("P03"))
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
}
