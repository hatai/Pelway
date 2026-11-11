//! 承認カードと承認 API の契約。03 §5（固定承認）と 04 §3 の `approvals/{id}/decide` を
//! デモ向けに「コメント保存」と「承認」の 2 つの別 API に分けたもの。
//!
//! # API（案）
//!
//! | method/path | 入力 | 動作 |
//! |---|---|---|
//! | `POST /v1/approvals/{card_id}/comments` | [`SaveCommentRequest`] | コメントを保存するだけ。状態・`card_version` を変えず、送信しない。新しい `snapshot_hash` を返す。送信承認待ちでないカードには 409 `not_awaiting_approval` |
//! | `POST /v1/approvals/{card_id}/approve` | [`ApproveRequest`] | 版・snapshot・冪等キーを検査し、疑似送信を 1 回だけ行う |
//!
//! # UI とサーバーの役割分担
//!
//! コメント欄での Enter（IME 変換中・確定後・変換なしのいずれも）は改行の挿入で、
//! 承認にも送信にもならない。承認は承認ボタンのクリック、またはボタンにフォーカスが
//! ある状態での Enter/Space だけ。メンション候補の Enter/Esc も送信しない。
//! **この区別は UI の防御**で、HTTP 要求からは Enter とクリックを区別できない。
//! サーバーが保証するのは次のことだけ：
//!
//! 1. `approve` が呼ばれない限り、何も送らない（コメント保存は送信状態へ移さない）。
//! 2. `approve` は `card_id`・`card_version`・`snapshot_hash` がすべて現在のカードと
//!    一致したときだけ受け付ける。コメント保存で `snapshot_hash` は変わるので、
//!    クライアントはコメント保存の応答 [`CommentSaved::snapshot_hash`] を承認に使う。
//! 3. 冪等キーは送信前に「処理中」として (`card_id`, `card_version`, `snapshot_hash`) に
//!    結び付けて記録し、送信が終わったら「完了」にする。
//!    - 同じキー・同じ内容の再送：完了なら最初と同じ結果を返し、2 回目の送信をしない。
//!      処理中なら [`ApproveError::ApprovalInProgress`]（409）で、送信しない。
//!    - 同じキー・違う内容：[`ApproveError::IdempotencyKeyReused`]（422）。前の結果は返さない。
//!    - 送信前の検査で失敗した場合（409 の版違い・snapshot 違いなど）は何も送っておらず、
//!      キーは記録しない（解放）。
//!
//! クライアントは承認ボタンを押すたびに新しい冪等キーを作り、通信の失敗で送り直すときだけ
//! 同じキーを使う。409 のあとはカードを読み直し、利用者がもう一度押したら新しいキーを作る。
//!
//! TODO(U06): 複数承認者・承認期限・担当交代時の再承認の規則は未決。ここでは単一承認者。
//! TODO(U01): 開始資格（5 秒）・attempt_started・結果不明への遷移は S1 検証前のため
//! 疑似送信では表さない。実外部 I/O は無効。

use crate::ids::{
    CardId, ChannelId, CommentId, IdempotencyKey, MemberId, SimulatedSendId, WorkspaceId,
};
use crate::member::MemberKind;
use crate::task::{ActionKind, TaskState};

/// コメント保存 API のパス。
pub const COMMENTS_PATH: &str = "/v1/approvals/{card_id}/comments";
/// 承認 API のパス。
pub const APPROVE_PATH: &str = "/v1/approvals/{card_id}/approve";

/// [`COMMENTS_PATH`] の `{card_id}` を差し替えて、要求に使う実際の経路を作る。
/// サーバー（`pelway-http`）の経路判定もテストもこの関数で経路を作り、
/// 文字列リテラルで書き写さない（定数と経路の一致を構造的に保つ）。
#[must_use]
pub fn comments_path(card_id: &CardId) -> String {
    COMMENTS_PATH.replace("{card_id}", card_id.as_str())
}

/// [`APPROVE_PATH`] の `{card_id}` を差し替えて、要求に使う実際の経路を作る。
#[must_use]
pub fn approve_path(card_id: &CardId) -> String {
    APPROVE_PATH.replace("{card_id}", card_id.as_str())
}

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

/// 承認カード。1 枚が 1 つの不変 `ActionSnapshot`（の現在版）を表す。
///
/// `card_version` は承認に関わる内容（payload の改訂、状態遷移）でだけ増える。
/// コメント保存では増えないが、`snapshot_hash` は変わる。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ApprovalCard {
    pub workspace_id: WorkspaceId,
    pub channel_id: ChannelId,
    pub card_id: CardId,
    pub card_version: u64,
    /// 固定 payload（宛先・本文・添付 hash・scope 等）の hash。
    pub payload_hash: String,
    /// 承認者が承認する値。`payload_hash` とコメントから
    /// [`crate::snapshot::compute_snapshot_hash`] で計算する。
    pub snapshot_hash: String,
    pub action_kind: ActionKind,
    /// 表示用の宛先（合成データのみ）。
    pub recipient: String,
    /// 表示用の本文（合成データのみ）。
    pub body: String,
    pub state: TaskState,
    pub comments: Vec<CardComment>,
    /// 疑似送信済みなら、その安定 ID。
    pub simulated_send_id: Option<SimulatedSendId>,
}

