# Pelway 日本語全文検索の選択肢調査

調査日: 2026-11-11。リポジトリのリリース日・ライセンスは GitHub API（`gh api repos/<owner>/<repo>` と `.../releases/latest`）で取得した値。ベンチマーク数値は出典が測定したものだけを載せ、自前の測定はしていない。

## 1. 前提（Pelway の制約）

- サーバーは Rust（tokio、axum、sqlx）。PostgreSQL を正本とする。full text は「Postgresの権限付き検索を入口とし、日本語tokenizer/拡張は未選定」で、候補は pg_bigm と Tantivy/Lindera（`docs/technical/v0.7/body/02-stack-comparison.md` §サーバー・AI層の比較）。
- 国内のローカルホストで rootless Podman により運用する。外部 SaaS は使わず、既定では外向き通信もしない。選択済みの外部連携へ検索語を送る場合は、送信先として表示し、初期 off にする（`10-domestic-only.md` 29行目・41行目）。
- 検索・embedding・RAG は「indexの作成前と検索結果の返却前に検査。本文、snippet、タイトル、件数、候補語から無権限sourceの存在を漏らさない」（`12-communication-secrecy.md` 97行目）。依頼者と Bot の双方の現 scope で絞る（同135行目）。
- 期限切れ・削除・根拠失効のものは、読取・検索・AI入力・公開の時点で拒否する。索引経由の参照にも同じ制限を適用する（`13-operational-defaults.md` §2）。索引・embedding には独立した TTL を持たせず、source と同じ期限にする（R03）。利用停止は削除受付を記録するトランザクションで確定し、検索の入口で現在の制限を確認する（D01）。
- 内容は日本語の業務チャット。短文で、日英・コードが混在し、カタカナ語、製品名、誤字を含む。将来は添付から抽出したテキストも対象になり、AI 検索（メモリ）の入力にもなる。
- 日本語検索が明確に良くなるなら、開発環境が重くなることは許容する。

## 2. 候補の比較表

