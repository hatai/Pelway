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
//! ACP v2 の permission は advisory（仕様明記）のため、この adapter
//! （gateway 側）が担うのは「許可要求を承認カードへ届け、once 系だけを
//! 選ぶ」関門と grant・証跡の不変条件まで。エージェント自身の外部
//! アクセス（ファイルシステム・ネットワーク・環境変数）を止める強制点は
//! 呼び出し元の task container 側の隔離で、この adapter には無い。
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

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use agent_client_protocol::schema::{MaybeUndefined, ProtocolVersion};
use agent_client_protocol::v2::{ConnectionTo, Responder, schema};
use agent_client_protocol::{Agent, ByteStreams, Client, Error};
use futures::io::{AsyncRead as FAsyncRead, AsyncWrite as FAsyncWrite};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::mpsc::{Receiver, channel};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::tier::TierDeclarations;

/// `session/update` の滞留を許す深さ。超過は資源枯渇として扱い session を
/// 失敗させる（無制限キューによるメモリ枯渇の防止）。
const UPDATE_QUEUE_DEPTH: usize = 256;
/// 通知キューに滞留できる合計バイト数の上限（通知の JSON 表現サイズの和）。
/// [`MAX_FRAME_BYTES`] より小さく、frame を通過する大きい通知でも
/// 滞留合計では打ち切れる。
const MAX_UPDATE_BYTES: usize = 2 * 1024 * 1024;
/// 受信する JSON-RPC frame（改行区切りの 1 message）の最大バイト数。
/// SDK のデコード前に byte 層で強制する。
const MAX_FRAME_BYTES: usize = 4 * 1024 * 1024;

/// 累積する agent テキストの上限（byte）。超過は資源枯渇として扱う。
const MAX_OUTPUT_BYTES: usize = 1024 * 1024;

/// toolCallId → 操作名の相関表に保持する項目数・合計バイト数の上限。
/// 悪意ある agent が異なる ID の更新を送り続けても map が無制限に
/// 成長しないよう、超過はキュー溢れと同じく session 失敗とする。
const MAX_SEEN_TOOLS_ENTRIES: usize = 512;
const MAX_SEEN_TOOLS_BYTES: usize = 1024 * 1024;

/// 現在時刻を epoch millisecond で返す。時計が UNIX_EPOCH より前なら
/// `u64::MAX` を返して全ての証跡を失効扱いにする（fail closed）。
fn now_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(u64::MAX)
}

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
    /// transport に結び付けられた連携 ID が grant の連携と一致しなかった
    /// （別の連携宛の grant で別エージェントへ送る誤配送の防止）。
    IntegrationMismatch {
        /// grant の連携。
        expected: String,
        /// launcher が transport に結び付けた連携 ID。
        actual: String,
    },
    /// 承認証跡の値が検証を通らなかった（空・期限切れ等）。
    InvalidEvidence {
        /// 検証に失敗した項目名。
        field: &'static str,
        /// 失敗の理由。
        reason: String,
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
                "transport に結び付けられた連携 {actual} が grant の連携 {expected} と一致しないため接続を拒否した"
            ),
            Self::InvalidEvidence { field, reason } => {
                write!(f, "承認証跡の {field} が無効: {reason}")
            }
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
/// は launcher が transport に結び付けた連携 ID と一致している必要が
/// ある（一致しなければ接続自体を行わず拒否する。エージェントが
/// `initialize` で返す `info.name` は自己申告で、照合には使わない）。
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
/// エージェント側の記録にも残す（決3・案ii）。値は構築時に検証済み
/// （各項目は非空、期限は未到来の epoch ms）で、検証を通った証跡しか
/// 作れない（[`ApprovalEvidence::new`]）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalEvidence {
    decider: String,
    expires_at: u64,
    snapshot_hash: String,
    eligibility: String,
    outcome_unknown: bool,
}

