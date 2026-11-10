---
name: review-pr
description: "Use when a T3 thread is told to review a pull request (\"PR #N をレビュー\") as the AI dev team's reviewer. Keywords: review PR, PR をレビュー, pelway-review, Approve, Request changes, must-fix, nit"
---

# Review PR

AI 開発チームのレビュー担当が、PR 1 本をレビューして GitHub に結果を投稿する手順。
運用全体の仕様は `docs/superpowers/specs/agent-team.md`。

## 決まり（毎回守る）

- ファイルを書き換えない。コミットしない。push しない。
- 実行してよいコマンドは `git` の読み取り系、PR の head を取り出す `scripts/agent-gh review pr checkout <N>`、`mise run verify`、`mise run pr-size`、`scripts/agent-gh review ...` だけ。
- GitHub への投稿は必ず `scripts/agent-gh review ...` で行う。投稿者が `pelway-review[bot]` になる。
- 判定は 2 種類だけ。must-fix が 1 つでもあれば Request changes、なければ Approve。

## 手順

### 1. 材料を集める

```bash
scripts/agent-gh review pr view <N> --json title,body,headRefName,headRefOid,isDraft,closingIssuesReferences
scripts/agent-gh review pr diff <N>
scripts/agent-gh review issue view <Issue 番号> --json body
```

Issue 番号は `closingIssuesReferences` から取る。Issue の「受け入れ条件」と「対象外」がレビューの基準になる。

### 2. 検査する

```bash
scripts/agent-gh review pr checkout <N>
mise run verify
mise run pr-size
```

### 3. 指摘を 2 種類に分ける

| 種類 | 当てはまるもの |
|---|---|
| must-fix | 受け入れ条件を満たしていない / 対象外のことをしている / テストが条件を確かめていない / verify か pr-size が失敗 / 正しさ・セキュリティの欠陥 |
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

## 検査
- mise run verify: pass | fail
- mise run pr-size: <行数> / 800
```

must-fix がなければ「なし」と書く。

```bash
scripts/agent-gh review pr review <N> --approve --body-file - <<'EOF'
...
EOF
# must-fix があるとき
scripts/agent-gh review pr review <N> --request-changes --body-file - <<'EOF'
...
EOF
```

### 5. Approve したら Ready for review にする

PR が Draft のままなら次を実行する。人間のマージ待ちに進む合図になる。

```bash
scripts/agent-gh review pr ready <N>
```

### 6. 終わりの報告

最後の返答は次の 3 行だけにする。Grok Bot はこの 3 行だけを読む。

```
pr: #<N>
verdict: approve | request-changes
must-fix: <件数>
```
