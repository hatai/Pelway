# Issue #574 の作業ログ

## 実装者メモ（round 1）

対象 head: 22048fa8efc480c48dc7ddfa5db0bd2ce0a8344e

1. 詰まった箇所: `docs/agents/grokbot-runbook.md` の「4. 仕事の始め方」手順 1 と「割り当て・停止・再開」: 指揮役は割り当ての前に `agent:working` を付けるため、実装スレッドに「すでに付いている Issue は取らない」をそのまま当てると、割り当てられたスレッドが自分の Issue を取れなくなる。
2. 曖昧だった文: 「すでに付いている Issue は取らない」（開発リードからの依頼文「内容」1）
3. 仮定と解釈: 指揮役が割り当てで付けたラベルは、そのスレッドが自分の占有として引き継ぐと解釈した。「取らない」の対象は、別ブランチの取得コメントが解放されずに残っているとき、または割り当て指示なしで立ったスレッドでラベルが付いているときとした。採らなかった解釈は、ラベルを付けるのを実装スレッドだけにする案（枠の数え方とボードの Working の導出が割り当て時点のラベルに依存するため）。QA コメントの `run:` は、その SHA の必須チェックの GitHub Actions の run の URL と仮定した（既存の QA 合格コメントの形がリポジトリに無かったため）。
4. 足りなかった情報・検証手段: QA の確認項目の文書がリポジトリに無かった（`pelway-qa` の名義の説明だけが `docs/dev-environment.md` にあった）ため、`docs/agents/qa-checklist.md` を新しく作った。`gh pr view --json comments` の `author.login` は App の `[bot]` を落とす（`pelway-qa`）ことを Issue #479 のコメントで確かめた。
5. 改善案: `scripts/conductor-watch.sh` の待ち先の導出で、Approve 済みでも同じ SHA の QA の合格が無い PR を `waiting_on=merge` ではなく QA 待ちとして出す（コードの変更なので、この PR の範囲外。runbook の「マージの条件」に、今は QA 待ちとして読み替えると書いた）。

使ったスキル: implement-issue
