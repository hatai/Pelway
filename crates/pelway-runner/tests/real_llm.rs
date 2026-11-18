//! 実 LLM（GLM-5.3 Flash / 予備の提供元）への接続確認（#330）。手動実行専用。
//!
//! 実行手順（fnox でキーを注入。値はリポジトリ・CI に置かない）:
//! ```sh
//! mise exec -- fnox exec -- cargo test -p pelway-runner --test real_llm -- --ignored
//! ```
//! API キーの環境変数（`demo::API_KEY_ENV`）が無いときは実行自体をスキップする（台本モードの確認は
//! `tests/runner.rs` のフェイクで済んでいる）。予備の GPT Luna の確認は
//! `PELWAY_LLM_PROVIDER=openai PELWAY_LLM_BASE_URL=... PELWAY_LLM_CHAT_MODEL=...`
//! を差し替えて同じコマンドを1回動かす（docs/dev-environment.md「デモの LLM 接続」）。
//!
//! この環境変数を CI には置かない（テストはデフォルトで ignored のため CI は
//! 実 LLM に接続しない）。

use std::time::Duration;

use pelway_contract::store::PurposeId;
use pelway_llm::{ChatMessage, ChatRequest};
use pelway_runner::demo::DemoLlmConfig;

/// A/B 案の応答スキーマ（runner と同じ形）。
fn ab_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "required": ["option_a", "option_b"],
        "properties": {
            "option_a": {"type": "string"},
            "option_b": {"type": "string"}
        },
        "additionalProperties": false
    })
}

/// 実キーで A/B 案が生成され、スキーマの検証を通ること（受け入れ: 実 LLM で
/// 1回完走）。`--ignored` 付きで明示的に実行する。
#[test]
#[ignore = "実 LLM 接続の手動確認用。fnox でキーを注入して実行する"]
fn real_llm_generates_ab_proposals_passing_schema() {
    if std::env::var(pelway_runner::demo::API_KEY_ENV).is_err() {
        eprintln!(
            "skip: {} が無い（fnox で注入して実行する）",
            pelway_runner::demo::API_KEY_ENV
        );
        return;
    }
    let config = DemoLlmConfig::from_env().expect("env config");
    let gateway = config.build_gateway();
    let request = ChatRequest {
        messages: vec![
            ChatMessage::system(
                "あなたは社内の AI Bot「ハル」です。依頼に対して案を A/B の2つ作ってください。",
            ),
            ChatMessage::user("案件Bの納期が遅れそうです。対応案を出してください。"),
        ],
        schema: Some(ab_schema()),
    };
    let reply = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("rt")
        .block_on(async move {
            tokio::time::timeout(Duration::from_secs(60), gateway.chat(request)).await
        })
        .expect("timeout")
        .expect("chat");

    // 応答は A/B の形（非空白の option_a / option_b）を満たす。
    let value: serde_json::Value = serde_json::from_str(&reply.text).expect("JSON");
    for key in ["option_a", "option_b"] {
        let text = value[key].as_str().unwrap_or("");
        assert!(!text.trim().is_empty(), "{key}: {value}");
    }
    // 応答の中身は出さない（本文をログに残さない決まり）。利用量だけ確認する。
    assert!(reply.usage.calls >= 1);
}

/// 設定の確認（実キー無しでも組み立てられる。既定は Z.ai + GLM-5.3 Flash + P06）。
#[test]
#[ignore = "手動確認用"]
fn real_llm_config_is_explicit() {
    let config = DemoLlmConfig::from_env().expect("env config");
    eprintln!(
        "provider={} base_url={} chat_model={} timeout={:?} basis={}",
        config.provider.name,
        config.provider.base_url,
        config.provider.chat_model,
        config.timeout,
        config.llm_basis.as_str()
    );
    assert_eq!(config.llm_basis, PurposeId::P06);
}
