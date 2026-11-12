//! Slint の attribution 表示（Issue 237）の検査。
//!
//! Slint Royalty-free License 2.0 §2 (a) は、Application の一部として Slint を
//! 配布する場合、`AboutSlint` widget をトップレベルメニューから到達できる
//! About 画面/ダイアログに表示することを求める（docs/research/slint-royalty-free-license.md）。
//! この検査はその要件を、テスト用 backend（窓を開かない）の上で確かめる:
//!
//! - メニュー項目の操作後に About ダイアログが表示されること
//!   （`i-slint-backend-testing` の `ElementHandle` で MenuBar を実際に開いて
//!   項目を押す。メニューの文言は Fluent の ja/en を `tr-*` に渡した値）。
//! - About ダイアログの中に `AboutSlint` widget が表示されること
//!   （要素の型名で探索）。
//! - 「閉じる」を押すとダイアログが閉じ、`AboutSlint` も消えること。
//!
//! 注: `ElementHandle` の探索には、build.rs が debug info 付きで `.slint` を
//! コンパイルしている必要がある（`with_debug_info(true)`）。

use i_slint_backend_testing::ElementHandle;
use pelway_client_ui::MainWindow;
use pelway_client_ui::i18n::{FluentVars, Messages, apply_translations};
use pelway_client_ui::theme::{Theme, apply_design_tokens};
use slint::platform::PointerEventButton;

/// テスト用 backend の初期化をプロセスで 1 回だけ行う
/// （`tests/slint_window.rs` と同じ理由で `Once`）。
fn init_testing_backend_once() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(i_slint_backend_testing::init_no_event_loop);
}

/// `lang` の文言を渡した `MainWindow` を作る（メニューの文言も Fluent 由来）。
fn window_with_texts(lang: &str) -> MainWindow {
    init_testing_backend_once();
    let window = MainWindow::new().expect("MainWindow を作れること（テスト用 backend）");
    apply_design_tokens(&window, Theme::Light);
    let messages = Messages::new(lang).unwrap_or_else(|| panic!("{lang} の bundle が作れること"));
    let missing = apply_translations(&window, &messages, &FluentVars::sample());
    assert!(
        missing.is_empty(),
        "{lang} で渡せないキーがあった: {missing:?}"
    );
    window
}

/// 条件付き要素（`if root.show-about : ...`）の生成・消去を即時に反映させる。
fn settle() {
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(0));
}

/// メニューを開いて項目を押す。`menu_title` は MenuBar の項目の文言、
/// `item_title` はその中の MenuItem の文言（どちらも Fluent から渡した値）。
fn open_about_from_menu(window: &MainWindow, menu_title: &str, item_title: &str) {
    let menu = ElementHandle::find_by_accessible_label(window, menu_title)
        .next()
        .unwrap_or_else(|| panic!("MenuBar に「{menu_title}」メニューがあること"));
    menu.mock_single_click(PointerEventButton::Left);
    let item = ElementHandle::find_by_accessible_label(window, item_title)
        .next()
        .unwrap_or_else(|| panic!("「{menu_title}」メニューに「{item_title}」項目があること"));
    item.mock_single_click(PointerEventButton::Left);
}

#[test]
fn menu_operation_shows_about_dialog_with_aboutslint() {
    // 注: この検査は1つのテスト関数にまとめる。テスト用 backend のモック時計は
    // スレッドをまたぐと単調性の検査に落ちるため（「The platform's clock is not
    // monotonic!」）、ja・en の両方を同じスレッドで動かす。

    // ---- ja ----
    let window = window_with_texts("ja");

    // 最初は About ダイアログは出ていないこと。
    settle();
    assert!(
        !window.get_show_about(),
        "最初は About ダイアログが表示されていないこと"
    );
    assert!(
        ElementHandle::find_by_element_type_name(&window, "AboutSlint")
            .next()
            .is_none(),
        "最初は AboutSlint が表示されていないこと"
    );

    // トップレベルメニュー「ヘルプ」→「このアプリについて」を押す。
    open_about_from_menu(&window, "ヘルプ", "このアプリについて");

    // About ダイアログが表示されること。
    assert!(
        window.get_show_about(),
        "メニュー項目の操作後に About ダイアログが表示されること"
    );

    // ダイアログの中に AboutSlint widget があること（ライセンスの attribution）。
    settle();
    let about_slint = ElementHandle::find_by_element_type_name(&window, "AboutSlint")
        .next()
        .expect("About ダイアログに AboutSlint が表示されること");
    assert!(about_slint.is_valid());

    // ダイアログの中の「閉じる」を押すと閉じ、AboutSlint も消えること。
    let dialog = ElementHandle::find_by_element_type_name(&window, "AboutDialogView")
        .next()
        .expect("About ダイアログがあること");
    let close = dialog
        .query_descendants()
        .match_predicate(|e| e.accessible_label().as_deref() == Some("閉じる"))
        .find_first()
        .expect("About ダイアログに「閉じる」があること");
    close.mock_single_click(PointerEventButton::Left);
    assert!(
        !window.get_show_about(),
        "「閉じる」を押すと About ダイアログが閉じること"
    );
    settle();
    assert!(
        ElementHandle::find_by_element_type_name(&window, "AboutSlint")
            .next()
            .is_none(),
        "閉じたあとは AboutSlint が表示されていないこと"
    );

    // ---- en ----
    // en を渡した別の窓でも、メニューの文言が en に切り替わり、同じ操作で
    // About が開くこと（メニューの文言が Fluent 経由であることの裏付け）。
    let window_en = window_with_texts("en");
    open_about_from_menu(&window_en, "Help", "About This App");
    assert!(
        window_en.get_show_about(),
        "en でもメニュー項目の操作後に About ダイアログが表示されること"
    );
}
