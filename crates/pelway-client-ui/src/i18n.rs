//! 画面の文言の Fluent キー。
//!
//! 画面の文言は `i18n/ja/main.ftl` と `i18n/en/main.ftl` に置く。
//! 段階Aでは Fluent の crate を使えないため、ここには Rust が使うキーの
//! 一覧だけを置く。段階Bで Fluent から引いて、その文字列を Slint の
//! `tr-*` プロパティへ渡す。Slint の `@tr()`（gettext）は仕様が Fluent のため使わない。
//!
//! 英語の文言は意味の分かる仮の訳。

/// Rust が使う文言キーの一覧。`ui/main.slint` の `tr-*` プロパティと同じ集合。
pub const USED_KEYS: &[&str] = &[
    // 左の列（rail）
    "rail-viewing-as",
    "rail-today",
    "rail-channels",
    "rail-dm-people",
    "rail-dm-ai",
    // AI 社員の在席状態（rail の表示）
    "ai-presence-standby",
    "ai-presence-working",
    "ai-presence-waiting",
    // 上の帯
    "search-placeholder",
    "search-shortcut",
    // 「今日」の画面
    "today-title",
    "today-section-consultations",
    "today-section-pending",
    "today-section-decided",
    "today-section-empty",
    "today-quiet",
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
];
