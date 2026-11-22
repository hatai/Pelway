# Issue #643 の作業ログ

## 実装者メモ（round 1）

対象 head: 086f9889f3cdf2a9bb953fbf47db4f0a92447015

1. 詰まった箇所: `scripts/tests/conductor-watch-stall.test.sh`: 既存のモックの `t3` が `thread list` 以外を `other-calls` に記録するため、新たに呼ぶ `thread show` で `no_writes` が落ちた。読み取りなのでモックに `thread show` の応答（空の作業ツリー）を足した。sandbox 内では実物の `t3 thread list --json` が空の出力で JSON にならず、`thread show` の形は Issue の記述（`thread.worktreePath`）に拠った。
2. 曖昧だった文: 「`t3 thread show` が失敗したスレッドはその巡回で数えず、ほかのスレッドの判定と見張りの周回は続く」（Issue #643 の受け入れ条件）
3. 仮定と解釈: 失敗したスレッドを含む既出の組み合わせは通知済みの記録から消さないことにした（消すと、失敗が直った次の巡回で同じ行が再び出る）。失敗中に残りの本数で別の組み合わせができたときは、その組み合わせを 1 行出す。目的の「トップレベル」に合わせ、`parentThreadId` があるスレッドも数えない（一覧には現れない前提）。
4. 足りなかった情報・検証手段: sandbox から実行中の T3 サーバーの `t3` に届かず、実物の `thread show --json` の出力を確かめられなかった。
5. 改善案: `docs/agents/grokbot-runbook.md` の「`pelway.agent-watch/v1`」の表の直後に「`THREAD shared-worktree` の組み合わせは、`thread show` の失敗中も通知済みとして保つ」と足す。

使ったスキル: implement-issue
