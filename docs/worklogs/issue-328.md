# Issue #328 の作業ログ

## 実装者メモ（round 1）

対象 head: 518474544632735ba78c62bbd33f170e6ef05f1c

1. 詰まった箇所: 分割の再構成で、素の `git fetch`/`git push` は SSH（`git@github.com:`）経路のため sandbox から届かず "access rights" で失敗した。`scripts/agent-gh dev git`（HTTPS＋App トークン）だけが通る。既存コミット fcd7d19 から `crates/pelway-contract/` だけを新しい origin/main（b8ba49f）上の `agent/328a-decision-contract` に移した。メモ込みで +804（上限 800 超）となったため、経路と本文の id 一致検査 `DecideRequest::check_path` を HTTP 層の役割として `handle_decide` のインライン比較に移し（`handle_comments` と同じ形）、+785/-1 に収めた。
2. 曖昧だった文: 「PR2 は PR1 のブランチに積み、PR1 のマージ前に base を main に替えること」（Issue #328 の `<!-- split-plan -->` コメント、作成者 hatai）
3. 仮定と解釈: 「マージ前に base を main に替える」は、文字通り PR1 未マージの間に替えると PR2 の差分に PR1 の +791 行が載って pr-size が超過し、CI も base=main でないと走らないため、PR1 がマージされる時点で GitHub の自動 retarget（またはその直前の手動 retarget）で main に替わる運用と解釈した。PR2 は `agent/328a-decision-contract` を base にして作り、レビューは積み上げ差分で受ける。PR1 の「契約テスト」は decision.rs のテスト群全体を指すと読み、承認コメントの AC 割当（PR2 が一覧・消失を確かめる）は検証の主担当の意味であって PR1 からテストを外す指示とは読まなかった。
4. 足りなかった情報・検証手段: `.github/workflows/verify.yml` は `pull_request: branches: [main]` のため、base が `agent/328a` の PR2 には verify・pr-size の CI が走らない（base=main に替わって初めて走る）。この挙動は verify.yml の trigger 設定を読んで確認した。
5. 改善案: .agents/skills/implement-issue/SKILL.md の「8. レビューを委託する」に「積み上げ PR は base が main でない間 CI が走らない。base=下段ブランチのままレビューを委託し、上段 PR のマージで main に替わったあと必須チェックを確認する」を足す。

使ったスキル: `implement-issue`（`.agents/skills/implement-issue`）、`tdd`（seam: 契約の台帳テスト）

## 実装者メモ（round 2）

対象 head: 15cc6a8b1c4e5b24ad89e80e6e5012460dbd0cc7

1. 詰まった箇所: `verify.yml` は `pull_request: branches: [main]` のため、base=`agent/328a` の積み上げ PR では必須チェック（verify・pr-size）が起動しない。手順 8 の「チェックが通ってから委託」が適用できないため、レビューは積み上げ差分のまま委託し、CI は base が main に替わった後に確認する運用にした。
2. 曖昧だった文: 「PR1 のマージ前に base を main に替えること」（Issue #328 の `<!-- split-plan -->` コメント、作成者 hatai）
3. 仮定と解釈: 文字通り PR1 未マージで base=main に替えると PR2 の差分に PR1 の全量が載り pr-size +1501 で必ず失敗するため、「PR1 のマージと同時（GitHub の自動 retarget）か、PR1 が Approve されてマージ直前になった時点で base を main に替える」と解釈した。PR2 は Draft のまま base=`agent/328a-decision-contract` で作る。確定経路は送信承認と別系統の `POST /v1/decisions/{decision_id}/decide`、操作者は approve と同じ合成ヘッダー（`x-pelway-member-id`/`x-pelway-member-kind`）で受けるとした。
4. 足りなかった情報・検証手段: なし。`scripts/pr-size.sh --base agent/328a-decision-contract` で積み上げ差分だけを +710/-1 と実測した（issue-93・issue-101 のメモで既知の手法）。
5. 改善案: .agents/skills/implement-issue/SKILL.md の「6. 検査してコミットし、行数を確かめてから push する」に「積み上げ PR は `mise run pr-size` ではなく `scripts/pr-size.sh --base <下段ブランチ>` で数え、CI は base=main に替わってから走る」を足す。

使ったスキル: `implement-issue`（`.agents/skills/implement-issue`）、`tdd`（seam: pelway-http のループバック HTTP/1.1 結合テスト）
