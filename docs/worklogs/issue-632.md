# Issue #632 の作業ログ

## 実装者メモ（round 1）

対象 head: 13565c6cfaff912ca0b1c60dd649d3f3e6d1827a

1. 詰まった箇所: 手順 4: 「runtimeMode が読めない」試験は旧スクリプトでも `-` が mode 不一致に落ちて exit 3 になり、red にならなかった。分岐を独立させる形の試験（exit 3・何も送らない・登録不変・ログ行の `runtimeMode=-`）で条件の全体を確かめることにした。
2. 曖昧だった文: 「実行するたびに、新指揮役の実際の `runtimeMode` と worktree のパスを、`conductor-rebuild: new conductor <ID> runtimeMode=<値> worktree=<パス>` の 1 行で標準エラーに出す」（Issue #632 受け入れ条件）。「実行するたびに」が exit 3 の失敗経路を含むか曖昧で、`thread show` が成功した時点で毎回出す（読み取り値をそのまま出すので `-` や不一致の値も残る）と解釈した。
3. 仮定と解釈: `--to` なしの停止は worktree 確かめの後・見張り役/T3 の確かめの前に置いた（worktree からの誤起動は引き続き worktree 拒否の文が出る並びを選んだ）。「使い方を表示」は既存の `usage`（usage の行を出して exit 2）と解釈した。権限が合わない・読めないときの案内は「T3 の画面で直す」ではなく「`t3_thread_launch` で立て直して `--to` で実行し直す」とした（Issue の経緯で画面での変更が保存されなかったため）。
4. 足りなかった情報・検証手段: 実際の `t3_thread_launch` で立てたスレッドの `thread show --json` が `worktreePath` を返す形は、偽の t3 の想定でしか確かめられない（実 T3 での交代は対象外）。
5. 改善案: `docs/agents/grokbot-runbook.md` の「MCP の `t3_thread_launch` と `t3_thread_send` の引数」に、実装・レビュー・調査とは別に恒久交代（指揮役）向けの例（`title`・`message` の形）を足すと、`conductor-context.md` の「恒久交代」からそのまま引ける。

使ったスキル: `.agents/skills/implement-issue`、`.agents/skills/write-pull-request`

## 実装者メモ（round 2）

対象 head: 815444b0e6c48c4f67448acdc12310aabf5bb552

1. 詰まった箇所: scripts/conductor-rebuild: Codex の must-fix（worktree 無しと権限の不適合が重なったとき、worktree の確かめが先に die して exit 1 になる）。権限の検査を worktree の検査より前に動かし、worktree 不成立の失敗も die の 1 行ではなく、権限の失敗と同じ 3 行の案内（理由・無送信・`t3_thread_launch` での立て直しと `--to` 付きの再実行）にそろえた。
2. 曖昧だった文: Codex の `[文書のずれ]` nit「『始め方』へ戻すか、スクリプトでも立て直し手順を表示」は二者択一で、スクリプト側に表示を足す方を選んだ（worktree 不成立も直し方は立て直しで、権限の失敗と同じ案内でそろうため）。
3. 仮定と解釈: Claude の `[文書のずれ]` nit に対し、H23 の「操作」欄は MCP での起動の主体を conductor-context.md:119 の「人か指揮役」にそろえ、Grok Bot 経由が「できない」のは本体 checkout での実行（PC の物理的な操作）に限ると理由に書いた。H23 の参照は :109（空行で参照先を失っていた）を :119（始め方）に、:62 を実際のコマンドがある :63 に直した。
4. 足りなかった情報・検証手段: T3 の `existing_worktree` で本体 checkout を指したとき `thread show` が `worktreePath` に何を返すかは未確認（Claude の nit。`new_wt` が `$project` と同じパスなら拒む多層防御は、範囲を広げるためこの round では入れていない）。
5. 改善案: `conductor-context.md:119` の `t3_thread_launch` の引数の例に必須の `title` が無い（Claude の nit。round 1 の改善案 5 と同じ趣旨。印の無い nit のためこの round では直していない。次の round で指摘が残れば対応し、残らなければ別 Issue 候補）。

使ったスキル: `.agents/skills/implement-issue`、`.agents/skills/receiving-code-review`
