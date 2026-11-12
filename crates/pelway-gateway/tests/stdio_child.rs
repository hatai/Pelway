//! stdio 子プロセスの MCP サーバー（src/bin/pelway-gateway-echo.rs）に
//! `Gateway::connect_stdio` で接続し、本体の既定の接続方式（02 の「国内ローカル
//! stdio のみ許可」案）で一覧と呼び出しが動くことを確かめる（#96）。
//! 段階が宣言されていないツールを止める検証は tests/inprocess_gateway.rs で
//! 行う（止める判断は Gateway にあり、接続方式に依らない）。

use pelway_gateway::gateway::Gateway;
use pelway_gateway::tier::{ActionTier, TierDeclarations};
use rmcp::model::JsonObject;

fn declarations() -> TierDeclarations {
    let mut declarations = TierDeclarations::new();
    declarations.declare("echo", ActionTier::Read);
    declarations
}

// 受け入れ条件（#96）: gateway の中で（stdio 子プロセスの）MCP サーバーに接続し、
// ツールの一覧・annotations・呼び出しを扱える。
#[tokio::test]
async fn stdio_child_process_connection_lists_and_calls_tools() {
    let bin = env!("CARGO_BIN_EXE_pelway-gateway-echo");
    let connected = Gateway::connect_stdio(declarations(), bin, &[])
        .await
        .expect("echo サーバーに stdio で接続できる");

    // 一覧: ツールと、手がかりとしての annotations が読み取れる。
    let catalog = connected.tool_catalog().await.expect("一覧が取れる");
    assert_eq!(catalog.len(), 1);
    let echo = &catalog[0];
    assert_eq!(echo.tier, ActionTier::Read);
    assert_eq!(echo.annotations.read_only_hint, Some(true));

    // 呼び出し: 宣言済みツールの引数が通り、テキストが返る。
    let outcome = connected
        .call_tool("echo", JsonObject::new())
        .await
        .expect("宣言済みツールは呼べる");
    assert!(!outcome.is_error);
    assert_eq!(outcome.text, vec!["echo:echo".to_owned()]);
}
