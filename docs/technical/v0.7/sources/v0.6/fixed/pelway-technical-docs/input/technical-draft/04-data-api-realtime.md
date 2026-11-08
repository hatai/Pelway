# 04 データモデル・API・リアルタイム同期と再接続

## 1. 主なテーブル（PostgreSQL）
すべての行に `workspace_id`（会社の分離）を持たせる。ローカル構成でも最初から入れる。

| テーブル | 主な列 | メモ |
|---|---|---|
| workspaces | id, name, locale_default | |
| members | id, workspace_id, kind(human/bot), display_name, locale, status | Botも同じ表 |
| bot_profiles | member_id, role, instructions_ref, tool_policy_id, budget_limit | |
| channels | id, workspace_id, name, kind, last_seq | `last_seq` は発言順の採番に使う |
| channel_members | channel_id, member_id, role | |
| messages | id, channel_id, seq, author_id, client_msg_id, body(Markdown), thread_root_id, edited_at, deleted_at | `(channel_id, seq)` と `(author_id, client_msg_id)` が一意 |
| tasks | id, channel_id, source_message_id, assignee_id, state(9状態・03), stop_reason, decision_owner, decision_topic, lease_owner, lease_until, fencing_token, version | 03の状態 |
| task_steps | task_id, step_no, kind, input_ref, output_ref, status | 再開の単位 |
| approvals | id, task_id, action_hash, requested_by, approver_rule, decided_by, decision, decided_at | ハッシュに結び付く |
| external_actions | id, task_id, approval_id, idempotency_key(承認ごとに1つ), action_hash, status(pending/success/failed/outcome_unknown), provider_ref | 一意キー＝idempotency_key |
| audit_events | id, workspace_id, at, actor_id, kind, subject, payload, prev_hash, hash | 追記専用 |
| attachments | id, message_id, storage_key, size, sha256 | 本体はファイル保存 |

## 2. API
- HTTPS（JSON）：取得・作成・更新。ページングは `seq` を基準（`before_seq` / `after_seq`）。
- WebSocket（`/v1/stream`）：サーバーからのイベント配信と、軽い操作（入力中表示など）。
- 型はRustで定義し、OpenAPIかJSON Schemaを出して TypeScript の型を生成（02）。

主なエンドポイント（初版）：
`POST /v1/channels/{id}/messages`（`client_msg_id` 必須）、`GET /v1/channels/{id}/messages?before_seq=`、`POST /v1/tasks/{id}/cancel`、`POST /v1/approvals/{id}/decide`（`action_hash` 必須。食い違えば409）、`GET /v1/sync?since=`。

## 3. リアルタイム同期と再接続
参考：Zulipのイベントキュー（[get-events](https://zulip.com/api/get-events)：`queue_id` と `last_event_id`）、Matrixの `/sync`（`since` トークン）。

- **順序**：チャンネルごとに `seq` をDBで採番（`UPDATE channels SET last_seq = last_seq + 1 … RETURNING`）。発言順はこの1か所で決まる。
- **送信**：クライアントは `client_msg_id`（UUIDv7）を付けて送る。再送しても一意制約で1件にしかならない。画面には「送信中」で先に出し、確定の `seq` が来たら置き換える。
- **配信**：サーバーは `{channel_id, seq, event}` を送る。全体の再接続用に、ワークスペース単位の `stream_pos`（単調増加）も付ける。
- **再接続**：クライアントは最後に受けた `stream_pos` を送り直す。
  - サーバーがその位置からの差分を持っていれば、差分を流す。
  - 古すぎる（保持期間外）場合は `resync_required` を返し、クライアントは開いているチャンネルの `after_seq` で取り直す。
  - チャンネルの `seq` に抜けを見つけたら、その範囲だけHTTPで取得する（欠番の補完）。
- **再接続の間隔**：指数バックオフ＋ゆらぎ（上限30秒・仮定）。オフライン中に書いた送信は `client-core` のキューに残し、つながったら順に送る。
- **複数のサーバープロセス**（将来）：PostgreSQLの LISTEN/NOTIFY で配信を知らせる（公開 technical.md にある案と同じ）。ローカル初版は1プロセス。

## 4. クライアントのキャッシュ
- ネイティブ：SQLiteに、開いたチャンネルの直近の範囲と未送信キュー。
- Web：IndexedDBに同じもの（`client-core` が保存先を差し替える）。
- キャッシュはあくまで表示の高速化用で、正しい値はサーバー。承認の判断はキャッシュでは行わない。

## 5. 国内限定との関係
全テーブル・ファイル・キャッシュ・ログの保存先は国内設置ホスト上のローカルボリューム。保存先の一覧と越境経路は 10-domestic-only.md。社内wiki・外向けQ&Aの自動生成は範囲外（プロダクト R14）なので、その表は置かない。
