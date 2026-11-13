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

## 決まり（毎回守る）

- 指揮役はコードを書かない。PR のレビューもしない（自分が関わった PR を自分で評価しない）。
- 指揮役が自分で作った PR（運用文書やスキルの変更）も、レビュー担当のスレッドに回す。
- スレッドに送る指示は、Issue 番号か PR 番号と skill 名だけにする。例外はラベル（例: `pr-size-exception`）と Issue のコメントで伝え、指示の文に書き足さない。ただし、スレッドを立てる指示（と、effort やモデルを変えた後の最初の修正の指示）には、2 行目に割り当ての行を必ず入れる。

  ```
  Issue #<N> を実装する。skill implement-issue に従う。
  Assigned: role=implement agent=<opencode|devin> model=<モデル名> effort=<effort>
  ```

  実装の値は、opencode なら `model=zai-coding-plan/glm-5.3 effort=<low|default|high|max>`、devin なら `model=swe-2-high effort=<medium|high|max>`（effort の値は `thought_level` に写す）にする。

  ```
  PR #<M> をレビューする。skill review-pr に従う。
  Assigned: role=review agent=<codex|claude|devin> model=<モデル名> effort=<effort>
  ```

  ```
  <報告の対象> を調査する。skill investigate-bug に従う。
  Assigned: role=investigate agent=opencode model=zai-coding-plan/glm-5.3 effort=<low|default|high|max>
  ```

  実装担当はこの値を PR 本文の `Implementer:` の行に、レビュー担当はレビュー本文の `Reviewer:` の行に、調査担当は報告の PR 本文の `Implementer:` の行に写す。
- 巡回では、ほかのセッションが担当しているスレッドや PR に介入しない（割り当ての記録の表で担当を確かめる）。人間から依頼された代行・引き継ぎだけが例外で、手順 5 の「代行の開始と終了」に従う。
- スレッドを立てるときは、必ず「スレッド名」の形式のタイトルを渡す。タイトルが形式に合わないまま作業を始めさせない。
- 割り当てのたびに、指揮役の作業ログの「割り当ての記録」の表に 1 行足す（日時、Issue・PR、役割、担当、モデル、effort、理由）。改善ループで、レビュー結果・実装者メモと対照する。
- スレッドへの指示は、新しい実行として送る（MCP の `t3_thread_send` で `mode: queue`）。実行中のスレッドに `steer`（既定の `auto`）で送ると、実行の途中に差し込まれ、その実行が指示を扱わずに終わることがある。終わりの報告と PR の head で、指示が扱われたかを確かめる。
- 判断はすべて Issue か PR にコメントで残す。セッションの文脈だけに置かない。
- 指揮役が Issue を立てる・分割する・差し戻された本文を直すときは、skill `write-issue` に従う。`ready-for-agent` を付ける前（人間に委任されたとき）は、`write-issue` の手順 6 のゲートを通す。
- 見張りの状態は `scripts/conductor-watch.sh` が `~/.local/state/pelway/` に置く。セッションの一時フォルダに置かない。

## スレッド名

指揮役が T3 で立てるスレッド（実装、レビュー、調査、修正、試験のすべて）のタイトルは、役割・対象の番号・日本語の要約の 3 つを含む次の形にする。要約は全角 30 字程度までで、一覧を見て何の作業か分かる内容にする。自動生成のタイトル、スレッド ID、ブランチ名だけのタイトルにしない。

| 役割 | 形式 | 例 |
|---|---|---|
| 実装 | `[実装] #<Issue 番号> <要約>` | `[実装] #101 pgBackRestで14日上限と復元を確かめる` |
| レビュー | `[レビュー] PR #<PR 番号> <要約>` | `[レビュー] PR #132 本体LLMの提供元とBYOK（ADR 0008）` |
| その他 | `[<役割>] <対象があれば番号> <要約>` | `[調査] GLMの利用上限の解除を確かめる`、`[修正] #95 ツールチェーンの更新` |

