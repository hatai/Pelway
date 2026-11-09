// 4564e88 の写し。統合のときに差し替える
//! 承認 API 契約の仮写し。原本はブランチ `demo-t3/backend` の commit 4564e88
//! `crates/pelway-contract`（`src/approval.rs`・`src/ids.rs`・`src/task.rs`）。
//! 統合時にこのファイルを差し替える。
//!
//! 写した範囲: コメント本文の検査（[`COMMENT_MAX_CHARS`]・[`comment_len`]・
//! [`validate_comment`]）、承認 API のエラー（[`ApproveError`]）とエラーコード、
//! 状態機械が必要とする要求・応答型と ID。値と数え方（コードポイント数）は変えない。
//!
//! この crate は外部依存を持たないため、原本の serde derive は外してある。
//! ワイヤ上のエラーコード（serde タグ `snake_case` の値）は [`ApproveError::code`]
//! が同じ文字列を返す。

use std::fmt;

// ---- 原本 `src/task.rs` から ----

/// タスクの状態。03 §3 の 9 状態。翻訳キーと UI 表示はここへ対応付ける。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskState {
    Requested,
    Working,
    AwaitingDecision,
    AwaitingSendApproval,
    Sending,
    SentAwaitingReply,
    OutcomeUnknown,
    Stopped,
    Done,
}

impl TaskState {
    /// 送信済み（または送信の途中）とみなす状態か。コメント保存はこの状態へ移さない。
    #[must_use]
    pub fn is_send_side(self) -> bool {
        matches!(
            self,
            Self::Sending | Self::SentAwaitingReply | Self::OutcomeUnknown
        )
    }
}

// ---- 原本 `src/ids.rs` から必要な分 ----

macro_rules! string_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
    /// 承認カード ID。カードは 1 つの `ActionSnapshot` を表す。
    CardId
);
string_id!(
    /// コメント ID（サーバー採番）。
    CommentId
);
string_id!(
    /// 疑似送信にサーバーが付ける安定 ID。同じ承認の再送（冪等リプレイ）では同じ値を返す。
    SimulatedSendId
);

/// 承認の冪等キー。クライアントが承認操作ごとに 1 つ生成する（例：UUID v4）。
///
/// 16〜128 文字の `[A-Za-z0-9_-]` だけを受け付ける。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

// ---- 原本 `src/approval.rs` から ----

/// コメント本文の上限。単位は Unicode のコードポイント数（Rust の `str::chars().count()`）。
/// 数え方は [`comment_len`] に一本化する。サーバーもクライアントもこの定数と関数を使う。
pub const COMMENT_MAX_CHARS: usize = 4000;

/// コメント本文の長さ（Unicode コードポイント数）。
///
/// 書記素（見た目の 1 文字）ではない。例：`"é"` を `e` + 結合用アクセントで書くと 2、
/// ZWJ でつないだ家族の絵文字は構成するコードポイントの数になる。改行も 1 と数える。
#[must_use]
pub fn comment_len(body: &str) -> usize {
    body.chars().count()
}

/// コメント本文を検査する。空白だけは [`CommentError::EmptyBody`]、
/// [`comment_len`] が [`COMMENT_MAX_CHARS`] を超えたら [`CommentError::TooLong`]。
///
/// # Errors
/// 上のとおり。
pub fn validate_comment(body: &str) -> Result<(), CommentError> {
    if body.trim().is_empty() {
        return Err(CommentError::EmptyBody);
    }
    if comment_len(body) > COMMENT_MAX_CHARS {
        return Err(CommentError::TooLong {
            max_chars: COMMENT_MAX_CHARS,
        });
    }
    Ok(())
}

/// `POST /v1/approvals/{card_id}/comments` の入力。書き手は認証済みセッションから決める。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveCommentRequest {
    pub card_id: CardId,
    pub body: String,
}

