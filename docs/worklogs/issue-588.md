# Issue #588 の作業ログ

## 実装者メモ（round 1）

対象 head: 未コミット（commit と push は人間へ引き渡す）

hatai との設計インタビューで、RedisInsight 方式の run 照合と Project Calico 方式の confirmed-delivery marker を組み合わせることにした。Sentry 方式の受け手側 idempotency と、cache・git ref への永続保存は採らない。Issue #588 の本文を最終仕様へ更新してから実装した。

1. 詰まった箇所: GitHub Actions では後の run から step output を読めない。そこで、Send の output は同じ job の条件にだけ使い、後の run からは `Record delivery` step の成功を jobs API で読む形にした。`run-name` は REST API では `display_title` として読める。
2. 曖昧だった文: 「同じイベント」と「送った」の意味。同じ出来事は同じ PR 番号・同じ head sha・`synchronize`、「送った」は webhook の 2xx と `Record delivery` の成功と決めた。`opened`・`ready_for_review`・`closed` は synchronize の dedup に使わない。
3. 仮定と解釈: API 失敗・壊れた応答・1000 件上限・120 秒の探索上限では、確実な送信済みが無ければ送る。確実な送信済みが見つかれば、ほかの履歴が読めなくても送らない。`head_branch` と `pull_requests` は因果関係の証拠にしない。
4. 足りなかった情報・検証手段: GitHub Actions の式評価と `display_title` の実際のレンダリングはローカルでは確かめられない。jactionlint と式の文字列照合までをローカルで行い、実イベントはマージ後に確認する。実際の rerun と GitHub API の consistency も未検証。
5. 改善案: 偽 `gh` の scenario を再利用可能な部品にすると、ほかの workflow の API テストでも使える。今回は notify-grokbot に限定した。

使ったスキル: grilling、brainstorming、writing-plans、executing-plans、test-driven-development、implement-issue

## 実装者メモ（round 2）

レビューで指摘された重要点を直した。dedup step 自体に `continue-on-error` を付けて構造的な fail-open にし、API timeout と retry sleep を残り budget に合わせて切り詰めた。fail-open テストは終了コードと `duplicate=false` を確認し、偽 `timeout` / `sleep` / `curl` の引数まで記録して確認するようにした。`total_count` の 1000 件境界と `Record delivery` の固定文も確認対象に入れた。

## 検証

- `bash scripts/tests/gh-notify-samples.test.sh`: pass。workflow から抽出した dedup・Send・Record delivery を偽 `gh`・`curl`・`sleep`・`date` で実行し、合成データの確認はすべて pass。
- `shellcheck --severity=warning docs/worklogs/gh-notify-samples/check.sh`: pass。抽出した 3 つの `run:` も `shellcheck --shell=bash --severity=warning` で pass。
- `python3` の構文確認: pass。
- `mise exec -- jactionlint .github/workflows/notify-grokbot.yml`: pass。
- `mise exec -- jactionlint`: pass。
- `mise run verify`: fail。失敗したのは `.agents/skills/verify-pelway/verify-run.test.sh` で、この差分と関係ない cargo shim / `CARGO_HOME` の環境問題。単独実行でも同じ `cargo_target_dir` の確認で exit 2 だった。この変更の `gh-notify-samples.test.sh` は verify の中でも pass した。
- `mise run script-test`: fail。失敗は verify と同じ `.agents/skills/verify-pelway/verify-run.test.sh` だけで、ほかの 61 件は pass。
- PR サイズ: `+768 / -50`（上限 +800 / -2000 以内）。

## マージ後に確かめること

- `pull_request` と `pull_request_review` の run の `display_title` が `notify #<PR番号> <action>` になること。
- synchronize で 2xx のあと `Record delivery` が成功すること。
- 同じ PR・同じ head sha の rerun で `duplicate=true` になり Send が実行されないこと。
- curl 失敗や非 2xx のあとの rerun で再送されること。
