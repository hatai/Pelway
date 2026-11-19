# Issue #384 の作業ログ

## 実装者メモ（round 1）

対象 head: 054e13fca9c5fc62751300ea8bc56443ee780eb0

1. 詰まった箇所: crates/pelway-client-ui/tests/i18n_fluent.rs: 変数を含む文言の期待値を生のリテラルで書いたところ、fluent-bundle が変数値の前後に挟む bidi isolate（U+2068/U+2069）の分だけ一致しなかった。テスト側で同じ挙動を組み立てる `iso()` を書いて完全一致に戻した（実装の出力は変えていない）。
1. 詰まった箇所: crates/pelway-client-ui/tests/i18n_switch.rs: テストを 2 関数に分けたら、ヘッドレスの CI で並列実行による backend 初期化の競合で `MainWindow::new()` が winit へフォールバックして落ちた（`No backends configured`）。`slint_window.rs` と同じ「1 バイナリ 1 関数」の構成に 1 関数へまとめて直した。
2. 曖昧だった文: 「未知の言語でも翻訳キーの生文字列を画面に出さない → 初期言語を差し込む試験（OS 値を引数またはテスト用の関数で与える）」（Issue #384 の受け入れ条件）
3. 仮定と解釈: OS の言語文字列の実際の取得は呼び出し側（将来の main）の役で、この Issue では crate に OS 値を引数で受ける `initial_lang` だけを置くと解釈した（依存 crate を増やさない）。sys-locale 等を導入して crate 内で実 OS 値を読む解釈は、起動のエントリポイントがまだ無く OS 依存の試験になるため選ばなかった。
4. 足りなかった情報・検証手段: 「起動後に言語を差し替えて `tr-*` を更新する経路」の関数の形は Issue に規定が無い。`Messages::new` と `apply_translations` を包み、未対応言語では `None`（`tr-*` 不変）を返す `apply_lang(window, lang, vars)` として具体化した。検証: `cargo test -p pelway-client-ui --locked --offline`。
5. 改善案: Issue #384 の「受け入れ条件」の初期言語の条に「OS 値の取得元（呼び出し側が引数で渡す。crate は読まない）」を 1 行足す。

使ったスキル: implement-issue
