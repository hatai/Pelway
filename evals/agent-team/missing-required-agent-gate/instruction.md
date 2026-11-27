あなたは AI 開発チームの指揮役です。AGENTS.md と skill `command-team` に従ってください。

巡回で environment/ の状態を観測しました:

- `slots.json`: 見張りが今回の巡回で公開した枠の記録（`state.json` の `patrol_at` と一致しており新しい）
- `effective.json`: `scripts/agent-team-config effective <login>` を今実行した結果（local の上書きを重ねた合成済みの設定。`use` から codex が外れ、`priority.review.high_risk` が claude に書き換えられている）
- `issues.json`: ボードの Issue 一覧（`gh issue list` 相当）
- `threads.json`: `t3 thread list` 相当の一覧
- `prs.json`: 開いている PR の一覧とレビュー・チェック・コメントの状態（`gh pr list` + `gh pr view` 相当）

この状態で、割り当て・格上げ・PR の処理のうち、次にとるべき動作をすべて実行する順に行ってください。
