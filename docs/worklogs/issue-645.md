# Issue #645 の作業ログ

## 実装者メモ（round 1）

対象 head: 737d49ae2337cb76371a6050f2c1b814a95f7346

1. 詰まった箇所: `.agents/skills/implement-issue/required-checks.test.sh`: フィクスチャを `fix=$(new_fixture)` で作ると番号の加算がサブシェルで消え、全ケースが同じディレクトリを共有して前のケースの `checks.json` が残った。`new_fixture` が `$fix` を直接置く形に直した。
2. 曖昧だった文: 「fail が無く、どちらかが pending か一覧に無いとき 2 で終わる」（Issue #645 の「受け入れ条件」）。チェックが 1 つも報告されていない head で `gh pr checks` が一覧を返さず `no checks reported` で失敗する場合が「一覧に無い」か「取得に失敗」かが書かれていない。
3. 仮定と解釈: `no checks reported` の失敗は「一覧に無い」として 2 に数え、それ以外の失敗と、標準出力が JSON の配列でないときは 3 にした。`gh pr checks` は pending で終了コード 8 を返しうるので、終了コードではなく出力の形で取得の成否を決めた。5 つ以外の未知の `bucket` は「判定できない」として 3 にした（pass にも pending にも数えない）。
4. 足りなかった情報・検証手段: `gh pr checks --json` が fail・pending のときに返す終了コードを、手元では PR #631（全 pass、終了コード 0）でしか確かめていない。fail・pending の head での実測は無く、モックの `checks.rc=8` で再現した。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「巡回から定型の指示を受けたとき」の 2 に残る「`pr-digest <PR 番号>` の `required` 行」を、手順 8 と同じく `required-checks` の終了コードに置き換える別 Issue を立てる（この Issue の受け入れ条件は手順 8 の 220 行だけを指すため触れていない）。

使ったスキル: implement-issue、write-pull-request

## 実装者メモ（round 2）

対象 head: 787b957c43da6f938a8681524e0a80f9ca39d50f

1. 詰まった箇所: `.agents/skills/implement-issue/required-checks`: round 1 は `jq -e 'type == "array"'` で形を確かめていたため、標準出力に JSON の配列が 2 つ続くと各配列が通り、判定が 2 行になって `case` のどれにも当たらず終了コード 0 で抜けた（レビューの must-fix 1）。閉じる Issue も番号だけで照会していた（must-fix 2）。
2. 曖昧だった文: 「PR が閉じる Issue に作成者 `hatai` で行頭が `<!-- pr-size-exception -->` のコメントがあれば」（Issue #645 の「受け入れ条件」）。閉じる Issue が別リポジトリにある場合をどう扱うかが書かれていない。
3. 仮定と解釈: 閉じる Issue は `closingIssuesReferences` の `repository.owner.login`/`repository.name` と番号の組で `issue view -R` に渡し、リポジトリが読めない参照は 3（判定できない）にした。別リポジトリの Issue を一律に無視する解釈は選ばなかった（受け入れ条件は「PR が閉じる Issue」とだけ言うため）。判定の値が pass・fail・pending 以外なら、どんな形でも 3 で終わるように `case` の既定を die にした（同じ種類の取りこぼしを全経路で塞ぐ決まり）。
4. 足りなかった情報・検証手段: `mise run verify` が今回 853.99s かかり、600 秒の制限を超えてバックグラウンドに回った。完了の通知を待って終了コード 0 を確かめた。
5. 改善案: `.agents/skills/review-pr/SKILL.md` の手順 2 の pr-size の例外の段に「PR が閉じる Issue」の照会を `-R <owner>/<name>` 付きにする旨を足す別 Issue を立てる（レビュー担当側も番号だけで `issue view` している。この PR の対象外）。

使ったスキル: implement-issue、write-pull-request
