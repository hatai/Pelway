# クライアント画面からの操作

状態: 失敗（起動口なし、#560 未マージ）。利用者の道筋（窓を開いてサーバーにつなぐ操作）は動かせない。テスト用 backend の範囲だけ確かめられる。

## Sub-features

- チャンネルの表示と投稿（作成欄・タイムライン）
- 判断待ちカードでの A/B の選択
- メール下書きの確認と送信の承認: 失敗（機能なし）。画面にも無い（`mail-draft.md` と同じ扱い）

## How to get to it (user POV)

利用者はデスクトップのクライアントで #案件B を開き、ハルにメンションし、判断待ちカードで案を選ぶ。

## Driving it with verify-run

`verify-run` からは呼ばない。run の合否にも入れない。テスト用 backend（Slint の testing backend。窓は開かない）の範囲で、既存のテストを動かす:

```bash
mise exec -- cargo test -p pelway-client-ui --locked --offline
```

- 要素の見つけ方: `ElementHandle::find_by_accessible_label`・`ElementHandle::find_by_element_type_name` で探し、`accessible_role()`（`AccessibleRole::Button`・`ListItem` など）で絞る。例は `crates/pelway-client-ui/tests/accessibility.rs`（ロケール `ja` を決めて `MainWindow` を作り、ラベルと役割の組で数える）。
- 結果は 2 つに分けて書く: 「テスト用 backend の範囲だけ合格（`cargo test` の `test result` の行）」と「利用者の道筋は失敗（起動口なし、#560 未マージ）」。

## Gotchas

- `crates/pelway-client-ui` はライブラリだけで、起動できるバイナリが無い（`cargo run` できる `main` が無い）。クライアントの起動口（`--smoke` など）を足すのは #568 の範囲外。
- サーバーにつなぐ部分（`live.rs` と `net.rs`）は PR #560 の中にあり、main にはまだ入っていない。main の `crates/pelway-client-ui/src/bridge.rs` は通信を書かない（`Effect` は保持するだけで未接続）。画面の操作はバックエンドに届かない。
- `ui/main.slint` の `accessible-label` は、ほとんどが i18n の文字列（`tr-*`、`root.a11y-label`）で、日本語・英語の切り替えで変わる。手がかりにするのは、ロケールを決めた既存のテストか、要素の型名（`RailAiRow` など）と `accessible-role` の組。ラベルの文字列を手順にじかに書かない。
- テスト用 backend で通っても、利用者の道筋の代わりにはならない。
