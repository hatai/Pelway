//! `pelway-llm` の結合試験。すべてローカルのモックHTTPサーバーで行い、
//! 外部ネットワークには出ない（Issue #95 の受け入れ条件）。
//!
//! モックサーバーは `tokio::net::TcpListener` 上の最小の HTTP/1.1 サーバーで、
//! 受け取った要求（メソッド・パス・Authorization・本文）を記録し、
//! テストが指定した応答（即時 / 遅延）を返す。応答は `Connection: close` で
//! 接続を閉じるため、要求1件ごとに接続が1本になる。

use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use pelway_llm::token::{FnoxEnvTokenSource, TokenSource};
use pelway_llm::{
    CallBudget, ChatMessage, ChatRequest, GatewayOptions, LlmError, LlmGateway, OpenAiGateway,
    ProviderConfig, ResponseFormatCapability,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// ---- モックHTTPサーバー ----

/// モックが記録した要求1件。
#[derive(Clone, Debug)]
struct Recorded {
    method: String,
    path: String,
    authorization: Option<String>,
    body: String,
}

/// モックの応答方法。
enum Reply {
    Json {
        status: u16,
        body: String,
    },
    /// `after` 待ってから応答する（クライアントの全体タイムアウトを試す）。
    DelayedJson {
        after: Duration,
        status: u16,
        body: String,
    },
}

impl Reply {
    fn ok(body: &str) -> Reply {
        Reply::Json {
            status: 200,
            body: body.to_string(),
        }
    }

    fn status(status: u16) -> Reply {
        Reply::Json {
            status,
            body: format!("{{\"error\":{{\"status\":{status}}}}}"),
        }
    }
}

struct Mock {
    base_url: String,
    records: Arc<Mutex<Vec<Recorded>>>,
    abort: tokio::task::AbortHandle,
}

impl Mock {
    fn records(&self) -> Vec<Recorded> {
        self.records.lock().unwrap().clone()
    }

    /// 記録が `count` 件に届くまで待つ（遅延応答の試験で使う）。
    async fn await_records(&self, count: usize) -> Vec<Recorded> {
        for _ in 0..100 {
            let records = self.records();
            if records.len() >= count {
                return records;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        self.records()
    }
}

impl Drop for Mock {
    fn drop(&mut self) {
        self.abort.abort();
    }
}

async fn mock_server(handler: Arc<dyn Fn(&Recorded) -> Reply + Send + Sync>) -> Mock {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let records: Arc<Mutex<Vec<Recorded>>> = Arc::new(Mutex::new(Vec::new()));
    let task_records = records.clone();
    let task = tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                break;
            };
            let records = task_records.clone();
            let handler = handler.clone();
            tokio::spawn(async move {
                let _ = serve_connection(stream, records, handler).await;
            });
        }
    });
    Mock {
        base_url: format!("http://{addr}/v1"),
        records,
        abort: task.abort_handle(),
    }
}

/// 接続1本 = 要求1件。要求を読んで記録し、応答して閉じる。
async fn serve_connection(
    mut stream: tokio::net::TcpStream,
    records: Arc<Mutex<Vec<Recorded>>>,
    handler: Arc<dyn Fn(&Recorded) -> Reply + Send + Sync>,
) -> std::io::Result<()> {
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 4096];
    let header_end = loop {
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(pos) = find_header_end(&buf) {
            break pos;
        }
    };
    let head = String::from_utf8_lossy(&buf[..header_end]).into_owned();
    let mut lines = head.split("\r\n");
    let mut parts = lines.next().unwrap_or_default().split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();
    let mut authorization = None;
    let mut content_length = 0usize;
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            let (name, value) = (name.trim().to_ascii_lowercase(), value.trim());
            match name.as_str() {
                "authorization" => authorization = Some(value.to_string()),
                "content-length" => content_length = value.parse().unwrap_or(0),
                _ => {}
            }
        }
    }
    let mut body = buf[header_end + 4..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    let record = Recorded {
        method,
        path,
        authorization,
        body: String::from_utf8_lossy(&body[..content_length.min(body.len())]).into_owned(),
    };
    records.lock().unwrap().push(record.clone());
    match handler(&record) {
        Reply::Json { status, body } => write_response(&mut stream, status, &body).await?,
        Reply::DelayedJson {
            after,
            status,
            body,
        } => {
            tokio::time::sleep(after).await;
            write_response(&mut stream, status, &body).await?;
        }
    }
    stream.shutdown().await.ok();
    Ok(())
}