- MCP の `t3_thread_launch` では `title` に、CLI の `t3 thread start` では `--title` に渡す。省略すると最初の指示の 1 行目がタイトルになるので、省略しない。
- 立てた後で形式に合わないと分かったら、そのスレッドへの次の操作の前に `t3_thread_update`（`action: rename`）で直す。形式に合わない古いスレッドも、次に操作するときに直す。
- `scripts/conductor-watch.sh` は、指揮役が立てたスレッドのうち、動いているものと新しく現れたもので形式に合わないタイトルを `THREAD bad-title` として知らせる。

## 手当て・引き継ぎの前の確認

PR や Issue に手を付けるときは、最初に対象 Issue の受け入れ条件と到達点の状態（「Draft PR の存在をもって完了とする」「マージまで行う」など、skill `write-issue` の受け入れ条件の書き方）を確認する。適用範囲は次の 4 つに限る。

- (a) PR の Draft 解除と Draft への復帰
- (b) PR への main 取り込み（update-branch）と base の変更
- (c) Issue のクローズと再オープン
- (d) 受け入れ条件に触れる Issue 本文の変更

操作が受け入れ条件や到達点の状態と矛盾する場合（例: 到達点が「Draft PR の存在」なのに Draft を解除する）は実行せず、指揮役（代行の場合は hatai）へ報告して止まる。マージとレビュー投稿はこの確認の対象外（マージは「2. 振り分け」の「マージ」の決まりに、レビューの投稿は「2. 振り分け」の担当の決まりに従う）。

## 1. サイクルの始め

1. `scripts/conductor-watch.sh` が動いていることを確かめる（動いていなければ起動する）
2. `t3 thread list --json` と `scripts/agent-gh dev pr list` で、実行中のスレッド、Draft の PR、レビュー待ちの PR を数える。PR の中身（チェック・レビュー・コメントの状態）を見るときは、生の `gh api` を読む代わりに skill `pr-watch` の `pr-digest` を使う
3. 実行中のスレッドのタイトルが「スレッド名」の形式に合っているかを見て、合わないものを直す
4. runbook の「同時数と順番待ち」の順で、次の一手を 1 つずつ決める
5. 実装担当からの報告に「レビューが回っていない（指揮役の巡回待ち）」（skill `pr-watch` の完了ゲート）があった PR は、この巡回でレビュー担当と effort を割り当てる（手順 2 の「レビュー担当と effort」）。割り当てたあとの見張りの再開は、レビューが Approve なら実装担当の完了ゲートの確認（skill `pr-watch` の完了ゲート）へ進ませ、Request changes なら指摘対応の push を待つ時点に戻す。再レビューは既存の再レビュー処理（runbook 4 の `request-changes`）に従う
6. レビュー待ちの PR は、`pr-digest` の `review-wait` 行で滞留を見る（下の「レビュー待ちの滞留」）
7. 週次の巡回（前回の棚卸しから 7 日以上経った巡回）では、下の「週次: 長命 PR の棚卸し」も行う

### 週次: 長命 PR の棚卸し

前回の棚卸しから 7 日以上経った巡回で、開いている PR の棚卸しをする。前回の実施日は巡回記録で確かめる。

巡回記録とは、指揮役の作業ログ（`docs/worklogs/`）の「割り当ての記録」と「判断の記録」のこと。GitHub に置き場の無い判断（誰を割り当てたか、PR を待たせる理由と期限）を書く場所である。

- 判定には GitHub の `updated_at`（`scripts/agent-gh dev pr list --json number,updatedAt,state`）と巡回記録を並べて使う。GitHub 側だけ見ると、指揮役が待たせる理由を決めている PR を「進展が無い」と誤認し、巡回記録側だけ見るとコミットやレビューの動きを書き漏らすためである。
- 「進展」は次のいずれか。進展が一定期間（目安 7 日）無い PR を棚卸しの対象にする。
  - コミットの追加
  - レビューの投稿（Approve・Request changes）
  - レビューコメント
  - CI 状態の変化（`updated_at` に現れないことがあるので `scripts/agent-gh dev pr list` の `statusCheckRollup` で確かめる）
  - 巡回記録への状態注記（待たせる理由や期限を指揮役が書いたとき。注記した日を進展と数える）
