//! テーマとトークンの検査。
//!
//! - テーマの enum がちょうど 4 つ（light・dark・light-high-contrast・dark-high-contrast）。
//! - トークンの値を入れていないこと（テーマごとの値の表は TODO の空の形）。
//! - `.slint` が参照しているトークン名が、許可された一覧にあること。

use pelway_client_ui::theme::{TOKEN_NAMES, Theme, token_value};

const SLINT: &str = include_str!("../ui/main.slint");

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
