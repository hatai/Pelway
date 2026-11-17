//! チャンネルの時系列の仮想化の検査（Issue 385。テスト用 backend。窓は開かない）。
//!
//! `crates/dev-client-probe/tests/virtualization.rs` と同型: ListView 内部の
//! flickable にスクロールイベントを送って、実体化した行を数えて位置を確かめる
//! （外側から `content-y` を差し込まない）。行の実体化と位置は
//! `timeline-row` の id を持つ要素を要素 API で数えて読む。行番号の記録は
//! `record-timeline-row` コールバックが流す（行が動くたびに自分の行番号と
//! 位置を流す）。
//!
//! 注意: ListView の行の y は「見えている行の積み上げ」が正で、行番号から
//! 逆算した絶対位置は推定を含む（仮想化のしくみ上、見ていない行の高さは
//! 推定するしかない）。このため「隣り合う行が隙間なく並ぶか」は、同時に
//! 実体化している行同士の並びで確かめる。
//!
//! テスト用 backend（窓を開かない）はテストごとに 1 回だけ入れる。Slint の
//! platform はスレッドローカルなのでテストごとに初期化する（プロセス全体の
//! `Once` では 2 つめ以降のテストが winit backend にフォールバックする）。

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, Once};

use i_slint_backend_testing::ElementRoot;
use pelway_client_ui::bridge::apply_channel_view;
use pelway_client_ui::state::sample;
use pelway_client_ui::{MainWindow, TimelineItem, TimelineRowKind};
use slint::{ComponentHandle, LogicalSize, Model, ModelRc, VecModel, WindowSize};

/// 行番号の記録（実体化した行が流してきた番号の一覧。遠くへ飛んだあとの
/// 行番号を確かめるのに使う）。
#[derive(Debug, Default)]
struct RowLog {
    indices: Mutex<BTreeSet<u32>>,
}

fn init_testing_backend_once() {
    thread_local! {
        static INIT: Once = const { Once::new() };
    }
    INIT.with(|init| init.call_once(i_slint_backend_testing::init_no_event_loop));
}

/// 記録の結線を済ませた窓を作る。右の枠は閉じて、画面に載る ListView と
/// flickable を時系列のものだけに絞る。
fn window_with_log() -> (MainWindow, Arc<RowLog>) {
    let window = MainWindow::new().expect("MainWindow を作れること（テスト用 backend）");
    window
        .window()
        .set_size(WindowSize::Logical(LogicalSize::new(1280.0, 800.0)));
    window.set_show_today(false);
    window.set_show_right_pane(false);
    let log = Arc::new(RowLog::default());
    let recorded = Arc::clone(&log);
    window.on_record_timeline_row(move |index, _y, _height| {
        recorded
            .indices
            .lock()
            .expect("記録のロック")
            .insert(index as u32);
    });
    (window, log)
}

/// 実体化している時系列の行を、上から順に (絶対位置 y, 高さ) で返す。
fn materialized_rows(window: &MainWindow) -> Vec<(f32, f32)> {
    let mut rows: Vec<(f32, f32)> = window
        .root_element()
        .query_descendants()
        .match_id("ChannelView::timeline-row")
        .find_all()
        .into_iter()
        .map(|el| (el.absolute_position().y, el.size().height))
        .collect();
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    rows
}

/// 時系列の ListView（スクロールの送り先。内部の flickable にイベントを送る）。
fn timeline_listview(window: &MainWindow) -> i_slint_backend_testing::ElementHandle {
    window
        .root_element()
        .query_descendants()
        .match_type_name("ListView")
        .find_first()
        .expect("時系列の ListView がある")
}

/// スクロールしてレイアウトを走らせる。スクロールは要素 API から
/// スクロールイベントを送って ListView 内部の flickable を動かして実現する。
fn scroll(window: &MainWindow, delta_y: f32) {
    timeline_listview(window).scroll(0.0, delta_y);
    slint::platform::update_timers_and_animations();
}

/// 件数の多い合成の時系列。種類と本文の行数を行番号で変えて、行の高さが
/// 行ごとに違うようにする。
fn synthetic_items(count: usize) -> Vec<TimelineItem> {
    let base = |kind| TimelineItem {
        kind,
        ..TimelineItem::default()
    };
    (0..count)
        .map(|i| {
            let kind = match i % 97 {
                0 => TimelineRowKind::DayDivider,
                31 => TimelineRowKind::UnreadMarker,
                47 => TimelineRowKind::ThreadLead,
                59 => TimelineRowKind::DecisionMark,
                71 => TimelineRowKind::Handoff,
                83 => TimelineRowKind::PrivateNote,
                _ => TimelineRowKind::Message,
            };
            // 本文の行数を行番号で変える（高さのばらつき）。
            let body = "合成データ\n".repeat(i % 5 + 1);
            match kind {
                TimelineRowKind::DayDivider => TimelineItem {
                    day_label: "11月10日（火）".into(),
                    ..base(kind)
                },
                TimelineRowKind::Handoff => TimelineItem {
                    handoff_from_to: "田中 → 濱田".into(),
                    handoff_title: "仕様変更の件".into(),
                    handoff_acknowledged: i % 2 == 0,
                    handoff_option_a: "A　範囲を絞って12/1".into(),
                    handoff_option_b: "B　12/3に延ばす".into(),
                    ..base(kind)
                },
                TimelineRowKind::Message | TimelineRowKind::PrivateNote => TimelineItem {
                    sender: "田中".into(),
                    initial: "田".into(),
                    time: "15:00".into(),
                    body: body.into(),
                    ..base(kind)
                },
                _ => base(kind),
            }
        })
        .collect()
}

