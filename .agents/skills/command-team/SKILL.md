---
name: command-team
description: "Use when acting as the conductor (指揮役) of the AI dev team: assigning Issues to implementer threads, routing reviews, choosing model and effort, escalating, merging, running the improvement loop, or starting/ending a substitute conductor session. Keywords: 指揮役, conductor, Grok Bot, 代行, review routing, effort, 改善ループ, 実装者メモ"
---

# Command Team

AI 開発チームの指揮役が、毎サイクル実行する判断の手順。

- 体制・役割・同時数の上限の正本は `docs/superpowers/specs/agent-team.md`
- イベントごとの対応表は `docs/agents/grokbot-runbook.md`
- このスキルは、その 2 つの間で指揮役が下す判断（振り分け、エスカレーション、改善ループ、代行）を決める

指揮役の正本は Grok Bot。Grok Bot が動かないときは、runbook の「代行」の手順で別のエージェントが代行する。このスキルは代行者にもそのまま当てはまる。

特定の条件でだけ読む詳細（週次の棚卸し、外部からのレビュー投入、入口B の調査、hatai のレビュー依頼と伝達、Dependabot、代行のチェック、レビュー担当の段階移行）は `docs/agents/command-team-details.md` に、コンテキスト圧縮と恒久交代の手順は `docs/agents/conductor-context.md` にある。各節に読む条件を書く。

## 決まり（毎回守る）

- 指揮役はコードを書かない。PR のレビューもしない（自分が関わった PR を自分で評価しない）。
- 指揮役が自分で作った PR（運用文書やスキルの変更）も、レビュー担当のスレッドに回す。
- スレッドに送る指示は、Issue 番号か PR 番号と skill 名だけにする。例外はラベル（例: `pr-size-exception`）と Issue のコメントで伝え、指示の文に書き足さない。ただし、スレッドを立てる指示（と、effort やモデルを変えた後の最初の修正の指示）には、2 行目に割り当ての行を必ず入れる。

  ```
  Issue #<N> を実装する。skill implement-issue に従う。
  Assigned: role=implement agent=<opencode|devin> model=<モデル名> effort=<effort>
  ```

  実装の値は、opencode なら `model=zai-coding-plan/glm-5.3 effort=<low|default|high|max>`、devin なら `model=swe-2-high effort=<medium|high|max>`（effort の値は `thought_level` に写す）にする。

  スキル改訂の Issue（skill `improve-skills` の入口Aで起票されたもの）を割り当てるときは、1 行目を「Issue #<N> を実装する。skill implement-issue と skill improve-skills に従う。」にする。実装担当は implement-issue の手順に加えて、improve-skills の入口B以降の手順を読む（入口Bの指定は improve-skills の本文が担う）。

  ```
  PR #<M> をレビューする。skill review-pr に従う。
  Assigned: role=review agent=<codex|claude|devin> model=<モデル名> effort=<effort> security=yes run=<委託記録のコメント ID>
  ```

  `run=` には、スレッドを立てる前に残した委託記録のコメント ID を書く（記録の形と ID の読み方、投稿との結び付けは skill `implement-issue` の手順 8 の「委託記録」と「実行の投稿」の段落）。`security=yes` は、差分が skill `implement-issue` の手順 8 で `security=yes` を付ける条件に当たるときだけ付け、当たらなければ書かない。付いているとレビュー担当は skill `security-review` も行う（skill `review-pr` の手順 2）。指揮役自身の PR は運用文書・スキルの変更が多く、たいてい当たる。

  このレビュー指示を使うのは、指揮役が自らレビュースレッドを立てるとき（指揮役自身の PR・外部投入）だけ。通常の実装担当の PR には使わない（実装担当が `delegate_task` で自分で委託する。skill `implement-issue` の手順 8）。調査スレッドを立てる指示の形は `docs/agents/command-team-details.md` の「調査（入口B）」。

  実装担当はこの値を PR 本文の `Implementer:` の行に、レビュー担当はレビュー本文の `Reviewer:` の行に、調査担当と棚卸し担当は報告の PR 本文の `Implementer:` の行に写す。
- 巡回で介入しないのは、人間が手で立てたスレッドと、別の指揮役のセッションが担当中のスレッド・PR だけ（割り当ての記録の表で担当を確かめる）。人間から依頼された代行・引き継ぎは例外で、手順 5 の「代行の開始と終了」に従う。止まっているチームの実装スレッドとレビュースレッドには、指揮役が次の指示を出す（runbook の「毎回の巡回でスレッドを点検する」）。
- スレッドを立てるときは、必ず「スレッド名」の形式のタイトルを渡す。タイトルが形式に合わないまま作業を始めさせない。
- 割り当てのたびに、指揮役の作業ログの「割り当ての記録」の表に 1 行足す（日時、Issue・PR、役割、担当、モデル、effort、理由）。改善ループで、レビュー結果・実装者メモと対照する。
- スレッドへの指示は、新しい実行として送る（MCP の `t3_thread_send` で `mode: queue`）。実行中のスレッドに `steer`（既定の `auto`）で送ると、実行の途中に差し込まれ、その実行が指示を扱わずに終わることがある。終わりの報告と PR の head で、指示が扱われたかを確かめる。
- 判断はすべて Issue か PR にコメントで残す。セッションの文脈だけに置かない。
- 指揮役が Issue を立てる・分割する・差し戻された本文を直すときは、skill `write-issue` に従う。`ready-for-agent` を付ける前（人間に委任されたとき）は、`write-issue` の手順 6 のゲートを通す。
- sandbox に止められた操作を、sandbox の外へ出る抜け道（`systemd-run`・`nohup`・別のシェル・ラッパースクリプトなど）で実行しない。止められたら止まって報告する。作業ログに残る同種の前例（`systemd-run` 経由の patrol-state 更新・ボードキュー手動適用など）は許可にならない（Issue #477）。
- 見張りの状態は `scripts/conductor-watch.sh` が `~/.local/state/pelway/` に置く。セッションの一時フォルダに置かない。巡回状態の補助記録 `patrol-state.json` だけは、sandbox 内の指揮役からも読み書きできるよう、リポジトリの共有 git dir（`git rev-parse --git-common-dir`）内の `pelway/` に置く（git の管理対象外。Issue #477）。

## スレッド名

指揮役が T3 で立てるスレッド（実装、レビュー、調査、修正、試験のすべて）のタイトルは、役割・対象の番号・日本語の要約の 3 つを含む次の形にする。要約は全角 30 字程度までで、一覧を見て何の作業か分かる内容にする。自動生成のタイトル、スレッド ID、ブランチ名だけのタイトルにしない。

| 役割 | 形式 | 例 |
|---|---|---|
| 実装 | `[実装] #<Issue 番号> <要約>` | `[実装] #101 pgBackRestで14日上限と復元を確かめる` |
| レビュー | `[レビュー] PR #<PR 番号> <要約>` | `[レビュー] PR #132 本体LLMの提供元とBYOK（ADR 0008）` |
| その他 | `[<役割>] <対象があれば番号> <要約>` | `[調査] GLMの利用上限の解除を確かめる`、`[修正] #95 ツールチェーンの更新` |

