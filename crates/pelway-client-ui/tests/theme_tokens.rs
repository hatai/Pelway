//! テーマとトークンの検査。
//!
//! - テーマの enum がちょうど 4 つ（light・dark・light-high-contrast・dark-high-contrast）。
//! - `.slint` が参照しているトークン名が、許可された一覧にあること。
//! - 許可された一覧 [`TOKEN_NAMES`] が正本 JSON（`tokens/pelway.tokens.json`）の
//!   `color.<theme>` のトークン名と一致すること（正本の写しは 97fb70d の
//!   `docs/design-system/tokens/pelway.tokens.json`）。
//! - その写しのバイト列が、97fb70d の正本と sha256 で一致すること
//!   （名前の照合だけでは値や構造のずれを検出できないため）。
//! - Rust が返すトークンの値（色）が正本 JSON の `$value` と一致すること。

use pelway_client_ui::theme::{TOKEN_NAMES, Theme, token_value};

const SLINT: &str = include_str!("../ui/main.slint");
const CANONICAL_TOKENS: &str = include_str!("../tokens/pelway.tokens.json");

// ---- 正本 JSON の最小の読み取り（std だけ） ----
//
// このテストに必要なのは「オブジェクトのキー」「$value の形（文字列か、
// colorSpace / components / alpha を持つオブジェクトか）」「components と alpha の数値」
// だけのため、真偽値・null の中身は読まずに読み飛ばす。
// 限界: キーと文字列の値のエスケープは解かない（正本のキー名と colorSpace の値に
// エスケープはない。スキャナは文字列リテラルを解釈しないため、コメントにも色の値の
// パターンを書かない運用と同じく、この読み方の前提は docs/worklog/client.md に記録）。

