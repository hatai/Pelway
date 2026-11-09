//! テーマとデザイントークンの名前。
//!
//! トークンはまだ確定していない。ここにあるのは名前だけで、値を入れない。
//! `.slint`（`ui/main.slint` の `DesignTokens` global）も名前だけを参照し、
//! テーマごとの値の表は [`token_value`] が「TODO: トークン確定後に入れる」の空の形。

/// テーマ。`name()` が返す安定名はこの enum だけが定義元（Slint 側には置かない）。
///
/// 2026-11-09 に `light`・`dark`・`light-high-contrast`・`dark-high-contrast` の 4 つに
/// 確定した。同日 17:30 の記録（docs/worklog/client.md 第4枠）にある
/// `light-accessible`・`dark-accessible` は古い名前であり、もう使わない。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Theme {
    /// 明るいテーマ。
    Light,
    /// 暗いテーマ。
    Dark,
    /// 明るい高コントラスト。
    LightHighContrast,
    /// 暗い高コントラスト。
    DarkHighContrast,
}

impl Theme {
    /// 定義されているテーマの一覧。ちょうど 4 つ。
    pub const ALL: [Theme; 4] = [
        Self::Light,
        Self::Dark,
        Self::LightHighContrast,
        Self::DarkHighContrast,
    ];

    /// テーマの安定名。テーマ名の文字列はここでだけ作る。
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::LightHighContrast => "light-high-contrast",
            Self::DarkHighContrast => "dark-high-contrast",
        }
    }

    /// 安定名からテーマを引く。未知の名前は `None`。
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|t| t.name() == name)
    }
}

/// デザイントークンの名前の一覧。
///
/// 書き出し時点（`ui-pro-draft-v0.1` の tokens）にある名前をそのまま使う。
/// 値は確定していないため、この一覧は名前だけ。`ui-pro-draft-v0.1` の
/// 生成物（`pelway_tokens.slint` など）は使わない。
pub const TOKEN_NAMES: &[&str] = &[
    // 面と文字
    "bg",
    "tile",
    "well",
    "ink",
    "sub",
    "mute",
    "line",
    // 強調
    "acc",
    "acc-fill",
    "acc-fill-hover",
    "acc-fill-pressed",
    "acc-wash",
    "on-acc",
    // 主ボタン
    "primary-bg",
    "primary-fg",
    "primary-bg-hover",
    "primary-bg-pressed",
    // アバター
    "avatar-human-bg",
    "avatar-human-fg",
    "avatar-ai-bg",
    "avatar-ai-fg",
    "ai-badge-bg",
    "ai-badge-fg",
    // 状態
    "pending-bg",
    "pending-bar",
    "success-fg",
    "success-bg",
    "danger-fg",
    "danger-bg",
    "warning-fg",
    "warning-bg",
    "unread",
    "private-bg",
    // 社外送信カード
    "ext-bg",
    "ext-fg",
    "ext-label",
    "ext-meta",
    "ext-address",
    "ext-link",
    "ext-body-bg",
    "ext-body-fg",
    "ext-go-bg",
    "ext-go-fg",
    "ext-fx-fg",
    "ext-fx-line",
    "ext-divider",
    // その他
    "focus-ring",
    "link",
    "selected-wash",
    "border-form",
];

/// テーマごとのトークンの値を引く。
///
/// TODO: トークン確定後に入れる。段階Aでは値を持たないため、常に `None`。
#[must_use]
pub fn token_value(_theme: Theme, _token: &str) -> Option<&'static str> {
    // TODO: トークン確定後に入れる（テーマごとの値の表）
    None
}