- MCP の `t3_thread_launch` では `title` に、CLI の `t3 thread start` では `--title` に渡す。`t3 thread start` の `--title` を省略すると最初の指示の 1 行目がタイトルになるので、省略しない（`t3_thread_launch` の `title` は必須項目で、省略は `-32602` になる）。`t3_thread_launch`・`t3_thread_send` の引数の JSON 例は `docs/agents/grokbot-runbook.md` の「4. 仕事の始め方」の「MCP の `t3_thread_launch` と `t3_thread_send` の引数」に従う。
- 立てた後で形式に合わないと分かったら、そのスレッドへの次の操作の前に `t3_thread_update`（`action: rename`）で直す。形式に合わない古いスレッドも、次に操作するときに直す。
- `scripts/conductor-watch.sh` は、指揮役が立てたスレッドのうち、動いているものと新しく現れたもので形式に合わないタイトルを `THREAD bad-title` として知らせる。

## 人間あての報告・連絡のリンク

指揮役が人間（hatai）あてに送るメッセージで Issue や PR の番号に言及するときは、必ず Markdown リンクとして表示させる。素の `#<番号>` 表記は GitHub 上では自動リンクになるが、表示先が GitHub とは限らない（T3 のスレッド画面など）ため、GitHub の自動リンクに頼らない。

- 対象は、指揮役が人間あてに出すすべてのメッセージ（マージ報告、巡回・ダイジェスト報告、エスカレーション（needs-human の連絡）、引き継ぎ報告、人間に判断を求める質問の列挙など）
- 形式は Markdown リンクで完全な URL を書く:
  - Issue: `[#<番号>](https://github.com/hatai/Pelway/issues/<番号>)`
  - PR: `[PR #<番号>](https://github.com/hatai/Pelway/pull/<番号>)`
- 同じ番号が 1 つのメッセージに複数回出る場合も、その都度リンクにする（「初出だけリンク」のような例外は作らない）
- 番号のリンクは URL を付ける決まり（部屋で人間に知らせる、まとめの例文）にも同じ形で適用する。報告の例文は runbook 8 の「まとめ」

## 手当て・引き継ぎの前の確認

PR や Issue に手を付けるときは、最初に対象 Issue の受け入れ条件と到達点の状態（「Draft PR の存在をもって完了とする」「マージまで行う」など、skill `write-issue` の受け入れ条件の書き方）を確認する。適用範囲は次の 4 つに限る。

- (a) PR の Draft 解除と Draft への復帰
- (b) PR への main 取り込み（update-branch）と base の変更
- (c) Issue のクローズと再オープン
- (d) 受け入れ条件に触れる Issue 本文の変更

操作が受け入れ条件や到達点の状態と矛盾する場合（例: 到達点が「Draft PR の存在」なのに Draft を解除する）は実行せず、指揮役（代行の場合は hatai）へ報告して止まる。マージとレビュー投稿はこの確認の対象外（マージは「2. 振り分け」の「マージ」の決まりに、レビューの投稿は「2. 振り分け」の担当の決まりに従う）。

## 1. サイクルの始め

1. `scripts/conductor-watch-ensure.sh` を実行する（冪等の起動入口。動いていなければ起き、動いていれば何もしない）
2. `t3 thread list --json` と `scripts/agent-gh dev pr list` で、settle されていないチームのスレッド全部（実行中のものと、終わって止まっているもの）、Draft の PR、レビュー待ちの PR を数え、runbook の「毎回の巡回でスレッドを点検する」で分類して、settle・指示・`automerge` を行う。巡回の間隔は runbook の「指揮役の定期巡回（スケジュール起床）」、報告の形は同じく「巡回の報告の形」。PR の中身（チェック・レビュー・コメントの状態）を見るときは、生の `gh api` を読む代わりに skill `pr-watch` の `pr-digest` を使う
3. 実行中のスレッドのタイトルが「スレッド名」の形式に合っているかを見て、合わないものを直す
4. runbook の「同時数と順番待ち」の順で、次の一手を 1 つずつ決める
5. 実装スレッドがレビュー委託の完了待ちの間も、指揮役は何も割り当てない（修正とレビューの繰り返しは実装スレッドの内側で進む）。実装スレッドの終わりの報告（`pr:` と `ready: done`）を runbook の「報告行の扱い」で確かめられたら、マージの判断（下の「マージ」）に進む
6. レビュー待ちの PR は、`pr-digest` の `review-wait` 行で滞留を見る（下の「レビュー待ちの滞留」）

週次の棚卸し（`docs/agents/command-team-details.md` の「週次: 長命 PR の棚卸し」）はこのサイクルでは行わず、週 1 回、指揮役とは別の「棚卸し担当」スレッドが行う。前回の棚卸し（棚卸し担当の前回の起動。割り当ての記録と前回の報告 PR で確かめる）から 7 日以上経ち、かつ open の棚卸し Issue（巡回記録に起票を記録した番号のもの。名義・題名では識別しない）が残っていない巡回では、指揮役が runbook「4. 仕事の始め方」の「棚卸し担当を立てる」で棚卸し担当を立てる（自分では棚卸ししない）。open の棚卸し Issue があって起動が済んでいないときは、新たに起票せず、同じ Issue の起動の確認・やり直しを同項の手順で行う（新規の起票と起動のやり直しは別の条件で、起動済みかどうかは起動の記録で分ける。`needs-human`・「人間の判断待ち」の保留中は起動しない）。棚卸し担当は提案（報告 PR の判断案と期限）までで、ラベル・マージ・スレッドへの指示はしない。提案の受け取り（巡回記録への転記・部屋での人間への連絡・期限切れのエスカレーション）は指揮役が巡回で行う。期限切れの検出は、転記した「判断の記録」の期限を毎回の巡回で GitHub の実状態と照合して行い、棚卸し担当の報告を待たない（回した判断は繰り返し通知しない。runbook「2. ボードと巡回」の巡回の処理順）。作業ログの定期キュレーションも棚卸し担当が行う（「4. 改善ループ」の「定期キュレーション」）。

巡回記録とは、指揮役の作業ログ（`docs/worklogs/`）の「割り当ての記録」と「判断の記録」のこと。GitHub に置き場の無い判断（誰を割り当てたか、PR を待たせる理由と期限）を書く場所である。

## 2. 振り分け

材料は既にあるものだけを使う: pr-size の行数（追加と削除）、変更パス、ラベル、修正の回数（`round`）。下の値は現在の値で、改善ループ（手順 4）で調整する。値を変えたら、この表を直す。

### 割り当ての可否（blocker の確認）

Issue を割り当てる直前に、runbook の「割り当て・停止・再開」の blocker の確認を通す。割り当てを止める条件、Issue に残すコメント、順番を変えるとき、割り当てた後に blocker が開き直したとき、見張りに確認を組み込むときの決まりも、同じ節が正本。

### 実装のモデルと effort

