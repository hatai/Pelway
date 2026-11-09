//! Slint が生成した `MainWindow` の検査（テスト用 backend。窓は開かない）。
//!
//! dev-dependency の `i-slint-backend-testing` が頭なしのテスト用 backend を
//! 提供する。`SLINT_BACKEND=testing` の選択は backend の初期化より前に必要な
//! ため、このファイルの検査は 1 つのテスト関数にまとめて直列に動かす
//! （並列実行での環境変数の競合を避ける。窓は開かない）。

use pelway_client_ui::theme::Theme;
use pelway_client_ui::{MainWindow, ThemeState};
use slint::Global;

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

    // 文言のプロパティ（tr-*）にも Rust から値を渡せること。
    window.set_tr_rail_today("今日".into());
    assert_eq!(window.get_tr_rail_today().as_str(), "今日");

    // 切替のプロパティもそのまま渡せること（窓は開かない）。
    window.set_show_today(false);
    assert!(!window.get_show_today());
}
