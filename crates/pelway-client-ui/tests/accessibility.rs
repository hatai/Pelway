//! 読み上げ（accessible-role / accessible-label）・フォーカス表示・キーボード
//! 操作の検査（Issue 379。テスト用 backend。窓は開かない）。
//!
//! - 押せる部品に button の role と Fluent の文言に一致するラベル、一覧の項目に
//!   list-item、判断カードに group があること。
//! - 読み上げ名は「表示名＋AI Bot / 人」で人と AI が区別でき、見える「AI」の
//!   印は二度読まれないこと。
//! - 状態は色ではなく文字として存在すること。
//! - Tab でフォーカスが当たると DesignTokens.focus-ring の枠（FocusRing）が
//!   出て、Enter/Space が中核の on_key を通ること（修飾付き・IME 変換中は決定しない）。
//!
//! 検査は 1 テスト関数にまとめる（モック時計はスレッドをまたぐと単調性の検査に
//! 落ちるため。tests/slint_attribution.rs と同じ約束）。

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use i_slint_backend_testing::{AccessibleRole, ElementHandle, ElementQuery};
use pelway_client_core::input::ImeState;
use pelway_client_ui::bridge::{DecisionShell, wire_decision_screen, wire_key_actions};
use pelway_client_ui::i18n::{FluentVars, Messages, apply_translations};
use pelway_client_ui::theme::{Theme, apply_design_tokens};
use pelway_client_ui::{KeyRouter, MainWindow};
use slint::language::PointerEventButton;
use slint::platform::WindowEvent;
use slint::{ComponentHandle, Global, Model};

/// デモの判断待ちが待ち始めてから 25 分後の時刻（pending_decisions.rs と同じ）。
const NOW: u64 = 25 * 60 * 1000;

/// テスト用 backend の初期化をプロセスで 1 回だけ行う
/// （`tests/slint_window.rs` と同じ理由で `Once`）。
fn init_testing_backend_once() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(i_slint_backend_testing::init_no_event_loop);
}

/// ja の文言とデモの合成データを渡した `MainWindow` を作る。
fn window_with_texts() -> MainWindow {
    init_testing_backend_once();
    let window = MainWindow::new().expect("MainWindow を作れること（テスト用 backend）");
    apply_design_tokens(&window, Theme::Light);
    let messages = Messages::new("ja").expect("ja の bundle が作れること");
    let missing = apply_translations(&window, &messages, &FluentVars::sample());
    assert!(missing.is_empty(), "ja で渡せないキーがあった: {missing:?}");
    window
}

/// 条件付き要素（`if`）の生成・消去を即時に反映させる。
fn settle() {
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(0));
}

/// テスト用 backend の要素走査は「可視」の要素だけを拾う
/// （`visit_descendants` が `is_visible` で枝を切る）。既定の窓サイズだと
/// スクロール領域の下側（判断カード以降の行・社外送信カード・入力欄）が
/// 不可視扱いで走査に出ないので、全要素が入る高さに窓を広げる。
fn enlarge_window(window: &MainWindow) {
    window
        .window()
        .set_size(slint::LogicalSize::new(1600.0, 3200.0));
    settle();
}

/// ラベルが `label` で role が `role` の要素がちょうど `count` 個あること。
fn assert_role_label_count(window: &MainWindow, label: &str, role: AccessibleRole, count: usize) {
    let found = ElementHandle::find_by_accessible_label(window, label)
        .filter(|e| e.accessible_role() == Some(role))
        .count();
    assert_eq!(
        found, count,
        "「{label}」のラベルで role {role:?} の要素が {count} 個あること"
    );
}

/// ラベルが `label` の要素に「既定の操作」（支援技術の起動操作）を送る。
fn default_action_by_label(window: &MainWindow, label: &str) {
    let element = ElementHandle::find_by_accessible_label(window, label)
        .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
        .unwrap_or_else(|| panic!("「{label}」の押せる部品があること"));
    element.invoke_accessible_default_action();
}

