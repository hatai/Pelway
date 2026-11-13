//! 提供元の設定。提供元はこの設定で1つだけ選び、自動切替の経路は持たない。

/// `response_format` の能力。Z.ai の公式APIリファレンスが `response_format` に挙げるのは
/// `text` と `json_object` だけのため、能力を設定で受け取る
/// （docs/research/rust-llm-client-comparison.md「Z.ai」）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResponseFormatCapability {
    /// `response_format: json_schema` を送れる（OpenAI、Vertex AI の互換エンドポイント）。
    /// スキーマはネイティブに渡し、応答は Pelway 側でも検証する。
    JsonSchema,
    /// `json_object` まで。スキーマはプロンプト側で渡し、応答は Pelway 側で検証する。
    JsonObject,
}

/// 1つの OpenAI 互換提供元への接続設定。
#[derive(Clone, Debug, PartialEq)]
pub struct ProviderConfig {
    /// 表示名（ログやエラー用。例: `zai`、`openai`）。
    pub name: String,
    /// OpenAI 互換のベースURL。末尾の `/` は許容する（例: `https://api.z.ai/api/paas/v4`）。
    pub base_url: String,
    /// チャット用モデル名。
    pub chat_model: String,
    /// 埋め込み用モデル名。
    pub embed_model: String,
    /// `response_format` の能力。
    pub response_format: ResponseFormatCapability,
}

impl ProviderConfig {
    pub fn new(
        name: impl Into<String>,
        base_url: impl Into<String>,
        chat_model: impl Into<String>,
        embed_model: impl Into<String>,
        response_format: ResponseFormatCapability,
    ) -> Self {
        Self {
            name: name.into(),
            base_url: base_url.into(),
            chat_model: chat_model.into(),
            embed_model: embed_model.into(),
            response_format,
        }
    }

    /// Z.ai の設定例（ADR 0008。汎用エンドポイント。`response_format` は `json_object` まで）。
    /// モデル名は測定の対象ごとに差し替える。
    pub fn zai_example() -> Self {
        Self::new(
            "zai",
            "https://api.z.ai/api/paas/v4",
            "glm-4.6",
            "embedding-3",
            ResponseFormatCapability::JsonObject,
        )
    }

    /// OpenAI の設定例（ADR 0008。`response_format` は `json_schema` 対応）。
    /// モデル名は測定の対象ごとに差し替える。
    pub fn openai_example() -> Self {
        Self::new(
            "openai",
            "https://api.openai.com/v1",
            "gpt-4o-mini",
            "text-embedding-3-small",
            ResponseFormatCapability::JsonSchema,
        )
    }
}
