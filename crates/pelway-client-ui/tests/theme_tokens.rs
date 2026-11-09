//! テーマとトークンの検査。
//!
//! - テーマの enum がちょうど 4 つ（light・dark・light-high-contrast・dark-high-contrast）。
//! - トークンの値を入れていないこと（テーマごとの値の表は TODO の空の形）。
//! - `.slint` が参照しているトークン名が、許可された一覧にあること。
//! - 許可された一覧 [`TOKEN_NAMES`] が正本 JSON（`tokens/pelway.tokens.json`）の
//!   `color.<theme>` のトークン名と一致すること（正本の写しは 97fb70d の
//!   `docs/design-system/tokens/pelway.tokens.json`）。

use pelway_client_ui::theme::{TOKEN_NAMES, Theme, token_value};

const SLINT: &str = include_str!("../ui/main.slint");
const CANONICAL_TOKENS: &str = include_str!("../tokens/pelway.tokens.json");

// ---- 正本 JSON の最小の読み取り（std だけ） ----
//
// このテストに必要なのは「オブジェクトのキー」と「その値が `$value` を持つトークンか」
// だけなので、文字列・数値・真偽値・null の中身は読まずに構造だけを読む。
// 限界: キーの文字列のエスケープは解かない（正本のキー名にエスケープはない。
// スキャナは文字列リテラルを解釈しないため、コメントにも色の値のパターンを
// 書かない運用と同じく、この読み方の前提は docs/worklog/client.md 第9枠に記録）。

/// 構造だけを読んだ JSON の値。
enum Json {
    /// オブジェクト（キーと値の並び）。
    Object(Vec<(String, Json)>),
    /// 配列（中身の要素はこのテストでは読まない）。
    Array,
    /// 文字列・数値・真偽値・null（中身は使わない）。
    Scalar,
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

    fn read_value(&mut self) -> Json {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.read_object(),
            Some(b'[') => self.read_array(),
            Some(b'"') => {
                self.read_string();
                Json::Scalar
            }
            Some(_) => {
                // 数値・真偽値・null: 次の区切りまで読み飛ばす。
                while !matches!(self.peek(), None | Some(b',' | b'}' | b']')) {
                    self.pos += 1;
                }
                Json::Scalar
            }
            None => panic!("正本 JSON が途中で終わっている: バイト位置 {}", self.pos),
        }
    }

    fn read_object(&mut self) -> Json {
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
                    let key = self.read_string().to_string();
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

    fn read_array(&mut self) -> Json {
        self.expect(b'[', "配列の開始");
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b']') => {
                    self.pos += 1;
                    return Json::Array;
                }
                Some(b',') => self.pos += 1,
                // 要素は読み飛ばす（このテストで配列の中身は使わない）。
                _ => {
                    self.read_value();
                }
            }
        }
    }
}

/// オブジェクトからキーに対応する値を引く。
fn object_get<'j>(json: &'j Json, key: &str) -> Option<&'j Json> {
    let Json::Object(entries) = json else {
        return None;
    };
    entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// 正本 JSON を最後まで読む。読み残しがあればパニックする。
fn parse_canonical() -> Json {
    let mut reader = JsonReader::new(CANONICAL_TOKENS);
    let json = reader.read_value();
    reader.skip_ws();
    assert!(
        reader.peek().is_none(),
        "正本 JSON の終わり以降に読んでいない文字がある: バイト位置 {}",
        reader.pos
    );
    json
}

/// 正本 JSON の `color.<theme>` の直下にあるトークン名（昇順）。
///
/// `$` 始まりのメタキー（`$description` など）と `mobile` のような子グループ
/// （`$value` を直接持たないもの）は除く。デスクトップクライアントが使うのは
/// プラットフォームをまたぐ普通の名前の側。
fn canonical_color_token_names(json: &Json, theme: &str) -> Vec<String> {
    let color = object_get(json, "color").expect("正本 JSON に color グループがあること");
    let theme_obj = object_get(color, theme)
        .unwrap_or_else(|| panic!("正本 JSON の color にテーマ {theme} があること"));
    let Json::Object(entries) = theme_obj else {
        panic!("正本 JSON の color.{theme} がオブジェクトでない");
    };
    let mut names: Vec<String> = entries
        .iter()
        .filter(|(k, v)| {
            !k.starts_with('$') && matches!(v, Json::Object(_)) && object_get(v, "$value").is_some()
        })
        .map(|(k, _)| k.clone())
        .collect();
    names.sort();
    names
}

