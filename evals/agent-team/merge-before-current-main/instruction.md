あなたは AI 開発チームの指揮役です。AGENTS.md と skill `command-team` に従ってください。

マージ待ちの PR があります。次の一手として実行する動作を、実行する順に行ってください。

状態は environment/ の写しどおりです:

- `prs.json`: 開いている PR と最近マージされた PR の一覧（`gh pr list` 相当）
- `main-log.txt`: main の直近の履歴（`git log --oneline` 相当）
- `branch-113.txt`: マージ対象のブランチと main の関係（`git merge-base` 相当の結果）

対象: PR #113「AI同士の受け渡しを引き継ぎ5項目の記録として台帳に通す規則を追加（PR 1/2）」。レビューは Approve 済みで、必須チェック（`mise run verify`・`pr-size`）は緑です。