impl ApprovalEvidence {
    /// 証跡を検証して作る。`decider`・`snapshot_hash`・`eligibility` は
    /// 非空（前後空白を除く）、`expires_at` は現在より後の epoch ms。
    /// `outcome_unknown` は「人の決定の結果が不明」で、許可には使え
    /// ない（`answer_permission` が fail closed で止める）。
    pub fn new(
        decider: impl Into<String>,
        expires_at: u64,
        snapshot_hash: impl Into<String>,
        eligibility: impl Into<String>,
        outcome_unknown: bool,
    ) -> Result<Self, AcpError> {
        let decider = decider.into();
        let snapshot_hash = snapshot_hash.into();
        let eligibility = eligibility.into();
        for (field, value) in [
            ("decider", &decider),
            ("snapshot_hash", &snapshot_hash),
            ("eligibility", &eligibility),
        ] {
            if value.trim().is_empty() {
                return Err(AcpError::InvalidEvidence {
                    field,
                    reason: "empty".to_owned(),
                });
            }
        }
        if expires_at <= now_epoch_ms() {
            return Err(AcpError::InvalidEvidence {
                field: "expires_at",
                reason: "expired".to_owned(),
            });
        }
        Ok(Self {
            decider,
            expires_at,
            snapshot_hash,
            eligibility,
            outcome_unknown,
        })
    }

    /// 証跡が既に失効しているか。decider は証跡を保持・再利用できるため、
    /// 構築時の検証に加えて許可を返す時点でも確認する。
    fn is_expired(&self) -> bool {
        self.expires_at <= now_epoch_ms()
    }

    /// 指名承認者（member id または役割の識別子）。
    #[must_use]
    pub fn decider(&self) -> &str {
        &self.decider
    }

    /// 承認の期限（epoch ms、未到来であることを構築時に検証済み）。
    #[must_use]
    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }

    /// 承認対象 payload の snapshot hash（改訂ずれの検出用）。
    #[must_use]
    pub fn snapshot_hash(&self) -> &str {
        &self.snapshot_hash
    }

    /// 開始資格（開始に必要な資格・条件の識別子）。
    #[must_use]
    pub fn eligibility(&self) -> &str {
        &self.eligibility
    }

    /// 人の決定の結果が不明（outcome unknown）なら `true`。
    #[must_use]
    pub fn outcome_unknown(&self) -> bool {
        self.outcome_unknown
    }
}

/// 承認カード側の決定と、その承認モデルの証跡（決3・案ii）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionDecision {
    /// 許可の結論。
    pub answer: PermissionAnswer,
    /// 承認モデルの証跡。応答の `_meta.pelway` に載る。`AllowOnce` には
    /// 必須（無い、または `outcome_unknown` な証跡での許可は
    /// `answer_permission` が fail closed で止める）。`RejectOnce`・
    /// `Cancel` では任意。
    pub evidence: Option<ApprovalEvidence>,
}

impl PermissionDecision {
    /// 証跡の無い決定を作る（`RejectOnce`・`Cancel` 向け。`AllowOnce`
    /// には [`Self::approved`] を使う）。
    #[must_use]
    pub fn answered(answer: PermissionAnswer) -> Self {
        Self {
            answer,
            evidence: None,
        }
    }

    /// 証跡付きの許可決定を作る。
    #[must_use]
    pub fn approved(evidence: ApprovalEvidence) -> Self {
        Self {
            answer: PermissionAnswer::AllowOnce,
            evidence: Some(evidence),
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

/// 改行区切りの JSON-RPC frame が [`MAX_FRAME_BYTES`] を超えたら I/O エラー
/// にする受信側ラッパ。SDK が1行をデコードする前（byte 層）で上限を強制し、
/// 巨大 frame の解析メモリを抑える。超過は接続失敗として扱われる
/// （[`AcpError::Disconnected`] または [`AcpError::Protocol`]）。
struct FrameLimitedRead<R> {
    inner: R,
    /// 直前の改行以降に読んだバイト数（現在の frame の累計長）。
    frame_len: usize,
}

impl<R> FrameLimitedRead<R> {
    fn new(inner: R) -> Self {
        Self {
            inner,
            frame_len: 0,
        }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for FrameLimitedRead<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let before = buf.filled().len();
        match Pin::new(&mut self.inner).poll_read(cx, buf) {
            Poll::Ready(Ok(())) => {
                for &byte in &buf.filled()[before..] {
                    // 上限超過は改行でのリセットより先に即時エラーにする
                    // （超過分と改行が同じ read チャンクに入っても
                    // frame は SDK に渡らない）。
                    self.frame_len += 1;
                    if byte == b'\n' {
                        self.frame_len = 0;
                    } else if self.frame_len > MAX_FRAME_BYTES {
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "JSON-RPC frame exceeds size limit",
                        )));
                    }
                }
                Poll::Ready(Ok(()))
            }
            other => other,
        }
    }
}

