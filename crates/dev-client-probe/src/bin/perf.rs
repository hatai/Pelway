//! Issue #98 の slint #3867（読み上げを有効にすると大きな ListView が遅い）の
//! 再現と実測のための計測プログラム。製品機能ではない。
//!
//! 読み上げ（アクセシビリティ）を有効にしたビルドと無効にしたビルドで同じ条件を
//! 実行し、CPU 時間の差を比べる。有効なビルドは、セッションの dbus に
//! `org.a11y.Status.IsEnabled` が true になっていることが前提。測定結果は
//! docs/research/slint-tauri-client-measurements.md。

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

use i_slint_backend_testing::ElementRoot;
use pelway_dev_client_probe::ProbeWindow;
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
    let mut args = std::env::args().skip(1);
    let mut values: HashMap<String, i64> = HashMap::new();
    while let Some(flag) = args.next() {
        let value = args.next().unwrap_or_default().parse().unwrap_or_default();
        values.insert(flag, value);
    }
    let rows = values.get("--rows").copied().unwrap_or(100_000) as i32;
    let events = values.get("--events").copied().unwrap_or(2_000) as u32;
    let interval = values.get("--interval").copied().unwrap_or(5) as u64;
    let static_cells = values.get("--static-cells").copied().unwrap_or(0) as i32;

    let window = ProbeWindow::new().expect("ProbeWindow の生成");
    window.set_item_count(rows);
    window.set_static_cells(static_cells);
    window.set_variable_height(false);
    let list = window
        .root_element()
        .query_descendants()
        .match_type_name("ListView")
        .find_first()
        .expect("ProbeWindow に ListView がある");
    let _ = window.show();

    let cpu_start = cpu_time();
    let wall_start = Instant::now();
    let sent = Rc::new(Cell::new(0u32));
    // タイマーでスクロールイベントを送り、送り終えたら終了する。
    let timer = slint::Timer::default();
    let timer_sent = sent.clone();
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(interval),
        move || {
            let count = timer_sent.get();
            if count >= events {
                let _ = slint::quit_event_loop();
                return;
            }
            list.scroll(0.0, -20.0);
            timer_sent.set(count + 1);
        },
    );
    let _ = slint::run_event_loop();

    let cpu = cpu_time().saturating_sub(cpu_start);
    let count = sent.get();
    let per_event = cpu.as_millis() / u128::from(count.max(1));
    println!(
        "rows={rows} events={count} static_cells={static_cells} cpu_ms={} wall_ms={} cpu_ms_per_event={per_event}",
        cpu.as_millis(),
        wall_start.elapsed().as_millis()
    );
}
