//! テーマとデザイントークン。
//!
//! トークン名の正本は `tokens/pelway.tokens.json`（97fb70d の
//! `docs/design-system/tokens/pelway.tokens.json` の写し）の `color.<theme>`。
//! トークンの値（色）も同じ JSON から読む（`.rs`・`.slint`・`.ftl` に色の値は
//! 書かない）。値は [`token_value`] が返し、[`apply_design_tokens`] が
//! `ui/main.slint` の `DesignTokens` グローバルへ設定する。

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::MainWindow;
use slint::Global;

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
/// 正本はこのクレートに入れた `tokens/pelway.tokens.json`（97fb70d の
/// `docs/design-system/tokens/pelway.tokens.json` の写し）の `color.<theme>`。
/// この一覧はそのトークン名と一致させる（`tests/theme_tokens.rs` が正本を
/// 読んで照合する。正本で名前が増えた・消えた・変わったらテストが落ちる）。
/// 値は [`token_value`] が同じ JSON から読む。`ui-pro-draft-v0.1` の
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
    "line-tabbar",
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
    "avatar-human-bg-section",
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
    "track",
    "grabber",
    "scrim",
    // 面（メニュー・シート・トースト）
    "menu-bg",
    "sheet-bg",
    "sheet-inset-bg",
    "toast-bg",
    "toast-fg",
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
    "ext-option-bg",
    "ext-option-hover",
    // その他
    "focus-ring",
    "link",
    "selected-wash",
    "border-form",
];

/// テーマごとのトークンの値（色）を引く。
///
/// 値はトークンの正本 `tokens/pelway.tokens.json` の `color.<theme>` の
/// `$value`（srgb の成分と alpha）から読む。正本が参照・式など直接の色として
/// 読めない形のトークンは `None` を返す（自分で参照を解決しない）。
#[must_use]
pub fn token_value(theme: Theme, token: &str) -> Option<slint::Color> {
    let value = *COLOR_TOKENS.get(theme.name())?.get(token)?;
    Some(slint::Color::from_argb_f32(
        value.alpha,
        value.red,
        value.green,
        value.blue,
    ))
}