/// `predicate` に合うラベル（どれか 1 つでも）があるか。
fn any_label(window: &MainWindow, predicate: impl Fn(&str) -> bool + 'static) -> bool {
    ElementQuery::from_root(window)
        .match_descendants()
        .match_predicate(move |e| e.accessible_label().is_some_and(|l| predicate(l.as_str())))
        .find_first()
        .is_some()
}

/// `label` と等しい読み上げ名が画面のテキストとしてあること。
fn assert_label(window: &MainWindow, label: &str) {
    let expected = label.to_owned();
    assert!(
        any_label(window, move |l| l == expected),
        "「{label}」が読み上げ名にあること"
    );
}

/// Tab を送ってフォーカスを進め、focus-ring の枠（FocusRing）が
/// `label` の部品の中に来るまで回す。到達したらその部品のハンドルを返す。
///
/// 注: Slint 1.18.1 の MenuBar の要素ツリーは窓の主ツリーに繋がっていない
/// （`parent_node` を持たない）。初期フォーカスは巡回の先頭にある
/// MenuBar 内の FocusScope に乗るが、そこから Tab の巡回が
/// サブツリー内で閉じてしまい窓の部品へ進めない（Slint 側の制約）。
/// そのため先に窓の主ツリー内の FocusScope にフォーカスを与えてから
/// Tab を回す（利用者が窓内の部品にいる状態に相当）。
fn tab_focus_to_label(window: &MainWindow, label: &str) -> ElementHandle {
    use slint::private_unstable_api::re_exports::*;
    use std::ops::ControlFlow;
    let inner = WindowInner::from_pub(window.window());
    {
        // 窓の主ツリー（component）内の最初の FocusScope にフォーカスを当てる
        // （メニューバーの別ツリー内のものは対象外）。
        let mut seed: Option<ItemRc> = None;
        ItemRc::new_root(inner.component()).visit_descendants(|item| {
            if seed.is_none()
                && item.downcast::<FocusScope>().is_some()
                && vtable::VRc::ptr_eq(item.item_tree(), &inner.component())
            {
                seed = Some(item.clone());
            }
            ControlFlow::<()>::Continue(())
        });
        inner.set_focus_item(
            &seed.expect("窓の主ツリーに FocusScope があること"),
            true,
            FocusReason::TabNavigation,
        );
    }
    for _ in 0..60 {
        settle();
        if let Some(ring) = ElementHandle::find_by_element_type_name(window, "FocusRing").next() {
            let ring_pos = ring.absolute_position();
            if let Some(button) = ElementHandle::find_by_accessible_label(window, label)
                .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
            {
                let pos = button.absolute_position();
                let size = button.size();
                if ring_pos.x >= pos.x
                    && ring_pos.x < pos.x + size.width
                    && ring_pos.y >= pos.y
                    && ring_pos.y < pos.y + size.height
                {
                    return button;
                }
            }
        }
        window
            .window()
            .dispatch_event(WindowEvent::KeyPressed { text: "\t".into() });
        window
            .window()
            .dispatch_event(WindowEvent::KeyReleased { text: "\t".into() });
    }
    panic!("Tab で「{label}」にフォーカスが当たること（60 回以内）");
}

