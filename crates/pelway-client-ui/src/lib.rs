//! Pelway デスクトップクライアントの UI 骨組み（段階B-2）。
//!
//! 段階B-1: `slint` 1.18.1 に依存し、build.rs が `ui/main.slint` をコンパイルする。
//! 段階B-2: トークンの値（色）を正本 JSON から読んで `DesignTokens` へ設定する。
//! 窓を開いての確認・Fluent の crate による文言の取得・
//! accessible-role / accessible-label は、この回ではまだ行わない。
//!
//! - モジュール [`theme`]:テーマ（4種）とデザイントークン。名前も値も正本
//!   `tokens/pelway.tokens.json` から読む（`.rs`・`.slint`・`.ftl` に色の値は書かない）。
//! - モジュール [`i18n`]:画面の文言の Fluent キー。Fluent の crate は未導入。
//! - モジュール [`state`]:画面の表示用の状態（Slint 非依存）と合成データ。
//! - モジュール [`bridge`]:中核ロジック（`input::on_key`・`ApprovalMachine`）へのつなぎ。
//! - モジュール [`testkit`]:テスト用の決まった値を返す `KeySource` と `Clock`。
//!
//! この下の `slint::include_modules!()` が `ui/main.slint` のコンパイル結果
//! （`MainWindow` など）をこのクレートに読み込む。Slint はこのクレートの中だけで使う。
//!
//! unsafe について: 手書きのコードは禁止のまま（`deny`）。Slint が生成した
//! コードには unsafe が含まれ、生成物内部の `allow` で自分自身を許可して
//! いるため、クレート全体は `forbid` では無く `deny` にしている。

#![deny(unsafe_code)]

pub mod bridge;
pub mod i18n;
pub mod state;
pub mod testkit;
pub mod theme;

slint::include_modules!();
