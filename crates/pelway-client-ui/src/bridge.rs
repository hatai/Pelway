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

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use pelway_client_core::approval::{ApprovalMachine, CardView, Clock, Effect, KeySource};
use pelway_client_core::contract::{
    ApproveError, ApproveOutcome, ChannelId, CreateDecisionRequest, DecideRequest, DecisionError,
    DecisionId, DecisionListItem, DecisionOption, DecisionOutcome, MemberId, MessageId, OptionId,
    TsMs, WorkspaceId,
};
use pelway_client_core::decision::DecisionBoard;
use pelway_client_core::input::{Action, Focus, ImeState, InputContext, Key, Modifiers, on_key};
use slint::{ComponentHandle, Global, Model, ModelRc, SharedString, VecModel};

use crate::state::{
    AvatarKind, ChannelView as ChannelState, DecisionCardState, DecisionOptionView,
    DecisionOutcomeView, ExternalSendState, HandoffOption, MyRequestItem, PendingDecisionItem,
    SAMPLE_DECISION_OPTION_A, SAMPLE_DECISION_OPTION_B, SAMPLE_DECISION_QUESTION,
    SAMPLE_MENTION_NAMES, TimelineEntry, format_wait_ms,
};
use crate::testkit::{FixedClock, FixedKeySource};
use crate::{
    ComposerKind, DecisionOptionItem, MainWindow, PendingItem, RequestItem, TimelineItem,
    TimelineRowKind,
};

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

/// 入力欄 1 つの表示用の状態（本文と候補一覧の開閉）。
///
/// 本文はこちらが正本: 候補の取り込み（`AcceptMention`）で橋が書き換える
/// ことがあるため、画面（Slint の TextInput）には都度書き戻す。
#[derive(Debug, Default)]
struct ComposerState {
    text: String,
    mention_list_open: bool,
}

/// 画面全体のキー操作のつなぎ。
///
/// フォーカス・IME・候補一覧の開閉を持ち、キー操作をすべて `on_key` に渡す。
/// 返ってきた [`Action`] だけで画面の状態を変える:
/// - `AcceptMention`: 先頭の候補を末尾の @… トークンと入れ替えて入れ、一覧を閉じる。
/// - `DismissMentionList`: 候補一覧を閉じる。入力済みの文字は残す。
/// - `InsertNewline`: 入力欄に改行を 1 つ入れる。
/// - `Send`: メッセージの送信として 1 回だけ数える（通信は書かない）。
/// - `Approve`: 承認カードの「送る」押下に結びつく（押せるかはカードが決める）。
pub struct ShellBridge {
    focus: Focus,
    ime: ImeState,
    /// 最後にフォーカスのあった入力欄（`handle_key` の候補一覧の参照先）。
    active_composer: ComposerKind,
    channel_composer: ComposerState,
    thread_composer: ComposerState,
    /// 候補一覧の項目（表示名。Enter は先頭の候補を入れる）。
    mention_candidates: Vec<String>,
    approval: ApprovalCardBridge<FixedKeySource, FixedClock>,
    composer_newlines: u32,
    sent_messages: u32,
}

