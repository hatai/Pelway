# Pelway 将来技術調査：データ・バックグラウンド処理・共同編集

## 結論

MVP（最初に実データで試す版）の前に検証する価値が高いのは、River の Rust 版（ジョブキュー）、pgBackRest（バックアップ）、Postgres 内のハッシュ鎖と C2SP 形式の署名付きチェックポイント（監査）の 3 系統です。
いずれも「PostgreSQL を唯一の正本とし、別のブローカーや別の DB を増やさない」という Pelway の制約を崩さずに導入できるためです。
同期エンジン（ElectricSQL、PowerSync、Zero）は、どれも論理レプリケーションと別サービスを必要とするため、見送りとします。ElectricSQL は同期データのログを Postgres 外のファイルに持つことも確認しました（PowerSync と Zero の保存先は未確認）。
日本語 OCR は YomiToku が一次資料上の精度で最も強いものの、ソースコードとモデル重みが CC BY-NC-SA 4.0 であり、Pelway のような商用製品は製品版の商用ライセンス購入が前提になります。
商用ライセンスを買わない場合の代替は NDLOCR-Lite（CC BY 4.0）と PaddleOCR（Apache-2.0）で、業務文書での精度比較は自前で行う必要があります。

## この文書の前提

- 調査日は 2026/11/11〜12 です。スター数・最終 push 日・最新リリースは `gh repo view` と GitHub API で取得しました。各リポジトリの URL は末尾の出典一覧にあります。
- 判定の語は依頼どおり次の 3 つです。「MVP前に検証」は MVP 前に合成データで試作・測定するもの、「将来の候補」は必要が生じた時点で再評価するもの、「見送り」は現在の制約と衝突するため採らないものです。
- 「（Pelway 側の評価）」と書いた段落は、出典の内容ではなく本調査での当てはめです。出典の記述と区別してください。
- 確認できなかった事項は「未確認」と書きます。
- 日本語全文検索（pg_bigm、PGroonga、Tantivy、Meilisearch、RAGFlow）は別調査の範囲のため扱いません。
- 何もインストール・実行していません。リポジトリのファイルも編集していません。

### 参照した Pelway 仕様（docs/technical/v0.7/body/ 配下）

- 01 §永続化と正しさ：業務の正本は server と PostgreSQL です。状態変更、監査、配信用 outbox（配信待ちイベントを貯める表）は同じ DB トランザクションで確定します。ローカル初版には別のメッセージブローカーを持ち込みません。
- 01 §全構成に適用する情報取扱い制御：将来の索引/RAG（検索拡張生成）は初期必須機能に戻しません。
- 02 の RAG 行：「初版必須ではない。必要時に国内モデル＋pgvectorを比較」とあります。
- 04 §順序と再接続：チャンネル内 `seq`、workspace 内の commit 順 cursor、REPEATABLE READ の単一スナップショットからの再同期、転送時の現権限の再検査を定めています。
- 04 §同時更新と成果物の耐久化：監査の `prev_hash` は workspace 内の鎖で、「管理者の改ざん耐性を保証するものではない」とあります。DB と volume の backup は同じ整合取得点を持つ手順が必要で、方式は未決です。
- 12 §4 表の「検索・embedding・RAG」行：index 作成前と結果返却前に検査し、件数や候補語からも無権限 source の存在を漏らしません。
- 12 §8.2：削除と復旧の順序です。復旧時は、復旧 DB と独立に保全した削除・制限・撤回記録を先に再適用します。
- 13 §3：R03（索引・embedding は独立 TTL なし、source と同じ期限）、R15（国内 backup は 1 日 1 回、作成から最大 14 日）、R16（削除・失効記録）。
- 13 §4：D01（利用停止は受付トランザクションで確定）、D03（オンライン物理削除は 24 時間以内の内部目標）。
- 08 継承する技術選定：A1（Rust の非 WebView GUI として Slint）、A2（Web は TypeScript/Octane、状態と同期は Rust/WASM）、A3（Rust tokio/axum＋sqlx/PostgreSQL）、A4（seq/client_msg_id と snapshot 再同期）。
- 仕様には共有キャンバス・文書の同時編集、添付ファイルの文書解析・OCR、ベクトル検索の要件はありません（v0.7 本文を grep で確認）。添付に関する記述は、hash 検査、承認対象としての添付 hash、quota に限られます。したがって領域 3・4・5 は「将来の候補」の評価です。

## 判定一覧

| 領域 | 候補 | 判定 | 一行の理由 |
|---|---|---|---|
| 1 キュー | River（Rust 版 `riverqueue`） | MVP前に検証 | sqlx のトランザクションに乗せてジョブを投入でき、outbox と同時 commit できる。ただし crate は新しい |
| 1 キュー | apalis（`apalis-postgres`） | 将来の候補 | MIT・Rust ネイティブで成熟度は高いが、同一トランザクション投入は未確認 |
| 1 キュー | pgmq | 将来の候補 | SQL だけで導入でき、SQS 型の可視性タイムアウトを持つ。ジョブ管理機能は薄い |
| 1 キュー | 自前 `FOR UPDATE SKIP LOCKED` | MVP前に検証（比較基準として） | 依存ゼロ。River と比べる基準線として持つ |
| 1 実行基盤 | Restate | 見送り | 状態を自前の RocksDB とログに持ち、Postgres が正本でなくなる。BSL 1.1 |
| 1 実行基盤 | Hatchet | 見送り | 別のコントロールプレーンが必要。Rust SDK の記載なし |
| 1 移行 | sqlx 組込み migrate | MVP前に検証（採用済みスタック） | sqlx 0.9 に同梱。別ツールを増やさない |
| 2 同期 | ElectricSQL / PowerSync / Zero | 見送り | 論理レプリケーションと別サービスが必要。Electric は shape ログを Postgres 外のファイルに持つ（他 2 製品は未確認） |
| 3 CRDT | Loro | 将来の候補 | Rust コアで、履歴を切り詰める shallow snapshot がある |
| 3 CRDT | Yrs | 将来の候補 | Yjs 互換の Rust 実装。削除内容の GC の挙動は未確認 |
| 3 CRDT | Automerge | 見送り寄りの将来候補 | Rust API は低水準と README 自身が述べる。全履歴保持と削除要件の両立は未確認 |
| 4 ベクトル | pgvector | 将来の候補 | Postgres 内で完結し削除は通常の DELETE。ただし近似索引ではフィルタが索引走査の後に効く |
| 4 ベクトル | pgvectorscale | 将来の候補 | ラベル付きフィルタ検索あり。PostgreSQL License |
| 4 ベクトル | VectorChord | 見送り | AGPLv3 か ELv2 の二択で、配布形態との相性が悪い |
| 5 文書解析 | YomiToku | 将来の候補（商用ライセンス購入が前提） | 日本語の表・手書きで一次資料上の最良値。OSS 版は非商用限定 |
| 5 文書解析 | NDLOCR-Lite | 将来の候補 | CC BY 4.0、CPU で動作。評価データは国立国会図書館の蔵書（歴史的活字）中心 |
| 5 文書解析 | PaddleOCR | 将来の候補 | Apache-2.0。日本語を含む 50 言語の単一モデル。日本語業務文書の一次比較なし |
| 5 文書解析 | Docling / Xberg（旧 Kreuzberg） | 将来の候補 | 文書構造化の枠組み。日本語 OCR 品質は内部の OCR エンジン次第 |
| 5 文書解析 | MarkItDown | 見送り | 画像キャプションと音声書き起こしが Web 上のサービスに依存 |
| 5 文書解析 | Tesseract jpn | 将来の候補（優先度低） | 日本語の一次比較は手書き 10 ページ（査読なし）だけで CER 97.57%。活字の業務文書は未検証 |
| 6 監査 | Postgres 内ハッシュ鎖＋C2SP 署名付きチェックポイント | MVP前に検証 | 既存の `prev_hash` 鎖の最新値に署名し、DB 管理者と別権限で保管すれば、鎖全体の書換えを検出できる |
| 6 監査 | Tessera | 将来の候補（設計参照） | Go ライブラリ。tlog-tiles 形式の参照実装 |
| 6 監査 | Trillian | 見送り | 保守モードで、新規利用者には Tessera を勧めている |
| 6 監査 | immudb | 見送り | 別 DB が増える。BSL |
| 7 backup | pgBackRest | MVP前に検証 | MIT、暗号化、POSIX リポジトリ、検証付き復元。保持期間の挙動に注意が必要 |
| 7 backup | WAL-G | 将来の候補 | Apache-2.0、libsodium 暗号化。単一 Go バイナリ |
| 7 backup | Barman | 将来の候補 | GPL-3.0、`RECOVERY WINDOW OF` 指定がある |

