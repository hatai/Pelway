# pgschema 方式と sqlx migrate の比較検討（Issue #214）

pelway-db のスキーマ管理について、現行の sqlx migrate（手続き的・連番の
移行ファイルの積み重ね）と、Buzz が採る宣言的管理（pgschema 方式: 望ましい
状態を SQL で宣言し、ツールが差分を生成する）を比較する。将来の移行要否を
判断する材料であり、移行の提案ではない。

## 調査の一次情報

- `pgplex/pgschema`（ツール本体）: README、`docs/cli/`（plan・apply・ignore）、`docs/syntax/unsupported.mdx`、リリース一覧
- `block/buzz`（実例）: `docs/database.md`、`migrations/AGENTS.md`、`schema/schema.sql`、`.pgschemaignore`、`scripts/reconcile-schema-after-pgschema.sql`、`crates/buzz-db/src/runtime/migration.rs`、ツリー一覧
- `launchbadge/sqlx`: `sqlx-cli/README.md`、`sqlx-core/src/migrate/migrator.rs`、`sqlx-postgres/src/migrate.rs`
- Pelway 側: `crates/pelway-db/migrations/`（0001〜0003）、`crates/pelway-db/src/lib.rs`、ADR 0005・0006

## 両方式のモデル

- **sqlx migrate（現行）**: `migrations/` に連番の SQL を足す方式。適用済みは
  `_sqlx_migrations` 表に version・checksum・success を記録し、適用済み
  ファイルを書き換えると checksum 不一致（`VersionMismatch`）で失敗する。
  `-r` で up/down の対にすると `migrate revert` で戻せる。`sqlx::migrate!` が
  ファイルをバイナリに埋め込み、アプリ起動時の `migrate(pool)` で適用する
  （Pelway はこれ）。DB 実態とファイル群の差分（drift）を見る機能はない。
  バイナリが知らない version が DB にあれば `VersionMissing` で起動を拒む。
- **pgschema（宣言的）**: `dump → 定義ファイルを編集 → plan → apply`。
  望ましい状態の SQL（`\i` で子ファイルを include できる）を埋め込み
  PostgreSQL に適用して正規化し、対象 DB の実スキーマと diff して、依存順に
  並べた移行 DDL を plan として出す。移行履歴表を持たない。apply は
  schema fingerprint で「plan 後に DB が変わっていないか」を検査し、
  `lock_timeout`・トランザクション適否の自動判定（`CREATE INDEX
  CONCURRENTLY` 等は非トランザクション）を持つ。定義に無いオブジェクトは
  計画上 DROP 対象になり、`.pgschemaignore`（TOML、種別ごとの
  `*`/`!` パターン）で管理対象から外す。

## block/buzz での実例

Buzz（`docs/database.md`）は**両方を併用している**。`migrations/` に sqlx
形式の連番 SQL（0001〜0060 超）を持ち、並行して `schema/` に宣言的な
「望ましい状態」を持つ（`tables/public/<表>.sql` に 1 表 1 ファイル、
`types/`・`functions/`・`triggers/`・`partitions/`・`seeds/` を
`schema.sql` が `\i` で合成）。vendored な `bin/pgschema`（1.7.4）を
`apply --file schema/schema.sql` で新規ブートストラップに使い、既存 DB は
migrations で追いつく。1 つの論理的変更が `migrations/`・
`schema/tables/`・`schema.sql`・`schema/seeds/`・
`scripts/reconcile-schema-after-pgschema.sql`・（gateway 表なら）
`crates/buzz-push-gateway/migrations/` の最大 6 か所にまたがる
（Rule 4「Change every schema location」）。

pgschema が保持しないものを冪等に収束させる reconcile スクリプトを全
apply 呼び出し側が実行する決まりで、具体例として seed の INSERT（pgschema
は INSERT をスキップ）、一部の storage parameter（reloptions）、
パーティションの ATTACH（pgschema は子表を standalone CREATE TABLE として
出し、親 trigger の複写が付くことがある）を手当てしている。ランタイムが
作る月次パーティションは `.pgschemaignore` で exempt しないと
`DROP TABLE ... CASCADE` で落とされる。

