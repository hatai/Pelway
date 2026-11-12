//! Issue #98 のクライアント技術の検証用クレート。製品機能ではない。
//! 2 つを確かめる: 可変高（行ごとに高さが違う）の仮想リストを Slint 1.18.1 が
//! どう扱うか（行の位置は [`Probe`] に記録する）、Rust の型から TypeScript の型を
//! 生成する道具（ts-rs と specta。生成結果は `bindings/` に置き、差分を検査）。

pub mod types;

/// Slint を使う UI 側の検証。WASM ビルドには Slint を入れない（同期処理の土台
/// だけを測るため）。
#[cfg(not(target_arch = "wasm32"))]
mod slint_probe {
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    slint::include_modules!();

    /// 行の位置の記録器。コールバックへ `Arc<Probe>` として渡す。
    #[derive(Debug, Default)]
    pub struct Probe {
        /// 行番号ごとの最後の y 座標（列表の座標系）。
        positions: Mutex<BTreeMap<u32, f32>>,
    }

    impl Probe {
        /// 新しい記録器を作る。
        #[must_use]
        pub fn new() -> Arc<Self> {
            Arc::new(Self::default())
        }

        /// 行番号ごとの最後の y 座標。
        #[must_use]
        pub fn positions(&self) -> BTreeMap<u32, f32> {
            self.positions.lock().expect("記録のロック").clone()
        }

        /// 記録を消す（次の測定を 0 から数える）。
        pub fn reset(&self) {
            self.positions.lock().expect("記録のロック").clear();
        }
    }

    /// 行の高さ。`ui/probe.slint` と同じ計算（97 行周期）。
    #[must_use]
    pub fn row_height(index: u32, variable_height: bool) -> f32 {
        if variable_height {
            20.0 + (index % 97) as f32
        } else {
            20.0
        }
    }

    /// 記録器に結線した検証用ウィンドウを作る。
    #[must_use]
    pub fn new_probe_window(probe: &Arc<Probe>) -> ProbeWindow {
        let window = ProbeWindow::new().expect("ProbeWindow の生成");
        let move_probe = Arc::clone(probe);
        window.on_record_move(move |index, y| {
            move_probe
                .positions
                .lock()
                .expect("記録のロック")
                .insert(index as u32, y);
        });
        window
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use slint_probe::{ImeWindow, Probe, ProbeWindow, new_probe_window, row_height};

/// WASM 向けの導出（同期処理の土台）。WASM ビルドのときだけ有効。JS 側との境界
/// は JSON 文字列で通す（Rust の構造体をそのまま境界に置くのは wasm-bindgen
/// では扱えないため）。
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
