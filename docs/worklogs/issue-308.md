# Issue #308 の作業ログ

## 実装者メモ（round 1）

1. 詰まった箇所: 環境: この sandbox では `~/.cache` が読み取り専用で mbx（`cargo`  shim 経由）が `Read-only file system` で全滅する。`~/.rustup/toolchains/1.99.1-*/bin` を PATH 先頭にし `CARGO_TARGET_DIR` を worktree 内に置いて cargo 系を実行した。`agent-sandbox.test.sh` は `~/.t3/worktrees` 兄弟ディレクトリの作成ができず失敗（従来のメモどおり CI でのみ確認）。走査（受け入れ条件 3 件目）: 実装者メモ 59 ファイルの「詰まった箇所」「改善案」全量抽出＋巡回記録 3 件の通読で絞り、候補の根拠（issue-96/95、issue-253、issue-217/255 の再掲関係）は rg で裏取りした
2. 曖昧だった文: 「結果は worklog に候補リストとして残す」（Issue #308 本文「採用する方式」）
3. 仮定と解釈: 「worklog」の置き場所は `docs/worklogs/<日付>-curation.md` と解釈した（候補を別の一覧ファイルではなく日付付きの記録として残す形）。走査対象の「直近」は、worklog 群全体が 2026-11-09〜2026-11-15 と 1 週間に収まるため全量とした。runbook への追加は節番号を振り替えず末尾に「## 12.」として置いた（command-team の「runbook 8 の『まとめ』」等の番号参照を壊さないため）
4. 足りなかった情報・検証手段: 「レビュー結果の記録」の独立した置き場が worklog に無い（PR のレビュー本文は GitHub 側）。実装者メモと巡回記録に引用されたレビュー指摘をたどる形で代用した。候補と open Issue の重複確認は `scripts/agent-gh dev issue list` で行った
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「受け入れ条件の書き方」に「worklog のような成果物の置き場は、既存の命名規則（`issue-<N>.md`・`<日付>-<slug>.md`）のどれに従うかを書く」を足す

使ったスキル: implement-issue、write-pull-request（PR 本文は作成時に使用）
