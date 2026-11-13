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

## 実装者メモ（round 3）

1. 詰まった箇所: crates/pelway-db/migrations/0003_rls_and_admin_audit.sql: `pelway_audit_write` の関数本体を `BEGIN ... END` なしの 1 文で書いて plpgsql の構文エラー（42601）になった。ブロック形式に直した。
2. 曖昧だった文: 「separate administrative execution from the audit store with privileges that the function's effective role cannot use to alter/delete audit rows」（PR #115 の pelway-review レビュー must-fix）。
3. 仮定と解釈: 開発トポロジー（マイグレーション=スーパーユーザー）ではスーパーユーザー所有の関数は何でもできてしまうため、実行主体を非スーパーユーザーに分離するしかない、と仮置きした。選んだ解釈: 実行主体 `pelway_admin_executor`（BYPASSRLS + tenant 表 DML のみ）と監査表所有者 `pelway_audit_writer` の 2 ロール分離。選ばなかった解釈: ステートメントの構文ブラックリスト（抜けが出る）。
4. 足りなかった情報・検証手段: 失敗した管理試行（tamper 等）は単一文トランザクションのロールバックで監査にも残らない。試行の記録には PROCEDURE と COMMIT 分割が必要で、この PR では対象外とした。
5. 改善案: docs/adr/0005-postgres-row-level-security.md の「Consequences」に「管理経路の監査は追記のみで、実行ロールは監査表に権限を持たない」を足す。

## 実装者メモ（round 4）

1. 詰まった箇所: crates/pelway-db/migrations/0003_rls_and_admin_audit.sql: SECURITY DEFINER 手続き内の COMMIT が invalid transaction termination（2D000）で拒否される（psql 実験で確定。SECURITY INVOKER なら可）。監査とステートメントの同一手続きを断念し、監査行を別文で先に確定し、実行手続きが監査行の存在を検査して拒否する構成に変えた。
2. 曖昧だった文: 「操作失敗時も監査記録が確定する経路に分離してください」（PR #115 の pelway-review レビュー must-fix）。
3. 仮定と解釈: 「経路の分離」はヘルパーの 2 文構成（監査書き込み → 実行）で満たすと解釈した。実行手続きが同 reason の監査行を検査して拒否するため、監査なしの実行は DB 側で不可能。選ばなかった解釈: 監査行の捏造（pelway_audit_write の直接呼び出し）の禁止 — 捏造も db_user・reason・時刻として記録に残り、利用の秘匿には使えないため。
4. 足りなかった情報・検証手段: 既存ロールの属性揃え込み（ALTER）の直接試験。並行テストが属性の一時的な破壊と干渉する恐れがあるため、マイグレーション後の属性値の表明検査のみ行った。
5. 改善案: docs/adr/0005-postgres-row-level-security.md の「Consequences」に「監査は実行の前に別文として確定し、実行手続きは監査行の存在を検査する」を足す。

## 実装者メモ（round 5）

1. 詰まった箇所: crates/pelway-db/src/lib.rs: `Ok(tx);`（セミコロン付き）を oldString にして編集が一致せず、ファイルの実状（`Ok(tx)`）を読み直してから admin_exec の削除をやり直した。また main の取り込みで Cargo.toml の crate 一覧コメントが衝突し、両側の文言を併合して解決した。
2. 曖昧だった文: 「管理ロールの監査の関数・ヘルパーは含めない（含める場合も監査の保証を主張しない）」（Issue #100 の split-plan コメント、hatai）。
3. 仮定と解釈: 監査の関数・ヘルパー（admin_access_log、pelway_audit_write、pelway_admin_exec、admin_exec/admin_query、pelway_admin_executor/pelway_audit_writer）をすべて外し、管理ロールはテナント表への直接 DML 権限を持つ素朴な形に戻す、と解釈した。ロールの分離（pelway_app と pelway_admin）と属性の明示揃え込みは RLS の本体として残した。選ばなかった解釈: 管理ロール自体も #149 に寄せる（ADR 0005 の Consequences がロールの存在自体を求めているため）。
4. 足りなかった情報・検証手段: 4 回目の must-fix（同じ reason の使い回し）への対応。監査機構ごと #149 に分離したため対応不要としたが、#149 の設計では再発しないようエスカレーションコメントの 4 つの穴を引き継ぐ必要がある。
5. 改善案: docs/adr/0005-postgres-row-level-security.md の「Consequences」の監査の文に「設計と実装は #149」を追記する。