/// 時系列が ListView で描かれていること（仮想化の前提）。
#[test]
fn timeline_is_drawn_with_listview() {
    init_testing_backend_once();
    let (window, _log) = window_with_log();
    assert!(timeline_listview(&window).is_valid());
}

/// 10,000 件を渡しても、実体化するのは見える範囲の行だけで、高さの違う行が
/// 抜けも重なりもなく並ぶこと（受け入れ条件の 1 つ目と 2 つ目）。
#[test]
fn timeline_materializes_only_visible_rows_and_stacks_them_at_their_heights() {
    init_testing_backend_once();
    let (window, _log) = window_with_log();
    window.set_timeline_items(ModelRc::new(VecModel::from(synthetic_items(10_000))));

    scroll(&window, -300.0);
    let rows = materialized_rows(&window);

    assert!(
        !rows.is_empty(),
        "スクロールしても 1 行も生成されないのは異常"
    );
    assert!(
        rows.len() <= 40,
        "10,000 行のうち {} 行が生成された",
        rows.len()
    );
    // 行の高さが実際に違うこと（高さの条件を確かめる検査が空でないこと）。
    let heights: BTreeSet<u32> = rows.iter().map(|(_, h)| h.to_bits()).collect();
    assert!(
        heights.len() >= 2,
        "生成された行の高さがすべて同じ（可変高の前提が成立していない）"
    );
    // 隣り合う実体化行は隙間なく重なりなく並ぶ（下の行の y が、上の行の
    // 底辺と一致する）。実体化は連続した範囲なので、すべての隣接行で確かめる。
    for pair in rows.windows(2) {
        let (top_y, top_h) = pair[0];
        let (next_y, _) = pair[1];
        let gap = next_y - (top_y + top_h);
        assert!(
            gap.abs() < 0.5,
            "隣り合う行の間に {gap}px の隙間・重なりがある（y={top_y}, h={top_h}, 次の y={next_y}）"
        );
    }
}

/// 遠い位置へスクロールしても、実体化する行は見える範囲に収まること
/// （受け入れ条件の 3 つ目）。
#[test]
fn scrolling_far_keeps_the_number_of_created_rows_small() {
    init_testing_backend_once();
    let (window, log) = window_with_log();
    window.set_timeline_items(ModelRc::new(VecModel::from(synthetic_items(10_000))));
    scroll(&window, -300.0);
    log.indices.lock().expect("記録のロック").clear();

    scroll(&window, -30_000.0);
    let rows = materialized_rows(&window);

    assert!(
        !rows.is_empty(),
        "遠い位置へスクロールしても行が生成されていない"
    );
    assert!(
        rows.len() <= 40,
        "遠い位置へスクロールした結果 {} 行が生成された",
        rows.len()
    );
    let indices = log.indices.lock().expect("記録のロック");
    let first = *indices.iter().next().expect("行がある");
    assert!(
        first > 100,
        "遠い位置へスクロールしたのに {first} 番目から（反映されていない）"
    );
}

/// 既存のカードを含むすべての種類の行が、時系列のリストに載ること
/// （受け入れ条件の 4 つ目）。`state::sample()` の時系列を
/// `apply_channel_view` で渡し、端までスクロールして各行の実体化を確かめる。
#[test]
fn every_row_kind_appears_on_the_list() {
    init_testing_backend_once();
    let (window, log) = window_with_log();
    apply_channel_view(&window, &sample().channel);

    let model = window.get_timeline_items();
    let count = model.row_count();
    assert!(count > 0, "時系列に行がある");
    // 実体化するのは見える範囲だけなので、端まで順にスクロールして全行の
    // 実体化を集める（判断カード・引き継ぎ・社外送信・私信を含む）。カードの
    // 部品の実体化も、画面を過ぎると行が消えるため、各段でその場で確かめる。
    let mut seen_types = BTreeSet::new();
    for _ in 0..32 {
        scroll(&window, -300.0);
        for name in [
            "DecisionCardView",
            "HandoffCardView",
            "ExternalSendCardView",
            "PrivateNoteRow",
        ] {
            if window
                .root_element()
                .query_descendants()
                .match_type_name(name)
                .find_first()
                .is_some()
            {
                seen_types.insert(name);
            }
        }
        if log.indices.lock().expect("記録のロック").len() == count {
            break;
        }
    }
    let indices = log.indices.lock().expect("記録のロック").clone();
    assert_eq!(
        indices.len(),
        count,
        "全行が一度は実体化されない（実体化した行番号: {indices:?}）"
    );
    // 記録された行番号から種類を引き、既存のカードの種類が揃うことを確かめる。
    let mut kinds = BTreeSet::new();
    for index in &indices {
        let kind = model
            .row_data(*index as usize)
            .expect("記録された行は一覧にある")
            .kind;
        kinds.insert(kind as i32);
    }
    for (kind, name) in [
        (TimelineRowKind::DecisionCard, "判断カード"),
        (TimelineRowKind::Handoff, "引き継ぎカード"),
        (TimelineRowKind::ExternalSend, "社外送信カード"),
        (TimelineRowKind::PrivateNote, "私信"),
    ] {
        assert!(
            kinds.contains(&(kind as i32)),
            "{name} の行が時系列のリストに載っていない"
        );
    }
    // カードの種類ごとの部品が実際に載ったこと（同じ種類のまま出る）。
    for name in [
        "DecisionCardView",
        "HandoffCardView",
        "ExternalSendCardView",
        "PrivateNoteRow",
    ] {
        assert!(
            seen_types.contains(name),
            "{name} が時系列に実体化されなかった"
        );
    }
}
