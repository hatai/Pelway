//! ui/main.slint をコンパイルして Rust のモジュールを生成する。
//! 生成物は lib.rs の `slint::include_modules!()` で読み込む。
//! Slint はこのクレートの中だけで使い、pelway-client-core には入れない。

fn main() {
    slint_build::compile("ui/main.slint").expect("ui/main.slint のコンパイル");
}
