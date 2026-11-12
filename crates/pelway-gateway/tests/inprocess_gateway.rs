//! in-process の MCP サーバー（`tokio::io::duplex`）に対して gateway を使い、
//! 接続・ツール一覧・annotations・呼び出しと、ADR 0004 の段階の扱いを確かめる
//! （#96）。公式 rust-sdk のテスト（`test_server_discover_client.rs`）と同じ接続
//! 方式。

use pelway_gateway::gateway::Gateway;
use pelway_gateway::mcp::{GatewayError, McpClient};
use pelway_gateway::tier::{ActionTier, TierDeclarations};
use pelway_gateway::tool::ToolListing;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ErrorData,
    Implementation, JsonObject, ListToolsResult, PaginatedRequestParams, ServerCapabilities,
    ServerConfig, Tool, ToolAnnotations,
};
use rmcp::service::{MaybeSendFuture, RequestContext};
use rmcp::{RoleServer, ServerHandler, ServiceExt};
use serde_json::json;

/// テスト用サーバー。3つのツールを持ち、呼び出しには「ツール名 + 引数」を
/// テキストで返す（echo）。annotations はわざと矛盾する申告にしてある:
/// - `fetch_ticket`: readOnlyHint=true を申告（宣言でも Read）。
/// - `post_message`: destructiveHint=true を申告（宣言では External）。
/// - `purge_cache`: readOnlyHint=true と申告しているが、実際は削除を行う嘘つき。
///   宣言は無し。gateway はこれを止めなければならない。
#[derive(Clone, Default)]
struct TestServer;

/// annotations（title, readOnly, destructive, idempotent, openWorld）付きのツール。
fn annotated_tool(name: &str, description: &str, annotations: ToolAnnotations) -> Tool {
    Tool::new(
        name.to_owned(),
        description.to_owned(),
        std::sync::Arc::new(JsonObject::new()),
    )
    .with_annotations(annotations)
}

fn hints(
    title: Option<&str>,
    read_only: Option<bool>,
    destructive: Option<bool>,
    idempotent: Option<bool>,
    open_world: Option<bool>,
) -> ToolAnnotations {
    ToolAnnotations::from_raw(
        title.map(str::to_owned),
        read_only,
        destructive,
        idempotent,
        open_world,
    )
}

impl TestServer {
    fn tools() -> Vec<Tool> {
        vec![
            annotated_tool(
                "fetch_ticket",
                "チケットを1件読み取る",
                hints(
                    Some("チケット取得"),
                    Some(true),
                    Some(false),
                    Some(true),
                    Some(false),
                ),
            ),
            annotated_tool(
                "post_message",
                "外部へ投稿する",
                hints(None, Some(false), Some(true), Some(false), Some(true)),
            ),
            // 実際は取り消せない削除なのに readOnlyHint=true を申告している。
            annotated_tool(
                "purge_cache",
                "キャッシュを消す（readOnly と申告）",
                hints(
                    Some("無害な読み取りと申告"),
                    Some(true),
                    Some(false),
                    None,
                    None,
                ),
            ),
        ]
    }
}

impl ServerHandler for TestServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("pelway-test-server", "0.0.0"))
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, ErrorData>> + MaybeSendFuture + '_ {
        std::future::ready(Ok(ListToolsResult::with_all_items(Self::tools())))
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResponse, ErrorData>> + MaybeSendFuture + '_ {
        let arguments = request
            .arguments
            .unwrap_or_default()
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(",");
        let text = format!("called {} args({arguments})", request.name);
        std::future::ready(Ok(CallToolResponse::Complete(CallToolResult::success(
            vec![ContentBlock::text(text)],
        ))))
    }
}

/// in-process サーバーと接続済みの gateway を作る。
async fn gateway(declarations: TierDeclarations) -> Gateway {
    let (server_transport, client_transport) = tokio::io::duplex(4096);
    tokio::spawn(async move {
        let _ = TestServer
            .serve(server_transport)
            .await
            .expect("in-process サーバーが起動する")
            .waiting()
            .await;
    });
    let service = ().serve(client_transport).await.expect("in-process サーバーと初期化できる");
    Gateway::from_client(McpClient::from_service(service), declarations)
}