/// コメント保存の結果。状態と版は保存前と同じ値、`snapshot_hash` は保存後の新しい値。
/// クライアントは承認にこの `snapshot_hash` と `card_version` を使う。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentSaved {
    pub card_id: CardId,
    pub comment_id: CommentId,
    pub card_version: u64,
    pub snapshot_hash: String,
    pub state: TaskState,
}

/// `POST /v1/approvals/{card_id}/approve` の入力。4 項目すべて必須。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApproveRequest {
    pub card_id: CardId,
    /// 承認者が画面で見ていた版。現在の版と違えば 409。
    pub card_version: u64,
    /// 承認者が画面で見ていた snapshot の hash。現在の値と違えば 409。
    pub snapshot_hash: String,
    pub idempotency_key: IdempotencyKey,
}

impl ApproveRequest {
    /// パスの `{card_id}` と本文の `card_id` が一致するか検査する。
    ///
    /// # Errors
    /// 一致しなければ [`ApproveError::CardIdMismatch`]。
    pub fn check_path(&self, path_card_id: &CardId) -> Result<(), ApproveError> {
        if &self.card_id == path_card_id {
            Ok(())
        } else {
            Err(ApproveError::CardIdMismatch)
        }
    }
}

/// 承認の結果。冪等リプレイでは最初と同じ値を返す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApproveOutcome {
    pub card_id: CardId,
    /// 承認による遷移後の版。
    pub card_version: u64,
    /// 疑似送信後は常に [`TaskState::SentAwaitingReply`]。
    pub state: TaskState,
    pub simulated_send_id: SimulatedSendId,
}

/// 承認 API のエラー。どれも新たな送信を起こさない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApproveError {
    CardNotFound,
    CardIdMismatch,
    /// 画面の版が古い。最新の版を読み直してから承認し直す。
    VersionConflict {
        current_version: u64,
    },
    /// 画面の snapshot が古い（コメント保存や payload 改訂のあと）。読み直してから承認し直す。
    SnapshotMismatch,
    /// 送信承認待ちではない（承認済み・送信済み・止まっている等）。
    NotAwaitingApproval {
        state: TaskState,
    },
    /// 同じ冪等キー・同じ内容の承認がまだ処理中。送信はしていない（この要求では）。
    /// 少し待って同じキーで送り直すと、完了後は最初の結果が返る。
    ApprovalInProgress,
    /// 同じ冪等キーが別の内容（card_id・card_version・snapshot_hash のどれか）に使われた。
    IdempotencyKeyReused,
}

impl ApproveError {
    /// ワイヤ上のエラーコード。原本の serde タグ（`snake_case`）と同じ値。
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::CardNotFound => "card_not_found",
            Self::CardIdMismatch => "card_id_mismatch",
            Self::VersionConflict { .. } => "version_conflict",
            Self::SnapshotMismatch => "snapshot_mismatch",
            Self::NotAwaitingApproval { .. } => "not_awaiting_approval",
            Self::ApprovalInProgress => "approval_in_progress",
            Self::IdempotencyKeyReused => "idempotency_key_reused",
        }
    }

    /// HTTP ステータスへの対応。
    #[must_use]
    pub fn http_status(&self) -> u16 {
        match self {
            Self::CardNotFound => 404,
            Self::CardIdMismatch | Self::IdempotencyKeyReused => 422,
            Self::VersionConflict { .. }
            | Self::SnapshotMismatch
            | Self::NotAwaitingApproval { .. }
            | Self::ApprovalInProgress => 409,
        }
    }
}

/// コメント保存 API のエラー。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommentError {
    CardNotFound,
    EmptyBody,
    TooLong { max_chars: usize },
}

impl CommentError {
    /// HTTP ステータスへの対応。
    #[must_use]
    pub fn http_status(&self) -> u16 {
        match self {
            Self::CardNotFound => 404,
            Self::EmptyBody | Self::TooLong { .. } => 422,
        }
    }
}
