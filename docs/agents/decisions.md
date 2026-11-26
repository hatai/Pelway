# 運用の決定

エージェントが手順書と食い違う指示を受けたときに確かめる、hatai の運用の決定の一覧。決定の中身だけを書き、日付や経緯は書かない。ID は既存の識別子を使い、無ければ反映する Issue 番号から作る。決定を手順書に反映したら、反映先をその手順書に変えるか行を消す。反映先が Issue 番号のものは未反映。

| ID | 決定の中身 | 反映先（反映済みの手順書、または反映する Issue） |
|---|---|---|
| t629u | レビューの修正ラウンドの回数では止めない。must-fix が残る限り、修正・push・再レビューを続ける。force push・マージ・`.github/workflows/` の push・pr-size の例外は、回数とは別に hatai が判断する。 | #725 |
| issue-806 | `mise run pr-size` が上限を超えたときは push せず、Issue に分割案・手元の head の SHA・`git diff --stat origin/main...HEAD` の出力を書いて止まる。例外が認められたあとは、hatai の `<!-- pr-size-exception -->` コメントと PR の `pr-size-exception` ラベルを確かめて push し、手順を再開する。 | #806 |
| issue-807 | `mise run verify` が exit 0 のとき、その HEAD の木の hash の印を git の外の手元に書き残す。`scripts/agent-gh <role> git push` は、push する HEAD の木がその印を持つときだけ通す。HEAD の木と印の木の差が `docs/worklogs/` の下だけなら直前の印を使ってよい。`PELWAY_AGENT_SKIP_VERIFY_GATE=1` のときは確かめずに通し、通したことを標準エラーに出す。 | #807 |
| issue-811 | 利用の上限によるレビュー委託の failed は、上限が解けると自動で再開される。Guard が head のずれを止めるので手で予約しない。今の head を見てもらうには、上限が解けてから新しい委託記録を残して委託し直す。上限による failed は 2 回連続の failed に数えない。 | #811 |
