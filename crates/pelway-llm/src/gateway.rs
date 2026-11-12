//! [`LlmGateway`] の実装。OpenAI 互換エンドポイントに rig-core でつなぐ。
//!
//! rig-core はここ（と [`crate::error`]）に隠れる。HTTP クライアントは Pelway 側で
//! `reqwest::Client` を作り（`retry::never()`・タイムアウト・プロキシ）、
//! [`rig_reqwest::ReqwestClient`] で包んで `connect()` に渡す。
//! 認証ヘッダーは [`HttpMiddleware`] の `before_request_headers` で要求ごとに付ける
//! （トークンを要求ごとに取り出す、ADR 0008）。rig 自身は再試行しない。

use std::sync::{Arc, Mutex};
use std::time::Duration;

use rig_core::completion::{CompletionRequest, Message};
use rig_core::http_client::middleware::{Exchange, HttpMiddleware};
use rig_core::http_client::{
    DynHttpClient, Error as TransportError, HeaderValue, Result as TransportResult,
};
use rig_core::providers::openai::OpenAIConfig;
use rig_core::providers::openai::wire::Auth;
use rig_reqwest::ReqwestClient;

use crate::config::{ProviderConfig, ResponseFormatCapability};
use crate::error::LlmError;
use crate::token::TokenSource;
use crate::{ChatReply, ChatRequest, ChatRole, EmbedReply, LlmGateway, Usage, schema};

/// HTTP クライアントと利用上限の設定。
#[derive(Clone, Debug)]
pub struct GatewayOptions {
    /// 接続のタイムアウト。
    pub connect_timeout: Duration,
    /// 要求全体（応答まで）のタイムアウト。この時間を過ぎると結果不明として返る。
    pub timeout: Duration,
    /// 限定proxyなど、明示的に渡すプロキシ（ADR 0008: 外向き通信はproxy経由に限定できる）。
    pub proxy: Option<reqwest::Proxy>,
    /// `https:` の URL だけに送る（既定）。モックサーバー試験では `false` にする。
    pub https_only: bool,
    /// 呼出し回数の上限。超える呼出しは送る前に [`LlmError::NotSent`] で止める。
    pub budget: Option<CallBudget>,
}

impl Default for GatewayOptions {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            timeout: Duration::from_secs(120),
            proxy: None,
            https_only: true,
            budget: None,
        }
    }
}

/// 呼出し回数の上限（ADR 0008「利用量をテナントごとに記録する」の下書き。
/// 保存は後のIssue。ここではメモリ上のカウンタで止める）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CallBudget {
    pub max_calls: u64,
}

/// ゲートウェイの累積利用量（メモリ上。永続化は後のIssue）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UsageState {
    pub calls: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// OpenAI 互換提供元へつなぐ [`LlmGateway`] 実装。
pub struct OpenAiGateway {
    name: String,
    base_url: String,
    capability: ResponseFormatCapability,
    chat: rig_core::driver::Model<rig_core::providers::openai::wire::Chat>,
    embed: rig_core::driver::Model<rig_core::providers::openai::wire::Embeddings>,
    meter: Arc<Mutex<UsageState>>,
    budget: Option<CallBudget>,
}

