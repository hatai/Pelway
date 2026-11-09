//! キー操作の判定（`src/input.rs`）のテスト。
//! QA-APR-01（コメント欄のキーで承認・送信を起こさない）と
//! QA-MEN-01（メンション候補一覧の Enter/Esc で送信しない）の仕様表。

use pelway_client_core::input::{Action, Focus, ImeState, Key, Modifiers, on_key};

const NO_MODS: Modifiers = Modifiers {
    ctrl_or_cmd: false,
    shift: false,
};
const CTRL_OR_CMD: Modifiers = Modifiers {
    ctrl_or_cmd: true,
    shift: false,
};
const SHIFT: Modifiers = Modifiers {
    ctrl_or_cmd: false,
    shift: true,
};

fn ctx(
    focus: Focus,
    ime: ImeState,
    mention_list_open: bool,
) -> pelway_client_core::input::InputContext {
    pelway_client_core::input::InputContext {
        focus,
        ime,
        mention_list_open,
    }
}

// ---- QA-APR-01: コメント欄 ----

/// コメント欄では、IME のどの状態でも Enter・Ctrl/Cmd+Enter・Space で承認も送信もしない。
#[test]
fn comment_box_never_approves_or_sends_on_any_ime_state() {
    for ime in [ImeState::Composing, ImeState::NotComposing] {
        for (key, mods) in [
            (Key::Enter, NO_MODS),
            (Key::Enter, CTRL_OR_CMD),
            (Key::Enter, SHIFT),
            (Key::Space, NO_MODS),
        ] {
            let action = on_key(&ctx(Focus::CommentBox, ime, false), key, mods);
            assert_ne!(action, Action::Approve, "{ime:?} {key:?} {mods:?}");
            assert_ne!(action, Action::Send, "{ime:?} {key:?} {mods:?}");
        }
    }
}

/// 確定後・無変換の Enter（修飾なし含む Enter 系）は改行を 1 つ入れる。
#[test]
fn comment_box_enter_after_ime_commit_inserts_one_newline() {
    assert_eq!(
        on_key(
            &ctx(Focus::CommentBox, ImeState::NotComposing, false),
            Key::Enter,
            NO_MODS
        ),
        Action::InsertNewline
    );
    // 修飾付き Enter も承認も送信もしない。この層では改行として扱う。
    assert_eq!(
        on_key(
            &ctx(Focus::CommentBox, ImeState::NotComposing, false),
            Key::Enter,
            CTRL_OR_CMD
        ),
        Action::InsertNewline
    );
    assert_eq!(
        on_key(
            &ctx(Focus::CommentBox, ImeState::NotComposing, false),
            Key::Enter,
            SHIFT
        ),
        Action::InsertNewline
    );
}

/// 変換中の Enter は IME の確定だけ。この層は何もしない（改行も入れない）。
#[test]
fn comment_box_enter_while_composing_does_nothing() {
    assert_eq!(
        on_key(
            &ctx(Focus::CommentBox, ImeState::Composing, false),
            Key::Enter,
            NO_MODS
        ),
        Action::None
    );
    assert_eq!(
        on_key(
            &ctx(Focus::CommentBox, ImeState::Composing, false),
            Key::Enter,
            CTRL_OR_CMD
        ),
        Action::None
    );
}

/// コメント欄の Space は空白入力（テキスト編集の既定）で、この層は何も指示しない。
#[test]
fn comment_box_space_types_a_blank_and_triggers_nothing() {
    assert_eq!(
        on_key(
            &ctx(Focus::CommentBox, ImeState::NotComposing, false),
            Key::Space,
            NO_MODS
        ),
        Action::None
    );
    assert_eq!(
        on_key(
            &ctx(Focus::CommentBox, ImeState::Composing, false),
            Key::Space,
            NO_MODS
        ),
        Action::None
    );
}

/// コメント欄の Esc には役割がない。
#[test]
fn comment_box_escape_does_nothing() {
    for ime in [ImeState::Composing, ImeState::NotComposing] {
        assert_eq!(
            on_key(&ctx(Focus::CommentBox, ime, false), Key::Escape, NO_MODS),
            Action::None
        );
    }
}

// ---- 承認ボタン ----

