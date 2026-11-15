# Issue #308 の作業ログ

## 実装者メモ（round 1）

対象 head: fdf5ecd5574d3540052b8b5087a99d5224a8bb25（この round の実装の実体を含むコミット。このメモを含むコミットの親）

1. 詰まった箇所: 環境: この sandbox では `~/.cache` が読み取り専用で mbx（`cargo`  shim 経由）が `Read-only file system` で全滅する。`~/.rustup/toolchains/1.99.1-*/bin` を PATH 先頭にし `CARGO_TARGET_DIR` を worktree 内に置いて cargo 系を実行した。`agent-sandbox.test.sh` は `~/.t3/worktrees` 兄弟ディレクトリの作成ができず失敗（従来のメモどおり CI でのみ確認）。また exec の引数に日本語を渡すと `\uXXXX` エスケープのまま実行環境に届く障害があり、PR タイトルがエスケープ文字列のまま作成された（PR #311 と同じ障害）。`api PATCH --input <JSON ファイル>` で書き直して直した。日本語のタイトル・コメント・本文はすべてファイル経由（`--body-file`・`--input`）で渡す必要がある。走査（受け入れ条件 3 件目）: 実装者メモ 59 ファイルの「詰まった箇所」「改善案」全量抽出＋巡回記録 3 件の通読で絞り、候補の根拠（issue-96/95、issue-253、issue-217/255 の再掲関係）は rg で裏取りした
2. 曖昧だった文: 「結果は worklog に候補リストとして残す」（Issue #308 本文「採用する方式」）
3. 仮定と解釈: 「worklog」の置き場所は `docs/worklogs/<日付>-curation.md` と解釈した（候補を別の一覧ファイルではなく日付付きの記録として残す形）。走査対象の「直近」は、worklog 群全体が 2026-11-09〜2026-11-15 と 1 週間に収まるため全量とした。runbook への追加は節番号を振り替えず末尾に「## 12.」として置いた（command-team の「runbook 8 の『まとめ』」等の番号参照を壊さないため）
4. 足りなかった情報・検証手段: 「レビュー結果の記録」の独立した置き場が worklog に無い（PR のレビュー本文は GitHub 側）。実装者メモと巡回記録に引用されたレビュー指摘をたどる形で代用した。候補と open Issue の重複確認は `scripts/agent-gh dev issue list` で行った
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「受け入れ条件の書き方」に「worklog のような成果物の置き場は、既存の命名規則（`issue-<N>.md`・`<日付>-<slug>.md`）のどれに従うかを書く」を足す

使ったスキル: implement-issue、write-pull-request（PR 本文は作成時に使用）

## 実装者メモ（round 2）

対象 head: df7fb5c2c42377afbbb1826c2c9ea338d2ef043d（このメモを含むコミットの親。実装の実体は round 1 の fdf5ecd5574d3540052b8b5087a99d5224a8bb25 で、round 2 は再レビューのみ）

1. 詰まった箇所: round 1 の Approve（fdf5ecd）後に「対象 head」行の追記を含むメモのみのコミット（df7fb5c）を push し、command-team の「Approve 後に push があった PR はレビューをやり直す」規則により再レビューが必要になった（指揮役からの指摘）
2. 曖昧だった文: なし（指揮役の指示どおりに再レビューへ進んだ）
3. 仮定と解釈: round 2 は再レビューのみでコード・ドキュメントの変更はないが、レビュー round の番号として数え、メモも round 2 の節として書き足した
4. 足りなかった情報・検証手段: なし
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 8（レビュー委託）または「対象 head」の書式の注記に、「対象 head の記入を含むメモのコミットはレビュー委託の前に行う。Approve 後の push（メモのみでも）はレビューのやり直しになる」を明記する（issue-290 の改善案の未反映分とも関係する）