| 条件 | モデル・effort |
|---|---|
| 定型・小規模（Issue の規模の見積もりが小、変更が文書・設定・テストだけ） | opencode `zai-coding-plan/glm-5.3`、variant `low` |
| 通常（仕様が固まった実装） | GLM と Devin CLI の空いている方。同時に空いていれば Devin CLI を優先する（`acpRegistry_devin`、`swe-2-high`、`thought_level=high`）。GLM なら variant 指定なし |
| 曖昧で長い調査・デバッグ系の Issue | opencode `zai-coding-plan/glm-5.3` |
| 「Devin CLI のハーネスごと任せる」チケット（人間が Issue で明示するか、指揮役が「実装スレッドの枠組みでは進めにくい自律作業」と判断したもの。例: 環境構築を伴う検証、複数リポジトリにまたがる調査） | Devin CLI（`acpRegistry_devin`、`swe-2-high`）。Issue に `agent:devin` を付ける |
| コア部分（`crates/pelway-contract`、承認・状態遷移・権限）、`unsafe` を含む・含みうる変更 | GLM なら variant `high`、Devin CLI なら `thought_level=max` |
| 同じ Issue の修正の指示が 2 回を超えた（3 回目の修正に入る）。数えるのは、レビューの本物の must-fix に基づく指示だけ（分割計画が無い時期の誤判定や、再起動による自動の再開は数えない） | `t3_thread_configure` で variant または `thought_level` を 1 段上げる（GLM: `low`→通常→`high`→`max`。Devin CLI: `medium`→`high`→`max`）。`max` でも直らなければ Devin CLI の `swe-2-high`（`thought_level=max`）に替える。Issue に理由をコメントする |
| 同じ Issue の修正の指示が 3 回を超えた（4 回目の修正に入る）。数え方は上の行と同じ | 修正を出さずにエスカレーション（runbook 7）。round 4 に入るのは hatai が例外と判断した PR だけ（例外は下の「マージ」。これ以外に広げない）。エスカレーションでは、残った must-fix の一覧と止まった head の sha を Issue に書かせ、止まっている間は指揮役も修正を中継しない（#462 の再発防止。「レビューのラウンド」の止まり方と同じ） |

runbook と仕様の `round`（スレッドの実行回数）は、誤判定や再起動による再開でも増える。指揮役は `round` を目安にとどめ、effort の引き上げとエスカレーションは上の表の「修正の指示の回数」で判断する。ここで数える「修正の指示の回数」と、実装側がレビューの委託回数で数えるレビューのラウンド（下の「レビューのラウンド」）は別の数え方である。

### レビュー担当と effort

| 条件 | レビュー担当 |
|---|---|
| 通常 | PR 番号が偶数は Codex（`codex_review`。モデルは下の「Codex のモデル」）、奇数は Devin CLI（`devin_review`、`swe-2-high`）。使えない担当（下の確認ゲート）の番では Codex に回す |
| 高リスク: `unsafe`、セキュリティの境界（認証・権限・承認・秘密情報・ネットワーク）、`.github/workflows/` の変更、pr-size の追加行が 400 を超える | Codex・Claude（`claude_review`）・Devin CLI の使えるもの全て（3 人） |
| 抜き取り | PR 番号を 4 で割った余りが 0 の通常の PR には Claude も付ける（Codex・Devin CLI の見落とし率の測定のため） |

- この表の適用は指揮役だけでなく実装担当も行う。実装担当はこの表でレビュー委託（`delegate_task`）の担当と effort を決め、Assigned 行に写す（skill `implement-issue` の手順 8）。通常と抜き取りの判定は PR 番号だけで決まり、割り当ての記録への記入は不要（実装担当の分は省く。指揮役が立てた分だけ記録する）
- レビューを `delegate_task` で委託するとき（実装担当の手順 8、指揮役が実装担当の代わりに委託するとき。指揮役は委託元の worktree が `main` のコミットにあることを確かめる。指揮役自身の PR・外部投入のレビュースレッドは委託せず、runbook の「レビューを始める」で立てる）は、呼び出しを次のブロックから丸ごと写し、`<...>` を埋める。`target.model` と `options` の値は skill `implement-issue` の手順 8 の表（codex は `reasoningEffort`、claude は `effort`、devin は `{}`）。`role`・`runtimeMode`・`isolation` は省かない。

  ```json
  {
    "target": {
      "providerInstanceId": "<codex_review|claude_review|devin_review>",
      "model": "<モデル名>",
      "options": {"<reasoningEffort|effort>": "<effort>"}
    },
    "role": "review",
    "runtimeMode": "auto-accept-edits",
    "isolation": "worktree",
    "task": "PR #<M> をレビューする。skill review-pr に従う。\nAssigned: role=review agent=<codex|claude|devin> model=<モデル名> effort=<effort> security=yes run=<委託記録のコメント ID>"
  }
  ```
- レビュー担当は、委託の指示とは別に、開始時に fail-closed のツール一覧チェックを行う（開始時に渡されたツール名の一覧を記録し、merge・Issue の変更・comment などの書き込み系ツールが含まれる、または一覧が読めないときは何もせず終了する。チェックの本体は skill `review-pr` の「決まり」）。チェックに失敗したレビュー（`verdict: failed` で `error:` が `設定の不備:` で始まるもの）は結果を採用しない（同じ環境では同じ結果になるため立て直さず、人間に知らせる。投稿済みのレビューも「hatai のレビューの扱い」の「有効な Approve」に数えない）。
- 再委託・遅延実行タスクの陳腐化ガード: レビュー・QA の再委託（同じ PR に 2 回目以降の委託を出すとき）と、遅延実行タスク（`schedule_task` などで、組んだ時点より後に発火して委託や実行の指示を出すタスク）の task には、上のブロックの task の 2 行の後に次の 1 行を定型文として足す。組んだ時点の head と、発火・着手の時点の head や成果の有無がずれていても、古い SHA のまま動かないようにするため。

  ```
  Guard: head=<対象 head の SHA>。実行時点で (a) 対象 PR の head が指定 SHA と一致すること、(b) この委託の担当（`Assigned:` の行の `agent=`）による、その SHA に向いた `DISMISSED` でないレビュー（本文の末尾行の `Reviewer:` の `agent=` が一致するもの）、または QA の判定が未投稿であることの両方を確かめ、どちらかが外れていれば何もせず終了する。
  ```

  - 受け取った側（委託の子・発火したタスク）は、作業の前に (a) と (b) を確かめる。読み取りの `<role>` は委託先の名義で、レビューは `review`、それ以外（QA を含む）は `dev`（`qa` は判定のコメントの投稿しか受け付けず、`pr view` や `api` は拒まれる）。下の「委託先の worktree に手順書が無いとき」の取得でも同じ。(a) は `scripts/agent-gh <role> pr view <M> --json headRefOid --jq .headRefOid` が指定 SHA と同じであること。(b) は、レビューなら `pr view <M> --json reviews` の `pelway-review` のレビューのうち、対象コミットが指定 SHA で、`state` が `DISMISSED` でなく、本文の末尾行（最後の空でない行。定義は skill `implement-issue` の手順 9「投稿後の `設定の不備:` の記録」）の `Reviewer:` の `agent=` がこの委託の担当と一致するものが無いこと（複数の担当へ委託するとき、先に投稿された他の担当のレビューでこの委託の完了と誤判定しないため。末尾行が `Reviewer:` の行でないレビューは確かめの対象にしない。dismiss されたレビューはその担当の結果として残らないため数えず、無効記録の解消後の再レビューも取り消した投稿で止まらない）、QA なら `pelway-qa[bot]` のコメントに指定 SHA を書いた判定が無いこと（合格・不合格の両方を数える。投稿者の確認は `docs/agents/qa-checklist.md` の「合格として数えるもの」を、判定と SHA の書式は同じ文書の「コメントの形」を参照する）。どちらかが外れていれば、委託・投稿・コメントのどれもせず、外れた条件（現在の head、見つけた成果の URL）を返答に書いて終える。
  - (b) は、末尾行に ` run=<ID>` の無い投稿（委託記録の ID で結ぶ決まりより前の書式）でも外れる。そうした投稿は、どの実行にも結び付かず Approve にも数えないため、再委託でも置き換えられない。委託する側は、再委託の前に、現在の head にどの実行にも結び付かない投稿が残っていないかを確かめ、残っていれば skill `implement-issue` の手順 9「結び付かない投稿の取り消し」のとおり、hatai の dismiss を確かめてから新しい委託記録を残して委託する（エージェントは dismiss しない。`CHANGES_REQUESTED` の投稿は、指摘を直す push で現在の head から外れる）。
  - 遅延実行タスクが委託を出すタスクなら、出す委託の task にもこの行を写す。