| 候補 | 分かち方 | 精度/再現率 | 権限・即時削除との整合 | 運用負担 | ライセンス | 保守状況 | Rust連携 | 出典 |
|---|---|---|---|---|---|---|---|---|
| pg_bigm | 2-gram。前後に空白を付けて2文字ずつ分割する。GIN のみ | 1〜2文字の語でも索引が効く。辞書がないので新語や製品名も拾える（再現率重視）。索引の候補は誤りを含むため recheck が必要。順位付けや強調表示の関数は文書に記載がない（`bigm_similarity` と `likequery` はある） | Postgres の通常の索引。同じ SQL で ACL・期限・削除の条件と結合できる。Postgres の索引アクセスメソッドは可視性判定に関与せず、MVCC 可視性はコア側がテーブル本体（heap）の行で判定する | 公式 postgres イメージには含まれないので、自前でビルドしたイメージが必要（PGDG パッケージの有無は未確認）。別プロセスは不要 | PostgreSQL License | 最新リリース v1.2-20250903（2025-09-02）。リポジトリは 2026-11 も更新あり。PostgreSQL 9.1〜19 で試験済みと記載 | sqlx で `LIKE likequery($1)` を発行するだけ | https://pgbigm.github.io/pg_bigm/pg_bigm.html 、https://github.com/pgbigm/pg_bigm 、https://www.postgresql.org/docs/current/index-scanning.html |
| PGroonga（Groonga） | 既定は TokenBigram。normalizer を指定すると ASCII は空白と文字種の境界で語単位に、非 ASCII は bigram に分ける。TokenMecab（形態素）と TokenNgram（n 指定）も選べる。NormalizerNFKC は unify_kana、unify_hyphen、unify_prolonged_sound_mark、unify_katakana_v_sounds、unify_to_romaji などで表記ゆれを吸収できる | bigram は再現率寄り。TokenMecab は公式に「good for precision rather than recall」とされ、新語を拾うには辞書の更新が要る。スコア（`pgroonga_score`）、強調表示（`pgroonga_highlight_html`）、snippet（`pgroonga_snippet_html`）を持つ | Postgres の索引アクセスメソッドとして動くので、同じ SQL で ACL・期限・削除を結合できる。可視性がテーブル本体で判定されるのは索引アクセスメソッド一般の性質からの推論で、PGroonga 固有の文書では未確認。`pgroonga.enable_wal` の既定は off で「PGroonga doesn't generate WAL」。PostgreSQL 15 以上では `pgroonga_wal_resource_manager`（3.2.1〜）で WAL を使うストリーミングレプリケーションができるが、primary 側の crash safe は別途 `pgroonga_crash_safer` が必要で、crash safe 機能は「still an experimental feature」 | 公式 Docker イメージ（pgroonga/docker、alpine/debian）がある。crash safe には `shared_preload_libraries = 'pgroonga_crash_safer'` と、ワーカープロセス数の調整が必要。レプリカを使う場合は `shared_preload_libraries = 'pgroonga_wal_resource_manager'` と `pgroonga.enable_wal_resource_manager = yes` を設定する。TokenMecab を使うなら MeCab と辞書を別に入れる | PGroonga は PostgreSQL License 系（COPYING）。Groonga は LGPL-2.1 | PGroonga 4.1.0（2026-10-09）、Groonga v16.1.5（2026-10-29）。活発 | SQL 演算子（`&@~` など）を sqlx から使う | https://pgroonga.github.io/reference/create-index-using-pgroonga.html 、https://pgroonga.github.io/reference/crash-safe.html 、https://pgroonga.github.io/reference/ 、https://pgroonga.github.io/reference/replication.html 、https://pgroonga.github.io/reference/modules/pgroonga-wal-resource-manager.html 、https://pgroonga.github.io/reference/parameters/enable-wal.html 、https://groonga.org/docs/reference/tokenizers/token_bigram.html 、https://groonga.org/docs/reference/tokenizers/token_mecab.html 、https://groonga.org/docs/reference/normalizers/normalizer_nfkc.html 、https://github.com/pgroonga/docker |
| pg_trgm | 3-gram。非英数字は語の区切りとして無視し、各語の前に空白2つ・後ろに1つを付ける | pg_bigm の比較表では「日本語検索：標準では未対応（ソース改変が必要で、遅い）」「1〜2文字の語：索引が使えず遅い」。trigram を取り出せないパターンは索引全体の走査に退化する | Postgres の通常の索引 | contrib に含まれ、追加の導入は不要 | PostgreSQL | Postgres 本体と同じ | sqlx | https://www.postgresql.org/docs/current/pgtrgm.html 、https://pgbigm.github.io/pg_bigm/pg_bigm.html |
| tsvector と日本語パーサ（textsearch_ja、pg_cjk_parser、zhparser など） | 標準パーサは日本語を分かち書きしない（pgsql-hackers の議論と Pleroma 文書による。どちらも検索結果の要約で見ただけで本文は未確認）。textsearch_ja は MeCab による形態素解析 | 形態素解析なので精度寄り（日本語での品質は未確認） | Postgres の通常の索引 | 拡張と MeCab・辞書のビルドが必要 | textsearch_ja のライセンスは未確認。zhparser は中国語用（SCWS） | 元の kenjiroy/textsearch_ja は 2014-12 が最終更新。フォークは Windows 向け（HiraokaHyperTools、2026-02 更新）と個人の alpine イメージ（tenboshi、2026-03 更新）程度。保守された日本語パーサは見当たらない | sqlx | https://github.com/kenjiroy/textsearch_ja 、https://github.com/HiraokaHyperTools/textsearch_ja 、https://github.com/tenboshi/postgres-ja-18-alpine 、https://www.postgresql.org/message-id/20190502.100711.2043302145145397919.t-ishii%40sraoss.co.jp 、https://docs.pleroma.social/backend/configuration/howto_search_cjk |
| Tantivy + Lindera（lindera-tantivy、プロセス内） | 形態素解析。IPADIC、IPADIC-NEologd、UniDic、SudachiDict などの辞書を feature（`embed-ipadic` など）でバイナリに埋め込める。n-gram tokenizer と別フィールドで併用できる（Tantivy 側の機能。詳細未確認） | 辞書次第で精度寄り。BM25 による順位付けと snippet は Tantivy の機能（本調査では未精査）。Lindera の IPADIC 処理速度は README の記載で単一スレッド約10〜20MB/s | 削除は `delete_term` の後に `commit()` して初めて見える。既定の reload policy では commit が返った直後でも検索が古い結果を返しうる。正本の Postgres と別の索引になるので、索引を候補集合として使い、返す前に Postgres で ACL・期限・削除を再検査する設計が必須。件数や候補語から存在が漏れないよう、事後フィルタとページングの扱いも必要 | 追加のコンテナは不要。索引ディレクトリのバックアップ・再構築と、Postgres との同期（outbox など）を自前で実装する。辞書の埋め込みでバイナリが大きくなる | Tantivy は MIT、Lindera は MIT。辞書のライセンスは辞書ごとに異なり、未確認 | Tantivy 0.26.1（2026-05-10）、Lindera v6.2.0（2026-09-26）、lindera-tantivy v6.2.0（2026-11-05）。活発 | ネイティブ（同じ Rust プロセス内） | https://github.com/lindera/lindera 、https://github.com/lindera/lindera-tantivy 、https://docs.rs/tantivy/latest/tantivy/indexer/struct.IndexWriter.html |
| Meilisearch（charabia。日本語は Lindera の IPA 辞書） | 形態素解析（Lindera、IPA 辞書） | タイポ許容と順位付けが標準で付く。半角カタカナがハングルと判定される open issue（charabia#375）がある | 文書の追加・削除は非同期タスクのキューで処理される。succeeded になるまで反映されないので、即時拒否は Postgres での再検査に頼ることになる | 別プロセス（別コンテナ）が必要 | MIT と BUSL-1.1 の混在（Enterprise Edition 部分が BUSL） | v1.56.2（2026-11-09）。活発 | HTTP クライアント | https://www.meilisearch.com/docs/learn/resources/language 、https://www.meilisearch.com/docs/learn/async/asynchronous_operations 、https://github.com/meilisearch/meilisearch/blob/main/LICENSE 、https://github.com/meilisearch/charabia/issues/375 |
| ParadeDB pg_search（Postgres 内の Tantivy） | `pdb.lindera(japanese)`。日本語は IPADIC 辞書 | BM25（ParadeDB の機能。日本語での品質は未確認） | Postgres のトランザクションと MVCC に従う。0.24.0 以降は WAL に記録され、クラッシュ後も残る。Community 版は索引を物理レプリケーションしない | 拡張入りのイメージが必要。HA とリードレプリカは Enterprise 版のみ | Community 版は AGPL-3.0、Enterprise 版は商用 | v0.26.6（2026-11-08）。活発 | SQL（sqlx） | https://www.paradedb.com/docs/documentation/tokenizers/available-tokenizers/lindera 、https://www.paradedb.com/docs/welcome/guarantees 、https://www.paradedb.com/docs/operate/deploy/enterprise 、https://github.com/paradedb/paradedb |
| （参考）RAGFlow 1.0 と Infinity | Infinity の全文検索 analyzer には MeCab をラップした `japanese` と、CJK に1〜2文字の gram を出す `sparsegram` がある。RAGFlow 自身の取り込み tokenizer（`rag_tokenizer`）が日本語をどう分割するかと、日本語 dataset が Infinity の `japanese` analyzer を使うかは未確認 | 未確認 | 共有の範囲は dataset・Memory などの資源ごとに「Only me」（所有者のみ）か「Team」（所有者のチームに参加したメンバー）の2つだけ。チームを抜けるか所有者が非公開にするとアクセスを失う。文書単位や channel 単位の ACL は文書に記載がない。検索インデックスへの削除の伝播は未確認 | 既定の文書エンジンは Elasticsearch（`DOC_ENGINE=elasticsearch`）。compose には MySQL、MinIO、Kvrocks、NATS があり、ClickHouse は profile 指定時に起動する。推奨は 4 CPU、16 GB RAM、50 GB ディスク、Docker ≥ 24 と Compose ≥ v2.26.1。rootless Podman での動作は未確認。embedding モデルが必要で、ローカルモデル（TEI）の profile がある | 両方とも Apache-2.0 | RAGFlow v1.0.0（2026-11-06、Go）、Infinity v0.7.5（2026-11-05、C++） | HTTP API | https://github.com/infiniflow/ragflow 、https://github.com/infiniflow/ragflow/blob/main/docker/.env 、https://github.com/infiniflow/ragflow/blob/main/docker/docker-compose-base.yml 、https://github.com/infiniflow/ragflow/blob/main/docs/guides/dataset/configuration.md 、https://github.com/infiniflow/ragflow/blob/main/docs/guides/team/index.md 、https://github.com/infiniflow/ragflow/blob/main/docs/guides/team/resource_sharing/resource_sharing_and_permissions.md 、https://github.com/infiniflow/infinity/blob/main/docs/guides/search_guide.md |