併用は「目標アーキテクチャへの移行途中」の姿で、新規の trigger・関数・
外部キーは禁止、out-of-band の宣言的適用は plain な table+index に限る
制約を課している。両経路の catalog が一致するかの parity テストはあるが、
完全な drift ゲートは未整備と明記されている。Buzz は RLS を使わず、
テナント境界はアプリ側の community admission で守る。

## 比較

### RLS ポリシーの扱い

pgschema はポリシーを対応オブジェクトとして明記する（USING/WITH CHECK、
PERMISSIVE/RESTRICTIVE、`ENABLE/DISABLE/FORCE ROW LEVEL SECURITY`）。
Pelway の `0003_rls_policies.sql` の内容は宣言的に書ける。**ただしロールは
対象外**で、`CREATE ROLE`/`ALTER ROLE ... BYPASSRLS` は非対応
（クラスタレベル）。`TO pelway_app` のような既存ロールへの参照は
plan の仮 DB で stub されるため使える。GRANT/REVOKE・列単位・default
privileges（`IN SCHEMA` 限定）は対応。`ALTER ... OWNER TO` は無視される。

Pelway の `0001_roles.sql`（`pelway_app`/`pelway_admin`、BYPASSRLS 付与）は
pgschema では表現できず、移行しても手続き的な残りが必ず残る。Buzz は RLS
を使わないため、RLS 前提の宣言的管理の実績は Buzz からは取れない。

### レビューのしやすさ

宣言的は「あるべき全体像」が 1 表 1 ファイルで読め、diff はツールが生成
する plan で確認する。手続き的は変更の意図がファイルそのものだが、現状を
知るには履歴を頭の中で積み上げる必要がある。Pelway は現時点で 3 ファイル
90 行程度で全体像は grep できる規模だが、ポリシー・監査まわりが増えると
乖離は見えにくくなる（Issue の背景どおり）。Buzz は 60 超の履歴を抱え
「2 つの正本」を維持しており、宣言的の可読性の対価として 1 変更あたり
最大 6 ファイルを正しく更新しなければならない。

### 既存移行との共存

共存は Buzz が実証しているが、安くはない。Pelway で採るなら同型の
「新規 DB は宣言的、既存 DB は移行ファイル」＋収束スクリプト＋ parity
テストが要るか、一括切替（dump → 宣言化 → migrations 凍結）になる。
埋め込み `sqlx::migrate!` でアプリ起動時に適用する現行経路は、宣言的に
替えると外部 CLI（Go バイナリ）を配布・実行する運用に変わる。管理外
オブジェクトは apply で DROP されるため、`_sqlx_migrations` 自体や
運用で生まれる表は `.pgschemaignore` 管理が必要。

### ツールの成熟度とライセンス

| 項目 | pgschema | sqlx migrate |
|---|---|---|
| 言語・形態 | Go の CLI（Rust からは外部プロセス） | Rust ライブラリ、バイナリ埋め込み可 |
| ライセンス | Apache-2.0（機能制限なし、Bytebase が後援） | MIT/Apache-2.0 |
| 歴史 | 2025-06 公開、v1.14.1（2026-11-08）。月 1 回以上のリリース | エコシステム標準、実績多数 |
| 規模 | 約 1,086 stars | — |
| 本番実績 | Block（Buzz）がブートストラップと併用運用で採用 | Pelway 含め広い |

若いが開発は活発で、Buzz の運用が実運用の具体例を提供する。一方、
reconcile スクリプトが必要な食い違い（seed・reloptions・partition attach）
が実在し、Pelway が当たるとは限らないが「差分が常に正しい」とは言えない。

## 結論（検討結果の記録。採否は hatai が決める）

**現行維持（sqlx migrate）。pgschema は将来の再評価候補として置く。**
「併用」は Buzz と同型の 2 正本を意味し、現規模では保守コストが効果を
上回ると判断する。理由:

1. **解くべき痛みがまだ小さい。** pgschema の実利は drift の可視化と
   「全体像の可読性」だが、3 ファイル規模では両方とも不要に近い。
