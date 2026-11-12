//! Slint 1.18.1 の ListView が行ごとに高さが違う場合（可変高の仮想リスト）を
//! どう扱うかを確かめるテスト。確かめることは 3 つ: 見える行だけが生成されるか
//! （仮想化）、行が重なったり抜けたりしないか、遠い位置へ飛んだあとも同じか。
//!
//! スクロールはテスト用 backend の要素 API からスクロールイベントを送って
//! ListView 内部の flickable を動かして実現する（content-y を外側から差し替える
//! 方式だと仮想化の範囲が追従しないため）。測定結果は
//! docs/research/slint-tauri-client-measurements.md。

use std::collections::BTreeMap;
use std::sync::{Arc, Once};

use i_slint_backend_testing::ElementRoot;
use pelway_dev_client_probe::{Probe, ProbeWindow, new_probe_window, row_height};

/// テスト用 backend（窓を開かない）を、テストごとに 1 回だけ入れる。Slint の
/// platform はスレッドローカルなのでテストごとに初期化する（プロセス-wide な
/// Once では 2 つめ以降のテストが winit backend にフォールバックする）。
fn init_testing_backend_once() {
    thread_local! {
        static INIT: Once = const { Once::new() };
    }
    INIT.with(|init| init.call_once(i_slint_backend_testing::init_no_event_loop));
}

/// 行数を変えた検証用ウィンドウと記録器を用意する。
fn setup(item_count: i32, variable_height: bool) -> (ProbeWindow, Arc<Probe>) {
    let probe = Probe::new();
    let window = new_probe_window(&probe);
    window.set_item_count(item_count);
    window.set_variable_height(variable_height);
    (window, probe)
}

/// スクロールしてレイアウトを走らせ、記録された行の位置を返す。
fn scroll_and_record(window: &ProbeWindow, probe: &Probe, delta_y: f32) -> BTreeMap<u32, f32> {
    let list = window
        .root_element()
        .query_descendants()
        .match_type_name("ListView")
        .find_first()
        .expect("検証用 UI に ListView がある");
    list.scroll(0.0, delta_y);
    slint::platform::update_timers_and_animations();
    probe.positions()
}

/// 記録された行が、それぞれの高さどおりに並んでいることを確かめる。
fn assert_rows_follow_their_heights(positions: &BTreeMap<u32, f32>, variable: bool) {
    let indices: Vec<u32> = positions.keys().copied().collect();
    assert!(!indices.is_empty(), "行が生成されていない");
    for pair in indices.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        // 間の行の高さ（作り直された行が記録に無いとき用）。
        let expected: f32 = (from..to).map(|i| row_height(i, variable)).sum();
        let actual = positions[&to] - positions[&from];
        assert!(
            (actual - expected).abs() < 0.5,
            "行 {from} と {to} の間が {actual}px（期待 {expected}px）。高さが反映されて\
             いないか、行が重なっている"
        );
    }
}

#[test]
fn visible_rows_only_are_created_and_placed_at_their_own_heights() {
    init_testing_backend_once();
    let (window, probe) = setup(10_000, true);

    let positions = scroll_and_record(&window, &probe, -300.0);

    assert!(
        !positions.is_empty(),
        "スクロールしても 1 行も生成されないのは異常"
    );
    assert!(
        positions.len() <= 40,
        "10,000 行のうち {} 行が生成された（可視範囲に収まっていない）",
        positions.len()
    );
    // 生成された行は連続した範囲になっている（途中の行が抜けていない）。
    let indices: Vec<u32> = positions.keys().copied().collect();
    let contiguous = indices.windows(2).all(|w| w[1] == w[0] + 1);
    assert!(contiguous, "生成された行が連続していない: {indices:?}");
    assert_rows_follow_their_heights(&positions, true);

    // 対照として、行の高さが固定でも同じ位置関係になる（高さの条件を確かめる検査が、
    // 可変高のときだけ成立していないかの確認）。
    let (window, probe) = setup(10_000, false);
    let positions = scroll_and_record(&window, &probe, -300.0);
    assert_rows_follow_their_heights(&positions, false);
}

#[test]
fn scrolling_far_keeps_the_number_of_created_rows_small() {
    init_testing_backend_once();
    let (window, probe) = setup(10_000, true);
    scroll_and_record(&window, &probe, -300.0);

    // 遠い位置へ飛んで、生成される行が可視範囲に収まることを確かめる。
    probe.reset();
    let positions = scroll_and_record(&window, &probe, -30_000.0);

    assert!(
        !positions.is_empty(),
        "遠い位置へスクロールしても行が生成されていない"
    );
    assert!(
        positions.len() <= 40,
        "遠い位置へスクロールした結果 {} 行が生成された（可視範囲に収まっていない）",
        positions.len()
    );
    let first = *positions.keys().next().expect("行がある");
    assert!(
        first > 100,
        "遠い位置へスクロールしたのに生成された行は {first} 番目から（反映されていない）"
    );
}