---

## 1. Postgres を使うジョブキューと永続実行

### Pelway の課題

01 §永続化と正しさは、状態変更・監査・outbox を同一トランザクションで確定し、ブローカーを持ち込まないと定めています。
直近の決定で、LISTEN/NOTIFY は起床通知（ワーカーを起こす合図）だけに使い、Temporal は採らないことになりました。
したがって、候補を分ける基準は「ジョブの投入を、状態・監査・outbox と同じ sqlx トランザクションに含められるか」です（Pelway 側の評価）。

### River（Rust 版 `riverqueue`）　判定：MVP前に検証

- **概要**：River は Postgres または SQLite を使うバックグラウンドジョブ基盤で、Rust 実装は Tokio 上で動き、他言語版と DB スキーマとジョブ形式を共有します [River Rust README (2026/11)]。
- **同一トランザクション投入**：`client.insert(args)` に `.tx(&mut transaction)` を付けると、アプリのデータと同じトランザクションでジョブを投入でき、ジョブはそのトランザクションの commit 後にだけ実行可能になります [River Rust README (2026/11), Transactions]。
- **sqlx との関係**：API は SQLx のプールとトランザクションの型を使い、SQLx 0.9 との互換が必要です [River Rust README (2026/11), Installation]。sqlx の最新安定版は 0.9.0 です [crates.io sqlx (2026/11)]。
- **機能**：一意ジョブ、定期ジョブ、再開可能ジョブがあります [River Rust README (2026/11)]。
- **ライセンスと保守**：MPL-2.0、スター約 5,885、最終 push 2026-11-10、Go 本体の最新リリース v0.51.2（2026-11-09）です [GitHub riverqueue/river (2026/11)]。
- **成熟度の懸念**：crates.io の `riverqueue` は 0.6.0、累計ダウンロード 2,754 件です [crates.io riverqueue (2026/11)]。Go 版に比べ Rust 版は利用実績が少ないと読めます（Pelway 側の評価）。
- **Pelway への当てはめ（Pelway 側の評価）**：outbox 行の書込みと同じトランザクションで配信ジョブを積めるため、01 の規則と一致します。MPL-2.0 はファイル単位のコピーレフトなので、River のファイルを改変しない限り Pelway 本体への影響は小さいと考えます（法的確認は未実施）。LISTEN/NOTIFY を起床通知だけに限る運用ができるかは未確認です。
- **検証項目**：(a) workspace 行ロック（04 §順序と再接続）と同じトランザクションでの投入、(b) リース切れ時の再実行と、外部副作用済みステップを再実行しない制御（04 §同時更新と成果物の耐久化）、(c) Go 版と同じスキーマを Pelway のマイグレーションでどう管理するか。

### apalis（`apalis-postgres`）　判定：将来の候補

- **概要**：Rust 用の型安全なバックグラウンド処理ライブラリで、Redis・PostgreSQL・SQLite・インメモリの保存先を持ちます [apalis README (2026/11)]。
- **機能**：再試行、再開可能ジョブ、cron、ワークフロー（`apalis-workflow`）があり、pgmq を保存先にする `apalis-pgmq` もあります [apalis README (2026/11)]。
- **ライセンスと保守**：MIT、スター約 1,496、最新 v1.1.1（2026-11-10）です [GitHub apalis-dev/apalis (2026/11)]。`apalis-postgres` 1.0.0 の累計ダウンロードは約 25 万件です [crates.io apalis-postgres (2026/11)]。
- **未確認**：呼び出し側の sqlx トランザクションの中でジョブを投入できるか。LISTEN/NOTIFY に依存するか。
- **Pelway への当てはめ（Pelway 側の評価）**：上の 2 点が確認できれば River の代替になります。確認できなければ outbox からの二段投入が必要になり、利点が薄れます。

### pgmq　判定：将来の候補

- **概要**：Postgres 上の軽量メッセージキューで、AWS SQS に似ています。可視性タイムアウト内で一度だけ消費者へ配る、メッセージを削除せずアーカイブする、といった機能を持ちます [pgmq README (2026/11)]。
- **導入方法**：拡張機能として入れるほか、SQL ファイルを psql で流し込むだけでも導入できます [pgmq README (2026/11), SQL Only]。
- **ライセンスと保守**：PostgreSQL License、スター約 5,393、最新 v1.13.1（2026-10-20）です [GitHub pgmq/pgmq (2026/11)]。Rust crate `pgmq` 0.33.7 があります [crates.io pgmq (2026/11)]。
- **Pelway への当てはめ（Pelway 側の評価）**：キュー操作は SQL 関数呼び出しなので、同一トランザクションに含めやすい構造です。一方、ジョブの一意性・定期実行・再試行方針は自前になります。アーカイブ機能を使うと本文を含むメッセージが残り、R01 などの保持期限の管理対象が増えます。

### 自前の `SELECT ... FOR UPDATE SKIP LOCKED`　判定：MVP前に検証（比較基準）

- **概要**：`SKIP LOCKED` を付けると、すぐにロックできない行は飛ばされます。公式文書は、これは一貫しないデータの見え方になるため汎用には向かないが、複数の消費者がキューのような表にアクセスする際のロック競合を避けるのに使える、と説明しています [PostgreSQL SELECT docs (2026/11), The Locking Clause]。
- **Pelway への当てはめ（Pelway 側の評価）**：04 で既に lock 順序（workspace 順序行 → subject → 承認/operation 行）を定めているため、キュー表の行ロックをこの順序へどう組み込むかを River と同じ試験で比べます。

### Restate（自前ホスト）　判定：見送り

- **概要**：単一バイナリで動く永続実行基盤です。状態は実行ディレクトリの `restate-data` に置き、主な耐久層はディスク上の複製ログ、状態キャッシュは RocksDB です [Restate Server overview (2026/11)]。同ページに Postgres など外部 DB の記載はありません。
- **ライセンス**：Business Source License 1.1 で、公開 Restate プラットフォームサービスとしての提供を禁じる追加利用許諾が付きます [restatedev/restate LICENSE (2026/11)]。
- **Rust**：Rust SDK があります [Restate README (2026/11)]。
- **Pelway への当てはめ（Pelway 側の評価）**：実行状態の正本が Postgres の外に移り、01 の「状態・監査・outbox を同一トランザクション」の原則と、12 §8.2 の削除列挙の対象が増えるため見送ります。

### Hatchet　判定：見送り

- **概要**：バックグラウンドタスク、AI エージェント、永続ワークフローのオーケストレーション基盤で、Python・TypeScript・Go・Ruby をサポートします [Hatchet README (2026/11)]。
- **依存**：自前ホストでは PostgreSQL が必須、RabbitMQ は任意です [Hatchet self-hosting docs (2026/11)]。
- **ライセンスと保守**：MIT、スター約 8,263、最新 v0.114.2 です [GitHub hatchet-dev/hatchet (2026/11)]。
- **Pelway への当てはめ（Pelway 側の評価）**：Rust SDK の記載がなく、Hatchet のコントロールプレーンが別サービスとして増えます。

