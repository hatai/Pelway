//! runner-lite（#329）。サーバー内プロセスの進行管理として、
//! 「ハルへのメンション → タスク化 → 進捗投稿 → LLM で A/B 案 → 判断待ち」
//! をメモリ上で回す。LLM は [`LlmGateway`] trait 越しに差し替えられる
//! （テストはフェイク。実提供元への接続と台本モードは [`demo`] モジュール。
//! Issue #330。決定後のメール下書き生成は PR-3 で加える）。
//!
//! 固定段階: メンション → タスク登録（Working）と「受付」の進捗 1 行 →
//! LLM 利用の根拠の検査（設定で受けた目的 ID。無ければ LLM を呼ばず停止投稿）
//! → LLM 呼び出し → 成功は A/B 案の投稿と判断待ち（#328）で AwaitingDecision。
//! 検証に失敗した応答は1回だけ再試行し、それでもだめなら台本の応答で進める
//! （フォールバック。タイムアウト・提供元の拒否など他の失敗も同様に台本へ落ちる。
//! 「台本の応答で進めます」の進捗行を出す）。
//! LLM 呼び出しの間はどのロックも持たない。

pub mod demo;

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use pelway_contract::decision::{CreateDecisionRequest, DecisionOption};
use pelway_contract::ids::{ChannelId, MemberId, MessageId, OptionId, WorkspaceId};
use pelway_contract::ledger::ApprovalLedger;
use pelway_contract::member::MemberKind;
use pelway_contract::store::{DataStore, NewMessage, PurposeId, StoredMessage};
use pelway_contract::task::TaskState;
use pelway_llm::{ChatMessage, ChatReply, ChatRequest, LlmError, LlmGateway, Usage};
use serde_json::Value;

use demo::SCRIPT_AB;

/// 「受付」の進捗 1 行（固定段階）。
const STAGE_ACCEPTED: &str = "依頼を受け付けました。A/B 案を作成しています。";
/// A/B 案の投稿の先頭行。
const PROPOSALS_LEAD: &str = "案を作成しました。どちらにするか判断をお願いします。";
/// 外部へ投稿してよい「応答を解釈できない」理由（内部の文字列は含めない）。
const REASON_INVALID: &str = "LLM の応答を解釈できません";
/// LLM 呼び出しが panic・キャンセルで中断したときの固定の理由（内部の値は出さない）。
const REASON_ABORTED: &str = "LLM の呼び出しが中断しました";

/// 根拠が確認できないときの停止理由（目的 ID は設定で受ける）。
fn stop_no_basis(basis: PurposeId) -> String {
    format!("LLM を呼ぶ根拠（{}）が確認できません", basis.as_str())
}

/// 台本へ落ちるときの進捗行（理由は投稿してよい形のものだけ）。
fn fallback_line(reason: &str) -> String {
    format!("LLM から応答を得られなかったため、台本の応答で進めます。（{reason}）")
}

/// runner が LLM を呼ぶ口。`LlmGateway` は `dyn` にできないため `chat` を箱に入れる（実装には自動適用）。
pub trait DynChatGateway: Send + Sync {
    fn chat<'a>(
        &'a self,
        req: ChatRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ChatReply, LlmError>> + Send + 'a>>;
}

impl<T: LlmGateway> DynChatGateway for T {
    fn chat<'a>(
        &'a self,
        req: ChatRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ChatReply, LlmError>> + Send + 'a>> {
        Box::pin(LlmGateway::chat(self, req))
    }
}

/// runner の設定。メンションを受ける AI Bot と、判断待ちの決定者を固定する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunnerConfig {
    /// メンションを受けてタスクを起こす AI Bot の member ID（デモ seed では `m-hal`）。
    pub bot_member_id: MemberId,
    /// 判断待ちの決定者。
    pub decider: MemberId,
    /// LLM を呼ぶ根拠の目的 ID（#330。コードやゲートに固定せず設定で受ける。
    /// デモの既定は P06。BYOK のため。hatai 決定 2026-11-16）。
    pub llm_basis: PurposeId,
    /// LLM の1呼び出しのタイムアウト（これを越えたら失敗として台本へ落ちる。
    /// 実提供元の HTTP タイムアウトとは別に runner 側でも区切る）。
    pub llm_timeout: Duration,
}

