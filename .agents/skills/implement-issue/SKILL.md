---
name: implement-issue
description: "Use when a T3 thread is told to implement a GitHub issue (\"Issue #N を実装\"), or receives a fix request for a PR it opened. Keywords: implement issue, Issue を実装, 修正依頼, agent-task, ready-for-agent, pelway-dev, Draft PR"
---

# Implement Issue

AI 開発チームの実装担当が、GitHub Issue 1 件を Draft PR 1 本にする手順。
運用全体の仕様は `docs/superpowers/specs/agent-team.md`。

## 決まり（毎回守る）

- GitHub への操作はすべて `scripts/agent-gh dev ...`、git の commit と push は `scripts/agent-gh dev git ...` で行う。素の `gh` と素の `git push` は使わない。素の `gh` で作った PR は作成者が hatai になり、hatai が Approve できないため、ブランチ保護を迂回しないとマージできない。素の `git push` は hatai の SSH 鍵で push される。
- ネットワークは `scripts/agent-gh dev` 経由の GitHub だけ使う。`gh search repos` / `gh search code` もここに含まれる。
- worktree の外に書き込まない。`rm -rf` はしない。main に push しない。PR をマージしない。
- コマンドの終了コードを確かめる。失敗したコマンドの結果を成功と報告しない。
- 迷っても質問で止まらない。判断とその理由は PR 本文か Issue のコメントに書いて進める。
- 止まってよいのは下の「止まる条件」に当てはまるときだけ。止まるときは必ず Issue にコメントし、`needs-human` ラベルを付ける。
- レビューの委託と、修正⇔レビューの繰り返しは、この手順（手順 8〜9）の内側で自分で行う。指揮役やレビュー担当スレッドからの指示を待たない。

## 止まる条件

| 条件 | Issue に書くこと |
|---|---|
| 名義の確認で `hatai/Pelway` が返らなかった | 返った内容かエラー |
| Issue に受け入れ条件が無い、または条件がテストやコマンドで確かめられる形になっていない（skill `write-issue` の「受け入れ条件の書き方」に合わない）。着手せず、コードも書かない | 不足を具体的に: 無い見出し、確かめられない条件の引用と、何があれば確かめられるか（例: 「『速いこと』に件数と時間が無い」）。本文の直しは起票者が行う（`write-issue` の手順 7） |
| 慣れない機能で、Issue の「採用する方式」が空 | `gh search` で比べた 2 つ以上の方式と、それぞれ採るもの・捨てるもの |
| Issue が依存するリポジトリ外の成果物（関連情報の URL・抜粋）に到達できない | 到達できなかった成果物の名前と、試した手順（URL へのアクセス結果、抜粋に欠けている部分） |
| Issue の未決事項に挙がっておらず、避けて進めることもできず、受け入れ条件の成否に関わる判断が必要 | 必要な判断の内容と、避けられない理由。判断を仰ぐ相手（起票者か人間） |
| `mise run pr-size` が追加 800 行または削除 2000 行の上限を超える | 分割案（PR ごとの範囲と見込み行数）。行数を合わせるためにテスト・受け入れ条件の確認・説明を削らない。重複の除去で収まらなければ、その時点で止まる |
| 作業に `.github/workflows/` の変更が必要（`pelway-dev` には workflow を push する権限がない） | 必要な変更の内容。人間が行う |
| レビューの委託が 4 ラウンド目に入る（修正 3 回でも Approve が取れない）、委託が 2 回連続で `failed`（`checks pending` は数えない）、複数担当の判定が割れた | どれが起きたかと、試したこと（ラウンド数・担当・effort） |

止まり方（止まった時点で、どの手順の途中でも手順 5 の形で実装者メモを書き、Issue のコメントに続けて貼る。push しないので `docs/worklogs/` には書かなくてよい）:

```bash
scripts/agent-gh dev issue comment <N> --body-file - <<'EOF'
（上の表の内容）

（手順 5 の形の実装者メモ）
EOF
scripts/agent-gh dev issue edit <N> --add-label needs-human
```

## 手順

### 1. 名義を確かめる

