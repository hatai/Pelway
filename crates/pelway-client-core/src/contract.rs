//! バックエンド契約（[`pelway-contract`]）の再エクスポート。手作業の部分写しだった
//! `src/contract.rs` を、ずれの恐れの無い直接利用に置き換えた（Issue #91）。
//!
//! 提供: `approval` からコメント本文の検査（[`COMMENT_MAX_CHARS`]・
//! [`comment_len`]・[`validate_comment`]）、要求・応答の型（[`SaveCommentRequest`]・
//! [`CommentSaved`]・[`ApproveRequest`]・[`ApproveOutcome`]）、エラー
//! （[`ApproveError`]・[`CommentError`]）、経路の構築（[`comments_path`]・
//! [`approve_path`]）。`ids` から [`CardId`]・[`CommentId`]・[`SimulatedSendId`]・
//! [`IdempotencyKey`]。`task` から [`TaskState`]。
//!
//! serde derive（ワイヤ形式）は原本が持つ。エラーのタグ（`snake_case`）も derive が
//! 作り、`http_status()` の値は原本のまま。手書きの `code()` は持たない
//! （テストは直列化で確かめる）。

pub use pelway_contract::approval::{
    ApproveError, ApproveOutcome, ApproveRequest, COMMENT_MAX_CHARS, CommentError, CommentSaved,
    SaveCommentRequest, approve_path, comment_len, comments_path, validate_comment,
};
pub use pelway_contract::ids::{
    CardId, CommentId, IdempotencyKey, InvalidIdempotencyKey, SimulatedSendId,
};
pub use pelway_contract::task::TaskState;
