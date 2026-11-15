//! ACP v2 で外部エージェント（Claude Code・Codex 等）を接続する Client adapter
//! の下書き（#133）。BYOK の例外枠で扱う（ADR 0008）。確定スキーマではない。
//!
//! 方式は質問票 R1 の確定回答どおり:
//! - 公式 `agent-client-protocol` crate の `unstable_protocol_v2`（固定版、
//!   `Client.v2()` の型付き API）を使う（R1-Q1 案A）。
//! - gateway が ACP の仲介者（intermediary）位置で Client 役を兼ね、外部
//!   エージェントは task コンテナ内の stdio 子プロセスとする（R1-Q6 案a）。
//! - エージェント種別は data として扱う（R1-Q2 案b）。試験は in-process の
//!   Agent double で行い、実エージェント接続は別 Issue の手動手順。
//! - task ごとの export 対象は [`ExportEnvelope`] に固定する（doc 10、R1-Q3 案a）。
//! - `session/request_permission` は once 系 option だけを選ぶ。承認の決定と
//!   記録は既存の承認カード側（呼び出し元が渡す [`PermissionDecider`]）が担い、
//!   ACP の permission は「判断を求める」面にだけ使う（R1-Q4 案i）。
//! - 段階は Pelway 側の宣言（[`TierDeclarations`]）で判定し、wire には載せ
//!   ない。宣言の無い操作は承認カードを作らず止める（fail closed、ADR 0004、
//!   R1-Q5 案a）。
//!
//! ACP v2 の permission は advisory（仕様明記）のため、承認なしに外部 I/O を
//! 試みない強制点はこの adapter（gateway 側）に残る。
//!
//! この adapter はエージェント・プロセスを起動しない。起動と隔離（環境
//! 変数・cwd・ファイルシステム・ネットワーク）は呼び出し元の task
//! container が transport を作る時点で強制し、ここでは隔離済みの
//! transport だけを受け取る。
//!
//! `session/request_permission` の操作名はエージェント側の申告であり、
//! 不届きなエージェントは要求の省略・操作名の詐称・拒否後の実行が
//! ありうる。したがって permission は「承認カードへ反映する情報面」とし、
//! Pelway が仲介する副作用経路は持たせない: この adapter は
//! `initialize` で client capability を一切 advertise せず（auth・
//! elicitation とも空）、`session/new` で client 提供の MCP server も
//! 追加ディレクトリも渡さない。エージェント自身のアクセスは task
//! container の隔離が止める前提で、操作実行を gateway 側 executor へ
//! 集約する設計（限定 MCP サーバー）は後続 Issue の責務とする。

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use agent_client_protocol::schema::{MaybeUndefined, ProtocolVersion};
use agent_client_protocol::v2::{ConnectionTo, Responder, schema};
use agent_client_protocol::{Agent, Client, ConnectTo, Error};
use tokio::sync::mpsc::{Receiver, channel};

use crate::tier::TierDeclarations;

/// `session/update` の滞留を許す深さ。超過は資源枯渇として扱い session を
/// 失敗させる（無制限キューによるメモリ枯渇の防止）。
const UPDATE_QUEUE_DEPTH: usize = 256;
/// 通知キューに滞留できる合計バイト数の上限（通知の JSON 表現サイズの和）。
const MAX_UPDATE_BYTES: usize = 4 * 1024 * 1024;

/// 累積する agent テキストの上限（byte）。超過は資源枯渇として扱う。
const MAX_OUTPUT_BYTES: usize = 1024 * 1024;

/// ACP adapter の失敗。SDK の [`Error`] をそのまま外に漏らさないための詰め替え。
#[derive(Debug)]
pub enum AcpError {
    /// `integration_grant` に該当する許可が無いため接続を拒否した（doc 10）。
    GrantDenied {
        /// 連携（外部エージェント種別）。
        integration: String,
        /// アカウント。
        account: String,
        /// 操作。
        operation: String,
    },
    /// エージェントが initialize で返した名が grant の連携と一致しなかった
    /// （別の連携宛の grant で別エージェントへ送る誤配送の防止）。
    IntegrationMismatch {
        /// grant の連携（期待したエージェント名）。
        expected: String,
        /// エージェントが initialize で返した名。
        actual: String,
    },
    /// エージェントが v2 の session capability を申告しなかった。
    NoSessionCapability,
    /// ACP プロトコル面の失敗（接続・initialize・session・prompt・close）。
    Protocol(String),
    /// 完了の通知を待つ前にエージェントが切れた。
    Disconnected,
    /// エージェントの更新通知が処理能力を超え、滞留を打ち切った。
    UpdateOverflow,
    /// エージェントの累積出力が上限を超えた。
    OutputTooLarge,
}

