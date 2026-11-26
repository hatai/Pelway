# Issue #807 の作業ログ

## 実装者メモ（round 1）

対象 head: 5f75c49121df02ca346ed9e6a7b50dd355d60b33（この round の実装の実体を含むコミット。`scripts/agent-gh-resign` による署名付け替え後の SHA。このメモを含むコミットの親）

1. 詰まった箇所: Issue #807 の受け入れ条件: verify は commit の前に回す手順なので、「HEAD の木」を commit 済みの HEAD の木と読むと、通常の手順で残る印が次の commit の木と一致せず、正しい push も止まってしまう。
2. 曖昧だった文: 「`mise run verify` が exit 0 で終わったとき、そのときの HEAD の木の hash の印が書き残される → `scripts/tests/` の試験」（Issue #807 の「受け入れ条件」）
3. 仮定と解釈: 「HEAD の木」は push 時に HEAD になる検証済みの内容の木と読み、verify 実行時の作業ツリー（無視するファイルを除く）を一時 index に読んで `write-tree` した hash を印にする。実装者メモだけの差を既存の commit に足す経路は、Issue の指定どおり `docs/worklogs/` の例外で通す。印の書き方は `mise.toml` の `verify` の `run` にし、`agent-gh` が verify を包む案は `mise run verify` の直接実行を捕捉できないため選ばなかった。
4. 足りなかった情報・検証手段: mise の `depends` と `run` の順序は文書で確認せず、依存タスクが失敗したとき `run` に進まないことを小さい合成タスクで確かめた。push の門の試験はローカルの bare remote で行い、GitHub には触れない。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「受け入れ条件の書き方」に「時間軸のある名詞（HEAD・現在・最新）は、どの時点の値かを検査手順に合わせて書く」を足す。

使ったスキル: `.agents/skills/implement-issue`、`.agents/skills/diagnosing-bugs`（verify の失敗の切り分け方針だけ。再現の失敗は無し）

## 実装者メモ（round 2）

対象 head: 774c1b29459f146e83f23ae61ae0176156c9fbda（この round の実装の実体を含むコミット。main の merge を含むため `agent-gh-resign` は署名付け替えを行わず、push 後も同じ SHA。このメモを含むコミットの親）

1. 詰まった箇所: round 1 の CI は main 由来の `agent-team-config.test.sh` の失敗で verify が落ち、門そのものの結果まで読めなかった。レビューの指摘どおり main（ac5c9fc）を取り込んでからやり直す。
2. 曖昧だった文: 「直し方: main の `scripts/agent-gh-resign:98` と同じく `git --no-replace-objects diff-tree -r --no-renames --name-only -z "$marker" "$tree"` にする（少なくとも `--no-renames` を付ける）。」（PR #832、claude_review の round 1）
3. 仮定と解釈: `diff-tree` に 2 つの木を渡す形でも出力はパスだけなので、`--name-only` の出力形は変えずに `--no-renames` へ寄せる。`[文書のずれ]` の nit はこの PR が作った手順の不整合なので、同じ round で `implement-issue` skill に skip の扱いを足す。
4. 足りなかった情報・検証手段: rename 検出で外の削除が隠れることは、同一内容のファイルを `git mv` で `docs/worklogs/` の下へ移したコミットをローカルの bare remote に push する試験で確かめる。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「受け入れ条件の書き方」に「検査器の差分表示を使う条件では、rename・copies・mode だけの変更を試験に入れる」を足す。

使ったスキル: `.agents/skills/implement-issue`、`.agents/skills/diagnosing-bugs`（CI 失敗の切り分け方針だけ）

## 実装者メモ（round 3）

1. 詰まった箇所: round 2 の Approve に残った `[文書のずれ]` は、調査の報告 PR が `docs/worklogs/` だけの差でも、印の無い worktree では新しい門に止められるという不整合だった。
2. 曖昧だった文: 「investigate-bug の報告 PR を出す段にどちらで push するかを足し、dev-environment.md:88 に「印が無ければ docs/worklogs/ の下だけの差でも止まる」を足す。」（PR #832、claude_review の round 2）
3. 仮定と解釈: 調査の報告 PR も実装と同じく、原則は `mise run verify` を成功させて印を残してから push する。調査と無関係の失敗が変わらないときだけ `PELWAY_AGENT_SKIP_VERIFY_GATE=1` を使い、PR 本文に残す。
4. 足りなかった情報・検証手段: 挙動は round 1・2 の門の試験で確かめてあるため、この round は文書の手順をそろえるだけにし、新しい試験は足さない。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「受け入れ条件の書き方」に「既存の作業経路を止める門では、例外を使う側の手順も受け入れ条件に入れる」を足す。

使ったスキル: `.agents/skills/implement-issue`
