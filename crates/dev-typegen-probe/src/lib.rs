//! Issue #98 の型生成の検証用クレート。製品機能ではない。
//!
//! A2 は「同期を Rust/WASM、UI を TS/React DOM とする」方針なので、Rust の型から
//! TypeScript の型を生成する道具（ts-rs と specta）を試し、型生成の維持費が
//! どれだけになるかを確かめる。生成結果は `bindings/` に置き、テストで差分を
//! 検査する（Rust の型を変えたら TS の型も変わることを CI で確かめられる形）。

pub mod types;

/// WASM 向けの导出（同期処理の土台）。WASM ビルドのときだけ有効。
///
/// JS 側との境界は JSON 文字列で通す（Rust の構造体をそのまま境界に置くのは
/// wasm-bindgen では扱えないため）。
#[cfg(target_arch = "wasm32")]
pub mod wasm {
    use wasm_bindgen::prelude::*;

    use crate::types::Message;

    /// メッセージ 1 件の JSON を受け取り、整形し直した JSON を返す。
    #[wasm_bindgen]
    pub fn message_roundtrip(message_json: &str) -> Result<String, JsError> {
        let message: Message =
            serde_json::from_str(message_json).map_err(|e| JsError::new(&e.to_string()))?;
        serde_json::to_string(&message).map_err(|e| JsError::new(&e.to_string()))
    }
}

/// 同期の往復（JSON 化と復元）が型どおりに動くことを確かめるための補助。
#[cfg(feature = "serde")]
pub fn roundtrip(message: &types::Message) -> types::Message {
    let json = serde_json::to_string(message).expect("メッセージの JSON 化");
    serde_json::from_str(&json).expect("メッセージの復元")
}