- 委託先の worktree に手順書が無いとき: 委託タスクが参照する手順書（skill の `SKILL.md`、`docs/agents/qa-checklist.md` など）が委託先の worktree に無いときは、まず対象 PR がそのパスを変えているかを `scripts/agent-gh <role> api 'repos/hatai/Pelway/compare/main...<対象 head の SHA>' --jq '.files[].filename'` にそのパスが含まれるかで確かめる（`<role>` は上の箇条と同じ。判定に PR の現在の差分を使わず、対象 head の SHA に固定して、判定と取得を同じ SHA に結び付ける。この一覧は GitHub が 300 ファイルで打ち切り、打ち切りを知らせないため、`.files | length` が 300 に達していたら対象のパスが一覧から漏れている可能性があり、そのパスを変えたと扱って main の版に従う）。変えていなければ、対象 head の版を `scripts/agent-gh <role> api 'repos/hatai/Pelway/contents/<path>?ref=<対象 head の SHA>' --jq .content | base64 -d` で読む。ただしレビューの委託先は、変わっていないパスも含めて従う手順を main の版に限る skill `review-pr` の「決まり」に従い、この規定と食い違うときは review-pr の決まりが優先する。変えていれば、手順として従うのは main の版（`ref=main`）で、PR が変えた版には従わない（検査される PR が、検査する側の手順を書き換えられないようにするため。この「変えたパスは main の版に従う」点は skill `review-pr` の「決まり」と同じ）。対象 head の版は、レビュー・QA の対象として読むだけにする。どちらの取得も失敗したら、手順書を推測で補わず、取得できなかったパスとエラーを返答に書いて止まる。
- Devin CLI が実装した PR には Devin CLI のレビューを付けない。通常の番が奇数でも Codex に回し、高リスクの PR でも Devin CLI の枠は外す。
- Devin CLI のレビュー担当は確認済み。確認記録は `docs/dev-environment.md`「レビュー担当の Devin CLI」の「確認手順の結果（記録欄）」。記録が消えた場合は、記録が戻るまで Devin CLI を使わず、通常は Codex、高リスクは Codex と Claude で回す。
- Devin CLI のレビュースレッド（`devin_review`）では、モデルオプションは `mode` だけを受け付ける（`thought_level`・`speed` は渡せず、effort の引き上げは行えない）。sandbox のため権限モードは Autonomous 一択で、`mode` は省略してよい。設定は `docs/dev-environment.md` の「レビュー担当の Devin CLI」。
- レビュー担当の確認ゲート（更新後の保留）。`codex_review` は Codex・T3・`~/.codex-review` のどれか、`claude_review` は T3・Claude Code・`~/.claude-review` のどれかを更新したら、`docs/dev-environment.md` の対応する確認（Codex は「専用 home での確認」、Claude は「レビュー担当の Claude」の MCP のツールと組み込みのツールの確認）が済むまで使わない（T3 の更新では両方が同時に保留になりうる。そのときは人間に知らせる）。更新を予定しているときは、更新の前に、影響を受ける側の節に `保留中:` の行を置き、その行が main に載るまで更新を始めない（行が main にあれば、委託や起動・再開の直前に main の最新の版を確かめるすべての割り当て元が保留を見られる）。更新済みと後から気づいたときは、気づいた者（人間か指揮役）が同じ行を置いてから確認を行い、その行が main に載るまでは、指揮役が動いている実装担当に直接伝え、委託元となりうるスレッドの新規の起動・再開を行わない（指揮役自身の起動でも同じ）。指揮役と実装担当は、委託やスレッドの起動・再開の直前に、main の最新の `docs/dev-environment.md` で、割り当てる担当と回し先の両方の節（「専用 home での確認」と「レビュー担当の Claude」）の `保留中:` の行の有無を確かめる。保留の間は、使えない担当の番とその担当への回し先をもう片方の担当に回し（Codex の保留なら Claude、Claude の保留なら Codex）、両方使えなければ人間に知らせる。
- Claude のレビュー担当は確認済み。確認記録は `docs/dev-environment.md`「レビュー担当の Claude」の「確認手順の結果（記録欄）」。記録が消えた場合は、記録が戻るまで Claude を使わず Codex だけで回し、高リスクの PR は人間に知らせる。
- 抜き取りの実施が 10 件に達するか、抜き取りの開始から 4 週間が経つか、どちらか早い方になったら、`docs/agents/command-team-details.md` の「レビュー担当の段階移行」に従って見落とし率を集計し、高リスク PR の複数レビュー体制の有効化を判断する。
- effort は、追加行が 200 以下で高リスク（セキュリティの境界）・仕様（正本 `docs/superpowers/specs/` と ADR `docs/adr/` を変える）の PR でなければ既定、それ以外は `high`。セキュリティと仕様の PR では 1 回目から effort を上げておく。同じ PR の 2 回目以降のレビューは 1 段上げる。3 回目のレビューで改めて effort を上げる運用はやめる（skill `implement-issue` の手順 9 も同じ）。
- Codex のモデル（選び方の正本。ほかの文書はここを参照する）: 委託のたびに、上と同じ材料（追加行数、高リスクかどうか、レビューの回数）で次の表から 1 つ選び、Assigned 行と `target.model` に写す。上から順に見て、最初に当たった行のモデルを使う。

  | 条件 | モデル |
  |---|---|
  | 追加行が 200 を超える、高リスク（上の表の高リスクの条件）、または同じ PR の 3 回目以降のレビュー | `gpt-6.2-sol` |
  | 文書だけの PR | `gpt-6-luna` |
  | それ以外（追加行が 200 以下で、高リスクでないコードの PR） | `gpt-6.1-terra` |

  - 文書だけの PR は、変更したファイルがすべて Markdown（`*.md`）の PR とする。確かめ方は `scripts/agent-gh dev pr diff <M> --name-only` の全行が `.md` で終わること。試験・スクリプト・設定ファイルが 1 つでも含まれれば、文書だけではない。
  - `gpt-6.2-astra` は使わない。コストが `gpt-6.2-sol` の 4 倍で、セキュリティの評価（SEC-Bench Pro）は `gpt-6.2-sol` の方が高いため（出典は Issue #563）。
  - 根拠: OpenAI の公式の資料で、`gpt-6.2-sol` は PR のレビューに勧められている。`gpt-6.1-terra` は速く効率的だがコードの評価が `gpt-6.2-sol` より低い。`gpt-6-luna` は最も安く、量の多い作業向け（出典は Issue #563）。
