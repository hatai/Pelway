//! ui/probe.slint をコンパイルして Rust のモジュールを生成する。スタイルは
//! 環境変数に左右されないよう fluent に固定する。debug info を有効にするのは、
//! testing backend の要素 API（検索・スクロールのイベント送出）するため。

fn main() {
    let config = slint_build::CompilerConfiguration::new()
        .with_style("fluent".to_owned())
        .with_debug_info(true);
    slint_build::compile_with_config("ui/probe.slint", config)
        .expect("ui/probe.slint のコンパイル");
}
