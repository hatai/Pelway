# Issue #222 の作業ログ

## 実装者メモ（round 1）

1. 詰まった箇所: 前セッションで `tools.zread.*` の呼び出しが 2 回連続で中断し進行が止まった。指揮役の指示どおり `scripts/agent-gh dev api repos/elie222/rakazo/contents/<path>` に替えたら読めた。
2. 曖昧だった文: 「最初の適用例は #217 の pr-digest（並行して進む別作業。そちらの成果物の形と矛盾しないこと）」（Issue #222 本文の「背景・前提」）。「成果物の形」は Issue 本文には無く、PR #241（agent/217-pr-watch-skill）の実 diff で確認するしかなかった。
3. 仮定と解釈: #217 の成果物の形は PR #241 の diff（`.agents/skills/pr-watch/pr-digest`＋`pr-digest.test.sh`、mise.toml の `tasks.script-test` への列挙追加、.gitattributes の `-linguist-generated` 列挙へ pr-watch 追加）と解釈し、規約をこの形に合わせた。PR #241 実装者メモの「glob 収集に替える」案は open 中の #241 と衝突するため採らなかった。規約の置き場所は docs/agents/skill-helpers.md 新設＋AGENTS.md から参照（issue-tracker.md 等と同じ形）とし、AGENTS.md 直書きは掣肘なく候補一覧まで載せられないため採らなかった。
4. 足りなかった情報・検証手段: 並行作業の PR 番号（#241）は Issue 本文に無く指揮役の指示で知った。検証: `scripts/agent-gh dev pr diff 241`、`grep -n '^## ' docs/agents/skill-helpers.md`。
5. 改善案: `.github/ISSUE_TEMPLATE/agent-task.yml` と skill `write-issue` の「本文の構成」に「依存する並行作業があるときは Issue 番号に加えて PR 番号も書く」を足す。