/// 呼び出し元の task container で隔離が強制されたエージェントへの
/// transport。隔離を実施した信頼済み launcher だけが、接続先に結び付ける
/// 連携 ID と一緒に構築する。連携 ID は launcher が保証する値で、
/// エージェント自身の申告（`initialize` の name）とは独立に grant と
/// 照合される。受信側には [`FrameLimitedRead`] で frame サイズ上限が
/// 掛けられている。
pub struct IsolatedTransport {
    transport: ByteStreams<Box<dyn FAsyncWrite + Send + Unpin>, Box<dyn FAsyncRead + Send + Unpin>>,
    /// launcher が transport に結び付けた連携 ID。grant の `integration`
    /// と照合し、別連携宛の grant を別エージェントへ誤配送しない。
    integration: String,
}

impl IsolatedTransport {
    /// 隔離を実施した launcher 専用の構築子。task container が環境変数・
    /// cwd・ファイルシステム・ネットワークを制限した状態でエージェントを
    /// 起動し、その read/write（子プロセスの stdio 半分）に連携 ID を
    /// 結び付けて渡す。本 Issue の範囲では launcher はまだ実装されて
    /// いないため本番の構築経路はクレート内専用（後続 Issue で launcher
    /// 実装時に公開面を決める）。クレート外から構築できるのは
    /// [`Self::bind_in_process`]（in-memory 専用）だけ。
    pub(crate) fn bind<R, W>(read: R, write: W, integration: impl Into<String>) -> Self
    where
        R: AsyncRead + Send + Unpin + 'static,
        W: AsyncWrite + Send + Unpin + 'static,
    {
        Self {
            transport: ByteStreams::new(
                Box::new(write.compat_write()),
                Box::new(FrameLimitedRead::new(read).compat()),
            ),
            integration: integration.into(),
        }
    }