impl OpenAiGateway {
    /// `config` の提供元へ、`tokens` から要求ごとにトークンを取ってつなぐゲートウェイを組む。
    pub fn new(
        config: &ProviderConfig,
        tokens: Arc<dyn TokenSource>,
        options: &GatewayOptions,
    ) -> Result<Self, LlmError> {
        let mut builder = reqwest::Client::builder()
            // reqwest 0.13 は既定で「安全に再試行できる」要求を最大2回再送する
            // （HTTP/2 GOAWAY 等）。結果不明の自動再送を許さないため無効化する。
            .retry(reqwest::retry::never())
            .connect_timeout(options.connect_timeout)
            .timeout(options.timeout);
        if let Some(proxy) = &options.proxy {
            builder = builder.proxy(proxy.clone());
        }
        if options.https_only {
            builder = builder.https_only(true);
        }
        let http = builder
            .build()
            .map_err(|e| LlmError::NotSent(format!("HTTPクライアントの構築に失敗: {e}")))?;

        let transport =
            DynHttpClient::new(ReqwestClient::from(http)).with_middleware(BearerHook(tokens));
        // 空キー + OptionalBearer: wire 自身は認証ヘッダーを付けない。
        // ヘッダーは BearerHook が要求ごとに付ける（失敗すれば要求は送られない）。
        let provider = OpenAIConfig::new("")
            .with_auth(Auth::OptionalBearer)
            .with_base_url(config.base_url.trim_end_matches('/'))
            .connect(transport);

        Ok(Self {
            name: config.name.clone(),
            base_url: config.base_url.clone(),
            capability: config.response_format,
            chat: provider.chat(config.chat_model.clone()),
            embed: provider.embedding(config.embed_model.clone(), None),
            meter: Arc::new(Mutex::new(UsageState::default())),
            budget: options.budget,
        })
    }

    /// 累積利用量のスナップショット。
    pub fn usage(&self) -> UsageState {
        *self.meter.lock().expect("usage meter poisoned")
    }

    /// 呼出し回数の予算を確認する。超えていれば送る前に止める（何も送らない）。
    fn reserve_call(&self) -> Result<(), LlmError> {
        if let Some(budget) = &self.budget {
            let calls = self.meter.lock().expect("usage meter poisoned").calls;
            if calls >= budget.max_calls {
                return Err(LlmError::NotSent(format!(
                    "呼出し回数の上限（{} 回）に達したため送信しません",
                    budget.max_calls
                )));
            }
        }
        Ok(())
    }

    /// 1呼出し分の利用量を記録し、応答に載せる形で返す。
    fn record_usage(&self, usage: &rig_core::completion::Usage) -> Usage {
        let mut state = self.meter.lock().expect("usage meter poisoned");
        state.calls += 1;
        state.input_tokens += usage.input_tokens.unwrap_or(0);
        state.output_tokens += usage.output_tokens.unwrap_or(0);
        Usage {
            input_tokens: usage.input_tokens.unwrap_or(0),
            output_tokens: usage.output_tokens.unwrap_or(0),
            total_tokens: usage.total_tokens.unwrap_or(0),
            calls: state.calls,
        }
    }
}

impl LlmGateway for OpenAiGateway {
    /// 最後のメッセージをプロンプト、残りを履歴として送る。
    /// スキーマ指定時は応答を JSON として解釈し、Pelway 側で検証する。
    async fn chat(&self, req: ChatRequest) -> Result<ChatReply, LlmError> {
        self.reserve_call()?;
        let mut messages: Vec<Message> = req.messages.iter().map(to_rig_message).collect();
        let mut additional_params = None;
        if let Some(schema) = &req.schema
            && self.capability == ResponseFormatCapability::JsonObject
        {
            // json_object までの提供元: スキーマをプロンプト側で渡し、
            // response_format は json_object にする（ADR 0008）。
            let instruction = format!(
                "Respond with a single JSON value and nothing else. \
                 It must conform to this JSON Schema:\n{schema}"
            );
            messages.insert(0, Message::system(instruction));
            additional_params =
                Some(serde_json::json!({"response_format": {"type": "json_object"}}));
        }
        // CompletionRequest::new は末尾のメッセージをプロンプトとして扱う。
        let prompt = messages
            .pop()
            .ok_or_else(|| LlmError::NotSent("メッセージが空です".to_string()))?;
        let mut builder = CompletionRequest::new(prompt).messages(messages);
        if let Some(schema) = &req.schema
            && self.capability == ResponseFormatCapability::JsonSchema
        {
            let rig_schema: rig_core::schemars::Schema = serde_json::from_value(schema.clone())
                .map_err(|e| LlmError::NotSent(format!("JSONスキーマを解釈できません: {e}")))?;
            builder = builder.output_schema(rig_schema);
        }
        if let Some(params) = additional_params {
            builder = builder.additional_params(params);
        }
        let response = self.chat.call(builder).await.map_err(LlmError::from)?;
        let text = response.text();
        if let Some(schema) = &req.schema {
            let value = parse_reply_json(&text)?;
            schema::validate(&value, schema).map_err(LlmError::Invalid)?;
        }
        let usage = self.record_usage(&response.usage);
        Ok(ChatReply { text, usage })
    }