/// `MainWindow` の `DesignTokens` グローバルに、`theme` の全プロパティの値を設定する。
///
/// `DesignTokens` は `.slint` では名前だけを宣言し（初期値なし）、この関数が
/// 正本 JSON から読んだ値を設定する。窓を開く前に呼ぶことを想定している。
pub fn apply_design_tokens(window: &MainWindow, theme: Theme) {
    let tokens = crate::DesignTokens::get(window);
    tokens.set_bg(theme_brush(theme, "bg"));
    tokens.set_tile(theme_brush(theme, "tile"));
    tokens.set_well(theme_brush(theme, "well"));
    tokens.set_ink(theme_brush(theme, "ink"));
    tokens.set_sub(theme_brush(theme, "sub"));
    tokens.set_mute(theme_brush(theme, "mute"));
    tokens.set_line(theme_brush(theme, "line"));
    tokens.set_acc(theme_brush(theme, "acc"));
    tokens.set_acc_fill(theme_brush(theme, "acc-fill"));
    tokens.set_acc_fill_hover(theme_brush(theme, "acc-fill-hover"));
    tokens.set_acc_fill_pressed(theme_brush(theme, "acc-fill-pressed"));
    tokens.set_acc_wash(theme_brush(theme, "acc-wash"));
    tokens.set_on_acc(theme_brush(theme, "on-acc"));
    tokens.set_primary_bg(theme_brush(theme, "primary-bg"));
    tokens.set_primary_fg(theme_brush(theme, "primary-fg"));
    tokens.set_primary_bg_hover(theme_brush(theme, "primary-bg-hover"));
    tokens.set_primary_bg_pressed(theme_brush(theme, "primary-bg-pressed"));
    tokens.set_avatar_human_bg(theme_brush(theme, "avatar-human-bg"));
    tokens.set_avatar_human_fg(theme_brush(theme, "avatar-human-fg"));
    tokens.set_avatar_ai_bg(theme_brush(theme, "avatar-ai-bg"));
    tokens.set_avatar_ai_fg(theme_brush(theme, "avatar-ai-fg"));
    tokens.set_ai_badge_bg(theme_brush(theme, "ai-badge-bg"));
    tokens.set_ai_badge_fg(theme_brush(theme, "ai-badge-fg"));
    tokens.set_pending_bg(theme_brush(theme, "pending-bg"));
    tokens.set_pending_bar(theme_brush(theme, "pending-bar"));
    tokens.set_success_fg(theme_brush(theme, "success-fg"));
    tokens.set_success_bg(theme_brush(theme, "success-bg"));
    tokens.set_danger_fg(theme_brush(theme, "danger-fg"));
    tokens.set_danger_bg(theme_brush(theme, "danger-bg"));
    tokens.set_warning_fg(theme_brush(theme, "warning-fg"));
    tokens.set_warning_bg(theme_brush(theme, "warning-bg"));
    tokens.set_unread(theme_brush(theme, "unread"));
    tokens.set_private_bg(theme_brush(theme, "private-bg"));
    tokens.set_ext_bg(theme_brush(theme, "ext-bg"));
    tokens.set_ext_fg(theme_brush(theme, "ext-fg"));
    tokens.set_ext_label(theme_brush(theme, "ext-label"));
    tokens.set_ext_meta(theme_brush(theme, "ext-meta"));
    tokens.set_ext_address(theme_brush(theme, "ext-address"));
    tokens.set_ext_link(theme_brush(theme, "ext-link"));
    tokens.set_ext_body_bg(theme_brush(theme, "ext-body-bg"));
    tokens.set_ext_body_fg(theme_brush(theme, "ext-body-fg"));
    tokens.set_ext_go_bg(theme_brush(theme, "ext-go-bg"));
    tokens.set_ext_go_fg(theme_brush(theme, "ext-go-fg"));
    tokens.set_ext_fx_fg(theme_brush(theme, "ext-fx-fg"));
    tokens.set_ext_fx_line(theme_brush(theme, "ext-fx-line"));
    tokens.set_ext_divider(theme_brush(theme, "ext-divider"));
    tokens.set_focus_ring(theme_brush(theme, "focus-ring"));
    tokens.set_link(theme_brush(theme, "link"));
    tokens.set_selected_wash(theme_brush(theme, "selected-wash"));
    tokens.set_border_form(theme_brush(theme, "border-form"));
}

/// テーマのトークンの値を `DesignTokens` に渡す `brush` の形にする。
/// 読めないトークン（参照など）は既定の `brush` のままにする。
fn theme_brush(theme: Theme, token: &str) -> slint::Brush {
    token_value(theme, token)
        .map(slint::Brush::from)
        .unwrap_or_default()
}

// ---- トークンの正本 JSON の読み取り（std だけ） ----
//
// 読み方: (b) `include_str!` で JSON を実行形式に取り込み、標準ライブラリだけの
// 最小の読み取り器で `color.<theme>` の直下の `$value` を読む。build.rs での
// コード生成（読み方 a）はしない（build.rs は slint のコンパイルだけにする。
// JSON を読む crate は足さない）。この読み取りに必要なのは「オブジェクトのキー」
// 「$value が文字列（参照）か、colorSpace / components / alpha を持つオブジェクトか」
// 「components と alpha の数値」だけのため、真偽値・null は読み飛ばす。
// 限界: キーと colorSpace の値のエスケープは解かない（正本の該当箇所にエスケープは
// ない。docs/worklog/client.md に記録）。

/// トークンの正本（97fb70d の `docs/design-system/tokens/pelway.tokens.json`
/// のバイト単位の写し。`tests/theme_tokens.rs` が sha256 で一致を検査する）。
const TOKENS_JSON: &str = include_str!("../tokens/pelway.tokens.json");