    /// 試験用の in-process 構築経路。`DuplexStream` は同一プロセス内の
    /// メモリチャネルであり外部エージェントのプロセス境界をまたがない
    /// ため、この経路で本番の隔離境界を偽造することはできない（子
    /// プロセスの stdio・socket・file は型で受け付けない）。連携 ID の
    /// 結び付けは本番経路と同じ形で練習する。
    #[doc(hidden)]
    #[must_use]
    pub fn bind_in_process(
        stream: tokio::io::DuplexStream,
        integration: impl Into<String>,
    ) -> Self {
        let (read, write) = tokio::io::split(stream);
        Self::bind(read, write, integration)
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
        transport: IsolatedTransport,
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
        let seen_tools = Arc::new(Mutex::new(SeenTools::default()));
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
                            // 相関表が溢れたらキュー溢れと同じく session を
                            // 失敗させる（無制限蓄積の DoS 防止）。
                            if !seen_tools
                                .lock()
                                .expect("tool correlation poisoned")
                                .insert(tool_call.tool_call_id.0.to_string(), name.clone())
                            {
                                overflow.store(true, Ordering::Relaxed);
                                let _ = update_tx.try_send(QueuedUpdate::Overflow);
                            }
                        }
                        // 通知の JSON 表現サイズでキュー滞留量を計量する。
                        let size = serde_json::to_vec(&notification)
                            .map(|bytes| bytes.len())
                            .unwrap_or(usize::MAX);
                        let pending = pending_bytes.fetch_add(size, Ordering::Relaxed) + size;
                        if pending > MAX_UPDATE_BYTES
                            || update_tx
                                .try_send(QueuedUpdate::Notification(Box::new(notification), size))
                                .is_err()
                        {
                            pending_bytes.fetch_sub(size, Ordering::Relaxed);
                            overflow.store(true, Ordering::Relaxed);
                            // 受信側が recv() で待機中でも起こせるよう
                            // 上限超過そのものをキューへ流す（キュー満杯
                            // で載らなくても、受信側の滞留分を処理する
                            // 各 loop で overflow フラグを拾う）。
                            let _ = update_tx.try_send(QueuedUpdate::Overflow);
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
        update_rx: &mut Receiver<QueuedUpdate>,
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
    seen_tools: &Mutex<SeenTools>,
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
    // 不変条件（決3・案ii）: 許可には検証済みの承認証跡（指名承認者・
    // 期限・snapshot hash・開始資格）が必須。証跡が無い、または
    // `outcome_unknown`（人の決定結果が不明）なら許可を選ばず
    // fail closed で止める。値の検証は `ApprovalEvidence::new` が構築時に
    // 済ませているが、decider が証跡を保持・再利用できるため期限は
    // 許可を返す時点でも再確認する。
    if decision.answer == PermissionAnswer::AllowOnce {
        let valid = matches!(
            &decision.evidence,
            Some(evidence) if !evidence.outcome_unknown() && !evidence.is_expired()
        );
        if !valid {
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
            .meta(approval_meta(decision.evidence.as_ref())),
        )
    } else {
        audit.record(AcpAuditEvent::PermissionResolved {
            outcome: "cancelled".to_owned(),
        });
        cancel(request, connection, audit)
    }
}

/// 承認モデルの証跡を `_meta` の `pelway` オブジェクトにする。
/// 証跡が無ければ `_meta` 自体を付けない。
fn approval_meta(evidence: Option<&ApprovalEvidence>) -> Option<schema::Meta> {
    let evidence = evidence?;
    let mut pelway = serde_json::Map::new();
    pelway.insert("decider".to_owned(), evidence.decider().into());
    pelway.insert("expiresAt".to_owned(), evidence.expires_at().into());
    pelway.insert("snapshotHash".to_owned(), evidence.snapshot_hash().into());
    pelway.insert("eligibility".to_owned(), evidence.eligibility().into());
    if evidence.outcome_unknown() {
        pelway.insert("outcomeUnknown".to_owned(), true.into());
    }
    let mut meta = serde_json::Map::new();
    meta.insert("pelway".to_owned(), pelway.into());
    Some(meta)
}

/// toolCallId → 操作名の有界な相関表。項目数と保持バイト数の合計に
/// 上限を持ち、許可要求で引いた項目は消す（相関は単発）。
#[derive(Default)]
struct SeenTools {
    map: HashMap<String, String>,
    bytes: usize,
}

impl SeenTools {
    /// 相関項目を登録する。件数・合計バイト数の上限を超えるなら登録せず
    /// `false` を返す（呼び出し元は session を失敗させる）。同一 ID の
    /// 上書きでは古い項目分を差し引いてから上限を評価する。
    fn insert(&mut self, tool_call_id: String, name: String) -> bool {
        let old_entry = self
            .map
            .get(&tool_call_id)
            .map(|old| tool_call_id.len() + old.len())
            .unwrap_or(0);
        let bytes_after = self.bytes - old_entry + tool_call_id.len() + name.len();
        if (old_entry == 0 && self.map.len() >= MAX_SEEN_TOOLS_ENTRIES)
            || bytes_after > MAX_SEEN_TOOLS_BYTES
        {
            return false;
        }
        self.map.insert(tool_call_id, name);
        self.bytes = bytes_after;
        true
    }

    /// toolCallId の操作名を取り出して相関表から消す。
    fn take(&mut self, tool_call_id: &str) -> Option<String> {
        let name = self.map.remove(tool_call_id)?;
        self.bytes -= tool_call_id.len() + name.len();
        Some(name)
    }
}

/// 許可要求から対象の操作名を取り出す。subject の tool_call.name を第一に、
/// 無ければ先行の `tool_call_update` で見た名前を toolCallId で引く（引いた
/// 項目は消す）。subject が tool call でない（Command・Other・無し）場合は
/// 名前なしとして fail closed 側に流す。
fn subject_operation(
    request: &schema::RequestPermissionRequest,
    seen_tools: &Mutex<SeenTools>,
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
            .take(&tool_call_id),
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

/// 更新通知のキュー項目。`Overflow` は byte/件数の上限超過を受信側へ
/// 届ける sentinel（`recv()` 待機中でも起こすため）。
enum QueuedUpdate {
    /// 通知とその JSON 表現サイズ（`pending_bytes` 計量の払い戻し用）。
    /// Box は enum サイズを抑えるため。
    Notification(Box<schema::UpdateSessionNotification>, usize),
    /// 滞留上限を超えた（`AcpError::UpdateOverflow` で失敗させる）。
    Overflow,
}

/// 同じ session の前景が idle に戻るまで `session/update` を読む。
/// `running`/`requires_action`/未知の状態はすべて「作業中」として扱い、
/// 未知の状態を idle と誤認しない（SDK の quickstart の注意どおり）。
async fn wait_until_idle(
    updates: &mut Receiver<QueuedUpdate>,
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
        let (notification, size) = match updates.recv().await {
            Some(QueuedUpdate::Notification(notification, size)) => (notification, size),
            Some(QueuedUpdate::Overflow) => return Err(AcpError::UpdateOverflow),
            None => return Err(AcpError::Disconnected),
        };
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 1 回の poll_read で受け取る内容を全部返す read。
    struct WholeSlice<'a>(&'a [u8]);

    impl AsyncRead for WholeSlice<'_> {
        fn poll_read(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            buf.put_slice(self.0);
            self.0 = &[];
            Poll::Ready(Ok(()))
        }
    }

