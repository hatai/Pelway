//! 「今日・判断待ち」画面とチャット内の判断カードの検査（Issue 331。テスト用 backend。
//! 窓は開かない）。
//!
//! 画面操作は、Slint の TouchArea が発行するのと同じ callback を Rust 側から
//! `invoke_*` で発行して確かめる（`decide-option`・`open-decision-chat`）。
//! テスト用 backend の winit のイベントループは 1 プロセスに 1 つしか作れない
//! ため、窓を作る検査は 1 テストにまとめる（他の検査は窓を作らない）。

use std::cell::RefCell;
use std::rc::Rc;

use pelway_client_ui::MainWindow;
use pelway_client_ui::bridge::{DecisionShell, apply_decision_screen, wire_decision_screen};
use slint::Model;

/// デモの判断待ちが待ち始めてから 25 分後の時刻（一覧の待ち時間「25分」用）。
const NOW: u64 = 25 * 60 * 1000;

/// 一覧の項目に依頼者・判断内容（論点）・待ち時間・対象チャネルが出ること
/// （窓を作らない、橋の読み取りの検査）。
#[test]
fn pending_rows_show_requester_question_wait_and_channel() {
    let shell = DecisionShell::sample("ja");
    let rows = shell.pending_rows(NOW);
    assert_eq!(rows.len(), 1, "判断待ち1件");
    let row = &rows[0];
    assert_eq!(row.requester, "ユイ", "依頼者の表示名");
    assert!(row.avatar.is_ai(), "依頼者は AI Bot");
    assert_eq!(row.channel, "# 案件B", "対象チャネルの表示名");
    assert_eq!(row.wait_text, "25分", "待ち時間の表示");
    assert!(row.question.contains("回答"), "論点（判断内容）");
    assert_eq!(row.options.len(), 2, "A/B の2択");
    assert_eq!(row.options[0].option_id, "a");
    assert_eq!(row.version, 1);
}

/// 「自分の依頼」の区切りには依頼と進み具合が出ること（窓を作らない）。
#[test]
fn request_rows_show_request_progress() {
    let shell = DecisionShell::sample("ja");
    let rows = shell.request_rows();
    assert_eq!(rows.len(), 1, "依頼1件");
    assert!(!rows[0].text.is_empty(), "依頼の内容");
    assert!(!rows[0].state_text.is_empty(), "進み具合の表示");
}

/// 画面の一連の操作。窓はこのテストで 1 つだけ作る（上の doc コメント参照）。
#[test]
fn pending_screen_end_to_end() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(i_slint_backend_testing::init_no_event_loop);

    let shell = Rc::new(RefCell::new(DecisionShell::sample("ja")));
    let window = MainWindow::new().expect("MainWindow を作れること（テスト用 backend）");
    window.set_show_today(true);
    wire_decision_screen(&window, Rc::clone(&shell), NOW);

    // ---- 一覧に依頼者・判断内容・待ち時間・対象チャネルが出る ----
    let model = window.get_pending_items();
    assert_eq!(model.row_count(), 1, "画面の一覧に1件");
    let item = model.row_data(0).unwrap();
    assert_eq!(item.requester.as_str(), "ユイ");
    assert_eq!(item.wait.as_str(), "25分");
    assert_eq!(item.channel.as_str(), "# 案件B");
    assert_eq!(item.options.row_count(), 2, "A/B の2択");
    let decision_id = item.decision_id;

    // 自分の依頼の一覧にも値が入る。
    let requests = window.get_request_items();
    assert_eq!(requests.row_count(), 1);
    assert!(!requests.row_data(0).unwrap().progress.is_empty());

    // ---- 一覧の項目から対象のチャットへ戻れる ----
    window.invoke_open_decision_chat(decision_id.clone());
    assert!(!window.get_show_today(), "チャットの画面に戻ること");
    assert_eq!(
        window.get_data_channel_title().as_str(),
        "# 案件B",
        "移った先の題名"
    );

    // チャット内の判断カードは同じ判断待ちを指す（一覧の項目と同じ id・版・2 択）。
    assert!(window.get_data_decision_visible(), "カードが見える");
    assert!(
        !window.get_data_decision_decided(),
        "カードは判断待ちのまま"
    );
    assert_eq!(window.get_data_decision_id(), decision_id);
    assert_eq!(window.get_data_decision_version(), 1);
    assert_eq!(window.get_data_decision_options().row_count(), 2);
    window.set_show_today(true);

    // ---- 古い版での確定は拒まれ、一覧とカードは変わらない ----
    window.invoke_decide_option(decision_id.clone(), "a".into(), 99);
    assert_eq!(window.get_pending_items().row_count(), 1, "一覧に残る");
    assert!(
        !window.get_data_decision_decided(),
        "カードは判断待ちのまま"
    );
    assert_eq!(
        shell.borrow().decide_request_count(),
        1,
        "要求の組み立ては起きる"
    );

    // ---- チャット内の判断カードから A/B を決める（カードの id・版で送る） ----
    let card_id = window.get_data_decision_id();
    let card_version = window.get_data_decision_version();
    window.invoke_decide_option(card_id, "a".into(), card_version);

    // 項目は一覧から消え、確定要求は「見ていた版」で組み立てられる。
    assert_eq!(
        window.get_pending_items().row_count(),
        0,
        "一覧から消えること"
    );
    assert_eq!(
        shell.borrow().decide_request_count(),
        2,
        "確定要求はもう1件"
    );
    let sent = shell.borrow().last_decide_request().unwrap().clone();
    assert_eq!(sent.version, 1, "画面で見ていた版");
    assert_eq!(sent.option_id.as_str(), "a");

    // チャット内の判断カードは確定表示に変わる（確定内容・決定者・版）。
    assert!(window.get_data_decision_visible(), "カードは残る");
    assert!(window.get_data_decision_decided(), "確定表示に変わる");
    assert_eq!(window.get_data_decision_decider().as_str(), "濱田");
    assert_eq!(window.get_data_decision_version(), 2, "確定後の版");
    assert_eq!(
        window.get_data_decision_selected().as_str(),
        "範囲を絞って12/1で送る"
    );

    // 正本は台帳——再適用しても一覧に戻らない。
    apply_decision_screen(&window, &shell.borrow(), NOW);
    assert_eq!(window.get_pending_items().row_count(), 0);
}
