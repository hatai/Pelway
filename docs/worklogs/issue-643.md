# Issue #643 の作業ログ

## 実装者メモ（round 1）

対象 head: 086f9889f3cdf2a9bb953fbf47db4f0a92447015

1. 詰まった箇所: `scripts/tests/conductor-watch-stall.test.sh`: 既存のモックの `t3` が `thread list` 以外を `other-calls` に記録するため、新たに呼ぶ `thread show` で `no_writes` が落ちた。読み取りなのでモックに `thread show` の応答（空の作業ツリー）を足した。sandbox 内では実物の `t3 thread list --json` が空の出力で JSON にならず、`thread show` の形は Issue の記述（`thread.worktreePath`）に拠った。
2. 曖昧だった文: 「`t3 thread show` が失敗したスレッドはその巡回で数えず、ほかのスレッドの判定と見張りの周回は続く」（Issue #643 の受け入れ条件）
3. 仮定と解釈: 失敗したスレッドを含む既出の組み合わせは通知済みの記録から消さないことにした（消すと、失敗が直った次の巡回で同じ行が再び出る）。失敗中に残りの本数で別の組み合わせができたときは、その組み合わせを 1 行出す。目的の「トップレベル」に合わせ、`parentThreadId` があるスレッドも数えない（一覧には現れない前提）。
4. 足りなかった情報・検証手段: sandbox から実行中の T3 サーバーの `t3` に届かず、実物の `thread show --json` の出力を確かめられなかった。
5. 改善案: `docs/agents/grokbot-runbook.md` の「`pelway.agent-watch/v1`」の表の直後に「`THREAD shared-worktree` の組み合わせは、`thread show` の失敗中も通知済みとして保つ」と足す。

使ったスキル: implement-issue

## 実装者メモ（round 2）

対象 head: ab51a167208edf8e27099d1cb83a1077120850a1

1. 詰まった箇所: `scripts/conductor-watch.sh`: round 1 は `if threads:` で判定を飛ばしており、一覧の取得失敗と正常な空の一覧（`[]`）を区別していなかった（codex の must-fix）。`thread list` の失敗時に `threads.json` を消し、python 側で `threads_ok` を立てて区別した。試験 6 を足した。
2. 曖昧だった文: 「同じ PR の 2 回目以降のレビューは 1 段上げる」（`.agents/skills/command-team/SKILL.md` の「レビュー担当と effort」）と「3 回目のレビューの委託では effort を 1 段上げる」（`.agents/skills/implement-issue/SKILL.md` の手順 9）
3. 仮定と解釈: effort は、実装担当が担当と effort を決めるときに従う表として command-team の文を採り、round 2 の委託から 1 段上げた（codex low→medium、claude medium→high）。claude の nit 4 件は `[文書のずれ]` の印が無いため直していない。
4. 足りなかった情報・検証手段: round 1 と同じく、実物の `t3 thread list`/`thread show` の JSON（`createdAt` の有無を含む）を sandbox から確かめられない。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 9 の「3 回目のレビューの委託では effort を 1 段上げる」を、command-team の「2 回目以降は 1 段上げる」と揃える。

使ったスキル: implement-issue
