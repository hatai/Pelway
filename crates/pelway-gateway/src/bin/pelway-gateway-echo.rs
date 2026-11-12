//! #96 の検証用 echo MCP サーバー（stdio）。テスト（tests/stdio_child.rs）が子
//! プロセスとして起動する。製品機能ではない。`echo` ツールを1つ持ち、
//! readOnlyHint=true を申告して `echo:<name>` を返す。
//!
//! rmcp の `Tool`・`ToolAnnotations` は `#[non_exhaustive]` のため、構文リテラル
//! ではなく `Tool::new(..).with_annotations(ToolAnnotations::from_raw(..))` で
//! 作る。

use std::sync::Arc;

use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ErrorData,
    Implementation, JsonObject, ListToolsResult, PaginatedRequestParams, ServerCapabilities,
    ServerConfig, Tool, ToolAnnotations,
};
use rmcp::service::{MaybeSendFuture, RequestContext};
use rmcp::transport::stdio;
use rmcp::{RoleServer, ServerHandler, ServiceExt};

#[derive(Clone, Default)]
struct EchoServer;

impl ServerHandler for EchoServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("pelway-gateway-echo", "0.0.0"))
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, ErrorData>> + MaybeSendFuture + '_ {
        let tool = Tool::new("echo", "ping を返す", Arc::new(JsonObject::new())).with_annotations(
            ToolAnnotations::from_raw(None, Some(true), Some(false), None, None),
        );
        std::future::ready(Ok(ListToolsResult::with_all_items(vec![tool])))
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResponse, ErrorData>> + MaybeSendFuture + '_ {
        let text = format!("echo:{}", request.name);
        std::future::ready(Ok(CallToolResponse::Complete(CallToolResult::success(
            vec![ContentBlock::text(text)],
        ))))
    }
}

// stdio の相手は1本なので current_thread で十分（rt-multi-thread は入れない）。
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let service = EchoServer
        .serve(stdio())
        .await
        .expect("echo サーバーが stdio で起動する");
    service.waiting().await.expect("echo サーバーが待機する");
}