async fn write_response(
    stream: &mut tokio::net::TcpStream,
    status: u16,
    body: &str,
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        _ => "OK",
    };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).await
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

// ---- テスト用の共通部品 ----

/// chat completions の応答本文。`content` は JSON 文字列リテラル（`"\"本文\""` の形）。
fn chat_json(content: &str) -> String {
    format!(
        r#"{{"id":"cmpl-mock","object":"chat.completion","created":1700000000,"model":"mock-chat",
        "choices":[{{"index":0,"message":{{"role":"assistant","content":{content}}},"finish_reason":"stop"}}],
        "usage":{{"prompt_tokens":11,"completion_tokens":7,"total_tokens":18}}}}"#
    )
}

/// 埋め込みの応答本文。`count` 件の `dim` 次元ベクトルを返す。
fn embed_json(dim: usize, count: usize) -> String {
    let vector = format!(
        "[{}]",
        vec![0.25; dim]
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    let data = (0..count)
        .map(|i| format!(r#"{{"object":"embedding","index":{i},"embedding":{vector}}}"#))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{"object":"list","model":"mock-embed","data":[{data}],"usage":{{"prompt_tokens":5,"completion_tokens":0,"total_tokens":5}}}}"#
    )
}

fn config(base_url: &str, capability: ResponseFormatCapability) -> ProviderConfig {
    ProviderConfig::new("mock", base_url, "mock-chat", "mock-embed", capability)
}

fn options() -> GatewayOptions {
    GatewayOptions {
        https_only: false,
        timeout: Duration::from_secs(5),
        ..GatewayOptions::default()
    }
}

fn test_gateway(
    base_url: &str,
    capability: ResponseFormatCapability,
    opts: GatewayOptions,
) -> OpenAiGateway {
    OpenAiGateway::new(
        &config(base_url, capability),
        Arc::new("fixed-token".to_string()),
        &opts,
    )
    .unwrap()
}

async fn chat(gateway: &OpenAiGateway, prompt: &str) -> Result<String, LlmError> {
    let reply = gateway
        .chat(ChatRequest::from_messages(vec![ChatMessage::user(prompt)]))
        .await?;
    Ok(reply.text)
}

// ---- 受け入れ条件: trait（chat スキーマなし/あり・embed）と rig の隠蔽 ----

/// スキーマなしの chat: 本文・Usage が返り、要求は1回だけ指定の提供元へ送られる。
#[tokio::test]
async fn chat_without_schema_returns_text_and_usage() {
    let mock = mock_server(Arc::new(|_: &Recorded| {
        Reply::ok(&chat_json("\"要約: 打ち合わせの要点\""))
    }))
    .await;
    let gateway = test_gateway(
        &mock.base_url,
        ResponseFormatCapability::JsonSchema,
        options(),
    );
    let reply = gateway
        .chat(ChatRequest::from_messages(vec![ChatMessage::user(
            "要約して",
        )]))
        .await
        .unwrap();
    assert_eq!(reply.text, "要約: 打ち合わせの要点");
    assert_eq!(reply.usage.input_tokens, 11);
    assert_eq!(reply.usage.output_tokens, 7);
    assert_eq!(reply.usage.total_tokens, 18);
    assert_eq!(reply.usage.calls, 1);

    let records = mock.records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].method, "POST");
    assert_eq!(records[0].path, "/v1/chat/completions");
    // トークンは TokenSource から要求ごとに取り出され、Bearer として送られる。
    assert_eq!(
        records[0].authorization.as_deref(),
        Some("Bearer fixed-token")
    );
    let body: serde_json::Value = serde_json::from_str(&records[0].body).unwrap();
    assert_eq!(body["model"], "mock-chat");
    assert_eq!(body["messages"][0]["role"], "user");
}

