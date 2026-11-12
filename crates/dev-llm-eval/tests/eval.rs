//! `pelway-dev-llm-eval` の試験。ローカルのモックHTTPサーバーだけで測定パイプラインを
//! 動かし、外部ネットワークに出ない（Issue #95 の受け入れ条件「CIでは外部へつながらず、
//! モックで動くことだけを試す」）。

use std::sync::Arc;
use std::time::Duration;

use pelway_dev_llm_eval::{EvalReport, markdown, parse_args, run_eval};
use pelway_llm::token::TokenSource;
use pelway_llm::{
    GatewayOptions, LlmError, OpenAiGateway, ProviderConfig, ResponseFormatCapability,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// ---- モックHTTPサーバー（最小版）----

/// chat は依頼スキーマの語（handover / risk / summary）から
/// 適合する JSON を返す。embeddings は入力文のバイト和から決まる
/// 決定的なベクトルを返す（recall@k が毎回同じ値になる）。
async fn mock_server() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                let mut buf = Vec::new();
                let mut chunk = [0u8; 4096];
                let (head_end, content_length, path) = loop {
                    let n = stream.read(&mut chunk).await.unwrap_or(0);
                    if n == 0 {
                        return;
                    }
                    buf.extend_from_slice(&chunk[..n]);
                    if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&buf[..pos]).into_owned();
                        let mut lines = head.split("\r\n");
                        let path = lines
                            .next()
                            .unwrap_or_default()
                            .split_whitespace()
                            .nth(1)
                            .unwrap_or_default()
                            .to_string();
                        let mut length = 0usize;
                        for line in lines {
                            if let Some((name, value)) = line.split_once(':')
                                && name.trim().eq_ignore_ascii_case("content-length")
                            {
                                length = value.trim().parse().unwrap_or(0);
                            }
                        }
                        break (pos, length, path);
                    }
                };
                let mut body = buf[head_end + 4..].to_vec();
                while body.len() < content_length {
                    let n = stream.read(&mut chunk).await.unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    body.extend_from_slice(&chunk[..n]);
                }
                let text = String::from_utf8_lossy(&body).into_owned();
                let reply = if path.ends_with("/embeddings") {
                    let request: serde_json::Value = serde_json::from_str(&text).unwrap();
                    let inputs = request["input"].as_array().unwrap();
                    let data: Vec<String> = inputs
                        .iter()
                        .map(|v| {
                            let sum: usize = v
                                .as_str()
                                .map(|s| s.bytes().map(usize::from).sum())
                                .unwrap_or(1);
                            let vector: Vec<String> = (0..4)
                                .map(|i| format!("{:.3}", ((sum >> (i * 3)) % 7) as f64 / 7.0))
                                .collect();
                            format!(
                                r#"{{"object":"embedding","index":0,"embedding":[{}]}}"#,
                                vector.join(",")
                            )
                        })
                        .collect();
                    format!(
                        r#"{{"object":"list","model":"mock","data":[{}],"usage":{{"prompt_tokens":5,"total_tokens":5}}}}"#,
                        data.join(",")
                    )
                } else {
                    // 課題の判別は応答スキーマのプロパティ名で行う
                    //（json_schema も json_object も本文にスキーマが現れる）。
                    let content = if text.contains("decisions") {
                        r#"{"summary":"送料精算の締め日を変更した。","decisions":["来月の請求分から適用"]}"#
                    } else if text.contains("risk") {
                        r#"{"title":"備品レンタル費の支払い","summary":"イベント出展に伴う備品レンタル費18万円を販売促進枠から支払う。","risk":"low"}"#
                    } else if text.contains("items") {
                        r#"{"items":["入力データのチェック","差異ログの記録","問い合わせ先の確認","締め日の確認","引き継ぎ会の設定"]}"#
                    } else {
                        r#"{"summary":"送料精算の締め日を変更した。","decisions":["来月の請求分から適用"]}"#
                    };
                    format!(
                        r#"{{"id":"cmpl-mock","object":"chat.completion","created":1700000000,"model":"mock","choices":[{{"index":0,"message":{{"role":"assistant","content":{}}},"finish_reason":"stop"}}],"usage":{{"prompt_tokens":11,"completion_tokens":7,"total_tokens":18}}}}"#,
                        serde_json::json!(content)
                    )
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                    reply.len()
                );
                let _ = stream.write_all(response.as_bytes()).await;
                let _ = stream.shutdown().await;
            });
        }
    });
    format!("http://{addr}/v1")
}