### sqlx のマイグレーション　判定：MVP前に検証（採用済みスタックの確認）

- sqlx 0.9.0 と `sqlx-cli` 0.9.0 が 2026-05-21 に公開されています [crates.io sqlx (2026/11)] [crates.io sqlx-cli (2026/11)]。sqlx のリポジトリは `transact-rs/sqlx` に移っています（Apache-2.0、スター約 17,652）[GitHub transact-rs/sqlx (2026/11)]。
- 代替の `refinery` 0.10.0 もあります [crates.io refinery (2026/11)]。
- sqlx 組込みマイグレーションの機能（`migrate!` マクロ、可逆マイグレーション）の詳細は本調査では一次資料を確認しておらず、未確認です。
- **Pelway への当てはめ（Pelway 側の評価）**：River を採る場合、River のスキーマ（`riverqueue-migrate`）と Pelway のマイグレーションの適用順を一本化する手順が要ります。

---

## 2. リアルタイム同期・ローカルファースト

### Pelway の課題

04 §順序と再接続は、commit 順の cursor、REPEATABLE READ の単一スナップショットからの再同期、転送時の現権限の再検査、権限取消時の tombstone（削除を伝える印）を定めています。
08 の A2 は、Web の状態と同期を Rust/WASM で持つ方針です。

### ElectricSQL　判定：見送り

- **概要**：Postgres のリアルタイム同期エンジンです [Electric README (2026/11)]。Apache-2.0、スター約 10,428 です [GitHub electric-sql/electric (2026/11)]。
- **認可**：shape（同期する行の部分集合）の認可は通常の HTTP 認可として扱い、プロキシで WHERE 句を付けて行を絞ります。クライアントの追加条件は絞り込みにしか働きません [Electric Auth guide (2026/11)]。
- **依存**：Postgres の論理レプリケーションと REPLICATION 属性のロールが必要です。shape のログとメタデータはホストのファイルシステムにキャッシュされます [Electric Deployment guide (2026/11)]。
- **Pelway への当てはめ（Pelway 側の評価）**：shape ログが Postgres 外のファイルに残るため、12 §8.2 の削除列挙と D03 の物理削除の対象が増えます。権限の判定がプロキシの WHERE 句に移り、04 の「全 subject・権限・watermark を同じ読取スナップショットから作る」規則と二重管理になります。

### PowerSync　判定：見送り

- **概要**：同期エンジンのサーバ側で、Docker で自前ホストできます [PowerSync Self-hosting (2026/11)]。`powersync-service` は Apache-2.0、スター約 405 です [GitHub powersync-ja/powersync-service (2026/11)]。
- **依存**：Postgres の WAL を論理レプリケーションで読み、`wal_level = logical` と `powersync` という名前の publication が必要です [PowerSync Source DB setup (2026/11)]。
- **未確認**：Rust 向け SDK の有無。
- **Pelway への当てはめ（Pelway 側の評価）**：Electric と同じく、別サービスと論理レプリケーションが必要です。

### Zero（Rocicorp）　判定：見送り

- **概要**：Postgres 15 以上と論理レプリケーションが必要で、`zero-cache` サービスが論理レプリケーションスロットを作ります [Zero Connecting to Postgres (2026/11)]。
- **認可**：Zero は RLS のような権限機構を持たず、読取権限は利用者のクエリ側のフィルタで表現します [Zero Permissions (2026/11)]。
- **クライアント**：文書の例は React・SolidJS・TypeScript だけで、Rust クライアントの記載はありません [Zero Permissions (2026/11)]。
- **ライセンスと保守**：`rocicorp/mono` は Apache-2.0、スター約 3,439、最終 push 2026-11-11 です [GitHub rocicorp/mono (2026/11)]。
- **Pelway への当てはめ（Pelway 側の評価）**：A1（Slint）と A2（Rust/WASM の同期コア）のどちらにも組み込みにくく、見送ります。

### Replicache　判定：見送り

- GitHub の `rocicorp/replicache` はアーカイブ済みで、最終 push は 2022-05-07 です [GitHub rocicorp/replicache (2026/11)]。現在のソースは `rocicorp/mono` 配下にリンクされています [Replicache docs (2026/11)]。保守状況とライセンスは未確認です。

### 同期領域のまとめ（Pelway 側の評価）

4 候補はいずれも Pelway の seq/cursor/snapshot 方式を置き換える前提で作られています。
04 の自前方式は既に削除と権限取消を明示的に扱っているため、置き換えるより、Electric の「shape を HTTP で配り、プロキシで絞る」や Zero の「読取権限をクエリのフィルタで表す」という設計を参考に留めるのが妥当です。

---

## 3. 人と AI が共同編集するキャンバス・文書の CRDT

### Pelway の課題

v0.7 本文には共有キャンバスや同時編集の要件はありません。
将来これを入れる場合の主な衝突点は、CRDT（複数の利用者が同時に編集しても自動で合流できるデータ構造）が削除済みの内容や履歴を保持し続ける性質と、13 の D03（物理削除 24 時間以内）や R01（365 日）との両立です（Pelway 側の評価）。

### Loro　判定：将来の候補

- **概要**：CRDT ライブラリで、Rust・JS（WASM 経由）・Swift から使えます。リッチテキスト、移動可能な木構造、履歴のタイムトラベルを持ちます [Loro README (2026/11)]。
- **履歴の切り詰め**：shallow snapshot という書き出し形式で、保持する開始版を指定すると、その版より前の履歴をすべて切り詰めます [Loro docs encoding (2026/11)]。
- **ライセンスと保守**：MIT、スター約 6,289、`loro` crate 1.17.1（2026-10-22）です [GitHub loro-dev/loro (2026/11)] [crates.io loro (2026/11)]。
- **Pelway への当てはめ（Pelway 側の評価）**：shallow snapshot は、期限切れ・削除対象の編集履歴を物理的に消す手段の候補になります。切り詰め後も他の端末が古い更新を持ち込めるかは未確認で、検証が必要です。保存は Postgres の bytea 列にスナップショットと更新を入れる形が考えられますが、Loro 側の推奨方式は未確認です。

### Yrs（Yjs の Rust 移植）　判定：将来の候補

- **概要**：Yjs の Rust 移植で、C FFI（yffi）と WASM（ywasm）のラッパーを持ちます。スナップショットと awareness（カーソルなどの一時状態の共有）に対応します [y-crdt README (2026/11)]。
- **ライセンスと保守**：MIT、スター約 2,202、`yrs` 0.28.1（2026-11-04）です [y-crdt LICENSE (2026/11)] [crates.io yrs (2026/11)]。
- **GC**：Yjs の文書は `gc` オプションでガベージコレクションを有効にでき、無効にすると古い内容を復元できると説明されています [Yjs Y.Doc docs (2026/11)]。GC 後に何が残るかは同ページに記載がなく、未確認です。
- **Pelway への当てはめ（Pelway 側の評価）**：Web 側のエディタ部品（Yjs 対応のもの）が多いのが利点です。削除要件との両立は GC の詳細確認後に判断します。

### Automerge　判定：見送り寄りの将来候補

- **概要**：JSON に似た CRDT、圧縮形式、同期プロトコルを持ち、コアは Rust で WASM や C から使えます [Automerge README (2026/11)]。
- **Rust API**：README 自身が、Rust のコードは JavaScript ラッパーの裏側向けで API が低水準だと述べています [Automerge README (2026/11), Rust]。
- **ライセンスと保守**：MIT、スター約 6,728、`automerge` crate 0.12.0 です [GitHub automerge/automerge (2026/11)] [crates.io automerge (2026/11)]。
- **未確認**：履歴を切り詰める公式手段。

