//! 連携 gateway のスパイク実装（#96）。MCP 公式 Rust SDK `rmcp` を固定版 3.9.0 で
//! 使えるか確かめるための下書きで、確定スキーマではない。
//!
//! # 守ること（この crate が表す不変条件）
//!
//! - MCP ツールの annotations（`readOnlyHint` など）は **信用しない**（ADR 0004）。
//!   [`annotations::AnnotationHints`] として管理者の画面の手がかりにだけ使い、
//!   段階（[`tier::ActionTier`]）の判定には一切使わない。
//! - 段階は連携ごとの宣言（[`tier::TierDeclarations`]）から決める。段階が決まって
//!   いない操作は「取り消せない操作」として扱う（fail closed）。
//! - 実外部 I/O は無い（S1 検証前、U01）。
//!
//! rmcp 3.9.0 のライセンス状態（MIT→Apache-2.0 移行中、NOTICE ファイル無し）は
//! Cargo.toml の注記に記録してある（#96 の確認結果）。

#![forbid(unsafe_code)]

pub mod annotations;
pub mod tier;
pub mod tool;