impl ShellBridge {
    /// 承認カードを 1 枚持って始める。キー生成・時計はテスト用の固定実装。
    /// 候補一覧の項目は合成データ（実メンバー結線の段階で差し替える）。
    #[must_use]
    pub fn new(card: CardView) -> Self {
        Self {
            focus: Focus::MessageInput,
            ime: ImeState::NotComposing,
            active_composer: ComposerKind::Channel,
            channel_composer: ComposerState::default(),
            thread_composer: ComposerState::default(),
            mention_candidates: SAMPLE_MENTION_NAMES
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
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

    /// キー操作の宛先にする入力欄を変える（フォーカスの移り変わりに合わせる）。
    pub fn set_active_composer(&mut self, composer: ComposerKind) {
        self.active_composer = composer;
    }

    /// 現在キー操作の宛先になっている入力欄。
    #[must_use]
    pub fn active_composer(&self) -> ComposerKind {
        self.active_composer
    }

    fn composer(&self, composer: ComposerKind) -> &ComposerState {
        match composer {
            ComposerKind::Channel => &self.channel_composer,
            ComposerKind::Thread => &self.thread_composer,
        }
    }

    fn composer_mut(&mut self, composer: ComposerKind) -> &mut ComposerState {
        match composer {
            ComposerKind::Channel => &mut self.channel_composer,
            ComposerKind::Thread => &mut self.thread_composer,
        }
    }

    /// メンション候補一覧の開閉を変える（宛先は `active_composer` の入力欄）。
    pub fn set_mention_list_open(&mut self, open: bool) {
        self.composer_mut(self.active_composer).mention_list_open = open;
    }

    /// メンション候補一覧が開いているか（`active_composer` の入力欄）。
    #[must_use]
    pub fn mention_list_open(&self) -> bool {
        self.composer(self.active_composer).mention_list_open
    }

    /// 入力欄ごとの候補一覧の開閉。
    #[must_use]
    pub fn mention_list_open_for(&self, composer: ComposerKind) -> bool {
        self.composer(composer).mention_list_open
    }

    /// 入力欄の本文（候補の取り込みで橋が書き換えるため、橋が正本）。
    #[must_use]
    pub fn composer_text(&self, composer: ComposerKind) -> &str {
        &self.composer(composer).text
    }

    /// 入力欄の本文が変わった（文字入力・貼り付け・IME の確定）。
    /// 末尾が「@から始まる未完のトークン」の間だけ候補一覧を開く。
    pub fn note_composer_text(&mut self, composer: ComposerKind, text: &str) {
        let open = has_open_mention_token(text);
        let state = self.composer_mut(composer);
        state.text = text.to_owned();
        state.mention_list_open = open;
    }

    /// 候補一覧の項目（表示名。Enter は先頭の候補を入れる）。
    #[must_use]
    pub fn mention_candidates(&self) -> &[String] {
        &self.mention_candidates
    }

    /// 候補一覧の項目を差し替える（実メンバー結線の段階の窓。今は合成データ）。
    pub fn set_mention_candidates(&mut self, candidates: Vec<String>) {
        self.mention_candidates = candidates;
    }

    /// キー操作を中核の `on_key` に渡し、返ってきた `Action` だけで状態を変える。
    /// （フォーカス・IME は `set_focus` / `set_ime` で受け取る経路。入力欄からの
    /// 直接の経路は [`Self::handle_composer_key`]）。
    pub fn handle_key(&mut self, key: Key, mods: Modifiers) -> Action {
        let ctx = InputContext {
            focus: self.focus,
            ime: self.ime,
            mention_list_open: self.mention_list_open(),
        };
        let action = on_key(&ctx, key, mods);
        self.apply(action);
        action
    }

    /// 入力欄 `composer` が受けたキー操作を `on_key` に渡す。
    ///
    /// フォーカスはその入力欄（`Focus::MessageInput`）、候補一覧の開閉はその
    /// 入力欄の状態で見る。`ime` は入力欄が持つ変換状態（Slint では TextInput の
    /// `preedit-text` があるかどうか）を呼び出し側が渡す。返した `Action` は
    /// ここでも適用する（一覧を閉じる・候補を本文に入れる・送信を数える）。
    pub fn handle_composer_key(
        &mut self,
        composer: ComposerKind,
        key: Key,
        mods: Modifiers,
        ime: ImeState,
    ) -> Action {
        let ctx = InputContext {
            focus: Focus::MessageInput,
            ime,
            mention_list_open: self.composer(composer).mention_list_open,
        };
        let action = on_key(&ctx, key, mods);
        self.apply_for(composer, action);
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
        self.apply_for(self.active_composer, action);
    }

    fn apply_for(&mut self, composer: ComposerKind, action: Action) {
        match action {
            Action::None => {}
            Action::InsertNewline => self.composer_newlines += 1,
            Action::AcceptMention => {
                // 先頭の候補を、末尾の @… トークンと入れ替えて入れる。
                let name = self.mention_candidates.first().cloned().unwrap_or_default();
                let state = self.composer_mut(composer);
                state.mention_list_open = false;
                if let Some(at) = state.text.rfind('@') {
                    state.text.truncate(at);
                }
                state.text.push('@');
                state.text.push_str(&name);
                state.text.push(' ');
            }
            Action::DismissMentionList => {
                self.composer_mut(composer).mention_list_open = false;
            }
            Action::Send => self.sent_messages += 1,
            Action::Approve => {
                self.approval.press_send_button();
            }
            // Decide の適用はボタンが対象を持って渡す `on_decide_key` 側
            // （`wire_key_actions`）。ここにはフォーカス中の対象が無い。
            Action::Decide => {}
        }
    }
}

/// 本文の末尾が「@から始まる未完のトークン」か（候補一覧を開く印）。
///
/// @ は行頭または空白の直後にあるものだけを見る（アドレス等の文中の @ は
/// 開かない）。@ のあとに空白が入ればトークンは終わっている。
fn has_open_mention_token(text: &str) -> bool {
    let Some(at) = text.rfind('@') else {
        return false;
    };
    let head_ok = text[..at]
        .chars()
        .next_back()
        .is_none_or(char::is_whitespace);
    head_ok && !text[at + 1..].contains(char::is_whitespace)
}

/// `Action` がキーイベントを消費するか（Slint の `key-pressed` が accept を
/// 返すかどうか）。`None`・`InsertNewline` は消費しない: 何もしないと
/// TextInput が文字入力・改行として処理する（改行の入れ方は TextInput に任せる）。
#[must_use]
pub fn consumes_key(action: Action) -> bool {
    matches!(
        action,
        Action::Send | Action::AcceptMention | Action::DismissMentionList | Action::Approve
    )
}

/// Slint の `KeyEvent.text`（キーの文字）を中核の `Key` に写す。
/// 対象外のキー（通常の文字・修飾キー・矢印など）は `None`（橋には渡さない）。
/// 入力欄では Escape も橋に渡す（候補一覧を閉じる操作のため）。ボタン系は
/// `key_from_text`（Enter・Space だけ）を使う。
fn composer_key_from_text(text: &str) -> Option<Key> {
    match text {
        "\n" | "\r" => Some(Key::Enter),
        "\u{1b}" => Some(Key::Escape),
        " " => Some(Key::Space),
        _ => None,
    }
}

/// 入力欄 1 つ分の画面の状態を橋に合わせる（本文と候補一覧の開閉）。
fn apply_composer(window: &MainWindow, shell: &ShellBridge, composer: ComposerKind) {
    match composer {
        ComposerKind::Channel => {
            window.set_composer_text(shell.composer_text(composer).into());
            window.set_composer_mention_list_open(shell.mention_list_open_for(composer));
        }
        ComposerKind::Thread => {
            window.set_reply_text(shell.composer_text(composer).into());
            window.set_reply_mention_list_open(shell.mention_list_open_for(composer));
        }
    }
}

/// 入力欄（チャンネルの下欄・スレッドの返信欄）とメンション候補一覧を
/// `shell` に結線する（Issue 383）。
///
/// - 本文の変更（`edited`）は橋に渡し、橋が持つ本文と候補一覧の開閉を画面へ
///   書き戻す（候補の取り込みで橋が本文を書き換えるため）。
/// - Enter・Esc・Space は `key-pressed` の時点で橋に渡し、`on_key` の指示が
///   消費するものなら Slint 側でも消費する（送信・候補確定・一覧を閉じる）。
///   それ以外のキーは TextInput の通常処理に任せる（文字入力・改行）。
/// - IME が変換中かどうかは Slint 側が `preedit-text` から見て callback の
///   引数で渡す。変換中の転送対象キーは消費する: IME が持つキーを `reject` で
///   TextInput に戻すと改行・空白が挿入されてしまうため（`on_key` は変換中に
///   `Action::None` を返すが、それは「この層は何もしない」であって編集部品に
///   渡してよい意味ではない）。
pub fn wire_composers(window: &MainWindow, shell: Rc<RefCell<ShellBridge>>) {
    // 初期状態（橋の本文・一覧の開閉・候補の項目）を画面へ。
    {
        let shell = shell.borrow();
        apply_composer(window, &shell, ComposerKind::Channel);
        apply_composer(window, &shell, ComposerKind::Thread);
        let candidates: Vec<SharedString> = shell
            .mention_candidates()
            .iter()
            .map(|name| name.as_str().into())
            .collect();
        window.set_mention_candidates(ModelRc::new(VecModel::from(candidates)));
    }

    let weak = window.as_weak();
    let edited_shell = Rc::clone(&shell);
    window.on_composer_edited(move |composer, text| {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let mut shell = edited_shell.borrow_mut();
        shell.set_active_composer(composer);
        shell.note_composer_text(composer, text.as_str());
        apply_composer(&window, &shell, composer);
    });

    let focused_shell = Rc::clone(&shell);
    window.on_composer_focused(move |composer| {
        let mut shell = focused_shell.borrow_mut();
        shell.set_active_composer(composer);
        shell.set_focus(Focus::MessageInput);
    });

    let weak = window.as_weak();
    let key_shell = Rc::clone(&shell);
    window.on_composer_key_pressed(move |composer, key_text, ctrl_or_cmd, shift, composing| {
        let Some(window) = weak.upgrade() else {
            return false;
        };
        let Some(key) = composer_key_from_text(key_text.as_str()) else {
            return false;
        };
        let ime = if composing {
            ImeState::Composing
        } else {
            ImeState::NotComposing
        };
        let mut shell = key_shell.borrow_mut();
        shell.set_active_composer(composer);
        let action =
            shell.handle_composer_key(composer, key, Modifiers { ctrl_or_cmd, shift }, ime);
        apply_composer(&window, &shell, composer);
        // 候補の取り込みで本文が書き換わった → カーソルを末尾へ（Slint 側が
        // この値の変化を見て set-selection-offsets を呼ぶ）。
        if action == Action::AcceptMention {
            match composer {
                ComposerKind::Channel => {
                    window.set_composer_cursor_to_end(window.get_composer_cursor_to_end() + 1)
                }
                ComposerKind::Thread => {
                    window.set_reply_cursor_to_end(window.get_reply_cursor_to_end() + 1)
                }
            }
        }
        // 変換中は IME が持つキーなので消費する（TextInput に戻さない）。
        composing || consumes_key(action)
    });
}

// ---- 「今日・判断待ち」画面とチャット内の判断カード（Issue 331） ----

/// メンバーの表示用の値（ID → 表示名とアバター）。
#[derive(Debug, Clone)]
pub struct MemberDisplay {
    /// メンバー ID（`u-*`）。
    pub id: MemberId,
    /// 表示名（架空の名前）。
    pub name: String,
    /// アバター（頭文字と AI かどうか）。
    pub avatar: AvatarKind,
}

/// チャンネルの表示用の値（ID → 表示名）。
#[derive(Debug, Clone)]
pub struct ChannelDisplay {
    /// チャンネル ID（`ch-*`）。
    pub id: ChannelId,
    /// 表示名（例: 「# 案件B」）。
    pub name: String,
}

/// 判断待ち画面とチャット内カードを束ねる橋。
///
/// 正本は [`DecisionBoard`]（メモリ上の台帳）。一覧の項目は台帳から毎回
/// 組み立て、メンバー・チャンネルの表示名を載せる。項目はポインタ型で、
/// 依頼メッセージの本文は一覧に乗せない（inbox.rs と同じ考え方）。
#[derive(Debug)]
pub struct DecisionShell {
    board: DecisionBoard,
    /// 待ち時間の表示の言語（`format_wait_ms` に渡す）。
    lang: String,
    members: Vec<MemberDisplay>,
    channels: Vec<ChannelDisplay>,
    requests: Vec<MyRequestItem>,
    /// 表示中のチャンネルに出す判断カードの ID。
    channel_card: Option<DecisionId>,
}

impl DecisionShell {
    /// 橋を組み立てる。`channel_card` は表示中のチャンネルに出す判断待ち。
    #[must_use]
    pub fn new(
        board: DecisionBoard,
        lang: impl Into<String>,
        members: Vec<MemberDisplay>,
        channels: Vec<ChannelDisplay>,
        requests: Vec<MyRequestItem>,
        channel_card: Option<DecisionId>,
    ) -> Self {
        Self {
            board,
            lang: lang.into(),
            members,
            channels,
            requests,
            channel_card,
        }
    }

    /// デモの合成データ一式（判断待ち1件・自分の依頼1件）。
    ///
    /// 判断待ち: AI Bot のユイが濱田（画面の向こうの人）に、B社への回答の
    /// 出し方の A/B を聞く。`waiting_since` は 0（呼び出し側の `now` が
    /// そのまま待ち時間になる）。
    #[must_use]
    pub fn sample(lang: &str) -> Self {
        let mut board = DecisionBoard::new(WorkspaceId::from("w-demo"), MemberId::from("u-hamada"));
        let card = board
            .seed(
                CreateDecisionRequest {
                    channel_id: ChannelId::from("ch-b"),
                    request_message_id: MessageId::from("m-decision"),
                    requester: MemberId::from("u-yui"),
                    decider: MemberId::from("u-hamada"),
                    question: SAMPLE_DECISION_QUESTION.to_owned(),
                    options: vec![
                        DecisionOption {
                            option_id: OptionId::from("a"),
                            body: SAMPLE_DECISION_OPTION_A.to_owned(),
                        },
                        DecisionOption {
                            option_id: OptionId::from("b"),
                            body: SAMPLE_DECISION_OPTION_B.to_owned(),
                        },
                    ],
                },
                0,
            )
            .expect("合成データの判断待ちは検査に通る");
        let channel_card = card.decision_id.clone();
        Self::new(
            board,
            lang,
            vec![
                MemberDisplay {
                    id: MemberId::from("u-yui"),
                    name: "ユイ".to_owned(),
                    avatar: AvatarKind::Ai("ユ".to_owned()),
                },
                MemberDisplay {
                    id: MemberId::from("u-hamada"),
                    name: "濱田".to_owned(),
                    avatar: AvatarKind::Human("濱".to_owned()),
                },
                MemberDisplay {
                    id: MemberId::from("u-riku"),
                    name: "リク".to_owned(),
                    avatar: AvatarKind::Ai("リ".to_owned()),
                },
            ],
            vec![ChannelDisplay {
                id: ChannelId::from("ch-b"),
                name: "# 案件B".to_owned(),
            }],
            vec![MyRequestItem {
                meta: "# 案件B · リク".to_owned(),
                text: "B社の注文一覧へのCSV出力の実装".to_owned(),
                state_text: "作業中".to_owned(),
            }],
            Some(channel_card),
        )
    }

    /// 「自分の判断待ち」の一覧の行（待ち開始が早い順。台帳から毎回組み立てる）。
    #[must_use]
    pub fn pending_rows(&self, now: TsMs) -> Vec<PendingDecisionItem> {
        self.board
            .pending(now)
            .iter()
            .map(|item| self.pending_row(item))
            .collect()
    }

    fn pending_row(&self, item: &DecisionListItem) -> PendingDecisionItem {
        PendingDecisionItem {
            decision_id: item.decision_id.as_str().to_owned(),
            version: item.version,
            requester: self.member_name(&item.requester),
            avatar: self.member_avatar(&item.requester),
            channel: self.channel_name(&item.channel_id),
            question: item.question.clone(),
            wait_text: format_wait_ms(item.wait_ms, &self.lang),
            options: item
                .options
                .iter()
                .map(|o| DecisionOptionView {
                    option_id: o.option_id.as_str().to_owned(),
                    body: o.body.clone(),
                })
                .collect(),
        }
    }

    /// 「自分の依頼」の一覧の行（進み具合を見るだけ）。
    #[must_use]
    pub fn request_rows(&self) -> Vec<MyRequestItem> {
        self.requests.clone()
    }

    /// 選択肢 `option_id` で確定する。`version` は画面で見ていた版。
    ///
    /// # Errors
    /// [`DecisionError`] のいずれか（決定者でない・版が違う・確定済み・選択肢が無い）。
    /// 同じ選択の再送は確定済みの結果を返す冪等リプレイ（`Ok`）。
    pub fn decide(
        &mut self,
        decision_id: &DecisionId,
        option_id: &str,
        version: u64,
        now: TsMs,
    ) -> Result<DecisionOutcome, DecisionError> {
        self.board
            .decide(decision_id, OptionId::from(option_id), version, now)
    }

    /// チャット内の判断カードの表示用の値（確定済みは `outcome` を持つ）。
    #[must_use]
    pub fn channel_card(&self) -> Option<DecisionCardState> {
        let id = self.channel_card.as_ref()?;
        let card = self.board.card(id)?;
        Some(DecisionCardState {
            decision_id: card.decision_id.as_str().to_owned(),
            version: card.version,
            requester: self.member_name(&card.requester),
            requester_avatar: self.member_avatar(&card.requester),
            question: card.question.clone(),
            options: card
                .options
                .iter()
                .map(|o| DecisionOptionView {
                    option_id: o.option_id.as_str().to_owned(),
                    body: o.body.clone(),
                })
                .collect(),
            outcome: card.outcome.as_ref().map(|o| DecisionOutcomeView {
                selected_body: o.selected.body.clone(),
                decider: self.member_name(&o.decider),
                version: o.version,
            }),
        })
    }

    /// 項目が指すチャンネルの表示名（一覧から元のチャットへ戻る導線）。
    #[must_use]
    pub fn channel_name_of(&self, decision_id: &DecisionId) -> Option<String> {
        self.board
            .channel_of(decision_id)
            .map(|id| self.channel_name(&id))
    }

    /// 一覧の項目から元のチャットを開く。表示中のチャンネルはそのチャンネルに
    /// なり、チャット内の判断カードはこの判断待ちを指す。戻り値は遷移先の
    /// チャンネル名（画面の題名に使う）。知らない判断待ちは `None`（何も開かない）。
    pub fn open_chat(&mut self, decision_id: &DecisionId) -> Option<String> {
        let channel_id = self.board.channel_of(decision_id)?;
        let name = self.channel_name(&channel_id);
        self.channel_card = Some(decision_id.clone());
        Some(name)
    }

    /// 確定要求を組み立てた回数（テストと監視用）。
    #[must_use]
    pub fn decide_request_count(&self) -> u32 {
        self.board.decide_request_count()
    }

    /// 最後に組み立てた確定要求（HTTP 結線の段階で送るもの）。
    #[must_use]
    pub fn last_decide_request(&self) -> Option<&DecideRequest> {
        self.board.last_decide_request()
    }

    /// 正本の盤への参照。
    #[must_use]
    pub fn board(&self) -> &DecisionBoard {
        &self.board
    }

    fn member_name(&self, id: &MemberId) -> String {
        self.members
            .iter()
            .find(|m| &m.id == id)
            .map_or_else(|| id.as_str().to_owned(), |m| m.name.clone())
    }

    fn member_avatar(&self, id: &MemberId) -> AvatarKind {
        self.members
            .iter()
            .find(|m| &m.id == id)
            .map_or_else(|| AvatarKind::Human("?".to_owned()), |m| m.avatar.clone())
    }

    fn channel_name(&self, id: &ChannelId) -> String {
        self.channels
            .iter()
            .find(|c| &c.id == id)
            .map_or_else(|| id.as_str().to_owned(), |c| c.name.clone())
    }
}

fn option_items(options: &[DecisionOptionView]) -> Vec<DecisionOptionItem> {
    options
        .iter()
        .map(|o| DecisionOptionItem {
            option_id: o.option_id.as_str().into(),
            body: o.body.as_str().into(),
        })
        .collect()
}

/// 判断待ち画面（「今日」の一覧・自分の依頼）とチャット内の判断カードを
/// `shell` の状態に合わせる。一覧は台帳から毎回組み立てるため、確定済みは
/// 自然に一覧から消える。
pub fn apply_decision_screen(window: &MainWindow, shell: &DecisionShell, now: TsMs) {
    let rows: Vec<PendingItem> = shell
        .pending_rows(now)
        .into_iter()
        .map(|r| PendingItem {
            decision_id: r.decision_id.as_str().into(),
            version: r.version as i32,
            requester: r.requester.as_str().into(),
            initial: r.avatar.initial().into(),
            is_ai: r.avatar.is_ai(),
            channel: r.channel.as_str().into(),
            question: r.question.as_str().into(),
            wait: r.wait_text.as_str().into(),
            options: option_items(&r.options).as_slice().into(),
        })
        .collect();
    window.set_pending_items(ModelRc::new(VecModel::from(rows)));

    let requests: Vec<RequestItem> = shell
        .request_rows()
        .into_iter()
        .map(|r| RequestItem {
            meta: r.meta.as_str().into(),
            text: r.text.as_str().into(),
            progress: r.state_text.as_str().into(),
        })
        .collect();
    window.set_request_items(ModelRc::new(VecModel::from(requests)));

    apply_decision_card_props(window, shell.channel_card().as_ref());
}

/// チャット内の判断カードのプロパティ（`data-decision-*`）を `card` に合わせる。
/// `None` ならカードを出さない（`data-decision-visible` を false にするだけで
/// ほかの欄は残す。次にカードが出たとき上書きされる）。
fn apply_decision_card_props(window: &MainWindow, card: Option<&DecisionCardState>) {
    match card {
        Some(card) => {
            // データを先に全部入れてから最後に visible を立てる。`if` 条件付き
            // の要素（依頼者の AI バッジなど）が初期値で確定評価されることを防ぐ
            window.set_data_decision_id(card.decision_id.as_str().into());
            window.set_data_decision_version(card.version as i32);
            window.set_data_decision_requester(card.requester.as_str().into());
            window.set_data_decision_requester_is_ai(card.requester_avatar.is_ai());
            window.set_data_decision_requester_initial(card.requester_avatar.initial().into());
            window.set_data_decision_question(card.question.as_str().into());
            window.set_data_decision_options(ModelRc::new(VecModel::from(option_items(
                &card.options,
            ))));
            window.set_data_decision_decided(card.outcome.is_some());
            match &card.outcome {
                Some(outcome) => {
                    window.set_data_decision_selected(outcome.selected_body.as_str().into());
                    window.set_data_decision_decider(outcome.decider.as_str().into());
                }
                None => {
                    window.set_data_decision_selected("".into());
                    window.set_data_decision_decider("".into());
                }
            }
            window.set_data_decision_visible(true);
        }
        None => window.set_data_decision_visible(false),
    }
}

// ---- チャンネル画面の時系列（Issue 385。ListView への適用） ----

/// `state` の時系列の項目 1 つを、画面の行一覧（`TimelineItem`）に展開する。
///
/// スレッドの要約が付くメッセージは、メッセージ行とスレッド導線の 2 行になる
/// （仮想化前の並びと同じ）。`MessageEntry` に時刻の欄は無いため `time` は
/// 空のまま。`TimelineEntry::DecisionCard` は位置だけの印で、中身は
/// `data-decision-*` プロパティ（台帳が正本）を行が読む。
fn timeline_row_items(entry: &TimelineEntry) -> Vec<TimelineItem> {
    let base = |kind| TimelineItem {
        kind,
        ..TimelineItem::default()
    };
    match entry {
        TimelineEntry::DayDivider { label } => vec![TimelineItem {
            day_label: label.as_str().into(),
            ..base(TimelineRowKind::DayDivider)
        }],
        TimelineEntry::UnreadMarker => vec![base(TimelineRowKind::UnreadMarker)],
        TimelineEntry::Message(message) => {
            let mut rows = vec![TimelineItem {
                sender: message.sender.as_str().into(),
                is_ai: message.avatar.is_ai(),
                initial: message.avatar.initial().into(),
                body: message.body.as_str().into(),
                // 「{from}〜{to}を読みました」の表示は、仮想化前はスレッドの
                // 要約が付くメッセージに出ていた（同じ条件で出す）。
                show_read_range: message.thread.is_some(),
                ..base(TimelineRowKind::Message)
            }];
            if let Some(thread) = &message.thread {
                rows.push(TimelineItem {
                    thread_working: thread.working,
                    thread_done: thread.done_at.is_some(),
                    ..base(TimelineRowKind::ThreadLead)
                });
            }
            rows
        }
        // 決定の印の文言（誰・時刻を含む）は tr-decision-mark が持つ（行には
        // 乗せない。仮想化前と同じ表示）。
        TimelineEntry::Decision { .. } => vec![base(TimelineRowKind::DecisionMark)],
        TimelineEntry::DecisionCard => vec![base(TimelineRowKind::DecisionCard)],
        TimelineEntry::Handoff(handoff) => vec![TimelineItem {
            handoff_from_to: format!("{} → {}", handoff.from, handoff.to).into(),
            handoff_title: handoff.title.as_str().into(),
            handoff_acknowledged: handoff.acknowledged.is_some(),
            handoff_option_a: handoff_option_text(handoff.options.first()),
            handoff_option_b: handoff_option_text(handoff.options.get(1)),
            ..base(TimelineRowKind::Handoff)
        }],
        TimelineEntry::PrivateNote {
            sender,
            avatar,
            body,
        } => vec![TimelineItem {
            sender: sender.as_str().into(),
            is_ai: avatar.is_ai(),
            initial: avatar.initial().into(),
            body: body.as_str().into(),
            ..base(TimelineRowKind::PrivateNote)
        }],
        TimelineEntry::ExternalSend(card) => vec![TimelineItem {
            ext_address: card.address.as_str().into(),
            ext_subject: card.subject.as_str().into(),
            ext_basis: card.basis.as_str().into(),
            ext_body: card.body.as_str().into(),
            ext_sent: matches!(card.state, ExternalSendState::Sent { .. }),
            ..base(TimelineRowKind::ExternalSend)
        }],
    }
}

/// 引き継ぎカードの選択肢 1 行の表示（「A　範囲を絞って…」。モックに合わせて
/// ラベルと本文の間は全角空白）。選択肢がなければ空文字。
fn handoff_option_text(option: Option<&HandoffOption>) -> slint::SharedString {
    option.map_or_else(slint::SharedString::default, |o| {
        format!("{}　{}", o.label, o.text).into()
    })
}

/// `state` のチャンネル画面の表示データを窓に渡す。
///
/// 題名・参加者に加え、時系列は ListView の行一覧（`timeline-items`）として
/// 渡し、判断カードは `decision_card` の内容を `data-decision-*` に載せる。
/// 台帳駆動の流れ（`apply_decision_screen`）が後から同じプロパティを
/// 上書きするため、確定などの操作後の値は台帳側が正本になる。
pub fn apply_channel_view(window: &MainWindow, channel: &ChannelState) {
    window.set_data_channel_title(channel.title.as_str().into());
    window.set_data_channel_members(channel.members.as_str().into());
    let items: Vec<TimelineItem> = channel
        .timeline
        .iter()
        .flat_map(timeline_row_items)
        .collect();
    window.set_timeline_items(ModelRc::new(VecModel::from(items)));
    apply_decision_card_props(window, channel.decision_card.as_ref());
}

/// 画面の操作を `shell` に結線する（一覧とチャット内カードは同じ callback を
/// 通る）。確定に成功したら画面を再適用して、項目を一覧から消し、カードを
/// 確定表示に変える。`now` は骨組みの固定時刻（結線の段階で時計に置き換える）。
pub fn wire_decision_screen(window: &MainWindow, shell: Rc<RefCell<DecisionShell>>, now: TsMs) {
    apply_decision_screen(window, &shell.borrow(), now);

    let weak = window.as_weak();
    let decide_shell = Rc::clone(&shell);
    window.on_decide_option(move |decision_id, option_id, version| {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let mut shell = decide_shell.borrow_mut();
        let id = DecisionId::from(decision_id.as_str());
        // 失敗（版違い・確定済み・権限なし）は台帳を変えないため再適用もしない。
        if shell.decide(&id, &option_id, version as u64, now).is_ok() {
            apply_decision_screen(&window, &shell, now);
        }
    });

    let weak = window.as_weak();
    let open_shell = Rc::clone(&shell);
    window.on_open_decision_chat(move |decision_id| {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let id = DecisionId::from(decision_id.as_str());
        // 知らない判断待ちでは画面を切り替えない（壊れた項目の導線を塞ぐ）。
        let mut shell = open_shell.borrow_mut();
        if let Some(title) = shell.open_chat(&id) {
            window.set_data_channel_title(title.as_str().into());
            window.set_show_today(false);
            apply_decision_screen(&window, &shell, now);
            ensure_decision_card_row(&window);
        }
    });
}

/// チャット内の判断カードを出す `decision-card` 行が時系列になければ、末尾に
/// 置く（Issue 385。`apply_channel_view` が時系列を渡す流れでは、判断カードの
/// 行はすでに載っている想定。この骨組みの流れでは時系列が空のままなので、
/// `data-decision-*` だけではカードが実体化しない）。
fn ensure_decision_card_row(window: &MainWindow) {
    if !window.get_data_decision_visible() {
        return;
    }
    let model = window.get_timeline_items();
    let has_card = (0..model.row_count()).any(|i| {
        model
            .row_data(i)
            .is_some_and(|item| item.kind == TimelineRowKind::DecisionCard)
    });
    if has_card {
        return;
    }
    let mut items: Vec<TimelineItem> = (0..model.row_count())
        .filter_map(|i| model.row_data(i))
        .collect();
    items.push(TimelineItem {
        kind: TimelineRowKind::DecisionCard,
        ..TimelineItem::default()
    });
    window.set_timeline_items(ModelRc::new(VecModel::from(items)));
}

// ---- 決定・承認ボタンのキー操作（Issue 379） ----

/// `.slint` の `KeyEvent.text` から中核の [`Key`] へ。対象外のキーは `None`
/// （イベントを消費せず伝播させる側に回る）。
fn key_from_text(text: &str) -> Option<Key> {
    match text {
        "\n" | "\r" => Some(Key::Enter),
        " " => Some(Key::Space),
        _ => None,
    }
}

/// 決定・承認ボタンのキー操作を中核の `on_key` に結線する（Issue 379）。
///
/// `.slint` の FocusScope は生のキー（`KeyEvent.text`・Ctrl/Cmd・Shift）を
/// `KeyRouter` global の `decide-key`・`send-key` callback に渡す。ここで
/// `on_key` が「決定・承認になるか」を決め、返ってきた [`Action`] だけで動く:
/// - `Decide`: 台帳を確定し、成功したら画面を再適用する。
/// - `Approve`: 「送る」ボタンの押下経路（`send-pressed` callback）に乗せる。
///
/// Enter・Space は `on_key` の結果が `None`（修飾付き・IME 変換中）でも消費
/// する（ボタンのキーが画面の他の部品へ漏れない）。それ以外のキーは
/// `false` を返し、通常のキー処理（Tab 移動など）へ伝播させる。
///
/// `ime` は画面の IME 状態を持つセル（骨組みでは既定 `NotComposing`。結線の
/// 段階で IME の実装が更新する）。
pub fn wire_key_actions(
    window: &MainWindow,
    shell: Rc<RefCell<DecisionShell>>,
    ime: Rc<Cell<ImeState>>,
    now: TsMs,
) {
    let router = crate::KeyRouter::get(window);
    let weak = window.as_weak();
    let decide_ime = Rc::clone(&ime);
    router.on_decide_key(
        move |decision_id, option_id, version, key_text, ctrl_or_cmd, shift| {
            let Some(window) = weak.upgrade() else {
                return false;
            };
            let Some(key) = key_from_text(key_text.as_str()) else {
                return false;
            };
            let ctx = InputContext {
                focus: Focus::DecisionButton,
                ime: decide_ime.get(),
                mention_list_open: false,
            };
            let mods = Modifiers { ctrl_or_cmd, shift };
            if on_key(&ctx, key, mods) == Action::Decide {
                let mut shell = shell.borrow_mut();
                let id = DecisionId::from(decision_id.as_str());
                // 失敗（版違い・確定済み・権限なし）は台帳を変えないため再適用もしない。
                if shell.decide(&id, &option_id, version as u64, now).is_ok() {
                    apply_decision_screen(&window, &shell, now);
                }
            }
            true
        },
    );

    let weak = window.as_weak();
    router.on_send_key(move |key_text, ctrl_or_cmd, shift| {
        let Some(window) = weak.upgrade() else {
            return false;
        };
        let Some(key) = key_from_text(key_text.as_str()) else {
            return false;
        };
        let ctx = InputContext {
            focus: Focus::ApproveButton,
            ime: ime.get(),
            mention_list_open: false,
        };
        let mods = Modifiers { ctrl_or_cmd, shift };
        if on_key(&ctx, key, mods) == Action::Approve {
            // 承認は「送る」の押下経路に乗せる（押せるかどうかは経路の先が決める）。
            window.invoke_send_pressed();
        }
        true
    });
}