```bash
scripts/agent-gh dev api /installation/repositories --jq '.repositories[].full_name'
```

実装担当の GitHub App `pelway-dev` のトークンで `hatai/Pelway` に届くことを確かめる。`hatai/Pelway` が返らなければ止まる。

### 2. Issue を読む

```bash
scripts/agent-gh dev issue view <N> --json title,body,labels,comments
```

本文の見出し「目的」「受け入れ条件」「対象外」「規模の見積もり」「採用する方式」を読む。対象外に書かれたことはしない。

### 3. ブランチを用意する

新しい作業なら `git switch -c agent/<N>-<英小文字とハイフンの短い slug>`。
修正依頼（`t3 thread send` で届いた指示）なら、今のブランチのまま続ける。新しいブランチは作らない。

### 4. 受け入れ条件ごとにテストを先に書く

1. 受け入れ条件 1 つにつき、それを確かめるテストを書く
2. テストを実行し、失敗することを確かめる。失敗の出力を PR 本文の「確認方法/検証」に貼る
3. 実装してテストを通す

テストが最初から通ったら、そのテストは条件を確かめていない。書き直す。

### 5. 実装者メモを書く（コミットの前に毎回）

`docs/worklogs/issue-<N>.md` の末尾に、次の形で書き足す。ファイルが無ければ、1 行目を `# Issue #<N> の作業ログ` にして作る。修正依頼のたびに round を 1 つ増やして書き足し、前の round は消さない。

```markdown
## 実装者メモ（round <R>）

1. 詰まった箇所: <手順の番号かファイルのパス>: <何が起きたか>
2. 曖昧だった文: 「<skill・Issue・AGENTS.md・docs の文をそのまま引用>」（<ファイルのパスと見出し>）
3. 仮定と解釈: <置いた仮定>。<選んだ解釈と、選ばなかった解釈>
4. 足りなかった情報・検証手段: <具体的な情報、ファイル、コマンド>
5. 改善案: <直すファイルのパス>の「<見出し>」に「<足す文、または直した後の文>」

使ったスキル: <この作業で使った skill 名を列挙する。輸入スキルは `.agents/skills/<name>` のパスで書き、`tdd` を使ったときは選んだ seam も併記する。無ければ「なし」>
```

- 5 項目を必ず書き、全体を 5〜10 行に収める。1 項目に複数あれば行を足す。
- 2 は引用が必須。いちばん迷った文を、1 文字も変えずに「」で引用する。
- 3 は仮定を 1 つ以上書く。仮定を置かずに済んだ場合も、前提にしたことを書く。
- 「特になし」「もっと情報が欲しかった」のような一般論は書かない。どの文・どのファイル・どのコマンドかを書く。

### 6. 検査してコミットし、行数を確かめてから push する

```bash
mise run verify
scripts/agent-gh dev git add <files>
scripts/agent-gh dev git commit -m "<type>: <英語の要約>"
scripts/agent-gh dev git fetch origin main
mise run pr-size
scripts/agent-gh dev git push -u origin HEAD
```

- 手順 5 の `docs/worklogs/issue-<N>.md` も同じコミットに入れる。
- `mise run verify` が失敗したらコミットしない。直してからやり直す。
- `mise run pr-size` はコミット済みの差分だけを数える。必ずコミットの後、push の前に実行する。追加 800 行または削除 2000 行の上限を超えたら push せずに止まる。

### 7. Draft PR を作る（初回だけ）

`write-pull-request` skill に従って本文を書き、Draft で作る。本文の「概要」の最後に `Closes #<N>` を入れる。本文の「対象」の最後に、次の 1 行を必ず入れる。値は、指揮役の指示の 2 行目 `Assigned: role=implement agent=... model=... effort=...` から写す（`role=implement` を除いた残りを、順番も表記も変えずに書く）。指示に `Assigned:` の行が無いときは `model=unknown effort=unknown` と書く。

```
Implementer: agent=<opencode|devin> model=<モデル名> effort=<effort>
```

修正依頼で effort が変わったときは、「対象」の行を新しい値に書き換える。