impl std::fmt::Display for AcpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GrantDenied {
                integration,
                account,
                operation,
            } => write!(
                f,
                "integration_grant が無いため外部エージェントへの接続を拒否した: {integration} {account} {operation}"
            ),
            Self::IntegrationMismatch { expected, actual } => write!(
                f,
                "エージェントの名 {actual} が grant の連携 {expected} と一致しないため接続を拒否した"
            ),
            Self::NoSessionCapability => {
                write!(
                    f,
                    "エージェントが ACP v2 の session capability を申告しなかった"
                )
            }
            Self::Protocol(m) => write!(f, "ACP の処理が失敗した: {m}"),
            Self::Disconnected => {
                write!(f, "prompt の完了を待つ前にエージェントとの接続が切れた")
            }
            Self::UpdateOverflow => {
                write!(f, "エージェントの更新通知が処理能力を超えた")
            }
            Self::OutputTooLarge => {
                write!(f, "エージェントの累積出力が上限を超えた")
            }
        }
    }
}

impl std::error::Error for AcpError {}

impl From<Error> for AcpError {
    fn from(error: Error) -> Self {
        Self::Protocol(error.to_string())
    }
}

/// `integration_grant` の許可（doc 10）。外部エージェント種別（連携）×
/// アカウント×操作の粒度。有効化は管理者が説明を読んで行う（ADR 0008 と同型）。
/// フィールドは非公開で [`GrantRegistry::require`] でしか取得できないため、
/// [`AcpClient::run_prompt`] の引数が「許可済み」を型で表す。`integration`
/// は接続先エージェントが `initialize` で返す `info.name` と一致している
/// 必要がある（一致しなければ session 開始前に拒否する）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentGrant {
    /// 連携（外部エージェント種別。例: `codex-acp`）。
    integration: String,
    /// アカウント（会社と利用者の識別。例: `acme/alice`）。
    account: String,
    /// 操作（`acp.session` など）。
    operation: String,
}

/// 宣言済みの grant の表。初期は空（doc 10）。未登録の要求はすべて拒否する。
#[derive(Debug, Default)]
pub struct GrantRegistry {
    entries: BTreeMap<(String, String, String), ()>,
}

impl GrantRegistry {
    /// 空の表。未登録の要求はすべて [`AcpError::GrantDenied`]。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// grant を1つ登録する（管理者の明示有効化に対応する設定面）。
    pub fn grant(
        &mut self,
        integration: impl Into<String>,
        account: impl Into<String>,
        operation: impl Into<String>,
    ) -> &mut Self {
        self.entries
            .insert((integration.into(), account.into(), operation.into()), ());
        self
    }

    /// grant の有無を確かめ、あれば [`AgentGrant`] を返す。無ければ
    /// [`AcpError::GrantDenied`]（fail closed）。
    pub fn require(
        &self,
        integration: &str,
        account: &str,
        operation: &str,
    ) -> Result<AgentGrant, AcpError> {
        let key = (
            integration.to_owned(),
            account.to_owned(),
            operation.to_owned(),
        );
        if self.entries.contains_key(&key) {
            Ok(AgentGrant {
                integration: key.0,
                account: key.1,
                operation: key.2,
            })
        } else {
            Err(AcpError::GrantDenied {
                integration: key.0,
                account: key.1,
                operation: key.2,
            })
        }
    }
}

/// task から外部エージェントへ渡す固定 export 対象（doc 10「taskごとの
/// export対象を固定する」、R1-Q3 案a）。`session/prompt` に載るのはこの本文
/// だけで、それ以上の Pelway 側データは（後続 Issue の限定 MCP サーバー経由を
/// 除き）渡さない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportEnvelope {
    body: String,
}

impl ExportEnvelope {
    /// 依頼本文＋明示した入力だけを載せた envelope を組み立てる。
    #[must_use]
    pub fn new(body: impl Into<String>) -> Self {
        Self { body: body.into() }
    }

