//! Issue #98 の Slint 1.18.1 ListView 検証用クレート。製品機能ではない。
//!
//! 可変高（行ごとに高さが違う）の仮想リストを Slint 1.18.1 がどう扱うかを
//! 確かめる。行の位置は [`Probe`] に記録し、テストと計測プログラムが読む。
//! 測定手順と結果は `docs/research/slint-tauri-client-measurements.md` にまとめる。

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

slint::include_modules!();

/// 行の位置の記録。
#[derive(Debug, Default)]
pub struct Records {
    /// 行番号ごとの最後の y 座標（列表の座標系）。
    pub positions: BTreeMap<u32, f32>,
}

/// 記録の入れ物。コールバックへ `Arc<Probe>` として渡す。
#[derive(Debug, Default)]
pub struct Probe {
    inner: Mutex<Records>,
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
        self.inner.lock().expect("記録のロック").positions.clone()
    }

    /// 記録を消す。次の測定を 0 から数えるために使う。
    pub fn reset(&self) {
        self.inner.lock().expect("記録のロック").positions.clear();
    }
}

/// 行の高さ。`ui/probe.slint` の ProbeRow と同じ計算（97 行周期）。
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
            .inner
            .lock()
            .expect("記録のロック")
            .positions
            .insert(index as u32, y);
    });
    window
}
