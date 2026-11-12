//! gateway の MCP 層のエッジ経路（#159）。`GatewayError` の Display 全文と、
//! tools/call の結果にテキスト以外のブロックが混じったときの絞り込みを確かめる。

use pelway_gateway::mcp::{GatewayError, McpClient};
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ErrorData,
    Implementation, JsonObject, ListToolsResult, PaginatedRequestParams, ServerCapabilities,
    ServerConfig, Tool,
};
use rmcp::service::{MaybeSendFuture, RequestContext};
use rmcp::{RoleServer, ServerHandler, ServiceExt};

// 各バリアントは中身の詳細を含む日本語メッセージを返す。
#[test]
fn gateway_error_display_covers_all_variants() {
    let cases: Vec<(GatewayError, &str)> = vec![
        (GatewayError::Spawn("spawn-detail".into()), "spawn-detail"),
        (
            GatewayError::Initialize("init-detail".into()),
            "init-detail",
        ),
        (GatewayError::ListTools("list-detail".into()), "list-detail"),
        (GatewayError::CallTool("call-detail".into()), "call-detail"),
        (
            GatewayError::UndeclaredTool {
                tool: "purge_cache".into(),
            },
            "purge_cache",
        ),
    ];
    for (error, detail) in cases {
        let text = error.to_string();
        assert!(text.contains(detail), "{text}");
        assert!(!text.is_empty());
    }
}

/// テキストと画像が混じった応答を返すテスト用サーバー。
#[derive(Clone, Default)]
struct MixedServer;

impl ServerHandler for MixedServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("pelway-mixed-server", "0.0.0"))
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, ErrorData>> + MaybeSendFuture + '_ {
        let tool = Tool::new(
            "mixed_content".to_owned(),
            "テキストと画像を返す".to_owned(),
            std::sync::Arc::new(JsonObject::new()),
        );
        std::future::ready(Ok(ListToolsResult::with_all_items(vec![tool])))
    }

    fn call_tool(
        &self,
        _request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResponse, ErrorData>> + MaybeSendFuture + '_ {
        std::future::ready(Ok(CallToolResponse::Complete(CallToolResult::success(
            vec![
                ContentBlock::text("first text".to_owned()),
                ContentBlock::image("QUJD".to_owned(), "image/png".to_owned()),
                ContentBlock::text("second text".to_owned()),
            ],
        ))))
    }
}

async fn client() -> McpClient {
    let (server_transport, client_transport) = tokio::io::duplex(4096);
    tokio::spawn(async move {
        let _ = MixedServer
            .serve(server_transport)
            .await
            .expect("in-process サーバーが起動する")
            .waiting()
            .await;
    });
    let service = ().serve(client_transport).await.expect("初期化できる");
    McpClient::from_service(service)
}

// tools/call の結果にテキスト以外（画像など）のブロックが混じっていても、
// gateway はテキストだけを取り出して返す（is_error は既定で false）。
#[tokio::test]
async fn call_tool_keeps_only_text_blocks() {
    let client = client().await;
    let outcome = client
        .call_tool("mixed_content", JsonObject::new())
        .await
        .expect("呼び出せる");
    assert!(!outcome.is_error);
    assert_eq!(
        outcome.text,
        vec!["first text".to_owned(), "second text".to_owned()]
    );
}