    /// `session/prompt` に送る本文。
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }
}

/// 承認カード側へ渡す許可要求の要約。決定と記録は呼び出し元（既存の承認
/// フロー）が行う（R1-Q4 案i）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionAsk {
    /// 要求のあった session。
    pub session_id: String,
    /// 対象の toolCallId（要求に無い場合は空文字列）。
    pub tool_call_id: String,
    /// 宣言レジストリで引いた操作名。
    pub operation: String,
    /// エージェントが表示用に付けた要求タイトル。
    pub title: String,
}

/// 承認カード側の決定。once 系だけを許す（`allow_always`/`reject_always` は
/// 運用で禁止、R1-Q4 案i）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionAnswer {
    /// 今回だけ許す（`allow_once` option を選ぶ）。
    AllowOnce,
    /// 今回だけ拒否する（`reject_once` option を選ぶ）。
    RejectOnce,
    /// 承認せず prompt ごと止める（`session/cancel` を送る）。
    Cancel,
}

/// Pelway の承認モデルの証跡。許可応答の `_meta.pelway` に載せて
/// エージェント側の記録にも残す（決3・案ii）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApprovalEvidence {
    /// 指名承認者（member id または役割の識別子）。
    pub decider: Option<String>,
    /// 承認の期限（RFC3339）。
    pub expires_at: Option<String>,
    /// 承認対象 payload の snapshot hash（改訂ずれの検出用）。
    pub snapshot_hash: Option<String>,
    /// 開始資格（開始に必要な資格・条件の識別子）。
    pub eligibility: Option<String>,
    /// 人の決定の結果が不明（outcome unknown）なら `true`。
    pub outcome_unknown: bool,
}

/// 承認カード側の決定と、その承認モデルの証跡（決3・案ii）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionDecision {
    /// 許可の結論。
    pub answer: PermissionAnswer,
    /// 承認モデルの証跡。応答の `_meta.pelway` に載る。
    pub evidence: ApprovalEvidence,
}

impl PermissionDecision {
    /// 証跡の無い決定を作る。
    #[must_use]
    pub fn answered(answer: PermissionAnswer) -> Self {
        Self {
            answer,
            evidence: ApprovalEvidence::default(),
        }
    }
}

/// 承認の決定を担う側。外部エージェントからの許可要求ごとに呼ばれる。
/// 段階が宣言されていない操作には呼ばれない（先に fail closed で止まる）。
pub trait PermissionDecider: Send + Sync {
    /// 要求1件への決定を返す。
    fn decide(&self, ask: PermissionAsk) -> PermissionDecision;
}

/// 監査に記録する adapter の出来事。actor・操作・結果は呼び出し元が相関 ID
/// と一緒に追記型の監査ログへ写す（ADR 0007）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcpAuditEvent {
    /// 接続の試行。
    ConnectAttempt {
        /// 許可済みの連携。
        integration: String,
        /// 許可済みのアカウント。
        account: String,
        /// 許可済みの操作。
        operation: String,
    },
    /// `initialize` の完了（相手の名）。
    Initialized {
        /// エージェント名。
        agent: String,
    },
    /// エージェントの申告名が grant の連携と一致せず接続を止めた。
    IdentityRejected {
        /// grant の連携（期待したエージェント名）。
        expected: String,
        /// エージェントが initialize で返した名。
        actual: String,
    },
    /// `session/new` の完了。
    SessionOpened {
        /// session ID。
        session_id: String,
    },
    /// `session/prompt` の受理。
    PromptSent,
    /// `session/request_permission` の受信。
    PermissionRequested {
        /// 対象の toolCallId。
        tool_call_id: String,
        /// 操作名。
        operation: String,
    },
    /// 宣言の無い操作への許可要求を fail closed で止めた。
    PermissionDeniedUndeclared {
        /// 操作名。
        operation: String,
    },
    /// 承認証跡が不完全、または `outcome_unknown` での許可を fail closed
    /// で止めた（承認なしの許可・不明な結果への許可は認めない）。
    PermissionDeniedIncompleteEvidence {
        /// 操作名。
        operation: String,
    },
    /// 許可要求への応答。
    PermissionResolved {
        /// 応答の種別（`allow_once`/`reject_once`/`cancelled`）。
        outcome: String,
    },
    /// `session/cancel` の送信。
    CancelSent {
        /// session ID。
        session_id: String,
    },
    /// `session/close` の完了。
    SessionClosed {
        /// session ID。
        session_id: String,
    },
}