- 複数のレビュー担当を付けた PR で判定が割れたら（approve と request-changes が混ざったら）、マージせず、修正側（request-changes）の指摘を優先して対応する。セキュリティの指摘は特に修正側を優先する（「レビューのラウンド」）。Issue にそれぞれの要点をコメントし、request-changes 側の must-fix に対応する修正をそのまま進める（人間へのエスカレーションは不要）。実装担当が委託した複数レビューで割れたときも同じで、実装担当が修正に進む（skill `implement-issue` の手順 9）。

### レビューのラウンド

レビューのラウンドの数え方と上限。実装側の処理（修正・再委託・ラウンドサマリー・止まり方）の正本は skill `implement-issue` の手順 8〜9。上限と例外の判断に関わる指揮役側の対応は「実装のモデルと effort」の表のとおり。

- 同じ head は 1 ラウンドに数える。コンフリクトの解消を含まない main の取り込みだけで head が変わっても、新しいラウンドにしない。
- 3 回目以降の must-fix は領域で決める。次に当たる指摘は、何回目でも must-fix に残す:
  - 認可・読める人の規則・取り消し
  - 秘密と認証情報
  - データの境界（コアの処理と保存は国内だけ、接続しただけでは送らない）
  - 外への通信
  - 入力の上限とリソース枯渇
  - データの消失と整合性
  - CI やフックなどの守りを外すこと
- それ以外（この領域の外側）を別 Issue に回すかは、レビュー役が分類する（分類の手順とレビュー本文への記載方法は skill `review-pr` の手順 3〜4）。実装役は分け直せない。迷うものは must-fix に残す。
- 別 Issue に回すときは、ラウンドサマリーに理由を 1 行書き、別 Issue を PR と元の Issue の閉じる条件にリンクする。
- セキュリティ領域が誤って回されていたら、マージを止める。
- セキュリティ以外の must-fix には、具体的な失敗の筋道を書く。
- 仕様の穴（拒否リストや検知の規則から抜け道が出続けるもの）は仕様に戻し、ラウンドを 1 から数え直す。hatai の OK なしの数え直しは 1 回まで。
- 人の作業待ちの項目は、その間は合格ではなく未検証として扱い、担当・期限・sha を記録する。ラウンドには数えない。
- ラウンドの上限は 4 回。上限まで来ても must-fix が残ったままマージしない。4 回目にセキュリティの must-fix が出たら止めて、hatai の判断に戻す。
- round 3 の must-fix が、別 Issue に回せる側を除いて残り、4 回目に入る条件（hatai の例外の判断。「マージ」の例外）を満たさないときは、修正もコミットも push もせずに止まる（skill `implement-issue` の止まる条件）。must-fix の一覧と止まった head の sha を Issue に書く。手元にすでに修正があれば、worktree に置いたまま push も削除もしない。止まっている間は、指揮役も修正を中継しない（#462 の再発防止）。すでに push された修正はそのまま残してよい。ただし、それで round 4 に進むかは hatai の例外の判断で、修正の差分も含めてレビューする。
- 判定が割れたときは「レビュー担当と effort」のとおり修正側（request-changes）を優先する。セキュリティの指摘は特に修正側を優先する。
- `--no-verify` は使わない（commit のフックを止めない）。
- 見張り（conductor-watch）は、別 Issue に回した件数を数える。QA があとで抜き出して確かめる。

### レビュースレッドの settle

レビューが終わったレビュースレッドを未 settle のまま残さない（runbook 9 の「片付け」と同じ決まり。settle しないと worktree も残る）。settle は `t3_thread_organize`（`action: settle`）で行う。

- verdict を読んだら、指揮役が自分で立てたレビュースレッド（指揮役自身の PR・外部投入）を settle する
- 次のラウンドのレビュースレッドを立てたら、置き換わった前ラウンドのレビュースレッドを settle する
- 実装担当の PR のレビュー（実装スレッドが `delegate_task` で立てた子タスク）は実装スレッドの内側で settle される（skill `implement-issue` の手順 8）。指揮役は settle しない
- `needs-human` 中のスレッドと、止まっている・迷走中のスレッドは settle しない

### レビュー待ちの滞留

レビュー待ちの滞留は、PR ごとに skill `pr-watch` の `pr-digest` を出し、`review-wait` 行（レビュー待ちの経過時間と起点の時刻）で機械的に検出する。起点は PR が最初にレビュー可能になった時刻で一意に決まり、レビュー担当の再割当では戻らない（形式は skill `pr-watch`）。

- 経過が目安の 24 時間を超えた PR の対応は、その時点の指揮役（正本の Grok Bot、代行中は代行者）が責任を持つ。責任の持ち方は、対象によって次のとおりその場で決めて実行することである。
  - 実装担当の PR: レビューの委託は実装担当の内側で進むため、まず実装スレッドの状態（実行中か・レビュー委託の子を待っているか・止まっているか）を `t3 thread list` と `t3 thread show` で確かめる。止まっていれば再開の指示か runbook 7 のエスカレーション、実装スレッドが動けないときは外部投入（`docs/agents/command-team-details.md` の「外部からのレビュースレッド投入」）へエスカレーションする
  - 指揮役自身の PR: レビュー担当を割り当てる（上の「レビュー担当と effort」で決める。レビューの枠が埋まっているときは、その PR を順番待ちの先頭に置く）
- 24 時間は目安であり、超えた時点で必ず上のどちらかを取る。何もしないまま放置しない。値は運用実績で見直す。

### 調査（入口B）

入口B（原因が未理解のバグ・インシデント報告）で調査スレッドを立てるとき、または調査スレッドが報告を出したときは、`docs/agents/command-team-details.md` の「調査（入口B）」に従う（調査の担当と effort、スレッドを立てる指示の形、報告の確認と起票可否の判断、報告 PR の扱い）。どの報告が入口B かは、正本 `docs/superpowers/specs/agent-team.md` の「タスクの入口と流れ」。

### hatai のレビューの扱い