---

## 4. Postgres 内のベクトル検索

### Pelway の課題

01 と 02 は RAG を初版必須にせず、必要時に pgvector を比較するとしています。
13 の R03 は索引と embedding（文章を数値ベクトルに変換したもの）に独立の期限を持たせず、12 §4 は件数や候補からも無権限 source を漏らさないことを求めます。

### pgvector　判定：将来の候補

- **概要**：Postgres の拡張機能で、厳密検索と近似検索（HNSW、IVFFlat という 2 種類の索引）を持ちます。削除は通常の `DELETE` です [pgvector README (2026/11)]。
- **フィルタの注意点**：近似索引を使うと、WHERE 句のフィルタは索引を走査した後に適用されます。条件が 10% の行にしか合わない場合、HNSW の既定 `hnsw.ef_search = 40` では平均 4 行しか返りません。0.8.0 からの iterative index scan（足りなければ索引を追加で走査する機能）で緩和できます [pgvector README (2026/11), Filtering / Iterative Index Scans]。
- **テナント分離**：テナント分離にはリストパーティションか別表を使うよう勧めています [pgvector README (2026/11), Filtering]。
- **ライセンスと保守**：PostgreSQL License、スター約 23,648、最新タグ v0.8.7 です [pgvector LICENSE (2026/11)] [GitHub pgvector/pgvector (2026/11)]。
- **Pelway への当てはめ（Pelway 側の評価）**：権限のフィルタが索引走査の後に効くため、取得件数が不足しても「無権限の候補があった」ことを件数で漏らさないよう、上限件数と返却前の再検査を設計する必要があります。workspace ごとのパーティションは 04 の `workspace_id` 主キーと整合します。行を `DELETE` すれば検索対象から外れますが、HNSW 索引の物理的な掃除は VACUUM が必要で時間がかかると README は述べています [pgvector README (2026/11), Vacuuming]。

### pgvectorscale　判定：将来の候補

- **概要**：pgvector を補う拡張で、DiskANN 系の StreamingDiskANN 索引と、ラベルによるフィルタ付きベクトル検索を持ちます。Rust の pgrx で書かれています [pgvectorscale README (2026/11)]。
- **ライセンスと保守**：PostgreSQL License、スター約 3,162、最新 0.9.1 です [GitHub timescale/pgvectorscale (2026/11)]。
- **Pelway への当てはめ（Pelway 側の評価）**：ラベルフィルタを workspace や channel の権限ラベルに使えれば、post-filter の件数不足を減らせる可能性があります。ラベル数の上限などの制約は未確認です。

### VectorChord　判定：見送り

- **概要**：RaBitQ 量子化を使う Postgres のベクトル検索拡張で、pgvector の型と構文に互換です [VectorChord README (2026/11)]。
- **ライセンス**：AGPLv3 と Elastic License v2 の二択です [VectorChord LICENSE (2026/11)]。最新リリースは 1.1.1（2026-02-28）です [GitHub supervc-stack/VectorChord (2026/11)]。
- **Pelway への当てはめ（Pelway 側の評価）**：どちらのライセンスも商用配布の条件が重く、性能上の必要が実測で示されるまで採りません。

---

## 5. 添付ファイルの文書解析と日本語 OCR（ローカル実行）

### Pelway の課題

v0.7 本文に添付の文書解析・OCR の要件はありません。
導入する場合は、A7（国内ホストの local 推論）と 10 の国内完結方針、R05（作業一時ファイル 24 時間）、R03（派生物は source と同じ期限）を満たす必要があります（Pelway 側の評価）。

### 日本語精度の一次資料（ここが判断の土台です）

YomiToku・NDLOCR・PaddleOCR・Tesseract を同じ日本語データで比べた一次資料は見つかりませんでした。以下の 3 件は、それぞれ対象と条件が異なります。

1. **表構造認識（農業技術文書の PDF）**：小樽商科大学の発表は、表構造認識ベンチマーク TOITA で各ツールを比べ、6 指標すべてで YomiToku が最良でした。構造の F1（GriTS Top F1）は YomiToku 0.8883、Azure AI Document Intelligence 0.8044、Docling 0.7211、内容の F1（GriTS Con F1）は YomiToku 0.8373、Docling 0.6806 です [中村ほか (2025/03), 表2]。言語処理学会第 31 回年次大会の発表論文です。
2. **手書きノート（大学生 6 名のノートから選んだ 10 ページ）**：平均 CER（文字誤り率。0 に近いほど良い）は YomiToku 30.51%、Google Cloud Vision API 31.26%、GPT-4o 58.15%、PyTesseract 97.57% でした。YomiToku のページごとの CER は 1.38%〜55.92% とばらつきました [本田ほか (2026/03), 表1・表2]。この論文は査読なしと明記されています。NDLOCR と PaddleOCR は対象外です。
3. **NDLOCR-Lite（国立国会図書館の蔵書の活字）**：国立国会図書館は、図書・雑誌の日本語活字に対する F 値を年代別に公表しています。例えば 1960 年代の文系図書では NDLOCR ver.2.1 が 0.9908、NDLOCR-Lite ver.1.2.1 が 0.9849 です [NDLOCR-Lite evaluation_jptype.csv (2026/11)]。手書き（JaWildText の 1,065 画像）では全体平均 CER 0.268 です [NDLOCR-Lite README (2026/11)]。対象は歴史的資料が中心で、現代の業務文書の精度は未確認です。

（Pelway 側の評価）業務文書（請求書、契約書、スクリーンショット）での比較は存在しないため、採用前に自前の合成・匿名化データで CER を測る必要があります。

### YomiToku　判定：将来の候補（商用ライセンス購入が前提）

- **概要**：日本語に特化した文書画像解析エンジンです。文字位置の検出、文字列認識、レイアウト解析、表構造認識の 4 モデルを独自に学習し、7,000 字超の日本語文字、手書き、縦書きに対応します。出力は HTML・Markdown・JSON・CSV とサーチャブル PDF です。GPU では VRAM 8GB 以内で動き、軽量モデルなら CPU でも推論できます [YomiToku README (2026/11)]。
- **ライセンス**：ソースコードと Hugging Face 上のモデル重みは CC BY-NC-SA 4.0 です [YomiToku README (2026/11), LICENSE]。
- **商用判定のガイドライン**：ガイドラインは次の利用を商用と定めています [YomiToku 利用ガイドライン (2026/11), §2]。
  - 顧客向け製品・サービスの一部としての利用（§2-2）
  - YomiToku を組み込んだ製品・SaaS・API・自社プロダクトでの利用（§2-3）
  - 「社内利用だから非営利」ではなく、社内業務の効率化も商用（§2-5）
  - OCR テキストの顧客提供や、SaaS の内部機能としての出力利用（§2-6）
  - 判断基準として、金銭の授受、売上またはコスト削減、出力の外部提供のいずれかがあれば商用（§4）
