# Issue #100 の作業ログ

## 実装者メモ（round 1）

1. 詰まった箇所: crates/pelway-db/tests/common/mod.rs: 並列実行で `tuple concurrently updated`（ALTER ROLE の競合）。PostgreSQL の advisory lock がデータベースローカルであることを実測で確認（別 DB から同じ ID の `pg_try_advisory_lock` が取得できた）し、ロールはクラスタ共有なので接続先 DB 上の advisory lock でマイグレーション全体を直列化した。
2. 詰まった箇所: crates/pelway-db/src/lib.rs: `sqlx::migrate!` が追加済みのマイグレーション 0003 を埋め込まない（プロシージャルマクロは migrations/ のファイル追加を検知できない。sqlx のドキュメント記載の既知の制限）。build.rs の `cargo:rerun-if-changed=migrations` で解決した。
3. 曖昧だった文: 「トランザクションごとに現在の `workspace_id` を設定し、各テーブルのポリシーで絞り込む」（Issue #100 本文「やること」）。
4. 仮定と解釈: 「各テーブル」= 04 §2 の全テナント表ではなく、RLS を示す代表表（workspaces/channels/messages）+ 監査表と解釈した。全表の確定 DDL は「確定したDBスキーマではない」（04 §2 冒頭）に反し、Issue の範囲も超えるため選ばなかった。workspace_id の DB 型も文書で未決なので、契約型 WorkspaceId（文字列 newtype）に合わせて TEXT にした。
5. 足りなかった情報・検証手段: 04 §2 に workspace_id の DB 型（uuid/text）の記述がなく、契約型の実装（crates/pelway-contract/src/ids.rs の WorkspaceId）から推定した。
6. 改善案: docs/technical/v0.7/body/04-data-api-realtime.md の「永続データ」冒頭に「`workspace_id` の DB 型は TEXT（契約型 WorkspaceId に合わせる）」を足す。

## 実装者メモ（round 2）

1. 詰まった箇所: crates/pelway-db/tests/rls.rs: main からのマージで rustfmt が import 行を並べ替えており、編集の対応を見誤って AC3 本体だけ差し替わった状態でコンパイルし、`begin_admin_tx` 未解決で気づいた。import を修正してから続けた。
2. 曖昧だった文: 「管理操作を監査付きの DB 関数に限定するなど、ロール利用自体が記録される設計にしてください」（PR #115 の pelway-review レビュー must-fix）。
3. 仮定と解釈: BYPASSRLS はポリシーを回避するが GRANT は回避しない、を前提に「pelway_admin から表への直接権限をすべて剥奪し、監査付き SECURITY DEFINER 関数（pelway_admin_exec / pelway_admin_query、1 文だけ受け付ける汎形）のみを許す」設計にした。選ばなかった解釈: 表ごとの型付き管理関数を並べる（移行・バックアップの任意文を扱えず関数が膨らむ）。監査記録の db_user には current_user ではなく session_user を記録する方を選んだ（SECURITY DEFINER の中では current_user が関数所有者になるため）。
4. 足りなかった情報・検証手段: 本番での関数所有者ロールの構成（表の所有者だと FORCE ROW LEVEL SECURITY が関数にも効く）。0003 に注意書きしたが、非スーパーユーザー所有者での実機検証はしていない。
5. 改善案: docs/adr/0005-postgres-row-level-security.md の「Consequences」に「管理ロールは表への直接権限を持たず、監査付き SECURITY DEFINER 関数だけを使う」を足す。