RAGFlow について: チャット本文の検索ではなく、添付・ナレッジを対象にした AI 検索の候補として扱う。サービスが多く、権限の単位が粗い。そのため 12・13 の channel 単位 ACL と即時削除を満たすには、Pelway 側で dataset を channel ごとに分けたうえで再検査する必要がある（設計上の推測）。現時点の Pelway の入口として採る理由は見当たらない。

## 3. 既存 OSS の事例

| プロジェクト | 選択 | 理由・状況 | 出典 |
|---|---|---|---|
| Zulip | 既定は PostgreSQL 組込み FTS（英語のストップワード）。PGroonga を実験的なオプションとして用意 | PGroonga は「supports all languages simultaneously, including Japanese and Chinese」。十分に試験できたら既定にする予定と記載。移行中は新規メッセージを送れない。索引は配信後に `process_fts_updates` がバックグラウンドで更新する | https://zulip.readthedocs.io/en/latest/subsystems/full-text-search.html |
| Misskey | `fulltextSearch.provider` で `sqlLike`（既定、拡張不要）、`sqlPgroonga`、`meilisearch` から選ぶ | 既定は LIKE。PGroonga は note の text 列に索引を作る。Meilisearch は別サービス | https://github.com/misskey-dev/misskey/blob/develop/.config/example.yml |
| Mastodon | Elasticsearch（任意）。content analyzer は `standard` tokenizer と `cjk_width`、英語のステミングの組み合わせで、日本語専用の analyzer はない。`searchable_by`（閲覧可能なアカウント ID）を索引に持たせて権限で絞る | 日本語の検索品質は未確認。権限情報を索引に埋め込む方式の例として参考になる。`refresh_interval: '30s'` なので反映に遅延がある | https://github.com/mastodon/mastodon/blob/main/app/chewy/statuses_index.rb |
| Mattermost | DB 検索、または Elasticsearch/OpenSearch | 日本語検索には「データベースの設定変更が必要」。中国語は PostgreSQL の zhparser（SCWS）を使う。Elasticsearch/OpenSearch の CJK 対応に EnableCJKAnalyzers 設定（v11.6〜）と analyzer プラグインが要るという記述は、サポート記事を検索結果の要約で見ただけなので未確認。投稿が 250 万件を超えたら Elasticsearch/OpenSearch を推奨 | https://docs.mattermost.com/administration-guide/configure/enabling-chinese-japanese-korean-search.html 、https://support.mattermost.com/hc/en-us/articles/51444970047636-Enabling-Korean-CJK-Search-with-OpenSearch-Elasticsearch-in-Multi-Pod-Deployments |
| Matrix Synapse | PostgreSQL FTS | 「バッタと鈴虫」の文で、先頭の「バッタ」は検索に当たるが途中の「鈴虫」は当たらないという issue が open のまま残っている。コメントで PGroonga が提案されている | https://github.com/matrix-org/synapse/issues/901 |

