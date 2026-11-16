# Issue #327 の作業ログ

## 実装者メモ（round 1）

対象 head: 8f55819e23d67555a62735db15a500dfda7cf245

1. 詰まった箇所: sandbox 内で DNS が壊れている（`/etc/resolv.conf` → `/mnt/wsl/resolv.conf` が無い）ため、各 exec の先頭で `mkdir -p /mnt/wsl && printf 'nameserver ...' > /mnt/wsl/resolv.conf` を実行（issue-322・326 と同じ）。mbx が `~/.cache` に書けず commit フックの cargo_check/clippy が失敗 → `XDG_CACHE_HOME=/tmp/xdg-cache` を add/commit の両方に付けて解消（326 round 2 の連鎖を踏まえ add と commit を同一 exec で実行）。`mise run verify` は環境起因で `scripts/tests/agent-sandbox.test.sh`（`~/.t3/worktrees` 書込不可）と `scripts/tests/conductor-watch-ensure.test.sh`（systemd ケース）が失敗するが、変更と無関係・既知（issue-322・326 に同記録）。
2. 曖昧だった文: 「チャンネル一覧とチャンネル情報が取得できる」（Issue #327 受け入れ条件）。経路の形は Issue には無い。
3. 仮定と解釈: 経路は 04 §3 と既存の route() の形に合わせ `GET /v1/workspaces/{w}/channels`（一覧は workspace 経路 = inbox・decisions と同じ形）、`GET /v1/channels/{c}`（情報は対象を指す経路 = decide と同じ発想）、`POST|GET /v1/channels/{c}/messages` とした。`purpose_id` は保存物で必須だが要求に無いため P01（指定参加者への保存・配送）に固定。GET 系の actor は他の GET 経路と同じくクエリ必須。投稿者は合成ヘッダー（comments/approve と同じ受け取り）。`before_seq`・`limit` は契約の `ListMessagesQuery` に既にある形に合わせて受理。未知 workspace の一覧は空配列（存在検査でなく検索結果）、未知 channel の情報・投稿・取得は 404。
4. 足りなかった情報・検証手段: 応答 JSON の形は Issue に無いため保存物（`Channel`・`StoredMessage`）の直列化をそのまま返す形とした（seq が含まれ、追加のラップを発明しない）。
5. 改善案: `docs/dev-environment.md` に「`scripts/agent-gh dev git add/commit` にも `XDG_CACHE_HOME=/tmp/xdg-cache` が要る（hk の cargo_check/clippy が mbx で失敗する）」を足す（326 の改善案の追補）。

使ったスキル: implement-issue（`.agents/skills/implement-issue`）
