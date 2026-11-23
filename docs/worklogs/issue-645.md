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

## 実装者メモ（round 3）

対象 head: acd1ba36ced55e601e2f4dcfc99abb1adbe4c109

1. 詰まった箇所: `.agents/skills/implement-issue/required-checks`: round 2 で `pr checks` の応答だけを「ちょうど 1 つの JSON の値」に限り、`pr view` の応答には同じ制約を当てていなかった（レビュー round 2 の must-fix）。
2. 曖昧だった文: 「同じ種類の must-fix が 2 round 続いたら（同じ種類かは自分で判断する）、指摘された経路だけを直さず、その種類の全経路に当てる決まりを決めてから直し」（`.agents/skills/implement-issue/SKILL.md` の手順 9）。round 1 の複数 JSON の指摘と round 2 の `pr view` の指摘を同じ種類と判断した。
3. 仮定と解釈: 決めた決まりは「GitHub の応答は、どれもちょうど 1 つの JSON の値で、型が期待どおりのときだけ使う」。`one_json` にまとめ、`pr checks`（配列）・`pr view`（オブジェクト）・`issue view`（オブジェクト。`--jq` で数えるのをやめ、生の応答を確かめてから手元の jq で数える）の 3 経路すべてに当てた。`issue_view_multiple_json` は修正前から通った（件数を数字だけに限る既存の検査が弾いていた）ので、修正の証拠ではなく決まりの回帰の確認として置いた。
4. 足りなかった情報・検証手段: 今回の `mise run verify` は 713.97s で、バックグラウンドで完了を待った。
5. 改善案: `docs/agents/skill-helpers.md` の「同梱できるものの範囲」に「GitHub の応答は、ちょうど 1 つの JSON の値で型が期待どおりのときだけ使い、それ以外は判定できないとして非 0 で終える」を足す（helper ごとに同じ指摘を受けないため）。

使ったスキル: implement-issue、write-pull-request

## 実装者メモ（round 4）

対象 head: dafef7ef970fa852c65e07d81c719928688e1091

1. 詰まった箇所: claude 担当（round 3）の `[文書のずれ]` nit 2 件のうち、test.sh のモック説明の方はファイル変更で、push が head を動かし両担当の approve が無効になるため、印の nit の round（通算 1 回目）として再委託まで行った。codex 担当の印の nit（本文の件数 33→32）は `pr edit` で直せたため round にせず済ませた。
2. 曖昧だった文: 手順 9 の「印の nit だけを直す round は、この PR で通算 2 回まで」。PR 本文だけの修正（head が動かず手順 5・6 のやり直しが無い）も回数に数えるかは書かれていない。
3. 仮定と解釈: 本文だけの修正は approve を無効にしないため round に数えず、ファイルを含むこの round を通算 1 回目の印の nit round とした。effort は上げず round 3 と同じ（codex `max`・claude `max`）にした。本文の「対象外」の署名の記述は、737d49ae が表せないため以降の push も全体署名なしになる実態に合わせた。
4. 足りなかった情報・検証手段: claude 担当のレビューは provider の 7 日間上限で 1 回起動に失敗し、snooze から自動再開した（issue-568 の先例どおり）。上限の解除を待つ間の動作は skill に書かれていないため、先例に従った。
5. 改善案: `agent-gh-resign` の「表せないコミットがあれば push 全体を署名なしにする」が後の push にも及ぶことは AGENTS.md:38 の書き方（push 単位の記述）からは読み取りにくい。補う別 Issue（この PR の範囲外。レビュー本文も「この PR の範囲外」と明記）。

使ったスキル: implement-issue