- 対象にした PR ごとに、判断期限（次の週次巡回まで）付きで「残す」「作り直す」「閉じる」のどれかを決め、判断と期限を巡回記録の「判断の記録」に書く。
  - 残す: 理由（例: 依存する PR のマージ待ち）と次の判断期限を注記する
  - 作り直す: 実装担当に新しいブランチ・PR で作り直させる
  - 閉じる: PR を閉じて Issue を `ready-for-human` か `wontfix` で整理する（`docs/agents/triage-labels.md`）
  - PR のクローズは指揮役の「しないこと」（runbook 1）なので、「作り直す」「閉じる」の操作は人間に依頼する（PR に依頼のコメントを書き、部屋で人間に知らせる）
- 判断期限を過ぎても判断が残っている PR は、次の巡回で runbook 6 のエスカレーションにする。
- 7 日という期間は目安であり、運用実績で見直す。

## 2. 振り分け

材料は既にあるものだけを使う: pr-size の行数（追加と削除）、変更パス、ラベル、修正の回数（`round`）。下の値は現在の値で、改善ループ（手順 4）で調整する。値を変えたら、この表を直す。

### 実装のモデルと effort

| 条件 | モデル・effort |
|---|---|
| 定型・小規模（Issue の規模の見積もりが小、変更が文書・設定・テストだけ） | opencode `zai-coding-plan/glm-5.3`、variant `low` |
| 通常（仕様が固まった実装） | GLM と Devin の空いている方。同時に空いていれば Devin を優先する（`acpRegistry_devin`、`swe-2-high`、`thought_level=high`）。GLM なら variant 指定なし |
| 曖昧で長い調査・デバッグ系の Issue | opencode `zai-coding-plan/glm-5.3` |
| 「Devin のハーネスごと任せる」チケット（人間が Issue で明示するか、指揮役が「実装スレッドの枠組みでは進めにくい自律作業」と判断したもの。例: 環境構築を伴う検証、複数リポジトリにまたがる調査） | Devin（`acpRegistry_devin`、`swe-2-high`）。Issue に `agent:devin` を付ける |
| コア部分（`crates/pelway-contract`、承認・状態遷移・権限）、`unsafe` を含む・含みうる変更 | GLM なら variant `high`、Devin なら `thought_level=max` |
| 同じ Issue の修正の指示が 2 回を超えた（3 回目の修正に入る）。数えるのは、レビューの本物の must-fix に基づく指示だけ（分割計画が無い時期の誤判定や、再起動による自動の再開は数えない） | `t3_thread_configure` で variant または `thought_level` を 1 段上げる（GLM: `low`→通常→`high`→`max`。Devin: `medium`→`high`→`max`）。`max` でも直らなければ Devin の `swe-2-high`（`thought_level=max`）に替える。Issue に理由をコメントする |
| 同じ Issue の修正の指示が 3 回を超えた（4 回目の修正に入る）。数え方は上の行と同じ | 修正を出さずにエスカレーション（runbook 6） |

runbook と仕様の `round`（スレッドの実行回数）は、誤判定や再起動による再開でも増える。指揮役は `round` を目安にとどめ、effort の引き上げとエスカレーションは上の表の「修正の指示の回数」で判断する。

### レビュー担当と effort

| 条件 | レビュー担当 |
|---|---|
| 通常 | Codex（`codex_review`、`gpt-6-luna`）と Devin（`acpRegistry_devin`、`swe-2-high`）を交互に割り当てる |
| 高リスク: `unsafe`、セキュリティの境界（認証・権限・承認・秘密情報・ネットワーク）、`.github/workflows/` の変更、pr-size の追加行が 400 を超える | Codex・Claude（`claude_review`）・Devin の 3 人 |
| 抜き取り | 通常の PR の 4 本に 1 本は Claude も付ける（Codex・Devin の見落とし率の測定のため） |

