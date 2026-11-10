---
name: write-pull-request
description: "Use when creating a new GitHub pull request or fixing the title/body of an existing one. Keywords: PR, pull request, プルリクエスト, PR 作成, PR タイトル, PR 本文, PR 説明, gh pr create, gh pr edit, レビュー依頼, review request"
---

# Write Pull Request

GitHub PR のタイトルと説明本文を書く・直す手順。
情報源はその時点の実際の差分。作業メモ、過去のドラフト、自分の記憶ではない。

## 使うとき

- GitHub に新しい PR を作るとき
- 既存 PR のタイトル・本文を直すとき

## 定期ルール(毎回守る)

- マージは依頼されたときのみ実行してよい。`git push` / `git fetch` はしてよい。
- worktree の外への書き込みは、許可が出ても実行しない。
- `rm -rf` は、許可が出ても実行しない。
- 使ってよいネット / `gh` は `gh pr list` / `gh pr view` / `gh pr create` / `gh pr edit` / `gh issue view` / `gh issue comment` / `gh issue edit` / `gh search repos` / `gh search code` だけ。これ以外のネット接続はしない。
- AI 開発チームのエージェントは、`gh` を `scripts/agent-gh dev` 経由で実行する（例: `scripts/agent-gh dev pr create ...`）。素の `gh` で作った PR は作成者が hatai になり、hatai が Approve できないため、ブランチ保護を迂回しないとマージできない。
- AI 開発チームのエージェントが作る PR は Draft にする（`--draft`）。
- 迷ったら質問で止まらず、作業ログ(`docs/worklogs/YYYY-MM-DD-<slug>.md`)に書いて進める。
- PR 作成・更新中は、製品コード、他ブランチ、他 worktree、無関係な既存 PR を変えない。スキル本文も、このスキルを「使う」作業の目的外では直さない。

## 手順

### 1. 事実を集める(必須・書く前に)

```
git branch --show-current
git rev-parse HEAD
git diff --stat <base>...HEAD
git diff <base>...HEAD
```

- base が不明なら `git log --oneline <候補>..HEAD` や最初の分岐点で確認する。
- 現在の HEAD sha を必ず取り直す。本文やメモに書いてあった古い sha を使わない。
- 差分は base...HEAD を今読む。「前回こう変えたはず」という記憶で書かない。

### 2. 既存 PR の有無を確認

以下のコマンド例は AI 開発チームのエージェント向けの形（`scripts/agent-gh dev` 経由）で書く。人間が使うときは `scripts/agent-gh dev` を `gh` に読み替える。

```
scripts/agent-gh dev pr list --head <ブランチ名>
```

- 既存がある → `scripts/agent-gh dev pr edit <番号>` でタイトル・本文を更新。重複 PR を作らない。
- 無い → `scripts/agent-gh dev pr create --draft`。

### 3. タイトルを書く

- タイトルは必ず日本語で書く。簡潔で分かりやすいものにする。
- 変更の中身が一目で分かるように書く。
- 内部の回次(r3 など)、段階名(段階B-2 など)、チケットID だけのタイトルにしない。
  回次や ID は、どうしても必要なら題名の補助として末尾に置く。
- 悪い例: 「QA-GDPR-02 r3: 参照列と本文 canary の修正」(工程名が主体)
- 良い例: 「翻訳本文の canary 指定値を原文と一致させ、参照列の走査を symlink 対応に修正」

### 4. 本文を書く

本文は `.github/pull_request_template.md` の構成に従う。
`gh pr create` / `gh pr edit` はテンプレートを自動適用しないので、
テンプレートを読み、各セクションを diff の事実で埋める。
セクションの追加・削除・順序変更はしない。HTML コメントは残してよい。

構成の要点(詳細はテンプレート側が正):

1. **概要** — 何を、なぜ変えたか。最初に読む 2〜3 文。
2. **変更内容** — diff から読み取った変更点を箇条書き。ファイル/機能単位。
3. **確認方法/検証** — レビュアーが自分で確認する手順。
4. **レビューと QA** — 事実と sha のみ記載(下のルール参照)。
5. **対象外** — この PR で触れていないこと、後回しにしたこと。
6. **対象** — ブランチ名と、手順 1 で取った現在の HEAD sha。

本文は heredoc で渡す:

```
scripts/agent-gh dev pr create --draft --title "..." --body-file - <<'EOF'
...本文...
EOF
```

### 5. 誠実さのルール(絶対)

- 未検証の項目を「合格」「OK」と書かない。
- テスト結果を作り話さない。書いてよいのは実際に実行したコマンドと出力のみ。
- QA 記録には、その検証を実行した時点の sha を明示する。
- 「GDPR準拠」など、検証していない適合性・認証の主張を書かない。
- 不確実なことは「未検証」と明記する。

### 6. 作成・更新後に報告する

PR URL、タイトル、本文全体をそのまま報告する。