impl AcpAuditEvent {
    /// 出来事の種別名（監査クエリ用）。
    #[must_use]
    pub fn kind(&self) -> String {
        match self {
            Self::ConnectAttempt { .. } => "connect_attempt",
            Self::Initialized { .. } => "initialized",
            Self::IdentityRejected { .. } => "identity_rejected",
            Self::SessionOpened { .. } => "session_opened",
            Self::PromptSent => "prompt_sent",
            Self::PermissionRequested { .. } => "permission_requested",
            Self::PermissionDeniedUndeclared { .. } => "permission_denied_undeclared",
            Self::PermissionDeniedIncompleteEvidence { .. } => {
                "permission_denied_incomplete_evidence"
            }
            Self::PermissionResolved { .. } => "permission_resolved",
            Self::CancelSent { .. } => "cancel_sent",
            Self::SessionClosed { .. } => "session_closed",
        }
        .to_owned()
    }
}

/// 監査の書き込み先。追記型の監査ログへの写しは呼び出し元が担う。
pub trait AcpAudit: Send + Sync {
    /// 出来事を1件記録する。
    fn record(&self, event: AcpAuditEvent);
}

impl AcpAudit for Mutex<Vec<AcpAuditEvent>> {
    fn record(&self, event: AcpAuditEvent) {
        self.lock().expect("audit sink poisoned").push(event);
    }
}

/// 呼び出し元の task container で隔離が強制されたエージェントへの
/// transport。隔離を実施した信頼済み launcher だけが、接続先に結び付ける
/// 連携 ID と一緒に構築する。連携 ID は launcher が保証する値で、
/// エージェント自身の申告（`initialize` の name）とは独立に grant と
/// 照合される。
pub struct IsolatedTransport<T> {
    transport: T,
    /// launcher が transport に結び付けた連携 ID。grant の `integration`
    /// と照合し、別連携宛の grant を別エージェントへ誤配送しない。
    integration: String,
}

impl<T> IsolatedTransport<T> {
    /// 隔離を実施した launcher 専用の構築子。task container が環境変数・
    /// cwd・ファイルシステム・ネットワークを制限した状態でエージェントを
    /// 起動し、その stdio 相当の transport に連携 ID を結び付けて渡す。
    /// 本 Issue の範囲では launcher はまだ実装されておらず、本番の構築
    /// 経路はクレート内専用とする（後続 Issue で launcher 実装時に
    /// 公開面を決める）。公開の構築経路は [`Self::bind_for_test`] のみ。
    pub(crate) fn bind(transport: T, integration: impl Into<String>) -> Self {
        Self {
            transport,
            integration: integration.into(),
        }
    }

    /// 試験用の構築経路。`bind` と同じく連携 ID を結び付けるが、隔離を
    /// 実施しない transport（in-process double 等）を包める。
    /// 本番コードからは呼ばないこと。
    #[doc(hidden)]
    #[must_use]
    pub fn bind_for_test(transport: T, integration: impl Into<String>) -> Self {
        Self::bind(transport, integration)
    }
}

/// prompt 一往復の結果。
#[derive(Debug, Clone, PartialEq)]
pub struct SessionReport {
    /// 閉じた session の ID。
    pub session_id: String,
    /// 前景の作業が idle に戻ったときの stopReason（`end_turn`/`cancelled` 等）。
    pub stop_reason: Option<schema::StopReason>,
    /// エージェントが出したメッセージ本文の連結。
    pub agent_text: String,
    /// 途中で前景状態 `requires_action` を観測したか。
    pub saw_requires_action: bool,
}

/// ACP v2 の Client 役。段階の宣言（[`TierDeclarations`]）と監査の書き込み先を
/// 持ち、許可要求への関門を担う。
pub struct AcpClient {
    declarations: TierDeclarations,
    audit: Arc<dyn AcpAudit>,
}

impl AcpClient {
    /// 宣言表と監査 sink を結び付けて作る。
    #[must_use]
    pub fn new(declarations: TierDeclarations, audit: Arc<dyn AcpAudit>) -> Self {
        Self {
            declarations,
            audit,
        }
    }

