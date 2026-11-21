# Issue #571 の作業ログ

## 実装者メモ（round 1）

対象 head: 24a327e421fc8bfecf685fe3252b9395ad538241

1. 詰まった箇所: 手順 2: 完了条件が `.github/workflows/` の新しい workflow を求めていて、`pelway-dev` では push できず、いったん止まった。hatai がスレッドで hatai 名義の操作を許可したため、workflow を含むコミット `24a327e4` だけを hatai の SSH で push して再開した。
2. 曖昧だった文: 「既存の検出が片付くまでは報告のみ（失敗させない）で始め、必須チェックにするかは濱田さんが決める」（Issue #571「進め方」）
3. 仮定と解釈: 「報告のみ」は、検出があってもジョブを成功させる一方、ツール自体の失敗はジョブを失敗させることと解釈した。`continue-on-error` で失敗まで隠す解釈は採らなかった。「PR は Draft か `WIP:` のまま」（Issue「守ること」）が手順 9 の Draft 解除より優先すると仮定し、Approve 後も Draft を外さない。
4. 足りなかった情報・検証手段: Semgrep の Rules License の「自社の内部の業務目的」が、公開リポジトリの CI に当たるかの判断材料がない（条文に明記がない）。Semgrep を mise の pypi で入れると `.mise/locks/.../uv.lock` が 1861 行になり、pr-size の上限を超えることは、`mise lock` を実行して初めて分かった。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「止まる条件」の workflow の行に「hatai が名義の使用を許可したときは、workflow を含むコミットだけを hatai の署名で push し、PR の作成とそれ以外の push は `scripts/agent-gh dev` で行う」を足す。

使ったスキル: implement-issue、write-pull-request

## 実装者メモ（round 2）

対象 head: 40bf3127a0ea9f2dfc81ad75084a882962955c7f

1. 詰まった箇所: 手順 9: Codex のレビューが `failed`（`pelway-review[bot]` 名義で投稿できなかった）。報告には must-fix 2 件があり、Claude の approve と割れた。修正側を優先して直した。
2. 曖昧だった文: 「委託の子が `failed` を報告したら、`error: checks pending` は待って同じラウンドを数えずに再委託し、それ以外は 1 回だけ再委託する」（`.agents/skills/implement-issue/SKILL.md` 手順 9）
3. 仮定と解釈: must-fix の中身がある `failed` は、request-changes と同じに扱うと解釈した。修正せずに同じ head を再委託する解釈は採らなかった。Gitleaks は `--cc` を読めない（使い捨てのリポジトリで 0 件）ため、`-m` を採った。件数は 23 件から 80 件に増えるが、場所の重複を除いた数をサマリーに出す。
4. 足りなかった情報・検証手段: マージコミットの差分の扱いは、Gitleaks の README に書かれていない。使い捨てのリポジトリで競合解消のときにだけ値を足し、`--log-opts` の値ごとに検出数を比べて確かめた（既定と `--cc` は 0 件、`-m` は 2 件）。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 9 の `failed` の扱いに「`failed` の報告に must-fix が書かれているときは、Request changes と同じに修正 round へ進み、次の round で再委託する」を足す。

使ったスキル: implement-issue