    async fn embed(&self, texts: Vec<String>) -> Result<EmbedReply, LlmError> {
        self.reserve_call()?;
        if texts.is_empty() {
            return Err(LlmError::NotSent(
                "埋め込み対象のテキストが空です".to_string(),
            ));
        }
        let response = self.embed.call(texts).await.map_err(LlmError::from)?;
        let usage = self.record_usage(&response.usage);
        Ok(EmbedReply {
            vectors: response.embeddings.iter().map(|e| e.vec.clone()).collect(),
            usage,
        })
    }
}

/// [`crate::ChatMessage`] を rig の [`Message`] へ写す。
fn to_rig_message(message: &crate::ChatMessage) -> Message {
    match message.role {
        ChatRole::System => Message::system(message.content.as_str()),
        ChatRole::User => Message::user(message.content.as_str()),
        ChatRole::Assistant => Message::assistant(message.content.as_str()),
    }
}

/// スキーマ指定の応答本文を JSON として解釈する。モデルがコードフェンスを
/// 付けてきた場合だけ外して再試行し、それでも失敗すれば Invalid。
fn parse_reply_json(text: &str) -> Result<serde_json::Value, LlmError> {
    let trimmed = text.trim();
    let unfenced = trimmed
        .strip_prefix("```")
        .map(|s| s.trim_start_matches("json").trim_start())
        .and_then(|s| s.strip_suffix("```"))
        .map(str::trim)
        .unwrap_or(trimmed);
    serde_json::from_str(unfenced)
        .map_err(|e| LlmError::Invalid(format!("応答を JSON として解釈できません: {e}")))
}

/// トークンを出さない Debug。トークンそのものは保持しない（要求ごとに取り出す）。
impl std::fmt::Debug for OpenAiGateway {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiGateway")
            .field("provider", &self.name)
            .field("base_url", &self.base_url)
            .field("response_format", &self.capability)
            .field("usage", &self.usage())
            .field("budget", &self.budget)
            .finish()
    }
}

/// 要求ごとに `Authorization: Bearer <token>` を付ける hook。
/// トークンの取り出しに失敗すると要求は送られない（未送信として返る）。
struct BearerHook(Arc<dyn TokenSource>);

impl HttpMiddleware for BearerHook {
    fn before_request_headers<'a>(
        &'a self,
        _exchange: Exchange<'a>,
        headers: &'a mut rig_core::http_client::HeaderMap,
    ) -> rig_core::wasm_compat::WasmBoxedFuture<'a, TransportResult<()>> {
        Box::pin(async move {
            let token = self.0.token().map_err(|e| {
                // io::Error で包む: error.rs はこれを未送信に分類する。
                TransportError::instance(std::io::Error::other(e.to_string()))
            })?;
            // SecretToken は Debug に出ない。値はここ（送信直前）だけで使う。
            let value: HeaderValue = format!("Bearer {}", token.expose())
                .parse()
                .map_err(TransportError::from)?;
            headers.insert(http::header::AUTHORIZATION, value);
            Ok(())
        })
    }
}

impl std::fmt::Debug for BearerHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("BearerHook")
            .field(&"<token source: トークンは出力しない>")
            .finish()
    }
}
