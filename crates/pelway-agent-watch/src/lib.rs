//! T3 のスレッドを見張り、状態の変化を Grok Bot に通知するための判定ロジック。
//! 外部コマンドを呼ぶ処理は `main.rs` 側に置き、ここには純粋関数だけを置く。

pub mod parse;

pub use parse::{extract_report, issue_number, parse_utc_epoch};
