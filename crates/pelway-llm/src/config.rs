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

/// Z.ai の汎用 API エンドポイント（`https://api.z.ai/api/paas/v4`。
/// Coding Plan 向けの `/api/coding/paas/v4` は自前アプリからの呼び出しが
/// 認められていないため使わない。ADR 0008、Issue #330）。
/// 提供元 endpoint の宣言はこのファイルに限定する（demo-check external-refs の
/// 除外理由を scripts/demo-check/lib.sh に記録）。
pub const ZAI_BASE_URL: &str = "https://api.z.ai/api/paas/v4";

/// OpenAI の API エンドポイント（予備の提供元。ADR 0008 の「設定の差し替えによる
/// 予備」。endpoint の宣言はこのファイルに限定する）。
pub const OPENAI_BASE_URL: &str = "https://api.openai.com/v1";

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

    /// Z.ai の汎用エンドポイントへの接続設定（`response_format` は
    /// `json_object` まで。モデル名は呼び出し側の設定で決める）。
    pub fn zai(chat_model: impl Into<String>, embed_model: impl Into<String>) -> Self {
        Self::new(
            "zai",
            ZAI_BASE_URL,
            chat_model,
            embed_model,
            ResponseFormatCapability::JsonObject,
        )
    }

    /// OpenAI の接続設定（`response_format` は `json_schema` 対応。
    /// モデル名は呼び出し側の設定で決める）。
    pub fn openai(chat_model: impl Into<String>, embed_model: impl Into<String>) -> Self {
        Self::new(
            "openai",
            OPENAI_BASE_URL,
            chat_model,
            embed_model,
            ResponseFormatCapability::JsonSchema,
        )
    }
}
