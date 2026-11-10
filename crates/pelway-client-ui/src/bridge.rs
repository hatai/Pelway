//! 中核ロジック（`pelway-client-core`）へのつなぎ。Slint に依存しない。
//!
//! - キー操作は [`pelway_client_core::input::on_key`] に渡し、返ってきた
//!   [`Action`] だけで動く。UI の側で同じ判定を書き直さない。
//! - 「送る」ボタンは [`ApprovalCardBridge`] が [`ApprovalMachine::press_approve`] に
//!   つなぎ、押せるかどうかは `approve_button_enabled()` で決める。
//! - 通信は書かない。`Effect` は [`ApprovalCardBridge`] が受け取って保持するだけ
//!   （未接続）。ログにも出さない。
//! - コメント欄の処理は中核にあるが、モックにコメント欄の見た目がないため
//!   画面には置かない。`comment_editable()` は表示用の状態としてだけ持つ。

use pelway_client_core::approval::{ApprovalMachine, CardView, Clock, Effect, KeySource};
use pelway_client_core::contract::{ApproveError, ApproveOutcome};
use pelway_client_core::input::{Action, Focus, ImeState, InputContext, Key, Modifiers, on_key};

use crate::testkit::{FixedClock, FixedKeySource};

/// 承認カード 1 枚の表示用の状態と操作のつなぎ。
///
/// `Effect` は未接続: この骨組みでは応答をどこにも送らず、保持するだけ。
pub struct ApprovalCardBridge<K: KeySource, C: Clock> {
    machine: ApprovalMachine<K, C>,
    last_effect: Option<Effect>,
    approve_request_count: u32,
}

impl<K: KeySource, C: Clock> ApprovalCardBridge<K, C> {
    /// 表示するカードと、キー生成・時計の実装を受け取る。
    #[must_use]
    pub fn new(card: CardView, keys: K, clock: C) -> Self {
        Self {
            machine: ApprovalMachine::new(card, keys, clock),
            last_effect: None,
            approve_request_count: 0,
        }
    }

    /// 「送る」ボタンが押せるか。中核の `approve_button_enabled()` そのまま。
    #[must_use]
    pub fn send_button_enabled(&self) -> bool {
        self.machine.approve_button_enabled()
    }

    /// コメント欄が書けるか（表示用の状態としてだけ持つ。モックにコメント欄の
    /// 見た目がないため画面には置かない。未決として作業ログに記録）。
    #[must_use]
    pub fn comment_editable(&self) -> bool {
        self.machine.comment_editable()
    }

    /// 送信済みの表示（「濱田 15:20 送信 · 顧客回答待ち」など）に移っているか。
    /// 承認が済んでカードが送信側の状態へ移ったら true。
    #[must_use]
    pub fn sent(&self) -> bool {
        self.machine.card().state.is_send_side()
    }

    /// 「送る」を押した（承認ボタンの Enter/Space もここに結びつく）。
    /// 押せないカードでは要求が出ない（`None` を返し、冪等キーも消費しない）。
    pub fn press_send_button(&mut self) -> Option<Effect> {
        if !self.send_button_enabled() {
            return None;
        }
        let effect = self.machine.press_approve();
        if matches!(effect, Some(Effect::Approve(_))) {
            self.approve_request_count += 1;
        }
        self.last_effect = effect.clone();
        effect
    }

    /// 承認 API の応答を中核へ戻す窓（未接続: 骨組みでは呼ばなくてもよい）。
    pub fn on_approve_response(
        &mut self,
        result: Result<ApproveOutcome, ApproveError>,
    ) -> Option<Effect> {
        let effect = self.machine.on_approve_response(result);
        if matches!(effect, Some(Effect::Approve(_))) {
            self.approve_request_count += 1;
        }
        self.last_effect = effect.clone();
        effect
    }

