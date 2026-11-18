//! 画面の文言の Fluent キーと、Fluent からの文言の取得。
//!
//! 画面の文言は `i18n/ja/main.ftl` と `i18n/en/main.ftl` に置く。
//! Rust が Fluent（`fluent-bundle`）として引いた文字列を Slint の `tr-*`
//! プロパティへ渡す（[`apply_translations`]）。`.ftl` は `include_str!` で
//! 実行形式に取り込むため、実行時にファイルを読まない。
//! Slint の `@tr()`（gettext）は仕様が Fluent のため使わない。
//!
//! 起動時は [`initial_lang`] で OS の言語から初期言語を決め、[`apply_lang`] で
//! 適用する。起動後の切替も [`apply_lang`] をもう一度呼ぶだけ（再起動なし。
//! 設定画面は別 Issue）。
//!
//! 英語の文言は意味の分かる仮の訳。
//!
//! Slint の attribution（Royalty-free License 2.0 §2 (a)）は About ダイアログの
//! AboutSlint widget で表示する（Issue 237）。fluent 関連 crate のライセンス表示を
//! どこにどう出すかは未決のまま。

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use unic_langid::LanguageIdentifier;

use crate::MainWindow;

/// `.ftl` の読み込み元（ja）。
const JA_FTL: &str = include_str!("../i18n/ja/main.ftl");
/// `.ftl` の読み込み元（en）。
const EN_FTL: &str = include_str!("../i18n/en/main.ftl");

/// Rust が使う文言キーの一覧。`ui/main.slint` の `tr-*` プロパティと同じ集合。
pub const USED_KEYS: &[&str] = &[
    // 左の列（rail）
    "rail-viewing-as",
    "rail-today",
    "rail-channels",
    "rail-dm-people",
    "rail-dm-ai",
    // AI Botの在席状態（rail の表示）
    "ai-presence-standby",
    "ai-presence-working",
    "ai-presence-waiting",
    // 上の帯
    "search-placeholder",
    "search-shortcut",
    // 「今日・判断待ち」の画面
    "today-title",
    "today-section-mine",
    "today-section-requests",
    "today-section-empty",
    "today-quiet",
    "pending-wait",
    "pending-open-chat",
    "decision-card-label",
    "decision-choose",
    "decision-outcome",
    // チャンネルの画面
    "unread-divider",
    "ai-badge",
    "decision-mark",
    "message-private-note",
    "thread-replies",
    "thread-working",
    "thread-done",
    "thread-view-work",
    "mention-read-range",
    // 引き継ぎカード
    "handoff-context",
    "handoff-acknowledged",
    "handoff-not-acknowledged",
    "handoff-acknowledge",
    "handoff-decision-owner",
    // 社外への送信の確認カード
    "ext-card-label",
    "ext-card-question",
    "ext-address-label",
    "ext-subject-label",
    "ext-basis-label",
    "ext-send",
    "ext-fix",
    "ext-lock-note",
    "ext-sent",
    // 右の枠（スレッド・AIの作業）
    "pane-thread-title",
    "pane-close",
    "pane-back-to-thread",
    "pane-ai-work-title",
    "pane-ai-steps",
    "pane-ai-from",
    "pane-ai-read-range",
    "pane-ai-note",
    "thread-input-placeholder",
    // 下のメッセージ入力欄
    "composer-placeholder",
    // トップレベルメニューと About ダイアログ（Slint の attribution、Issue 237）
    "menu-help",
    "menu-about",
    "about-title",
    "about-close",
];

/// 用意している言語（`.ftl` がある言語だけ）。
pub const LANGS: &[&str] = &["ja", "en"];