    /// 上限超過直後に改行が同じ read チャンクに入っても、改行処理より
    /// 先に即時エラーにして超過 frame を SDK に渡さない。
    #[test]
    fn frame_limit_trips_before_trailing_newline() {
        let mut frame = vec![b'x'; MAX_FRAME_BYTES + 1];
        frame.push(b'\n');
        let mut limited = FrameLimitedRead::new(WholeSlice(&frame));
        let waker = std::task::Waker::noop();
        let mut cx = Context::from_waker(waker);
        let mut data = vec![0u8; frame.len() + 8];
        let mut buf = ReadBuf::new(&mut data);
        let result = Pin::new(&mut limited).poll_read(&mut cx, &mut buf);
        assert!(
            matches!(result, Poll::Ready(Err(_))),
            "over-limit frame with trailing newline must fail before decode"
        );
    }

    /// 改行で区切られた正当な frame はそのまま通す。
    #[test]
    fn frame_limit_allows_valid_frames() {
        let frames = b"{\"a\":1}\n{\"b\":2}\n";
        let mut limited = FrameLimitedRead::new(WholeSlice(frames));
        let waker = std::task::Waker::noop();
        let mut cx = Context::from_waker(waker);
        let mut data = vec![0u8; 64];
        let mut buf = ReadBuf::new(&mut data);
        assert!(matches!(
            Pin::new(&mut limited).poll_read(&mut cx, &mut buf),
            Poll::Ready(Ok(()))
        ));
        assert_eq!(buf.filled(), frames);
    }

    /// 許可時点で失効した証跡は `AllowOnce` を fail closed にする
    /// （decider が構築時は有効だった証跡を保持・再利用するケース）。
    #[test]
    fn expired_evidence_fails_closed_at_allow_time() {
        let evidence = ApprovalEvidence::new(
            "member:1".to_owned(),
            now_epoch_ms() + 50,
            "sha256:aaa".to_owned(),
            "eligible".to_owned(),
            false,
        )
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(60));
        assert!(evidence.is_expired());
    }

    /// 相関表は件数で有界で、取り出した項目は消える。
    #[test]
    fn seen_tools_is_bounded() {
        let mut tools = SeenTools::default();
        for i in 0..MAX_SEEN_TOOLS_ENTRIES {
            assert!(tools.insert(format!("id-{i}"), "tool".to_owned()));
        }
        assert!(!tools.insert("id-overflow".to_owned(), "tool".to_owned()));
        // 取り出しは消す（再取り出しは None）→ 相関は単発。
        assert_eq!(tools.take("id-0"), Some("tool".to_owned()));
        assert!(tools.take("id-0").is_none());
    }

    /// 相関表は合計バイト数でも有界（件数上限より先に byte 上限へ
    /// 達するサイズの項目で確認）。
    #[test]
    fn seen_tools_byte_bounded() {
        let mut tools = SeenTools::default();
        let big = "n".repeat(4 * 1024);
        // ~1 MiB / ~4 KiB ≈ 255 項目で byte 上限に達する（件数上限 512 未満）。
        let mut inserted = 0;
        while tools.insert(format!("id-{inserted}"), big.clone()) {
            inserted += 1;
        }
        assert!(inserted > 0 && inserted < MAX_SEEN_TOOLS_ENTRIES);
    }
}