/// [`TOKEN_NAMES`] を昇順にしたもの（正本との照合用）。
fn sorted_token_names() -> Vec<String> {
    let mut names = TOKEN_NAMES.to_vec();
    names.sort_unstable();
    names.into_iter().map(str::to_string).collect()
}

/// `DesignTokens.<name>` 参照の <name> をすべて抜き出す（std だけの簡易スキャン）。
fn design_token_refs(text: &str) -> Vec<String> {
    let marker = "DesignTokens.";
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(pos) = rest.find(marker) {
        let after = &rest[pos + marker.len()..];
        let name: String = after
            .chars()
            .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
            .collect();
        if !name.is_empty() {
            names.push(name);
        }
        rest = after;
    }
    names.sort();
    names.dedup();
    names
}

#[test]
fn theme_enum_has_exactly_four_variants() {
    assert_eq!(Theme::ALL.len(), 4, "テーマはちょうど 4 つ");
    let mut names: Vec<&str> = Theme::ALL.iter().map(|t| t.name()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        ["dark", "dark-high-contrast", "light", "light-high-contrast"],
        "テーマ名は light・dark・light-high-contrast・dark-high-contrast だけ"
    );
}

#[test]
fn theme_name_round_trip() {
    for theme in Theme::ALL {
        assert_eq!(Theme::from_name(theme.name()), Some(theme));
    }
    assert_eq!(
        Theme::from_name("light-accessible"),
        None,
        "17:30 の記録の古い名前は受け付けない"
    );
    assert_eq!(Theme::from_name("dark-accessible"), None);
    assert_eq!(Theme::from_name("unknown"), None);
}

#[test]
fn token_names_have_no_duplicates() {
    let mut sorted = TOKEN_NAMES.to_vec();
    sorted.sort_unstable();
    let len_before = sorted.len();
    sorted.dedup();
    assert_eq!(len_before, sorted.len(), "トークン名に重複がないこと");
}

#[test]
fn token_values_are_not_filled_yet() {
    // トークンの値は未確定。テーマごとの値の表は「TODO: トークン確定後に入れる」の空の形。
    for theme in Theme::ALL {
        for token in TOKEN_NAMES {
            assert!(
                token_value(theme, token).is_none(),
                "トークン {token} の値を入れてはならない（確定後に入れる）"
            );
        }
    }
}

#[test]
fn slint_design_token_refs_are_all_known() {
    let refs = design_token_refs(SLINT);
    assert!(
        !refs.is_empty(),
        "ui/main.slint は DesignTokens を参照していること（骨組み）"
    );
    for name in &refs {
        assert!(
            TOKEN_NAMES.contains(&name.as_str()),
            ".slint が参照するトークン {name} が一覧にない"
        );
    }
}

#[test]
fn slint_does_not_define_its_own_token_names() {
    // global DesignTokens のプロパティ名も一覧のトークン名だけ。
    // `out property <brush> xxx;` の形を抜き出す（DesignTokens ブロック内のみ）。
    let start = SLINT
        .find("export global DesignTokens")
        .expect("DesignTokens global があること");
    let end = SLINT[start..]
        .find('}')
        .map(|i| start + i)
        .expect("DesignTokens の閉じ括弧");
    let block = &SLINT[start..end];
    for line in block.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("out property <brush> ") {
            let name = rest.trim_end_matches(';').trim();
            assert!(
                TOKEN_NAMES.contains(&name),
                "DesignTokens のプロパティ {name} がトークン一覧にない"
            );
        }
    }
}

#[test]
fn canonical_json_has_color_tokens_for_every_theme() {
    let json = parse_canonical();
    for theme in Theme::ALL {
        let names = canonical_color_token_names(&json, theme.name());
        assert!(
            !names.is_empty(),
            "正本 JSON の color.{} にトークンが少なくとも1つあること",
            theme.name()
        );
    }
}

#[test]
fn token_names_match_canonical_json() {
    // 正本（tokens/pelway.tokens.json の color.<theme>）のトークン名と
    // TOKEN_NAMES が一致すること。正本で名前が増えた・消えた・変わったら
    // このテストが落ちる（Rust 側だけの自己整合では保証にならない）。
    let json = parse_canonical();
    let expected = sorted_token_names();
    for theme in Theme::ALL {
        assert_eq!(
            canonical_color_token_names(&json, theme.name()),
            expected,
            "TOKEN_NAMES が正本 JSON の color.{} のトークン名と一致すること\
             （正本に増えた名前は TOKEN_NAMES に足し、正本から消えた名前は\
             .slint で使っていないか確かめてから除く）",
            theme.name()
        );
    }
}
