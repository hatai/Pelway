# Issue #328 の作業ログ

## 実装者メモ（round 1）

対象 head: 6c04f3735bd7a3a5bc3efac9f13f77e1f3d7c9a6

1. 詰まった箇所: 分割の再構成で、素の `git fetch`/`git push` は SSH（`git@github.com:`）経路のため sandbox から届かず "access rights" で失敗した。`scripts/agent-gh dev git`（HTTPS＋App トークン）だけが通る。既存コミット fcd7d19 から `crates/pelway-contract/` だけを新しい origin/main（b8ba49f）上の `agent/328a-decision-contract` に移してコミット dde0e47→署名付け替え 6c04f37。
2. 曖昧だった文: 「PR2 は PR1 のブランチに積み、PR1 のマージ前に base を main に替えること」（Issue #328 の `<!-- split-plan -->` コメント、作成者 hatai）
3. 仮定と解釈: 「マージ前に base を main に替える」は、文字通り PR1 未マージの間に替えると PR2 の差分に PR1 の +791 行が載って pr-size が超過し、CI も base=main でないと走らないため、PR1 がマージされる時点で GitHub の自動 retarget（またはその直前の手動 retarget）で main に替わる運用と解釈した。PR2 は `agent/328a-decision-contract` を base にして作り、レビューは積み上げ差分で受ける。PR1 の「契約テスト」は decision.rs のテスト群全体を指すと読み、承認コメントの AC 割当（PR2 が一覧・消失を確かめる）は検証の主担当の意味であって PR1 からテストを外す指示とは読まなかった。
4. 足りなかった情報・検証手段: `.github/workflows/verify.yml` は `pull_request: branches: [main]` のため、base が `agent/328a` の PR2 には verify・pr-size の CI が走らない（base=main に替わって初めて走る）。この挙動は verify.yml の trigger 設定を読んで確認した。
5. 改善案: .agents/skills/implement-issue/SKILL.md の「8. レビューを委託する」に「積み上げ PR は base が main でない間 CI が走らない。base=下段ブランチのままレビューを委託し、上段 PR のマージで main に替わったあと必須チェックを確認する」を足す。

使ったスキル: `implement-issue`（`.agents/skills/implement-issue`）、`tdd`（seam: 契約の台帳テスト）
