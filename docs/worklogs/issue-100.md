# Issue #100 の作業ログ

## 実装者メモ（round 1）

1. 詰まった箇所: crates/pelway-db/tests/common/mod.rs: 並列実行で `tuple concurrently updated`（ALTER ROLE の競合）。PostgreSQL の advisory lock がデータベースローカルであることを実測で確認（別 DB から同じ ID の `pg_try_advisory_lock` が取得できた）し、ロールはクラスタ共有なので接続先 DB 上の advisory lock でマイグレーション全体を直列化した。
2. 詰まった箇所: crates/pelway-db/src/lib.rs: `sqlx::migrate!` が追加済みのマイグレーション 0003 を埋め込まない（プロシージャルマクロは migrations/ のファイル追加を検知できない。sqlx のドキュメント記載の既知の制限）。build.rs の `cargo:rerun-if-changed=migrations` で解決した。
3. 曖昧だった文: 「トランザクションごとに現在の `workspace_id` を設定し、各テーブルのポリシーで絞り込む」（Issue #100 本文「やること」）。
4. 仮定と解釈: 「各テーブル」= 04 §2 の全テナント表ではなく、RLS を示す代表表（workspaces/channels/messages）+ 監査表と解釈した。全表の確定 DDL は「確定したDBスキーマではない」（04 §2 冒頭）に反し、Issue の範囲も超えるため選ばなかった。workspace_id の DB 型も文書で未決なので、契約型 WorkspaceId（文字列 newtype）に合わせて TEXT にした。
5. 足りなかった情報・検証手段: 04 §2 に workspace_id の DB 型（uuid/text）の記述がなく、契約型の実装（crates/pelway-contract/src/ids.rs の WorkspaceId）から推定した。
6. 改善案: docs/technical/v0.7/body/04-data-api-realtime.md の「永続データ」冒頭に「`workspace_id` の DB 型は TEXT（契約型 WorkspaceId に合わせる）」を足す。
