//! Issue #98 の slint #3867（読み上げを有効にすると大きな ListView が遅い）の
//! 再現と実測のための計測プログラム。製品機能ではない。
//!
//! 使い方:
//! ```text
//! cargo run --release -p pelway-dev-slint-listview --bin perf -- \
//!     --rows 100000 --events 1000 --interval 5 --static-cells 1000
//! ```
//!
//! 読み上げ（アクセシビリティ）を有効にしたビルドと無効にしたビルドで同じ
//! 条件を実行し、CPU 時間の差を比べる。有効なビルドは、セッションの dbus に
//! `org.a11y.Status.IsEnabled` が true になっていることが前提（AccessKit は
//! 読み上げ支援の利用が求められているときだけアクセシビリティ木を作る）。
//!
//! 測定手順と結果は docs/research/slint-tauri-client-measurements.md にまとめる。

use std::time::{Duration, Instant};

use i_slint_backend_testing::ElementRoot;
use pelway_dev_slint_listview::ProbeWindow;
use slint::ComponentHandle;

/// プロセスが消費した CPU 時間（user + sys）。
fn cpu_time() -> Duration {
    // CLOCK_PROCESS_CPUTIME_ID（100ns 単位）。
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    if unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut ts) } == 0 {
        Duration::new(ts.tv_sec as u64, ts.tv_nsec as u32)
    } else {
        Duration::ZERO
    }
}

fn main() {
    // 引数（既定値つき）。
    let mut rows = 100_000;
    let mut events = 2_000;
    let mut interval = 5;
    let mut static_cells = 0;
    let mut iter = std::env::args().skip(1);
    while let Some(flag) = iter.next() {
        let value = iter.next().unwrap_or_default();
        match flag.as_str() {
            "--rows" => rows = value.parse().unwrap_or(rows),
            "--events" => events = value.parse().unwrap_or(events),
            "--interval" => interval = value.parse().unwrap_or(interval),
            "--static-cells" => static_cells = value.parse().unwrap_or(static_cells),
            _ => eprintln!("未知の引数: {flag}"),
        }
    }

    let window = ProbeWindow::new().expect("ProbeWindow の生成");
    window.set_item_count(rows);
    window.set_static_cells(static_cells);
    window.set_variable_height(false);

    // スクロールイベントを送る対象（ListView）。
    let list = window
        .root_element()
        .query_descendants()
        .match_type_name("ListView")
        .find_first()
        .expect("ProbeWindow に ListView がある");
    let _ = window.show();

    let cpu_start = cpu_time();
    let wall_start = Instant::now();

    let sent = std::rc::Rc::new(std::cell::Cell::new(0u32));
    let done = std::rc::Rc::new(std::cell::Cell::new(false));
    let timer = slint::Timer::default();
    let timer_sent = sent.clone();
    let timer_done = done.clone();
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(interval),
        move || {
            let count = timer_sent.get();
            if count >= events {
                timer_done.set(true);
                return;
            }
            // スクロールイベントを送り、ListView の中身を動かす。
            list.scroll(0.0, -20.0);
            timer_sent.set(count + 1);
        },
    );

    // 完了したら終了する。
    let exit_timer = slint::Timer::default();
    let exit_done = done.clone();
    exit_timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(20),
        move || {
            if exit_done.get() {
                let _ = slint::quit_event_loop();
            }
        },
    );
    let _ = slint::run_event_loop();

    let cpu = cpu_time().saturating_sub(cpu_start);
    let count = sent.get();
    println!(
        "rows={rows} events={count} static_cells={static_cells} cpu_ms={} wall_ms={} cpu_ms_per_event={}",
        cpu.as_millis(),
        wall_start.elapsed().as_millis(),
        cpu.as_millis() / u128::from(count.max(1)),
    );
}