/// スキーマあり（json_schema 対応の提供元）: response_format が json_schema になり、
/// Pelway 側の検証でスキーマ違反は Invalid になる。
#[tokio::test]
async fn chat_with_schema_uses_native_json_schema_and_validates_reply() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "summary": {"type": "string"},
            "risk": {"enum": ["low", "high"]}
        },
        "required": ["summary"],
        "additionalProperties": false
    });
    let reply_body = chat_json(r#""{\"summary\":\"要点\",\"risk\":\"low\"}""#);
    let mock = mock_server(Arc::new(move |_: &Recorded| Reply::ok(&reply_body))).await;
    let gateway = test_gateway(
        &mock.base_url,
        ResponseFormatCapability::JsonSchema,
        options(),
    );
    let reply = gateway
        .chat(ChatRequest {
            messages: vec![ChatMessage::user("要約して")],
            schema: Some(schema.clone()),
        })
        .await
        .unwrap();
    assert_eq!(reply.text, "{\"summary\":\"要点\",\"risk\":\"low\"}");

    let body: serde_json::Value = serde_json::from_str(&mock.records()[0].body).unwrap();
    assert_eq!(body["response_format"]["type"], "json_schema");

    // スキーマ違反の応答は、提供元が 200 を返しても Pelway 側で Invalid にする。
    let violating = chat_json(r#""{\"summary\":3}""#);
    let mock = mock_server(Arc::new(move |_: &Recorded| Reply::ok(&violating))).await;
    let gateway = test_gateway(
        &mock.base_url,
        ResponseFormatCapability::JsonSchema,
        options(),
    );
    let err = gateway
        .chat(ChatRequest {
            messages: vec![ChatMessage::user("要約して")],
            schema: Some(schema.clone()),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, LlmError::Invalid(_)), "{err:?}");

    // JSON ですらない応答も Invalid。
    let mock = mock_server(Arc::new(|_: &Recorded| {
        Reply::ok(&chat_json("\"これはJSONではない\""))
    }))
    .await;
    let gateway = test_gateway(
        &mock.base_url,
        ResponseFormatCapability::JsonSchema,
        options(),
    );
    let err = gateway
        .chat(ChatRequest {
            messages: vec![ChatMessage::user("要約して")],
            schema: Some(schema),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, LlmError::Invalid(_)), "{err:?}");
}

/// json_object までの提供元: スキーマはプロンプト側に渡り、response_format は
/// json_object になる。応答はやはり Pelway 側で検証する。
#[tokio::test]
async fn json_object_provider_receives_schema_in_prompt() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {"handover": {"type": "array", "items": {"type": "string"}}},
        "required": ["handover"]
    });
    let ok_reply = chat_json(r#""{\"handover\":[\"1 点目\",\"2 点目\"]}""#);
    let mock = mock_server(Arc::new(move |_: &Recorded| Reply::ok(&ok_reply))).await;
    let gateway = test_gateway(
        &mock.base_url,
        ResponseFormatCapability::JsonObject,
        options(),
    );
    let reply = gateway
        .chat(ChatRequest {
            messages: vec![ChatMessage::user("引き継ぎ5項目を出して")],
            schema: Some(schema.clone()),
        })
        .await
        .unwrap();
    assert!(reply.text.contains("handover"));

    let body: serde_json::Value = serde_json::from_str(&mock.records()[0].body).unwrap();
    assert_eq!(body["response_format"]["type"], "json_object");
    // スキーマ本文がプロンプト（先頭の system メッセージ）に現れる。
    // rig は system の content をテキスト部品の配列にして送る。
    let first = &body["messages"][0];
    assert_eq!(first["role"], "system");
    let prompt = first["content"][0]["text"].as_str().unwrap();
    assert!(prompt.contains("handover"), "prompt: {prompt}");
    // json_schema は送らない。
    assert!(body["response_format"].get("json_schema").is_none());

    // プロンプトで渡っても従わない応答は Pelway 側の検証で Invalid。
    let violating = chat_json(r#""{\"handover\":\"1 点目\"}""#);
    let mock = mock_server(Arc::new(move |_: &Recorded| Reply::ok(&violating))).await;
    let gateway = test_gateway(
        &mock.base_url,
        ResponseFormatCapability::JsonObject,
        options(),
    );
    let err = gateway
        .chat(ChatRequest {
            messages: vec![ChatMessage::user("引き継ぎ5項目を出して")],
            schema: Some(schema),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, LlmError::Invalid(_)), "{err:?}");
}

/// embed: 入力順のベクトルと Usage が返る。usage のない応答は Invalid。
#[tokio::test]
async fn embed_returns_vectors_in_order_with_usage() {
    let payload = embed_json(3, 2);
    let mock = mock_server(Arc::new(move |_: &Recorded| Reply::ok(&payload))).await;
    let gateway = test_gateway(
        &mock.base_url,
        ResponseFormatCapability::JsonSchema,
        options(),
    );
    let reply = gateway
        .embed(vec!["1 件目".to_string(), "2 件目".to_string()])
        .await
        .unwrap();
    assert_eq!(reply.vectors.len(), 2);
    assert_eq!(reply.vectors[0].len(), 3);
    assert_eq!(reply.vectors[0], vec![0.25, 0.25, 0.25]);
    assert_eq!(reply.usage.input_tokens, 5);
    assert_eq!(reply.usage.calls, 1);

    let records = mock.records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].path, "/v1/embeddings");
    let body: serde_json::Value = serde_json::from_str(&records[0].body).unwrap();
    assert_eq!(body["model"], "mock-embed");
    assert_eq!(body["input"][0], "1 件目");
    assert_eq!(body["input"][1], "2 件目");

    // OpenAI 互換の埋め込み応答に usage は必須（rig が検査する）→ Invalid。
    let no_usage = r#"{"object":"list","model":"mock-embed","data":[{"object":"embedding","index":0,"embedding":[0.1]}]}"#;
    let mock = mock_server(Arc::new(move |_: &Recorded| Reply::ok(no_usage))).await;
    let gateway = test_gateway(
        &mock.base_url,
        ResponseFormatCapability::JsonSchema,
        options(),
    );
    let err = gateway.embed(vec!["x".to_string()]).await.unwrap_err();
    assert!(matches!(err, LlmError::Invalid(_)), "{err:?}");
}

