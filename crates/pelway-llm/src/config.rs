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

/// 設定で受けるカスタム接続先の検証（#330 QA 点検 2026-11-17）。loopback
/// （127.0.0.0/8・::1・localhost）への接続先は任意のスキームを許し（ローカルの
/// モック用）、それ以外は https の URL に限る。http の外部接続先は Bearer キーの
/// 送出経路になりうるため拒否する。
pub fn custom_base_url_allowed(base_url: &str) -> bool {
    let Ok(parsed) = url::Url::parse(base_url) else {
        return false;
    };
    let loopback = match parsed.host() {
        Some(url::Host::Domain(d)) => d == "localhost" || d.ends_with(".localhost"),
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        None => false,
    };
    loopback || parsed.scheme() == "https"
}

/// 接続先 URL のログ向けの伏せ（Issue #404）。URL に秘密が入りうるのは
/// `PELWAY_LLM_CUSTOM_BASE_URL` で明示したカスタム接続先のみで、userinfo
/// （`user:pass@`）は丸ごと `***` に、クエリの `key` / `token` / `api_key` /
/// `access_token` / `auth` / `credential` など（キー名を小文字化して `_`・`-`
/// を抜いた形が `key`・`token`・`secret`・`password`・`auth`・`credential` を
/// 含むもの）は値を `***` に伏せる（キー名は残す）。fragment（`#` 以降）は
/// 秘密の載せ場所になりうるため中身を見ずに丸ごと落とす。どれも無い URL は
/// そのまま返す。URL として解釈できないものは中身を出さず全体を `***` にする。
pub fn redact_url_for_log(base_url: &str) -> String {
    let Ok(mut parsed) = url::Url::parse(base_url) else {
        return "***".to_owned();
    };
    // userinfo は丸ごと伏せる（user 名も password も出さない。`***@` だけ残る）。
    if !parsed.username().is_empty() || parsed.password().is_some() {
        let _ = parsed.set_username("***");
        let _ = parsed.set_password(None);
    }
    // fragment は丸ごと落とす（`#access_token=...` など OAuth implicit flow の
    // 秘密の載せ場所。中身を見て判断しない）。
    parsed.set_fragment(None);
    // クエリは値だけ伏せる。キー名の正規化（小文字化・`_`/`-` 除去）で
    // `api_key`・`api-key`・`apiKey` など同一視し、秘密を含みうる名前は
    // 過剰側（含む一致）で判定する（`auth`・`credential` は
    // `authorization`・`credentials` などの表記ゆれも含む一致でカバー）。
    if parsed.query().is_some() {
        let redacted: Vec<(String, String)> = parsed
            .query_pairs()
            .map(|(k, v)| {
                let normalized = k.to_lowercase().replace(['_', '-'], "");
                let secret_key = ["key", "token", "secret", "password", "auth", "credential"]
                    .iter()
                    .any(|marker| normalized.contains(marker));
                (
                    k.into_owned(),
                    if secret_key {
                        "***".to_owned()
                    } else {
                        v.into_owned()
                    },
                )
            })
            .collect();
        parsed
            .query_pairs_mut()
            .clear()
            .extend_pairs(redacted.iter().map(|(k, v)| (k.as_str(), v.as_str())));
    }
    parsed.to_string()
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