- **無償で認められる範囲**：社内データでの精度検証、モデル比較、本番利用を目的としない技術選定、社内限定のデモは非営利扱いです [YomiToku 利用ガイドライン (2026/11), §1-1]。
- **禁止事項**：出力を他の OCR・文書解析モデルの学習データや蒸留に使うこと、競合する製品・モデルの開発を目的とした精度評価・ベンチマークに使うことは、非営利・商用を問わず禁止です [YomiToku 利用ガイドライン (2026/11), §3]。一方、本番に使わない社内でのモデル比較検証は §1-1 で非営利扱いとされています。
- **商用版**：オンプレミス向け商用ライセンスがあり、手書き認識の精度向上、画像の自動向き補正、レイアウト解析の強化など製品版のみの機能があります。窓口は https://www.mlism.com/ です [YomiToku README (2026/11), 商用利用について]。価格と条件は未確認です。
- **保守**：スター約 1,699、最新 v0.16.2（2026-11-10）です [GitHub kotaro-kinoshita/yomitoku (2026/11)]。
- **Pelway への当てはめ**：Pelway は利用企業に提供する業務チャット製品であり、添付の OCR 結果を利用者に返す使い方はガイドライン §2-3 と §2-6 に当たります。したがって、**Pelway に YomiToku を組み込むには製品版の商用ライセンス契約が必要です**。OSS 版で許されるのは、本番に使わない社内の精度検証（§1-1）までです。§3 により、YomiToku の出力を他エンジンの学習データにすることはできません。Pelway の採否を決めるための社内比較は §1-1 の範囲と読めますが、OCR 製品を開発しない立場であることを含め、判断に迷う場合はガイドライン §5 の窓口へ確認するのが安全です（Pelway 側の評価）。

### NDLOCR-Lite　判定：将来の候補（商用で使える代替の第一候補）

- **概要**：国立国会図書館が公開する OCR で、GPU なしで一般的なノート PC でも高速に動くことを特徴とします。対応 GPU と onnxruntime-gpu があれば GPU も使えます [NDLOCR-Lite README (2026/11)]。
- **ライセンス**：CC BY 4.0 です。同梱の学習済みレイアウトモデルは DINOv3 を蒸留した ViT-Tiny をバックボーンに使っており、DINOv3 ライセンスの遵守も求めています [NDLOCR-Lite README (2026/11)]。
- **DINOv3 ライセンス**：使用・複製・配布・改変を認める非独占・無償のライセンスで、貿易管理規制や ITAR の対象用途を禁じています [DINOv3 LICENSE (2026/11), §1]。条文中に商用利用を禁じる文言は grep では見つかりませんでしたが、全文の法的確認は未実施です。
- **保守**：スター約 1,378、最新 1.3.1（2026-09-07）です [GitHub ndl-lab/ndlocr-lite (2026/11)]。旧版の `ndlocr_cli` は ver.2.1（2023-08）が最新リリースです [GitHub ndl-lab/ndlocr_cli (2026/11)]。
- **Pelway への当てはめ（Pelway 側の評価）**：CPU で動き、国内の機関が公開しているため、10 の国内完結方針と相性が良い候補です。表構造の抽出機能の有無と、現代の業務文書での精度は未確認です。

### PaddleOCR　判定：将来の候補

- **概要**：OCR と文書構造化のツールキットです。PP-OCRv6 は中国語・英語・日本語と 46 のラテン文字言語を単一モデルで扱い、PP-StructureV3 は PDF や画像を Markdown・JSON に変換します [PaddleOCR README (2026/11)]。
- **精度の主張**：PP-OCRv6 は PP-OCRv5 より検出 +4.6%、認識 +5.1% と README は述べています [PaddleOCR README (2026/11)]。日本語単独の数値は README にありません。
- **ライセンスと保守**：Apache-2.0、スター約 92,574、最新 v3.7.0（2026-06-11）です [GitHub PaddlePaddle/PaddleOCR (2026/11)]。
- **Pelway への当てはめ（Pelway 側の評価）**：商用で使えるライセンスで、Xberg（後述）が ONNX Runtime 経由の OCR バックエンドとして採用しているため、Rust から呼べる経路があります。

### Docling　判定：将来の候補

- **概要**：文書を生成 AI 向けに構造化するツールで、機微データや外部から隔離された環境でのローカル実行をうたっています [Docling README (2026/11)]。
- **OCR エンジン**：RapidOCR、EasyOCR、Tesseract（Python バインディング版と CLI 版）などを選べます [Docling pipeline_options.py (2026/11)]。
- **ライセンスと保守**：コードは MIT で、個々のモデルは元パッケージのライセンスに従います [Docling README (2026/11), License]。スター約 70,438、最新 v2.150.0 です [GitHub docling-project/docling (2026/11)]。
- **日本語の証拠**：日本語の表構造認識では YomiToku や Azure より低い値でした [中村ほか (2025/03), 表2]。

### Xberg（旧 Kreuzberg）　判定：将来の候補

- **改名**：`kreuzberg-dev/kreuzberg` は `xberg-io/xberg` に転送され、README は Xberg を Kreuzberg の次の版と説明しています [Xberg README (2026/11)]。
- **概要**：107 形式からテキスト・表・メタデータを抽出するエンジンで、Rust の crate があります。OCR は Tesseract、PaddleOCR（ONNX Runtime）、Candle、VLM のバックエンドを切り替えられます [Xberg README (2026/11)]。
- **ライセンスと保守**：MIT、スター約 9,576、最新 v1.3.10 です [GitHub xberg-io/xberg (2026/11)]。
- **Pelway への当てはめ（Pelway 側の評価）**：Rust から直接呼べる唯一の候補で、A3 のスタックに最も近い位置にあります。日本語の精度は選んだ OCR バックエンド次第です。

### MarkItDown　判定：見送り

- **概要**：ファイルを LLM 向けの Markdown に変換する Python ツールです。画像キャプションと音声書き起こしは Web 上のサービスに依存すると README が明記しています。また、プロセスの権限でファイルやネットワークにアクセスするため、信頼できない入力は無害化するよう警告しています [MarkItDown README (2026/11)]。
- **Pelway への当てはめ（Pelway 側の評価）**：画像系の機能が外部サービスに依存し、08 の A8（本体の外向き通信を既定で禁止）に反します。

### Unstructured　判定：将来の候補（優先度低）

- Apache-2.0、スター約 15,716、最新 0.27.37 です [GitHub Unstructured-IO/unstructured (2026/11)]。日本語 OCR の品質を示す一次資料は見つからず、未確認です。

### Tesseract（jpn）　判定：将来の候補（優先度低）

- Apache-2.0、最新 5.5.4（2026-10-28）です [GitHub tesseract-ocr/tesseract (2026/11)]。高精度モデル集 `tessdata_best` の最終 push は 2024-03-09 です [GitHub tesseract-ocr/tessdata_best (2026/11)]。
- 手書きノートでは CER 97.57% でした [本田ほか (2026/03), 表1]。この結果は査読なしの論文の手書き 10 ページに限られ、活字の業務文書での比較は未確認です。Docling と Xberg の OCR バックエンドとして使えるため、比較対象としては残します（Pelway 側の評価）。

### 商用で使える構成の比較（Pelway 側の評価）

| 構成 | 商用利用 | 日本語の証拠 | Rust からの呼び出し |
|---|---|---|---|
| YomiToku 製品版 | 商用ライセンス契約が必要（価格未確認） | 表構造で最良、手書きで最良（上記 2 論文） | Python 経由のみ（未確認） |
| NDLOCR-Lite | CC BY 4.0（DINOv3 ライセンスも遵守） | 1870〜1960 年代の図書（CSV の先頭 19 行）で F 値 0.88〜0.98 | ONNX モデル（直接呼び出しは未確認） |
| Xberg＋PaddleOCR | MIT＋Apache-2.0 | 日本語単独の公表値なし | crate あり |
| Docling＋RapidOCR | MIT（モデルは個別） | 表構造で YomiToku より低い | Python 経由 |

---

## 6. 改ざん検出可能な監査ログ

### Pelway の課題

04 は `audit_events` に `prev_hash` を持たせ、workspace 内で鎖にします。
同じ節で「管理者の改ざん耐性を保証するものではない」とも述べています。
DB 管理者は鎖全体を書き換えて再計算できるためです（Pelway 側の評価）。

