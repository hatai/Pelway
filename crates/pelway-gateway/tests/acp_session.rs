//! ACP v2 による外部エージェント接続の Client adapter（`acp` feature）の試験
//! （#133）。in-process の Agent double（公式 rust-sdk の `Agent.v2()`）との
//! session/new → prompt → requires_action → 承認 → idle → close の一往復と、
//! grant 判定・段階の fail closed・once 系限定・cancel・監査記録を確かめる。
//! 実エージェント（codex-acp 等）との接続は対象外（質問票 R1-Q2(b)・R1-Q7(a)）。

#![cfg(feature = "acp")]

use std::sync::{Arc, Mutex};

use agent_client_protocol::v2::{ConnectionTo, Responder, schema};
use agent_client_protocol::{Agent, ByteStreams, Client, ConnectTo, Result};
use pelway_gateway::acp::{
    AcpAuditEvent, AcpClient, AcpError, ExportEnvelope, GrantRegistry, PermissionAnswer,
    PermissionAsk, PermissionDecider,
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
            self.state
                .lock()
                .expect("double state poisoned")
                .permission_outcome = Some(outcome.clone());
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
    impl ConnectTo<Client>,
    impl std::future::Future<Output = Result<()>>,
    Arc<Mutex<DoubleState>>,
) {
    let (client_io, agent_io) = tokio::io::duplex(8192);
    let (client_read, client_write) = tokio::io::split(client_io);
    let (agent_read, agent_write) = tokio::io::split(agent_io);
    let client_transport = ByteStreams::new(client_write.compat_write(), client_read.compat());
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
            async |request: schema::InitializeRequest,
                   responder: Responder<schema::InitializeResponse>,
                   _c: ConnectionTo<Client>| {
                responder.respond(
                    schema::InitializeResponse::new(
                        request.protocol_version,
                        schema::Implementation::new("acp-double", "0.0.0"),
                    )
                    .capabilities(
                        schema::AgentCapabilities::new()
                            .session(schema::SessionCapabilities::new()),
                    ),
                )
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let double = double.clone();
                async move |_request: schema::NewSessionRequest,
                            responder: Responder<schema::NewSessionResponse>,
                            _c: ConnectionTo<Client>| {
                    double
                        .state
                        .lock()
                        .expect("double state poisoned")
                        .session_seq += 1;
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

    (client_transport, serve, state)
}

/// 試験用の設定一式。`post_message` だけを External として宣言する。
fn declarations() -> TierDeclarations {
    let mut declarations = TierDeclarations::new();
    declarations.declare("post_message", ActionTier::External);
    declarations
}

fn grants() -> GrantRegistry {
    let mut registry = GrantRegistry::new();
    registry.grant("codex-acp", "acme/alice", "acp.session");
    registry
}

fn grant(registry: &GrantRegistry) -> pelway_gateway::acp::AgentGrant {
    registry
        .require("codex-acp", "acme/alice", "acp.session")
        .expect("grant should exist")
}

/// 決定を記録しつつ返す PermissionDecider。
struct RecordingDecider {
    asks: Mutex<Vec<PermissionAsk>>,
    answer: PermissionAnswer,
}

impl RecordingDecider {
    fn answered(answer: PermissionAnswer) -> Self {
        Self {
            asks: Mutex::new(Vec::new()),
            answer,
        }
    }
}

impl PermissionDecider for RecordingDecider {
    fn decide(&self, ask: PermissionAsk) -> PermissionAnswer {
        self.asks.lock().expect("decider poisoned").push(ask);
        self.answer
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
            transport,
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
    });
    let server = tokio::spawn(serve);

    let audit = Arc::new(Mutex::new(Vec::new()));
    let decider = Arc::new(RecordingDecider::answered(PermissionAnswer::AllowOnce));
    let client = AcpClient::new(declarations(), audit.clone());
    let report = client
        .run_prompt(
            &grant(&grants()),
            transport,
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
    });
    let server = tokio::spawn(serve);

    let audit = Arc::new(Mutex::new(Vec::new()));
    let decider = Arc::new(RecordingDecider::answered(PermissionAnswer::AllowOnce));
    let client = AcpClient::new(declarations(), audit.clone());
    client
        .run_prompt(
            &grant(&grants()),
            transport,
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
    });
    let server = tokio::spawn(serve);

    let audit = Arc::new(Mutex::new(Vec::new()));
    let decider = Arc::new(RecordingDecider::answered(PermissionAnswer::AllowOnce));
    let client = AcpClient::new(declarations(), audit.clone());
    client
        .run_prompt(
            &grant(&grants()),
            transport,
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
    });
    let server = tokio::spawn(serve);

    let audit = Arc::new(Mutex::new(Vec::new()));
    let decider = Arc::new(RecordingDecider::answered(PermissionAnswer::Cancel));
    let client = AcpClient::new(declarations(), audit.clone());
    let report = client
        .run_prompt(
            &grant(&grants()),
            transport,
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
