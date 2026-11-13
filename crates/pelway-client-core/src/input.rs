//! キー操作の判定。QA-APR-01（コメント欄）と QA-MEN-01（メンション候補）の仕様表。
//!
//! この層は純粋関数: 「どの部品にフォーカスがあるか・IME の状態・
//! 候補一覧の開閉・押されたキーと修飾キー」から「クライアントが次に何をするか」を
//! 決める。テキストの入力・変換そのものは IME とテキスト編集部品が担い、
//! この層は承認・送信・改行・候補確定だけを指示する。
//!
//! QA-APR-01: コメント欄では、IME がどの状態でも Enter・Ctrl/Cmd+Enter・Space で
//! 承認も送信もしない。確定後・無変換の Enter は改行 1 つ。変換中の Enter は
//! IME の確定だけ（この層は何もしない）。承認は承認ボタンでの Enter/Space（修飾なし）だけ。
//!
//! QA-MEN-01: 候補一覧が開いている間の Enter は候補を 1 つ入れて一覧を閉じ、
//! 送信しない（IME 変換中でも同じ）。Esc は入力文字を残して一覧だけを閉じる
//! （これも IME 変換中でも同じ）。一覧が閉じたあとの Enter は送信 1 回。

/// キー操作の宛先になる部品。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    /// 承認カードのコメント欄。
    CommentBox,
    /// 承認ボタン。
    ApproveButton,
    /// メッセージ入力欄。
    MessageInput,
}

/// 判定対象のキー。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Enter,
    Space,
    Escape,
}

/// IME の状態。変換中のキーは IME が消費するため、この層は何もしない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImeState {
    /// 変換中（未確定文字列がある）。
    Composing,
    /// 確定済み・無変換。
    NotComposing,
}

/// 修飾キー。Ctrl と Cmd（macOS）は同じ扱いにする。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    /// Ctrl または Cmd。
    pub ctrl_or_cmd: bool,
    pub shift: bool,
}

/// キー操作を見るときの状況。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputContext {
    /// フォーカスがある部品。
    pub focus: Focus,
    /// IME の状態。
    pub ime: ImeState,
    /// メンション候補一覧が開いているか。
    pub mention_list_open: bool,
}

/// クライアントが次にすること。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// 何もしない（IME の変換・確定、通常の文字入力など、この層の管轄外）。
    None,
    /// 改行を 1 つ入れる。
    InsertNewline,
    /// 承認を 1 回。実際の冪等キー管理は [`crate::approval`] が担う。
    Approve,
    /// メンション候補を 1 つ入れて一覧を閉じる。
    AcceptMention,
    /// メンション候補一覧を閉じるだけ。入力済みの文字は残す。
    DismissMentionList,
    /// メッセージを送信する 1 回。
    Send,
}

/// キー操作 1 つから、次にすることを決める。
///
/// 候補一覧が開いている間（メッセージ入力欄）の Enter と Esc は、IME が変換中でも
/// 有効: Enter は候補を 1 つ入れて一覧を閉じ、Esc は入力文字を残して一覧だけを閉じる
/// （どちらも送信しない。QA-MEN-01 は IME 状態を理由に例外を設けない）。
/// それ以外の部品では、変換中（[`ImeState::Composing`]）のキーは IME の操作
/// （確定・変換・キャンセル）を優先し、この層は常に [`Action::None`] を返す。
/// 確定後の Enter・Ctrl/Cmd+Enter・Shift+Enter のいずれも、コメント欄では
/// 改行 1 つになる（承認も送信もしない）。
#[must_use]
pub fn on_key(ctx: &InputContext, key: Key, mods: Modifiers) -> Action {
    // QA-MEN-01: 一覧が開いている間の Enter・Esc は IME 状態より優先する。
    if ctx.mention_list_open && ctx.focus == Focus::MessageInput {
        return match key {
            Key::Enter => Action::AcceptMention,
            Key::Escape => Action::DismissMentionList,
            Key::Space => Action::None,
        };
    }
    if ctx.ime == ImeState::Composing {
        return Action::None;
    }
    let plain = mods == Modifiers::default();
    match ctx.focus {
        Focus::CommentBox => match key {
            Key::Enter => Action::InsertNewline,
            Key::Space | Key::Escape => Action::None,
        },
        Focus::ApproveButton => match (key, plain) {
            (Key::Enter | Key::Space, true) => Action::Approve,
            _ => Action::None,
        },
        // ここへ来るとき候補一覧は閉じている（開いていれば上で返している）。
        Focus::MessageInput => match (key, plain) {
            (Key::Enter, true) => Action::Send,
            _ => Action::None,
        },
    }
}