/// OS の言語設定（BCP-47 相当のタグ）から初期言語を決める。
///
/// 日本語（`ja`・`ja-JP` など）は `ja`、それ以外（未知・空を含む）は `en`。
/// 返り値は常に [`LANGS`] のどれかなので、翻訳の無い言語で起動して翻訳キーの
/// 生文字列を画面へ出す経路は無い。OS からの実際の取得は呼び出し側が行い、
/// この関数は文字列を受け取るだけにする（OS 依存をこの crate に入れない）。
#[must_use]
pub fn initial_lang(os_lang: &str) -> &'static str {
    // 言語タグの先頭（"ja-JP" → "ja"）だけを見る。区切りは - と _ の両方を受け付ける。
    let primary = os_lang
        .split(['-', '_'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if primary == "ja" { "ja" } else { "en" }
}

/// 文言の変数（Fluent の `{ $name }` など）に代入する値。
///
/// 画面のデータ（誰の・いつ・何件）はまだ結合していないため、骨組みでは
/// 合成データ（[`crate::state::sample`] と同じ架空の値）を代入して表示だけを
/// 通す。実際の値は結合の段階で各部品から渡す。
#[derive(Debug, Clone)]
pub struct FluentVars {
    /// 人の名前（{ $name }）。
    pub name: String,
    /// 時刻（{ $time }）。
    pub time: String,
    /// 件数（{ $n }）。
    pub n: String,
    /// 範囲の始まり（{ $from }）。
    pub from: String,
    /// 読んだ範囲の終わり・時刻（{ $to }）。
    pub to: String,
    /// 社外送信の宛先（{ $address }）。
    pub address: String,
    /// 済ませた手順数（{ $done }）。
    pub done: String,
    /// 手順の総数（{ $total }）。
    pub total: String,
    /// チャンネル名（{ $channel }）。
    pub channel: String,
}

impl FluentVars {
    /// 合成データと同じ架空の値（実在の情報は入れない）。
    #[must_use]
    pub fn sample() -> Self {
        Self {
            name: "濱田".to_owned(),
            time: "15:12".to_owned(),
            n: "5".to_owned(),
            from: "昨日18:40".to_owned(),
            to: "今日9:30".to_owned(),
            address: "csv-support@b-sha.example".to_owned(),
            done: "2".to_owned(),
            total: "4".to_owned(),
            channel: "# 案件B".to_owned(),
        }
    }
}

/// 1つの言語の Fluent の文言（`.ftl` を `include_str!` から組み立てる）。
pub struct Messages {
    bundle: FluentBundle<FluentResource>,
}

impl Messages {
    /// 言語（`ja` / `en`）の [`Messages`] を作る。用意していない言語は `None`。
    ///
    /// `.ftl` は `include_str!` で実行形式に取り込んだものを使う（実行時に
    /// ファイルを読まない）。`.ftl` の解釈に失敗した場合も `None`。
    #[must_use]
    pub fn new(lang: &str) -> Option<Self> {
        let source = match lang {
            "ja" => JA_FTL,
            "en" => EN_FTL,
            _ => return None,
        };
        let langid: LanguageIdentifier = lang.parse().ok()?;
        let resource = FluentResource::try_new(source.to_owned()).ok()?;
        let mut bundle = FluentBundle::new(vec![langid]);
        bundle.add_resource(resource).ok()?;
        Some(Self { bundle })
    }

    /// キーの文言を引いて、変数に `vars` の値を代入した文字列を返す。
    ///
    /// キーが無い・Fluent の整形に誤りが出たら `None`（文言の中身は呼び出し側で
    /// 扱うため、この関数は空文字列を返さない）。
    pub fn message(&self, key: &str, vars: &FluentVars) -> Option<String> {
        let msg = self.bundle.get_message(key)?;
        let pattern = msg.value()?;
        let mut args = FluentArgs::new();
        // すべての変数を渡す（そのキーが使わない変数は Fluent が無視する）。
        args.set("name", vars.name.as_str());
        args.set("time", vars.time.as_str());
        args.set("n", vars.n.as_str());
        args.set("from", vars.from.as_str());
        args.set("to", vars.to.as_str());
        args.set("address", vars.address.as_str());
        args.set("done", vars.done.as_str());
        args.set("total", vars.total.as_str());
        args.set("channel", vars.channel.as_str());
        let mut errors = Vec::new();
        let value = self
            .bundle
            .format_pattern(pattern, Some(&args), &mut errors);
        if errors.is_empty() {
            Some(value.into_owned())
        } else {
            None
        }
    }
}

/// `MainWindow` の `tr-*` プロパティに、`messages` の全キーの文言を渡す。
///
/// 渡せなかったキー（`message` が `None` を返したキー）の一覧を返す
/// （空ならすべて渡せた）。窓は開かない。
pub fn apply_translations(
    window: &MainWindow,
    messages: &Messages,
    vars: &FluentVars,
) -> Vec<String> {
    let mut missing = Vec::new();
    let mut text = |key: &str| -> slint::SharedString {
        match messages.message(key, vars) {
            Some(s) => s.into(),
            None => {
                missing.push(key.to_owned());
                slint::SharedString::default()
            }
        }
    };
    window.set_tr_rail_viewing_as(text("rail-viewing-as"));
    window.set_tr_rail_today(text("rail-today"));
    window.set_tr_rail_channels(text("rail-channels"));
    window.set_tr_rail_dm_people(text("rail-dm-people"));
    window.set_tr_rail_dm_ai(text("rail-dm-ai"));
    window.set_tr_ai_presence_standby(text("ai-presence-standby"));
    window.set_tr_ai_presence_working(text("ai-presence-working"));
    window.set_tr_ai_presence_waiting(text("ai-presence-waiting"));
    window.set_tr_search_placeholder(text("search-placeholder"));
    window.set_tr_search_shortcut(text("search-shortcut"));
    window.set_tr_today_title(text("today-title"));
    window.set_tr_today_section_mine(text("today-section-mine"));
    window.set_tr_today_section_requests(text("today-section-requests"));
    window.set_tr_today_section_empty(text("today-section-empty"));
    window.set_tr_today_quiet(text("today-quiet"));
    window.set_tr_pending_wait(text("pending-wait"));
    window.set_tr_pending_open_chat(text("pending-open-chat"));
    window.set_tr_decision_card_label(text("decision-card-label"));
    window.set_tr_decision_choose(text("decision-choose"));
    window.set_tr_decision_outcome(text("decision-outcome"));
    window.set_tr_unread_divider(text("unread-divider"));
    window.set_tr_ai_badge(text("ai-badge"));
    window.set_tr_decision_mark(text("decision-mark"));
    window.set_tr_message_private_note(text("message-private-note"));
    window.set_tr_thread_replies(text("thread-replies"));
    window.set_tr_thread_working(text("thread-working"));
    window.set_tr_thread_done(text("thread-done"));
    window.set_tr_thread_view_work(text("thread-view-work"));
    window.set_tr_mention_read_range(text("mention-read-range"));
    window.set_tr_handoff_context(text("handoff-context"));
    window.set_tr_handoff_acknowledged(text("handoff-acknowledged"));
    window.set_tr_handoff_not_acknowledged(text("handoff-not-acknowledged"));
    window.set_tr_handoff_acknowledge(text("handoff-acknowledge"));
    window.set_tr_handoff_decision_owner(text("handoff-decision-owner"));
    window.set_tr_ext_card_label(text("ext-card-label"));
    window.set_tr_ext_card_question(text("ext-card-question"));
    window.set_tr_ext_address_label(text("ext-address-label"));
    window.set_tr_ext_subject_label(text("ext-subject-label"));
    window.set_tr_ext_basis_label(text("ext-basis-label"));
    window.set_tr_ext_send(text("ext-send"));
    window.set_tr_ext_fix(text("ext-fix"));
    window.set_tr_ext_lock_note(text("ext-lock-note"));
    window.set_tr_ext_sent(text("ext-sent"));
    window.set_tr_pane_thread_title(text("pane-thread-title"));
    window.set_tr_pane_close(text("pane-close"));
    window.set_tr_pane_back_to_thread(text("pane-back-to-thread"));
    window.set_tr_pane_ai_work_title(text("pane-ai-work-title"));
    window.set_tr_pane_ai_steps(text("pane-ai-steps"));
    window.set_tr_pane_ai_from(text("pane-ai-from"));
    window.set_tr_pane_ai_read_range(text("pane-ai-read-range"));
    window.set_tr_pane_ai_note(text("pane-ai-note"));
    window.set_tr_thread_input_placeholder(text("thread-input-placeholder"));
    window.set_tr_composer_placeholder(text("composer-placeholder"));
    window.set_tr_menu_help(text("menu-help"));
    window.set_tr_menu_about(text("menu-about"));
    window.set_tr_about_title(text("about-title"));
    window.set_tr_about_close(text("about-close"));
    missing
}

/// 窓の文言を `lang`（`ja` / `en`）の言語に切り替える（再起動なし）。
///
/// [`Messages::new`] で作り直した bundle で [`apply_translations`] を呼び直す
/// 経路。起動時の初期適用にも使い、起動後の切替にもう一度呼べる。用意して
/// いない言語では `None` を返し、`tr-*` を変えない（翻訳の無いキーを画面へ
/// 出さない）。適用できたときの返り値は [`apply_translations`] と同じ
/// （渡せなかったキーの一覧。空ならすべて渡せた）。
pub fn apply_lang(window: &MainWindow, lang: &str, vars: &FluentVars) -> Option<Vec<String>> {
    let messages = Messages::new(lang)?;
    Some(apply_translations(window, &messages, vars))
}
