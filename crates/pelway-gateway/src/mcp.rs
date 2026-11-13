//! rmcp による MCP サーバーへの接続（#96 の検証対象）。
//!
//! 接続方式は stdio 子プロセスだけ（02 の「本体の既定 tool は国内ローカル stdio のみ
//! 許可」案）。テストでは `tokio::io::duplex` の in-process サーバーに対しても使える
//! よう、接続済みサービスから作る入口も公開している。

use rmcp::model::{CallToolRequestParams, JsonObject};
use rmcp::service::RunningService;
use rmcp::transport::child_process::TokioChildProcess;
use rmcp::{RoleClient, ServiceExt};

use crate::annotations::AnnotationHints;
use crate::tool::RawTool;

/// gateway の失敗。rmcp の失敗をそのまま外に漏らさないための詰め替え。
#[derive(Debug)]
pub enum GatewayError {
    /// 子プロセスの起動に失敗した。
    Spawn(String),
    /// MCP の初期化（ハンドシェイク）に失敗した。
    Initialize(String),
    /// `tools/list` が失敗した。
    ListTools(String),
    /// `tools/call` が失敗した。
    CallTool(String),
    /// 段階が宣言されていないツールを呼び出そうとした（fail closed、ADR 0004）。
    UndeclaredTool {
        /// 呼び出そうとしたツール名。
        tool: String,
    },
}

impl std::fmt::Display for GatewayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawn(m) => write!(f, "MCP サーバーの子プロセスを起動できない: {m}"),
            Self::Initialize(m) => write!(f, "MCP サーバーとの初期化に失敗した: {m}"),
            Self::ListTools(m) => write!(f, "tools/list が失敗した: {m}"),
            Self::CallTool(m) => write!(f, "tools/call が失敗した: {m}"),
            Self::UndeclaredTool { tool } => write!(
                f,
                "段階が宣言されていないため呼び出しを止めた（取り消せない操作として扱う）: {tool}"
            ),
        }
    }
}

impl std::error::Error for GatewayError {}

/// `tools/call` の結果のうち、gateway が外に出す部分。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallOutcome {
    /// ツールが返したテキスト内容（テキストブロックだけを抜き出したもの）。
    pub text: Vec<String>,
    /// ツール自身がエラーを申告したか（`isError`）。
    pub is_error: bool,
}

/// rmcp のクライアント接続の詰め替え。`tools/list` と `tools/call` だけを扱う。
/// 失敗の内容は [`GatewayError`] の各変数の説明のとおり。
pub struct McpClient {
    service: RunningService<RoleClient, ()>,
}

impl McpClient {
    /// stdio 子プロセスの MCP サーバーに接続する。本体の既定の接続方式。
    pub async fn connect_stdio(program: &str, args: &[&str]) -> Result<Self, GatewayError> {
        let mut command = tokio::process::Command::new(program);
        command.args(args);
        let transport = TokioChildProcess::new(command)
            .map_err(|error| GatewayError::Spawn(error.to_string()))?;
        let service = ()
            .serve(transport)
            .await
            .map_err(|error| GatewayError::Initialize(error.to_string()))?;
        Ok(Self { service })
    }

    /// 接続済みの rmcp サービスから作る。in-process の検証用（テストが
    /// `tokio::io::duplex` の相手側を渡して作った接続を受け取る）。
    #[must_use]
    pub fn from_service(service: RunningService<RoleClient, ()>) -> Self {
        Self { service }
    }

    /// `tools/list` を送り、[`RawTool`] の一覧に詰め替える。annotations は手がかり
    /// としてそのまま入る。
    pub async fn list_tools(&self) -> Result<Vec<RawTool>, GatewayError> {
        let tools = self
            .service
            .list_all_tools()
            .await
            .map_err(|error| GatewayError::ListTools(error.to_string()))?;
        Ok(tools
            .iter()
            .map(|tool| RawTool {
                name: tool.name.to_string(),
                description: tool.description.as_deref().map(str::to_owned),
                annotations: tool
                    .annotations
                    .as_ref()
                    .map(AnnotationHints::from)
                    .unwrap_or_else(AnnotationHints::absent),
            })
            .collect())
    }

    /// `tools/call` を送る。段階の宣言（fail closed）は [`crate::gateway::Gateway`]
    /// 側で担うため、この関数はツール名の検査をしない。
    pub async fn call_tool(
        &self,
        tool: &str,
        arguments: JsonObject,
    ) -> Result<ToolCallOutcome, GatewayError> {
        let params = CallToolRequestParams::new(tool.to_owned()).with_arguments(arguments);
        let result = self
            .service
            .call_tool(params)
            .await
            .map_err(|error| GatewayError::CallTool(error.to_string()))?;
        let text = result
            .content
            .iter()
            .filter_map(|block| match block {
                rmcp::model::ContentBlock::Text(text) => Some(text.text.clone()),
                _ => None,
            })
            .collect();
        Ok(ToolCallOutcome {
            text,
            is_error: result.is_error.unwrap_or(false),
        })
    }
}
