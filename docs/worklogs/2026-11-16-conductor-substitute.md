# 2026-11-16 指揮役（代行）の作業ログ

Todo 一括処理（hatai 指示）の続きと、監視の根本対策。前日の記録は `2026-11-15-conductor-substitute.md`。

## 割り当ての記録

| 日時 | Issue・PR | 役割 | 担当 | モデル | effort | 理由 |
|---|---|---|---|---|---|---|
| 2026-11-16 01:05 | PR #361（conductor-watch の t3 解決） | review | devin | swe-2-high | default | 指揮役自身の PR。奇数のため Devin CLI（追加 39 行、高リスクなし） |
| 2026-11-16 01:30 | PR #356（再委託 review/356-2） | review | codex | gpt-6-luna | default | 1 回目は必須チェック pending で中断。チェック通過後に同じ担当で再実行 |
| 2026-11-16 02:00 | PR #362（レビュー判定割れは修正側優先） | review | codex | gpt-6-luna | default | 指揮役自身の PR。偶数のため Codex（追加 4 行、高リスクなし） |
| 2026-11-16 02:10 | #264 | implement | opencode | zai-coding-plan/glm-5.3 | low | 定型・小規模（selftest.sh へのテストケース追加）。GLM 枠の空きに Todo の次（非ブロック）を割り当て |
| 2026-11-16 02:14 | #218 | implement | opencode | zai-coding-plan/glm-5.3 | low | 定型・小規模（AGENTS.md への規則追記）。Todo の次の非ブロック |

## 判断の記録

- #228: hatai 判断「待つ」を適用（#229 と同じ扱い）。needs-human と agent:working を外し Todo へ。上流（rsa 0.10 正式版対応）のリリースで再割り当て
- #328: hatai 判断「分割で」。split-plan コメント（PR1 contract +791 / PR2 http +710）を hatai 名義の gh で投稿し、実装スレッドに PR1 からの再開を指示
- #325・#326: hatai 判断「修正する」。判定割れは request-changes 側を正とし、両実装スレッドに修正指示を送信、needs-human を外して Working へ
- 恒久ルール化（hatai 指示）: 「今後、要修正と承認でレビュー結果が割れたときは修正を優先して」。command-team・implement-issue のスキルと agent-team 仕様の 4 箇所を修正し PR #362 としてレビュー中
- 監視の根本対策: conductor-watch.sh が /proc から実行中 T3 サーバーの実行ファイルを解決する修正（PR #361）。サービスは修正版で即時復旧済み
- ボード事故から復旧: Needs human 列追加の singleSelectOptions 置換で全 127 アイテムの Status が失われたが、スナップショット＋操作履歴から全件書き戻し検証済み。列は追加済み（RED）
- #264・#218 を GLM 枠の空き（#229・#228 の解除と #323 のマージで 3 本空き）に割り当て
- 監視→指揮役の通知経路の障害と恒久対策（hatai 指示「原因を分析して恒久対策」）: 原因は 3 つ — (1) watch.log → 指揮役スレッドへの転送スクリプト（notify-conductor.sh）の送信先スレッド ID が前々回の代行スレッドに固定されたまま、(2) 送信に使う `t3` CLI が conductor-watch と同じ DB 所有権の衝突で失敗（notify.log は 11/15 11:08 で停止）、(3) スクリプトが systemd 管理でなくマシン再起動（11/15 22:34）で消滅。検知（conductor-watch）は機能していたが、指揮役が hatai に起こされるまでイベントに対する処理（報告確認・マージ判断）に入らない構造だった。恒久対策として、T3 の scheduled task（interval 30 分・指揮役スレッドに bind）で指揮役を自動起床し watch.log の未読分（patrol-state.json の watch_log_offset）を処理する仕組みを導入（タスク ID は patrol-state.json に記録）。転送スクリプト経路は廃止し、runbook の「指揮役の定期巡回（スケジュール起床）」節に正本化
