# Issue #571 の作業ログ

## 実装者メモ（round 1）

対象 head: 24a327e421fc8bfecf685fe3252b9395ad538241

1. 詰まった箇所: 手順 2: 完了条件が `.github/workflows/` の新しい workflow を求めていて、`pelway-dev` では push できず、いったん止まった。hatai がスレッドで hatai 名義の操作を許可したため、workflow を含むコミット `24a327e4` だけを hatai の SSH で push して再開した。
2. 曖昧だった文: 「既存の検出が片付くまでは報告のみ（失敗させない）で始め、必須チェックにするかは濱田さんが決める」（Issue #571「進め方」）
3. 仮定と解釈: 「報告のみ」は、検出があってもジョブを成功させる一方、ツール自体の失敗はジョブを失敗させることと解釈した。`continue-on-error` で失敗まで隠す解釈は採らなかった。「PR は Draft か `WIP:` のまま」（Issue「守ること」）が手順 9 の Draft 解除より優先すると仮定し、Approve 後も Draft を外さない。
4. 足りなかった情報・検証手段: Semgrep の Rules License の「自社の内部の業務目的」が、公開リポジトリの CI に当たるかの判断材料がない（条文に明記がない）。Semgrep を mise の pypi で入れると `.mise/locks/.../uv.lock` が 1861 行になり、pr-size の上限を超えることは、`mise lock` を実行して初めて分かった。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「止まる条件」の workflow の行に「hatai が名義の使用を許可したときは、workflow を含むコミットだけを hatai の署名で push し、PR の作成とそれ以外の push は `scripts/agent-gh dev` で行う」を足す。

使ったスキル: implement-issue、write-pull-request
