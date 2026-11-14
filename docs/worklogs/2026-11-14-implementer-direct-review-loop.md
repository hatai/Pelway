# 実装担当によるレビュー委託への切り替え（2026-11-14）

コードレビューの割り当てを「指揮役がレビュースレッドを立てる」から「実装担当が `delegate_task` でレビューを委託し、修正⇔レビューを自分で回す」へ切り替えた変更の記録。対象 PR の本文に変更内容と確認方法がある。ここには経緯と判断を残す。

## 決めたこと（人間との問答で確定）

1. 指揮役はマージとエスカレーション受けを維持。レビューの割り当ては実装担当へ
2. 委託は `delegate_task`（`role: "review"`、`runtimeMode: "auto-accept-edits"`、`isolation: "worktree"`、target は `codex_review` などレビュー用プロバイダー）。ラウンドごとに新規委託
3. レビュー本体は従来どおり `scripts/agent-gh review` で `pelway-review[bot]` 名義。実装担当はラウンドサマリーを `pelway-dev[bot]` 名義でコメント
4. Approve で完了、自分で `pr ready`。修正 3 回（4 ラウンド目）・委託 `failed` 2 回・判定割れでエスカレーション。3 ラウンド目の委託で effort を 1 段上げる
5. レビューの同時数上限は目安に格下げ（実装担当は自分の PR を直列で回す）
6. 抜き取り（Claude）は PR 番号 % 4 == 0、通常の担当は PR 番号の偶奇（偶数 Codex・奇数 Devin CLI）で決定論化。Devin 実装 PR への Devin レビュー禁止は維持
7. `security=yes` は差分ベース条件の機械判定に一本化（実装担当が Assigned 行に書く）
8. 指揮役が T3 スレッドとして動くとき、作業スレッドの作成は `t3_thread_launch` に統一（Grok Bot クラウドは T3 Connect MCP のまま）
9. スレッドの片付け（settle）: PR を `link_pull_request` で紐付けるとマージで自動 settle。PR なし等は指揮役が `t3_thread_organize`（`action: settle`）。`needs-human` 中は settle しない
10. 実装担当の最終報告は 5 行（`issue` / `pr` / `review` / `ready` / `memo`）。`verify:` / `pr-size:` 行は廃止（指揮役は GitHub のチェックで確かめる）
11. 調査報告 PR のレビューと `scripts/agent-gh` を変える PR のレビューは、指揮役が main から立てるレビュースレッドのまま（委託の子の worktree が PR head の checkout になるため、レビュー担当が PR の版の `scripts/agent-gh` を実行しないようにする）

## 残っている確認

- `delegate_task` の子が `codex_review` の sandbox（ネットワーク GitHub 限定・権限プロファイル `review`）を継承するか → PR #275 の round 1 で実施し、4 点とも合格。`docs/dev-environment.md` の記録欄に記録済み
- `claude_review`・Devin CLI のレビュー稼働ゲート（既存の確認欄が未実施のまま）
