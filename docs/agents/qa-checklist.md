# QA の確認項目

QA がレビュー済みの PR を独立に確かめるときの確認項目と、合格・不合格のコメントの形。QA の合格は PR のマージの条件の 1 つである（skill `command-team` の「マージ」）。

## 対象

- 対象は PR の現在の head の SHA 1 つ（`scripts/agent-gh dev pr view <M> --json headRefOid --jq .headRefOid` で取る）。確かめている間に head が動いたら、その確認は捨てて新しい head でやり直す
- 合格はコメントに書いた SHA に対してだけ有効。後から push されて head が動いたら、その合格は失効する。QA が新しい head で確かめるまで、合格として扱わない

## 確認項目

1. 必須チェック（`mise run verify`・`pr-size`）が、その SHA の run で通っている。確かめた run（GitHub Actions の run の URL）をコメントに書く
2. Issue の受け入れ条件を、その SHA で確かめる（PR が閉じる Issue の「受け入れ条件」。分割された PR では、hatai 作の `<!-- split-plan -->` の表でその PR に割り当てられた条件）
3. 確かめ方を弱める変更（定義は skill `review-pr` の手順 2「確かめ方を弱める変更」の一覧。テストの削除、`#[ignore]`・`.skip` などの追加、アサーションの削減や緩和）が差分にあるかを確かめる。ある変更ごとに、PR 本文の「変更内容」の見出し「確かめ方を弱める変更」に理由が書かれているかを読む。理由が無い変更が 1 件でもあれば不合格

1〜3 のどれかが満たされなければ不合格。

## コメントの形

QA の名義（GitHub App `pelway-qa`、`docs/dev-environment.md` の「QA 担当の GitHub App（pelway-qa）」）で、PR にコメントする。

```bash
scripts/agent-gh qa comment <M> --body-file - <<'EOF'
QA: pass | fail
sha: <確かめた head の SHA（40 桁）>
run: <確認項目 1 で確かめた run の URL>

- 必須チェック: pass | fail
- 受け入れ条件: <条件ごとの結果>
- 確かめ方を弱める変更: なし | あり（理由あり <件数> / 理由なし <件数>）
EOF
```

不合格のときは、どの項目で落ちたかと、直すべきこと（理由の無い変更のファイルと行など）を本文に書く。

## 合格として数えるもの

指揮役がマージの条件として数える QA の合格は、次をすべて満たすコメントだけである（リポジトリは public で、誰でもコメントできるため）。

- 作成者が GitHub App `pelway-qa` である。REST の `user.login` が `pelway-qa[bot]` で `user.type` が `Bot` であることで確かめる（`scripts/agent-gh dev api --paginate repos/hatai/Pelway/issues/<M>/comments --jq '.[] | select(.user.login == "pelway-qa[bot]" and .user.type == "Bot") | .body'`）。`gh pr view --json comments` の `author.login` は `[bot]` を落として `pelway-qa` と出すため、同名の利用者と区別できない
- 1 行目が `QA: pass` である
- `sha:` の値が PR の現在の head の SHA と一致する
- `run:` の行がある
