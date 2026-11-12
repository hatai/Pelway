//! トークンの供給。要求ごとに取り出し、将来の短時間トークン
//! （Vertex AI の1時間で切れるアクセストークン等）に差し替えられる形にする（ADR 0008）。

use crate::error::LlmError;

/// APIトークン。`Debug` に値を出さない（PR の受け入れ条件: トークンはログに出ない）。
/// 値の取り出しは [`Self::expose`] だけ。構造体に保持せず、使い捨てる。
#[derive(Clone, PartialEq, Eq)]
pub struct SecretToken(String);

impl SecretToken {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// トークンの値。ヘッダーへの埋め込みなど、送信直前だけ使う。
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for SecretToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretToken([redacted])")
    }
}

/// 呼出しのたびにトークンを1件返す供給源。
pub trait TokenSource: Send + Sync {
    /// トークンを返す。`Err` のとき要求は送られない（[`LlmError::NotSent`]）。
    fn token(&self) -> Result<SecretToken, LlmError>;
}

/// fnox が環境変数へ注入したトークンを読む供給源（MVP の単一テナント分、ADR 0008）。
///
/// fnox.toml の `PELWAY_LLM_API_KEY` に値を登録し、
/// `mise exec -- fnox exec -- <コマンド>` で実行すると環境変数が渡る
/// （docs/dev-environment.md「シークレットの扱い（fnox）」）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FnoxEnvTokenSource {
    /// 環境変数名。既定は `PELWAY_LLM_API_KEY`。
    pub var: &'static str,
}

impl FnoxEnvTokenSource {
    pub const DEFAULT_VAR: &'static str = "PELWAY_LLM_API_KEY";

    pub fn new() -> Self {
        Self {
            var: Self::DEFAULT_VAR,
        }
    }
}

impl TokenSource for FnoxEnvTokenSource {
    fn token(&self) -> Result<SecretToken, LlmError> {
        std::env::var(self.var).map(SecretToken::new).map_err(|_| {
            LlmError::NotSent(format!(
                "{} が設定されていません（fnox で注入する）",
                self.var
            ))
        })
    }
}

impl TokenSource for SecretToken {
    fn token(&self) -> Result<SecretToken, LlmError> {
        Ok(self.clone())
    }
}

impl TokenSource for String {
    fn token(&self) -> Result<SecretToken, LlmError> {
        Ok(SecretToken::new(self.clone()))
    }
}
