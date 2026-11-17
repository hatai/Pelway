//! 常時表示の「デモ・模擬応答」バナーと AI 識別（Issue 381）の検査。
//!
//! エントリポイントの画面契約（docs/design/brief-2026-11-10/source-of-truth.md
//! 「画面契約（抜粋）」）に従い、テスト用 backend（窓を開かない）の上で確かめる:
//!
//! - 今日・チャンネル両画面で常時表示のバナーが 1 つだけ見え、文言が Fluent の
//!   ja/en どちらでも正しいこと。消す操作が無いこと。
//! - 幅を 360px 相当まで狭めても「（模擬）」/"simulated" の印が欠けず、
//!   文言は省略・三点リーダー・脱落なしに最大 2 行まで折り返すこと。
//! - AI の行には見える「AI」バッジがあり、人の行には無いこと
//!   （メッセージ・あなただけに表示・判断待ち・判断カード・レールの AI 一覧・
//!   AI の作業の枠）。
//! - バッジの読み上げは「AIメンバー」/"AI member" で、見える「AI」を
//!   読み上げで二重に読ませないこと。バッジを隠す仕組みが無いこと。
//!
//! 窓を作る検査は 1 テストにまとめる（tests/slint_attribution.rs と同じ理由で、
//! テスト用 backend のモック時計はスレッドをまたぐと単調性の検査に落ちる）。

use i_slint_backend_testing::ElementHandle;
use pelway_client_ui::MainWindow;
use pelway_client_ui::bridge::{DecisionShell, apply_decision_screen};
use pelway_client_ui::i18n::{FluentVars, Messages, apply_translations};
use pelway_client_ui::theme::{Theme, apply_design_tokens};
use slint::{ComponentHandle, LogicalSize};

const SLINT: &str = include_str!("../ui/main.slint");

/// テスト用 backend の初期化をプロセスで 1 回だけ行う
/// （`tests/slint_window.rs` と同じ理由で `Once`）。
fn init_testing_backend_once() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(i_slint_backend_testing::init_no_event_loop);
}

/// `lang` の文言を渡した `MainWindow` を作る。
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

/// `lang` の Fluent bundle から `key` の文言を引く。
fn fluent(lang: &str, key: &str) -> String {
    let messages = Messages::new(lang).unwrap_or_else(|| panic!("{lang} の bundle が作れること"));
    messages
        .message(key, &FluentVars::sample())
        .unwrap_or_else(|| panic!("{lang} の bundle にキー「{key}」があること"))
}

/// 条件付き要素（`if ... : ...`）とレイアウトの更新を即時に反映させる。
fn settle() {
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(0));
}

/// `.slint` から `component <name>` の定義ブロック（次の `component` /
/// `export component` の手前まで）を切り出す。
fn component_block<'a>(source: &'a str, name: &str) -> &'a str {
    let marker = format!("component {name} ");
    let start = source
        .find(&marker)
        .unwrap_or_else(|| panic!("main.slint に {marker} があること"));
    let rest = &source[start + marker.len()..];
    let end = rest
        .find("\ncomponent ")
        .or_else(|| rest.find("\nexport component "))
        .map(|pos| start + marker.len() + pos)
        .unwrap_or(source.len());
    &source[start..end]
}

/// 要素の子孫の accessible-label を全部集める。
fn descendant_labels(el: &ElementHandle) -> Vec<String> {
    el.query_descendants()
        .find_all()
        .iter()
        .filter_map(|e| e.accessible_label().map(|label| label.to_string()))
        .collect()
}

