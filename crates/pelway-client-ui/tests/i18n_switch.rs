//! 起動後の言語切替の検査（テスト用 backend。窓は開かない）。
//!
//! Issue 384: `apply_lang` で言語を差し替えると、再起動なしで `tr-*` が
//! 対応する `.ftl` の文言に変わること。代表キー（`today-title`・`ext-send`）の
//! 文字列を ja・en の `.ftl` の文言（リテラル）と直接照合する。
//! 未対応の言語では `None` を返し、`tr-*` を変えないこと。
//!
//! 検査は 1 関数にまとめる（`slint_window.rs` と同じ構成）。同一プロセスで
//! テストを並列に走らせると、backend の初期化と `MainWindow::new()` の間で
//! 競合し、ヘッドレスの CI で winit へのフォールバックに落ちることがある。

use pelway_client_ui::MainWindow;
use pelway_client_ui::i18n::{FluentVars, apply_lang};

/// テスト用 backend の初期化をプロセスで 1 回だけ行う（`slint_window.rs` と
/// 同じ理由。`i_slint_backend_testing::init_no_event_loop` は 2 回呼ぶと落ちる）。
fn init_testing_backend_once() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(i_slint_backend_testing::init_no_event_loop);
}

/// ja と en の `.ftl` に書いた代表キーの文言（照合の正本は `.ftl`）。
const JA_TODAY_TITLE: &str = "今日";
const EN_TODAY_TITLE: &str = "Today";
const JA_EXT_SEND: &str = "送る";
const EN_EXT_SEND: &str = "Send";

#[test]
fn apply_lang_switches_texts_and_rejects_unsupported_langs() {
    // テスト用 backend は環境変数でなく明示的に初期化する（`slint_window.rs`
    // と同じ理由。このファイルの検査はこの 1 関数だけ）。
    init_testing_backend_once();

    let window = MainWindow::new().expect("MainWindow を作れること（テスト用 backend）");
    let vars = FluentVars::sample();

    // ja で始める（起動時の初期適用と同じ形）。
    let missing = apply_lang(&window, "ja", &vars).expect("ja は適用できること");
    assert!(missing.is_empty(), "ja で渡せないキーがあった: {missing:?}");
    assert_eq!(window.get_tr_today_title().as_str(), JA_TODAY_TITLE);
    assert_eq!(window.get_tr_ext_send().as_str(), JA_EXT_SEND);

    // ja → en: 再起動なしで文言が en の `.ftl` に変わること。
    let missing = apply_lang(&window, "en", &vars).expect("en は適用できること");
    assert!(missing.is_empty(), "en で渡せないキーがあった: {missing:?}");
    assert_eq!(window.get_tr_today_title().as_str(), EN_TODAY_TITLE);
    assert_eq!(window.get_tr_ext_send().as_str(), EN_EXT_SEND);

    // en → ja: 逆方向も同じこと。
    let missing = apply_lang(&window, "ja", &vars).expect("ja は適用できること");
    assert!(missing.is_empty(), "ja で渡せないキーがあった: {missing:?}");
    assert_eq!(window.get_tr_today_title().as_str(), JA_TODAY_TITLE);
    assert_eq!(window.get_tr_ext_send().as_str(), JA_EXT_SEND);

    // 未対応の言語では `None` を返し、`tr-*` は今の言語のまま変わらないこと
    // （翻訳の無いキーを画面へ出さない）。
    assert!(
        apply_lang(&window, "fr", &vars).is_none(),
        "用意していない言語は None であること"
    );
    assert_eq!(
        window.get_tr_today_title().as_str(),
        JA_TODAY_TITLE,
        "未対応言語の適用では tr-* が変わらないこと"
    );
    assert_eq!(window.get_tr_ext_send().as_str(), JA_EXT_SEND);
}