/// runner が採番するタスク ID。サーバー付けで連番。
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RunnerTaskId(pub String);

/// runner が管理するタスク 1 件（メモリ上。永続化しない）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunnerTask {
    pub task_id: RunnerTaskId,
    pub workspace_id: WorkspaceId,
    pub channel_id: ChannelId,
    /// 依頼メンションのメッセージ（判断待ちの `request_message_id` に写す）。
    pub request_message_id: MessageId,
    /// メンションを書いた人。
    pub requester: MemberId,
    /// 依頼本文（LLM への要求・判断待ちの論点に使う）。
    pub request_body: String,
    /// 状態。runner が触るのは Working → AwaitingDecision まで（失敗で
    /// Stopped。台本フォールバックは Stopped を経由しない）。
    pub state: TaskState,
    /// Stopped のときの理由（停止メッセージに出したものと同じ）。
    pub stop_reason: Option<String>,
}

/// runner の操作の失敗。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunnerError {
    /// 指定したタスクが無い。
    TaskNotFound,
    /// タスクは停止状態ではない（フォールバックの差し替えは Stopped のタスクだけ）。
    TaskNotStopped,
    /// フォールバック応答を A/B の形に解釈できない、または判断待ちの作成が拒否された。
    InvalidReply(String),
}

/// タスクの帳簿（メモリ上のみ。`by_request` は依頼メッセージの再配達で二重起票しない索引）。
struct TaskBook {
    next: u64,
    tasks: HashMap<RunnerTaskId, RunnerTask>,
    by_request: HashMap<MessageId, RunnerTaskId>,
}

impl TaskBook {
    /// 依頼メッセージからタスクを採番して登録する。
    fn register(&mut self, message: &StoredMessage) -> RunnerTask {
        self.next += 1;
        let task = RunnerTask {
            task_id: RunnerTaskId(format!("task-{:08}", self.next)),
            workspace_id: message.workspace_id.clone(),
            channel_id: message.channel_id.clone(),
            request_message_id: message.message_id.clone(),
            requester: message.author_id.clone(),
            request_body: message.body.clone(),
            state: TaskState::Working,
            stop_reason: None,
        };
        self.by_request
            .insert(task.request_message_id.clone(), task.task_id.clone());
        self.tasks.insert(task.task_id.clone(), task.clone());
        task
    }
}

/// サーバー内プロセスの runner-lite。タスクごとにスレッドを立てて LLM を呼ぶ。
#[derive(Clone)]
pub struct Runner {
    inner: Arc<RunnerInner>,
}

struct RunnerInner {
    config: RunnerConfig,
    store: Arc<Mutex<DataStore>>,
    ledger: Arc<Mutex<ApprovalLedger>>,
    gateway: Arc<dyn DynChatGateway>,
    book: Mutex<TaskBook>,
}

impl Runner {
    /// runner を作る。`gateway` は trait 越しの差し替え（テストはフェイク。
    /// `LlmGateway` の実装ならそのまま渡せる）。
    #[must_use]
    pub fn new(
        config: RunnerConfig,
        store: Arc<Mutex<DataStore>>,
        ledger: Arc<Mutex<ApprovalLedger>>,
        gateway: Arc<dyn DynChatGateway>,
    ) -> Self {
        Self {
            inner: Arc::new(RunnerInner {
                config,
                store,
                ledger,
                gateway,
                book: Mutex::new(TaskBook {
                    next: 0,
                    tasks: HashMap::new(),
                    by_request: HashMap::new(),
                }),
            }),
        }
    }

