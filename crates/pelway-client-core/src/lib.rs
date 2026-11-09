//! Pelway デスクトップクライアントの中核ロジック（UI 非依存）。
//!
//! Slint にも HTTP クライアントにも依存しない純粋な Rust のロジック:
//! - [`contract`](crate::contract): バックエンド契約の仮写し（4564e88）
//! - [`input`](crate::input): キー操作の判定（QA-APR-01 / QA-MEN-01）
//! - [`approval`](crate::approval): 承認カードの状態機械（「次に何をするか」を返す。通信は書かない）

#![forbid(unsafe_code)]

pub mod approval;
pub mod contract;
pub mod input;