- Devin が実装した PR には Devin のレビューを付けない。ローテーションで Devin の番が来ても Codex に回し、高リスクの PR でも Devin の枠は外す。
- Devin のレビュー担当は、`docs/dev-environment.md` の「レビュー担当の Devin」の確認が済むまで使わない。済むまでは通常は Codex、高リスクは Codex と Claude で回す。
- Devin のレビュースレッドでは、effort の値を `thought_level` に写す。
- Claude のレビュー担当は、`docs/dev-environment.md` の「レビュー担当の Claude」の確認が済むまで使わない。済むまでは Codex だけで回し、高リスクの PR は人間に知らせる。
- 段階移行: 抜き取りの実施が 10 件に達するか、抜き取りの開始から 4 週間が経つか、どちらか早い方になったら、初回の見落とし率を集計する（定義は手順 4 の「Codex・Devin の見落とし」）。その集計をもって高リスク PR の複数レビュー（Codex と Claude、確認済みなら Devin も）を有効化し、有効化したことと集計結果を指揮役の作業ログに記録する。複数体制は段階的に有効化し、まず抜き取りから始める。抜き取りの実施件数は「割り当ての記録」の表で数え、集計のときに作業ログに残す。
- effort は、追加行が 200 以下で高リスクでなければ既定、それ以外は `high`。同じ PR の 2 回目以降のレビューは 1 段上げる。
- 複数のレビュー担当を付けた PR で判定が割れたら（approve と request-changes が混ざったら）、マージせずに Issue にそれぞれの要点をコメントし、`needs-human` を付ける。

### レビュー待ちの滞留

レビュー待ちの滞留は、PR ごとに skill `pr-watch` の `pr-digest` を出し、`review-wait` 行（レビュー待ちの経過時間と起点の時刻）で機械的に検出する。起点は PR が最初にレビュー可能になった時刻で一意に決まり、レビュー担当の再割当では戻らない（形式は skill `pr-watch`）。

- 経過が目安の 24 時間を超えた PR の対応は、その時点の指揮役（正本の Grok Bot、代行中は代行者）が責任を持つ。責任の持ち方は、次のどちらかをその場で決めて実行することである。
  1. レビュー担当を割り当てる（上の「レビュー担当と effort」で決める。レビューの枠が埋まっているときは、その PR を順番待ちの先頭に置く）
  2. レビューの枠が当分空かない、割り当てたレビュー自体が止まっているときは、外部投入へエスカレーションする（下の「外部からのレビュースレッド投入」）
- 24 時間は目安であり、超えた時点で必ず上のどちらかを取る。何もしないまま放置しない。値は運用実績で見直す。

### 外部からのレビュースレッド投入

レビューの割り当てが滞留したとき、正本の指揮役の代わりに、別の指揮役・代行者・人間が許可した外部のエージェント（例: Muse）がレビュースレッドを投入してよい。場当たりに行わず、次の手順で行う。

1. 投入前に、その PR に既存のレビュー担当が居ないか確かめる。`t3 thread list` でその PR のブランチ（`review/<PR 番号>-`）のレビュースレッドを探し、`scripts/agent-gh dev pr view <PR 番号> --json reviewDecision,reviews` で投稿済み・進行中のレビューを確かめる。担当が居れば投入しない。担当が居るのに止まっているときは、正本の指揮役（居なければ人間）に知らせる。
2. 滞留の証拠（`pr-digest` の `review-wait` 行）を確かめてから、レビュースレッドを立てる。立て方は runbook の「レビューを始める」と同じ（skill `review-pr`、権限は `auto-accept-edits`、worktree は main から、スレッド名は「スレッド名」の形式）。
3. 投入した事実を巡回記録に残す。投入した者が PR に投入の旨（誰が・いつ・なぜ）をコメントし、指揮役は次の巡回でそれを指揮役の作業ログの「割り当ての記録」の表に 1 行足す（日時、PR、役割 review、投入した者、モデル、effort、理由は「レビュー滞留への外部投入」）。投入されたスレッドは `t3 thread list` で見えるので、レビューの同時数の上限（runbook 2）に数える。