// ---- 受け入れ条件: 再送しない（5xx・タイムアウトで要求は1回）----

/// 5xx と 429 は拒否として返り、再送されない（要求は1回だけ）。
/// 呼出しごとに利用量が返り、回数上限を超える呼出しは送る前に止まる。
#[tokio::test]
async fn usage_accumulates_and_budget_stops_before_send() {
    let payload = chat_json("\"ok\"");
    let mock = mock_server(Arc::new(move |_: &Recorded| Reply::ok(&payload))).await;
    let opts = GatewayOptions {
        https_only: false,
        budget: Some(CallBudget { max_calls: 2 }),
        ..GatewayOptions::default()
    };
    let gateway = OpenAiGateway::new(
        &config(&mock.base_url, ResponseFormatCapability::JsonSchema),
        Arc::new("fixed-token".to_string()),
        &opts,
    )
    .unwrap();
    let first = gateway
        .chat(ChatRequest::from_messages(vec![ChatMessage::user("1")]))
        .await
        .unwrap();
    assert_eq!(first.usage.calls, 1);
    let second = gateway
        .chat(ChatRequest::from_messages(vec![ChatMessage::user("2")]))
        .await
        .unwrap();
    assert_eq!(second.usage.calls, 2);
    assert_eq!(gateway.usage().calls, 2);
    assert_eq!(gateway.usage().input_tokens, 22);

    let err = chat(&gateway, "3 回目").await.unwrap_err();
    assert!(matches!(err, LlmError::NotSent(_)), "{err:?}");
    // 上限での停止は送信前: モックは2件しか受けていない。
    assert_eq!(mock.records().len(), 2);
}

// ---- 受け入れ条件: 再送しない（5xx・タイムアウトで要求は1回）----

/// 5xx と 429 は拒否として返り、再送されない（要求は1回だけ）。
#[tokio::test]
async fn rejected_responses_are_returned_without_retry() {
    for status in [500u16, 429] {
        let mock = mock_server(Arc::new(move |_: &Recorded| Reply::status(status))).await;
        let gateway = test_gateway(
            &mock.base_url,
            ResponseFormatCapability::JsonSchema,
            options(),
        );
        let err = chat(&gateway, "要約して").await.unwrap_err();
        match err {
            LlmError::Rejected { status: got, .. } => assert_eq!(got, status),
            other => panic!("{status} は Rejected になるべき: {other:?}"),
        }
        // 自動再送も提供元の切替もしない: 設定した1つのオリジンへ1回だけ。
        assert_eq!(mock.records().len(), 1);
    }
}

