# Issue #477 実装者メモ

対象 head: `08f258e`（agent-gh-resign 後の署名付き head）

## Issue と採用した方式

Issue #477: sandbox の抜け道を禁止することの明文化、および指揮役の `patrol-state.json` を sandbox から読み書きできる場所への移動。指揮役・実装役・レビュー役・調査役の手順と runbook に「sandbox に止められたら `systemd-run`・`nohup`・別のシェル・ラッパースクリプトなどの抜け道を使わず止まって報告する。作業ログの前例は許可にならない」を明記し、`docs/worklogs/2026-11-18-conductor-substitute.md` の `systemd-run` 前例 2 箇所に禁止の追記、2026-11-19 の事故（`run-u3294.service`・10:31 JST・終了コード 0・`patrol-state.json` の `scheduled_task_id` と `conductor_handover` 書き換え）の記録を `docs/worklogs/2026-11-19-conductor-substitute.md` として作成した。

`patrol-state.json` の新しい置き場は共有 git dir（`git rev-parse --path-format=absolute --git-common-dir` が指す `.git`）内の `pelway/patrol-state.json`。git の管理対象外で、主 checkout と全 worktree が共有し、sandbox は共有 git dir をコミット用に書き込み可でマウントするため指揮役の sandbox 内からも読み書きできる。旧置き場 `~/.local/state/pelway/patrol-state.json` は `conductor-watch.sh` の初回起動時に中身ごと引き継ぐ（`subjects`・`items`・`scheduled_task_id`・`conductor_handover`・`watch_log_offset` など全キー）。同じ理由で `pr-digest` の既出の指摘の記録も共有 git dir の `pelway/pr-digest` に移した。

## 仮定・解釈

- 「リポジトリまたは worktree の中で git の管理対象外の場所」は共有 git dir 内の `pelway/` と解釈した。worktree 直下のファイルは git 管理対象になり得るし worktree 間で共有されないため。sandbox-shell が共有 git dir を書き込み可で bind するのは既存の動作で、writable_roots 等の設定変更は伴わない。
- `conductor-watch` のログ・ロック・unit 生成は従来どおり XDG state を使う（ホストの systemd unit 経路が変わっていないため）。patrol-state だけが移動対象。
- 高リスク判定: `.agents/skills/`・`docs/agents/`・sandbox の境界に触れるため security=yes のレビューが必要。実装者が Devin CLI のためレビューは Codex・Claude の枠（Devin CLI 枠は外す）。

## テスト

- `scripts/tests/conductor-watch.test.sh`・`conductor-watch-label-actor.test.sh`・`conductor-watch-board-queue.test.sh`: 全パス。新パスへの書き込み・旧パスからの移行（`scheduled_task_id`・`items`・`watch_log_offset`・`conductor_handover` の引き継ぎ・旧ファイル保持）を確認。
- `scripts/tests/conductor-watch-ensure.test.sh`: fake repo が git 管理でないため `PELWAY_PATROL_STATE` を export して対応。加えて起動・kill 直後の `count` 即時確認を `waitn` 待機に置き換え（プロセス数え上げの競合で実行ごとに別の検査が落ちる既存の flaky を解消）。3 連続全パス。
- `mise run verify`: 全パス。
- `mise run pr-size`: +101 / -26（上限内）。
- sandbox 内からの実測: `test -w $(git rev-parse --git-common-dir)` 可・実ファイルの書き込み・削除成功。旧パス `~/.local/state/pelway/patrol-state.json` は `test -w` で書き込み不可（想定どおり）。

## レビュー

（round 1 委託後に記録）

## 使ったスキル・縫い目

- スキル: `implement-issue`（本メモ自身）・`command-team`・`review-pr`・`investigate-bug`・`pr-watch`（`pr-digest` の置き場変更）を対象に編集。
- 縫い目: `PELWAY_PATROL_STATE`（patrol-state のパス差し替え・全 conductor-watch テストで使用）・`PR_DIGEST_SEEN_DIR`（pr-digest の既出記録の差し替え）。両方とも運用上は既定値に任せ、テストと個別環境だけが差し替える形。
- 抜け道禁止の明文化は runbook 1 箇所・スキル 4 件のすべてに追加（抜け落ちなし）。
