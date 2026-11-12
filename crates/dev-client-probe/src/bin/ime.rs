//! Issue #98 の slint #10861（IME 変換中にフォーカスが外れると、最後の文字が
//! 消える）の再現用プログラム。製品機能ではない。LineEdit 1 つだけの窓を出し、
//! テキストと入力の focused の変化を stdout に記録する。手順と判定は
//! docs/research/slint-tauri-client-measurements.md の 4 節にある。

use std::cell::RefCell;
use std::time::Duration;

use pelway_dev_client_probe::ImeWindow;
use slint::ComponentHandle;

fn main() {
    let window = ImeWindow::new().expect("ImeWindow の生成");
    let _ = window.show();

    // テキストとフォーカスの変化を記録する（callback が無いのでタイマーで見て、
    // 変化のときだけ出す）。
    let weak = window.as_weak();
    let last = RefCell::new((window.get_text(), window.get_input_has_focus()));
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(50),
        move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let now = (window.get_text(), window.get_input_has_focus());
            let mut last = last.borrow_mut();
            if now != *last {
                println!("text: {:?} input-has-focus: {}", now.0, now.1);
                *last = now;
            }
        },
    );

    let _ = slint::run_event_loop();
}