    /// 隔離済みの transport（task container 内で起動された子プロセスの
    /// stdio、試験の in-process double）で envelope を1往復送る。
    /// `initialize`（`protocolVersion: 2`）→ `session/new` →
    /// `session/prompt` → 前景が idle に戻るまで待ち → `session/close`。
    /// 許可要求は [`Self::answer_permission`] の関門を通る。
    ///
    /// このアダプタはエージェントのプロセスを起動しない。起動・環境変数・
    /// cwd・ファイルシステム・ネットワークの隔離は呼び出し元（task
    /// container、doc 10 の runner 境界）が transport を作る時点で強制
    /// する前提。引数が [`IsolatedTransport`] なのは「隔離を実施した
    /// launcher が連携 ID を結び付けた transport だけが渡せる」ことを
    /// 型で表すため。接続前に、transport に結び付けられた連携 ID が
    /// grant の連携と一致するか照合し、一致しなければ何も送らず拒否する
    /// （`initialize` の name はエージェント自身の申告値で認証になら
    /// ないため、launcher が結び付けた ID で照合する）。
    pub async fn run_prompt(
        &self,
        grant: &AgentGrant,
        transport: IsolatedTransport<impl ConnectTo<Client> + 'static>,
        envelope: &ExportEnvelope,
        decider: Arc<dyn PermissionDecider>,
    ) -> Result<SessionReport, AcpError> {
        self.audit.record(AcpAuditEvent::ConnectAttempt {
            integration: grant.integration.clone(),
            account: grant.account.clone(),
            operation: grant.operation.clone(),
        });
        // grant の連携と transport に結び付けられた連携 ID の照合。
        // 一致しなければ接続自体を行わない（別連携宛の grant を別
        // エージェントへ誤配送するのを防ぐ）。
        if transport.integration != grant.integration {
            self.audit.record(AcpAuditEvent::IdentityRejected {
                expected: grant.integration.clone(),
                actual: transport.integration.clone(),
            });
            return Err(AcpError::IntegrationMismatch {
                expected: grant.integration.clone(),
                actual: transport.integration.clone(),
            });
        }
        // エージェントからの更新通知を溜める有界キュー。件数だけでなく
        // 合計バイト数にも上限を持たせ（通知の JSON 表現サイズで計量）、
        // 超過した通知は捨てて overflow を立て、wait_until_idle で
        // session を失敗・停止させる（無制限蓄積によるメモリ枯渇の防止）。
        // JSON-RPC の frame 層は SDK 内部のためデコード前のサイズ制限は
        // 掛けられず、通知単位の計量で代替する。
        let (update_tx, mut update_rx) = channel(UPDATE_QUEUE_DEPTH);
        let overflow = Arc::new(AtomicBool::new(false));
        let pending_bytes = Arc::new(AtomicUsize::new(0));
        // toolCallId → 操作名の相関表。許可要求の subject が名前を持たない
        // 場合に、先行する tool_call_update の `name` で引く（R1-Q5 案aの
        // 「toolCallId/名前は判定キーへの相関にだけ使う」）。
        let seen_tools = Arc::new(Mutex::new(HashMap::<String, String>::new()));
        let declarations = self.declarations.clone();
        let audit = Arc::clone(&self.audit);
        let envelope_body = envelope.body().to_owned();

        Client
            .v2()
            .name("pelway-gateway")
            .on_receive_notification(
                {
                    let seen_tools = Arc::clone(&seen_tools);
                    let overflow = Arc::clone(&overflow);
                    let pending_bytes = Arc::clone(&pending_bytes);
                    async move |notification: schema::UpdateSessionNotification,
                                _connection: ConnectionTo<Agent>| {
                        if let schema::SessionUpdate::ToolCallUpdate(tool_call) =
                            &notification.update
                            && let MaybeUndefined::Value(name) = &tool_call.name
                        {
                            seen_tools
                                .lock()
                                .expect("tool correlation poisoned")
                                .insert(tool_call.tool_call_id.0.to_string(), name.clone());
                        }
                        // 通知の JSON 表現サイズでキュー滞留量を計量する。
                        let size = serde_json::to_vec(&notification)
                            .map(|bytes| bytes.len())
                            .unwrap_or(usize::MAX);
                        let pending = pending_bytes.fetch_add(size, Ordering::Relaxed) + size;
                        if pending > MAX_UPDATE_BYTES
                            || update_tx.try_send((notification, size)).is_err()
                        {
                            pending_bytes.fetch_sub(size, Ordering::Relaxed);
                            overflow.store(true, Ordering::Relaxed);
                        }
                        Ok(())
                    }
                },
                agent_client_protocol::on_receive_notification!(),
            )
            .on_receive_request(
                async move |request: schema::RequestPermissionRequest,
                            responder: Responder<schema::RequestPermissionResponse>,
                            connection: ConnectionTo<Agent>| {
                    responder.respond(answer_permission(
                        &request,
                        &declarations,
                        &seen_tools,
                        decider.as_ref(),
                        audit.as_ref(),
                        &connection,
                    )?)
                },
                agent_client_protocol::on_receive_request!(),
            )
            .connect_with(transport.transport, async move |connection| {
                // AcpError の種別（GrantDenied 系・NoSessionCapability・
                // Disconnected）を保ったまま外へ出すため、内側の結果を
                // Result<_, AcpError> で包む。SDK の Error は Protocol に詰める。
                match self
                    .run_session(
                        &connection,
                        &envelope_body,
                        &mut update_rx,
                        &overflow,
                        &pending_bytes,
                    )
                    .await
                {
                    Ok(report) => Ok(Ok(report)),
                    Err(AcpError::Protocol(message)) => Err(Error::internal_error().data(message)),
                    Err(acp_error) => Ok(Err(acp_error)),
                }
            })
            .await
            .map_err(|error| AcpError::Protocol(error.to_string()))?
    }
}

