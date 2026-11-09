//! Slint が生成した `MainWindow` の検査（テスト用 backend。窓は開かない）。
//!
//! dev-dependency の `i-slint-backend-testing` が頭なしのテスト用 backend を
//! 提供する。`SLINT_BACKEND=testing` の選択は backend の初期化より前に必要な
//! ため、このファイルの検査は 1 つのテスト関数にまとめて直列に動かす
//! （並列実行での環境変数の競合を避ける。窓は開かない）。

use pelway_client_ui::i18n::{FluentVars, Messages, apply_translations};
use pelway_client_ui::theme::{Theme, apply_design_tokens, token_value};
use pelway_client_ui::{DesignTokens, MainWindow, ThemeState};
use slint::{Brush, Global};

#[test]
fn main_window_can_be_created_and_accepts_all_theme_names() {
    // 安全性: 環境変数はこのテストバイナリの他のスレッドが動く前に設定する
    // （このファイルの検査はこの 1 関数だけ）。
    unsafe { std::env::set_var("SLINT_BACKEND", "testing") };

    let window = MainWindow::new().expect("MainWindow を作れること（テスト用 backend）");
    let theme_state = ThemeState::get(&window);

    // テーマ名 4 つ（light・dark・light-high-contrast・dark-high-contrast）を
    // ThemeState.current-name に渡せること。テーマ名の定義元は Rust の
    // Theme::name()（.slint 側は受け取るだけ）。
    for theme in Theme::ALL {
        theme_state.set_current_name(theme.name().into());
        assert_eq!(
            theme_state.get_current_name().as_str(),
            theme.name(),
            "ThemeState.current-name に {} を渡せること",
            theme.name()
        );
    }

    // DesignTokens（in-out 化したグローバル）にテーマごとの値を渡せること。
    // 値は token_value（トークンの正本 JSON から読む）と一致し、
    // テーマを切り替えると値が変わること（窓は開かない）。
    let tokens = DesignTokens::get(&window);
    let light_bg = token_value(Theme::Light, "bg").expect("light の bg に値があること");
    let dark_bg = token_value(Theme::Dark, "bg").expect("dark の bg に値があること");
    apply_design_tokens(&window, Theme::Light);
    assert_eq!(
        tokens.get_bg(),
        Brush::from(light_bg),
        "light を適用すると DesignTokens.bg が light の値になること"
    );
    apply_design_tokens(&window, Theme::Dark);
    assert_eq!(
        tokens.get_bg(),
        Brush::from(dark_bg),
        "dark を適用すると DesignTokens.bg が dark の値になること"
    );
    assert_ne!(
        Brush::from(light_bg),
        Brush::from(dark_bg),
        "light と dark で bg の値が変わること（切替の確認）"
    );
    // もう1つのトークン（ink）も適用したテーマの値になっていること。
    let dark_ink = token_value(Theme::Dark, "ink").expect("dark の ink に値があること");
    assert_eq!(
        tokens.get_ink(),
        Brush::from(dark_ink),
        "dark を適用すると DesignTokens.ink も dark の値になること"
    );

    // 文言のプロパティ（tr-*）にも Rust から値を渡せること。
    window.set_tr_rail_today("今日".into());
    assert_eq!(window.get_tr_rail_today().as_str(), "今日");

    // Fluent から引いた文言を tr-* に渡せること（本文は出力しない。
    // ja と en で同じキーに別の文言が入ること）。
    let vars = FluentVars::sample();
    let ja = Messages::new("ja").expect("ja の bundle が作れること");
    let missing_ja = apply_translations(&window, &ja, &vars);
    assert!(
        missing_ja.is_empty(),
        "ja で渡せないキーがあった: {missing_ja:?}"
    );
    let ja_today = ja.message("rail-today", &vars).expect("ja の rail-today");
    assert_eq!(
        window.get_tr_rail_today().as_str(),
        ja_today.as_str(),
        "渡したあと tr-rail-today が ja の文言になっていること"
    );
    let ja_unread = ja
        .message("unread-divider", &vars)
        .expect("ja の unread-divider");
    assert_eq!(
        window.get_tr_unread_divider().as_str(),
        ja_unread.as_str(),
        "渡したあと tr-unread-divider が ja の文言になっていること"
    );
    let en = Messages::new("en").expect("en の bundle が作れること");
    let missing_en = apply_translations(&window, &en, &vars);
    assert!(
        missing_en.is_empty(),
        "en で渡せないキーがあった: {missing_en:?}"
    );
    let en_today = en.message("rail-today", &vars).expect("en の rail-today");
    assert_eq!(
        window.get_tr_rail_today().as_str(),
        en_today.as_str(),
        "en に切り替えると tr-rail-today が en の文言になっていること"
    );

    // 切替のプロパティもそのまま渡せること（窓は開かない）。
    window.set_show_today(false);
    assert!(!window.get_show_today());
}