共通点: Postgres を正本とするチャット OSS のうち Zulip と Misskey は、既定（Zulip は標準 FTS、Misskey は LIKE）とは別に、日本語対応の選択肢として PGroonga を用意している。どちらも既定にはしていない（Zulip は「十分に試験できたら既定にする予定」）。標準の FTS だけの Synapse では、日本語の部分一致が当たらない issue が open のまま残っている。

## 4. ハイブリッドの型

- **n-gram と形態素の併用**: n-gram で検索漏れを減らし、形態素解析でノイズを減らす。Human Made は Elasticsearch で n-gram と Kuromoji を組み合わせ、ICU プラグインで正規化した。測定値は示されていない。https://humanmade.com/engineering/enhancing-japanese-search-relevancy-with-elasticsearch/
- **英字・コードの扱い**: Groonga の TokenBigram に normalizer を付けると、ASCII は空白と文字種の境界で語単位に分かれる（例：「100cents!!!」は `100`、`cents`、`!!!`）。日本語部分だけが bigram になる。英字も bigram にしたい場合は TokenBigramSplit 系を使う。https://groonga.org/docs/reference/tokenizers/token_bigram.html
- **PGroonga での併用**: 同じ列に TokenBigram の索引と TokenMecab の索引を別々に作り、片方で再現率、片方でスコアを取る構成は理論上可能。ただし実例と性能は未確認。
- **索引を候補集合として使う**: 外部や別プロセスの索引（Tantivy、Meilisearch、Infinity）は候補 ID を返すだけにする。返す直前に Postgres で ACL・期限・削除・利用制限を再検査する。13 の「stale な索引を返さない」を満たすための必須条件になる。

