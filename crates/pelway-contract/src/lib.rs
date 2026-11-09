//! Pelway デモ用バックエンドの API 契約型（下書き）。
//!
//! 根拠は `docs/technical/v0.7/body/03-ai-bot-runtime.md`（9状態・固定承認）と
//! `docs/technical/v0.7/body/04-data-api-realtime.md`（データ・API）。
//! 確定スキーマではなく、11/10 12:00 の契約案のための下書き。
//!
//! # 守ること（この crate が表す不変条件）
//!
//! - サーバーは **承認 API（approve）が呼ばれない限り何も送らない**。
//!   コメント保存は送信状態へ一切遷移しない。
//! - 承認には `card_id`・`card_version`・`snapshot_hash`・`idempotency_key` が必須。
//! - `snapshot_hash` は固定 payload の hash とコメントから計算する。コメントを保存すると
//!   `snapshot_hash` が変わり（`card_version` は変わらない）、保存前の hash での承認は拒否する。
//! - 冪等キーは送信前に「処理中」として (`card_id`, `card_version`, `snapshot_hash`) に
//!   結び付けて記録し、送信が終わったら「完了」にする。何も送っていない失敗のときだけ解放する。
//! - 疑似送信の結果はサーバーが採番した安定 ID（[`ids::SimulatedSendId`]）を持ち、
//!   状態は [`task::TaskState::SentAwaitingReply`] に対応付ける。
//! - 実外部 I/O は無い。疑似送信はメモリ上の記録だけ（S1 検証前、U01）。
//!
//! Enter キーとクリックの区別は UI 側の防御で、サーバーには区別できない。
//! サーバーが保証するのは「明示的な approve 呼び出しなしに送信しない」ことだけ。

#![forbid(unsafe_code)]

pub mod approval;
pub mod ids;
pub mod ledger;
pub mod member;
pub mod message;
pub mod snapshot;
pub mod task;