    /// 投稿されたメッセージを runner に渡すフック。bot へのメンションならタスクを
    /// 起こして ID を返す（bot 自身の投稿・メンション無しでは起こさず、再配達では
    /// 処理済みの ID を返す）。設定の根拠が確認できるときだけスレッドで LLM を呼ぶ。
    #[must_use]
    pub fn on_message_posted(&self, message: &StoredMessage) -> Option<RunnerTaskId> {
        // bot 自身の投稿は起動しない（runner の投稿で再発火しない）。
        if message.member_kind == MemberKind::Bot
            || message.author_id == self.inner.config.bot_member_id
        {
            return None;
        }
        // bot へのメンションか（保存部のロックは読み取りだけの短い区間）。
        let mentioned = self
            .inner
            .store
            .lock()
            .ok()
            .map(|store| {
                store.mentioned_members(&message.workspace_id, &message.channel_id, &message.body)
            })
            .unwrap_or_default();
        if !mentioned.contains(&self.inner.config.bot_member_id) {
            return None;
        }
        // タスク登録（再配達は既存の ID を返す）。
        let task = {
            let mut book = self.inner.book.lock().ok()?;
            if let Some(id) = book.by_request.get(&message.message_id) {
                return Some(id.clone());
            }
            book.register(message)
        };
        // 設定の根拠の検査（ロックは読み取りだけの短い区間）。確認できなければ
        // LLM を呼ばずに止まる。ロックが取れないときも確認できなかったとする。
        let basis = self.inner.config.llm_basis;
        let has_basis = self
            .inner
            .store
            .lock()
            .map(|store| store.is_purpose_enabled(&message.workspace_id, basis, now_ms()))
            .unwrap_or(false);
        if !has_basis {
            self.stop(&task, &stop_no_basis(basis));
            return Some(task.task_id);
        }
        // 「受付」の進捗 1 行を出してから、LLM 呼び出し用のスレッドを立てる。
        let _ = self.post_stage(&task, STAGE_ACCEPTED);
        let runner = self.clone();
        let task_id = task.task_id.clone();
        std::thread::spawn(move || runner.run_task(task));
        Some(task_id)
    }

    /// タスク 1 件を読む。
    #[must_use]
    pub fn task(&self, task_id: &RunnerTaskId) -> Option<RunnerTask> {
        self.inner
            .book
            .lock()
            .ok()
            .and_then(|book| book.tasks.get(task_id).cloned())
    }

    /// 停止中のタスクにフォールバック応答を差し替える（#329）。LLM の応答と
    /// 同じ A/B 形式を取り、案の投稿と判断待ちの作成まで進み AwaitingDecision になる。
    ///
    /// # Errors
    /// タスクが無い・Stopped でない・応答が A/B の形でないとき。
    pub fn provide_fallback(
        &self,
        task_id: &RunnerTaskId,
        reply: ChatReply,
    ) -> Result<(), RunnerError> {
        let Some(task) = self.task(task_id) else {
            return Err(RunnerError::TaskNotFound);
        };
        if task.state != TaskState::Stopped {
            return Err(RunnerError::TaskNotStopped);
        }
        self.complete(&task, reply)
            .map_err(RunnerError::InvalidReply)
    }

    /// タスク用スレッド: LLM を呼び、成功は案の投稿＋判断待ち、失敗は台本の
    /// 応答で案の投稿＋判断待ちへ（フォールバック。Stopped を経由しない）。
    /// 呼び出しの間はどのロックも持たない。
    fn run_task(&self, task: RunnerTask) {
        let request = ab_request(&task.request_body);
        match self.call_chat(&request, |r| parse_ab(&r.text).is_ok()) {
            Ok(reply) => {
                if let Err(reason) = self.complete(&task, reply) {
                    self.stop(&task, &reason);
                }
            }
            Err(reason) => {
                let _ = self.post_stage(&task, &fallback_line(&reason));
                if let Err(reason) = self.complete(&task, script_reply(SCRIPT_AB)) {
                    self.stop(&task, &reason);
                }
            }
        }
    }

