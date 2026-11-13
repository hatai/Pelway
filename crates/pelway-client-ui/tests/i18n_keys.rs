//! `.ftl` のキーの照合（標準ライブラリだけの簡易パーサ）。
//!
//! - 日英の `.ftl` でキーの集まりが同じであること。
//! - どちらにも空の値がないこと。
//! - Rust（`i18n::USED_KEYS`）と `.slint`（`tr-*` プロパティ）が使っているキーが、
//!   両方の `.ftl` にあること。

use std::collections::BTreeMap;

use pelway_client_ui::i18n::USED_KEYS;

const JA: &str = include_str!("../i18n/ja/main.ftl");
const EN: &str = include_str!("../i18n/en/main.ftl");
const SLINT: &str = include_str!("../ui/main.slint");

/// `.ftl` の行頭「キー = 値」を読む。コメント行（先頭 #）と空行は読み飛ばす。
/// Fluent の完全な構文は扱わない（段階Aは Fluent の crate を使わないため）。
fn parse_ftl(text: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once(" = ") else {
            panic!("行を「キー = 値」の形で読めない: {trimmed}");
        };
        assert!(
            key.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "キーは安定した名前（小文字・数字・ハイフン）にする: {key}"
        );
        map.insert(key.to_owned(), value.to_owned());
    }
    map
}

/// `.slint` 内の `tr-<key>` 出現からキー集合を作る。
fn slint_tr_keys(text: &str) -> Vec<String> {
    let marker = "tr-";
    let mut keys = Vec::new();
    let mut rest = text;
    while let Some(pos) = rest.find(marker) {
        let after = &rest[pos + marker.len()..];
        let key: String = after
            .chars()
            .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
            .collect();
        if !key.is_empty() {
            keys.push(key);
        }
        rest = after;
    }
    keys.sort();
    keys.dedup();
    keys
}

#[test]
fn ja_and_en_have_the_same_key_set() {
    let ja = parse_ftl(JA);
    let en = parse_ftl(EN);
    let ja_keys: Vec<&String> = ja.keys().collect();
    let en_keys: Vec<&String> = en.keys().collect();
    assert_eq!(
        ja_keys, en_keys,
        "日英の .ftl のキーの集まりは同じであること"
    );
}

#[test]
fn no_empty_values_in_either_locale() {
    for (locale, text) in [("ja", JA), ("en", EN)] {
        for (key, value) in parse_ftl(text) {
            assert!(!value.trim().is_empty(), "{locale} の {key} が空の値");
        }
    }
}

#[test]
fn rust_used_keys_exist_in_both_ftl_files() {
    let ja = parse_ftl(JA);
    let en = parse_ftl(EN);
    for key in USED_KEYS {
        assert!(
            ja.contains_key(*key),
            "ja の .ftl に Rust が使うキー {key} がない"
        );
        assert!(
            en.contains_key(*key),
            "en の .ftl に Rust が使うキー {key} がない"
        );
    }
}

#[test]
fn slint_tr_keys_exist_in_both_ftl_files() {
    let ja = parse_ftl(JA);
    let en = parse_ftl(EN);
    for key in slint_tr_keys(SLINT) {
        assert!(
            ja.contains_key(key.as_str()),
            "ja の .ftl に .slint が使うキー {key} がない"
        );
        assert!(
            en.contains_key(key.as_str()),
            "en の .ftl に .slint が使うキー {key} がない"
        );
    }
}

#[test]
fn slint_tr_keys_and_rust_used_keys_are_the_same_set() {
    // Rust から Fluent で引いて Slint の tr-* プロパティへ渡す形。
    // 両者の键集合が一致していることで、足りない・余分なキーを出さない。
    let slint_keys = slint_tr_keys(SLINT);
    let mut rust_keys: Vec<&str> = USED_KEYS.to_vec();
    rust_keys.sort_unstable();
    rust_keys.dedup();
    assert_eq!(
        slint_keys, rust_keys,
        ".slint の tr-* キーと Rust の USED_KEYS は同じ集合であること"
    );
}

#[test]
fn ftl_has_no_extra_keys_beyond_the_used_set() {
    let ja = parse_ftl(JA);
    let used: Vec<&str> = USED_KEYS.to_vec();
    for key in ja.keys() {
        assert!(
            used.contains(&key.as_str()),
            "ja の .ftl に使われていないキー {key} がある"
        );
    }
}