### 調査の担当と effort（入口B）

入口B（原因が未理解のバグ・インシデント報告）で調査スレッドを立てるときの判断。どの報告が入口B かは、正本 `docs/superpowers/specs/agent-team.md` の「タスクの入口と流れ」。

- 担当は GLM（opencode `zai-coding-plan/glm-5.3`）。実装スレッドと合算して同時 3 本の枠で数える（正本の「制約」）。数え方は、`agent:working` が付いた Issue の数と実行中の調査スレッドの数の合計
- 枠が満杯のときは指揮役が優先度を判断する。原則として、本番インシデントの調査をほかの実装や調査より先に置く
- effort は、調査は読み取り中心のため既定。対象が広い（複数のクレートやサブシステムにまたがる等）場合は 1 段上げる
- 割り当ての記録の表には、役割 investigate で書く

### 調査の引き渡し（入口B）

調査スレッドが報告を出したときの処理。

- 報告は `docs/worklogs/bugs/<slug>/report.md` と、そのファイルだけの Draft PR（skill `investigate-bug` の手順 5）。PR とスレッドの返答の両方で報告を確認する
- 指揮役が報告を確認して起票可否を判断し、判断と理由を報告の PR にコメントする
- 直す必要があるときは、指揮役が skill `write-issue` で起票する（実装の指示は出さない）。人間は `ready-for-agent` で承認し、入口A の流れに合流する。Issue 本文の「関連情報」には、報告のパス（`docs/worklogs/bugs/<slug>/report.md`）と調査スレッドのリンクを書く
- 直す必要がないとき（仕様どおり、誤報告など）は Issue を立てない
- 起票の可否にかかわらず、報告の PR はレビューに回してマージする（報告が main に残る）。レビューの割り当ては通常の PR と同じ「レビュー担当と effort」で行い、Draft の解除は調査スレッドにさせる（実装スレッドと同じ流れ）
- 調査で報告に不足があって `needs-info` を付けたときは、情報が届いたら原則として同じ調査スレッドを再開する（`t3_thread_send` で「続きを調査する。skill investigate-bug に従う。」）。新しい調査スレッドは立てない
- 実装は、調査を担ったスレッドへの役割の追加としては行わない。起票した Issue の実装スレッドとして割り当て替える（正本の「役割と GitHub 上の名義」の役割の変更の原則）
- 調査スレッドは脆弱性の疑いで止まることがある（skill `investigate-bug` の手順 4）。このとき報告の PR は無いので、`needs-human` の判断を待つ

### マージ

`pelway-review` の Approve と、必須チェック（`mise run verify`、`pr-size`）の通過を確かめたら、指揮役が `gh pr merge --merge --admin --delete-branch` でマージする（`CODEOWNERS` の人間の Approve を、人間から委任された形で迂回する）。マージ以外の PR・Issue への操作（下の main の取り込みや base の変更、Draft の扱い、Issue のクローズ）は、先に「手当て・引き継ぎの前の確認」を通す。`--admin` は管理者の権限が要り App のトークンでは使えないので、このマージは人間の名義の `gh` で行う。目印付きのコメント（`<!-- pr-size-exception -->`、`<!-- split-plan -->`）も人間の名義の `gh issue comment` で書く。レビュー担当は作成者が `hatai` であることで、実装担当（`pelway-dev`）や第三者のコメントと区別する。それ以外の GitHub の操作（読み取り、ラベル、ふつうのコメント、`pr update-branch`）は `scripts/agent-gh dev` で行う。