impl AcpClient {
    /// connect_with の内側で走らせる session 本体。`initialize` →
    /// `session/new` → `session/prompt` → idle 待ち → `session/close`。
    async fn run_session(
        &self,
        connection: &ConnectionTo<Agent>,
        envelope_body: &str,
        update_rx: &mut Receiver<(schema::UpdateSessionNotification, usize)>,
        overflow: &AtomicBool,
        pending_bytes: &AtomicUsize,
    ) -> Result<SessionReport, AcpError> {
        let initialize = connection
            .send_request(schema::InitializeRequest::new(
                ProtocolVersion::V2,
                schema::Implementation::new("pelway-gateway", env!("CARGO_PKG_VERSION")),
            ))
            .block_task()
            .await?;
        if initialize.capabilities.session.is_none() {
            return Err(AcpError::NoSessionCapability);
        }
        self.audit.record(AcpAuditEvent::Initialized {
            agent: initialize.info.name.clone(),
        });
        let opened = connection
            .build_session_cwd()?
            .start_session()
            .block_task()
            .await?;
        let session = opened.session;
        let session_id = session.session_id().0.to_string();
        self.audit.record(AcpAuditEvent::SessionOpened {
            session_id: session_id.clone(),
        });
        session.send_prompt(envelope_body).block_task().await?;
        self.audit.record(AcpAuditEvent::PromptSent);
        match wait_until_idle(update_rx, &session_id, overflow, pending_bytes).await {
            Ok(mut report) => {
                report.session_id = session_id.clone();
                session.close().block_task().await?;
                self.audit
                    .record(AcpAuditEvent::SessionClosed { session_id });
                Ok(report)
            }
            Err(error) => {
                // リソース上限違反: agent に cancel を送って前景作業を
                // 止め、session close を試みてから失敗を返す。エラーだけ
                // 返して agent を動かし続けさせない。
                let _ = connection.send_notification(schema::CancelSessionNotification::new(
                    session.session_id().clone(),
                ));
                self.audit.record(AcpAuditEvent::CancelSent {
                    session_id: session_id.clone(),
                });
                let _ = tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    session.close().block_task(),
                )
                .await;
                Err(error)
            }
        }
    }
}