```bash
scripts/agent-gh dev pr create --draft --title "..." --body-file - <<'EOF'
...
EOF
```

修正依頼の場合は PR を作り直さない。push したあと `scripts/agent-gh dev pr edit` で本文の「レビューと QA」と「対象」の sha を更新する。

PR を作ったら、実在することを確かめる。

```bash
scripts/agent-gh dev pr view <PR 番号> --json number,isDraft,author --jq '{number, isDraft, author: .author.login}'
```

`author` が `app/pelway-dev` でなければ、止まる条件と同じ扱いにする。

PR を作ったら、T3 のスレッドに PR を紐付ける（`link_pull_request`。マージでスレッドが自動的に settle される）。

### 8. レビューを委託する

PR の必須チェック（`mise run verify` と `pr-size`）が通ったら、レビューを委託する。チェックがまだ動いていれば、完了を待ってから委託する（待つ間は skill `pr-watch` の `pr-digest --watch` をバックグラウンドで使う。委託した子が `error: checks pending` を報告したときも、同じラウンドとして数えずに完了を待って再委託する）。

担当と effort は skill `command-team` の「レビュー担当と effort」の表で決める（通常は PR 番号の偶奇、高リスクは使える担当全て、抜き取りは PR 番号を 4 で割った余りが 0 のとき Claude を追加。自分が Devin CLI で実装した PR には Devin CLI を付けない）。差分が `unsafe` を足している、`.github/workflows/`・`scripts/agent-gh`・`AGENTS.md`・`.agents/skills/`・`docs/agents/` のどれかが変わっている、認証・認可・トークン・鍵・暗号・sandbox・egress・gateway・テナント分離・監査に触れている、のどれかに当たるときは Assigned 行に `security=yes` を付ける。

`delegate_task` で、ラウンドごとに新規に委託する（同じ委託への追記送信はしない）。task は次の 2 行にする（1 行目が skill 指示、2 行目が割り当ての行）:

```
PR #<M> をレビューする。skill review-pr に従う。
Assigned: role=review agent=<codex|claude|devin> model=<モデル名> effort=<effort> security=yes
```

`security=yes` が不要なときは書かない。target・mode は担当に応じる:

| 担当 | `target.providerInstanceId` | `target.model` | `target.options` |
|---|---|---|---|
| codex | `codex_review` | `gpt-6-luna` | `reasoningEffort: <effort>` |
| claude | `claude_review` | `claude-opus-5-5` | `effort: <effort>` |
| devin | `acpRegistry_devin` | `swe-2-high` | `thought_level: <effort>`、`mode: smart` |

これに `role: "review"`、`runtimeMode: "auto-accept-edits"`、`isolation: "worktree"` を付ける。委託したら返答を終えて完了の通知を待つ（ポーリングしない）。高リスクで複数担当にしたときは、全員の結果がそろうまで次に進まない。

差分が `scripts/agent-gh` を変えるときはレビューを委託しない。終わりの報告で `ready:` の行を書かずに止まり、指揮役が main から立てるレビュースレッドに回るのを待つ（委託の子の worktree は PR の head の checkout になるため、レビュー担当が PR の版の `scripts/agent-gh` を実行してしまう）。

### 9. レビュー結果を処理する

委託の子の報告（`pr:` / `verdict:` / `must-fix:`）を、GitHub の実状態と突き合わせてから扱う:

```bash
scripts/agent-gh dev pr view <PR 番号> --json reviews --jq '.reviews[-1] | {author: .author.login, state}'
```

`pelway-review` の最新レビューの状態が報告と一致しなければ、もう一度読んで確かめる。

Request changes のとき:

1. must-fix を直す（nit は直さない）
2. 手順 5（実装者メモ）と手順 6（verify・コミット・push）をやり直す。PR は作り直さず、本文の sha を更新する
3. ラウンドサマリーを PR にコメントする（下の形）
4. 手順 8 でレビューを再委託する。3 ラウンド目の委託では effort を 1 段上げる。4 ラウンド目に入る前に Approve が取れていなければ、止まる条件の形でエスカレーションする