- pr-size の失敗が許されるのは、指揮役が Issue に行頭が `<!-- pr-size-exception -->` で始まるコメントで超過を認めた理由を書き、PR に `pr-size-exception` ラベルを付けたときだけ（`scripts/agent-gh dev pr edit <M> --add-label pr-size-exception`）。レビュー担当（skill `review-pr`）はラベルとコメントの両方を確かめてから、pr-size の失敗を must-fix に数えない。レビューの指示に例外の文を足さない。
- 必須チェックの完了待ち（マージ前の CI 待ち、巡回での PR のベビーシット）には、ポーリングの代わりに skill `pr-watch` の `pr-digest --watch` をバックグラウンド実行して使う。ただし pr-watch の境界線のとおり、マージはこのスキルの外で人間の名義の `gh` で行う。
- skill `pr-watch` の `VERDICT green` は、その SHA でチェックが完走して通過していることの確認要素の 1 つ。マージの条件（`pelway-review` の Approve、必須チェック、レビュー済み SHA 以降の追加コミットの確認、main の取り込み、mergeable）を置き換えない。
- Approve の後に PR に push があったら、新しいコミットが main の取り込みだけであることを確かめる（`git rev-list <レビューした sha>..<head> --no-merges ^origin/main` が 0 件）。それ以外のコミットがあればレビューをやり直す。
- マージの直前に、PR が今の main を含んでいるかを確かめる（`git merge-base --is-ancestor origin/main origin/<PR のブランチ>`）。含んでいなければ、触るファイルが重ならなく見えても `scripts/agent-gh dev pr update-branch` で main を取り込み、CI が通ってからマージする（型や関数の変更は、ファイルが重ならなくてもビルドを壊す）。
- 別の PR に積んだ PR は、土台の PR をマージする前に base を main に替える（`scripts/agent-gh dev pr edit <後の PR> --base main`）。マージは `--delete-branch` で土台のブランチを消すので、base のままだと GitHub が後の PR を閉じ、閉じた PR は base を替えられない。土台のマージ後に `scripts/agent-gh dev pr update-branch` で main を取り込んで CI を起こす。
- 複数の PR を続けてマージするときは、`scripts/conductor-merge-queue.sh` に「PR 番号・ブランチ・レビューした sha」の組を並べて、1 本ずつ進める（main の取り込み、CI の待ち、レビュー後のコミットの確認、マージを順に行う）。このスクリプトを同時に 2 つ動かさない（同じリポジトリの `git fetch` がぶつかって両方止まる）。
- PR ができたら mergeable を確かめる（`scripts/agent-gh dev pr view <M> --json mergeable`）。`CONFLICTING` の PR には GitHub が `pull_request` の workflow を動かさず、チェックが 1 つも付かない。実装担当に「main を取り込んで衝突を解消し、verify を通す」を送る。
- マージしたら Issue から `agent:working` を外し、人間に報告する。
- Dependabot が立てた依存更新の PR（`dependencies` ラベル、`build(deps):` で始まるコミット）のうち、minor と patch は、通常の PR と同じレビュー（skill `review-pr`）とマージの決まりで扱う。
- Dependabot の major と 0.x の更新は直接処理しない。移行 Issue（更新の影響範囲と検証内容を書く。skill `write-issue`）を切って通常フロー（入口A）へ回し、Dependabot の PR 自体はクローズせず残す（クローズは人間の操作）。
- Action の更新の PR（`ci(deps):` で始まるコミット）では、Dependabot は `.github/workflows/actions.lock` を作り直さないため、指揮役がマージ前にその PR のブランチを checkout して `gh actions-lock` を実行し、生成し直した `actions.lock` を同じブランチに push してから CI の通過を確かめる（`gh actions-lock` は人間の名義の `gh` 拡張で、App のトークンでは動かせない）。この操作は、Workflows 権限が必要なために人間の名義で行う例外の 1 つで、例外の正本は `docs/superpowers/specs/agent-team.md` の「制約」。

### 分割を認めるとき