/// 許可要求への関門。ADR 0004 の fail closed（宣言の無い操作は止める）と、
/// once 系限定（`allow_always`/`reject_always` は選ばない）を強制する。
/// 承認の決定自体は `decider`（既存の承認カード側）に委ねる。
fn answer_permission(
    request: &schema::RequestPermissionRequest,
    declarations: &TierDeclarations,
    seen_tools: &Mutex<HashMap<String, String>>,
    decider: &dyn PermissionDecider,
    audit: &dyn AcpAudit,
    connection: &ConnectionTo<Agent>,
) -> Result<schema::RequestPermissionResponse, Error> {
    let (tool_call_id, operation) = subject_operation(request, seen_tools);
    audit.record(AcpAuditEvent::PermissionRequested {
        tool_call_id: tool_call_id.clone(),
        operation: operation.clone(),
    });

    // 段階が宣言されていない操作は取り消せない操作として扱い、承認カードを
    // 作らず止める（fail closed）。annotations は使わない（ADR 0004）。
    if operation.is_empty() || !declarations.is_declared(&operation) {
        audit.record(AcpAuditEvent::PermissionDeniedUndeclared {
            operation: operation.clone(),
        });
        return reject_or_cancel(request, connection, audit);
    }

    let decision = decider.decide(PermissionAsk {
        session_id: request.session_id.0.to_string(),
        tool_call_id,
        operation: operation.clone(),
        title: request.title.clone(),
    });
    // 不変条件（決3・案ii）: 許可には完全な承認証跡（指名承認者・期限・
    // snapshot hash・開始資格）が必須。証跡が欠ける、または
    // `outcome_unknown`（人の決定結果が不明）なら許可を選ばず
    // fail closed で止める。
    if decision.answer == PermissionAnswer::AllowOnce {
        let evidence = &decision.evidence;
        let complete = evidence.decider.is_some()
            && evidence.expires_at.is_some()
            && evidence.snapshot_hash.is_some()
            && evidence.eligibility.is_some();
        if evidence.outcome_unknown || !complete {
            audit.record(AcpAuditEvent::PermissionDeniedIncompleteEvidence { operation });
            return reject_or_cancel(request, connection, audit);
        }
    }
    let wanted = match decision.answer {
        PermissionAnswer::AllowOnce => schema::PermissionOptionKind::AllowOnce,
        PermissionAnswer::RejectOnce => schema::PermissionOptionKind::RejectOnce,
        PermissionAnswer::Cancel => {
            audit.record(AcpAuditEvent::PermissionResolved {
                outcome: "cancelled".to_owned(),
            });
            return cancel(request, connection, audit);
        }
    };
    // once 系 option が無ければ always 系は選ばず cancelled で応える
    // （R1-Q4 案i。「記憶する選択肢」を Pelway の承認として使わない）。
    if let Some(option) = request.options.iter().find(|option| option.kind == wanted) {
        audit.record(AcpAuditEvent::PermissionResolved {
            outcome: option.option_id.0.to_string(),
        });
        // 決3・案ii: Pelway の承認モデルの証跡を `_meta.pelway` に載せる。
        Ok(
            schema::RequestPermissionResponse::new(schema::RequestPermissionOutcome::Selected(
                schema::SelectedPermissionOutcome::new(option.option_id.clone()),
            ))
            .meta(approval_meta(&decision.evidence)),
        )
    } else {
        audit.record(AcpAuditEvent::PermissionResolved {
            outcome: "cancelled".to_owned(),
        });
        cancel(request, connection, audit)
    }
}

/// 承認モデルの証跡を `_meta` の `pelway` オブジェクトにする。
/// 証跡が空なら `_meta` 自体を付けない。
fn approval_meta(evidence: &ApprovalEvidence) -> Option<schema::Meta> {
    let mut pelway = serde_json::Map::new();
    if let Some(decider) = &evidence.decider {
        pelway.insert("decider".to_owned(), decider.clone().into());
    }
    if let Some(expires_at) = &evidence.expires_at {
        pelway.insert("expiresAt".to_owned(), expires_at.clone().into());
    }
    if let Some(snapshot_hash) = &evidence.snapshot_hash {
        pelway.insert("snapshotHash".to_owned(), snapshot_hash.clone().into());
    }
    if let Some(eligibility) = &evidence.eligibility {
        pelway.insert("eligibility".to_owned(), eligibility.clone().into());
    }
    if evidence.outcome_unknown {
        pelway.insert("outcomeUnknown".to_owned(), true.into());
    }
    if pelway.is_empty() {
        return None;
    }
    let mut meta = serde_json::Map::new();
    meta.insert("pelway".to_owned(), pelway.into());
    Some(meta)
}