/// 読み取った色トークン（srgb の成分と alpha。すべて 0〜1）。
#[derive(Clone, Copy, Debug)]
struct ColorValue {
    alpha: f32,
    red: f32,
    green: f32,
    blue: f32,
}

/// `color.<theme>.<token>` の表。最初の呼び出しで JSON を 1 回だけ読む。
static COLOR_TOKENS: LazyLock<HashMap<&'static str, HashMap<&'static str, ColorValue>>> =
    LazyLock::new(read_color_tokens);

/// 正本 JSON から `color.<theme>` の直下の読めるトークンだけを表に入れる。
///
/// `$` 始まりのメタキー、`mobile` のような `$value` を直接持たない子グループ、
/// `$value` が参照（文字列）で直接の色として読めないトークンは表に入れない
/// （`token_value` が `None` を返す）。デスクトップは直下の名前だけを使い、
/// `mobile` の上書きは使わない。
fn read_color_tokens() -> HashMap<&'static str, HashMap<&'static str, ColorValue>> {
    let root = parse_json(TOKENS_JSON);
    let color = object_get(&root, "color").expect("正本 JSON に color グループがあること");
    let Json::Object(themes) = color else {
        panic!("正本 JSON の color がオブジェクトでない");
    };
    let mut table = HashMap::new();
    for (theme, entries) in themes {
        if theme.starts_with('$') {
            continue;
        }
        let Json::Object(tokens) = entries else {
            continue;
        };
        let mut values = HashMap::new();
        for (name, entry) in tokens {
            if name.starts_with('$') {
                continue;
            }
            let Some(value) = object_get(entry, "$value") else {
                continue;
            };
            if let Some(color) = srgb_color(value) {
                values.insert(*name, color);
            }
        }
        table.insert(*theme, values);
    }
    table
}

/// `$value` が srgb の直接の色なら [`ColorValue`] を返す。
/// 参照（文字列）・srgb 以外・成分が3つの数値でないものは `None`。
fn srgb_color(value: &Json<'_>) -> Option<ColorValue> {
    if matches!(value, Json::Str(_)) {
        return None;
    }
    let Json::Str(space) = object_get(value, "colorSpace")? else {
        return None;
    };
    if *space != "srgb" {
        return None;
    }
    let Json::Array(items) = object_get(value, "components")? else {
        return None;
    };
    if items.len() != 3 {
        return None;
    }
    let mut rgb = [0.0f64; 3];
    for (slot, item) in rgb.iter_mut().zip(items) {
        let Json::Number(n) = item else {
            return None;
        };
        *slot = *n;
    }
    let alpha = match object_get(value, "alpha") {
        Some(Json::Number(n)) => *n,
        _ => 1.0,
    };
    Some(ColorValue {
        alpha: alpha as f32,
        red: rgb[0] as f32,
        green: rgb[1] as f32,
        blue: rgb[2] as f32,
    })
}

