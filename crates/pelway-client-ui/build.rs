//! ui/main.slint をコンパイルして Rust のモジュールを生成する。
//! 生成物は lib.rs の `slint::include_modules!()` で読み込む。
//! Slint はこのクレートの中だけで使い、pelway-client-core には入れない。
//!
//! debug info 付きでコンパイルする（Issue 237）: i-slint-backend-testing の
//! ElementHandle（tests/slint_attribution.rs がメニュー操作に使う）が、
//! 生成コードに debug info（要素の型名・id）を要求するため。

fn main() {
    let config = slint_build::CompilerConfiguration::new().with_debug_info(true);
    slint_build::compile_with_config("ui/main.slint", config).expect("ui/main.slint のコンパイル");
}
