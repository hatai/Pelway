//! 色の値の不在検査。
//!
//! `crates/pelway-client-ui` の `.slint`・`.rs`・`.ftl`（クレート直下の
//! `build.rs` を含む）に、色の値（# のあとに 16 進 3・4・6・8 桁、
//! rgb / rgba / hsl / hsla の関数呼び出し、Slint の Colors シングルトン参照）が
//! 書かれていないことを確かめる。色はトークン名だけで参照する。
//!
//! このテスト自身が検出対象に含まれるため、パターンの文字列は分割して組み立てる
//! （このファイルに連続した色のリテラルを置かない。上の説明にも書かない）。

use std::fs;
use std::path::{Path, PathBuf};

const SLINT: &str = include_str!("../ui/main.slint");
const JA: &str = include_str!("../i18n/ja/main.ftl");
const EN: &str = include_str!("../i18n/en/main.ftl");

/// スキャン対象の検索語（ファイル上で連続したリテラルにしないため分割組み立て）。
fn forbidden_needles() -> Vec<String> {
    [
        ["rg", "b("].concat(),
        ["rg", "ba("].concat(),
        ["hs", "l("].concat(),
        ["hs", "la("].concat(),
        ["colo", "rs."].concat(),
    ]
    .to_vec()
}

fn is_hex_digit(c: char) -> bool {
    c.is_ascii_hexdigit()
}

/// 色の値らしき文字列をすべて抜き出す。
fn scan_color_values(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '#' {
            let mut j = i + 1;
            while j < chars.len() && is_hex_digit(chars[j]) {
                j += 1;
            }
            let hex_len = j - i - 1;
            if matches!(hex_len, 3 | 4 | 6 | 8) {
                found.push(chars[i..j].iter().collect::<String>());
            }
            i = j;
        } else {
            i += 1;
        }
    }
    let lower = text.to_ascii_lowercase();
    for needle in forbidden_needles() {
        let needle_lower = needle.to_ascii_lowercase();
        if lower.contains(&needle_lower) {
            found.push(needle);
        }
    }
    found.sort();
    found.dedup();
    found
}

/// クレート内の `.rs`・`.slint`・`.ftl` を列挙する（cargo の出力物は除く）。
/// クレート直下の `build.rs` も `.rs` として対象に含める。
fn source_files() -> Vec<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for dir in ["src", "tests", "ui", "i18n"] {
        let dir_path = manifest.join(dir);
        collect_files(&dir_path, &mut files);
    }
    let build_rs = manifest.join("build.rs");
    if build_rs.is_file() {
        files.push(build_rs);
    }
    files.sort();
    files
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, out);
        } else if matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("rs") | Some("slint") | Some("ftl")
        ) {
            out.push(path);
        }
    }
}

#[test]
fn scanner_detects_hex_color_values() {
    // スキャナ自体が検出できることの単体確認（連結してファイル上にリテラルを置かない）。
    let sample = String::from("color: #") + "a1b2c3;";
    assert_eq!(scan_color_values(&sample).len(), 1);
    let sample4 = String::from("#") + "abc8;";
    assert_eq!(
        scan_color_values(&sample4).as_slice(),
        [String::from("#") + "abc8"]
    );
    // 5 桁・7 桁は色の値の形式でないため誤検出しない。
    let sample7 = String::from("#") + "abc8def;";
    assert!(
        scan_color_values(&sample7).is_empty(),
        "7 桁は色の値ではない"
    );
}

#[test]
fn scanner_detects_function_and_named_colors() {
    let sample = ["background: ", "hsl", "(120, 50%, 40%);"].concat();
    assert_eq!(scan_color_values(&sample).len(), 1);
    let named = ["fill: ", "Colors", ".red;"].concat();
    assert_eq!(
        scan_color_values(&named).len(),
        1,
        "Slint の名前付き色も検出する"
    );
}

#[test]
fn no_color_values_anywhere_in_the_crate() {
    let mut failures = Vec::new();
    for path in source_files() {
        let Ok(text) = fs::read_to_string(&path) else {
            panic!("読めない: {}", path.display());
        };
        let found = scan_color_values(&text);
        if !found.is_empty() {
            failures.push(format!("{}: {:?}", path.display(), found));
        }
    }
    assert!(
        failures.is_empty(),
        "色の値が見つかった（色はトークン名だけで参照する）:\n{}",
        failures.join("\n")
    );
}

#[test]
fn bundled_slint_and_ftl_are_part_of_the_scan() {
    // include_str! した主要ファイルが走査対象に含まれていること（抜け防止）。
    let files = source_files();
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert!(files.contains(&manifest.join("ui/main.slint")));
    assert!(files.contains(&manifest.join("i18n/ja/main.ftl")));
    assert!(files.contains(&manifest.join("i18n/en/main.ftl")));
    // クレート直下の build.rs も走査対象に入っていること。
    assert!(files.contains(&manifest.join("build.rs")));
    // スキャン関数が実際に空でないテキストを検査できていること。
    assert!(!SLINT.is_empty() || !JA.is_empty() || !EN.is_empty());
}