fn mock_gateway(base_url: &str) -> OpenAiGateway {
    struct Fixed(&'static str);
    impl TokenSource for Fixed {
        fn token(&self) -> Result<pelway_llm::SecretToken, LlmError> {
            Ok(pelway_llm::SecretToken::new(self.0))
        }
    }
    OpenAiGateway::new(
        &ProviderConfig::new(
            "mock",
            base_url,
            "mock-chat",
            "mock-embed",
            ResponseFormatCapability::JsonSchema,
        ),
        Arc::new(Fixed("mock-token")),
        &GatewayOptions {
            https_only: false,
            timeout: Duration::from_secs(5),
            ..GatewayOptions::default()
        },
    )
    .unwrap()
}

// ---- 受け入れ条件: 測定コマンドがモックで動き、表を出す ----

/// モックに対して全課題を流すと、件数・スキーマ適合数・recall@k が揃った
/// 表が出る。recall@k は決定的なモックベクトルで毎回同じ値になる。
#[tokio::test]
async fn eval_runs_against_mock_and_reports_table() {
    let base_url = mock_server().await;
    let gateway = mock_gateway(&base_url);
    let report = run_eval(&gateway, 3).await.unwrap();

    assert_eq!(report.tasks.len(), 3);
    for task in &report.tasks {
        assert_eq!(task.schema_ok, task.total, "{:?}", task.task);
        assert!(task.avg_ms > 0.0);
        assert!(task.input_tokens > 0);
        assert!(task.output_tokens > 0);
    }
    let retrieval = report.retrieval.as_ref().unwrap();
    assert_eq!(retrieval.queries, 4);
    assert!((0.0..=1.0).contains(&retrieval.recall_at_k));

    let table = markdown(&report);
    assert!(table.contains("| タスク |"), "表のヘッダがない: {table}");
    assert!(table.contains("要約"), "{table}");
    assert!(table.contains("承認カード"), "{table}");
    assert!(table.contains("引き継ぎ"), "{table}");
    assert!(table.contains("recall@3"), "{table}");
    // スキーマ適合は「n/n」の形で出る。
    assert!(table.contains("3/3"), "{table}");

    // 決定的: もう一度流しても recall は同じ値。
    let gateway = mock_gateway(&base_url);
    let again = run_eval(&gateway, 3).await.unwrap();
    assert_eq!(
        again.retrieval.as_ref().unwrap().recall_at_k,
        retrieval.recall_at_k
    );
}

/// 空のレポートでも表の形は保つ。
#[test]
fn markdown_keeps_shape_for_partial_report() {
    let report = EvalReport {
        tasks: Vec::new(),
        retrieval: None,
    };
    let table = markdown(&report);
    assert!(table.contains("| タスク |"));
    assert!(table.contains("検索"), "{table}");
}

// ---- CLI 引数 ----

/// 既定は Z.ai、上書きは base_url・モデル・トークン変数・k。
#[test]
fn args_defaults_and_overrides() {
    let cli = parse_args(["llm-eval".to_string()].into_iter()).unwrap();
    assert_eq!(cli.provider, "zai");
    assert_eq!(cli.api_key_env, "PELWAY_LLM_API_KEY");
    assert_eq!(cli.k, 3);
    assert!(cli.https_only, "https の既定 URL では https_only");

    let cli = parse_args(
        [
            "llm-eval".to_string(),
            "--provider".to_string(),
            "openai".to_string(),
            "--base-url".to_string(),
            "http://127.0.0.1:9/v1".to_string(),
            "--chat-model".to_string(),
            "gpt-x".to_string(),
            "--k".to_string(),
            "5".to_string(),
        ]
        .into_iter(),
    )
    .unwrap();
    assert_eq!(cli.provider, "openai");
    assert_eq!(cli.base_url.as_deref(), Some("http://127.0.0.1:9/v1"));
    assert_eq!(cli.chat_model.as_deref(), Some("gpt-x"));
    assert_eq!(cli.k, 5);
    assert!(!cli.https_only, "http の URL では https_only を切る");

    assert!(parse_args(["llm-eval".to_string(), "--nope".to_string()].into_iter()).is_err());
}