/// 応答が遅く全体タイムアウトを過ぎると結果不明。要求は1回だけ。
#[tokio::test]
async fn timeout_after_send_is_outcome_unknown_without_resend() {
    let slow_body = chat_json("\"遅い\"");
    let mock = mock_server(Arc::new(move |_: &Recorded| Reply::DelayedJson {
        after: Duration::from_secs(30),
        status: 200,
        body: slow_body.clone(),
    }))
    .await;
    let mut opts = options();
    opts.timeout = Duration::from_millis(500);
    let gateway = test_gateway(&mock.base_url, ResponseFormatCapability::JsonSchema, opts);
    let err = chat(&gateway, "要約して").await.unwrap_err();
    assert!(matches!(err, LlmError::OutcomeUnknown(_)), "{err:?}");
    assert_eq!(mock.await_records(1).await.len(), 1);
}

// ---- 受け入れ条件: エラーの4分類 ----

/// 接続先がない（接続拒否）場合は未送信。
#[tokio::test]
async fn connection_refused_is_not_sent() {
    // 使っていないポートを1つ確保して閉じ、そこへ向ける。
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    let gateway = test_gateway(
        &format!("http://{addr}/v1"),
        ResponseFormatCapability::JsonSchema,
        options(),
    );
    let err = chat(&gateway, "要約して").await.unwrap_err();
    assert!(matches!(err, LlmError::NotSent(_)), "{err:?}");
}

/// 200 でも解釈できない本文は Invalid。
#[tokio::test]
async fn malformed_success_response_is_invalid() {
    let mock = mock_server(Arc::new(|_: &Recorded| {
        Reply::ok("これは JSON ではありません")
    }))
    .await;
    let gateway = test_gateway(
        &mock.base_url,
        ResponseFormatCapability::JsonSchema,
        options(),
    );
    let err = chat(&gateway, "要約して").await.unwrap_err();
    assert!(matches!(err, LlmError::Invalid(_)), "{err:?}");

    // JSON だが chat completions の形でない（choices がない）場合も Invalid。
    let bad_shape = r#"{"id":"cmpl-mock","model":"mock-chat"}"#;
    let mock = mock_server(Arc::new(move |_: &Recorded| Reply::ok(bad_shape))).await;
    let gateway = test_gateway(
        &mock.base_url,
        ResponseFormatCapability::JsonSchema,
        options(),
    );
    let err = chat(&gateway, "要約して").await.unwrap_err();
    assert!(matches!(err, LlmError::Invalid(_)), "{err:?}");
}

// ---- 受け入れ条件: トークン（要求ごとの取り出し・秘匿）----

/// 呼出しごとに TokenSource からトークンを取り出して Authorization に使う。
#[tokio::test]
async fn token_is_fetched_per_request() {
    struct Rotating(AtomicUsize);
    impl TokenSource for Rotating {
        fn token(&self) -> Result<pelway_llm::SecretToken, LlmError> {
            Ok(pelway_llm::SecretToken::new(format!(
                "tok-{}",
                self.0.fetch_add(1, Ordering::SeqCst)
            )))
        }
    }
    let payload = chat_json("\"ok\"");
    let mock = mock_server(Arc::new(move |_: &Recorded| Reply::ok(&payload))).await;
    let gateway = OpenAiGateway::new(
        &config(&mock.base_url, ResponseFormatCapability::JsonSchema),
        Arc::new(Rotating(AtomicUsize::new(0))),
        &options(),
    )
    .unwrap();
    chat(&gateway, "1 回目").await.unwrap();
    chat(&gateway, "2 回目").await.unwrap();
    let records = mock.records();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].authorization.as_deref(), Some("Bearer tok-0"));
    assert_eq!(records[1].authorization.as_deref(), Some("Bearer tok-1"));
}

/// トークン取得に失敗すると要求は送られない。
#[tokio::test]
async fn token_failure_aborts_before_send() {
    struct Broken;
    impl TokenSource for Broken {
        fn token(&self) -> Result<pelway_llm::SecretToken, LlmError> {
            Err(LlmError::NotSent("トークンが無効".to_string()))
        }
    }
    let payload = chat_json("\"ok\"");
    let mock = mock_server(Arc::new(move |_: &Recorded| Reply::ok(&payload))).await;
    let gateway = OpenAiGateway::new(
        &config(&mock.base_url, ResponseFormatCapability::JsonSchema),
        Arc::new(Broken),
        &options(),
    )
    .unwrap();
    let err = chat(&gateway, "要約して").await.unwrap_err();
    assert!(matches!(err, LlmError::NotSent(_)), "{err:?}");
    assert!(mock.records().is_empty());
}

