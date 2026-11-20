# Issue #564 の作業ログ

## 実装者メモ（round 1）

対象 head: 168adf884976dda71a4dc071beb38747749e3de7（hatai の依頼で足した mise のタスク `conductor-register`・`conductor-rebuild` を含む）

1. 詰まった箇所: `scripts/tests/conductor-rebuild.test.sh`: 試験で HOME を一時ディレクトリに向けると、PATH の `python3`（mise の shim）が mise の設定を読めずに失敗した。`scripts/conductor-rebuild` では、`pitchfork-check` と同じく `/usr/bin/python3` を使った。
2. 曖昧だった文: 「`--provider`・`--model`・`--effort` を省略したとき、旧指揮役の `t3 thread show --json` の provider・model・effort が新しい指揮役に渡る」（Issue #564 の受け入れ条件）— `t3 thread show --json` には effort が無かった（`providerInstanceId`・`model` だけ）。そこで、effort は `--effort` を指定したときだけ渡す形にした（PR 本文に書く）。
3. 仮定と解釈:
   - `scripts/conductor-register` を使うので、#555 の PR #561 のブランチに積んだ。
   - 新しい指揮役の題名は「[指揮役] 恒久交代で引き継ぐ」にした（command-team の「スレッド名」の `[役割] 要約` の形）。
   - `t3` は、agent-watch と同じく `$HOME/.t3/wsl-runtime/` の下のものだけを受け入れる。
4. 足りなかった情報・検証手段: 本物の T3 で、旧指揮役が停止の文で止まり、新指揮役が settle まで進むかは、実行していない（試験は偽の `t3`）。
5. 改善案: `t3 thread show --json` に model の options（effort）が出るようになれば、省略時に旧指揮役の effort も引き継ぐ（このスクリプトの `--effort` の扱いを直す）。

使ったスキル: implement-issue、write-pull-request
