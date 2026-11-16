//! デモの LLM 接続設定と台本モード（Issue #330）。
//!
//! - 接続設定は環境変数（`PELWAY_LLM_*`。モデル名・接続先・環境変数名は
//!   このモジュールの定数として明示する）。提供元の自動切替はしない
//!   （ADR 0008）。予備の GPT Luna は設定の差し替えで使う
//!   （docs/dev-environment.md「デモの LLM 接続」）。
//! - キーの環境変数が無い、または値が送信に使える形でないときは、落ちずに
//!   台本モード（[`ScriptGateway`]。フォールバック応答セット）で起動する。
//! - LLM を呼ぶ根拠の目的 ID も設定で受ける（デモの既定は P06。BYOK。
//!   hatai 決定 2026-11-16）。コード・ゲートには固定しない。
//! - キーはこのモジュールに保持しない（要求ごとに
//!   [`pelway_llm::FnoxEnvTokenSource`] が環境変数から取り出す）。キーと
//!   依頼本文・LLM 応答本文はログ・エラーに出さない。

use std::sync::Arc;
use std::time::Duration;

use pelway_contract::store::PurposeId;
use pelway_llm::config::ResponseFormatCapability;
use pelway_llm::{
    ChatReply, ChatRequest, EmbedReply, FnoxEnvTokenSource, GatewayOptions, LlmError, LlmGateway,
    OpenAiGateway, ProviderConfig, Usage,
};

use crate::DynChatGateway;

/// キーを読む環境変数の既定名（fnox が注入する値。fnox.toml 参照）。
pub const API_KEY_ENV: &str = FnoxEnvTokenSource::DEFAULT_VAR;
/// キーを読む変数名を差し替える環境変数（通常は使わない）。
const API_KEY_ENV_NAME: &str = "PELWAY_LLM_KEY_ENV";
/// 提供元の表示名を受ける環境変数。
pub const PROVIDER_ENV: &str = "PELWAY_LLM_PROVIDER";
/// OpenAI 互換のベース URL を受ける環境変数。
pub const BASE_URL_ENV: &str = "PELWAY_LLM_BASE_URL";
/// チャット用モデル名を受ける環境変数。
pub const CHAT_MODEL_ENV: &str = "PELWAY_LLM_CHAT_MODEL";
/// 埋め込み用モデル名を受ける環境変数（デモの runner は埋め込みを呼ばない）。
pub const EMBED_MODEL_ENV: &str = "PELWAY_LLM_EMBED_MODEL";
/// `response_format` の能力を受ける環境変数（`json_object` / `json_schema`）。
pub const RESPONSE_FORMAT_ENV: &str = "PELWAY_LLM_RESPONSE_FORMAT";
/// 要求全体のタイムアウト秒数を受ける環境変数。
pub const TIMEOUT_SECS_ENV: &str = "PELWAY_LLM_TIMEOUT_SECS";
/// LLM を呼ぶ根拠の目的 ID を受ける環境変数（P01〜P08）。
pub const BASIS_ENV: &str = "PELWAY_LLM_BASIS";

/// 既定のチャット用モデル。GLM-5.3 Flash のモデルコード `glm-5.3-flash`
/// （Z.ai 公式一覧）。Coding Plan 向け endpoint は自前アプリからの呼び出しが
/// 認められないため使わない。
pub const DEFAULT_CHAT_MODEL: &str = "glm-5.3-flash";
/// 既定の埋め込み用モデル（デモでは未使用。OpenAiGateway の組み立てに要る）。
pub const DEFAULT_EMBED_MODEL: &str = "embedding-3";
/// 既定の提供元の表示名。
pub const DEFAULT_PROVIDER: &str = "zai";
/// 既定の要求全体タイムアウト（秒）。接続のタイムアウトは
/// [`GatewayOptions::default`] の 10 秒を使う。
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;
/// デモの LLM 根拠の既定（BYOK のため P06。hatai 決定 2026-11-16）。
pub const DEFAULT_BASIS: PurposeId = PurposeId::P06;

/// デモの LLM 接続設定。`from_env` / `from_vars` で組み立てる。
#[derive(Clone, Debug, PartialEq)]
pub struct DemoLlmConfig {
    /// 提供元への接続設定（モデル名・接続先・`response_format` の能力）。
    pub provider: ProviderConfig,
    /// キーを読む環境変数の名前（値そのものは持たない）。
    pub api_key_env: String,
    /// LLM を呼ぶ根拠の目的 ID（runner のゲートと seed に渡す）。
    pub llm_basis: PurposeId,
    /// 要求全体（応答まで）のタイムアウト。
    pub timeout: Duration,
}

impl DemoLlmConfig {
    /// 環境変数から組み立てる。
    pub fn from_env() -> Result<Self, String> {
        Self::from_vars(|name| std::env::var(name).ok())
    }