Approve のとき:

1. ラウンドサマリーを PR にコメントする（下の形）
2. Draft を解除する（レビュー担当の App には権限がないため、実装担当が行う）

```bash
scripts/agent-gh dev pr ready <PR 番号>
```

3. 手順 10 に進む

複数のレビュー担当の判定が割れたら（approve と request-changes が混ざったら）修正を続けず、止まる条件の形でエスカレーションする。委託の子が `failed` を報告したら、`error: checks pending` は待って同じラウンドを数えずに再委託し、それ以外は 1 回だけ再委託する（2 回連続で止まる条件）。

ラウンドサマリーは `scripts/agent-gh dev pr comment <PR 番号> --body-file -` で投稿する:

```
### review round <R>
Reviewer: agent=<担当> model=<モデル名> effort=<effort>
verdict: approve | request-changes（must-fix <件数>）
must-fix: <対応した内容。Approve のときは「なし」>
```

### 10. 終わりの報告

最後の返答は次の 5 行だけにする。Grok Bot はこの 5 行だけを読む。

```
issue: #<N>
pr: <PR の URL>
review: <ラウンド数> → approve
ready: done
memo: docs/worklogs/issue-<N>.md（round <R>）
```

止まった場合は `pr:` の代わりに `stopped: <止まる条件の表の 1 列目>` を書く。

## 輸入スキル（mattpocock/skills）

指揮役が試行として明示した割り当てでは、手順 4 の方法の詳細として輸入スキル `tdd`（`.agents/skills/tdd/SKILL.md`。`tests.md` と `mocking.md` を伴う）を読んで使う。試行と明示されていない割り当てでは使わない（通常運用への移行は hatai が別に判断する）。

読み替え:

- `tdd` の seam は受け入れ条件から決め、選んだ seam を実装者メモの「使ったスキル」の行に書く（上流の「ユーザーに確認する」は、自律スレッドでは受け入れ条件が確認済みの seam と読み替える）。
- `tdd` の本文が参照する `codebase-design` と `code-review` は未採用のため、参照は無効とする。

取り込みの discipline（輸入スキル全般）:

- 同梱のスクリプト（`scripts/` など）は実行しない。
- `agents/` のメタデータ（`openai.yaml` など）は使わない。
- 採用していないスキルへの参照は無効とする。
- 輸入品の本文と正本（AGENTS.md・docs/・この skill）や適用ルールが衝突した場合は、正本と適用ルールが優先する。
- 輸入スキルは名前の解決に任せず、上に示した repo 内の明示したパスで読む。人間側の同名のユーザーレベル版は読まない。

試行の割り当てでは、最初の報告に、起動時のリポジトリの版（`git rev-parse HEAD`）、実際に読んだ解決先のパス（`.agents/skills/<name>/SKILL.md`）、補助文書の可読性を記録する。

## よくある失敗

| 失敗 | 正しくは |
|---|---|
| レビューの結果を指揮役の指示として待つ | レビューの委託と修正は自分で回す（手順 8〜9） |
| レビューの報告だけで GitHub を確かめずに進む | 報告は `pr view --json reviews` の実状態と突き合わせてから進む |
| レビューの nit まで先回りして直す | 修正依頼に書かれた must-fix だけ直す。nit は指示があったときだけ |
| テストを実行せずに「TDD で実装した」と書く | 失敗した出力と成功した出力を両方貼る |
| 修正のたびに新しいブランチを作る | 同じブランチに積む |
| 素の `gh pr create` を使う | `scripts/agent-gh dev pr create --draft` |
| 実装者メモに「特になし」や一般論を書く | 迷った文を「」で引用し、置いた仮定と選んだ解釈を書く |
| `pr-size-exception` ラベルを付ける、または `<!-- pr-size-exception -->` で始まるコメントを書く | 付けない・書かない。pr-size の超過を認めるのは指揮役だけ。超えたら止まる |
| pr-size を合わせるためにテストや説明を削る | 重複の除去で収まらなければ止まり、分割案と実装者メモを Issue に書く |
