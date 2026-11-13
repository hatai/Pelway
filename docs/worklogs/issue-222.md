# Issue #222 の作業ログ

## 実装者メモ（round 1）

1. 詰まった箇所: 前セッションで `tools.zread.*` の呼び出しが 2 回連続で中断し進行が止まった。指揮役の指示どおり `scripts/agent-gh dev api repos/elie222/rakazo/contents/<path>` に替えたら読めた。
2. 曖昧だった文: 「最初の適用例は #217 の pr-digest（並行して進む別作業。そちらの成果物の形と矛盾しないこと）」（Issue #222 本文の「背景・前提」）。「成果物の形」は Issue 本文には無く、PR #241（agent/217-pr-watch-skill）の実 diff で確認するしかなかった。
3. 仮定と解釈: #217 の成果物の形は PR #241 の diff（`.agents/skills/pr-watch/pr-digest`＋`pr-digest.test.sh`、mise.toml の `tasks.script-test` への列挙追加、.gitattributes の `-linguist-generated` 列挙へ pr-watch 追加）と解釈し、規約をこの形に合わせた。PR #241 実装者メモの「glob 収集に替える」案は open 中の #241 と衝突するため採らなかった。規約の置き場所は docs/agents/skill-helpers.md 新設＋AGENTS.md から参照（issue-tracker.md 等と同じ形）とし、AGENTS.md 直書きは掣肘なく候補一覧まで載せられないため採らなかった。
4. 足りなかった情報・検証手段: 並行作業の PR 番号（#241）は Issue 本文に無く指揮役の指示で知った。検証: `scripts/agent-gh dev pr diff 241`、`grep -n '^## ' docs/agents/skill-helpers.md`。
5. 改善案: `.github/ISSUE_TEMPLATE/agent-task.yml` と skill `write-issue` の「本文の構成」に「依存する並行作業があるときは Issue 番号に加えて PR 番号も書く」を足す。

## 実装者メモ（round 2）

1. 詰まった箇所: docs/agents/skill-helpers.md:13。依存の列挙を閉じた形（bash、git、jq、scripts/agent-gh）で書いた結果、適用例の pr-digest（grep・sed・awk・sort・comm・cut・dirname・head・sleep・tail を使用）が規約不適合になった。
2. 曖昧だった文: 「許可する依存の範囲を実例に合わせて明記してください」（PR #248 のレビュー pelway-review の must-fix）。列挙の限界か範囲指定かは指定が無かった。
3. 仮定と解釈: PR #241 の pr-digest の diff から抽出した実コマンド（awk comm cut dirname git grep head jq mkdir printf sed sleep sort tail）を全てカバーする範囲指定（bash＋POSIX 標準コマンド＋git＋jq＋scripts/agent-gh、追加ランタイム・パッケージ不可）を採った。コマンドを全列挙する案は次の同梱でまた崩れるため採らなかった。
4. 足りなかった情報・検証手段: 抽出コマンドは `scripts/agent-gh dev pr diff 241 | grep -oE ...` で確認した。
5. 改善案: skill `write-pull-request` の「手順 4. 本文を書く」に「既存 PR を規約の適用例として挙げるときは、その実装の依存（使っているコマンド等）と規約の記述を突き合わせる」を足す。