### Postgres 内のハッシュ鎖＋C2SP 署名付きチェックポイント　判定：MVP前に検証

- **C2SP 仕様**：tlog-checkpoint は、ログの識別子（origin）、木の大きさ（葉の数）、RFC 6962 形式の Merkle 木の根ハッシュの 3 行を本文とし、signed-note（署名付きテキスト）形式で署名したものです [C2SP tlog-checkpoint (2026/11)]。つまり checkpoint の中身は Merkle 木を前提とします。
- **Rust 実装**：Cloudflare の `tlog_tiles`（tlog-tiles と tlog-checkpoint の実装）と `signed_note`（signed-note の実装）の crate があります [crates.io tlog_tiles (2026/11)] [crates.io signed_note (2026/11)]。両 crate の最終更新は 2025-04-18 で、所属リポジトリ `cloudflare/azul` のライセンスは GitHub 上で判定できず未確認です [GitHub cloudflare/azul (2026/11)]。
- **Pelway の鎖との違い（Pelway 側の評価）**：04 の `prev_hash` は線形のハッシュ鎖で、Merkle 木ではありません。そのため、そのまま流用できるのは signed-note の署名形式だけです。tlog-checkpoint 互換にするには、監査行から Merkle 木を別に計算する必要があります。MVP 前の検証は「線形鎖の最新値と件数を signed-note で署名する」最小形から始めるのが妥当です。
- **Pelway への当てはめ（Pelway 側の評価）**：既存の `prev_hash` 鎖はそのまま使えます。追加するのは次の 2 点です。
  1. 一定間隔で workspace ごとの最新ハッシュと件数に署名し、チェックポイント（その時点の鎖の状態を示す署名付き記録）を作ること。
  2. チェックポイントを DB 管理者と別の権限で保管すること。保管先の候補は、12 §8.2 の「国内の限定権限による別保存」です。
  これで、後から鎖を丸ごと書き換えても、保管済みチェックポイントとの不一致として検出できます。
- **削除との衝突**：監査に本文を含めない設計（04 の `audit_events` 行）なら、本文の物理削除と鎖の検証は両立します（Pelway 側の評価）。R11 の 365 日経過後に古い監査を消すときは、消す範囲の最後のハッシュを新しい起点として署名して残す手順が必要です（Pelway 側の評価）。

### Tessera　判定：将来の候補（設計参照）

- **概要**：tlog-tiles 形式の透明性ログを作る Go ライブラリです。保存先ドライバは GCP・AWS・POSIX で、チェックポイントは背景処理で定期発行され、Witness（第三者による共同署名）にも対応します [Tessera README (2026/11)]。
- **ライセンスと保守**：Apache-2.0、最新 v1.0.4 です [GitHub transparency-dev/tessera (2026/11)]。
- **Pelway への当てはめ（Pelway 側の評価）**：Go のため直接は使いません。POSIX ドライバとチェックポイント発行の設計を参考にします。

### Trillian　判定：見送り

- README は Trillian が保守モードであり、新しいログ運用者にはまず Tessera を試すよう勧めると明記しています。保存層は MySQL/MariaDB です [Trillian README (2026/11)]。

### immudb　判定：見送り

- **概要**：暗号学的な証明と検証を内蔵した DB で、PostgreSQL のワイヤプロトコルに対応します [immudb README (2026/11)]。
- **ライセンス**：1.9DOM.2 以降は Business Source License で、Codenotary の有償版と競合する形でホスト・組込み提供しない限り本番利用を認めます [immudb LICENSE (2026/11)]。
- **Pelway への当てはめ（Pelway 側の評価）**：別の DB を増やすと、01 の「状態・監査・outbox を同一トランザクション」が成立しなくなります。

---

## 7. ローカルホストでの Postgres バックアップ・復旧

### Pelway の課題

13 の R15 は、国内 backup を 1 日 1 回取得し、作成から最大 14 日で復旧対象外とします。
12 §8.2 の手順 6 は、復旧時に、独立保全した削除・制限・撤回記録を先に再適用することを求めます。
04 §同時更新と成果物の耐久化は、DB と添付 volume の backup に同じ整合取得点が必要で、方式は未決としています。

### pgBackRest　判定：MVP前に検証

- **概要**：並列のバックアップと復元、ファイル単位・ブロック単位の差分・増分バックアップ、全ファイルのチェックサムと復元時の再検査、中断したバックアップの再開を持ちます [pgBackRest README (2026/11)]。
- **保存先と暗号化**：S3・Azure・GCS 互換のオブジェクトストアと SFTP にリポジトリを置けます。リポジトリの暗号化に対応します [pgBackRest README (2026/11)]。`repo-type` には POSIX 準拠ファイルシステム（`posix`）もあります [pgBackRest reference.xml (2026/11), repo-type]。
- **保持期間の挙動**：`repo-retention-full-type=time` にすると日数指定になります。ただし、指定日数以上経ったバックアップが他に 1 つ以上ある場合にだけ古いフルバックアップを消します。文書の例では、30 日指定で 25 日前と 35 日前のフルバックアップがあると、どちらも消しません [pgBackRest reference.xml (2026/11), repo-retention-full-type]。
- **保守状況**：2026-04-27 に作者が保守終了を告知しました。その後、スポンサー連合（AWS、Supabase、pgEdge、Tiger Data、Percona、Eon）の支援で 2026-05-18 に継続が確定しました [pgBackRest News (2026/11)]。
- **暗号化の不具合**：2.59.3 より前の版は、暗号化用の乱数生成の失敗を検出せず、弱いサブキーとソルトを作る可能性がありました。暗号化していないリポジトリは影響を受けません [pgBackRest News (2026/11), Weak Encryption]。
- **ライセンスと保守**：MIT、スター約 4,491、最新 2.60.0（2026-10-18）です [pgBackRest LICENSE (2026/11)] [GitHub pgbackrest/pgbackrest (2026/11)]。
- **Pelway への当てはめ（Pelway 側の評価）**：
  - 時間指定の保持は「14 日を超えないこと」ではなく「最低限の復旧範囲を残すこと」を保証する設計です。R15 の「作成から最大 14 日で復旧対象外」を守るには、件数指定（毎日フルバックアップなら 14 件）にするか、期限超過分を別の定期処理で expire する必要があります。
  - 2.59.3 以上を使い、暗号化を有効にします。
  - 添付 volume は pgBackRest の対象外なので、04 の整合取得点の問題は別途残ります。

### WAL-G　判定：将来の候補

- **概要**：`delete` コマンドに `retain`、`before`、`everything`、`target` の 4 モードがあり、`retain FULL 5` のように件数で保持できます。暗号化は libsodium または OpenPGP です [WAL-G docs README (2026/11)]。
- **ライセンスと保守**：Apache-2.0、スター約 4,332、最新 v3.0.9（2026-08-20）です [WAL-G LICENSE (2026/11)] [GitHub wal-g/wal-g (2026/11)]。
- **未確認**：ローカルファイルシステムを主保存先にした運用の文書（`WALG_FILE_PREFIX` はフェイルオーバー例にだけ登場しました）。

### Barman　判定：将来の候補

- **概要**：Python で書かれた Postgres のバックアップ・復旧管理ツールで、EnterpriseDB が保守します [Barman README (2026/11)]。
- **保持期間**：`retention_policy = RECOVERY WINDOW OF 4 WEEKS` のような復旧期間指定と、`REDUNDANCY 2` のような件数指定ができます [Barman barman.conf (2026/11)]。
- **ライセンスと保守**：GPL-3.0、最新 3.20.1 です [GitHub EnterpriseDB/barman (2026/11)]。
- **Pelway への当てはめ（Pelway 側の評価）**：別ホストからの取得を前提とする構成が中心と思われますが、未確認です。GPL-3.0 は Pelway に同梱する場合に影響するため、別プロセスの運用ツールとして使う前提になります。

