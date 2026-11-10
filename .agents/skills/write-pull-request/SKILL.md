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

マージはしない。`git push` / `git fetch` もしない(担当が行う)。

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
  (main 取り込みで head が進み、本文の sha が実態とずれた PR が実際にある)
- 差分は base...HEAD を今読む。「前回こう変えたはず」という記憶で書かない。

### 2. remote にブランチがあるか確認

`git branch -r --list origin/<ブランチ名>` などで確認(fetch はしない)。
remote に無ければ PR を作らず、その場で止めて報告する
(「push が必要」を伝え、push は自分で行わない)。

### 3. 既存 PR の有無を確認

```
gh pr list --head <ブランチ名>
```

- 既存がある → `gh pr edit <番号>` でタイトル・本文を更新。重複 PR を作らない。
- 無い → `gh pr create` 。

### 4. タイトルを書く

- 変更の中身が分かる日本語で書く。
- 内部の回次(r3 など)、段階名(段階B-2 など)、チケットID だけのタイトルにしない。
  回次や ID は、どうしても必要なら題名の補助として末尾に置く。
- 悪い例: 「QA-GDPR-02 r3: 参照列と本文 canary の修正」(工程名が主体)
- 良い例: 「翻訳本文の canary 指定値を原文と一致させ、参照列の走査を symlink 対応に修正」

### 5. 本文を書く

推奨構成(この順で):

1. **概要** — 何を、なぜ変えたか。最初に読む 2〜3 文。
2. **変更内容** — diff から読み取った変更点を箇条書き。ファイル/機能単位。
3. **確認方法/検証** — レビュアーが自分で確認する手順。
4. **レビューと QA** — 事実と sha のみ記載(下のルール参照)。
5. **対象外** — この PR で触れていないこと、後回しにしたこと。
6. **対象** — ブランチ名と、手順 1 で取った現在の HEAD sha。

本文は heredoc で渡す:

```
gh pr create --title "..." --body-file - <<'EOF'
...本文...
EOF
```

### 6. 誠実さのルール(絶対)

- 未検証の項目を「合格」「OK」と書かない。
- テスト結果を作り話さない。書いてよいのは実際に実行したコマンドと出力のみ。
- QA 記録には、その検証を実行した時点の sha を明示する。
- 「GDPR準拠」など、検証していない適合性・認証の主張を書かない。
- 不確実なことは「未検証」と明記する。

### 7. 作成・更新後に報告する

PR URL、タイトル、本文全体をそのまま報告する。

## 反面教師(このリポジトリの実際の失敗)

#59「クライアントUI 段階B-2 r5: 色照合の完全一致と全キー翻訳の振る舞いテスト」、
#61「QA-GDPR-02 r3: 参照列と本文 canary の修正」、
#62「QA-GDPR-13 r3: 本文 canary の指定値揃えと走査ルートの symlink 対応」に共通する問題:

- 工程メモ(Codex ハッシュ、QA の通し番号)が中心で、製品変更の説明が薄い
- 本文が「合格」から始まり、概要(何をなぜ)が無い
- 本文に書いた HEAD sha が実際の head sha とずれている(main 取り込み後に本文を更新しなかった)

→ これらをしないための手順が上の 1, 4, 5, 6 である。