/// JSON の値（このモジュールで必要な範囲だけ）。
enum Json<'a> {
    /// オブジェクト（キーと値の並び）。
    Object(Vec<(&'a str, Json<'a>)>),
    /// 配列。
    Array(Vec<Json<'a>>),
    /// 文字列（参照の判定に使う。エスケープは解かない）。
    Str(&'a str),
    /// 数値（成分と alpha の読み取りに使う）。
    Number(f64),
    /// 真偽値・null（中身は使わない）。
    Scalar,
}

/// JSON を最後まで読む。読み残しがあればパニックする。
fn parse_json(text: &'static str) -> Json<'static> {
    let mut reader = JsonReader::new(text);
    let json = reader.read_value();
    reader.skip_ws();
    assert!(
        reader.peek().is_none(),
        "正本 JSON の終わり以降に読んでいない文字がある: バイト位置 {}",
        reader.pos
    );
    json
}

/// JSON の構造を読む読み取り器。
struct JsonReader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> JsonReader<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            bytes: text.as_bytes(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, byte: u8, what: &str) {
        assert_eq!(
            self.peek(),
            Some(byte),
            "正本 JSON が読めない（{what} を期待）: バイト位置 {pos}",
            pos = self.pos
        );
        self.pos += 1;
    }

    /// 文字列を読み、引用符の内側の部分を返す（エスケープは解かない）。
    fn read_string(&mut self) -> &'a str {
        self.expect(b'"', "文字列の開始");
        let start = self.pos;
        while let Some(c) = self.peek() {
            match c {
                b'"' => {
                    let raw = std::str::from_utf8(&self.bytes[start..self.pos])
                        .expect("正本 JSON は UTF-8 のこと");
                    self.pos += 1;
                    return raw;
                }
                // エスケープの 2 バイト目（" \ / b f n r t u）は常に ASCII。
                b'\\' => self.pos += 2,
                _ => self.pos += 1,
            }
        }
        panic!("正本 JSON の文字列が閉じていない: バイト位置 {start}");
    }

    fn read_value(&mut self) -> Json<'a> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.read_object(),
            Some(b'[') => self.read_array(),
            Some(b'"') => Json::Str(self.read_string()),
            Some(c) if c == b'-' || c.is_ascii_digit() => self.read_number(),
            Some(b't' | b'f' | b'n') => {
                // true / false / null: 次の区切りまで読み飛ばす（中身は使わない）。
                while !matches!(self.peek(), None | Some(b',' | b'}' | b']')) {
                    self.pos += 1;
                }
                Json::Scalar
            }
            None => panic!("正本 JSON が途中で終わっている: バイト位置 {}", self.pos),
            _ => panic!("正本 JSON の値が読めない: バイト位置 {}", self.pos),
        }
    }

    /// 数値を読む（次の区切りまでを f64 として解釈する）。
    fn read_number(&mut self) -> Json<'a> {
        let start = self.pos;
        while !matches!(self.peek(), None | Some(b',' | b'}' | b']')) {
            self.pos += 1;
        }
        let raw = std::str::from_utf8(&self.bytes[start..self.pos])
            .expect("正本 JSON は UTF-8 のこと")
            .trim();
        let n: f64 = raw
            .parse()
            .unwrap_or_else(|e| panic!("正本 JSON の数値 {raw} が読めない: {e}"));
        Json::Number(n)
    }

    fn read_object(&mut self) -> Json<'a> {
        self.expect(b'{', "オブジェクトの開始");
        let mut entries = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b'}') => {
                    self.pos += 1;
                    return Json::Object(entries);
                }
                Some(b',') => self.pos += 1,
                Some(b'"') => {
                    let key = self.read_string();
                    self.skip_ws();
                    self.expect(b':', "キーのあとのコロン");
                    entries.push((key, self.read_value()));
                }
                _ => panic!(
                    "正本 JSON のオブジェクトが読めない: バイト位置 {}",
                    self.pos
                ),
            }
        }
    }

    fn read_array(&mut self) -> Json<'a> {
        self.expect(b'[', "配列の開始");
        let mut items = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b']') => {
                    self.pos += 1;
                    return Json::Array(items);
                }
                Some(b',') => self.pos += 1,
                _ => {
                    items.push(self.read_value());
                }
            }
        }
    }
}

/// オブジェクトからキーに対応する値を引く。
/// 借りの寿命（'j）とデータの寿命（'a）を分けて、中の文字列の寿命を縮めない。
fn object_get<'j, 'a>(json: &'j Json<'a>, key: &str) -> Option<&'j Json<'a>> {
    let Json::Object(entries) = json else {
        return None;
    };
    entries.iter().find(|(k, _)| *k == key).map(|(_, v)| v)
}