## 5. 選択肢

### 選択肢 A：PGroonga（TokenBigram と NormalizerNFKC を正本の Postgres 内に置く）【推奨】

- **採るもの**: Postgres 内の index AM、TokenBigram（日本語は bigram、英字とコードは語単位）、NormalizerNFKC の unify 系オプション（かなの統一、長音・ハイフンの統一）、`pgroonga_score`、`pgroonga_highlight_html` と `pgroonga_snippet_html`。
- **捨てるもの**: TokenMecab（初期は使わない。精度寄りで、辞書の更新が必要なため）。外部の検索プロセス。
- **理由**: ACL・期限・削除・利用制限を同じ SQL とトランザクションの中で判定できる。可視性もテーブル本体で判定されるので、D01 の「受付トランザクションで確定」と R03 の「索引に独立 TTL なし」にそのまま合う。順位付けと強調表示があり、pg_bigm より機能が多い。Zulip と Misskey が日本語対応の選択肢として PGroonga を用意している（どちらも既定ではない）。
- **注意**: `pgroonga.enable_wal` の既定は off で、PGroonga は既定では WAL を生成しない。crash safe（`pgroonga_crash_safer`）は experimental で、回復に失敗すると Groonga のデータベースを作り直して REINDEX する。R15 のバックアップ・復元の演習で、索引を復元するのか REINDEX で作り直すのかを検証する必要がある。Groonga 本体は LGPL-2.1。
- **注意（英字・コード）**: normalizer 付きの TokenBigram は ASCII を語単位で扱う。そのため識別子の途中の部分一致は当たらない（公式の例では「pp」で「Apple」は当たらない）。コードの部分一致が必要なら、TokenBigramSplit 系か TokenNgram の unify 系オプションを使う（公式は TokenNgram の unify 系を推奨しないと記載）。TokenBigramSplit 系で実在する名前は `TokenBigramSplitSymbol`、`TokenBigramSplitSymbolAlpha`、`TokenBigramSplitSymbolAlphaDigit`（Groonga 16.1.3 の `tokenizer_list` で確認。#93 の計測）。「pp」で「Apple」を当てるには `TokenBigramSplitSymbolAlpha` を使う。
- **誤字**: bigram 方式は誤字への耐性を意味しない。`pgroonga_condition` に `fuzzy_max_distance_ratio` という引数がある（3.1.6〜）が、挙動は未確認。
- **開発環境への影響**: Postgres のコンテナを PGroonga の公式イメージか派生イメージに替える（rootless Podman での動作は未確認）。追加のプロセスはない。

### 選択肢 B：pg_bigm（最小構成）

