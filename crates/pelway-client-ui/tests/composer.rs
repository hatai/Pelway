//! 入力欄（チャンネル・スレッド）とメンション候補一覧の結線の検査（Issue 383。
//! テスト用 backend。窓は開かない）。
//!
//! - 入力欄に文字を入れられ、入力値が残ること（backend が送るのと同じ経路で
//!   `WindowEvent::KeyPressed` を dispatch する）。
//! - IME 変換中の Enter で送信回数が増えないこと（橋の試験と画面 callback の両経路）。
//! - 候補一覧の QA-MEN-01: 開いている間の Enter は候補を 1 つ入れて一覧を閉じる
//!   （送信しない）、Esc は入力文字を残して一覧だけ閉じる、閉じたあとの修飾なし
//!   Enter は送信 1 回。
//! - Shift+Enter は送信せず改行になること（on_key の「何もしない」を TextInput が
//!   改行として処理する）。
//!
//! 窓を作る検査は末尾の 1 テストにまとめる（tests/pending_decisions.rs と同じ理由）。

use std::cell::RefCell;
use std::rc::Rc;

use i_slint_backend_testing::ElementHandle;
use pelway_client_core::approval::CardView;
use pelway_client_core::contract::{CardId, TaskState};
use pelway_client_core::input::{Action, ImeState, Key, Modifiers};
use pelway_client_ui::bridge::{ShellBridge, wire_composers};
use pelway_client_ui::{ComposerKind, MainWindow};
use slint::ComponentHandle;
use slint::platform::{Key as SlintKey, PointerEventButton, WindowEvent};

fn awaiting_card() -> CardView {
    CardView {
        card_id: CardId::from("card-composer"),
        card_version: 1,
        snapshot_hash: "hash-composer".to_owned(),
        state: TaskState::AwaitingSendApproval,
    }
}

// ---- 橋の試験（窓を作らない） ----

/// 2 つの入力欄は別々の本文を持ち、末尾の @… トークンで候補一覧が開く。
#[test]
fn composer_text_is_recorded_and_mention_list_follows_trailing_at_token() {
    let mut shell = ShellBridge::new(awaiting_card());
    shell.note_composer_text(ComposerKind::Channel, "こんにちは");
    assert_eq!(
        shell.composer_text(ComposerKind::Channel),
        "こんにちは",
        "入力した文字が残る"
    );
    assert!(!shell.mention_list_open_for(ComposerKind::Channel));

    // スレッド側は別の本文を持つ。
    shell.note_composer_text(ComposerKind::Thread, "返信です");
    assert_eq!(shell.composer_text(ComposerKind::Thread), "返信です");
    assert_eq!(
        shell.composer_text(ComposerKind::Channel),
        "こんにちは",
        "チャンネル側は残る"
    );

    // 末尾の @… トークンがある間だけ一覧が開く。
    shell.note_composer_text(ComposerKind::Channel, "こんにちは @");
    assert!(
        shell.mention_list_open_for(ComposerKind::Channel),
        "@ の直後で開く"
    );
    shell.note_composer_text(ComposerKind::Channel, "こんにちは @リ");
    assert!(shell.mention_list_open_for(ComposerKind::Channel));
    // @ のあとに空白が入ればトークンは終わり、一覧は閉じる。
    shell.note_composer_text(ComposerKind::Channel, "こんにちは @リク です");
    assert!(!shell.mention_list_open_for(ComposerKind::Channel));
    // スレッド側の一覧は独立している。
    assert!(!shell.mention_list_open_for(ComposerKind::Thread));
}

/// IME 変換中（ImeState::Composing）の Enter では送信回数が増えない。
#[test]
fn ime_composing_enter_does_not_send() {
    let mut shell = ShellBridge::new(awaiting_card());
    let action = shell.handle_composer_key(
        ComposerKind::Channel,
        Key::Enter,
        Modifiers::default(),
        ImeState::Composing,
    );
    assert_eq!(action, Action::None);
    assert_eq!(shell.sent_messages(), 0, "変換中の Enter は送信しない");
}

/// QA-MEN-01: Enter=候補を 1 つ入れて閉じる / Esc=入力を残して閉じる /
/// 閉じたあとの Enter=送信 1 回。
#[test]
fn mention_list_enter_accept_escape_dismiss_and_send_after_close() {
    let mut shell = ShellBridge::new(awaiting_card());
    shell.note_composer_text(ComposerKind::Channel, "@");
    assert!(shell.mention_list_open_for(ComposerKind::Channel));

    // 開いている間の Enter: 候補を 1 つ入れて一覧を閉じる。送信しない。
    let action = shell.handle_composer_key(
        ComposerKind::Channel,
        Key::Enter,
        Modifiers::default(),
        ImeState::NotComposing,
    );
    assert_eq!(action, Action::AcceptMention);
    assert_eq!(shell.sent_messages(), 0);
    assert!(!shell.mention_list_open_for(ComposerKind::Channel));
    assert_eq!(
        shell.composer_text(ComposerKind::Channel),
        "@田中 ",
        "先頭の候補が本文に入る"
    );

    // もう一度開いて Esc: 入力文字を残して一覧だけ閉じる。
    shell.note_composer_text(ComposerKind::Channel, "@リ");
    assert!(shell.mention_list_open_for(ComposerKind::Channel));
    let action = shell.handle_composer_key(
        ComposerKind::Channel,
        Key::Escape,
        Modifiers::default(),
        ImeState::NotComposing,
    );
    assert_eq!(action, Action::DismissMentionList);
    assert!(!shell.mention_list_open_for(ComposerKind::Channel));
    assert_eq!(
        shell.composer_text(ComposerKind::Channel),
        "@リ",
        "入力文字は残る"
    );

    // 一覧が閉じたあとの修飾なし Enter は送信 1 回。
    let action = shell.handle_composer_key(
        ComposerKind::Channel,
        Key::Enter,
        Modifiers::default(),
        ImeState::NotComposing,
    );
    assert_eq!(action, Action::Send);
    assert_eq!(shell.sent_messages(), 1);
}

