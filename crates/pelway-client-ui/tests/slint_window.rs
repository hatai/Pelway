//! Slint が生成した `MainWindow` の検査（テスト用 backend。窓は開かない）。
//!
//! dev-dependency の `i-slint-backend-testing` が頭なしのテスト用 backend を
//! 提供する。`SLINT_BACKEND=testing` の選択は backend の初期化より前に必要な
//! ため、このファイルの検査は 1 つのテスト関数にまとめて直列に動かす
//! （並列実行での環境変数の競合を避ける。窓は開かない）。

use pelway_client_ui::i18n::{FluentVars, Messages, USED_KEYS, apply_translations};
use pelway_client_ui::theme::{Theme, apply_design_tokens, token_value};
use pelway_client_ui::{DesignTokens, MainWindow, ThemeState};
use slint::{Brush, Global, SharedString};

/// `MainWindow` の `tr-*` プロパティの getter の型。
type TrGetter = fn(&MainWindow) -> SharedString;

/// [`USED_KEYS`] の各キーと `MainWindow` の対応する `tr-*` の getter の対応表。
///
/// `apply_translations` が全キーを `tr-*` に渡すことを振る舞い（プロパティの値）で
/// 確かめるために使う。返り値の `missing` は Fluent から取れなかったキーだけを
/// 数えるため、setter が1つ消えても `missing` は空のまま——値を読んで確かめる
/// 必要がある。キーと getter の対応はこの表に 1 か所にまとめ、表の件数とキーの
/// 集合が `USED_KEYS` と一致することもテストで確かめる（表から1つ漏れたら落ちる）。
const TR_GETTERS: &[(&str, TrGetter)] = &[
    ("rail-viewing-as", |w| w.get_tr_rail_viewing_as()),
    ("rail-today", |w| w.get_tr_rail_today()),
    ("rail-channels", |w| w.get_tr_rail_channels()),
    ("rail-dm-people", |w| w.get_tr_rail_dm_people()),
    ("rail-dm-ai", |w| w.get_tr_rail_dm_ai()),
    ("ai-presence-standby", |w| w.get_tr_ai_presence_standby()),
    ("ai-presence-working", |w| w.get_tr_ai_presence_working()),
    ("ai-presence-waiting", |w| w.get_tr_ai_presence_waiting()),
    ("search-placeholder", |w| w.get_tr_search_placeholder()),
    ("search-shortcut", |w| w.get_tr_search_shortcut()),
    ("today-title", |w| w.get_tr_today_title()),
    ("today-section-consultations", |w| {
        w.get_tr_today_section_consultations()
    }),
    ("today-section-pending", |w| {
        w.get_tr_today_section_pending()
    }),
    ("today-section-decided", |w| {
        w.get_tr_today_section_decided()
    }),
    ("today-section-empty", |w| w.get_tr_today_section_empty()),
    ("today-quiet", |w| w.get_tr_today_quiet()),
    ("unread-divider", |w| w.get_tr_unread_divider()),
    ("ai-badge", |w| w.get_tr_ai_badge()),
    ("decision-mark", |w| w.get_tr_decision_mark()),
    ("message-private-note", |w| w.get_tr_message_private_note()),
    ("thread-replies", |w| w.get_tr_thread_replies()),
    ("thread-working", |w| w.get_tr_thread_working()),
    ("thread-done", |w| w.get_tr_thread_done()),
    ("thread-view-work", |w| w.get_tr_thread_view_work()),
    ("mention-read-range", |w| w.get_tr_mention_read_range()),
    ("handoff-context", |w| w.get_tr_handoff_context()),
    ("handoff-acknowledged", |w| w.get_tr_handoff_acknowledged()),
    ("handoff-not-acknowledged", |w| {
        w.get_tr_handoff_not_acknowledged()
    }),
    ("handoff-acknowledge", |w| w.get_tr_handoff_acknowledge()),
    ("handoff-decision-owner", |w| {
        w.get_tr_handoff_decision_owner()
    }),
    ("ext-card-label", |w| w.get_tr_ext_card_label()),
    ("ext-card-question", |w| w.get_tr_ext_card_question()),
    ("ext-address-label", |w| w.get_tr_ext_address_label()),
    ("ext-subject-label", |w| w.get_tr_ext_subject_label()),
    ("ext-basis-label", |w| w.get_tr_ext_basis_label()),
    ("ext-send", |w| w.get_tr_ext_send()),
    ("ext-fix", |w| w.get_tr_ext_fix()),
    ("ext-lock-note", |w| w.get_tr_ext_lock_note()),
    ("ext-sent", |w| w.get_tr_ext_sent()),
    ("pane-thread-title", |w| w.get_tr_pane_thread_title()),
    ("pane-close", |w| w.get_tr_pane_close()),
    ("pane-back-to-thread", |w| w.get_tr_pane_back_to_thread()),
    ("pane-ai-work-title", |w| w.get_tr_pane_ai_work_title()),
    ("pane-ai-steps", |w| w.get_tr_pane_ai_steps()),
    ("pane-ai-from", |w| w.get_tr_pane_ai_from()),
    ("pane-ai-read-range", |w| w.get_tr_pane_ai_read_range()),
    ("pane-ai-note", |w| w.get_tr_pane_ai_note()),
    ("thread-input-placeholder", |w| {
        w.get_tr_thread_input_placeholder()
    }),
    ("composer-placeholder", |w| w.get_tr_composer_placeholder()),
];

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

    // 対応表が USED_KEYS と同じ件数・同じ集合であること（表から1つキーが
    // 漏れたり余分なキーが入ったりしたら落ちる）。
    assert_eq!(
        TR_GETTERS.len(),
        USED_KEYS.len(),
        "tr-* の対応表の件数が USED_KEYS の件数と同じであること"
    );
    let mut table_keys: Vec<&str> = TR_GETTERS.iter().map(|(key, _)| *key).collect();
    table_keys.sort_unstable();
    let mut used_keys: Vec<&str> = USED_KEYS.to_vec();
    used_keys.sort_unstable();
    assert_eq!(
        table_keys, used_keys,
        "tr-* の対応表のキーが USED_KEYS と同じ集合であること"
    );

    // Fluent から引いた文言を tr-* に渡せること（本文は出力しない）。
    // apply_translations のあと、USED_KEYS の全キーについて tr-* の値が
    // message(key) の結果と一致することを日英それぞれで確かめる。
    // 返り値の missing は Fluent から取れなかったキーだけを数えるため、
    // setter が1つ消えても missing は空のまま——値を読んで確かめる。
    // ja と en の順に適用するため、en の適用で値が実際に切り替わることも
    // この照合が見る（ja の文言が残っていれば en の側で落ちる）。
    let vars = FluentVars::sample();
    for lang in ["ja", "en"] {
        let messages =
            Messages::new(lang).unwrap_or_else(|| panic!("{lang} の bundle が作れること"));
        let missing = apply_translations(&window, &messages, &vars);
        assert!(
            missing.is_empty(),
            "{lang} で渡せないキーがあった: {missing:?}"
        );
        for (key, getter) in TR_GETTERS {
            let expected = messages
                .message(key, &vars)
                .unwrap_or_else(|| panic!("{lang} の {key} が引けること"));
            assert_eq!(
                getter(&window).as_str(),
                expected.as_str(),
                "{lang} を渡したあとの {key} の tr-* が message の結果と一致すること"
            );
        }
    }

    // 切替のプロパティもそのまま渡せること（窓は開かない）。
    window.set_show_today(false);
    assert!(!window.get_show_today());
}