- **採るもの**: 2-gram の GIN 索引と `LIKE` の部分一致。
- **捨てるもの**: 順位付けと強調表示（アプリ側で単純に実装する）。表記ゆれの正規化（アプリ側で NFKC などを前処理する）。
- **理由**: 依存が最も小さく、ライセンスも単純で、Postgres の WAL と MVCC にそのまま乗る。権限・削除との整合は A と同じ。
- **開発環境への影響**: pg_bigm をビルドした Postgres イメージが必要（PGDG パッケージの有無は未確認）。追加のプロセスはない。

### 選択肢 C：Tantivy と Lindera をプロセス内に置き、Postgres で再検査する

- **採るもの**: Lindera（IPADIC か UniDic を埋め込み）と n-gram の2フィールド索引、BM25 による順位付け。索引は候補集合として扱い、返す直前に Postgres で再検査する。
- **捨てるもの**: 「索引が正しい」という前提。
- **理由**: Rust にネイティブで、別プロセスもいらない。順位付けと分かち書きを最も自由に調整でき、将来の AI 検索や添付テキストにも流用できる。
- **注意**: 正本との二重管理になる（outbox による同期、commit と reload の遅延、索引の削除と期限処理の自前実装、復旧時の再構築）。事後フィルタによるページングのずれと、件数から存在が漏れる問題を設計で防ぐ必要がある。
- **開発環境への影響**: コンテナは増えない。索引用のボリュームが要り、ビルド時間とバイナリサイズが増える。

採らなかった候補:
- ParadeDB: AGPL-3.0 で、HA は Enterprise 版のみ。日本語の辞書は IPADIC に限られる。
- Meilisearch: 削除が非同期で反映される。BUSL の部分が混在し、別プロセスも要る。
- tsvector と日本語パーサ: 保守されているものが見当たらない。
- pg_trgm: 日本語に標準では対応していない。

## 6. 推奨

A（PGroonga）を入口にする。13 の即時拒否と 12 の件数・候補への ACL 適用を、索引と正本の二重管理なしで満たせるのは Postgres 内の方式だけで、その中で順位付け・強調表示・日本語の正規化を備えるのは PGroonga だからである。辞書を持たない bigram 方式は、辞書の更新なしに新語・製品名を拾える。英字とコードは語単位で扱われるので、識別子の部分一致が必要なら tokenizer の設定を追加で検証する。将来、意味検索や添付の大量テキストで順位付けを強化する必要が出たら、C を AI 検索専用の補助索引として足し、Postgres での再検査を必須にする。B は PGroonga のイメージが運用で問題になった場合の後退先にする。

## 7. 未確認事項

- PGroonga の公式イメージが rootless Podman で動くか。PostgreSQL 18 向けのタグがあるか。
- PGroonga の物理レプリケーション（`pgroonga.enable_wal`、`pgroonga_standby_maintainer`）と、Pelway のバックアップ・復元方式（R15）との整合。
- pg_bigm の PGDG パッケージ（apt/yum）の有無。
- PGroonga と pg_bigm の索引サイズ、書込みコスト、検索速度の実測（公的なベンチマークは本調査で確認していない。Pelway の合成データで測る必要がある）。
- 同じ列に TokenBigram と TokenMecab の索引を併用する構成の実例と性能。
- Tantivy の n-gram tokenizer と Lindera を併用した設定の詳細、および辞書（IPADIC、UniDic、SudachiDict）のライセンス。
- textsearch_ja のライセンスと最新の PostgreSQL への対応。
- Mastodon（standard tokenizer）での日本語検索品質。
- RAGFlow の `rag_tokenizer` の日本語処理、日本語 dataset が Infinity の `japanese` analyzer を使うか、削除の伝播、rootless Podman での動作。ClickHouse は compose で profile 指定時のみ起動するので、依頼文の「既定サービス」の一覧とは異なる。
- Mattermost の EnableCJKAnalyzers（v11.6〜）の記述、pgsql-hackers の議論、Pleroma の CJK 文書（いずれも本文未取得）。
- PGroonga の `fuzzy_max_distance_ratio` の挙動。
- Meilisearch の localizedAttributes による日本語・中国語の判定制御の挙動。