/// 承認は承認ボタンでの Enter・Space（修飾なし）だけ。どちらも承認を 1 回返す。
#[test]
fn approve_button_enter_and_space_approve_once() {
    assert_eq!(
        on_key(
            &ctx(Focus::ApproveButton, ImeState::NotComposing, false),
            Key::Enter,
            NO_MODS
        ),
        Action::Approve
    );
    assert_eq!(
        on_key(
            &ctx(Focus::ApproveButton, ImeState::NotComposing, false),
            Key::Space,
            NO_MODS
        ),
        Action::Approve
    );
}

/// 承認ボタンにフォーカスがあっても、修飾付きキーでは承認しない。
#[test]
fn approve_button_ignores_modified_keys() {
    for key in [Key::Enter, Key::Space] {
        assert_eq!(
            on_key(
                &ctx(Focus::ApproveButton, ImeState::NotComposing, false),
                key,
                CTRL_OR_CMD
            ),
            Action::None
        );
        assert_eq!(
            on_key(
                &ctx(Focus::ApproveButton, ImeState::NotComposing, false),
                key,
                SHIFT
            ),
            Action::None
        );
    }
}

/// 承認ボタンでの Esc は承認しない。
#[test]
fn approve_button_escape_does_nothing() {
    assert_eq!(
        on_key(
            &ctx(Focus::ApproveButton, ImeState::NotComposing, false),
            Key::Escape,
            NO_MODS
        ),
        Action::None
    );
}

// ---- QA-MEN-01: メッセージ入力欄とメンション候補一覧 ----

/// 候補一覧が開いている間の Enter は、候補を 1 つ入れて一覧を閉じ、送信しない。
#[test]
fn open_mention_list_enter_accepts_candidate_and_does_not_send() {
    let action = on_key(
        &ctx(Focus::MessageInput, ImeState::NotComposing, true),
        Key::Enter,
        NO_MODS,
    );
    assert_eq!(action, Action::AcceptMention);
    // 修飾付き Enter でも送信しない（候補確定が優先）。
    let action = on_key(
        &ctx(Focus::MessageInput, ImeState::NotComposing, true),
        Key::Enter,
        CTRL_OR_CMD,
    );
    assert_eq!(action, Action::AcceptMention);
}

/// Esc は一覧を閉じるだけ。何も入れず、入力済みの文字を残し、送信しない。
#[test]
fn open_mention_list_escape_closes_list_without_inserting_or_sending() {
    let action = on_key(
        &ctx(Focus::MessageInput, ImeState::NotComposing, true),
        Key::Escape,
        NO_MODS,
    );
    assert_eq!(action, Action::DismissMentionList);
}

/// 一覧が閉じたあとの Enter は送信を 1 回。
#[test]
fn closed_mention_list_enter_sends_once() {
    assert_eq!(
        on_key(
            &ctx(Focus::MessageInput, ImeState::NotComposing, false),
            Key::Enter,
            NO_MODS
        ),
        Action::Send
    );
}

/// IME 変換中の Enter・Esc は IME の操作（確定・キャンセル）を優先し、
/// 候補の挿入も一覧のクローズも送信もしない。
#[test]
fn composing_keys_go_to_ime_even_with_mention_list_open() {
    assert_eq!(
        on_key(
            &ctx(Focus::MessageInput, ImeState::Composing, true),
            Key::Enter,
            NO_MODS
        ),
        Action::None
    );
    assert_eq!(
        on_key(
            &ctx(Focus::MessageInput, ImeState::Composing, true),
            Key::Escape,
            NO_MODS
        ),
        Action::None
    );
}

/// メッセージ欄の修飾付き Enter（一覧が閉じている）の扱いは未決。
/// 誤送信防止のため送信しない（None）。Space は文字入力。
#[test]
fn message_input_modified_enter_and_space_do_nothing() {
    assert_eq!(
        on_key(
            &ctx(Focus::MessageInput, ImeState::NotComposing, false),
            Key::Enter,
            CTRL_OR_CMD
        ),
        Action::None
    );
    assert_eq!(
        on_key(
            &ctx(Focus::MessageInput, ImeState::NotComposing, false),
            Key::Enter,
            SHIFT
        ),
        Action::None
    );
    assert_eq!(
        on_key(
            &ctx(Focus::MessageInput, ImeState::NotComposing, false),
            Key::Space,
            NO_MODS
        ),
        Action::None
    );
    assert_eq!(
        on_key(
            &ctx(Focus::MessageInput, ImeState::NotComposing, true),
            Key::Space,
            NO_MODS
        ),
        Action::None
    );
}
