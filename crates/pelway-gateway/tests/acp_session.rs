//! ACP v2 による外部エージェント接続の Client adapter（`acp` feature）の試験
//! （#133）。in-process の Agent double（公式 rust-sdk の `Agent.v2()`）との
//! session/new → prompt → requires_action → 承認 → idle → close の一往復と、
//! grant 判定・段階の fail closed・once 系限定・cancel・監査記録を確かめる。
//! 実エージェント（codex-acp 等）との接続は対象外（質問票 R1-Q2(b)・R1-Q7(a)）。

#![cfg(feature = "acp")]

use std::sync::{Arc, Mutex};

use agent_client_protocol::v2::{ConnectionTo, Responder, schema};
use agent_client_protocol::{Agent, ByteStreams, Client, Result};
use pelway_gateway::acp::{
    AcpAuditEvent, AcpClient, AcpError, ApprovalEvidence, ExportEnvelope, GrantRegistry,
    IsolatedTransport, PermissionAnswer, PermissionAsk, PermissionDecider, PermissionDecision,
};
use pelway_gateway::tier::{ActionTier, TierDeclarations};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

/// Agent double の振る舞い指定。
#[derive(Clone, Default)]
struct DoubleSpec {
    /// prompt の処理中にこの操作名の許可を求める（`tool_call_update` ＋
    /// `session/request_permission`）。`None` なら許可要求を送らず echo で終える。
    permission_tool: Option<&'static str>,
    /// 許可要求で提示する option の kind（提示順）。
    option_kinds: Vec<schema::PermissionOptionKind>,
    /// initialize で返すエージェント名。`None` なら `acp-double`。
    agent_name: Option<&'static str>,
    /// prompt への応答の前にこの数の小さい update を連投する（キュー溢れの確認）。
    flood_updates: Option<usize>,
    /// prompt への応答の前に、合計このバイト数になるまで中サイズの
    /// update を連投する（キューのバイト上限の確認。各 frame は
    /// frame 上限未満に割る）。
    flood_bytes: Option<usize>,
    /// prompt への応答の前にこのバイト数の巨大 update を 1 本送る
    /// （SDK デコード前の frame 上限の確認）。
    giant_frame_bytes: Option<usize>,
    /// prompt 応答後（クライアントが idle 待ちに入った後）にこのバイト数の
    /// update を 1 本送る（frame 上限未満・滞留 byte 上限超過の単発通知で
    /// 待機解除されることの確認）。
    late_giant_bytes: Option<usize>,
    /// idle の前に 1 本の巨大テキストを送る（出力上限の確認）。
    huge_output: bool,
    /// 異なる toolCallId の `tool_call_update` をこの数だけ連投する
    /// （toolCallId 相関表の上限の確認）。
    flood_tool_calls: Option<usize>,
}

/// 検証用の ACP Agent。v2 の baseline（initialize・session/new・prompt・cancel・
/// close）を扱い、prompt では spec に従って許可を求めるか echo する。
#[derive(Clone)]
struct Double {
    spec: DoubleSpec,
    state: Arc<Mutex<DoubleState>>,
}

#[derive(Default)]
struct DoubleState {
    session_seq: u64,
    cancelled: bool,
    /// クライアントが permission request に返した結果の記録（表示用の文字列）。
    permission_outcome: Option<String>,
    /// クライアントが permission 応答に載せた `_meta`（承認モデル証跡の確認用）。
    permission_meta: Option<serde_json::Value>,
    /// 受け取った initialize の client capabilities（零 capability の確認用）。
    client_capabilities: Option<schema::ClientCapabilities>,
    /// 受け取った session/new の `mcpServers` と `additionalDirectories` の件数。
    new_session_channels: Option<(usize, usize)>,
}

const DOUBLE_SESSION: &str = "double-session-1";

fn send_update(
    connection: &ConnectionTo<Client>,
    session_id: &str,
    update: schema::SessionUpdate,
) -> Result<()> {
    connection.send_notification(schema::UpdateSessionNotification::new(
        schema::SessionId::new(session_id),
        update,
    ))
}

fn option_for(kind: &schema::PermissionOptionKind) -> schema::PermissionOption {
    let (id, name) = match kind {
        schema::PermissionOptionKind::AllowOnce => ("allow_once", "allow once"),
        schema::PermissionOptionKind::AllowAlways => ("allow_always", "allow always"),
        schema::PermissionOptionKind::RejectOnce => ("reject_once", "reject once"),
        schema::PermissionOptionKind::RejectAlways => ("reject_always", "reject always"),
        schema::PermissionOptionKind::Other(_) => ("other", "other"),
        _ => ("unknown", "unknown"),
    };
    schema::PermissionOption::new(id, name, kind.clone())
}