/// カードに付いたコメント。`snapshot_hash` の計算に含まれる。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CardComment {
    pub comment_id: CommentId,
    pub author_id: MemberId,
    pub member_kind: MemberKind,
    /// 改行（Enter で挿入されたもの）をそのまま含む。
    pub body: String,
}

/// `POST /v1/approvals/{card_id}/comments` の入力。書き手は認証済みセッションから決める。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SaveCommentRequest {
    pub card_id: CardId,
    pub body: String,
}

/// コメント保存の結果。状態と版は保存前と同じ値、`snapshot_hash` は保存後の新しい値。
/// クライアントは承認にこの `snapshot_hash` と `card_version` を使う。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CommentSaved {
    pub card_id: CardId,
    pub comment_id: CommentId,
    pub card_version: u64,
    pub snapshot_hash: String,
    pub state: TaskState,
}

/// `POST /v1/approvals/{card_id}/approve` の入力。4 項目すべて必須。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "code", rename_all = "snake_case"))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "code", rename_all = "snake_case"))]
pub enum CommentError {
    CardNotFound,
    EmptyBody,
    TooLong {
        max_chars: usize,
    },
    /// 送信承認待ちではない（送信中・送信済み等）。コメント保存は
    /// `snapshot_hash` を変えるので、予約・送信記録と食い違うのを防ぐために断る。
    NotAwaitingApproval {
        state: TaskState,
    },
}

impl CommentError {
    /// HTTP ステータスへの対応。
    #[must_use]
    pub fn http_status(&self) -> u16 {
        match self {
            Self::CardNotFound => 404,
            Self::EmptyBody | Self::TooLong { .. } => 422,
            Self::NotAwaitingApproval { .. } => 409,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comment_len_counts_code_points() {
        assert_eq!(comment_len(""), 0);
        assert_eq!(comment_len("abc"), 3);
        // CJK は 1 文字 1 コードポイント（UTF-8 では 3 バイト）。
        assert_eq!(comment_len("合成データ"), 5);
        // 絵文字 1 つ（U+1F600、UTF-8 では 4 バイト）。
        assert_eq!(comment_len("\u{1F600}"), 1);
        // e + 結合用アキュート（見た目は 1 文字、コードポイントは 2）。
        assert_eq!(comment_len("e\u{0301}"), 2);
        // ZWJ でつないだ家族の絵文字：男・ZWJ・女・ZWJ・女の子 = 5。
        assert_eq!(
            comment_len("\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}"),
            5
        );
        // 国旗（地域指示子 2 つ）。
        assert_eq!(comment_len("\u{1F1EF}\u{1F1F5}"), 2);
        // 改行も 1。
        assert_eq!(comment_len("1行目\n2行目"), 7);
    }

    #[test]
    fn validate_comment_boundaries() {
        for unit in ["a", "あ", "\u{1F600}", "\u{0301}"] {
            let exact = format!("x{}", unit.repeat(COMMENT_MAX_CHARS - 1));
            assert_eq!(comment_len(&exact), COMMENT_MAX_CHARS, "unit={unit:?}");
            assert_eq!(validate_comment(&exact), Ok(()), "unit={unit:?}");
            let over = format!("{exact}{unit}");
            assert_eq!(comment_len(&over), COMMENT_MAX_CHARS + 1);
            assert_eq!(
                validate_comment(&over),
                Err(CommentError::TooLong {
                    max_chars: COMMENT_MAX_CHARS
                }),
                "unit={unit:?}"
            );
        }
        // バイト数では上限の 4 倍でも、コードポイント数が 4000 なら通る。
        let emoji = "\u{1F600}".repeat(COMMENT_MAX_CHARS);
        assert_eq!(emoji.len(), COMMENT_MAX_CHARS * 4);
        assert_eq!(validate_comment(&emoji), Ok(()));
        // 結合文字列：e + U+0301 を 2000 組 = 4000 コードポイントで通り、1 つ足すと超える。
        let combining = "e\u{0301}".repeat(COMMENT_MAX_CHARS / 2);
        assert_eq!(validate_comment(&combining), Ok(()));
        assert!(validate_comment(&format!("{combining}e")).is_err());
        // ZWJ 家族絵文字 800 個 = 4000 コードポイント。
        let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}".repeat(800);
        assert_eq!(comment_len(&family), 4000);
        assert_eq!(validate_comment(&family), Ok(()));
        assert!(validate_comment(&format!("{family}\u{200D}")).is_err());
    }

    #[test]
    fn validate_comment_rejects_blank() {
        for blank in ["", " ", "\n", " \n\t ", "\u{3000}"] {
            assert_eq!(validate_comment(blank), Err(CommentError::EmptyBody));
        }
        assert_eq!(validate_comment("\n了解\n"), Ok(()));
    }

    #[test]
    fn path_builders_replace_the_card_id_segment() {
        let card = CardId::from("card-1");
        assert_eq!(comments_path(&card), "/v1/approvals/card-1/comments");
        assert_eq!(approve_path(&card), "/v1/approvals/card-1/approve");
        // テンプレートのプレフィックスは仕様 04 §3 の approvals の下。
        assert!(comments_path(&card).starts_with("/v1/approvals/"));
        assert!(approve_path(&card).starts_with("/v1/approvals/"));
    }
}