有効な Approve: `pelway-review` の最新の有効なレビューが APPROVED で、対象が主 PR の現在の head であるもの（head が変われば失効し、レビューが無いものとして扱う。dismiss されたレビューは数えない）。投稿したレビュー担当が後から `error:` が `設定の不備:` で始まる `verdict: failed` を報告したレビューは、Approve・Request changes のどちらも数えない（fail-closed のツール一覧チェックに失敗したレビューは結果を採用しない決まりによる。GitHub 上の投稿は残るため、環境の修復と投稿の dismiss は人間が扱う。同じ環境では同じ結果になるため、再委託では置き換えない）。その報告の対象は、対象 PR に残される `[review-invalid]` の記録（対象の PR・SHA・レビューを特定する定型コメント。形・記録として数える作成者（`pelway-dev[bot]` だけ）・解消の照合は skill `implement-issue` の手順 9「投稿後の `設定の不備:` の記録」が正本）で確かめる。未解消の記録が主 PR に 1 件でもあるあいだは、記録のレビューに限らず、その PR のどの Approve も有効な Approve に数えない（同じ head に別の担当の Approve が先にあっても数えない）。承認の判定はこの定義を通るため、環境の修復とすべての記録の解消までは、その PR の作業の再開も Ready to merge も `automerge` も行わない。記録の解消は、記録のレビューを hatai が手で dismiss したこと（push による自動の取り消しは数えない）か、`review: unknown` の記録と対象を dismiss で確かめられない記録では hatai の解消コメント（`[review-invalid-resolved]`。エージェントは書かない）で確かめる。すべての記録が解消した後も、記録の対象の head（解消コメントで解消した記録では現在の head）が現在の head なら、記録の `to:` の担当の再レビュー（すべての記録の解消の後に残した委託記録の実行の投稿で、同じ head を向いた、dismiss されていないもの。条件と始め方は同じ節）がそろうまで、その head の Approve を数えない。

hatai へのレビュー依頼（「hatai 待ち」の定型コメント・終了条件・Status の扱い）と、hatai が紐づく PR にレビュー・コメントを投稿したときの伝達の手順は、`docs/agents/command-team-details.md` の「hatai のレビューの扱い」に従う。巡回での検知と Status の反映は runbook の「ボードと巡回」の「hatai のレビューに関する連動」が正本。

### マージ

マージは Kodiak（ホスト版の GitHub App。正本は `docs/superpowers/specs/agent-team.md` の「マージキュー」）が行う。原則は、条件を満たしたら指揮役が `automerge` ラベルを付けて Kodiak がマージする、である。指揮役が行うのはキューへの投入だけ: 下の条件を確かめて、PR に `automerge` ラベルを付ける（`scripts/agent-gh dev pr edit <M> --add-label automerge`）。このラベルを付けることがマージの指示であり、付けるのは指揮役だけ。実装スレッドは自分の PR に付けない。ラベルが付くと、Kodiak がブランチの最新化とマージを直列に進める。指揮役はマージを確認してから Issue から `agent:working` を外し、人間に報告する。

`automerge` ラベルを付ける条件（すべて確かめてから付ける）:

- 有効な Approve（上の「hatai のレビューの扱い」）がある。未解消の `[review-invalid]` の記録があればこの条件を満たさない
- その Approve を投稿したレビューの委託の結果が確定している: 現在の head の必要な実行（skill `implement-issue` の手順 8 の「必要な実行」。必要な担当は委託の時点で上の「レビュー担当と effort」の表で決まり、その head に向けた委託記録のある担当も含む。担当ごとに、その head に向けた委託記録の実行のうち、`DISMISSED` でない投稿をちょうど 1 件持つ最新のもの。そうした実行の無い担当が残れば確定しない）のすべてから結果の報告を受け、そのすべての実行の投稿が現在の head を向いてちょうど 1 件ずつあり、さらに現在の head を向いた `pelway-review[bot]` の `APPROVED`・`CHANGES_REQUESTED` の投稿（dismiss されたものを除く）のすべてが、結果の報告を受けた実行の投稿（skill `implement-issue` の手順 8 の「実行の投稿」。委託記録の ID と末尾行の `run=` で結び付き、実行ごとにちょうど 1 件）である。結果の報告は、実装スレッドの終わりの報告（`state: done`）では `pr:` の行がその head を指し、`review:` の行の `run` にその実行の ID があること、指揮役が立てたレビュースレッドでは、その実行（スレッドを立てる前に残した委託記録の ID）から `verdict: approve` か `verdict: request-changes` の報告を受けていること。指揮役は、報告の ID ごとに、委託記録（対象 PR のコメント（REST の `issue_url` が `/issues/<M>` で終わる）で、`user.login` が `pelway-dev[bot]`、1 行目が `[review-request] head: <その head> to: <担当>`。skill `implement-issue` の手順 8 の「実行の投稿」の最後の文の定義）と、その実行の投稿の `id`・`state`・`commit_id` を GitHub で確かめる。必要な実行のうち報告の無いもの（まだ投稿していない・実行中のものを含む）が 1 つでも、報告の無い実行の投稿・どの実行にも結び付かない投稿（末尾行に `run=` が無いものを含む）が現在の head に 1 件でもあれば、先に結果の確定した担当の Approve がそろっていても、最新のレビューが `APPROVED` でもキュー投入の承認に使わない。結び付かない投稿は、skill `implement-issue` の手順 9「結び付かない投稿の取り消し」のとおり hatai の dismiss を待つ（レビュー担当は投稿の後に `設定の不備:` で失敗を報告することがあり、同じ App の名義で複数の担当が投稿するため。skill `review-pr` の「決まり」）
- PR に `needs-human` が付いていない
- その Approve のレビューの nit に行頭が `[文書のずれ]` のものが残っていないこと（残っていれば実装担当に直させる。skill `implement-issue` の手順 9「Approve のとき」）
- 必須チェック（`mise run verify` と `pr-size`）が通っている
- Draft ではなく、タイトルに `WIP:` がない
- 現在の head と同じ SHA の QA の合格コメントがある（QA 合格コメントの形と数え方は `docs/agents/qa-checklist.md` の「合格として数えるもの」が正本。この節の末尾も見る）。SHA が head と違う合格・`pelway-qa` 名義でない合格・編集された合格・`run:` の無い合格は数えない。QA が確かめるまでは合格として扱わない
- Approve と QA 合格のあとに許すのは、コンフリクトの解消を含まない main の取り込みだけ。指揮役は、取り込みのコミットに main 以外の差分がないことを確かめてからラベルを付ける（`git rev-list <Approve した sha>..<head> --no-merges ^origin/main` が 0 件。それ以外のコミットがあればレビューと QA をやり直す。Kodiak がブランチ最新化で作る main 取り込みのマージコミットはこの条件を壊さない）
- コンフリクトを解消したときは変更として扱い、新しい head でレビューと QA をやり直す

hatai が判断する例外（マージを hatai の admin マージで行う。これ以外には広げない）:

- round 4 以降の例外 PR（ラウンドの上限は「レビューのラウンド」。round 4 に進むかは hatai が判断する）
- `.github/` の下を変える PR
- `fnox.toml` を変える PR

Kodiak が応答しない・詰まった場合（キューの切り戻し）と、必須チェックが失敗したままのマージ（`pr-size-exception`）は、マージの例外には加えない（例外は上の 3 つだけ。これ以外に広げない）。Kodiak でマージできないこれらの PR は、hatai が上の例外のどれかに該当すると判断したときだけ admin マージされる。`scripts/conductor-merge-queue.sh` は hatai の admin マージの補助として残す。

ラベルを付けたあと:

- 新しい push や Draft への戻しがあったら、指揮役がラベルを外す。条件が再び揃ったら（Approve のやり直しを含む）付け直す
- Kodiak の自動更新（main への追いつき）が取り込みを行ったあとも、指揮役は取り込みのコミットに main 以外の差分がないことを確かめる。Kodiak はコンフリクトを解消しないため、その取り込みは「コンフリクトの解消を含まない main の取り込み」に当たる（hatai の決定、Issue #480）。検知と手当は runbook の「マージキュー待ちの PR」