    /// LLM を1回呼ぶ。検証に失敗した応答（提供元の検証失敗 [`LlmError::Invalid`]
    /// または `is_valid` を満たさない応答）は1回だけ再試行する。タイムアウト・
    /// その他の失敗は再試行せず、投稿してよい形の理由で `Err` を返す。
    /// ランタイムはタスクごとに1つ作り、再試行でも使い回す。
    fn call_chat(
        &self,
        request: &ChatRequest,
        is_valid: impl Fn(&ChatReply) -> bool,
    ) -> Result<ChatReply, String> {
        let Ok(rt) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        else {
            return Err("ランタイムを立てられません".to_owned());
        };
        let mut retried = false;
        let result = loop {
            match self.block_on_chat(&rt, request.clone()) {
                CallOutcome::Reply(reply) => {
                    if is_valid(&reply) {
                        break Ok(reply);
                    }
                    if retried {
                        break Err(REASON_INVALID.to_owned());
                    }
                    retried = true;
                }
                CallOutcome::Err(err) => {
                    if matches!(err, LlmError::Invalid(_)) && !retried {
                        retried = true;
                        continue;
                    }
                    break Err(llm_reason(&err));
                }
                CallOutcome::Timeout => {
                    break Err("LLM が時間内に応答しませんでした".to_owned());
                }
                CallOutcome::Aborted(reason) => break Err(reason),
            }
        };
        // タイムアウトで残った spawn 中の呼び出しを待たずに畳む（drop だと
        // reqwest の名前解決など spawn_blocking の終了を無期限に待つ）。
        rt.shutdown_background();
        result
    }

    /// `LlmGateway::chat` が返す Future を `rt` 上で動かす。呼び出しは別タスクに
    /// 入れて `JoinError` として拾う（gateway の Future が panic しても
    /// `block_on` を巻き戻ってタスクスレッドを終わらせず、台本へ落ちる理由に写す）。
    /// 設定のタイムアウトを越えた呼び出しは [`CallOutcome::Timeout`] にする。
    fn block_on_chat(&self, rt: &tokio::runtime::Runtime, request: ChatRequest) -> CallOutcome {
        let timeout = self.inner.config.llm_timeout;
        let gateway = Arc::clone(&self.inner.gateway);
        // `tokio::time::timeout` はタイマー要のため、ランタイム上の async ブロック内で
        // 組み立てる（外で作ると "no reactor running" で落ちる）。
        match rt.block_on(async move {
            tokio::time::timeout(
                timeout,
                tokio::spawn(async move { gateway.chat(request).await }),
            )
            .await
        }) {
            Err(_elapsed) => CallOutcome::Timeout,
            // panic・キャンセルのどちらでも中身は出さず固定の理由に写す。
            Ok(Err(_join)) => CallOutcome::Aborted(REASON_ABORTED.to_owned()),
            Ok(Ok(Err(err))) => CallOutcome::Err(err),
            Ok(Ok(Ok(reply))) => CallOutcome::Reply(reply),
        }
    }

    /// A/B の応答を案の投稿と判断待ちに写し、AwaitingDecision へ進める。
    fn complete(&self, task: &RunnerTask, reply: ChatReply) -> Result<(), String> {
        let (option_a, option_b) = parse_ab(&reply.text)?;
        self.post_stage(
            task,
            &format!("{PROPOSALS_LEAD}\n\n案A: {option_a}\n\n案B: {option_b}"),
        )?;
        self.inner
            .ledger
            .lock()
            .map_err(|_| "台帳のロックに失敗しました".to_owned())?
            .create_decision(
                task.workspace_id.clone(),
                CreateDecisionRequest {
                    channel_id: task.channel_id.clone(),
                    request_message_id: task.request_message_id.clone(),
                    requester: task.requester.clone(),
                    decider: self.inner.config.decider.clone(),
                    question: task.request_body.clone(),
                    options: vec![
                        DecisionOption {
                            option_id: OptionId::from("a"),
                            body: option_a,
                        },
                        DecisionOption {
                            option_id: OptionId::from("b"),
                            body: option_b,
                        },
                    ],
                },
                now_ms(),
            )
            .map_err(|_| "判断待ちを作れませんでした".to_owned())?;
        self.set_state(&task.task_id, TaskState::AwaitingDecision, None);
        Ok(())
    }