2. **保安上の要であるロールが宣言的の外に残る。** `pelway_admin` の
   BYPASSRLS は pgschema の守備範囲（スキーマレベル）に入らず、完全な
   宣言化は原理的に不可能。RLS ポリシー自体は宣言できるため、移行する
   意味はあるが、移行しても必ず手続き的な残り（ロール管理）が残る。
3. **Buzz の教訓は「併用は移行期のコスト」であること。** 彼らは out-of-band
   適用という規模由来の要請があって 2 正本を引き受けている。Pelway に
   その要請は無く、真似るとルール 4（全 schema 位置の更新）と収束
   スクリプトの保守だけを輸入することになる。
4. **sqlx 側の弱み（drift 不可視）は運用で補える。** 手書き変更を
   DB に入れない約束と、`pg_dump --schema-only` のスナップショットを
   レビューに添える程度で、現規模の実害は防げる。

**再評価のきっかけ（どれかが立ったらこの文書を見直す）:**

- 移行ファイル・ポリシーの数が増えて全体像の確認がレビューで辛くなる
- pgschema がクラスタレベル（ロール）に対応する、または同等の Rust
  組み込み可能なツールが出る
- expand/contract や out-of-band 適用を採る規模・運用形態になる
- Pelway でも RLS ポリシー・GRANT が増え、宣言的一覧の価値が
  2 正本のコストを上回る

（使ったスキル: `.agents/skills/research`（一次情報はすべて GitHub 上の
一次ソース）、`.agents/skills/domain-modeling`）

## 出典

- [pgplex/pgschema README, 2026/11] pgplex. "README.md." GitHub. https://github.com/pgplex/pgschema
- [pgschema Unsupported, 2026/11] pgplex. "docs/syntax/unsupported.mdx." GitHub. https://github.com/pgplex/pgschema/blob/main/docs/syntax/unsupported.mdx
- [pgschema plan, 2026/11] pgplex. "docs/cli/plan.mdx." GitHub. https://github.com/pgplex/pgschema/blob/main/docs/cli/plan.mdx
- [pgschema apply, 2026/11] pgplex. "docs/cli/apply.mdx." GitHub. https://github.com/pgplex/pgschema/blob/main/docs/cli/apply.mdx
- [pgschema ignore, 2026/11] pgplex. "docs/cli/ignore.mdx." GitHub. https://github.com/pgplex/pgschema/blob/main/docs/cli/ignore.mdx
- [Buzz database.md, 2026/11] Block. "docs/database.md." GitHub. https://github.com/block/buzz/blob/main/docs/database.md
- [Buzz migrations AGENTS, 2026/11] Block. "migrations/AGENTS.md." GitHub. https://github.com/block/buzz/blob/main/migrations/AGENTS.md
- [Buzz schema.sql, 2026/11] Block. "schema/schema.sql." GitHub. https://github.com/block/buzz/blob/main/schema/schema.sql
- [Buzz .pgschemaignore, 2026/11] Block. ".pgschemaignore." GitHub. https://github.com/block/buzz/blob/main/.pgschemaignore
- [Buzz reconcile, 2026/11] Block. "scripts/reconcile-schema-after-pgschema.sql." GitHub. https://github.com/block/buzz/blob/main/scripts/reconcile-schema-after-pgschema.sql
- [Buzz migration.rs, 2026/11] Block. "crates/buzz-db/src/runtime/migration.rs." GitHub. https://github.com/block/buzz/blob/main/crates/buzz-db/src/runtime/migration.rs
- [sqlx-cli README, 2026/11] launchbadge. "sqlx-cli/README.md." GitHub. https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md
- [sqlx migrator.rs, 2026/11] launchbadge. "sqlx-core/src/migrate/migrator.rs." GitHub. https://github.com/launchbadge/sqlx/blob/main/sqlx-core/src/migrate/migrator.rs
- [Pelway ADR-0005] docs/adr/0005-postgres-row-level-security.md
- [Pelway ADR-0006] docs/adr/0006-db-queue-choice.md