    /// `get` で環境変数を読む形（テストは写像を渡す）。値の形が合わないもの
    /// （秒数・目的 ID・`response_format`）は `Err`。キーの有無はエラーにしない。
    pub fn from_vars(get: impl Fn(&str) -> Option<String>) -> Result<Self, String> {
        let var = |name: &str| get(name).filter(|v| !v.trim().is_empty());
        let response_format = match var(RESPONSE_FORMAT_ENV).as_deref().unwrap_or("json_object") {
            "json_object" => ResponseFormatCapability::JsonObject,
            "json_schema" => ResponseFormatCapability::JsonSchema,
            other => {
                return Err(format!(
                    "{RESPONSE_FORMAT_ENV} には json_object か json_schema を指定してください: {other}"
                ));
            }
        };
        let timeout = match var(TIMEOUT_SECS_ENV) {
            Some(text) => {
                let secs = text
                    .parse::<u64>()
                    .ok()
                    .filter(|s| *s >= 1)
                    .ok_or_else(|| {
                        format!("{TIMEOUT_SECS_ENV} には 1 秒以上の秒数を指定してください: {text}")
                    })?;
                Duration::from_secs(secs)
            }
            None => Duration::from_secs(DEFAULT_TIMEOUT_SECS),
        };
        let llm_basis = match var(BASIS_ENV) {
            Some(text) => PurposeId::parse(&text)
                .ok_or_else(|| format!("{BASIS_ENV} には P01〜P08 を指定してください: {text}"))?,
            None => DEFAULT_BASIS,
        };
        Ok(Self {
            provider: ProviderConfig::new(
                var(PROVIDER_ENV).unwrap_or_else(|| DEFAULT_PROVIDER.to_owned()),
                var(BASE_URL_ENV).unwrap_or_else(|| pelway_llm::config::ZAI_BASE_URL.to_owned()),
                var(CHAT_MODEL_ENV).unwrap_or_else(|| DEFAULT_CHAT_MODEL.to_owned()),
                var(EMBED_MODEL_ENV).unwrap_or_else(|| DEFAULT_EMBED_MODEL.to_owned()),
                response_format,
            ),
            api_key_env: var(API_KEY_ENV_NAME).unwrap_or_else(|| API_KEY_ENV.to_owned()),
            llm_basis,
            timeout,
        })
    }

    /// キーが使えるときは実提供元のゲートウェイ、無い・使えない形のときは
    /// 台本モード（[`ScriptGateway`]）を返す。落ちない（起動はどちらでも進む）。
    pub fn build_gateway(&self) -> Arc<dyn DynChatGateway> {
        self.build_gateway_with(|name| std::env::var(name).ok())
    }

    /// `get` で環境変数を読む形（テストは写像を渡す）。
    pub fn build_gateway_with(
        &self,
        get: impl Fn(&str) -> Option<String>,
    ) -> Arc<dyn DynChatGateway> {
        if !key_is_usable(get(&self.api_key_env).as_deref()) {
            return Arc::new(ScriptGateway);
        }
        // var は &'static str。プロセスに1度だけ組み立てる設定のためリークで
        // 'static にする（dev-llm-eval と同じ手口）。
        let var: &'static str = Box::leak(self.api_key_env.clone().into_boxed_str());
        let options = GatewayOptions {
            timeout: self.timeout,
            https_only: self.provider.base_url.starts_with("https"),
            ..GatewayOptions::default()
        };
        match OpenAiGateway::new(
            &self.provider,
            Arc::new(FnoxEnvTokenSource { var }),
            &options,
        ) {
            Ok(gateway) => Arc::new(gateway),
            // 組み立てに失敗しても落ちずに台本モードで起動する。
            Err(_) => Arc::new(ScriptGateway),
        }
    }
}

/// キーの値が送信に使える形か（未設定・空白のみ・ヘッダーに載せられない文字を
/// 含むものは「使えない」とみなし台本モードへ落ちる）。値の正当性
/// （提供元に受け入れられるか）はここでは判断しない。
fn key_is_usable(value: Option<&str>) -> bool {
    match value {
        Some(v) => {
            let t = v.trim();
            !t.is_empty() && t.is_ascii() && !t.chars().any(|c| c.is_control())
        }
        None => false,
    }
}

/// 台本モードの gateway（フォールバック応答セット。#336 が切り替え操作と
/// 表示を担う）。実提供元には接続せず、runner が要求するスキーマに合う
/// 固定の応答を返す。
#[derive(Debug, Default)]
pub struct ScriptGateway;

/// 台本の A/B 案（合成）。実 LLM 失敗時のフォールバックにも使う（runner が
/// 「台本の応答で進めます」の進捗行とともに投稿する）。
pub(crate) const SCRIPT_AB: &str =
    r#"{"option_a": "納期を2週間延ばす案", "option_b": "範囲を絞って納期を守る案"}"#;
/// 台本のメール下書き（合成）。実 LLM 失敗時のフォールバックにも使う。
pub(crate) const SCRIPT_DRAFT: &str = r#"{"to": "B社 ご担当者", "subject": "案件B 納期のご相談", "body": "お世話になっております。案件Bの納期について、納期を2週間延ばす案で調整させていただけますでしょうか。"}"#;

impl LlmGateway for ScriptGateway {
    async fn chat(&self, req: ChatRequest) -> Result<ChatReply, LlmError> {
        // 要求スキーマで応答を選ぶ（下書き要求には下書きの台本を返す。required に
        // `subject` を含むものを下書き要求とみなす）。
        let is_draft = req
            .schema
            .as_ref()
            .and_then(|s| s.get("required"))
            .and_then(|r| r.as_array())
            .is_some_and(|a| a.iter().any(|v| v == "subject"));
        let text = if is_draft { SCRIPT_DRAFT } else { SCRIPT_AB };
        Ok(ChatReply {
            text: text.to_owned(),
            usage: Usage::default(),
        })
    }

    async fn embed(&self, _texts: Vec<String>) -> Result<EmbedReply, LlmError> {
        Err(LlmError::NotSent(
            "台本モードでは埋め込みを呼びません".to_owned(),
        ))
    }
}