/// 常時バナー・AI バッジの検査（窓はこのテストでだけ作る。ja/en・今日/チャンネル・
/// 判断カード・AI の作業の枠・狭い幅を順に確かめる）。
#[test]
fn persistent_display_end_to_end() {
    // 判断待ちの合成データ（依頼者ユイは AI）。一覧の PendingDecisionRow と
    // チャット内の DecisionCardView の両方に出る。
    const NOW: u64 = 25 * 60 * 1000;

    for lang in ["ja", "en"] {
        let window = window_with_texts(lang);
        let banner_text = fluent(lang, "banner-demo-mock");
        let badge_text = fluent(lang, "ai-badge");
        let badge_a11y = fluent(lang, "label-ai-member-a11y");
        // 注: テスト用 backend の要素探索は ItemRc::is_visible() を通るため、
        // Flickable のビューポート外にクリップされた行は見つからない。
        // 行の検査の前に窓を縦長にして、一覧の全行を可視にしておく。

        // ---- 常時バナー: 今日・チャンネルの両画面で 1 つだけ見える ----
        for show_today in [true, false] {
            window.set_show_today(show_today);
            settle();
            let banners: Vec<ElementHandle> =
                ElementHandle::find_by_element_type_name(&window, "DemoBannerView").collect();
            assert_eq!(
                banners.len(),
                1,
                "{lang} show-today={show_today} でも常時バナーが 1 つだけあること"
            );
            let banner = &banners[0];
            let wanted = banner_text.clone();
            let text_el = banner
                .query_descendants()
                .match_predicate(move |e| e.accessible_label().as_deref() == Some(wanted.as_str()))
                .find_first()
                .unwrap_or_else(|| panic!("{lang} のバナーに「{banner_text}」が見えること"));
            let size = text_el.size();
            assert!(
                size.width > 0.0 && size.height > 0.0,
                "{lang} のバナーの文言が見えていること"
            );
            // 「消す」操作は作らない: バナーの内側に押せる要素が無いこと。
            assert!(
                banner
                    .query_descendants()
                    .match_type_name("TouchArea")
                    .find_first()
                    .is_none(),
                "{lang} の常時バナーに閉じる・消す操作が無いこと"
            );
        }

        // ---- 狭い幅（360px 相当）でも「（模擬）」/"simulated" が欠けない ----
        window.set_show_today(false);
        settle();
        let find_banner_text = |w: &MainWindow| {
            ElementHandle::find_by_accessible_label(w, &banner_text)
                .next()
                .unwrap_or_else(|| panic!("「{banner_text}」が見えること"))
        };
        let one_line_h = find_banner_text(&window).size().height;
        assert!(one_line_h > 0.0, "バナーの文言が 1 行で見えていること");
        let mut wrapped = false;
        for width in [360.0f32, 300.0, 260.0, 220.0, 190.0, 160.0, 140.0] {
            window.window().set_size(LogicalSize::new(width, 600.0));
            settle();
            let h = find_banner_text(&window).size().height;
            assert!(
                h > 0.5 * one_line_h,
                "{lang} 幅 {width}px でもバナーの文言が見えること (h={h})"
            );
            assert!(
                h <= 2.4 * one_line_h,
                "{lang} 幅 {width}px でも文言は最大 2 行に収まること (h={h}, 1行={one_line_h})"
            );
            if h > 1.3 * one_line_h {
                wrapped = true;
            }
        }
        assert!(
            wrapped,
            "{lang} のバナーの文言は狭い幅で 2 行に折り返すこと（省略や切り詰めではなく）"
        );
        // 縦長の窓にして一覧の全行をビューポート内に収める（上の注を参照）。
        window.window().set_size(LogicalSize::new(800.0, 2400.0));
        settle();

        // ---- AI の行には「AI」バッジ、人の行には無い ----
        // 送信者名の先頭で AI 行と人の行を区別できる合成データを入れる
        // （バッジの読み上げ文言とは衝突しない前置詞にする）。
        window.set_data_msg1_sender("H_甲".into());
        window.set_data_msg2_sender("AI_乙".into());
        window.set_data_msg3_sender("H_丙".into());
        window.set_data_note_sender("AI_丁".into());
        window.set_data_thread_msg1_sender("AI_戊".into());
        window.set_data_thread_msg2_sender("H_己".into());
        settle();

        // メッセージの行（チャンネル 3 件＋スレッドの枠 2 件）は型名で探せる
        // （複数回使う部品は型名が残る）。「あなただけに表示」の行は 1 回しか
        // 使わないため Slint が親にインラインし、型名では探せないので、宣言した
        // 要素 id で探す。
        let rows =
            ElementHandle::find_by_element_type_name(&window, "MessageRow").collect::<Vec<_>>();
        assert_eq!(
            rows.len(),
            5,
            "{lang} のチャンネル画面に MessageRow が 5 件あること"
        );
        for row in &rows {
            let labels = descendant_labels(row);
            let is_ai_row = labels.iter().any(|l| l.starts_with("AI_"));
            let badges = row
                .query_descendants()
                .match_type_name("AiBadgeView")
                .find_all()
                .len();
            assert_eq!(
                badges,
                if is_ai_row { 1 } else { 0 },
                "{lang} の MessageRow（{labels:?}）は AI の行だけ「{badge_text}」バッジが出ること"
            );
        }
        let note = ElementHandle::find_by_element_id(&window, "ChannelView::private-note")
            .next()
            .expect("「あなただけに表示」の行があること");
        assert_eq!(
            note.query_descendants()
                .match_type_name("AiBadgeView")
                .find_all()
                .len(),
            1,
            "{lang} の「あなただけに表示」の AI の行に「{badge_text}」バッジが出ること"
        );

        // レールの AI 一覧の各行にもバッジが出る（AI メンバーの名前一覧）。
        let rail_rows =
            ElementHandle::find_by_element_type_name(&window, "RailAiRow").collect::<Vec<_>>();
        assert_eq!(
            rail_rows.len(),
            3,
            "{lang} のレールに AI の行が 3 件あること"
        );
        for row in &rail_rows {
            assert_eq!(
                row.query_descendants()
                    .match_type_name("AiBadgeView")
                    .find_all()
                    .len(),
                1,
                "{lang} のレールの AI の行に「{badge_text}」バッジが出ること"
            );
        }

        // チャット内の判断カード（依頼者が AI）にもバッジが出る。
        apply_decision_screen(&window, &DecisionShell::sample(lang), NOW);
        settle();
        let card = ElementHandle::find_by_element_id(&window, "ChannelView::decision-card")
            .next()
            .expect("チャット内の判断カードが見えること");
        assert_eq!(
            card.query_descendants()
                .match_type_name("AiBadgeView")
                .find_all()
                .len(),
            1,
            "{lang} の判断カードの AI 依頼者に「{badge_text}」バッジが出ること"
        );

        // 今日画面の「自分の判断待ち」の行（依頼者が AI）にもバッジが出る。
        window.set_show_today(true);
        settle();
        let pending = ElementHandle::find_by_element_id(&window, "TodayView::pending-row")
            .collect::<Vec<_>>();
        assert_eq!(
            pending.len(),
            1,
            "{lang} の今日画面に判断待ちの行が 1 件あること"
        );
        assert_eq!(
            pending[0]
                .query_descendants()
                .match_type_name("AiBadgeView")
                .find_all()
                .len(),
            1,
            "{lang} の判断待ちの AI 依頼者に「{badge_text}」バッジが出ること"
        );

        // AI の作業の枠にもバッジが出る。
        window.set_show_today(false);
        window.set_right_pane_ai_work(true);
        settle();
        let pane = ElementHandle::find_by_element_id(&window, "MainWindow::ai-work-pane")
            .next()
            .expect("AI の作業の枠が見えること");
        assert_eq!(
            pane.query_descendants()
                .match_type_name("AiBadgeView")
                .find_all()
                .len(),
            1,
            "{lang} の AI の作業の枠に「{badge_text}」バッジが出ること"
        );
        window.set_right_pane_ai_work(false);
        settle();

        // ---- バッジの読み上げ: 見える「AI」を二重に読ませない ----
        // バッジの内側で読み上げに出る文字は「AIメンバー」/"AI member" だけ
        // （見える文字の「AI」をそのまま読み上げ名にしない）。
        let badges =
            ElementHandle::find_by_element_type_name(&window, "AiBadgeView").collect::<Vec<_>>();
        assert!(
            badges.len() >= 4,
            "{lang} の画面に「{badge_text}」バッジが複数出ていること: {}",
            badges.len()
        );
        for badge in &badges {
            assert_eq!(
                descendant_labels(badge),
                vec![badge_a11y.clone()],
                "{lang} のバッジの読み上げは「{badge_a11y}」だけであること（「AI」を二重に読ませない）"
            );
        }
    }
}

