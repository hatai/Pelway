//! エラーの分類。Pelway 03 の送信状態に合わせ、少なくとも
//! 「送っていない」「送ったが結果不明」「拒否」「応答が不正」の4つに分ける。
//!
//! 分類の基準（docs/research/rust-llm-client-comparison.md §2 の整理に従う）:
//! - `reqwest::Error::is_connect()` が真なら未送信が確定 → [`LlmError::NotSent`]
//! - それ以外の送信エラー・タイムアウトは結果不明と保守的に扱う → [`LlmError::OutcomeUnknown`]
//!   （タイムアウトは結果の確定ではなく、再送は許可しない: Pelway 03）
//! - 4xx/5xx は拒否。5xx も「相手が処理したか不明」なため自動再送はしない → [`LlmError::Rejected`]
//! - 応答の解釈失敗・スキーマ違反 → [`LlmError::Invalid`]

use rig_core::error::ProviderError;
use rig_core::http_client;

/// LLMゲートウェイの失敗。上の4分類のとおり。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LlmError {
    /// 送っていない（接続失敗、送信前の検査・予算・トークン取得の失敗）。
    NotSent(String),
    /// 送ったが結果不明（送信後のタイムアウト・切断）。自動再送はしない。
    OutcomeUnknown(String),
    /// 提供元の拒否（4xx/5xx）。
    Rejected { status: u16, body: String },
    /// 応答が不正（解釈失敗・スキーマ違反）。
    Invalid(String),
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmError::NotSent(m) => write!(f, "not sent: {m}"),
            LlmError::OutcomeUnknown(m) => write!(f, "outcome unknown: {m}"),
            LlmError::Rejected { status, body } => write!(f, "rejected: status {status}: {body}"),
            LlmError::Invalid(m) => write!(f, "invalid reply: {m}"),
        }
    }
}

impl std::error::Error for LlmError {}

impl From<ProviderError> for LlmError {
    fn from(err: ProviderError) -> Self {
        // 非2xx 応答（rig-reqwest がステータスと本文を保持したもの）。
        if let Some(status) = err.provider_response_status() {
            return LlmError::Rejected {
                status: status.as_u16(),
                body: err.provider_response_body().unwrap_or_default().to_string(),
            };
        }
        match &err {
            ProviderError::Http(inner) => {
                // トランスポート自身が非2xxを保持している形（通常は上で処理済み）。
                if let Some(status) = inner.non_success_status() {
                    return LlmError::Rejected {
                        status: status.as_u16(),
                        body: inner.non_success_body().unwrap_or_default().to_string(),
                    };
                }
                if let http_client::Error::Instance(inner) = &**inner {
                    // Pelway 側のミドルウェアが送信前に中断したもの（トークン取得失敗等）。
                    if inner.downcast_ref::<std::io::Error>().is_some() {
                        return LlmError::NotSent(err.to_string());
                    }
                    if let Some(req) = inner.downcast_ref::<reqwest::Error>() {
                        return if req.is_connect() {
                            LlmError::NotSent(err.to_string())
                        } else {
                            LlmError::OutcomeUnknown(err.to_string())
                        };
                    }
                }
                LlmError::OutcomeUnknown(err.to_string())
            }
            // 応答は受け取ったが解釈できなかった、または形状が守られていない。
            ProviderError::Json(_)
            | ProviderError::Response(_)
            | ProviderError::Provider(_)
            | ProviderError::Truncated
            | ProviderError::MismatchedDimensions { .. } => LlmError::Invalid(err.to_string()),
            // 送る前の段階で失敗したもの。
            ProviderError::Url(_) | ProviderError::UnsupportedOption(_) => {
                LlmError::NotSent(err.to_string())
            }
            // 送ったかどうか分からないものは保守的に結果不明へ。
            _ => LlmError::OutcomeUnknown(err.to_string()),
        }
    }
}
