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
- スレッドに送る指示は、Issue 番号か PR 番号と skill 名だけにする。例外を伝える必要があるとき（指揮役が認めた pr-size の超過など）は、Issue にコメントしてから、そのコメントを参照させる一文だけ足す。
- 判断はすべて Issue か PR にコメントで残す。セッションの文脈だけに置かない。
- 見張りの状態は `scripts/conductor-watch.sh` が `~/.local/state/pelway/` に置く。セッションの一時フォルダに置かない。

## 1. サイクルの始め

1. `scripts/conductor-watch.sh` が動いていることを確かめる（動いていなければ起動する）
2. `t3 thread list --json` と `gh pr list` で、実行中のスレッド、Draft の PR、レビュー待ちの PR を数える
3. runbook の「同時数と順番待ち」の順で、次の一手を 1 つずつ決める

## 2. 振り分け

材料は既にあるものだけを使う: pr-size の行数（追加と削除）、変更パス、ラベル、修正の回数（`round`）。下の値は現在の値で、改善ループ（手順 4）で調整する。値を変えたら、この表を直す。

### 実装のモデルと effort

| 条件 | モデル・effort |
|---|---|
| 定型・小規模（Issue の規模の見積もりが小、変更が文書・設定・テストだけ） | opencode `zai-coding-plan/glm-5.3`、variant `low` |
| 通常 | opencode `zai-coding-plan/glm-5.3`（variant 指定なし） |
| コア部分（`crates/pelway-contract`、承認・状態遷移・権限）、`unsafe` を含む・含みうる変更 | opencode `zai-coding-plan/glm-5.3`、variant `high` |
| 同じ Issue の `round` が 2 を超えた（3 回目の修正に入る） | `t3_thread_configure` で variant を 1 段上げる（`low`→通常→`high`→`max`）。`max` でも直らなければモデルを替える。Issue に理由をコメントする |
| `round` が 4 以上 | 修正を出さずにエスカレーション（runbook 6） |

### レビュー担当と effort

| 条件 | レビュー担当 |
|---|---|
| 通常 | Codex（`codex_review`、`gpt-6-luna`） |
| 高リスク: `unsafe`、セキュリティの境界（認証・権限・承認・秘密情報・ネットワーク）、`.github/workflows/` の変更、pr-size の追加行が 400 を超える | Codex と Claude（`claude_review`）の両方 |
| 抜き取り | 通常の PR の 4 本に 1 本は Claude も付ける（見落とし率の測定のため） |

- Claude のレビュー担当は、`docs/dev-environment.md` の「レビュー担当の Claude」の確認が済むまで使わない。済むまでは Codex だけで回し、高リスクの PR は人間に知らせる。
- effort は、追加行が 200 以下で高リスクでなければ既定、それ以外は `high`。同じ PR の 2 回目以降のレビューは 1 段上げる。
- Codex と Claude の判定が割れたら（片方が approve、片方が request-changes）、マージせずに Issue に両方の要点をコメントし、`needs-human` を付ける。

### マージ

`pelway-review` の Approve と、必須チェック（`mise run verify`、`pr-size`）の通過を確かめたら、指揮役が `gh pr merge --merge --admin --delete-branch` でマージする（`CODEOWNERS` の人間の Approve を、人間から委任された形で迂回する）。

- pr-size の失敗が許されるのは、指揮役が Issue にコメントで超過を認めたときだけ。
- Approve の後に PR に push があったら、新しいコミットが main の取り込みだけであることを確かめる（`git rev-list <レビューした sha>..<head> --no-merges ^origin/main` が 0 件）。それ以外のコミットがあればレビューをやり直す。
- 同じファイルを触る PR が続くときは、先にマージした後、残りの PR に `scripts/agent-gh dev pr update-branch` で main を取り込み、CI が通ってからマージする。
- 別の PR に積んだ PR は、土台の PR をマージした後に base を main に替える。base を替えただけでは CI が動かないので、`scripts/agent-gh dev pr update-branch` で main を取り込んで CI を起こす。
- マージしたら Issue から `agent:working` を外し、人間に報告する。

## 3. エスカレーション

runbook 6 の手順に従う。人間に上げる前に、手順 2 の「effort を上げる・モデルを替える」の段を必ず挟む。上げるときは、試したこと（effort、モデル、round）を Issue のコメントに書く。

## 4. 改善ループ

改善の対象は、指揮役のセッションだけでなく、実装担当（opencode＋GLM）とレビュー担当（Codex、Claude）の両側。

### 材料を集める

| 材料 | 場所 |
|---|---|
| 実装者メモ（一次情報） | `docs/worklogs/issue-<N>.md` の「実装者メモ」、止まったときは Issue のコメント |
| 実装セッションの記録 | `t3 thread show <thread>`。圧縮のやり直し、ループ、誤読、取りこぼしを見る |
| レビューの指摘・差し戻し | PR の `pelway-review` のレビュー本文 |
| Codex の見落とし | 抜き取りで付けた Claude のレビューが、Codex の approve の後に must-fix を出したもの |
| 指揮役の学び | 指揮役の作業ログ（`docs/worklogs/`） |

### 裏取りする

- 実装者メモは自己申告で、本人が気づいていない誤解は書かれない。レビューの指摘とセッションの記録で裏を取る。食い違ったら、実際の成果物とセッションの記録を優先する。
- レビューで同じ種類の指摘が 2 回以上出たら、実装側の指示・スキルの不足として扱う。
- レビュー担当の指摘が head の版のファイルと合わないときは、レビュー担当側の材料として扱う。

### 共有層に反映する

- 書き込み先は両方のツールが読む場所だけ: `AGENTS.md`、`.agents/skills/`、`docs/`。
- `.claude/skills/<name>` は `.agents/skills/<name>` へのシンボリックリンクにする。変更したら `diff -r .claude/skills/<name> .agents/skills/<name>` の差分がゼロであることを確かめてから完了とする。
- 変更は PR にして、レビュー担当のスレッドに回す。

### 効果を測る

- 変更の効果は、Claude の挙動だけでなく GLM の成果物で測る。基準は、同じ種類のレビュー指摘・差し戻しの減少。
- Claude では効いても GLM で効かなかった文言は「共有スキルとして未達」とし、モデルの差を前提に書き直す（例: 抽象的な原則より、手順と具体例と禁止事項で書く）。
- Codex の見落とし率 = 抜き取りで Claude が must-fix を出した PR の数 ÷ 抜き取りの PR の数。改善ループの材料にし、同時数の上限の見直しにも使う。

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
