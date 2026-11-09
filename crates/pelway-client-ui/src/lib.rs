//! Pelway デスクトップクライアントの UI 骨組み（段階A）。
//!
//! 段階Aは外部の crate に依存せず、`pelway_client_core` と標準ライブラリだけで
//! 画面の骨組みの土台を作る。Slint で窓を開くのは段階B（別指示）。
//!
//! - モジュール [`theme`]：テーマ（4種）とデザイントークンの名前。値は未確定のため入れない。
//! - モジュール [`i18n`]：画面の文言の Fluent キー。段階Bで Fluent から引く。
//! - モジュール [`state`]：画面の表示用の状態（Slint 非依存）と合成データ。
//! - モジュール [`bridge`]：中核ロジック（`input::on_key`・`ApprovalMachine`）へのつなぎ。
//! - モジュール [`testkit`]：テスト用の決まった値を返す `KeySource` と `Clock`。

#![forbid(unsafe_code)]

pub mod bridge;
pub mod i18n;
pub mod state;
pub mod testkit;
pub mod theme;