    /// 「止まっていることと理由」を投稿し、タスクを Stopped にする。
    fn stop(&self, task: &RunnerTask, reason: &str) {
        let _ = self.post_stage(task, &format!("止まりました: {reason}"));
        self.set_state(&task.task_id, TaskState::Stopped, Some(reason.to_owned()));
    }

    /// runner 段階の投稿（bot 発言）。ロックはこの書き込みだけの短い区間。
    fn post_stage(&self, task: &RunnerTask, body: &str) -> Result<(), String> {
        self.inner
            .store
            .lock()
            .map_err(|_| "保存部のロックに失敗しました".to_owned())?
            .add_message(
                NewMessage {
                    workspace_id: task.workspace_id.clone(),
                    channel_id: task.channel_id.clone(),
                    author_id: self.inner.config.bot_member_id.clone(),
                    member_kind: MemberKind::Bot,
                    purpose_id: PurposeId::P01,
                    body: body.to_owned(),
                    thread_root: None,
                    client_msg_id: None,
                },
                now_ms(),
            );
        Ok(())
    }

    fn set_state(&self, task_id: &RunnerTaskId, state: TaskState, stop_reason: Option<String>) {
        if let Ok(mut book) = self.inner.book.lock()
            && let Some(task) = book.tasks.get_mut(task_id)
        {
            task.state = state;
            task.stop_reason = stop_reason;
        }
    }
}

/// LLM 呼び出しの結果（runner 内部用。応答本文など内部の値は残さない）。
enum CallOutcome {
    Reply(ChatReply),
    Err(LlmError),
    /// 設定のタイムアウトを越えた（応答は捨てる）。
    Timeout,
    /// 呼び出しタスクが panic・キャンセルで中断した。
    Aborted(String),
}

/// A/B 案の応答スキーマ（LLM への要求に使う）。
fn ab_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "required": ["option_a", "option_b"],
        "properties": {
            "option_a": {"type": "string"},
            "option_b": {"type": "string"}
        },
        "additionalProperties": false
    })
}

/// LLM への要求。応答は `schema` 指定の A/B JSON に固定し、runner 側でも同じ形に解釈する。
fn ab_request(request_body: &str) -> ChatRequest {
    ChatRequest {
        messages: vec![
            ChatMessage::system(
                "あなたは社内の AI Bot「ハル」です。依頼に対して案を A/B の2つ作ってください。",
            ),
            ChatMessage::user(request_body.to_owned()),
        ],
        schema: Some(ab_schema()),
    }
}

/// 台本の応答（フォールバック用。`usage` は 0 のまま）。
fn script_reply(text: &'static str) -> ChatReply {
    ChatReply {
        text: text.to_owned(),
        usage: Usage::default(),
    }
}

/// 外部へ投稿してよい LLM 失敗の理由。内部の文字列（提供元の応答本文など）は含めない。
fn llm_reason(err: &LlmError) -> String {
    match err {
        LlmError::NotSent(_) => "LLM へ送信できませんでした".to_owned(),
        LlmError::OutcomeUnknown(_) => "LLM の結果が不明です".to_owned(),
        LlmError::Rejected { status, .. } => format!("提供元が拒否しました（status {status}）"),
        LlmError::Invalid(_) => REASON_INVALID.to_owned(),
    }
}

/// 応答本文を A/B の形に解釈する。`option_a`・`option_b` の非空白文字列が必須。
fn parse_ab(text: &str) -> Result<(String, String), String> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|_| invalid_ab())?;
    let pick = |key: &str| pick_str(&value, key).ok_or_else(invalid_ab);
    Ok((pick("option_a")?, pick("option_b")?))
}

fn invalid_ab() -> String {
    "応答を A/B の形に解釈できません".to_owned()
}

/// `value` から `key` の非空白文字列を取り出す。
fn pick_str(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

/// サーバーが付ける現在時刻（UNIX エポックからのミリ秒）。
fn now_ms() -> u64 {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0),
    )
    .unwrap_or(u64::MAX)
}