/// Debug 出力とログにトークンが出ない。
#[tokio::test]
async fn token_never_appears_in_debug_or_logs() {
    let token = "sk-secret-do-not-print-1234";
    let payload = chat_json("\"ok\"");
    let mock = mock_server(Arc::new(move |_: &Recorded| Reply::ok(&payload))).await;
    let gateway = OpenAiGateway::new(
        &config(&mock.base_url, ResponseFormatCapability::JsonSchema),
        Arc::new(token.to_string()),
        &options(),
    )
    .unwrap();
    let debug = format!("{gateway:?}");
    assert!(!debug.contains(token), "Debug にトークンが出た: {debug}");

    // rig のログ（TRACE まで出す）を捕まえて、トークンが一度も出ないことを確かめる。
    let sink = Arc::new(Mutex::new(Vec::<u8>::new()));
    #[derive(Clone)]
    struct SharedWriter(Arc<Mutex<Vec<u8>>>);
    impl Write for SharedWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for SharedWriter {
        type Writer = SharedWriter;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }
    let subscriber = tracing_subscriber::fmt()
        .with_writer(SharedWriter(sink.clone()))
        .with_ansi(false)
        .with_max_level(tracing_subscriber::filter::LevelFilter::TRACE)
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    chat(&gateway, "要約して").await.unwrap();
    let logs = String::from_utf8_lossy(&sink.lock().unwrap()).into_owned();
    assert!(!logs.contains(token), "ログにトークンが出た: {logs}");
}

/// fnox の環境変数（PELWAY_LLM_API_KEY）からトークンを取る。
/// ほかのテストと並行しても壊れないよう、環境変数の変更は排他する。
/// このテストがロックを取っている間に同じロックを取るタスクは無い
/// （#[tokio::test] は current_thread。env を使うのはこのテストだけ）ため、
/// await をまたいでの保持を許す。
#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn fnox_env_token_source_reads_pelway_llm_api_key() {
    static ENV_LOCK: Mutex<()> = Mutex::new(());
    let _guard = ENV_LOCK.lock().unwrap();

    let source = FnoxEnvTokenSource::new();
    assert_eq!(source.var, "PELWAY_LLM_API_KEY");
    // 未設定なら未送信として失敗する。
    unsafe { std::env::remove_var("PELWAY_LLM_API_KEY") };
    assert!(matches!(source.token(), Err(LlmError::NotSent(_))));

    let payload = chat_json("\"ok\"");
    let mock = mock_server(Arc::new(move |_: &Recorded| Reply::ok(&payload))).await;
    unsafe { std::env::set_var("PELWAY_LLM_API_KEY", "env-token-value") };
    let gateway = OpenAiGateway::new(
        &config(&mock.base_url, ResponseFormatCapability::JsonSchema),
        Arc::new(FnoxEnvTokenSource::new()),
        &options(),
    )
    .unwrap();
    chat(&gateway, "要約して").await.unwrap();
    unsafe { std::env::remove_var("PELWAY_LLM_API_KEY") };
    let records = mock.records();
    assert_eq!(
        records[0].authorization.as_deref(),
        Some("Bearer env-token-value")
    );
}

// ---- 受け入れ条件: 利用量と上限 ----

// ---- 受け入れ条件: HTTPクライアントの設定 ----

/// プロキシはゲートウェイ構築時に渡る。不正な URL のプロキシは
/// `reqwest::Proxy::all` の段階で弾かれる（ゲートウェイまで届かない）。
#[tokio::test]
async fn proxy_is_applied_at_gateway_construction() {
    assert!(reqwest::Proxy::all("this is not a url").is_err());
    let proxy = reqwest::Proxy::all("http://127.0.0.1:9").unwrap();
    let opts = GatewayOptions {
        https_only: false,
        proxy: Some(proxy),
        ..GatewayOptions::default()
    };
    let gateway = OpenAiGateway::new(
        &config(
            "http://127.0.0.1:1/v1",
            ResponseFormatCapability::JsonSchema,
        ),
        Arc::new("t".to_string()),
        &opts,
    );
    assert!(gateway.is_ok());
}