// ---- 画面の検査（窓はこのテストで 1 つだけ作る） ----

/// 条件付き要素（`if` の中の画面部品）の生成・消去を即時に反映させる。
fn settle() {
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(0));
}

/// backend が送るのと同じ経路でキーイベントを窓に送る
/// （修飾キーはこのイベントで窓の修飾状態が更新される）。
fn press_key(window: &MainWindow, key: impl Into<char>) {
    window.window().dispatch_event(WindowEvent::KeyPressed {
        text: key.into().to_string().into(),
    });
}

fn release_key(window: &MainWindow, key: impl Into<char>) {
    window.window().dispatch_event(WindowEvent::KeyReleased {
        text: key.into().to_string().into(),
    });
}

/// 文字列を 1 文字ずつキーイベントとして打ち込む。
fn type_text(window: &MainWindow, text: &str) {
    for c in text.chars() {
        press_key(window, c);
    }
}

#[test]
fn composer_screen_end_to_end() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(i_slint_backend_testing::init_no_event_loop);

    let shell = Rc::new(RefCell::new(ShellBridge::new(awaiting_card())));
    let window = MainWindow::new().expect("MainWindow を作れること（テスト用 backend）");
    // チャンネルの画面を出す（既定は「今日」の画面で入力欄が無い）。
    window.set_show_today(false);
    wire_composers(&window, Rc::clone(&shell));
    settle();

    // ---- チャンネル入力に文字を入れられ、入力値が残る ----
    let composer = ElementHandle::find_by_element_id(&window, "ChannelView::composer-input")
        .next()
        .expect("チャンネルの入力欄があること");
    composer.mock_single_click(PointerEventButton::Left);
    type_text(&window, "hello");
    assert_eq!(window.get_composer_text().as_str(), "hello");
    assert_eq!(
        shell.borrow().composer_text(ComposerKind::Channel),
        "hello",
        "橋にも本文が残る"
    );

    // ---- スレッド入力は別の値を持つ ----
    let reply = ElementHandle::find_by_element_id(&window, "ThreadPaneView::reply-input")
        .next()
        .expect("スレッドの入力欄があること");
    reply.mock_single_click(PointerEventButton::Left);
    type_text(&window, "world");
    assert_eq!(window.get_reply_text().as_str(), "world");
    assert_eq!(
        window.get_composer_text().as_str(),
        "hello",
        "チャンネル側は残る"
    );

    // ---- 修飾なし Enter で送信 1 回（改行は入らない） ----
    press_key(&window, SlintKey::Return);
    assert_eq!(shell.borrow().sent_messages(), 1, "Enter は送信 1 回");
    assert_eq!(
        window.get_reply_text().as_str(),
        "world",
        "送信で改行は入らない"
    );

    // ---- Shift+Enter は送信せず改行 ----
    press_key(&window, SlintKey::Shift);
    press_key(&window, SlintKey::Return);
    release_key(&window, SlintKey::Shift);
    assert_eq!(
        shell.borrow().sent_messages(),
        1,
        "Shift+Enter は送信しない"
    );
    assert_eq!(window.get_reply_text().as_str(), "world\n", "改行が入る");

    // ---- メンション候補: @ で開き、Enter で候補を 1 つ入れて閉じる（送信しない） ----
    type_text(&window, "@");
    assert!(window.get_reply_mention_list_open(), "@ で候補一覧が開く");
    press_key(&window, SlintKey::Return);
    assert!(!window.get_reply_mention_list_open(), "Enter で閉じる");
    assert_eq!(
        shell.borrow().sent_messages(),
        1,
        "一覧を開いた Enter は送信しない"
    );
    assert!(
        window.get_reply_text().ends_with("@田中 "),
        "先頭の候補が本文に入る: {}",
        window.get_reply_text()
    );

    // ---- Esc: 入力文字を残して一覧だけ閉じる ----
    type_text(&window, "@リ");
    assert!(window.get_reply_mention_list_open());
    press_key(&window, SlintKey::Escape);
    assert!(!window.get_reply_mention_list_open());
    assert!(
        window.get_reply_text().ends_with("@リ"),
        "入力文字は残る: {}",
        window.get_reply_text()
    );

    // ---- 閉じたあとの修飾なし Enter は送信 1 回 ----
    press_key(&window, SlintKey::Return);
    assert_eq!(shell.borrow().sent_messages(), 2);

    // ---- IME 変換中の Enter は送信しない（callback 経路で変換中を渡す） ----
    // 変換中のキーは IME のもの: 消費はするが送信も改行挿入もしない。
    let consumed =
        window.invoke_composer_key_pressed(ComposerKind::Thread, "\n".into(), false, false, true);
    assert!(consumed, "変換中の Enter は消費する（IME のキー）");
    assert_eq!(
        shell.borrow().sent_messages(),
        2,
        "変換中の Enter は送信しない"
    );
    assert!(
        !window.get_reply_text().ends_with('\n'),
        "変換中の Enter で改行は入らない: {}",
        window.get_reply_text()
    );
}
