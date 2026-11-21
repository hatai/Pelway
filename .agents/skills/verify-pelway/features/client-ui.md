# クライアント画面からの操作

状態: 失敗（起動できるクライアントが無く、画面と HTTP の通信が未結線）。

## Sub-features

- チャンネルの表示と投稿（作成欄・タイムライン）
- 判断待ちカードでの A/B の選択
- メール下書きの確認と送信の承認

## How to get to it (user POV)

利用者はデスクトップのクライアントで #案件B を開き、ハルにメンションし、判断待ちカードで案を選ぶ。

## Driving it with verify-run

動かせない。`verify-run` は HTTP の道筋だけを動かす。

## Gotchas

- `crates/pelway-client-ui` はライブラリだけで、起動できるバイナリが無い（`cargo run` できる `main` が無い）。
- `crates/pelway-client-ui/src/bridge.rs` は通信を書かない（`Effect` は保持するだけで未接続）。画面の操作はバックエンドに届かない。
- 画面の振る舞いはテスト用の backend（`i-slint-backend-testing`）で `cargo test -p pelway-client-ui` が確かめるが、これは利用者の道筋の代わりにならない。
