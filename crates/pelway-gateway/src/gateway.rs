//! 連携 gateway 本体。MCP 接続（[`McpClient`]）と段階の宣言
//! （[`TierDeclarations`]）を組み合わせ、ADR 0004 の関門を守る。

use rmcp::model::JsonObject;

use crate::mcp::{GatewayError, McpClient, ToolCallOutcome};
use crate::tier::TierDeclarations;
use crate::tool::{ToolListing, build_catalog};

/// 段階の宣言を結び付けた MCP 接続。ツール一覧の提供と、宣言済みツールの呼び出し
/// だけを行う。
pub struct Gateway {
    client: McpClient,
    declarations: TierDeclarations,
}

impl Gateway {
    /// stdio 子プロセスの MCP サーバーに、段階の宣言と一緒に接続する。失敗の内容は
    /// [`GatewayError`] の各変数の説明のとおり。
    pub async fn connect_stdio(
        declarations: TierDeclarations,
        program: &str,
        args: &[&str],
    ) -> Result<Self, GatewayError> {
        let client = McpClient::connect_stdio(program, args).await?;
        Ok(Self {
            client,
            declarations,
        })
    }

    /// 接続済みのクライアントから作る。in-process の検証用。
    #[must_use]
    pub fn from_client(client: McpClient, declarations: TierDeclarations) -> Self {
        Self {
            client,
            declarations,
        }
    }

    /// ツール一覧を管理者の画面向けに組む。annotations は手がかりとして載り、段階は
    /// 宣言から決まる。失敗の内容は [`GatewayError`] の各変数の説明のとおり。
    pub async fn tool_catalog(&self) -> Result<Vec<ToolListing>, GatewayError> {
        let raw = self.client.list_tools().await?;
        Ok(build_catalog(&raw, &self.declarations))
    }

    /// ツールの呼び出し。段階が宣言されていないツールは、annotations がどんな申告
    /// でも（`readOnlyHint=true` でも）[`GatewayError::UndeclaredTool`] で止まる
    /// （fail closed、ADR 0004）。それ以外の失敗の内容は [`GatewayError`] の各変数の
    /// 説明のとおり。
    pub async fn call_tool(
        &self,
        tool: &str,
        arguments: JsonObject,
    ) -> Result<ToolCallOutcome, GatewayError> {
        // 段階が宣言されていない操作は取り消せない操作として扱い、止める（ADR 0004）。
        // annotations はこの判定に入れない。
        if !self.declarations.is_declared(tool) {
            return Err(GatewayError::UndeclaredTool {
                tool: tool.to_owned(),
            });
        }
        self.client.call_tool(tool, arguments).await
    }
}
