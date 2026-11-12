//! `pelway-llm` の単体試験（外部との通信なし）。
//! 公開 API と純粋ロジック（設定・トークン・エラー分類・スキーマ検証）を確かめる。
//! rig-core を使った接続の結合試験は tests/gateway.rs（後続の差分）が担う。

use std::sync::Mutex;

use pelway_llm::config::{ProviderConfig, ResponseFormatCapability};
use pelway_llm::token::{FnoxEnvTokenSource, SecretToken, TokenSource};
use pelway_llm::{LlmError, SecretToken as ReexportedSecretToken};
use rig_core::error::ProviderError;
use rig_core::{ProviderResponseError, http_client::StatusCode};

/// 設定例（Z.ai・OpenAI）は必須項目を満たす。
#[test]
fn provider_config_examples() {
    let zai = ProviderConfig::zai_example();
    assert_eq!(zai.base_url, "https://api.z.ai/api/paas/v4");
    assert_eq!(zai.response_format, ResponseFormatCapability::JsonObject);
    assert!(!zai.chat_model.is_empty());
    assert!(!zai.embed_model.is_empty());
    let openai = ProviderConfig::openai_example();
    assert_eq!(openai.base_url, "https://api.openai.com/v1");
    assert_eq!(openai.response_format, ResponseFormatCapability::JsonSchema);
    // カスタム提供元も同じ形で受け取れる。
    let custom = ProviderConfig::new(
        "mock",
        "http://127.0.0.1:9/v1",
        "chat",
        "embed",
        ResponseFormatCapability::JsonSchema,
    );
    assert_eq!(custom.name, "mock");
}

/// トークンは Debug 出力に出ない。
#[test]
fn secret_token_never_appears_in_debug() {
    let secret = SecretToken::new("sk-secret-do-not-print-1234");
    let debug = format!("{secret:?}");
    assert!(debug.contains("redacted"), "{debug}");
    assert!(!debug.contains("sk-secret-do-not-print-1234"), "{debug}");
    // タプルや構造体に入れても同じ（Debug の伝播）。
    let wrapped = format!("{:?}", (1, secret.clone()));
    assert!(
        !wrapped.contains("sk-secret-do-not-print-1234"),
        "{wrapped}"
    );
    // 値そのものは expose で取り出せる。
    assert_eq!(secret.expose(), "sk-secret-do-not-print-1234");
    // 再エクスポートも同じ型。
    let _: ReexportedSecretToken = ReexportedSecretToken::new("x");
}

/// fnox の環境変数（PELWAY_LLM_API_KEY）からトークンを取る。
/// 環境変数はプロセス全体に効くため、変更は排他する。
#[test]
fn fnox_env_token_source_reads_pelway_llm_api_key() {
    static ENV_LOCK: Mutex<()> = Mutex::new(());
    let _guard = ENV_LOCK.lock().unwrap();

    let source = FnoxEnvTokenSource::new();
    assert_eq!(source.var, "PELWAY_LLM_API_KEY");
    // 未設定なら未送信として失敗する。
    unsafe { std::env::remove_var("PELWAY_LLM_API_KEY") };
    assert!(matches!(source.token(), Err(LlmError::NotSent(_))));

    unsafe { std::env::set_var("PELWAY_LLM_API_KEY", "env-token-value") };
    let token = source.token().unwrap();
    assert_eq!(token.expose(), "env-token-value");
    unsafe { std::env::remove_var("PELWAY_LLM_API_KEY") };
}

/// エラーの分類: 拒否（ProviderResponse）はステータスと本文を保つ。
#[test]
fn error_mapping_rejected_keeps_status_and_body() {
    let error = ProviderError::ProviderResponse(ProviderResponseError::new(
        StatusCode::TOO_MANY_REQUESTS,
        "rate limited",
    ));
    match LlmError::from(error) {
        LlmError::Rejected { status, body } => {
            assert_eq!(status, 429);
            assert_eq!(body, "rate limited");
        }
        other => panic!("Rejected になるべき: {other:?}"),
    }
}

/// エラーの分類: 応答の解釈失敗は「応答が不正」。
#[test]
fn error_mapping_invalid_for_parse_and_shape_failures() {
    let json_error: serde_json::Error = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
    assert!(matches!(
        LlmError::from(ProviderError::Json(std::sync::Arc::new(json_error))),
        LlmError::Invalid(_)
    ));
    assert!(matches!(
        LlmError::from(ProviderError::Response("missing usage".to_string())),
        LlmError::Invalid(_)
    ));
    assert!(matches!(
        LlmError::from(ProviderError::Truncated),
        LlmError::Invalid(_)
    ));
}

/// エラーの分類: 送る前の段階の失敗は「送っていない」。
#[test]
fn error_mapping_not_sent_for_pre_send_failures() {
    let url_error = "::::".parse::<url::Url>().unwrap_err();
    assert!(matches!(
        LlmError::from(ProviderError::Url(url_error)),
        LlmError::NotSent(_)
    ));
}

/// スキーマ検証（部分実装）の確認。
#[test]
fn schema_validation_subset() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {"n": {"type": "integer"}, "tag": {"enum": ["a", "b"]}},
        "required": ["n"],
        "additionalProperties": false
    });
    assert!(
        pelway_llm::schema::validate(&serde_json::json!({"n": 3, "tag": "a"}), &schema).is_ok()
    );
    // JSON Schema では整数値の 5.0 も integer とみなす。
    assert!(pelway_llm::schema::validate(&serde_json::json!({"n": 5.0}), &schema).is_ok());
    assert!(pelway_llm::schema::validate(&serde_json::json!({"n": 1.5}), &schema).is_err());
    assert!(pelway_llm::schema::validate(&serde_json::json!({"tag": "a"}), &schema).is_err());
    assert!(pelway_llm::schema::validate(&serde_json::json!({"n": 1, "x": 2}), &schema).is_err());
    assert!(
        pelway_llm::schema::validate(&serde_json::json!({"n": 1, "tag": "c"}), &schema).is_err()
    );
    // 配列と複数 type。
    let array_schema = serde_json::json!({"type": "array", "items": {"type": "string"}});
    assert!(pelway_llm::schema::validate(&serde_json::json!(["a", "b"]), &array_schema).is_ok());
    assert!(pelway_llm::schema::validate(&serde_json::json!([1]), &array_schema).is_err());
}

/// 未対応の type 名はスキーマエラーになる（fail-closed）。
/// `type_matches` が未知の名前に常に一致すると、誤記を含むスキーマが
/// すべての値を通してしまう（PR #163 の must-fix）。
#[test]
fn schema_validation_rejects_unknown_type_names() {
    let typo = serde_json::json!({"type": "not-a-type", "properties": {}});
    assert!(pelway_llm::schema::validate(&serde_json::json!("x"), &typo).is_err());
    let partially_unknown = serde_json::json!({"type": ["string", "also-not-a-type"]});
    assert!(pelway_llm::schema::validate(&serde_json::json!("x"), &partially_unknown).is_err());
    // 入れ子のスキーマでも同じように拒否する。
    let nested = serde_json::json!({
        "type": "object",
        "properties": {"a": {"type": "unknown"}},
        "required": ["a"]
    });
    assert!(pelway_llm::schema::validate(&serde_json::json!({"a": true}), &nested).is_err());
}
