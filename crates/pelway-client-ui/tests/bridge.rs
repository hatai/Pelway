//! UI 側の「つなぎ」のテスト。中核（pelway-client-core）のテストを書き写すのではなく、
//! `on_key` が返す `Action` だけで UI の状態がどう動くかを確かめる。
//!
//! - 候補一覧が開いているときの Enter で送信しないこと。
//! - 「送る」が押せないカードでは承認の要求（Effect）が出ないこと。

use pelway_client_core::approval::CardView;
use pelway_client_core::contract::{
    ApproveError, ApproveOutcome, CardId, SimulatedSendId, TaskState,
};
use pelway_client_ui::bridge::{ApprovalCardBridge, ShellBridge};
use pelway_client_ui::testkit::{FixedClock, FixedKeySource};

fn awaiting_card() -> CardView {
    CardView {
        card_id: CardId::from("card-1"),
        card_version: 3,
        snapshot_hash: "hash-1".to_owned(),
        state: TaskState::AwaitingSendApproval,
    }
}

fn sending_card() -> CardView {
    CardView {
        card_id: CardId::from("card-2"),
        card_version: 1,
        snapshot_hash: "hash-2".to_owned(),
        state: TaskState::Sending,
    }
}

#[test]
fn mention_list_open_enter_does_not_send_and_closes_the_list() {
    let mut shell = ShellBridge::new(awaiting_card());
    shell.set_focus(pelway_client_core::input::Focus::MessageInput);
    shell.set_mention_list_open(true);

    // 候補一覧が開いている間の Enter は候補を 1 つ入れて一覧を閉じる。送信しない。
    let action = shell.handle_key(
        pelway_client_core::input::Key::Enter,
        pelway_client_core::input::Modifiers::default(),
    );
    assert_eq!(action, pelway_client_core::input::Action::AcceptMention);
    assert_eq!(
        shell.sent_messages(),
        0,
        "候補一覧が開いた Enter で送信してはならない"
    );
    assert!(!shell.mention_list_open(), "AcceptMention で一覧は閉じる");

    // 一覧が閉じたあとの Enter は送信 1 回だけ。
    let action = shell.handle_key(
        pelway_client_core::input::Key::Enter,
        pelway_client_core::input::Modifiers::default(),
    );
    assert_eq!(action, pelway_client_core::input::Action::Send);
    assert_eq!(shell.sent_messages(), 1);
}

#[test]
fn mention_list_open_escape_closes_without_sending() {
    let mut shell = ShellBridge::new(awaiting_card());
    shell.set_focus(pelway_client_core::input::Focus::MessageInput);
    shell.set_mention_list_open(true);

    let action = shell.handle_key(
        pelway_client_core::input::Key::Escape,
        pelway_client_core::input::Modifiers::default(),
    );
    assert_eq!(
        action,
        pelway_client_core::input::Action::DismissMentionList
    );
    assert_eq!(shell.sent_messages(), 0);
    assert!(!shell.mention_list_open());
}

#[test]
fn send_button_not_enabled_card_emits_no_approval_request() {
    // カードが送信承認待ちでなければ「送る」は押せず、押しても要求が出ない。
    let mut bridge =
        ApprovalCardBridge::new(sending_card(), FixedKeySource::new(), FixedClock::new(0));
    assert!(!bridge.send_button_enabled());
    assert!(bridge.press_send_button().is_none());
    assert_eq!(
        bridge.approve_request_count(),
        0,
        "押せないカードでは承認の要求が出ない"
    );
    assert!(bridge.last_effect().is_none());
    // 冪等キーも消費しない（中核が保証する。ここでは要求回数 0 で見る）。
}

#[test]
fn approve_button_key_on_disabled_card_emits_no_approval_request() {
    // 承認ボタンにフォーカスした Enter は Action::Approve を返すが、
    // カードが押せなければ UI のつなぎは何も要求しない。
    let mut shell = ShellBridge::new(sending_card());
    shell.set_focus(pelway_client_core::input::Focus::ApproveButton);
    let action = shell.handle_key(
        pelway_client_core::input::Key::Enter,
        pelway_client_core::input::Modifiers::default(),
    );
    assert_eq!(action, pelway_client_core::input::Action::Approve);
    assert_eq!(shell.approval().approve_request_count(), 0);
    assert!(shell.approval().last_effect().is_none());
}

