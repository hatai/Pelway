# Issue #249 の作業ログ

## 棚卸し結果（round 1）

### symlink 方式への準拠（既存方針の確認）

- `.agents/skills/`（59 スキル）と `.claude/skills/` の一覧の突き合わせ（`comm -3`）: 差分なし
- `.claude/skills/<name>` はすべて `../../.agents/skills/<name>` へのシンボリックリンク（実体コピーは 1 つも無い）
- 確認方法: `comm -3 <(ls .agents/skills | sort) <(ls .claude/skills | sort)` が空、`readlink` の確認、変更した 4 スキルの `diff -r .claude/skills/<name> .agents/skills/<name>` が空
- 結論: 全スキルが `.agents` 実体＋`.claude` symlink の方式に準拠している。寄せる対象なし

### 同梱規約（docs/agents/skill-helpers.md）への準拠（既存方針の確認）

- `pr-watch` の `pr-digest`（bash、POSIX 標準コマンド＋git＋jq、GitHub との通信は `scripts/agent-gh dev` 経由）と `pr-digest.test.sh` が規約どおり
- `mise.toml` の `tasks.script-test` に `bash .agents/skills/pr-watch/pr-digest.test.sh` が列挙済み（`mise run verify` で走る）
- `.gitattributes` で `.agents/skills/pr-watch/**` は `-linguist-generated` 列挙済み
- 今回の変更でヘルパーに落とす機械な手順は無し（調査の振り分けは判断が主）。skill-helpers.md の「同梱の候補」の表は候補のまま

### タイトル検出への準拠（既存方針の確認）

- command-team「スレッド名」の「その他」の行（`[<役割>] <対象があれば番号> <要約>`）が調査スレッドのタイトルをカバー。既存の例文に `[調査] GLMの利用上限の解除を確かめる` がある
- `scripts/conductor-watch.sh` の `bad-title` 検出はタイトルの形式で判定するため、調査スレッドも検出対象。変更不要

### 重複記述の棚卸し

| 記述 | 置かれている場所 | 判定 |
|---|---|---|
| 指揮役はコードを書かない・レビューを兼ねない | agent-team.md「レビュー担当の権限」と command-team「決まり」 | 正本＋指揮役が毎回読む決まりの再掲。機能しているため両方残す |
| pr-size の上限（追加 800・削除 2000） | AGENTS.md と implement-issue「止まる条件」 | 正本＋実装担当向けの再掲。残す |
| pr-watch のマージ境界線（マージしない） | pr-watch「境界線」と command-team「マージ」 | pr-watch は skill 参照付き。残す |
| GLM の同時 3 本の枠 | agent-team.md「制約」（上限）、runbook 2（数え方）、command-team「調査の担当と effort」（判断） | 上限・数え方・判断で役割が分かれている。今回、満杯時の優先判断は command-team に一元化した（正本からは参照だけ） |
| workflows 権限の例外（actions.lock の再生成） | agent-team.md「制約」、command-team「マージ」、runbook 1 | 今回、正本（agent-team.md）に例外の列挙を置き、command-team と runbook を正本参照に揃えた。runbook の「しないこと」と command-team の actions.lock push の矛盾を解消 |

### 「未使用の節」の判定基準

- 参照されていない: どの正本・スキル・runbook・AGENTS.md からも参照されず、スキルの description（トリガー）にも現れない節
- 他の正本と重複している: 同じ規則が 2 か所以上にあり、更新の正本が明示されていないため片方だけが更新される恐れがある節
- 前提が変わっている: 節が書かれたときの前提（存在した Issue・ツール・運用）が解消または変更されていて、指示が現状に合わない節

適用結果: 今回確認した範囲（agent-team.md、command-team、investigate-bug、pr-watch、implement-issue、review-pr、write-issue、write-pull-request、grokbot-runbook.md、skill-helpers.md、AGENTS.md）で、削除に値する「未使用の節」は無し。skill-helpers.md の「同梱の候補」の表は候補一覧と明記されており維持する。

### 見つかった不揃い（改善案として記録）

- `.gitattributes` の `-linguist-generated` 列挙（このリポジトリで書いているスキルの行数を pr-size に数える仕組み）が不揃い: 列挙済みは implement-issue、review-pr、write-pull-request、pr-watch。未列挙は command-team、write-issue、investigate-bug（今回の変更分は pr-size に数えられない）
- 改善案: `.gitattributes` に `.agents/skills/command-team/**`、`.agents/skills/write-issue/**`、`.agents/skills/investigate-bug/**` を `-linguist-generated` で足す（Issue #249 は文書変更のため対象外と判断し、別 Issue で行う）

