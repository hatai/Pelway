//! Rust の型から TypeScript の型を生成し、置いたもの（`bindings/`）と
//! 一致することを確認する。差分があればテストが落ちる。
//!
//! A2 は UI を TS/React DOM とするので、Rust 側の型を変えたのに TS 側の型を
//! 忘れると UI 側が古い型でコンパイルされる。CI でこの差分を検出できるように
//! するのがこのテストの目的。
//!
//! 置いた生成結果の更新は `UPDATE_GOLDEN=1 cargo test -p pelway-dev-typegen-probe`
//! で行う（更新したあと、内容を確認してコミットする）。

use std::path::{Path, PathBuf};

use pelway_dev_typegen_probe::types::{ChannelId, Message, MessageBody, MessageId, NoticeLevel};
use ts_rs::TS;

/// 置いている生成結果の置き場。
fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("bindings")
}

/// 置いたものと生成したものが一致することを確認し、違えば差分を出して落とす。
/// 置いたものが無ければ作る（UPDATE_GOLDEN=1 のときだけ更新する）。
fn assert_same(golden: &Path, actual: &str) {
    if !golden.exists() {
        assert!(
            std::env::var("UPDATE_GOLDEN").is_ok(),
            "置いた生成結果に {} が無い（UPDATE_GOLDEN=1 で生成できる）",
            golden.display()
        );
        std::fs::write(golden, actual).expect("置いた生成結果を作れる");
    }
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        std::fs::write(golden, actual).expect("置いた生成結果を更新できる");
    }
    let golden_text = std::fs::read_to_string(golden).expect("置いた生成結果を読める");
    assert_eq!(
        golden_text.trim(),
        actual.trim(),
        "生成結果が置いているものと違う: {}",
        golden.display()
    );
}

#[test]
fn ts_rs_output_matches_the_committed_bindings() {
    // ts-rs は export_to = "bindings/" なので、出力先の bindings/ に生成される。
    let out_dir = std::env::temp_dir().join(format!("pelway-ts-rs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out_dir);
    let config = ts_rs::Config::default().with_out_dir(&out_dir);
    Message::export_all(&config).expect("ts-rs で型を生成できる");

    let golden = golden_dir();
    let mut count = 0;
    for entry in std::fs::read_dir(out_dir.join("bindings")).expect("生成結果の一覧を取れる")
    {
        let entry = entry.expect("生成結果を読み込める");
        let name = entry.file_name().to_string_lossy().into_owned();
        let actual = std::fs::read_to_string(entry.path()).expect("生成した読める");
        assert_same(&golden.join(&name), &actual);
        count += 1;
    }
    assert!(count >= 3, "ts-rs が {count} 個しか生成しなかった");
}

#[test]
fn specta_output_matches_the_committed_bindings() {
    let types = specta::Types::default()
        .register::<Message>()
        .register::<MessageBody>()
        .register::<ChannelId>()
        .register::<MessageId>()
        .register::<NoticeLevel>();
    let actual = specta_typescript::Typescript::default()
        .export(&types, specta_serde::Format)
        .expect("specta で型を生成できる");

    assert_same(&golden_dir().join("specta.ts"), &actual);
}
