# Issue #632 の作業ログ

## 実装者メモ（round 1）

対象 head: 13565c6cfaff912ca0b1c60dd649d3f3e6d1827a

1. 詰まった箇所: 手順 4: 「runtimeMode が読めない」試験は旧スクリプトでも `-` が mode 不一致に落ちて exit 3 になり、red にならなかった。分岐を独立させる形の試験（exit 3・何も送らない・登録不変・ログ行の `runtimeMode=-`）で条件の全体を確かめることにした。
2. 曖昧だった文: 「実行するたびに、新指揮役の実際の `runtimeMode` と worktree のパスを、`conductor-rebuild: new conductor <ID> runtimeMode=<値> worktree=<パス>` の 1 行で標準エラーに出す」（Issue #632 受け入れ条件）。「実行するたびに」が exit 3 の失敗経路を含むか曖昧で、`thread show` が成功した時点で毎回出す（読み取り値をそのまま出すので `-` や不一致の値も残る）と解釈した。
3. 仮定と解釈: `--to` なしの停止は worktree 確かめの後・見張り役/T3 の確かめの前に置いた（worktree からの誤起動は引き続き worktree 拒否の文が出る並びを選んだ）。「使い方を表示」は既存の `usage`（usage の行を出して exit 2）と解釈した。権限が合わない・読めないときの案内は「T3 の画面で直す」ではなく「`t3_thread_launch` で立て直して `--to` で実行し直す」とした（Issue の経緯で画面での変更が保存されなかったため）。
4. 足りなかった情報・検証手段: 実際の `t3_thread_launch` で立てたスレッドの `thread show --json` が `worktreePath` を返す形は、偽の t3 の想定でしか確かめられない（実 T3 での交代は対象外）。
5. 改善案: `docs/agents/grokbot-runbook.md` の「MCP の `t3_thread_launch` と `t3_thread_send` の引数」に、実装・レビュー・調査とは別に恒久交代（指揮役）向けの例（`title`・`message` の形）を足すと、`conductor-context.md` の「恒久交代」からそのまま引ける。

使ったスキル: `.agents/skills/implement-issue`、`.agents/skills/write-pull-request`