---

## MVP 前の検証項目（Pelway 側の提案）

1. **River の Rust 版**：outbox と同一トランザクションでの投入、リース切れ時の挙動、workspace 行ロックとの lock 順序、Go 版スキーマのマイグレーション管理を、自前 `SKIP LOCKED` と比べて測る。
2. **pgBackRest**：件数指定で 14 日を超えない保持を作れるか、2.59.3 以上での暗号化、復旧後に 12 §8.2 の削除記録を再適用する手順を合成データで通す。
3. **監査チェックポイント**：`prev_hash` 鎖の最新値を signed-note 形式で署名したチェックポイントを作り、別権限の保管先と突き合わせて改ざんを検出できるかを試す。

## 未確認事項の一覧

- apalis が呼び出し側の sqlx トランザクション内で投入できるか。
- sqlx 組込みマイグレーションの詳細機能。なお、Pelway の現在の Cargo ワークスペース（`Cargo.toml` と `crates/*/Cargo.toml`）には sqlx 依存も `.sql` マイグレーションもまだありません。River の Rust 版が要求する SQLx 0.9 との衝突は現時点では生じません。
- PowerSync の Rust SDK、Replicache の保守状況とライセンス。
- Yrs の GC 後に残る情報、Loro の shallow snapshot 後に古い更新が再流入しうるか、Automerge の履歴切り詰め手段。
- pgvectorscale のラベル数の上限。
- YomiToku 製品版の価格・条件、NDLOCR-Lite の表抽出機能、現代の業務文書での各 OCR の精度、DINOv3 ライセンスの法的評価。
- `cloudflare/azul` のライセンス。
- WAL-G のローカルファイルシステム運用、Barman の構成前提。

## 出典