部屋や指揮役の報告と GitHub が食い違ったら GitHub を正とし、報告の値では動かない。指揮役は報告を GitHub の CI 結果・sha・PR の状態と突き合わせてから動く（runbook の「報告行の扱い」と同じ原則）。

QA 合格コメント:

- QA の合格は PR コメントで残す。PR 本文だけの「合格」は無効。形と、合格として数える条件（`pelway-qa[bot]` の Bot であること・投稿の後に編集されていないこと・1 行目・`sha:`・`run:`）と確かめ方のコマンドは、`docs/agents/qa-checklist.md` の「合格として数えるもの」に従う
- 作成者の login だけでは足りない。実装担当の App（`pelway-dev`）など Issues・PR の書き込み権限を持つ名義は、ほかの名義のコメントを編集できるため、古い合格コメントの `sha:` を新しい head に書き換えると、作成者・1 行目・SHA の一致がすべて通ってしまう。編集されたコメントは、作成者が `pelway-qa[bot]` でも数えない
- ほかの作成者のコメントは、本文の形と SHA が合っていても QA の合格に数えない。hatai の名義のコメントも QA の合格に数えず、`pelway-qa` の名義のコメントは hatai 本人の裏付けに数えない（Issue #480 の決定）
- `pelway-qa` の名義で合格コメントを投稿できないあいだ（App のトークンが使えないなど）は、QA の合格をもとに `automerge` を付けない。ラベルを付けずに止め、hatai に報告する（admin マージの例外には加えない。例外は上の 3 つだけ）
- 指揮役は、コメントの `sha:` が PR の現在の head と一致することを確かめてからラベルを付ける（head が変わったら QA のやり直しを依頼する）

マージ以外の PR・Issue への操作（base の変更、Draft の扱い、Issue のクローズ）は、先に「手当て・引き継ぎの前の確認」を通す。目印付きのコメント（`<!-- pr-size-exception -->`、`<!-- split-plan -->`）は人間の名義の `gh issue comment` で書く。レビュー担当は作成者が `hatai` であることで、実装担当（`pelway-dev`）や第三者のコメントと区別する。それ以外の GitHub の操作（読み取り、ラベル、ふつうのコメント）は `scripts/agent-gh dev` で行う。

- pr-size の失敗が許されるのは、指揮役が Issue に行頭が `<!-- pr-size-exception -->` で始まるコメントで超過を認めた理由を書き、PR に `pr-size-exception` ラベルを付けたときだけ（`scripts/agent-gh dev pr edit <M> --add-label pr-size-exception`）。レビュー担当（skill `review-pr`）はラベルとコメントの両方を確かめてから、pr-size の失敗を must-fix に数えない。レビューの指示に例外の文を足さない。pr-size の失敗したままの PR は必須チェックを通らないため Kodiak ではマージできず、`automerge` ラベルは付けずに hatai の例外の判断に回す（マージの例外は上の 3 つだけで、pr-size-exception 自体は例外に加えない）。運用まわりの PR（主な変更先が skill `write-issue` の手順 4 の「運用まわりの Issue」の定義に当たる PR）が pr-size を超えたときは、例外を提案するより先に `<!-- split-plan -->` での分割を提案する（下の「分割を認めるとき」）。
- 必須チェックの完了待ち（ラベルを付ける前の CI 待ち、巡回での PR のベビーシット）は、ポーリングせず skill `pr-watch` の「待ち方」に従う（T3 のスレッドでは `watch_pull_request`）。マージの実行は Kodiak が行うため、指揮役がマージコマンドを叩くことはない。
- skill `pr-watch` の `VERDICT green` は、その SHA でチェックが完走して通過していることの確認要素の 1 つ。上の「`automerge` ラベルを付ける条件」を置き換えない。
- ブランチの最新化はルールセットが「マージ前にブランチを最新化すること」で必須とし、実行は Kodiak が行う。指揮役が `pr update-branch` で手動に取り込む操作は、手当スレッドでの衝突解消などの例外を除き原則しない。
- 別の PR に積んだ PR は、土台の PR に `automerge` ラベルを付ける前に、後の PR の base を main に替える（`scripts/agent-gh dev pr edit <後の PR> --base main`）。base のままだと、リポジトリのブランチ自動削除で土台のブランチが消えたときに後の PR が閉じることがあり、閉じた PR は base を替えられない。
- 複数の PR に `automerge` ラベルを付けてよい。Kodiak が直列に 1 本ずつマージする。ただしラベルを付ける時点で各 PR が上の条件を満たしていることを確かめる。
- PR ができたら mergeable を確かめる（`scripts/agent-gh dev pr view <M> --json mergeable`）。`CONFLICTING` の PR には GitHub が `pull_request` の workflow を動かさず、チェックが 1 つも付かない。実装担当に「main を取り込んで衝突を解消し、verify を通す」を送る。`automerge` ラベルを付けた後に Kodiak がマージできなかったときは、runbook の「マージキュー待ちの PR」に従う。
- マージしたら Issue から `agent:working` と `agent:devin` を外し、人間に報告する。マージの検知は notify-grokbot の `pull_request`（`merged: true`）の通知と巡回で行う。
- マージが完了したら、ローカルの `main` を `origin/main` へ一致させる。`git fetch origin main` の後、ローカルの `main` が `origin/main` の先祖になっていることを確かめ（`git merge-base --is-ancestor main origin/main`）、`git merge --ff-only origin/main` を `main` で実行して fast-forward のみで行う。ローカルの `main` に origin に無いコミットがあって fast-forward できないときは、強制せず（rebase も reset もしない）そこで止めて、指揮役（代行の場合は hatai）へ報告する。
- Dependabot の PR の扱い（minor・patch の通常扱い、major・0.x の移行 Issue 化、Action の更新の `actions.lock` の再生成）は `docs/agents/command-team-details.md` の「Dependabot の PR の扱い」に従う。

### 分割を認めるとき

実装担当が pr-size の超過で止まり、分割案が追加行の境目できれいに分かれるときは、分割を認める。Issue に、行頭が `<!-- split-plan -->` で始まるコメントで、PR ごとの範囲と、その PR で確かめる受け入れ条件を表にして書く。受け入れ条件は skill `write-issue` の「受け入れ条件の書き方」に従う。レビュー担当（skill `review-pr`）は、この表でそれぞれの PR を判定する。後の PR は前の PR のブランチに積ませ、前の PR のマージの前に base を main に替える（上の「マージ」の積んだ PR の項）。運用まわりの PR が pr-size を超えたときは、例外（`<!-- pr-size-exception -->`）の提案より先にこの分割を提案する。

### 利用上限で止まったとき

実装スレッドやレビュースレッドが理由を残さずに `failed` になったら、`t3_thread_read`（`view: activity`）で error の項目を読む。「Usage limit reached」（GLM の 5 時間あたりの上限、Devin のプランの上限など）なら、Issue の中身の失敗ではないので、runbook の「2 回目の `failed` でエスカレーション」に数えない。リセット時刻まで新しいスレッドを立てるのを止め、リセット後に小さな試験のスレッドで使えることを確かめてから、止まったスレッドに「続きをやり直す」を送る。