/// 許可要求から対象の操作名を取り出す。subject の tool_call.name を第一に、
/// 無ければ先行の `tool_call_update` で見た名前を toolCallId で引く。
/// subject が tool call でない（Command・Other・無し）場合は名前なしとして
/// fail closed 側に流す。
fn subject_operation(
    request: &schema::RequestPermissionRequest,
    seen_tools: &Mutex<HashMap<String, String>>,
) -> (String, String) {
    let Some(schema::RequestPermissionSubject::ToolCall(subject)) = &request.subject else {
        return (String::new(), String::new());
    };
    let tool_call_id = subject.tool_call.tool_call_id.0.to_string();
    let name = match &subject.tool_call.name {
        MaybeUndefined::Value(name) => Some(name.clone()),
        _ => seen_tools
            .lock()
            .expect("tool correlation poisoned")
            .get(&tool_call_id)
            .cloned(),
    };
    (tool_call_id, name.unwrap_or_default())
}

/// `reject_once` option があればそれを選んで応え、無ければ prompt ごと
/// `session/cancel` する（fail closed の応答）。
fn reject_or_cancel(
    request: &schema::RequestPermissionRequest,
    connection: &ConnectionTo<Agent>,
    audit: &dyn AcpAudit,
) -> Result<schema::RequestPermissionResponse, Error> {
    if let Some(option) = request
        .options
        .iter()
        .find(|option| option.kind == schema::PermissionOptionKind::RejectOnce)
    {
        return Ok(schema::RequestPermissionResponse::new(
            schema::RequestPermissionOutcome::Selected(schema::SelectedPermissionOutcome::new(
                option.option_id.clone(),
            )),
        ));
    }
    cancel(request, connection, audit)
}

/// `session/cancel` を送り、許可要求には cancelled outcome で応える。
fn cancel(
    request: &schema::RequestPermissionRequest,
    connection: &ConnectionTo<Agent>,
    audit: &dyn AcpAudit,
) -> Result<schema::RequestPermissionResponse, Error> {
    connection.send_notification(schema::CancelSessionNotification::new(
        request.session_id.clone(),
    ))?;
    audit.record(AcpAuditEvent::CancelSent {
        session_id: request.session_id.0.to_string(),
    });
    Ok(schema::RequestPermissionResponse::new(
        schema::CancelledPermissionOutcome::new().into(),
    ))
}

/// 同じ session の前景が idle に戻るまで `session/update` を読む。
/// `running`/`requires_action`/未知の状態はすべて「作業中」として扱い、
/// 未知の状態を idle と誤認しない（SDK の quickstart の注意どおり）。
async fn wait_until_idle(
    updates: &mut Receiver<(schema::UpdateSessionNotification, usize)>,
    session_id: &str,
    overflow: &AtomicBool,
    pending_bytes: &AtomicUsize,
) -> Result<SessionReport, AcpError> {
    let mut text = String::new();
    let mut observed_active = false;
    let mut saw_requires_action = false;
    loop {
        if overflow.load(Ordering::Relaxed) {
            return Err(AcpError::UpdateOverflow);
        }
        let (notification, size) = updates.recv().await.ok_or(AcpError::Disconnected)?;
        pending_bytes.fetch_sub(size, Ordering::Relaxed);
        if notification.session_id.0.as_ref() != session_id {
            continue;
        }
        match notification.update {
            schema::SessionUpdate::StateUpdate(schema::StateUpdate::Idle(idle)) => {
                if observed_active {
                    return Ok(SessionReport {
                        session_id: session_id.to_owned(),
                        stop_reason: idle.stop_reason,
                        agent_text: text,
                        saw_requires_action,
                    });
                }
            }
            schema::SessionUpdate::StateUpdate(schema::StateUpdate::RequiresAction(_)) => {
                observed_active = true;
                saw_requires_action = true;
            }
            schema::SessionUpdate::StateUpdate(_) => observed_active = true,
            schema::SessionUpdate::AgentMessage(message) if observed_active => {
                if let MaybeUndefined::Value(blocks) = message.content {
                    for block in blocks {
                        if let schema::ContentBlock::Text(t) = block {
                            text.push_str(&t.text);
                        }
                    }
                }
            }
            schema::SessionUpdate::AgentMessageChunk(chunk) if observed_active => {
                if let schema::ContentBlock::Text(t) = chunk.content {
                    text.push_str(&t.text);
                }
            }
            _ => {}
        }
        if text.len() > MAX_OUTPUT_BYTES {
            return Err(AcpError::OutputTooLarge);
        }
    }
}
