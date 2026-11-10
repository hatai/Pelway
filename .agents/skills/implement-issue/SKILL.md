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
| `mise run pr-size` が 800 行を超える | 分割案（PR ごとの範囲と見込み行数） |
| 作業に `.github/workflows/` の変更が必要（`pelway-dev` には workflow を push する権限がない） | 必要な変更の内容。人間が行う |

止まり方:

```bash
scripts/agent-gh dev issue comment <N> --body-file - <<'EOF'
（上の表の内容）
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

### 5. 検査してコミットし、行数を確かめてから push する

```bash
mise run verify
scripts/agent-gh dev git add <files>
scripts/agent-gh dev git commit -m "<type>: <英語の要約>"
scripts/agent-gh dev git fetch origin main
mise run pr-size
scripts/agent-gh dev git push -u origin HEAD
```

- `mise run verify` が失敗したらコミットしない。直してからやり直す。
- `mise run pr-size` はコミット済みの差分だけを数える。必ずコミットの後、push の前に実行する。800 行を超えたら push せずに止まる。

### 6. Draft PR を作る（初回だけ）

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

### 7. レビューで Approve されたら Ready for review にする

Grok Bot から「Approve された」と届いたら、次を実行する。レビュー担当の App には Draft を解除する権限がないため、実装担当が行う。

```bash
scripts/agent-gh dev pr ready <PR 番号>
```

### 8. 終わりの報告

最後の返答は次の 4 行だけにする。Grok Bot はこの 4 行だけを読む。

```
issue: #<N>
pr: <PR の URL>
verify: pass | fail
pr-size: <数えた行数> / 800
```

止まった場合は `pr:` の代わりに `stopped: <止まる条件の表の 1 列目>` を書く。

## よくある失敗

| 失敗 | 正しくは |
|---|---|
| レビューの nit まで先回りして直す | 修正依頼に書かれた must-fix だけ直す。nit は指示があったときだけ |
| テストを実行せずに「TDD で実装した」と書く | 失敗した出力と成功した出力を両方貼る |
| 修正のたびに新しいブランチを作る | 同じブランチに積む |
| 素の `gh pr create` を使う | `scripts/agent-gh dev pr create --draft` |