## 3. エスカレーション

runbook 7 の手順に従う。人間に上げる前に、手順 2 の「effort を上げる・モデルを替える」の段を必ず挟む。上げるときは、試したこと（effort、モデル、round）を Issue のコメントに書く。人間に上げるときはおすすめ付きで出す: Issue のコメントにチームのおすすめ（選ぶ案と理由）とほかの案を書き、緊急でない判断待ちの詳細は一括出しに回す（正本は runbook 7「エスカレーション」の手順 2・3 と、runbook 8 の「判断待ちの一括出し」）。

## 4. 改善ループ

改善の対象は、指揮役のセッションだけでなく、実装担当（opencode＋GLM、Devin CLI＋SWE-2）とレビュー担当（Codex、Devin CLI、Claude）の両側。

### 材料を集める

| 材料 | 場所 |
|---|---|
| 実装者メモ（一次情報） | `docs/worklogs/issue-<N>.md` の「実装者メモ」、止まったときは Issue のコメント |
| 実装セッションの記録 | `t3 thread show <thread>`。圧縮のやり直し、ループ、誤読、取りこぼしを見る |
| レビューの指摘・差し戻し | PR の `pelway-review` のレビュー本文 |
| Codex・Devin CLI の見落とし | 抜き取りで付けた Claude のレビューが、Codex・Devin CLI の approve の後に must-fix を出したもの |
| 指揮役の学び | 指揮役の作業ログ（`docs/worklogs/`） |

### 裏取りする

- 実装者メモは自己申告で、本人が気づいていない誤解は書かれない。レビューの指摘とセッションの記録で裏を取る。食い違ったら、実際の成果物とセッションの記録を優先する。
- レビューで同じ種類の指摘が 2 回以上出たら、実装側の指示・スキルの不足として扱う。
- レビュー担当の指摘が head の版のファイルと合わないときは、レビュー担当側の材料として扱う。
- eval ケース・事例を材料にするときは、一次記録（Issue・PR のコメント、レビュー本文、実装者メモ、セッション記録）からの引用を付ける。引用で裏付けられない内容は「想定」と明記する。

### 共有層に反映する

- 書き込み先は両方のツールが読む場所だけ: `AGENTS.md`、`.agents/skills/`、`docs/`。
- `.claude/skills/<name>` は `.agents/skills/<name>` へのシンボリックリンクにする。変更したら `diff -r .claude/skills/<name> .agents/skills/<name>` の差分がゼロであることを確かめてから完了とする。
- 変更は PR にして、レビュー担当のスレッドに回す。
- スキル改訂の候補（自前スキルの改善・作成・最適化）は、skill `improve-skills` の入口A（候補の調査と起票）に渡す。共有層への反映のうち、スキル改訂の編集作業そのものは improve-skills の手順に従う。この手順 4 が担い続けるのは、材料の集め方・裏取り・効果測定・棚卸しの管理である。

### 効果を測る

- 変更の効果は、Claude の挙動だけでなく GLM と SWE-2 の成果物で測る。基準は、同じ種類のレビュー指摘・差し戻しの減少。
- Claude では効いても GLM・SWE-2 で効かなかった文言は「共有スキルとして未達」とし、モデルの差を前提に書き直す（例: 抽象的な原則より、手順と具体例と禁止事項で書く）。
- レビュー担当ごとの見落とし率 = 抜き取りで Claude が must-fix を出した PR の数 ÷ 抜き取りの PR の数（Codex 担当分と Devin CLI 担当分で分けて数える）。改善ループの材料にし、同時数の上限の見直しにも使う。
- 巡回と実装担当の止まり方の効果は、巡回記録に残した次の 4 つで測る（記録の仕方は runbook の「巡回の報告の形」）:
  - 指示待ちの放置時間（目標: 2 巡回以内）
  - 毎日のまとめの時点の不要な未 settle のスレッドの件数（目標: 0 件）
  - 状態の行の無いまま終わった実装スレッドのターンの割合（実装担当のモデルごと。目標: 0）
  - 巡回の指示が無ければ再開しなかった `waiting-review`・`waiting-checks` の件数

### 棚卸しする

- 改善のたびに、関係するスキル・文書の古い記述、矛盾する記述を探し、直すか消す。溜め込まない。
- 作業ログの改善点の表で、反映済み・見送りになったものに印を付ける。

### 定期キュレーション（worklog の走査）

週 1 回、棚卸し担当（「1. サイクルの始め」。指揮役とは別のスレッド）が worklog 群を横断して走査し、レビュー指摘とは別の入口から改善ループの材料を拾う。狙いは、レビュー指摘に至る前の滞留・手戻りの型と、実装者メモ・巡回記録に書かれたまま拾われていない正本とのずれである。実行手順（頻度・対象・観点・出力先）は runbook の「作業ログの定期キュレーション」。

- 役割分担: レビュー指摘を入口とする既存の流れは変えない。キュレーションは、繰り返しの型・正本と実態のずれ・長期間動いていない待ちを候補として拾い、既存の改善ループの材料にする
- キュレーションは候補の提案までとし、正本（AGENTS.md、スキル、runbook）を直接編集しない。正本の変更が要る候補は Issue 化を人間に提案し、Issue の作成と `ready-for-agent` の判断は従来どおり人間の関与する流れに乗せる

## 5. 代行の開始と終了

正本の指揮役（Grok Bot）が動かず、人間が代行を頼んだときの開始のチェックと、正本へ戻すときの終了のチェックは、`docs/agents/command-team-details.md` の「代行の開始と終了」に従う（代行の条件・接続・引き継ぐ状態・代行の作業ログ・見張りの停止）。

## 6. 輸入スキルの試行（mattpocock/skills）

mattpocock/skills から `.agents/skills/` に取り込んだ輸入スキル（一覧と状態は `docs/agents/external-skills.md`）を使う割り当ては、通常運用への移行が決まるまで、試行として明示したものに限る。試行として割り当てるときの記録と、各スキルの試行（`tdd`・`diagnosing-bugs`・設計・ADR・`retro`・grill）の割り当て方の正本は `docs/agents/external-skills.md` の「試行の割り当て」。輸入品の取り込みと適用の discipline は同文書の「取り込みの discipline」が正本。

## 7. コンテキスト管理

指揮役スレッドのコンテキスト圧縮（計画的・予期しない）と恒久交代の手順の正本は `docs/agents/conductor-context.md`。次のいずれかで開き、その手順に従う。

- 圧縮のトリガーに達した（使用率 70% 以上。取得できない環境では代理指標: 起算点から完了 Issue 5 件以上、または巡回 75 回以上。前提欠落の自己検知は数値条件より優先する）
- 環境の自動圧縮を検知した、または前提と実状態の食い違いに気づいた
- 圧縮で改善しないため恒久交代を検討するとき（恒久交代の手順の前に、同文書の「立て替える前の確認」を先に行う。代行の開始でも同じ確認を先に済ませる）

結果を伴う操作（マージ、ラベル、コメント、スレッドへの指示・起動、ステータス変更）は、発行前に種類・対象・照合識別子を巡回記録に残し、結果確定後に追記する（正本の「操作発行時の記録（常時ルール）」。この記録が結果不明の操作の照合の土台になる）。
