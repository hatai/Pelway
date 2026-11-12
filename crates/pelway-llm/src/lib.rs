//! Pelway の LLMゲートウェイ（下書き）。提供元の OpenAI 互換 API を
//! [`rig-core`](https://crates.io/crates/rig-core) 0.46 で呼ぶ（ADR 0008、#95）。
//!
//! - rig-core はこの crate の内側に隠す。公開するのは [`LlmGateway`] trait と
//!   その実装 [`OpenAiGateway`]（OpenAI 互換 chat completions と embeddings 専用。
//!   rig のエージェント・ツール・streaming は使わない）。
//! - 提供元は設定（[`ProviderConfig`]）で1つだけ選ぶ。自動切替・fallback はない
//!   （ADR 0008「提供元への接続には rig-core を使う」、Pelway 10 の D2/D3）。
//! - HTTP クライアントは Pelway 側で作って rig に渡す。`reqwest::retry::never()`、
//!   接続・全体のタイムアウト、プロキシを設定し、再送は一切しない。
//! - トークンは要求ごとに [`TokenSource`] から取り出す（MVP は fnox の
//!   `PELWAY_LLM_API_KEY`、将来の短時間トークンに差し替えられる形）。
//!   トークンは [`SecretToken`] に包み、`Debug` 出力に出さない。
//! - JSONスキーマ指定の応答は、提供元を問わず Pelway 側で検証する
//!   （[`schema::validate`]）。`json_object` だけの提供元にはスキーマをプロンプトへ渡す。
//!
//! 試験はすべてローカルのモックHTTPサーバーで行い、外部ネットワークに出ない。

pub mod config;
pub mod error;
pub mod gateway;
pub mod schema;
pub mod token;

pub use config::{ProviderConfig, ResponseFormatCapability};
pub use error::LlmError;
pub use gateway::{CallBudget, GatewayOptions, OpenAiGateway, UsageState};
pub use token::{FnoxEnvTokenSource, SecretToken, TokenSource};

use serde_json::Value;

/// LLMゲートウェイの公開インターフェース。rig-core は実装の内側に隠れる。
pub trait LlmGateway: Send + Sync {
    /// チャット補完（MVP はまとめて受け取る方式のみ。streaming なし）。
    ///
    /// [`ChatRequest::schema`] を渡すと、応答はその JSON スキーマに従うよう求められ、
    /// Pelway 側で検証される。従わない応答は [`LlmError::Invalid`]。
    fn chat(&self, req: ChatRequest) -> impl Future<Output = Result<ChatReply, LlmError>> + Send;

    /// 埋め込み。`texts` の順番どおりにベクトルを返す。
    fn embed(
        &self,
        texts: Vec<String>,
    ) -> impl Future<Output = Result<EmbedReply, LlmError>> + Send;
}

/// チャットの要求。
#[derive(Clone, Debug, PartialEq)]
pub struct ChatRequest {
    /// 会話（時系列順）。最初の system メッセージは前置きとして扱う。
    pub messages: Vec<ChatMessage>,
    /// 応答に課す JSON スキーマ。`None` なら自由文。
    pub schema: Option<Value>,
}

impl ChatRequest {
    /// メッセージだけで（スキーマなしで）組み立てる。
    pub fn from_messages(messages: Vec<ChatMessage>) -> Self {
        Self {
            messages,
            schema: None,
        }
    }
}

/// チャットの発話1件。
#[derive(Clone, Debug, PartialEq)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

impl ChatMessage {
    pub fn new(role: ChatRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self::new(ChatRole::System, content)
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new(ChatRole::User, content)
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(ChatRole::Assistant, content)
    }
}

/// 発話の役割。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

/// チャットの応答。`text` はモデルの出力そのもの（スキーマ指定時は JSON 文字列）。
#[derive(Clone, Debug, PartialEq)]
pub struct ChatReply {
    pub text: String,
    pub usage: Usage,
}

/// 埋め込みの応答。
#[derive(Clone, Debug, PartialEq)]
pub struct EmbedReply {
    /// 入力 `texts` の順番どおりのベクトル。
    pub vectors: Vec<Vec<f64>>,
    pub usage: Usage,
}

/// 1呼出し分の利用量。提供元が報告しない項目は 0。
/// `calls` はこのゲートウェイの累積呼出し回数。送信前に予約として数え、
/// 「未送信」に分類された失敗（何も送らなかったもの）は戻す。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub calls: u64,
}