/// `.slint` 上の契約: 常時バナーの文言は三点リーダーで省略しないこと。
#[test]
fn banner_text_is_never_elided() {
    let block = component_block(SLINT, "DemoBannerView");
    assert!(
        !block.contains("elide"),
        "常時バナーの文言に overflow: elide を付けない（省略禁止。折り返しのみ）"
    );
}

/// `.slint` 上の契約: AI バッジの読み上げ名を出すこと・隠す仕組みを作らないこと。
#[test]
fn ai_badge_has_no_hide_mechanism() {
    let block = component_block(SLINT, "AiBadgeView");
    // 読み上げ名を出す（見える「AI」をそのまま二重に読ませない）。読み上げの
    // 全文の適用は Issue 379、ここではバッジの読み上げ名だけを契約にする。
    assert!(
        block.contains("accessible-label"),
        "AiBadgeView の読み上げ名（accessible-label）を出すこと"
    );
    assert!(
        !block.contains("in property <bool>"),
        "AiBadgeView に表示/非表示を切るプロパティを付けないこと"
    );
    for pat in [
        "hide-badge",
        "show-badge",
        "badge-visible",
        "hide-ai-badge",
        "ai-badge-visible",
    ] {
        assert!(
            !SLINT.contains(pat),
            "main.slint にバッジを隠す仕組み（{pat}）を作らないこと"
        );
    }
}