[River Rust README, 2026/11] riverqueue. "River for Rust." GitHub. https://github.com/riverqueue/river/blob/master/rust/README.md
[GitHub riverqueue/river, 2026/11] riverqueue. "river." GitHub. https://github.com/riverqueue/river
[crates.io riverqueue, 2026/11] crates.io. "riverqueue." https://crates.io/crates/riverqueue
[crates.io sqlx, 2026/11] crates.io. "sqlx." https://crates.io/crates/sqlx
[crates.io sqlx-cli, 2026/11] crates.io. "sqlx-cli." https://crates.io/crates/sqlx-cli
[crates.io refinery, 2026/11] crates.io. "refinery." https://crates.io/crates/refinery
[GitHub transact-rs/sqlx, 2026/11] transact-rs. "sqlx." GitHub. https://github.com/transact-rs/sqlx
[apalis README, 2026/11] apalis-dev. "apalis." GitHub. https://github.com/apalis-dev/apalis
[GitHub apalis-dev/apalis, 2026/11] apalis-dev. "apalis." GitHub. https://github.com/apalis-dev/apalis
[crates.io apalis-postgres, 2026/11] crates.io. "apalis-postgres." https://crates.io/crates/apalis-postgres
[pgmq README, 2026/11] pgmq. "pgmq." GitHub. https://github.com/pgmq/pgmq
[GitHub pgmq/pgmq, 2026/11] pgmq. "pgmq." GitHub. https://github.com/pgmq/pgmq
[crates.io pgmq, 2026/11] crates.io. "pgmq." https://crates.io/crates/pgmq
[Restate Server overview, 2026/11] Restate. "Server overview." Restate Docs. https://docs.restate.dev/server/overview
[restatedev/restate LICENSE, 2026/11] Restate. "LICENSE (Business Source License 1.1)." GitHub. https://github.com/restatedev/restate/blob/main/LICENSE
[Restate README, 2026/11] Restate. "restate." GitHub. https://github.com/restatedev/restate
[Hatchet README, 2026/11] Hatchet. "hatchet." GitHub. https://github.com/hatchet-dev/hatchet
[Hatchet self-hosting docs, 2026/11] Hatchet. "Self-hosting." Hatchet Docs. https://docs.hatchet.run/self-hosting
[GitHub hatchet-dev/hatchet, 2026/11] Hatchet. "hatchet." GitHub. https://github.com/hatchet-dev/hatchet
[Electric README, 2026/11] ElectricSQL. "Electric." GitHub. https://github.com/electric-sql/electric
[GitHub electric-sql/electric, 2026/11] ElectricSQL. "electric." GitHub. https://github.com/electric-sql/electric
[Electric Auth guide, 2026/11] ElectricSQL. "Auth." Electric Docs. https://electric.ax/docs/guides/auth
[Electric Deployment guide, 2026/11] ElectricSQL. "Deployment." Electric Docs. https://electric.ax/docs/guides/deployment
[PowerSync Self-hosting, 2026/11] PowerSync. "Self-hosting: Getting started." PowerSync Docs. https://docs.powersync.com/self-hosting/getting-started
[PowerSync Source DB setup, 2026/11] PowerSync. "Source database setup." PowerSync Docs. https://docs.powersync.com/configuration/source-db/setup
[GitHub powersync-ja/powersync-service, 2026/11] PowerSync. "powersync-service." GitHub. https://github.com/powersync-ja/powersync-service
[Zero Connecting to Postgres, 2026/11] Rocicorp. "Connecting to Postgres." Zero Docs. https://zero.rocicorp.dev/docs/connecting-to-postgres
[Zero Permissions, 2026/11] Rocicorp. "Permissions." Zero Docs. https://zero.rocicorp.dev/docs/permissions
[GitHub rocicorp/mono, 2026/11] Rocicorp. "mono." GitHub. https://github.com/rocicorp/mono
[GitHub rocicorp/replicache, 2026/11] Rocicorp. "replicache." GitHub. https://github.com/rocicorp/replicache
[Replicache docs, 2026/11] Rocicorp. "Replicache." https://doc.replicache.dev/
[Loro README, 2026/11] Loro. "loro." GitHub. https://github.com/loro-dev/loro
[GitHub loro-dev/loro, 2026/11] Loro. "loro." GitHub. https://github.com/loro-dev/loro
[Loro docs encoding, 2026/11] Loro. "Encoding (Shallow Snapshot Encoding)." GitHub. https://github.com/loro-dev/loro-docs/blob/main/pages/docs/tutorial/encoding.mdx
[crates.io loro, 2026/11] crates.io. "loro." https://crates.io/crates/loro
[y-crdt README, 2026/11] y-crdt. "y-crdt." GitHub. https://github.com/y-crdt/y-crdt
[y-crdt LICENSE, 2026/11] y-crdt. "LICENSE (MIT)." GitHub. https://github.com/y-crdt/y-crdt/blob/main/LICENSE
[crates.io yrs, 2026/11] crates.io. "yrs." https://crates.io/crates/yrs
[Yjs Y.Doc docs, 2026/11] Yjs. "Y.Doc." Yjs Docs. https://docs.yjs.dev/api/y.doc
[Automerge README, 2026/11] Automerge. "automerge." GitHub. https://github.com/automerge/automerge
[GitHub automerge/automerge, 2026/11] Automerge. "automerge." GitHub. https://github.com/automerge/automerge
[crates.io automerge, 2026/11] crates.io. "automerge." https://crates.io/crates/automerge
[pgvector README, 2026/11] pgvector. "pgvector." GitHub. https://github.com/pgvector/pgvector
[pgvector LICENSE, 2026/11] pgvector. "LICENSE." GitHub. https://github.com/pgvector/pgvector/blob/master/LICENSE
[GitHub pgvector/pgvector, 2026/11] pgvector. "pgvector." GitHub. https://github.com/pgvector/pgvector
[pgvectorscale README, 2026/11] Timescale. "pgvectorscale." GitHub. https://github.com/timescale/pgvectorscale
[GitHub timescale/pgvectorscale, 2026/11] Timescale. "pgvectorscale." GitHub. https://github.com/timescale/pgvectorscale
[VectorChord README, 2026/11] supervc-stack. "VectorChord." GitHub. https://github.com/supervc-stack/VectorChord
[VectorChord LICENSE, 2026/11] supervc-stack. "LICENSE (Dual License Notice)." GitHub. https://github.com/supervc-stack/VectorChord/blob/main/LICENSE
[GitHub supervc-stack/VectorChord, 2026/11] supervc-stack. "VectorChord." GitHub. https://github.com/supervc-stack/VectorChord
[中村ほか, 2025/03] 中村彩乃, 杉山陽菜乃, 阿部瑞稀, 前多陸玖, 坂口遥哉, 佐藤栄作, 木村泰知. "農林業基準技術文書を対象としたPDF解析ツールの表構造認識の性能評価." 言語処理学会第31回年次大会発表論文集 Q7-22. https://www.anlp.jp/proceedings/annual_meeting/2025/pdf_dir/Q7-22.pdf
[本田ほか, 2026/03] 本田彬, 木村泰知, 片岡駿, 三浦克宜, 沼澤政信. "大学生のノートテイキングにおけるノート構造の比較分析." 言語処理学会第32回年次大会発表論文集 P7-3（査読なし）. https://www.anlp.jp/proceedings/annual_meeting/2026/pdf_dir/P7-3.pdf
[NDLOCR-Lite README, 2026/11] 国立国会図書館. "NDLOCR-Lite." GitHub. https://github.com/ndl-lab/ndlocr-lite
[NDLOCR-Lite evaluation_jptype.csv, 2026/11] 国立国会図書館. "F値による日本語活字の評価結果." GitHub. https://github.com/ndl-lab/ndlocr-lite/blob/master/evaluation_jptype.csv
[GitHub ndl-lab/ndlocr-lite, 2026/11] 国立国会図書館. "ndlocr-lite." GitHub. https://github.com/ndl-lab/ndlocr-lite
[GitHub ndl-lab/ndlocr_cli, 2026/11] 国立国会図書館. "ndlocr_cli." GitHub. https://github.com/ndl-lab/ndlocr_cli
[DINOv3 LICENSE, 2026/11] Meta. "DINOv3 License." GitHub. https://github.com/facebookresearch/dinov3/blob/main/LICENSE.md
[YomiToku README, 2026/11] Kotaro Kinoshita. "YomiToku." GitHub. https://github.com/kotaro-kinoshita/yomitoku
[YomiToku 利用ガイドライン, 2026/11] Kotaro Kinoshita. "YomiToku 利用ガイドライン（CC BY-NC-SA 追加説明）." GitHub. https://github.com/kotaro-kinoshita/yomitoku/blob/main/docs/commercial_use_guideline.ja.md
[GitHub kotaro-kinoshita/yomitoku, 2026/11] Kotaro Kinoshita. "yomitoku." GitHub. https://github.com/kotaro-kinoshita/yomitoku
[PaddleOCR README, 2026/11] PaddlePaddle. "PaddleOCR." GitHub. https://github.com/PaddlePaddle/PaddleOCR
[GitHub PaddlePaddle/PaddleOCR, 2026/11] PaddlePaddle. "PaddleOCR." GitHub. https://github.com/PaddlePaddle/PaddleOCR
[Docling README, 2026/11] docling-project. "Docling." GitHub. https://github.com/docling-project/docling
[Docling pipeline_options.py, 2026/11] docling-project. "pipeline_options.py." GitHub. https://github.com/docling-project/docling/blob/main/docling/datamodel/pipeline_options.py
[GitHub docling-project/docling, 2026/11] docling-project. "docling." GitHub. https://github.com/docling-project/docling
[Xberg README, 2026/11] xberg-io. "Xberg." GitHub. https://github.com/xberg-io/xberg
[GitHub xberg-io/xberg, 2026/11] xberg-io. "xberg." GitHub. https://github.com/xberg-io/xberg
[MarkItDown README, 2026/11] Microsoft. "MarkItDown." GitHub. https://github.com/microsoft/markitdown
[GitHub Unstructured-IO/unstructured, 2026/11] Unstructured-IO. "unstructured." GitHub. https://github.com/Unstructured-IO/unstructured
[GitHub tesseract-ocr/tesseract, 2026/11] tesseract-ocr. "tesseract." GitHub. https://github.com/tesseract-ocr/tesseract
[GitHub tesseract-ocr/tessdata_best, 2026/11] tesseract-ocr. "tessdata_best." GitHub. https://github.com/tesseract-ocr/tessdata_best
[crates.io tlog_tiles, 2026/11] crates.io. "tlog_tiles." https://crates.io/crates/tlog_tiles
[crates.io signed_note, 2026/11] crates.io. "signed_note." https://crates.io/crates/signed_note
[GitHub cloudflare/azul, 2026/11] Cloudflare. "azul." GitHub. https://github.com/cloudflare/azul
[Tessera README, 2026/11] transparency-dev. "Tessera." GitHub. https://github.com/transparency-dev/tessera
[GitHub transparency-dev/tessera, 2026/11] transparency-dev. "tessera." GitHub. https://github.com/transparency-dev/tessera
[Trillian README, 2026/11] Google. "Trillian." GitHub. https://github.com/google/trillian
[immudb README, 2026/11] Codenotary. "immudb." GitHub. https://github.com/codenotary/immudb
[immudb LICENSE, 2026/11] Codenotary. "LICENSE (Business Source License)." GitHub. https://github.com/codenotary/immudb/blob/master/LICENSE
[pgBackRest README, 2026/11] pgBackRest. "pgBackRest." GitHub. https://github.com/pgbackrest/pgbackrest
[pgBackRest LICENSE, 2026/11] pgBackRest. "LICENSE (MIT)." GitHub. https://github.com/pgbackrest/pgbackrest/blob/main/LICENSE
[GitHub pgbackrest/pgbackrest, 2026/11] pgBackRest. "pgbackrest." GitHub. https://github.com/pgbackrest/pgbackrest
[pgBackRest reference.xml, 2026/11] pgBackRest. "Configuration reference: repo-retention-full-type." GitHub. https://github.com/pgbackrest/pgbackrest/blob/main/doc/xml/reference.xml
[pgBackRest News, 2026/11] pgBackRest. "News." https://pgbackrest.org/news.html
[WAL-G docs README, 2026/11] WAL-G. "docs/README.md." GitHub. https://github.com/wal-g/wal-g/blob/master/docs/README.md
[WAL-G LICENSE, 2026/11] WAL-G. "LICENSE.md (Apache-2.0)." GitHub. https://github.com/wal-g/wal-g/blob/master/LICENSE.md
[GitHub wal-g/wal-g, 2026/11] WAL-G. "wal-g." GitHub. https://github.com/wal-g/wal-g
[PostgreSQL SELECT docs, 2026/11] PostgreSQL Global Development Group. "SELECT: The Locking Clause." PostgreSQL Documentation. https://www.postgresql.org/docs/current/sql-select.html
[C2SP tlog-checkpoint, 2026/11] C2SP. "tlog-checkpoint." https://c2sp.org/tlog-checkpoint
[Barman README, 2026/11] EnterpriseDB. "Barman." GitHub. https://github.com/EnterpriseDB/barman
[Barman barman.conf, 2026/11] EnterpriseDB. "docs/barman.conf." GitHub. https://github.com/EnterpriseDB/barman/blob/master/docs/barman.conf
[GitHub EnterpriseDB/barman, 2026/11] EnterpriseDB. "barman." GitHub. https://github.com/EnterpriseDB/barman
