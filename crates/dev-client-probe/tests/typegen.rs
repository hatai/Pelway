//! Rust の型から TypeScript の型を生成し、置いたもの（`bindings/`）と一致する
//! ことを確認する。差分があればテストが落ちる。A2 は UI を TS/React DOM とする
//! ので、Rust 側の型を変えたのに TS 側の型を忘れると UI 側が古い型でコンパイル
//! される。CI でこの差分を検出できるようにすることがこのテストの目的。
//! （ts-rs で実装している。specta を試した結果も測定報告書に残している。）
//!
//! 置いた生成結果の更新は `UPDATE_GOLDEN=1 cargo test -p pelway-dev-client-probe`
//! で行う（更新したあと、内容を確認してコミットする）。

use std::path::{Path, PathBuf};

use pelway_dev_client_probe::types::Message;
use ts_rs::TS;

/// 置いている生成結果の置き場。
fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("bindings")
}

/// 置いたものと生成したものが一致することを確認し、違えば差分を出して落とす。
/// 置いたものが無ければ作る（UPDATE_GOLDEN=1 のときだけ更新する）。
fn assert_same(golden: &Path, actual: &str) {
    let update = std::env::var("UPDATE_GOLDEN").is_ok();
    assert!(
        update || golden.exists(),
        "置いた生成結果に {} が無い（UPDATE_GOLDEN=1 で生成できる）",
        golden.display()
    );
    if update {
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
fn generated_typescript_matches_the_committed_bindings() {
    // ts-rs: 1 型 1 ファイル。export_to = "bindings/" なので出力先の bindings/ に
    // 生成される。置いたものと違えば落ちる（差分検査）。
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