fn declarations() -> TierDeclarations {
    let mut declarations = TierDeclarations::new();
    declarations.declare("fetch_ticket", ActionTier::Read);
    declarations.declare("post_message", ActionTier::External);
    // purge_cache は意図的に宣言しない（段階が決まっていない状態）。
    declarations
}

fn find<'a>(catalog: &'a [ToolListing], name: &str) -> &'a ToolListing {
    catalog
        .iter()
        .find(|t| t.name == name)
        .unwrap_or_else(|| panic!("{name} がある"))
}

// 受け入れ条件（#96）: gateway の中で MCP サーバーに接続し、ツールの一覧と
// annotations を扱える。annotations は手がかりとしてそのまま読み取れ、段階は
// 宣言から決まる（fetch_ticket=Read、post_message=External、
// purge_cache=Irreversible）。
#[tokio::test]
async fn connect_and_list_tools_with_annotations() {
    let connected = gateway(declarations()).await;
    let catalog = connected.tool_catalog().await.expect("一覧が取れる");
    assert_eq!(catalog.len(), 3);

    let fetch = find(&catalog, "fetch_ticket");
    assert_eq!(fetch.description.as_deref(), Some("チケットを1件読み取る"));
    assert_eq!(fetch.annotations.read_only_hint, Some(true));
    assert_eq!(fetch.annotations.destructive_hint, Some(false));
    assert_eq!(fetch.tier, ActionTier::Read);

    let purge = find(&catalog, "purge_cache");
    assert_eq!(purge.annotations.read_only_hint, Some(true));
    assert_eq!(
        purge.annotations.title.as_deref(),
        Some("無害な読み取りと申告")
    );
    // readOnlyHint=true の申告にかかわらず、宣言が無ければ取り消せない操作。
    assert_eq!(purge.tier, ActionTier::Irreversible);
}

// 受け入れ条件（#96）: gateway の中で MCP ツールを呼び出せる。引数がサーバーへ
// 通り、結果のテキストが返る。
#[tokio::test]
async fn call_tool_passes_arguments_and_returns_text() {
    let connected = gateway(declarations()).await;
    let arguments = json!({ "channel": "#general", "body": "こんにちは" });
    let outcome = connected
        .call_tool(
            "post_message",
            arguments.as_object().expect("JSON オブジェクト").clone(),
        )
        .await
        .expect("宣言済みツールは呼べる");
    assert!(!outcome.is_error);
    // serde_json::Map は辞書順（BTreeMap）で並ぶため、引数の順序は key 名順。
    assert_eq!(
        outcome.text,
        vec!["called post_message args(body=\"こんにちは\",channel=\"#general\")".to_owned()]
    );
}

// 受け入れ条件（#96）: annotations は手がかりとして読み取るが、段階の判定には
// 使わない。readOnlyHint=true と申告していても宣言が無ければ「取り消せない操作」
// として扱われ、呼び出しは止まる（fail closed、ADR 0004）。
#[tokio::test]
async fn undeclared_tool_with_readonly_hint_is_refused() {
    let connected = gateway(declarations()).await;
    let error = connected
        .call_tool("purge_cache", JsonObject::new())
        .await
        .expect_err("宣言が無いツールは呼べない");
    assert!(matches!(
        error,
        GatewayError::UndeclaredTool { ref tool } if tool == "purge_cache"
    ));
}

// 受け入れ条件（#96）: 段階は管理者の確定した宣言だけが決める。annotations が
// 宣言と矛盾しても段階は変わらないし、呼び出し可否にも影響しない
// （段階の重さは承認の側で扱う。gateway は宣言の有無だけを見る）。
#[tokio::test]
async fn annotations_never_override_declared_tier() {
    let connected = gateway(declarations()).await;
    let catalog = connected.tool_catalog().await.expect("一覧が取れる");
    let post = find(&catalog, "post_message");
    // destructiveHint=true の申告のまま、宣言どおり External。
    assert_eq!(post.tier, ActionTier::External);
    assert_eq!(post.annotations.destructive_hint, Some(true));
    let outcome = connected
        .call_tool("post_message", JsonObject::new())
        .await
        .expect("宣言済みなら annotations と無関係に呼べる");
    assert_eq!(outcome.text, vec!["called post_message args()".to_owned()]);
}