    /// 中核が最後に返した `Effect`。未接続のため保持するだけ（通信・ログ出力なし）。
    #[must_use]
    pub fn last_effect(&self) -> Option<&Effect> {
        self.last_effect.as_ref()
    }

    /// 承認の要求（`Effect::Approve`）が出た回数。
    #[must_use]
    pub fn approve_request_count(&self) -> u32 {
        self.approve_request_count
    }
}

/// 画面全体のキー操作のつなぎ。
///
/// フォーカス・IME・候補一覧の開閉を持ち、キー操作をすべて `on_key` に渡す。
/// 返ってきた [`Action`] だけで画面の状態を変える:
/// - `AcceptMention` / `DismissMentionList`: 候補一覧を閉じる。
/// - `InsertNewline`: 入力欄に改行を 1 つ入れる。
/// - `Send`: メッセージの送信として 1 回だけ数える（通信は書かない）。
/// - `Approve`: 承認カードの「送る」押下に結びつく（押せるかはカードが決める）。
pub struct ShellBridge {
    focus: Focus,
    ime: ImeState,
    mention_list_open: bool,
    approval: ApprovalCardBridge<FixedKeySource, FixedClock>,
    composer_newlines: u32,
    sent_messages: u32,
}

impl ShellBridge {
    /// 承認カードを 1 枚持って始める。キー生成・時計はテスト用の固定実装。
    #[must_use]
    pub fn new(card: CardView) -> Self {
        Self {
            focus: Focus::MessageInput,
            ime: ImeState::NotComposing,
            mention_list_open: false,
            approval: ApprovalCardBridge::new(card, FixedKeySource::new(), FixedClock::new(0)),
            composer_newlines: 0,
            sent_messages: 0,
        }
    }

    /// フォーカスのある部品を変える。
    pub fn set_focus(&mut self, focus: Focus) {
        self.focus = focus;
    }

    /// 現在フォーカスのある部品。
    #[must_use]
    pub fn focus(&self) -> Focus {
        self.focus
    }

    /// IME の状態を変える。
    pub fn set_ime(&mut self, ime: ImeState) {
        self.ime = ime;
    }

    /// メンション候補一覧の開閉を変える。
    pub fn set_mention_list_open(&mut self, open: bool) {
        self.mention_list_open = open;
    }

    /// メンション候補一覧が開いているか。
    #[must_use]
    pub fn mention_list_open(&self) -> bool {
        self.mention_list_open
    }

    /// キー操作を中核の `on_key` に渡し、返ってきた `Action` だけで状態を変える。
    pub fn handle_key(&mut self, key: Key, mods: Modifiers) -> Action {
        let ctx = InputContext {
            focus: self.focus,
            ime: self.ime,
            mention_list_open: self.mention_list_open,
        };
        let action = on_key(&ctx, key, mods);
        self.apply(action);
        action
    }

    /// 承認カードの橋（表示用の状態の読み取りと「送る」の押下）。
    pub fn approval(&mut self) -> &mut ApprovalCardBridge<FixedKeySource, FixedClock> {
        &mut self.approval
    }

    /// 承認カードの橋（読み取り専用）。
    #[must_use]
    pub fn approval_ref(&self) -> &ApprovalCardBridge<FixedKeySource, FixedClock> {
        &self.approval
    }

    /// 入力欄に挿入された改行の数（`InsertNewline` の適用結果）。
    #[must_use]
    pub fn composer_newlines(&self) -> u32 {
        self.composer_newlines
    }

    /// 送信（`Send`）が適用された回数。
    #[must_use]
    pub fn sent_messages(&self) -> u32 {
        self.sent_messages
    }

    fn apply(&mut self, action: Action) {
        match action {
            Action::None => {}
            Action::InsertNewline => self.composer_newlines += 1,
            Action::AcceptMention | Action::DismissMentionList => {
                self.mention_list_open = false;
            }
            Action::Send => self.sent_messages += 1,
            Action::Approve => {
                self.approval.press_send_button();
            }
        }
    }
}