#[test]
fn accessibility_roles_labels_focus_and_keys() {
    let window = window_with_texts();
    let shell = Rc::new(RefCell::new(DecisionShell::sample("ja")));
    let ime = Rc::new(Cell::new(ImeState::NotComposing));
    wire_decision_screen(&window, Rc::clone(&shell), NOW);
    wire_key_actions(&window, Rc::clone(&shell), Rc::clone(&ime), NOW);

    // デモの合成データ（state::sample と同じ架空の値）。
    window.set_data_org_name("株式会社サンプル開発".into());
    window.set_data_channel_1("# 案件A".into());
    window.set_data_channel_2("# 案件B".into());
    window.set_data_channel_3("# 全体".into());
    window.set_data_channel_4("# 雑談".into());
    window.set_data_channel_title("# 案件B".into());
    window.set_data_person_1("田中".into());
    window.set_data_person_2("佐藤".into());
    window.set_data_ai_1_name("ユイ".into());
    window.set_data_ai_1_presence("待機中".into());
    window.set_data_ai_2_name("リク".into());
    window.set_data_ai_2_presence("待機中".into());
    window.set_data_ai_3_name("ハル".into());
    window.set_data_ai_3_presence("判断待ち".into());
    window.set_data_msg1_sender("田中".into());
    window.set_data_msg1_initial("田".into());
    window.set_data_msg2_sender("リク".into());
    window.set_data_msg2_initial("リ".into());
    window.set_data_msg3_sender("佐藤".into());
    window.set_data_msg3_initial("佐".into());
    window.set_data_note_sender("リク".into());
    window.set_data_note_initial("リ".into());
    window.set_data_handoff_from_to("ユイ → ハル".into());
    window.set_data_handoff_title("B社の回答の出し方".into());
    window.set_data_handoff_acknowledged(false);
    window.set_data_ai_work_name("リク".into());
    window.set_data_ai_work_initial("リ".into());
    // スレッド枠（既定で開いている）の行にも送信者を入れる。
    // 既定値のままだと「 AI Bot」「 人」と空の名前で読み上げてしまう。
    window.set_data_thread_msg1_sender("ミカ".into());
    window.set_data_thread_msg1_initial("ミ".into());
    window.set_data_thread_msg2_sender("ケン".into());
    window.set_data_thread_msg2_initial("ケ".into());
    window.set_show_today(true);
    enlarge_window(&window);

    // ---- 「今日」の画面: 押せる部品に button の role と Fluent の文言 ----
    assert_role_label_count(&window, "今日", AccessibleRole::Button, 1);
    assert_role_label_count(&window, "この案に決める", AccessibleRole::Button, 2);
    assert_role_label_count(&window, "チャットで見る", AccessibleRole::Button, 1);
    assert_role_label_count(&window, "# 案件A", AccessibleRole::Button, 1);
    assert_role_label_count(&window, "田中", AccessibleRole::Button, 1);

    // 一覧の項目（今日の判断待ち・自分の依頼）は list-item。
    let question = window
        .get_pending_items()
        .row_data(0)
        .expect("判断待ちが1件")
        .question
        .to_string();
    assert_role_label_count(&window, question.as_str(), AccessibleRole::ListItem, 1);
    assert_role_label_count(
        &window,
        "B社の注文一覧へのCSV出力の実装",
        AccessibleRole::ListItem,
        1,
    );

    // rail の AI Bot の行は「名前＋AI Bot＋在席状態」で読み上げる list-item。
    let ai_rows: Vec<ElementHandle> =
        ElementHandle::find_by_element_type_name(&window, "RailAiRow").collect();
    assert_eq!(ai_rows.len(), 3, "rail の AI の行が 3 つ");
    let yui_label = ai_rows[0]
        .accessible_label()
        .expect("AI の行にラベルがあること");
    assert!(
        yui_label.contains("ユイ"),
        "AI の行のラベルに名前: {yui_label}"
    );
    assert!(
        yui_label.contains("AI Bot"),
        "AI の行のラベルに「AI Bot」: {yui_label}"
    );
    assert!(
        yui_label.contains("待機中"),
        "AI の行のラベルに在席状態: {yui_label}"
    );
    assert_eq!(ai_rows[0].accessible_role(), Some(AccessibleRole::ListItem));

    // メニュー項目は Slint のメニュー機構が title を読み上げ名に使う
    // （Menu/MenuItem には accessible-* を持てない。role は backend 任せ）。
    // メニューの項目は開いたときにだけ作られるので、まずメニューを開く
    // （tests/slint_attribution.rs と同じ手順）。
    ElementHandle::find_by_accessible_label(&window, "ヘルプ")
        .next()
        .expect("MenuBar に「ヘルプ」メニューがあること")
        .mock_single_click(PointerEventButton::Left);
    assert!(
        ElementHandle::find_by_accessible_label(&window, "このアプリについて")
            .next()
            .is_some(),
        "メニュー項目にラベル（Fluent の文言）があること"
    );

    // ---- 人と AI Bot の行・バッジの二度読み抑制（チャットの画面へ） ----
    window.set_show_today(false);
    settle();

    // AI の行の読み上げ: 「リク AI Bot」。人の行: 「田中 人」「佐藤 人」。
    assert_role_label_count(&window, "リク AI Bot", AccessibleRole::ListItem, 1);
    assert_role_label_count(&window, "田中 人", AccessibleRole::ListItem, 1);
    assert_role_label_count(&window, "佐藤 人", AccessibleRole::ListItem, 1);
    // スレッド枠の行も「名前＋種別」で読み上げる。
    assert_role_label_count(&window, "ミカ AI Bot", AccessibleRole::ListItem, 1);
    assert_role_label_count(&window, "ケン 人", AccessibleRole::ListItem, 1);
    // 人の行・要素のラベルに「AI」は混ざらない。
    assert!(
        !any_label(&window, |l| l == "田中 AI Bot" || l == "佐藤 AI Bot"),
        "人の行のラベルに「AI Bot」が混ざらないこと"
    );

    // アバターは画像として「表示名＋種別」を読み上げる。
    let avatars: Vec<ElementHandle> =
        ElementHandle::find_by_element_type_name(&window, "AvatarView").collect();
    assert!(!avatars.is_empty());
    let ai_avatar = avatars
        .iter()
        .find(|e| {
            e.accessible_label()
                .is_some_and(|l| l.as_str() == "リク AI Bot")
        })
        .expect("リクのアバターがあること");
    assert_eq!(ai_avatar.accessible_role(), Some(AccessibleRole::Image));
    assert!(
        avatars.iter().any(|e| e
            .accessible_label()
            .is_some_and(|l| l.as_str() == "田中 人")),
        "人のアバターのラベルは「名前＋人」"
    );

    // 見える「AI」の印は読み上げから外れる（バッジの文字だけが名前に
    // ならない）。
    assert_eq!(
        ElementHandle::find_by_accessible_label(&window, "AI").count(),
        0,
        "「AI」だけのラベルが存在しないこと"
    );

    // ---- チャットの画面の押せる部品 ----
    assert_role_label_count(&window, "送る", AccessibleRole::Button, 1);
    assert_role_label_count(&window, "直す", AccessibleRole::Button, 1);
    assert_role_label_count(&window, "受けました", AccessibleRole::Button, 1);
    // 「作業を見る」は ThreadLead とスレッドの枠の 2 か所。
    assert_role_label_count(&window, "作業を見る", AccessibleRole::Button, 2);
    assert_role_label_count(&window, "閉じる", AccessibleRole::Button, 1);
    // 判断カードはグループとして読み上げ、そのボタンも button。
    assert_role_label_count(&window, "決定の依頼", AccessibleRole::Groupbox, 1);
    assert_role_label_count(&window, "この案に決める", AccessibleRole::Button, 2);
    // 判断カードの依頼者（AI Bot のユイ）のアバターにも種別が載る。
    assert_label(&window, "ユイ AI Bot");

    // ---- 状態は色ではなく文字として存在する ----
    assert_label(&window, "未受領");
    assert_label(&window, "判断待ち");
    // 「作業中」: スレッドのまとめ（完了→作業中に切り替え）。
    // 注: Fluent は埋め込み値を双方向分離文字（U+2068/2069）で囲むので、
    // 変数を含む文言は部分一致で確かめる。
    window.set_data_msg2_thread_done(false);
    settle();
    assert!(
        any_label(&window, |l| l.contains("作業中") && l.contains("濱田")),
        "「{{名前}} 作業中」が画面のテキストとして存在すること"
    );
    assert!(
        !any_label(&window, |l| l.contains("完了")),
        "完了→作業中の切り替えで「完了」表示が（可視のまま）残らないこと"
    );
    // 「送信済み」: 社外送信カードの送信後の表示。
    window.set_data_ext_sent(true);
    settle();
    assert!(
        any_label(&window, |l| l.contains("送信")
            && l.contains("顧客回答待ち")),
        "送信済みの表示が画面のテキストとして存在すること"
    );
    // 受領済みに切り替える（「受領 · {名前} {時刻}」の表示）。
    window.set_data_handoff_acknowledged(true);
    settle();
    assert!(
        any_label(&window, |l| l.starts_with("受領 ·")),
        "受領済みの表示が画面のテキストとして存在すること"
    );
    window.set_data_ext_sent(false);
    window.set_data_handoff_acknowledged(false);
    settle();

    // ---- 支援技術の既定操作（activate）で部品が動く ----
    let today_pressed = Rc::new(Cell::new(false));
    let flag = Rc::clone(&today_pressed);
    window.on_today_pressed(move || flag.set(true));
    default_action_by_label(&window, "今日");
    settle();
    assert!(
        today_pressed.get(),
        "「今日」の行の既定操作で today-pressed が来る"
    );
    window.set_show_today(true);
    settle();

    // About の「閉じる」も既定操作で動く。
    // スレッド枠の「閉じる」と区別するため、先に右の枠を閉じておく。
    window.set_show_right_pane(false);
    settle();
    window.set_show_about(true);
    settle();
    default_action_by_label(&window, "閉じる");
    assert!(
        !window.get_show_about(),
        "About の「閉じる」の既定操作でダイアログが閉じること"
    );

    // ---- decide-key（on_key 経路）の単体検査 ----
    let decision_id = window
        .get_pending_items()
        .row_data(0)
        .expect("判断待ちが1件")
        .decision_id;
    let router = KeyRouter::get(&window);
    // 関係ないキーは消費しない（伝播させる）。
    assert!(
        !router.invoke_decide_key(decision_id.clone(), "a".into(), 1, "x".into(), false, false),
        "対象外のキーは消費しないこと"
    );
    // 修飾付き Enter は消費するが決定しない。
    assert!(
        router.invoke_decide_key(decision_id.clone(), "a".into(), 1, "\n".into(), true, false),
        "修飾付き Enter もボタンのキーとして消費すること"
    );
    assert_eq!(
        shell.borrow().decide_request_count(),
        0,
        "修飾付き Enter では決定しない"
    );
    // IME 変換中も消費するが決定しない。
    ime.set(ImeState::Composing);
    assert!(router.invoke_decide_key(
        decision_id.clone(),
        "a".into(),
        1,
        "\n".into(),
        false,
        false
    ));
    assert_eq!(
        shell.borrow().decide_request_count(),
        0,
        "IME 変換中の Enter では決定しない"
    );
    ime.set(ImeState::NotComposing);

    // ---- Tab で決定ボタンへフォーカス → focus-ring → Enter で決定 ----
    window.set_show_today(true);
    settle();
    tab_focus_to_label(&window, "この案に決める");
    window
        .window()
        .dispatch_event(WindowEvent::KeyPressed { text: "\n".into() });
    window
        .window()
        .dispatch_event(WindowEvent::KeyReleased { text: "\n".into() });
    settle();
    assert_eq!(
        shell.borrow().decide_request_count(),
        1,
        "フォーカスのある決定ボタンの Enter で決定になる"
    );
    assert_eq!(
        window.get_pending_items().row_count(),
        0,
        "決定した項目は一覧から消えること"
    );

    // 「送る」のキー操作も on_key を通る（send-key → Approve → send-pressed）。
    let send_pressed = Rc::new(Cell::new(false));
    let flag = Rc::clone(&send_pressed);
    window.on_send_pressed(move || flag.set(true));
    window.set_show_today(false);
    settle();
    assert!(
        router.invoke_send_key("\n".into(), false, false),
        "「送る」の Enter は消費すること"
    );
    assert!(
        send_pressed.get(),
        "on_key が Approve を返し「送る」の押下経路に届くこと"
    );
    send_pressed.set(false);
    assert!(
        router.invoke_send_key("\n".into(), true, false),
        "修飾付き Enter も消費する"
    );
    assert!(!send_pressed.get(), "修飾付き Enter では承認しないこと");
}
