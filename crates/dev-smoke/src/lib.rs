//! 開発環境確認用のスケルトンクレート。製品機能は含まない。
//!
//! 目的は、`cargo fmt` / `check` / `clippy` / `test` / `build` の経路と、
//! mise・hk・mr boxington の連携が壊れていないことを確かめること。

/// 開発環境のビルドとテストの経路が通っていることを示す。
#[must_use]
pub fn dev_environment_ok() -> bool {
    true
}
