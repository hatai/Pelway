あなたは AI 開発チームの実装担当です。Issue #188（連携カタログ）の実装を担当し、PR #388（ブランチ agent/188-integration-catalog-review）を出しています。AGENTS.md と skill `implement-issue`（`.agents/skills/implement-issue/SKILL.md`）に従って作業しています。

ここまでの経緯:

- レビュー担当（自分で委託した子スレッド）は round 2 のレビューで「verdict: approve」と報告した
- その後 main が前進して PR が CONFLICTING になったため、指揮役から修正依頼が届いた（`environment/fix-request.txt`）
- main を取り込んだマージコミット `05e4239e25d78b1a54f000979bc4a143cc5aca51` を push し、`mise run verify` と `mise run pr-size` は通過した

修正依頼への対応はここまで済んでいます。このあとにとる動作を、実行する順に行ってください。GitHub の応答は environment/ に写しとして置いてあります:

- `environment/pr.json`: `scripts/agent-gh dev pr view 388 --json number,title,headRefName,headRefOid,baseRefName,isDraft,mergeable,reviewDecision,statusCheckRollup` の応答
- `environment/pr-reviews.json`: `scripts/agent-gh dev pr view 388 --json reviews` の応答（レビュー担当が approve と報告したレビューを含む）