#[test]
fn approve_button_key_on_awaiting_card_emits_exactly_one_request() {
    let mut shell = ShellBridge::new(awaiting_card());
    shell.set_focus(pelway_client_core::input::Focus::ApproveButton);
    let action = shell.handle_key(
        pelway_client_core::input::Key::Space,
        pelway_client_core::input::Modifiers::default(),
    );
    assert_eq!(action, pelway_client_core::input::Action::Approve);
    assert_eq!(shell.approval().approve_request_count(), 1);
    assert!(matches!(
        shell.approval().last_effect(),
        Some(pelway_client_core::approval::Effect::Approve(_))
    ));
    // 応答待ちの間はもう押せない（二重に要求を出さない）。
    assert!(!shell.approval().send_button_enabled());
    let again = shell.handle_key(
        pelway_client_core::input::Key::Enter,
        pelway_client_core::input::Modifiers::default(),
    );
    assert_eq!(again, pelway_client_core::input::Action::Approve);
    assert_eq!(
        shell.approval().approve_request_count(),
        1,
        "応答待ちの再押下では要求を増やさない"
    );
}

#[test]
fn effect_is_kept_but_not_executed_untouched_wiring() {
    // Effect は「未接続」: 通信を書かないため、受け取って保持するだけ。
    // press → last_effect に残るが、承認の応答を戻さなければ表示は送信済みに移らない。
    let mut bridge =
        ApprovalCardBridge::new(awaiting_card(), FixedKeySource::new(), FixedClock::new(0));
    let effect = bridge.press_send_button();
    assert!(matches!(
        effect,
        Some(pelway_client_core::approval::Effect::Approve(_))
    ));
    assert!(
        !bridge.sent(),
        "応答を戻す前に送信済み表示へ移ってはならない"
    );
}

#[test]
fn approve_response_moves_card_to_sent_view_state() {
    let mut bridge =
        ApprovalCardBridge::new(awaiting_card(), FixedKeySource::new(), FixedClock::new(0));
    bridge.press_send_button();
    let effect = bridge.on_approve_response(Ok(ApproveOutcome {
        card_id: CardId::from("card-1"),
        card_version: 4,
        state: TaskState::SentAwaitingReply,
        simulated_send_id: SimulatedSendId::from("send-1"),
    }));
    assert!(effect.is_none());
    assert!(bridge.sent(), "承認が済んだら送信後の表示へ移る");
    assert!(!bridge.send_button_enabled());
}

#[test]
fn comment_editable_is_exposed_as_view_state_only() {
    // モックにコメント欄の見た目がないため画面には置かないが、
    // 表示用の状態（comment_editable）は橋が持っている。
    let awaiting =
        ApprovalCardBridge::new(awaiting_card(), FixedKeySource::new(), FixedClock::new(0));
    assert!(awaiting.comment_editable());
    let sending =
        ApprovalCardBridge::new(sending_card(), FixedKeySource::new(), FixedClock::new(0));
    assert!(!sending.comment_editable());
}

#[test]
fn comment_box_enter_only_inserts_newline_and_never_sends() {
    // コメント欄にフォーカスがある Enter は改行 1 つだけ。送信・承認に結びつかない。
    let mut shell = ShellBridge::new(awaiting_card());
    shell.set_focus(pelway_client_core::input::Focus::CommentBox);
    let action = shell.handle_key(
        pelway_client_core::input::Key::Enter,
        pelway_client_core::input::Modifiers::default(),
    );
    assert_eq!(action, pelway_client_core::input::Action::InsertNewline);
    assert_eq!(shell.composer_newlines(), 1);
    assert_eq!(shell.sent_messages(), 0);
    assert_eq!(shell.approval().approve_request_count(), 0);
}

#[test]
fn version_conflict_response_keeps_card_unsent_and_button_disabled() {
    // 409 version_conflict では送信済み表示へ移さず、読み直し待ち（押せない）になる。
    let mut bridge =
        ApprovalCardBridge::new(awaiting_card(), FixedKeySource::new(), FixedClock::new(0));
    bridge.press_send_button();
    let effect =
        bridge.on_approve_response(Err(ApproveError::VersionConflict { current_version: 4 }));
    assert!(matches!(
        effect,
        Some(pelway_client_core::approval::Effect::Notice(
            pelway_client_core::approval::Notice::NeedsReload { .. }
        ))
    ));
    assert!(!bridge.sent());
    assert!(!bridge.send_button_enabled());
}
