//! runner-lite（#329）。サーバー内プロセスの進行管理として、
//! 「ハルへのメンション → タスク化 → 進捗投稿 → LLM で A/B 案 → 判断待ち」を
//! メモリ上で回す。LLM は [`LlmGateway`] trait 越しに差し替えられる
//! （テストはフェイク。実提供元への接続は T2b・対象外）。
//!
//! 固定段階: メンション → タスク登録（Working）と「受付」の進捗 1 行 → P03
//! （LLM 利用の根拠）の検査（無ければ LLM を呼ばず停止投稿）→ LLM 呼び出し
//! → 成功は A/B 案の投稿と判断待ち（#328）で AwaitingDecision、失敗は
//! 「止まっていることと理由」の投稿で Stopped（[`Runner::provide_fallback`]
//! で差し替えられる）。台帳・保存部のロックは読み書きの短い区間だけに
//! 限り、LLM 呼び出しの間はどのロックも持たない。

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use pelway_contract::decision::{CreateDecisionRequest, DecisionOption};
use pelway_contract::ids::{ChannelId, MemberId, MessageId, OptionId, WorkspaceId};
use pelway_contract::ledger::ApprovalLedger;
use pelway_contract::member::MemberKind;
use pelway_contract::store::{DataStore, NewMessage, PurposeId, StoredMessage};
use pelway_contract::task::TaskState;
use pelway_llm::{ChatMessage, ChatReply, ChatRequest, LlmError, LlmGateway};

/// 「受付」の進捗 1 行（固定段階）。
const STAGE_ACCEPTED: &str = "依頼を受け付けました。A/B 案を作成しています。";
/// 根拠（P03）が確認できないときの停止理由。
const STOP_NO_BASIS: &str = "LLM を呼ぶ根拠（P03）が確認できません";
/// A/B 案の投稿の先頭行。
const PROPOSALS_LEAD: &str = "案を作成しました。どちらにするか判断をお願いします。";

/// runner が LLM を呼ぶ口。`LlmGateway` は `impl Future` を返すため `dyn` に
/// できないので、`chat` だけを箱に入れて dyn 越しに呼べる形にする（実装には自動適用）。
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
    /// 状態。runner が触るのは Working → AwaitingDecision | Stopped まで。
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
    /// 処理済みの ID を返す）。根拠（P03）が確認できるときだけスレッドで LLM を呼ぶ。
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
        // 根拠（P03）の検査（ロックは読み取りだけの短い区間）。確認できなければ
        // LLM を呼ばずに止まる。ロックが取れないときも確認できなかったとする。
        let has_basis = self
            .inner
            .store
            .lock()
            .map(|store| store.is_purpose_enabled(&message.workspace_id, PurposeId::P03, now_ms()))
            .unwrap_or(false);
        if !has_basis {
            self.stop(&task, STOP_NO_BASIS);
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

    /// タスク用スレッド: LLM を呼び、成功は案の投稿＋判断待ち、失敗は停止投稿へ。
    /// 呼び出しの間はどのロックも持たない。
    fn run_task(&self, task: RunnerTask) {
        // `LlmGateway::chat` が返す Future を同期スレッドから動かす。実提供元の
        // reqwest が動くよう current_thread ランタイムで `block_on` する。
        let called = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map(|rt| rt.block_on(self.inner.gateway.chat(ab_request(&task.request_body))));
        match called {
            Ok(Ok(reply)) => {
                if let Err(reason) = self.complete(&task, reply) {
                    self.stop(&task, &reason);
                }
            }
            Ok(Err(err)) => {
                self.stop(&task, &format!("LLM の呼び出しに失敗しました（{err}）"));
            }
            Err(err) => {
                self.stop(&task, &format!("LLM のランタイムを立てられません（{err}）"));
            }
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
            .map_err(|e| format!("判断待ちを作れませんでした（{e:?}）"))?;
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

/// LLM への要求。応答は `schema` 指定の A/B JSON に固定し、runner 側でも同じ形に解釈する。
fn ab_request(request_body: &str) -> ChatRequest {
    ChatRequest {
        messages: vec![
            ChatMessage::system(
                "あなたは社内の AI Bot「ハル」です。依頼に対して案を A/B の2つ作ってください。",
            ),
            ChatMessage::user(request_body.to_owned()),
        ],
        schema: Some(serde_json::json!({
            "type": "object",
            "required": ["option_a", "option_b"],
            "properties": {
                "option_a": {"type": "string"},
                "option_b": {"type": "string"}
            },
            "additionalProperties": false
        })),
    }
}

/// 応答本文を A/B の形に解釈する。`option_a`・`option_b` の非空白文字列が必須。
fn parse_ab(text: &str) -> Result<(String, String), String> {
    let invalid = || "応答を A/B の形に解釈できません".to_owned();
    let value: serde_json::Value = serde_json::from_str(text).map_err(|_| invalid())?;
    let pick = |key: &str| {
        value
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .ok_or_else(invalid)
    };
    Ok((pick("option_a")?, pick("option_b")?))
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
