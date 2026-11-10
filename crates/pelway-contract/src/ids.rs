//! ID の newtype。ワイヤ上はただの文字列。

use std::fmt;

macro_rules! string_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(transparent))]
        pub struct $name(pub String);

        impl $name {
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }
    };
}

string_id!(
    /// 会社（テナント）の主キー。04 §2 の `workspace_id`。
    WorkspaceId
);
string_id!(
    /// チャンネル ID。
    ChannelId
);
string_id!(
    /// メッセージ ID（サーバー採番）。
    MessageId
);
string_id!(
    /// チャンネルのメンバー ID（人または AI）。
    MemberId
);
string_id!(
    /// クライアントが付ける送信 ID。04 §2：同じ `client_msg_id` で本文が違えば 409。
    ClientMsgId
);
string_id!(
    /// 承認カード ID。カードは 1 つの `ActionSnapshot` を表す。
    CardId
);
string_id!(
    /// コメント ID（サーバー採番）。
    CommentId
);
string_id!(
    /// 疑似送信にサーバーが付ける安定 ID。同じ承認の再送（冪等リプレイ）では同じ値を返す。
    /// 実外部の提供元 ID ではない。
    SimulatedSendId
);
string_id!(
    /// 根拠記録 ID（サーバー採番）。12 §3.2 `communication_basis_records` の `basis_id`。
    BasisId
);
string_id!(
    /// 明示メモリの ID（サーバー採番）。
    MemoryId
);
string_id!(
    /// 削除・制限記録の ID（サーバー採番）。
    DeletionRestrictionId
);

/// 承認の冪等キー。クライアントが承認操作ごとに 1 つ生成する（例：UUID v4）。
///
/// 16〜128 文字の `[A-Za-z0-9_-]` だけを受け付ける。
/// 03 §5 の「固定承認版の処理 key」に当たり、提供元へ渡す冪等 key（logical operation に固定）
/// とは別物。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "String", into = "String"))]
pub struct IdempotencyKey(String);

/// 冪等キーの書式エラー。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidIdempotencyKey;

impl fmt::Display for InvalidIdempotencyKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("idempotency_key must be 16-128 chars of [A-Za-z0-9_-]")
    }
}

impl std::error::Error for InvalidIdempotencyKey {}

impl IdempotencyKey {
    pub const MIN_LEN: usize = 16;
    pub const MAX_LEN: usize = 128;

    /// 書式を検査して作る。
    ///
    /// # Errors
    /// 長さまたは文字種が範囲外なら [`InvalidIdempotencyKey`]。
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidIdempotencyKey> {
        let value = value.into();
        let len_ok = (Self::MIN_LEN..=Self::MAX_LEN).contains(&value.len());
        let chars_ok = value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
        if len_ok && chars_ok {
            Ok(Self(value))
        } else {
            Err(InvalidIdempotencyKey)
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for IdempotencyKey {
    type Error = InvalidIdempotencyKey;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<IdempotencyKey> for String {
    fn from(value: IdempotencyKey) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::IdempotencyKey;

    #[test]
    fn idempotency_key_accepts_uuid_like_values() {
        assert!(IdempotencyKey::new("3f1c2a9e-7b4d-4e21-9a0f-1c2d3e4f5a6b").is_ok());
    }

    #[test]
    fn idempotency_key_rejects_short_long_and_odd_chars() {
        assert!(IdempotencyKey::new("short").is_err());
        assert!(IdempotencyKey::new("a".repeat(129)).is_err());
        assert!(IdempotencyKey::new("has space in the key!!").is_err());
        assert!(IdempotencyKey::new("").is_err());
    }
}