impl Double {
    async fn run_prompt(
        &self,
        request: schema::PromptRequest,
        connection: ConnectionTo<Client>,
    ) -> Result<()> {
        let session_id = request.session_id.to_string();
        send_update(
            &connection,
            &session_id,
            schema::SessionUpdate::StateUpdate(schema::StateUpdate::Running(
                schema::RunningStateUpdate::new(),
            )),
        )?;

        if self.spec.huge_output {
            // 各通知は frame・滞留 byte の上限未満に割り、累積テキストだけで
            // 出力上限（1 MiB）を超える形にする（上限経路の切り分け）。
            for i in 0..3 {
                send_update(
                    &connection,
                    &session_id,
                    schema::SessionUpdate::AgentMessage(
                        schema::AgentMessage::new(schema::MessageId::new(format!("big-{i}")))
                            .content(vec!["a".repeat(512 * 1024).into()]),
                    ),
                )?;
            }
        }

        if let Some(bytes) = self.spec.late_giant_bytes {
            // クライアントが idle 待ちに入った後に、frame 上限未満・滞留
            // byte 上限超過の単発通知を送り、受信側の待機解除を確かめる。
            send_update(
                &connection,
                &session_id,
                schema::SessionUpdate::AgentMessage(
                    schema::AgentMessage::new(schema::MessageId::new("late-giant"))
                        .content(vec!["x".repeat(bytes).into()]),
                ),
            )?;
        }

        if let Some(tool) = self.spec.permission_tool {
            send_update(
                &connection,
                &session_id,
                schema::SessionUpdate::ToolCallUpdate(
                    schema::ToolCallUpdate::new(schema::ToolCallId::new("call-1")).name(tool),
                ),
            )?;
            send_update(
                &connection,
                &session_id,
                schema::SessionUpdate::StateUpdate(schema::StateUpdate::RequiresAction(
                    schema::RequiresActionStateUpdate::new(),
                )),
            )?;
            let response = connection
                .send_request(
                    schema::RequestPermissionRequest::new(
                        request.session_id.clone(),
                        format!("run {tool}?"),
                        self.spec
                            .option_kinds
                            .iter()
                            .map(option_for)
                            .collect::<Vec<_>>(),
                    )
                    .subject(schema::RequestPermissionSubject::from(
                        schema::ToolCallUpdate::new(schema::ToolCallId::new("call-1")).name(tool),
                    )),
                )
                .block_task()
                .await?;
            let outcome = match response.outcome {
                schema::RequestPermissionOutcome::Selected(selected) => {
                    format!("selected:{}", selected.option_id.0)
                }
                schema::RequestPermissionOutcome::Cancelled(_) => "cancelled".to_owned(),
                _ => "other".to_owned(),
            };
            {
                let mut state = self.state.lock().expect("double state poisoned");
                state.permission_outcome = Some(outcome.clone());
                state.permission_meta = response.meta.clone().map(serde_json::Value::Object);
            }
            send_update(
                &connection,
                &session_id,
                schema::SessionUpdate::AgentMessage(
                    schema::AgentMessage::new(schema::MessageId::new("agent-msg-1"))
                        .content(vec![outcome.into()]),
                ),
            )?;
        } else {
            send_update(
                &connection,
                &session_id,
                schema::SessionUpdate::AgentMessage(
                    schema::AgentMessage::new(schema::MessageId::new("agent-msg-1"))
                        .content(vec!["Echo: done".into()]),
                ),
            )?;
        }

        let cancelled = self.state.lock().expect("double state poisoned").cancelled;
        send_update(
            &connection,
            &session_id,
            schema::SessionUpdate::StateUpdate(schema::StateUpdate::Idle(
                schema::IdleStateUpdate::new().stop_reason(if cancelled {
                    schema::StopReason::Cancelled
                } else {
                    schema::StopReason::EndTurn
                }),
            )),
        )
    }
}

