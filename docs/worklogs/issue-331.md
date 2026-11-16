# Issue #331 の作業ログ

## 実装者メモ（round 1）

対象 head: ecab2c3b8d66b9401301ffd086487b34cb2855a0（PR-1 実装コミット、署名付け替え後）

1. 詰まった箇所: 手順 6（pr-size）: 単一 PR だと追加 +1442 が上限 800 を超え、Issue コメントで分割案を提示し `needs-human` で止まった。指揮役が split-plan（3 PR 積み上げ）を承認したため、PR-1 = client-core のみ（+313）で再開。ほか、Slint で `if` 直下の `for` が書けず三項で回避、i-slint-backend-testing が複数 `MainWindow` で落ちるため窓検査を 1 テストに集約（後続 PR の話）。
2. 曖昧だった文: 「読み取り側は既存の inbox.rs の定義（自分の判断待ち／自分の依頼の2区分）が正本。main.slint の TodayView モック（3区分）とは食い違うので、コードの定義に合わせる。」（Issue #331 本文「背景・前提」）— TodayView 全体を 2 区分に作り替えるか、判断待ちだけを 2 区分に見せるか。
3. 仮定と解釈: `TodaySectionKind` の 3 区分を消して正本の 2 区分に作り替える解釈を採用（3 区分を残すと「コードの定義に合わせる」と矛盾）。PR-1 では `DecisionBoard` を UI 非依存の台帳窓口とし、HTTP や永続化は持たせず「将来送る `DecideRequest` を組み立てる」責務に限定した。
4. 足りなかった情報・検証手段: `mise run verify` がこの sandbox では直接動かず（cargo shim が mbx 経由で `~/.cache/mbx` read-only、`/etc/resolv.conf` が `/mnt/wsl/resolv.conf` へ dangling で rustup sync が失敗）、rustup toolchain 直指定 + `MBX_CACHE_DIR=/tmp/mbx-cache` + `MISE_DISABLE_TOOLS=rust,node` + deny-check のみ DNS 修正済み userns の分割実行で同等を確認。
5. 改善案: `docs/dev-environment.md` の「ローカル検証」に「pelway-sandbox では `MISE_DISABLE_TOOLS=rust` + rustup toolchain 直指定 + `MBX_CACHE_DIR` で verify を分割実行する」旨を足す。

使ったスキル: implement-issue。TDD（seam: `pelway-client-core::decision::DecisionBoard`）。
