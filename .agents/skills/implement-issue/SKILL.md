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

## 止まる条件

| 条件 | Issue に書くこと |
|---|---|
| 名義の確認で `hatai/Pelway` が返らなかった | 返った内容かエラー |
| 受け入れ条件がテストで確かめられる形になっていない | どの条件がなぜ確かめられないか |
| 慣れない機能で、Issue の「採用する方式」が空 | `gh search` で比べた 2 つ以上の方式と、それぞれ採るもの・捨てるもの |
| `mise run pr-size` が 800 行を超える | 分割案（PR ごとの範囲と見込み行数）。行数を合わせるためにテスト・受け入れ条件の確認・説明を削らない。重複の除去で収まらなければ、その時点で止まる |
| 作業に `.github/workflows/` の変更が必要（`pelway-dev` には workflow を push する権限がない） | 必要な変更の内容。人間が行う |

止まり方（Issue のコメントには、手順 5 の実装者メモも続けて貼る）:

```bash
scripts/agent-gh dev issue comment <N> --body-file - <<'EOF'
（上の表の内容）

（手順 5 の実装者メモ）
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
- `mise run pr-size` はコミット済みの差分だけを数える。必ずコミットの後、push の前に実行する。800 行を超えたら push せずに止まる。

### 7. Draft PR を作る（初回だけ）

`write-pull-request` skill に従って本文を書き、Draft で作る。本文の「概要」の最後に `Closes #<N>` を入れる。

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

### 8. レビューで Approve されたら Ready for review にする

Grok Bot から「Approve された」と届いたら、次を実行する。レビュー担当の App には Draft を解除する権限がないため、実装担当が行う。

```bash
scripts/agent-gh dev pr ready <PR 番号>
```

### 9. 終わりの報告

最後の返答は次の 5 行だけにする。Grok Bot はこの 5 行だけを読む。

```
issue: #<N>
pr: <PR の URL>
verify: pass | fail
pr-size: <数えた行数> / 800
memo: docs/worklogs/issue-<N>.md（round <R>）
```

止まった場合は `pr:` の代わりに `stopped: <止まる条件の表の 1 列目>` を書く。

## よくある失敗

| 失敗 | 正しくは |
|---|---|
| レビューの nit まで先回りして直す | 修正依頼に書かれた must-fix だけ直す。nit は指示があったときだけ |
| テストを実行せずに「TDD で実装した」と書く | 失敗した出力と成功した出力を両方貼る |
| 修正のたびに新しいブランチを作る | 同じブランチに積む |
| 素の `gh pr create` を使う | `scripts/agent-gh dev pr create --draft` |
| 実装者メモに「特になし」や一般論を書く | 迷った文を「」で引用し、置いた仮定と選んだ解釈を書く |
| pr-size を合わせるためにテストや説明を削る | 重複の除去で収まらなければ止まり、分割案と実装者メモを Issue に書く |
