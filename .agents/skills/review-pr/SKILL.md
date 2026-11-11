---
name: review-pr
description: "Use when a T3 thread is told to review a pull request (\"PR #N をレビュー\") as the AI dev team's reviewer. Keywords: review PR, PR をレビュー, pelway-review, Approve, Request changes, must-fix, nit"
---

# Review PR

AI 開発チームのレビュー担当が、PR 1 本をレビューして GitHub に結果を投稿する手順。
運用全体の仕様は `docs/superpowers/specs/agent-team.md`。

## 決まり（毎回守る）

- PR のコードを実行しない（`mise run`、`cargo`、テスト、PR の中のスクリプトを含む）。PR のコードはレビュー担当の秘密鍵を読めるため。verify と pr-size は PR のチェック（CI）の結果で判断する。
- ファイルを書き換えない。コミットしない。push しない。PR の head を worktree に取り出さない（`git checkout`・`git switch` をしない）。取り出すと worktree の `scripts/agent-gh` が PR の版に置き換わり、それを実行することになるため。
- 実行してよいコマンドは `git` の読み取り系（`git show`・`git diff`・`git log`・`git grep`）、`scripts/agent-gh review ...` だけ。
- GitHub との通信は `scripts/agent-gh review ...` だけで行う。素の `git fetch` や `gh pr checkout` は SSH で接続するため、ネットワークの制限で通らない。
- 自分の worktree の外でコマンドを実行しない。PR の head を取ってこられないときは、他の worktree で代わりに検査せず、`verdict: failed` で報告する。
- `pr ready` は実行しない（レビュー担当の App には権限がない。Ready for review への切り替えは実装担当の App が行う）。
- 投稿系のコマンドは `&&` でつながず 1 つずつ実行し、終了コードを確かめる。失敗したら `verdict: failed` で報告する。成功していない投稿を成功と報告しない。
- GitHub への投稿は必ず `scripts/agent-gh review ...` で行う。投稿者が `pelway-review[bot]` になる。
- 判定は 2 種類だけ。must-fix が 1 つでもあれば Request changes、なければ Approve。

## 手順

### 1. 材料を集める

```bash
scripts/agent-gh review pr view <N> --json title,body,author,headRefName,headRefOid,isDraft,closingIssuesReferences
scripts/agent-gh review pr diff <N>
scripts/agent-gh review issue view <Issue 番号> --json body
```

Issue 番号は `closingIssuesReferences` から取る。Issue の「受け入れ条件」と「対象外」がレビューの基準になる。

### 2. 検査する

PR の head を取ってきて、worktree には取り出さずに読む。

```bash
scripts/agent-gh review git fetch origin pull/<N>/head
git rev-parse FETCH_HEAD
git show FETCH_HEAD:<パス>
git grep <語> FETCH_HEAD
```

`git rev-parse FETCH_HEAD` が 1 の `headRefOid` と一致しなければ、`verdict: failed` で報告する。

PR のチェックが終わるのを待ち、結果を読む。

```bash
scripts/agent-gh review pr checks <N> --required --watch --interval 30
scripts/agent-gh review pr checks <N> --required --json name,bucket
```

`mise run verify` と `pr-size` の `bucket` が `pass` なら pass、`fail` なら fail。どちらかが `pending` のまま終わったとき、または見つからないときは、`verdict: failed` で報告する。

### 3. 指摘を 2 種類に分ける

| 種類 | 当てはまるもの |
|---|---|
| must-fix | 受け入れ条件を満たしていない / 対象外のことをしている / テストが条件を確かめていない / verify か pr-size が失敗 / 正しさ・セキュリティの欠陥 / ブランチが `agent/` で始まるのに PR の作成者（`author.login`）が `app/pelway-dev` でない |
| nit | 上に当てはまらない改善案 |

### 4. 投稿する

本文は次の形にする。

```
Verdict: approve | request-changes
Reviewed: <headRefOid>

## must-fix
- <file>:<line> <何がどう問題か。直し方>

## nit
- <file>:<line> <提案>

## 検査（PR のチェック）
- mise run verify: pass | fail
- pr-size: pass | fail
```

must-fix がなければ「なし」と書く。

PR が Draft のままでも投稿する。

```bash
scripts/agent-gh review pr review <N> --approve --body-file - <<'EOF'
...
EOF
```

must-fix があるときは Request changes にする。

```bash
scripts/agent-gh review pr review <N> --request-changes --body-file - <<'EOF'
...
EOF
```

投稿したら、PR に自分のレビューが付いたことを確かめる。

```bash
scripts/agent-gh review pr view <N> --json reviews --jq '.reviews[-1] | {author: .author.login, state}'
```

`state` が `APPROVED` か `CHANGES_REQUESTED` で、投稿した判定と一致していれば成功。そうでなければ `verdict: failed` で報告する。

### 5. 終わりの報告

最後の返答は次の 3 行だけにする。Grok Bot はこの 3 行だけを読む。

```
pr: #<N>
verdict: approve | request-changes | failed
must-fix: <件数>
```

`failed` のときは 4 行目に `error: <失敗したコマンドとエラーの 1 行>` を足す。
