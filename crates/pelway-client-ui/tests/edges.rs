//! UI 層の未カバー経路の補強（Issue 160）。表示用の状態関数・アクセサ・テスト用の
//! 固定部品の既定値を確かめる。中核のロジックは `pelway-client-core` 側の
//! テストが担う（ここでは橋のふるまいだけを見る）。

use pelway_client_core::approval::{CardView, KeySource};
use pelway_client_core::contract::{CardId, TaskState};
use pelway_client_core::input::{Focus, ImeState, Key, Modifiers};
use pelway_client_ui::bridge::ShellBridge;
use pelway_client_ui::state::{AiPresence, AvatarKind, TodaySectionKind, ai_presence_key};
use pelway_client_ui::testkit::{FixedClock, FixedKeySource};

fn awaiting_card() -> CardView {
    CardView {
        card_id: CardId::from("card-edge"),
        card_version: 1,
        snapshot_hash: "hash-edge".to_owned(),
        state: TaskState::AwaitingSendApproval,
    }
}

// アバターの頭文字は人・AI どちらも中の文字列を返す。
#[test]
fn avatar_kind_initial_and_is_ai() {
    let human = AvatarKind::Human("は".to_owned());
    let ai = AvatarKind::Ai("デ".to_owned());
    assert_eq!(human.initial(), "は");
    assert_eq!(ai.initial(), "デ");
    assert!(!human.is_ai());
    assert!(ai.is_ai());
}

// 在席状態の文言キーは 3 値それぞれ安定している。
#[test]
fn ai_presence_key_maps_each_state() {
    assert_eq!(ai_presence_key(AiPresence::Standby), "ai-presence-standby");
    assert_eq!(ai_presence_key(AiPresence::Working), "ai-presence-working");
    assert_eq!(
        ai_presence_key(AiPresence::WaitingDecision),
        "ai-presence-waiting"
    );
}

// 「今日」の区切りの見出しキーは 3 種それぞれ安定している。
#[test]
fn today_section_heading_key_maps_each_kind() {
    assert_eq!(
        TodaySectionKind::Consultations.heading_key(),
        "today-section-consultations"
    );
    assert_eq!(
        TodaySectionKind::Pending.heading_key(),
        "today-section-pending"
    );
    assert_eq!(
        TodaySectionKind::DecidedWhileAway.heading_key(),
        "today-section-decided"
    );
}

// FixedKeySource の Default は new と同じく 1 から順に振る。
#[test]
fn fixed_key_source_default_behaves_like_new() {
    let mut a = FixedKeySource::default();
    let mut b = FixedKeySource::new();
    assert_eq!(a.fresh_key(), b.fresh_key());
    assert_eq!(a.fresh_key(), b.fresh_key());
}

// ShellBridge のフォーカス・IME の読み書きと、承認カードの橋の読み取り専用参照。
#[test]
fn shell_focus_ime_and_approval_ref() {
    let mut shell = ShellBridge::new(awaiting_card());
    shell.set_focus(Focus::MessageInput);
    assert_eq!(shell.focus(), Focus::MessageInput);
    shell.set_focus(Focus::ApproveButton);
    assert_eq!(shell.focus(), Focus::ApproveButton);

    shell.set_ime(ImeState::Composing);
    shell.set_ime(ImeState::NotComposing);

    // 読み取り専用の参照からも同じ状態が見える。
    let approval = shell.approval_ref();
    assert_eq!(approval.approve_request_count(), 0);
    assert!(approval.last_effect().is_none());
    // FixedClock はテスト用の時計（ここでは生成だけ確認する）。
    let _clock = FixedClock::new(0);
}

// MessageInput フォーカスの Enter は Action::Send を適用し、送信回数が増える。
#[test]
fn enter_on_message_input_counts_a_send() {
    let mut shell = ShellBridge::new(awaiting_card());
    shell.set_focus(Focus::MessageInput);
    let action = shell.handle_key(Key::Enter, Modifiers::default());
    assert_eq!(action, pelway_client_core::input::Action::Send);
    assert_eq!(shell.sent_messages(), 1);
}