実装担当が pr-size の超過で止まり、分割案が追加行の境目できれいに分かれるときは、分割を認める。Issue に、行頭が `<!-- split-plan -->` で始まるコメントで、PR ごとの範囲と、その PR で確かめる受け入れ条件を表にして書く。受け入れ条件は skill `write-issue` の「受け入れ条件の書き方」に従う。レビュー担当（skill `review-pr`）は、この表でそれぞれの PR を判定する。後の PR は前の PR のブランチに積ませ、前の PR のマージの前に base を main に替える（上の「マージ」の積んだ PR の項）。

### 利用上限で止まったとき

実装スレッドやレビュースレッドが理由を残さずに `failed` になったら、`t3_thread_read`（`view: activity`）で error の項目を読む。「Usage limit reached」（GLM の 5 時間あたりの上限、Devin の無料モードの同時セッション上限など）なら、Issue の中身の失敗ではないので、runbook の「2 回目の `failed` でエスカレーション」に数えない。リセット時刻まで新しいスレッドを立てるのを止め、リセット後に小さな試験のスレッドで使えることを確かめてから、止まったスレッドに「続きをやり直す」を送る。

## 3. エスカレーション

runbook 6 の手順に従う。人間に上げる前に、手順 2 の「effort を上げる・モデルを替える」の段を必ず挟む。上げるときは、試したこと（effort、モデル、round）を Issue のコメントに書く。

## 4. 改善ループ

改善の対象は、指揮役のセッションだけでなく、実装担当（opencode＋GLM、Devin＋SWE-2）とレビュー担当（Codex、Devin、Claude）の両側。

### 材料を集める

| 材料 | 場所 |
|---|---|
| 実装者メモ（一次情報） | `docs/worklogs/issue-<N>.md` の「実装者メモ」、止まったときは Issue のコメント |
| 実装セッションの記録 | `t3 thread show <thread>`。圧縮のやり直し、ループ、誤読、取りこぼしを見る |
| レビューの指摘・差し戻し | PR の `pelway-review` のレビュー本文 |
| Codex・Devin の見落とし | 抜き取りで付けた Claude のレビューが、Codex・Devin の approve の後に must-fix を出したもの |
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

### 効果を測る

- 変更の効果は、Claude の挙動だけでなく GLM と SWE-2 の成果物で測る。基準は、同じ種類のレビュー指摘・差し戻しの減少。
- Claude では効いても GLM・SWE-2 で効かなかった文言は「共有スキルとして未達」とし、モデルの差を前提に書き直す（例: 抽象的な原則より、手順と具体例と禁止事項で書く）。
- レビュー担当ごとの見落とし率 = 抜き取りで Claude が must-fix を出した PR の数 ÷ 抜き取りの PR の数（Codex 担当分と Devin 担当分で分けて数える）。改善ループの材料にし、同時数の上限の見直しにも使う。

### 棚卸しする

- 改善のたびに、関係するスキル・文書の古い記述、矛盾する記述を探し、直すか消す。溜め込まない。
- 作業ログの改善点の表で、反映済み・見送りになったものに印を付ける。

## 5. 代行の開始と終了

### 開始のチェック

1. Grok Bot が動かないことを人間が確かめ、代行を頼んだ
2. T3 Connect の MCP につながる（レビュースレッドを `auto-accept-edits` で立てるために必要）
3. `scripts/conductor-watch.sh` を起動した
4. 引き継ぐ状態を読んだ: `agent:working` の Issue、Draft・レビュー待ちの PR、実行中のスレッド、`needs-human` の Issue、指揮役の作業ログ
5. 代行の作業ログ `docs/worklogs/<日付>-conductor-substitute.md` を作った（判断と改善点を書く）

### 終了のチェック（正本の指揮役へ戻す）

1. 新しいスレッドを立てるのを止める
2. 作業ログに、進行中の Issue・PR、待っているもの、未反映の改善点を書く
3. 作業ログを PR にしてマージする
4. 人間に、作業ログの場所と、戻した時点の状態を報告する
5. `scripts/conductor-watch.sh` を止める
