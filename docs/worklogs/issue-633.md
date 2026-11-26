# Issue #633 の作業ログ

## 未決事項の結論（`t3 thread show --json` に worktree 準備中を示す項目があるか）

項目は無いと判断した。確かめた範囲:

- `t3 thread show --help`: フラグ（`--base-dir`・`--messages`・`--json`）だけで、出力項目の記載は無い。
- 手元の T3 Code のソース（`~/.t3/source/dfd2187ccb0f7d16b0ff6d13f79eb5236ca9c46f`）: `t3 thread show --json` は MCP の `t3_thread_read` を呼び、`thread` を `OrchestratorMcpThreadDetail` で返す（`apps/server/src/cli/thread.ts`）。その項目は `threadId`・`status`・`runtimeMode`・`branch`・`worktreePath` などで、worktree を作っている途中を示す項目は無い。
- サーバー側には `WorktreeSetupSnapshot`（`phase: "running"` と段階の状態）があるが、WebSocket の `worktreeSetup` 通知向けで、`t3_thread_read` には出ない。CLI に `worktree` のサブコマンドも無い（`t3 --help`）。
- `ThreadLaunchService` は worktree を作り終えてから `thread.metadata.update` で `branch`・`worktreePath` を書くため、準備中は `worktreePath` が null のまま — worktree なしで立てたスレッドと見分けが付かない。`status`（`idle`/`preparing`/`starting`/...）は run の状態で、worktree なしのスレッドでも同じ値を通るため区別に使えない。

そのため時間切れの行は「worktree がまだ無い（準備中か、worktree なしで立てられた）」と両方を書き、T3 の画面で新指揮役の worktree を確かめる案内にした。

## 実装者メモ（round 1）

対象 head: 555aa264c41947857da25a1151760c94549e0120

1. 詰まった箇所: `docs/agents/conductor-context.md`: 「恒久交代」の「手順 2」の指す箇所。#632 で番号が 1 ずつ繰り上がり、受け入れ条件の「手順 2」は #632 前の番号では権限確かめの段（現行の項目 1）だった。待ちの記述は項目 1 に置き、導入文に lock のやり直しを足した。
2. 曖昧だった文: 「`docs/agents/conductor-context.md` の「恒久交代」の手順 2 に、worktree を上限付きで待つことが書かれている」（Issue #633「受け入れ条件」）
3. 仮定と解釈: 「手順 2」は #632 前の番号（権限確かめ）と解釈し、現行の項目 1 に書いた（worktree の確かめはそこにしか無いため）。もう 1 つの解釈（現行の項目 2＝停止文の段）は意味的に置き場が無い。「最大 3 回やり直し」は初回 + 3 回（計 4 回）と読んだ（「3 回のやり直しも失敗したら」とあるため）。時間切れの「待った秒数」は実際に待った秒数（`SECONDS` の差）を出し、上限も併記した。
4. 足りなかった情報・検証手段: `t3 thread show --json` の出力項目は CLI の `--help`・文書に無く、T3 Code のソース（`packages/contracts/src/orchestratorMcp.ts` の `OrchestratorMcpThreadDetail`）を読んで確かめた。`t3 --help` や CLI 文書に JSON の項目一覧があれば調べなくて済む。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「手順 1」の「文書の節（見出しを `grep`）」の行に「箇条書きの項目は番号（手順 N）ではなく見出し・節名で指す（番号は編集でずれる）」を足す。

使ったスキル: .agents/skills/implement-issue、.agents/skills/diagnosing-bugs（`verify-pelway/verify-run.test.sh` の失敗の切り分けに Phase 2。`scripts/red-check origin/main` で同じ失敗を再現し、環境起因と確かめた。この環境は `/tmp`・`/home` が nobody:nogroup の所有で、`dir_chain_safe` の「置き場より上は自分か root の持ち物」に反するため、書き込める場所の階層では必ず落ちる）。`mise run verify` は `script-test` 1/55（この試験のみ）で落ちたが、ほかの全段は通った。
