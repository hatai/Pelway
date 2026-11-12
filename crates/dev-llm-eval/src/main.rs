//! `llm-eval` コマンド本体。`mise run llm-eval -- --provider zai` の形で使い、
//! fnox の `PELWAY_LLM_API_KEY`（`--api-key-env` で変更可）からトークンを取る。
//! モックサーバーに対しては `--base-url http://127.0.0.1:PORT/v1` を渡す。

use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use pelway_dev_llm_eval::{markdown, parse_args, run_eval};
use pelway_llm::token::FnoxEnvTokenSource;
use pelway_llm::{GatewayOptions, OpenAiGateway, ProviderConfig};

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let cli = match parse_args(std::env::args()) {
        Ok(cli) => cli,
        Err(message) => {
            eprintln!("llm-eval: {message}");
            eprintln!(
                "使い方: llm-eval [--provider zai|openai] [--base-url URL] [--chat-model NAME] [--embed-model NAME] [--api-key-env VAR] [--k N]"
            );
            return ExitCode::FAILURE;
        }
    };
    let mut config = match cli.provider.as_str() {
        "zai" => ProviderConfig::zai_example(),
        "openai" => ProviderConfig::openai_example(),
        other => {
            eprintln!("llm-eval: --provider は zai か openai を指定してください（{other}）");
            return ExitCode::FAILURE;
        }
    };
    if let Some(base_url) = &cli.base_url {
        config.base_url = base_url.clone();
    }
    if let Some(model) = &cli.chat_model {
        config.chat_model = model.clone();
    }
    if let Some(model) = &cli.embed_model {
        config.embed_model = model.clone();
    }
    // var は &'static str のため、CLI の実行が終わるまで存続させる。
    let api_key_env: &'static str = Box::leak(cli.api_key_env.clone().into_boxed_str());
    let options = GatewayOptions {
        https_only: cli.https_only,
        // 実測では長い応答も扱うため、全体のタイムアウトを広めに取る。
        timeout: Duration::from_secs(300),
        ..GatewayOptions::default()
    };
    let gateway = match OpenAiGateway::new(
        &config,
        Arc::new(FnoxEnvTokenSource { var: api_key_env }),
        &options,
    ) {
        Ok(gateway) => gateway,
        Err(error) => {
            eprintln!("llm-eval: ゲートウェイを構築できません: {error}");
            return ExitCode::FAILURE;
        }
    };
    println!(
        "# LLMゲートウェイ測定: {}（chat: {}, embed: {}, k={}）",
        config.name, config.chat_model, config.embed_model, cli.k
    );
    match run_eval(&gateway, cli.k).await {
        Ok(report) => {
            println!("{}", markdown(&report).trim_end());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("llm-eval: 測定に失敗しました: {error}");
            ExitCode::FAILURE
        }
    }
}
