# Issue #807 の作業ログ

## 実装者メモ（round 1）

対象 head: 5f75c49121df02ca346ed9e6a7b50dd355d60b33（この round の実装の実体を含むコミット。`scripts/agent-gh-resign` による署名付け替え後の SHA。このメモを含むコミットの親）

1. 詰まった箇所: Issue #807 の受け入れ条件: verify は commit の前に回す手順なので、「HEAD の木」を commit 済みの HEAD の木と読むと、通常の手順で残る印が次の commit の木と一致せず、正しい push も止まってしまう。
2. 曖昧だった文: 「`mise run verify` が exit 0 で終わったとき、そのときの HEAD の木の hash の印が書き残される → `scripts/tests/` の試験」（Issue #807 の「受け入れ条件」）
3. 仮定と解釈: 「HEAD の木」は push 時に HEAD になる検証済みの内容の木と読み、verify 実行時の作業ツリー（無視するファイルを除く）を一時 index に読んで `write-tree` した hash を印にする。実装者メモだけの差を既存の commit に足す経路は、Issue の指定どおり `docs/worklogs/` の例外で通す。印の書き方は `mise.toml` の `verify` の `run` にし、`agent-gh` が verify を包む案は `mise run verify` の直接実行を捕捉できないため選ばなかった。
4. 足りなかった情報・検証手段: mise の `depends` と `run` の順序は文書で確認せず、依存タスクが失敗したとき `run` に進まないことを小さい合成タスクで確かめた。push の門の試験はローカルの bare remote で行い、GitHub には触れない。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「受け入れ条件の書き方」に「時間軸のある名詞（HEAD・現在・最新）は、どの時点の値かを検査手順に合わせて書く」を足す。

使ったスキル: `.agents/skills/implement-issue`、`.agents/skills/diagnosing-bugs`（verify の失敗の切り分け方針だけ。再現の失敗は無し）