/// double を `tokio::io::duplex` 越しに serve する。戻り値はクライアント側の
/// transport と、serve 側の future（spawn しておくもの）。
fn spawn_double(
    spec: DoubleSpec,
) -> (
    tokio::io::DuplexStream,
    impl std::future::Future<Output = Result<()>>,
    Arc<Mutex<DoubleState>>,
) {
    let (client_io, agent_io) = tokio::io::duplex(8192);
    let (agent_read, agent_write) = tokio::io::split(agent_io);
    let agent_transport = ByteStreams::new(agent_write.compat_write(), agent_read.compat());

    let double = Double {
        spec,
        state: Arc::new(Mutex::new(DoubleState::default())),
    };
    let state = double.state.clone();

    let serve = Agent
        .v2()
        .name("acp-double")
        .on_receive_request(
            {
                let double = double.clone();
                async move |request: schema::InitializeRequest,
                            responder: Responder<schema::InitializeResponse>,
                            _c: ConnectionTo<Client>| {
                    {
                        let mut state = double.state.lock().expect("double state poisoned");
                        state.client_capabilities = Some(request.capabilities.clone());
                    }
                    let name = double.spec.agent_name.unwrap_or("acp-double");
                    responder.respond(
                        schema::InitializeResponse::new(
                            request.protocol_version,
                            schema::Implementation::new(name, "0.0.0"),
                        )
                        .capabilities(
                            schema::AgentCapabilities::new()
                                .session(schema::SessionCapabilities::new()),
                        ),
                    )
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let double = double.clone();
                async move |request: schema::NewSessionRequest,
                            responder: Responder<schema::NewSessionResponse>,
                            _c: ConnectionTo<Client>| {
                    {
                        let mut state = double.state.lock().expect("double state poisoned");
                        state.session_seq += 1;
                        state.new_session_channels = Some((
                            request.mcp_servers.len(),
                            request.additional_directories.len(),
                        ));
                    }
                    responder.respond(schema::NewSessionResponse::new(schema::SessionId::new(
                        DOUBLE_SESSION,
                    )))
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let double = double.clone();
                async move |request: schema::PromptRequest,
                            responder: Responder<schema::PromptResponse>,
                            connection: ConnectionTo<Client>| {
                    // 応答前に連投し、クライアント側の有界キューを溢れさせる。
                    if let Some(n) = double.spec.flood_updates {
                        for i in 0..n {
                            send_update(
                                &connection,
                                &request.session_id.to_string(),
                                schema::SessionUpdate::AgentMessage(
                                    schema::AgentMessage::new(schema::MessageId::new(format!(
                                        "flood-{i}"
                                    )))
                                    .content(vec!["x".into()]),
                                ),
                            )?;
                        }
                    }
                    if let Some(bytes) = double.spec.flood_bytes {
                        // 各通知の frame は上限未満に割り、滞留バイト数だけで
                        // 上限を超える形にする（frame 上限との切り分け）。
                        let mut sent = 0usize;
                        let mut seq = 0u64;
                        while sent < bytes {
                            let chunk = (512 * 1024).min(bytes - sent);
                            send_update(
                                &connection,
                                &request.session_id.to_string(),
                                schema::SessionUpdate::AgentMessage(
                                    schema::AgentMessage::new(schema::MessageId::new(format!(
                                        "flood-bytes-{seq}"
                                    )))
                                    .content(vec!["x".repeat(chunk).into()]),
                                ),
                            )?;
                            sent += chunk;
                            seq += 1;
                        }
                    }
                    if let Some(n) = double.spec.flood_tool_calls {
                        for i in 0..n {
                            send_update(
                                &connection,
                                &request.session_id.to_string(),
                                schema::SessionUpdate::ToolCallUpdate(
                                    schema::ToolCallUpdate::new(schema::ToolCallId::new(format!(
                                        "call-{i}"
                                    )))
                                    .name(format!("tool-{i}")),
                                ),
                            )?;
                        }
                    }
                    if let Some(bytes) = double.spec.giant_frame_bytes {
                        send_update(
                            &connection,
                            &request.session_id.to_string(),
                            schema::SessionUpdate::AgentMessage(
                                schema::AgentMessage::new(schema::MessageId::new("giant-frame"))
                                    .content(vec!["x".repeat(bytes).into()]),
                            ),
                        )?;
                    }
                    responder.respond(schema::PromptResponse::new(schema::MessageId::new(
                        "user-msg-1",
                    )))?;
                    connection.spawn({
                        let double = double.clone();
                        let connection = connection.clone();
                        async move { double.run_prompt(request, connection).await }
                    })
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async |_request: schema::CloseSessionRequest,
                   responder: Responder<schema::CloseSessionResponse>,
                   _c: ConnectionTo<Client>| {
                responder.respond(schema::CloseSessionResponse::new())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_notification(
            {
                let double = double.clone();
                async move |notification: schema::CancelSessionNotification,
                            _c: ConnectionTo<Client>| {
                    let _ = notification;
                    double
                        .state
                        .lock()
                        .expect("double state poisoned")
                        .cancelled = true;
                    Ok(())
                }
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .connect_to(agent_transport);

    (client_io, serve, state)
}

/// 試験用の設定一式。`post_message` だけを External として宣言する。
fn declarations() -> TierDeclarations {
    let mut declarations = TierDeclarations::new();
    declarations.declare("post_message", ActionTier::External);
    declarations
}

fn grants() -> GrantRegistry {
    let mut registry = GrantRegistry::new();
    registry.grant("acp-double", "acme/alice", "acp.session");
    registry
}

fn grant(registry: &GrantRegistry) -> pelway_gateway::acp::AgentGrant {
    registry
        .require("acp-double", "acme/alice", "acp.session")
        .expect("grant should exist")
}

/// 決定を記録しつつ返す PermissionDecider。
struct RecordingDecider {
    asks: Mutex<Vec<PermissionAsk>>,
    decision: PermissionDecision,
}

/// 完全な承認証跡（許可を出せる試験用の値）。
fn full_evidence() -> ApprovalEvidence {
    ApprovalEvidence::new(
        "member:test-approver",
        1_900_000_000_000, // 2030-03 頃の epoch ms（未到来）
        "sha256:test",
        "role:test",
        false,
    )
    .expect("test evidence should validate")
}

impl RecordingDecider {
    /// 指定の応答を完全な証跡付きで返す decider。AllowOnce は証跡必須の
    /// ため、証跡なしの許可を試したい場合は `decided` で
    /// `PermissionDecision::answered`（証跡なし）を渡す。
    fn answered(answer: PermissionAnswer) -> Self {
        Self::decided(PermissionDecision {
            answer,
            evidence: Some(full_evidence()),
        })
    }

    fn decided(decision: PermissionDecision) -> Self {
        Self {
            asks: Mutex::new(Vec::new()),
            decision,
        }
    }
}

impl PermissionDecider for RecordingDecider {
    fn decide(&self, ask: PermissionAsk) -> PermissionDecision {
        self.asks.lock().expect("decider poisoned").push(ask);
        self.decision.clone()
    }
}

fn audit_events(audit: &Mutex<Vec<AcpAuditEvent>>) -> Vec<String> {
    audit
        .lock()
        .expect("audit poisoned")
        .iter()
        .map(AcpAuditEvent::kind)
        .collect()
}

// 受け入れ条件: grant が登録されていなければ接続要求を拒否する（実 I/O 前の関門）。
#[test]
fn require_denies_without_grant() {
    let registry = GrantRegistry::new();
    let error = registry
        .require("codex-acp", "acme/alice", "acp.session")
        .expect_err("grant が無いのに許可された");
    assert!(matches!(error, AcpError::GrantDenied { .. }));
}

// 受け入れ条件: double との session/new → prompt → idle → close の一往復が
// 成立し、監査に各段が記録される。
#[tokio::test]
async fn roundtrip_echo_session() {
    let (transport, serve, _state) = spawn_double(DoubleSpec::default());
    let server = tokio::spawn(serve);

    let audit = Arc::new(Mutex::new(Vec::new()));
    let client = AcpClient::new(declarations(), audit.clone());
    let report = client
        .run_prompt(
            &grant(&grants()),
            IsolatedTransport::bind_in_process(transport, "acp-double"),
            &ExportEnvelope::new("レビューして"),
            Arc::new(RecordingDecider::answered(PermissionAnswer::RejectOnce)),
        )
        .await
        .expect("一往復に失敗");

    assert_eq!(report.stop_reason, Some(schema::StopReason::EndTurn));
    assert!(report.agent_text.contains("Echo: done"));
    let kinds = audit_events(&audit);
    for expected in [
        "connect_attempt",
        "session_opened",
        "prompt_sent",
        "session_closed",
    ] {
        assert!(
            kinds.contains(&expected.to_owned()),
            "{expected} が無い: {kinds:?}"
        );
    }
    server.abort();
}

// 受け入れ条件: 宣言済みの操作への許可要求は承認決定（once 系）で応え、
// requires_action を経て idle に戻る。
#[tokio::test]
async fn permission_allow_once_for_declared_tool() {
    let (transport, serve, state) = spawn_double(DoubleSpec {
        permission_tool: Some("post_message"),
        option_kinds: vec![
            schema::PermissionOptionKind::AllowAlways,
            schema::PermissionOptionKind::AllowOnce,
            schema::PermissionOptionKind::RejectOnce,
        ],
        ..DoubleSpec::default()
    });
    let server = tokio::spawn(serve);

    let audit = Arc::new(Mutex::new(Vec::new()));
    let decider = Arc::new(RecordingDecider::answered(PermissionAnswer::AllowOnce));
    let client = AcpClient::new(declarations(), audit.clone());
    let report = client
        .run_prompt(
            &grant(&grants()),
            IsolatedTransport::bind_in_process(transport, "acp-double"),
            &ExportEnvelope::new("post it"),
            decider.clone(),
        )
        .await
        .expect("一往復に失敗");

    // decider には宣言済みの操作名が渡る。
    let asks = decider.asks.lock().expect("decider poisoned");
    assert_eq!(asks.len(), 1);
    assert_eq!(asks[0].operation, "post_message");
    drop(asks);
    // double は allow_once の option_id を受け取った。
    assert_eq!(
        state
            .lock()
            .expect("double state poisoned")
            .permission_outcome
            .as_deref(),
        Some("selected:allow_once")
    );
    assert_eq!(report.stop_reason, Some(schema::StopReason::EndTurn));
    assert!(report.saw_requires_action);
    server.abort();
}

// 受け入れ条件: 段階が宣言されていない操作は承認カードを作らず止める
// （fail closed、ADR 0004）。decider は呼ばれない。
#[tokio::test]
async fn undeclared_operation_is_rejected_without_decider() {
    let (transport, serve, state) = spawn_double(DoubleSpec {
        permission_tool: Some("purge_cache"),
        option_kinds: vec![
            schema::PermissionOptionKind::AllowOnce,
            schema::PermissionOptionKind::RejectOnce,
        ],
        ..DoubleSpec::default()
    });
    let server = tokio::spawn(serve);

    let audit = Arc::new(Mutex::new(Vec::new()));
    let decider = Arc::new(RecordingDecider::answered(PermissionAnswer::AllowOnce));
    let client = AcpClient::new(declarations(), audit.clone());
    client
        .run_prompt(
            &grant(&grants()),
            IsolatedTransport::bind_in_process(transport, "acp-double"),
            &ExportEnvelope::new("purge it"),
            decider.clone(),
        )
        .await
        .expect("一往復に失敗");

    assert!(decider.asks.lock().expect("decider poisoned").is_empty());
    assert_eq!(
        state
            .lock()
            .expect("double state poisoned")
            .permission_outcome
            .as_deref(),
        Some("selected:reject_once")
    );
    let kinds = audit_events(&audit);
    assert!(
        kinds.contains(&"permission_denied_undeclared".to_owned()),
        "fail closed の監査が無い: {kinds:?}"
    );
    server.abort();
}

// 受け入れ条件: once 系 option が無い許可要求には承認を作らず cancelled で応え、
// allow_always/reject_always は選ばない（R1-Q4(i)）。
#[tokio::test]
async fn always_options_are_never_selected() {
    let (transport, serve, state) = spawn_double(DoubleSpec {
        permission_tool: Some("post_message"),
        option_kinds: vec![
            schema::PermissionOptionKind::AllowAlways,
            schema::PermissionOptionKind::RejectAlways,
        ],
        ..DoubleSpec::default()
    });
    let server = tokio::spawn(serve);

    let audit = Arc::new(Mutex::new(Vec::new()));
    let decider = Arc::new(RecordingDecider::answered(PermissionAnswer::AllowOnce));
    let client = AcpClient::new(declarations(), audit.clone());
    client
        .run_prompt(
            &grant(&grants()),
            IsolatedTransport::bind_in_process(transport, "acp-double"),
            &ExportEnvelope::new("post it"),
            decider,
        )
        .await
        .expect("一往復に失敗");

    assert_eq!(
        state
            .lock()
            .expect("double state poisoned")
            .permission_outcome
            .as_deref(),
        Some("cancelled")
    );
    server.abort();
}

// 受け入れ条件: 承認決定が Cancel のとき `session/cancel` を送り、
// stop_reason=cancelled で終わる。
#[tokio::test]
async fn cancel_decision_cancels_session() {
    let (transport, serve, _state) = spawn_double(DoubleSpec {
        permission_tool: Some("post_message"),
        option_kinds: vec![
            schema::PermissionOptionKind::AllowOnce,
            schema::PermissionOptionKind::RejectOnce,
        ],
        ..DoubleSpec::default()
    });
    let server = tokio::spawn(serve);

    let audit = Arc::new(Mutex::new(Vec::new()));
    let decider = Arc::new(RecordingDecider::answered(PermissionAnswer::Cancel));
    let client = AcpClient::new(declarations(), audit.clone());
    let report = client
        .run_prompt(
            &grant(&grants()),
            IsolatedTransport::bind_in_process(transport, "acp-double"),
            &ExportEnvelope::new("post it"),
            decider,
        )
        .await
        .expect("一往復に失敗");

    assert_eq!(report.stop_reason, Some(schema::StopReason::Cancelled));
    let kinds = audit_events(&audit);
    assert!(
        kinds.contains(&"cancel_sent".to_owned()),
        "cancel_sent が無い: {kinds:?}"
    );
    server.abort();
}

// 受け入れ条件: transport に結び付けられた連携 ID（launcher が保証する
// 値）が grant の連携と一致しなければ、接続自体を行わず拒否する
// （別連携宛の grant を別エージェントへ誤配送するのを防ぐ）。
#[tokio::test]
async fn identity_mismatch_refuses_before_session() {
    let (transport, serve, state) = spawn_double(DoubleSpec::default());
    let server = tokio::spawn(serve);

    let audit = Arc::new(Mutex::new(Vec::new()));
    let client = AcpClient::new(declarations(), audit.clone());
    let error = client
        .run_prompt(
            &grant(&grants()),
            IsolatedTransport::bind_in_process(transport, "other-integration"),
            &ExportEnvelope::new("レビューして"),
            Arc::new(RecordingDecider::answered(PermissionAnswer::RejectOnce)),
        )
        .await
        .expect_err("名が違うエージェントへの送信を許した");

    assert!(matches!(error, AcpError::IntegrationMismatch { .. }));
    let state = state.lock().expect("double state poisoned");
    assert_eq!(state.session_seq, 0, "session が開かれた");
    drop(state);
    let kinds = audit_events(&audit);
    assert!(
        kinds.contains(&"identity_rejected".to_owned()),
        "identity_rejected が無い: {kinds:?}"
    );
    server.abort();
}

// 受け入れ条件: クライアントは副作用経路を advertise しない（initialize の
// capability は空、session/new に MCP server も追加ディレクトリも無い）。
// permission は情報面であり、Pelway 仲介の副作用経路は存在しない。
#[tokio::test]
async fn client_advertises_no_side_effect_channels() {
    let (transport, serve, state) = spawn_double(DoubleSpec::default());
    let server = tokio::spawn(serve);

    let audit = Arc::new(Mutex::new(Vec::new()));
    let client = AcpClient::new(declarations(), audit.clone());
    client
        .run_prompt(
            &grant(&grants()),
            IsolatedTransport::bind_in_process(transport, "acp-double"),
            &ExportEnvelope::new("レビューして"),
            Arc::new(RecordingDecider::answered(PermissionAnswer::RejectOnce)),
        )
        .await
        .expect("一往復に失敗");

    let state = state.lock().expect("double state poisoned");
    let capabilities = state
        .client_capabilities
        .as_ref()
        .expect("initialize を受け取っていない");
    assert!(
        capabilities.auth.is_none() && capabilities.elicitation.is_none(),
        "client capability を advertise している: {capabilities:?}"
    );
    assert_eq!(
        state.new_session_channels,
        Some((0, 0)),
        "session/new に MCP server か追加ディレクトリがある"
    );
    server.abort();
}

// 受け入れ条件: エージェントが処理能力を超える量の update を送っても、
// キューは bounded で session は失敗する（メモリ枯渇の DoS 防止）。
#[tokio::test]
async fn flooding_updates_fail_the_session() {
    let (transport, serve, _state) = spawn_double(DoubleSpec {
        flood_updates: Some(400),
        ..DoubleSpec::default()
    });
    let server = tokio::spawn(serve);

    let audit = Arc::new(Mutex::new(Vec::new()));
    let client = AcpClient::new(declarations(), audit.clone());
    let error = client
        .run_prompt(
            &grant(&grants()),
            IsolatedTransport::bind_in_process(transport, "acp-double"),
            &ExportEnvelope::new("レビューして"),
            Arc::new(RecordingDecider::answered(PermissionAnswer::RejectOnce)),
        )
        .await
        .expect_err("連投するエージェントを許した");

    assert!(matches!(error, AcpError::UpdateOverflow), "{error:?}");
    server.abort();
}

// 受け入れ条件: 累積する agent 出力が上限（1 MiB）を超えたら session を
// 失敗させる（巨大出力によるメモリ枯渇の防止）。
#[tokio::test]
async fn oversized_output_fails_the_session() {
    let (transport, serve, _state) = spawn_double(DoubleSpec {
        huge_output: true,
        ..DoubleSpec::default()
    });
    let server = tokio::spawn(serve);

    let audit = Arc::new(Mutex::new(Vec::new()));
    let client = AcpClient::new(declarations(), audit.clone());
    let error = client
        .run_prompt(
            &grant(&grants()),
            IsolatedTransport::bind_in_process(transport, "acp-double"),
            &ExportEnvelope::new("レビューして"),
            Arc::new(RecordingDecider::answered(PermissionAnswer::RejectOnce)),
        )
        .await
        .expect_err("巨大な出力を許した");

    assert!(matches!(error, AcpError::OutputTooLarge), "{error:?}");
    server.abort();
}

// 受け入れ条件（決3・案ii）: decider が返した Pelway 承認モデルの証跡
// （指名承認者・期限・snapshot hash・開始資格・outcome_unknown）は
// permission 応答の meta.pelway に載ってエージェントへ届く。
#[tokio::test]
async fn approval_evidence_is_carried_in_response_meta() {
    let (transport, serve, state) = spawn_double(DoubleSpec {
        permission_tool: Some("post_message"),
        option_kinds: vec![
            schema::PermissionOptionKind::AllowOnce,
            schema::PermissionOptionKind::RejectOnce,
        ],
        ..DoubleSpec::default()
    });
    let server = tokio::spawn(serve);

    let audit = Arc::new(Mutex::new(Vec::new()));
    let client = AcpClient::new(declarations(), audit.clone());
    let decider = Arc::new(RecordingDecider::decided(PermissionDecision::approved(
        ApprovalEvidence::new(
            "member:hatai",
            1_900_000_000_000,
            "sha256:abc123",
            "role:representative",
            false,
        )
        .expect("test evidence should validate"),
    )));
    client
        .run_prompt(
            &grant(&grants()),
            IsolatedTransport::bind_in_process(transport, "acp-double"),
            &ExportEnvelope::new("レビューして"),
            decider,
        )
        .await
        .expect("ACP session failed");

    let state = state.lock().expect("double state poisoned");
    assert_eq!(
        state.permission_outcome.as_deref(),
        Some("selected:allow_once")
    );
    let meta = state
        .permission_meta
        .as_ref()
        .expect("permission response should carry meta");
    let pelway = &meta["pelway"];
    assert_eq!(pelway["decider"], "member:hatai");
    assert_eq!(pelway["expiresAt"], 1_900_000_000_000_u64);
    assert_eq!(pelway["snapshotHash"], "sha256:abc123");
    assert_eq!(pelway["eligibility"], "role:representative");
    drop(state);

    server.abort();
}

// 不変条件（決3・案ii）: 承認証跡なしの許可は選ばない。decider が証跡を
// 付けずに AllowOnce を返しても fail closed で reject 側に倒す。
#[tokio::test]
async fn allow_without_evidence_fails_closed() {
    let (transport, serve, state) = spawn_double(DoubleSpec {
        permission_tool: Some("post_message"),
        option_kinds: vec![
            schema::PermissionOptionKind::AllowOnce,
            schema::PermissionOptionKind::RejectOnce,
        ],
        ..DoubleSpec::default()
    });
    let server = tokio::spawn(serve);

    let audit = Arc::new(Mutex::new(Vec::new()));
    let client = AcpClient::new(declarations(), audit.clone());
    client
        .run_prompt(
            &grant(&grants()),
            IsolatedTransport::bind_in_process(transport, "acp-double"),
            &ExportEnvelope::new("レビューして"),
            Arc::new(RecordingDecider::decided(PermissionDecision::answered(
                PermissionAnswer::AllowOnce,
            ))),
        )
        .await
        .expect("ACP session failed");

    let state = state.lock().expect("double state poisoned");
    assert_eq!(
        state.permission_outcome.as_deref(),
        Some("selected:reject_once"),
        "証跡なしの許可を通した"
    );
    drop(state);
    let kinds = audit_events(&audit);
    assert!(
        kinds.contains(&"permission_denied_incomplete_evidence".to_owned()),
        "permission_denied_incomplete_evidence が無い: {kinds:?}"
    );
    server.abort();
}

// 不変条件（決3・案ii）: `outcome_unknown`（人の決定結果が不明）は絶対に
// 許可しない。証跡が揃っていても許可を選ばず fail closed で止める。
#[tokio::test]
async fn outcome_unknown_never_allows() {
    let (transport, serve, state) = spawn_double(DoubleSpec {
        permission_tool: Some("post_message"),
        option_kinds: vec![
            schema::PermissionOptionKind::AllowOnce,
            schema::PermissionOptionKind::RejectOnce,
        ],
        ..DoubleSpec::default()
    });
    let server = tokio::spawn(serve);

    let audit = Arc::new(Mutex::new(Vec::new()));
    let client = AcpClient::new(declarations(), audit.clone());
    client
        .run_prompt(
            &grant(&grants()),
            IsolatedTransport::bind_in_process(transport, "acp-double"),
            &ExportEnvelope::new("レビューして"),
            Arc::new(RecordingDecider::decided(PermissionDecision::approved(
                ApprovalEvidence::new(
                    "member:test-approver",
                    1_900_000_000_000,
                    "sha256:test",
                    "role:test",
                    true,
                )
                .expect("test evidence should validate"),
            ))),
        )
        .await
        .expect("ACP session failed");

    let state = state.lock().expect("double state poisoned");
    assert_eq!(
        state.permission_outcome.as_deref(),
        Some("selected:reject_once"),
        "outcome_unknown での許可を通した"
    );
    drop(state);
    let kinds = audit_events(&audit);
    assert!(
        kinds.contains(&"permission_denied_incomplete_evidence".to_owned()),
        "permission_denied_incomplete_evidence が無い: {kinds:?}"
    );
    server.abort();
}

// 不変条件（決3・案ii）: 構築時は有効だった証跡が許可時点で失効して
// いれば `AllowOnce` を選ばず fail closed で止める（decider が証跡を
// 保持・再利用するケース）。
#[tokio::test]
async fn expired_evidence_fails_closed() {
    let (transport, serve, state) = spawn_double(DoubleSpec {
        permission_tool: Some("post_message"),
        option_kinds: vec![
            schema::PermissionOptionKind::AllowOnce,
            schema::PermissionOptionKind::RejectOnce,
        ],
        ..DoubleSpec::default()
    });
    let server = tokio::spawn(serve);

    let expires_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock before epoch")
        .as_millis() as u64
        + 150;
    let evidence = ApprovalEvidence::new(
        "member:test-approver",
        expires_at,
        "sha256:test",
        "role:test",
        false,
    )
    .expect("test evidence should validate at construction");
    // 許可時点には失効しているよう、期限が過ぎるのを待つ。
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let audit = Arc::new(Mutex::new(Vec::new()));
    let client = AcpClient::new(declarations(), audit.clone());
    client
        .run_prompt(
            &grant(&grants()),
            IsolatedTransport::bind_in_process(transport, "acp-double"),
            &ExportEnvelope::new("レビューして"),
            Arc::new(RecordingDecider::decided(PermissionDecision::approved(
                evidence,
            ))),
        )
        .await
        .expect("ACP session failed");

    let state = state.lock().expect("double state poisoned");
    assert_eq!(
        state.permission_outcome.as_deref(),
        Some("selected:reject_once"),
        "失効済みの証跡での許可を通した"
    );
    drop(state);
    let kinds = audit_events(&audit);
    assert!(
        kinds.contains(&"permission_denied_incomplete_evidence".to_owned()),
        "permission_denied_incomplete_evidence が無い: {kinds:?}"
    );
    server.abort();
}

// 受け入れ条件: 異なる toolCallId の更新を送り続ける agent に対し、
// toolCallId → 操作名の相関表は有界で、溢れたら session は失敗する
// （相関表の無制限蓄積によるメモリ枯渇の防止）。
#[tokio::test]
async fn flooding_tool_calls_fail_the_session() {
    let (transport, serve, _state) = spawn_double(DoubleSpec {
        flood_tool_calls: Some(600),
        ..DoubleSpec::default()
    });
    let server = tokio::spawn(serve);

    let audit = Arc::new(Mutex::new(Vec::new()));
    let client = AcpClient::new(declarations(), audit.clone());
    let error = client
        .run_prompt(
            &grant(&grants()),
            IsolatedTransport::bind_in_process(transport, "acp-double"),
            &ExportEnvelope::new("レビューして"),
            Arc::new(RecordingDecider::answered(PermissionAnswer::RejectOnce)),
        )
        .await
        .expect_err("相関表を溢れさせるエージェントを許した");

    assert!(matches!(error, AcpError::UpdateOverflow), "{error:?}");
    server.abort();
}

// 不変条件: 操作名を持たない（空の）許可要求は decider を呼ばず
// fail closed で止める。
#[tokio::test]
async fn empty_operation_name_fails_closed() {
    let (transport, serve, state) = spawn_double(DoubleSpec {
        permission_tool: Some(""),
        option_kinds: vec![
            schema::PermissionOptionKind::AllowOnce,
            schema::PermissionOptionKind::RejectOnce,
        ],
        ..DoubleSpec::default()
    });
    let server = tokio::spawn(serve);

    let audit = Arc::new(Mutex::new(Vec::new()));
    let client = AcpClient::new(declarations(), audit.clone());
    let decider = Arc::new(RecordingDecider::answered(PermissionAnswer::AllowOnce));
    client
        .run_prompt(
            &grant(&grants()),
            IsolatedTransport::bind_in_process(transport, "acp-double"),
            &ExportEnvelope::new("レビューして"),
            decider.clone(),
        )
        .await
        .expect("ACP session failed");

    let state = state.lock().expect("double state poisoned");
    assert_eq!(
        state.permission_outcome.as_deref(),
        Some("selected:reject_once"),
        "操作名なしの許可要求を decider に回した"
    );
    drop(state);
    assert!(
        decider.asks.lock().expect("decider poisoned").is_empty(),
        "操作名なしの要求で decider を呼んだ"
    );
    let kinds = audit_events(&audit);
    assert!(
        kinds.contains(&"permission_denied_undeclared".to_owned()),
        "permission_denied_undeclared が無い: {kinds:?}"
    );
    server.abort();
}

// 受け入れ条件: 通知の合計バイト数が上限を超えたら session を失敗させ、
// agent に cancel を送って session を止める。
#[tokio::test]
async fn giant_updates_close_the_session() {
    let (transport, serve, state) = spawn_double(DoubleSpec {
        flood_bytes: Some(5 * 1024 * 1024),
        ..DoubleSpec::default()
    });
    let server = tokio::spawn(serve);

    let audit = Arc::new(Mutex::new(Vec::new()));
    let client = AcpClient::new(declarations(), audit.clone());
    let error = client
        .run_prompt(
            &grant(&grants()),
            IsolatedTransport::bind_in_process(transport, "acp-double"),
            &ExportEnvelope::new("レビューして"),
            Arc::new(RecordingDecider::answered(PermissionAnswer::RejectOnce)),
        )
        .await
        .expect_err("巨大な通知を許した");

    assert!(matches!(error, AcpError::UpdateOverflow), "{error:?}");
    let kinds = audit_events(&audit);
    assert!(
        kinds.contains(&"cancel_sent".to_owned()),
        "cancel_sent が無い: {kinds:?}"
    );
    drop(state);
    server.abort();
}

// 受け入れ条件: 滞留 byte 上限を超える単発の通知（frame 上限未満）が
// 待機中に届いても、受信側は起こされて session を失敗させる。
#[tokio::test]
async fn late_giant_wakes_waiting_receiver() {
    let (transport, serve, _state) = spawn_double(DoubleSpec {
        late_giant_bytes: Some(3 * 1024 * 1024),
        ..DoubleSpec::default()
    });
    let server = tokio::spawn(serve);

    let audit = Arc::new(Mutex::new(Vec::new()));
    let client = AcpClient::new(declarations(), audit.clone());
    let error = client
        .run_prompt(
            &grant(&grants()),
            IsolatedTransport::bind_in_process(transport, "acp-double"),
            &ExportEnvelope::new("レビューして"),
            Arc::new(RecordingDecider::answered(PermissionAnswer::RejectOnce)),
        )
        .await
        .expect_err("待機中の巨大な通知を許した");

    assert!(matches!(error, AcpError::UpdateOverflow), "{error:?}");
    server.abort();
}

// 受け入れ条件: 1 frame（改行区切りの 1 message）が上限を超えたら SDK
// のデコード前に接続自体が失敗する（解析メモリの枯渇防止）。
#[tokio::test]
async fn giant_frame_fails_before_decode() {
    let (transport, serve, _state) = spawn_double(DoubleSpec {
        giant_frame_bytes: Some(5 * 1024 * 1024),
        ..DoubleSpec::default()
    });
    let server = tokio::spawn(serve);

    let audit = Arc::new(Mutex::new(Vec::new()));
    let client = AcpClient::new(declarations(), audit.clone());
    let error = client
        .run_prompt(
            &grant(&grants()),
            IsolatedTransport::bind_in_process(transport, "acp-double"),
            &ExportEnvelope::new("レビューして"),
            Arc::new(RecordingDecider::answered(PermissionAnswer::RejectOnce)),
        )
        .await
        .expect_err("巨大な frame を許した");

    assert!(
        matches!(error, AcpError::Protocol(_) | AcpError::Disconnected),
        "{error:?}"
    );
    server.abort();
}