/// 構造だけを読んだ JSON の値。
enum Json {
    /// オブジェクト（キーと値の並び）。
    Object(Vec<(String, Json)>),
    /// 配列。
    Array(Vec<Json>),
    /// 文字列（参照などの判定に使う）。
    Str(String),
    /// 数値（components・alpha の読み取りに使う）。
    Number(f64),
    /// 真偽値・null（中身は使わない）。
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
            Some(b'"') => Json::Str(self.read_string().to_owned()),
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
    fn read_number(&mut self) -> Json {
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
        let mut items = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b']') => {
                    self.pos += 1;
                    return Json::Array(items);
                }
                Some(b',') => self.pos += 1,
                // 要素は読み飛ばす（このテストで配列の中身は使わない）。
                _ => {
                    items.push(self.read_value());
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

// ---- SHA-256（std だけの実装） ----
//
// Cargo.lock にハッシュ用の crate（sha2 など）がなく、オフラインで新しい依存を
// 足せないため、テストの中に標準ライブラリだけで SHA-256 を書く。
// 下の「既知の入力と答え」の照合（FIPS 180-4 の例）で実装の正しさを確認する。

/// SHA-256 の丸め定数（FIPS 180-4 の K）。
const SHA256_K: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

/// SHA-256 の計算器（ストリーミング更新＋締め）。
struct Sha256 {
    state: [u32; 8],
    buf: [u8; 64],
    buf_len: usize,
    total_bits: u64,
}

impl Sha256 {
    fn new() -> Self {
        Self {
            state: [
                0x6a09_e667,
                0xbb67_ae85,
                0x3c6e_f372,
                0xa54f_f53a,
                0x510e_527f,
                0x9b05_688c,
                0x1f83_d9ab,
                0x5be0_cd19,
            ],
            buf: [0; 64],
            buf_len: 0,
            total_bits: 0,
        }
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for (i, chunk) in block.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA256_K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (s, v) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *s = s.wrapping_add(v);
        }
    }

    fn update(&mut self, mut data: &[u8]) {
        self.total_bits = self
            .total_bits
            .wrapping_add(u64::try_from(data.len()).unwrap() * 8);
        if self.buf_len > 0 {
            let take = (64 - self.buf_len).min(data.len());
            self.buf[self.buf_len..self.buf_len + take].copy_from_slice(&data[..take]);
            self.buf_len += take;
            data = &data[take..];
            if self.buf_len == 64 {
                let block = self.buf;
                self.compress(&block);
                self.buf_len = 0;
            }
        }
        while data.len() >= 64 {
            let (block, rest) = data.split_at(64);
            let mut chunk64 = [0u8; 64];
            chunk64.copy_from_slice(block);
            self.compress(&chunk64);
            data = rest;
        }
        if !data.is_empty() {
            self.buf[..data.len()].copy_from_slice(data);
            self.buf_len = data.len();
        }
    }

    fn finalize(mut self) -> [u8; 32] {
        let bit_len = self.total_bits;
        self.update(&[0x80]);
        while self.buf_len != 56 {
            self.update(&[0]);
        }
        let mut block = self.buf;
        block[56..].copy_from_slice(&bit_len.to_be_bytes());
        self.compress(&block);
        let mut out = [0u8; 32];
        for (chunk, word) in out.chunks_exact_mut(4).zip(self.state) {
            chunk.copy_from_slice(&word.to_be_bytes());
        }
        out
    }
}

/// バイト列の SHA-256 を16進の小文字で返す。
fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let digest = hasher.finalize();
    digest.iter().fold(String::new(), |mut acc, b| {
        use std::fmt::Write as _;
        let _ = write!(acc, "{b:02x}");
        acc
    })
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

/// 正本 JSON の `color.<theme>.<token>` の `$value` を `(alpha, r, g, b)` で返す。
///
/// `$value` が文字列（DTCG の参照）や、colorSpace が srgb 以外・components が
/// 3つの数値でないなど、直接の色として読めない形なら `None`。
fn canonical_color_token_value(
    json: &Json,
    theme: &str,
    token: &str,
) -> Option<(f64, f64, f64, f64)> {
    let color = object_get(json, "color")?;
    let theme_obj = object_get(color, theme)?;
    let token_obj = object_get(theme_obj, token)?;
    let value = object_get(token_obj, "$value")?;
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
    let mut rgb = [0.0f64; 3];
    if items.len() != 3 {
        return None;
    }
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
    Some((alpha, rgb[0], rgb[1], rgb[2]))
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

/// 読めないトークン（正本の `$value` が直接の色でないもの）。
///
/// `avatar-human-bg-section` は4テーマすべてで `{color.<theme>.avatar-human-bg}`
/// の形の参照（DTCG のエイリアス）。自分で参照を解決して値を作らないため、
/// このトークンは値なし（`None`）のまま残す。
const UNREADABLE_TOKENS: &[&str] = &["avatar-human-bg-section"];

/// JSON の成分（0〜1）と、色のチャネル（0〜255）が丸めの誤差の範囲で一致すること。
fn assert_channel(theme: &str, token: &str, what: &str, json_value: f64, channel: u8) {
    let expected = json_value * 255.0;
    let got = f64::from(channel);
    assert!(
        (got - expected).abs() <= 1.0,
        "{theme}/{token} の {what}: JSON の成分と色のチャネルが 1/255 の範囲で一致しない"
    );
}

#[test]
fn token_values_match_canonical_json() {
    // 4テーマすべてで、TOKEN_NAMES の各トークンについて、Rust が返す値が
    // 正本 JSON の $value（srgb の成分と alpha）と一致すること。
    // 読めないトークン（UNREADABLE_TOKENS）は両方とも値なしであること。
    let json = parse_canonical();
    for theme in Theme::ALL {
        let mut filled = 0usize;
        for token in TOKEN_NAMES {
            let expected = canonical_color_token_value(&json, theme.name(), token);
            let got = token_value(theme, token);
            if UNREADABLE_TOKENS.contains(token) {
                assert!(
                    expected.is_none(),
                    "{token} は読めないトークンの前提（正本の $value が直接の色でない）"
                );
                assert!(
                    got.is_none(),
                    "読めないトークン {theme:?}/{token} に値を入れていないこと"
                );
                continue;
            }
            let Some((exp_a, exp_r, exp_g, exp_b)) = expected else {
                panic!(
                    "正本の {}/{token} が直接の色として読める前提が崩れた",
                    theme.name()
                );
            };
            let got = got.unwrap_or_else(|| {
                panic!(
                    "トークン {}/{token} の値が None（実装が正本を読めていない）",
                    theme.name()
                )
            });
            assert_channel(theme.name(), token, "alpha", exp_a, got.alpha());
            assert_channel(theme.name(), token, "red", exp_r, got.red());
            assert_channel(theme.name(), token, "green", exp_g, got.green());
            assert_channel(theme.name(), token, "blue", exp_b, got.blue());
            filled += 1;
        }
        assert_eq!(
            filled,
            TOKEN_NAMES.len() - UNREADABLE_TOKENS.len(),
            "テーマ {} で値を入れたトークンの件数（読めないトークンを除く）",
            theme.name()
        );
    }
}

#[test]
fn unreadable_tokens_match_canonical_json_forms() {
    // 読めないトークンの前提: 4テーマすべてで正本の $value が参照（文字列）であること。
    // 正本がこの形をやめたら（直接の色になったら）、このテストが落ちる。
    // そのときは UNREADABLE_TOKENS から外して値を入れてよいかを確かめる。
    let json = parse_canonical();
    for theme in Theme::ALL {
        for token in UNREADABLE_TOKENS {
            let value = canonical_color_token_value(&json, theme.name(), token);
            assert!(
                value.is_none(),
                "正本の {}/{token} が直接の色として読める形になった（前提の見直し）",
                theme.name()
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
    // `in-out property <brush> xxx;` の形を抜き出す（DesignTokens ブロック内のみ。
    // 値は Rust から設定するため in-out。.slint の中に色の値は書かない）。
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
        if let Some(rest) = trimmed.strip_prefix("in-out property <brush> ") {
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

// ---- 写しと正本の一致（sha256） ----

#[test]
fn sha256_known_answers() {
    // std だけで書いた SHA-256 の実装が正しいこと（FIPS 180-4 の既知の例）。
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn token_copy_bytes_match_canonical_by_sha256() {
    // 97fb70d の docs/design-system/tokens/pelway.tokens.json を byte 単位で
    // 写したものが crates/pelway-client-ui/tokens/pelway.tokens.json。
    // 名前の照合（token_names_match_canonical_json）だけでは値や構造のずれを
    // 検出できないため、バイト列の sha256 を 97fb70d 時点の値と比べる。
    // 正本が更新されたときは、git show で新しい正本の sha256 を確かめてから
    // この定数を書き換える（写し自身も同じバイト列に置き換える）。
    const EXPECTED_SHA256: &str =
        "4f523ce19145250df391975dea3924aa7b5bdcbd417c0c31e910f3b17c543953";
    assert_eq!(
        sha256_hex(CANONICAL_TOKENS.as_bytes()),
        EXPECTED_SHA256,
        "tokens/pelway.tokens.json のバイト列が 97fb70d の正本から変わっている"
    );
}