## 実装者メモ（round 1）

1. 詰まった箇所: Issue 本文「背景・前提」: ワークフロー素案そのものへのリンク（PR・ドキュメント・添付いずれも）が Issue 本文・コメントに無く、リポジトリ内を「入口」等で検索しても見つからなかった。受け入れ条件の列挙を素案の内容と解釈して反映内容を組み立てた
2. 曖昧だった文: 「ワークフロー素案は Codex レビューを2回受け、must-fix が解消された版が確定している。本 Issue はその内容の正本化であり、新しい方針の検討は含まない。」（Issue #249 本文「背景・前提」）
3. 仮定と解釈: (a) 素案の内容は受け入れ条件の列挙と同じと解釈した（選ばなかった解釈: 素案を探して止まる。受け入れ条件が具体的で判定可能なため）。(b) 調査報告を GitHub 上で読めない指揮役（Grok Bot）が確認できるよう、調査スレッドが報告ファイルだけの Draft PR（ブランチ `agent/investigate-<slug>`）を出す運用を investigate-bug・command-team・runbook に書いた。Issue には報告の PR についての指定が無い（選ばなかった解釈: 起票後の実装 PR に含める。PR の無い期間に指揮役が GitHub 上で報告を確認できない）。(c) Dependabot の major/0.x の PR のクローズは人間の操作と書いた（runbook で指揮役は PR をクローズしないため）。(d) 入口C の検出元は正本に機能名（nightly 検証、eval ケース）で書き、Issue 番号（#180、#220）はこの作業ログに記録した（Facts over history）
4. 足りなかった情報・検証手段: 調査報告の Draft PR のブランチ名の規則（`agent/investigate-<slug>` は本 PR で新たに決めた）。`agent/` 始まりのため pelway-agent-watch の `issue_number` は null になり、runbook の調査スレッドの行で `agent/investigate-` との区別を明示した
5. 改善案: `.gitattributes` のコメント「このリポジトリで書いている skill…」の下に、上の「見つかった不揃い」の 3 スキルを `-linguist-generated` 列挙として足す

## 実装者メモ（round 2）

1. 詰まった箇所: `.agents/skills/investigate-bug/SKILL.md` の手順 5: must-fix が名指しする箇所のほか、同じ規則（報告の Draft PR と `agent/investigate-<slug>` ブランチ）が command-team・review-pr・grokbot-runbook にも書かれていた（round 1 の仮定 (b)）。1 か所だけ戻すと正本に実在しない PR への参照が残るため、規則ごと 4 ファイルから除去し、review-pr は報告 PR のレビュー基準の段落ごと戻した（結果として review-pr の差分は 0 になった）。棚卸しメモのスキル数は `ls .agents/skills | wc -l` の 59 に訂正した
2. 曖昧だった文: 「この Draft PR とブランチ規則を除き、承認済み要件に含まれる報告・引き渡しだけを反映してください。」（PR #250 レビュー本文 must-fix 1）
3. 仮定と解釈: (a)「規則を除く」の範囲は、Draft PR とブランチの規則とその周辺（Assigned 行の報告 PR への写し、runbook のイベント処理にある `pr:` とブランチ接頭辞での識別）まで含むと解釈した（選ばなかった解釈: investigate-bug の該当文のみ戻す。ほかの正本に整合しない言及が残るため）。(b) runbook の調査スレッドの識別は、ブランチ接頭辞に代えて既存のスレッド名形式のタイトル `[調査]` で書いた（選ばなかった解釈: 識別の文を消す。`finished` の報告を「調査の引き渡し」へ進める手がかりがなくなるため）
4. 足りなかった情報・検証手段: 報告ファイル `docs/worklogs/bugs/<slug>/report.md` を main へ入れる経路（Draft PR を除いた結果、Issue #249 に指定が無いまま残った。報告はスレッドの返答と worktree 内のファイルで行われる）
5. 改善案: `docs/superpowers/specs/agent-team.md` の「入口B（バグ・インシデント）の調査と起票」に「報告ファイルを main に入れる経路は未決（人間が決める）」を足す（新規の方式になるため別 Issue で人間と決める）
