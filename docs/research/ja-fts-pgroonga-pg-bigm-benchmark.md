# PGroonga と pg_bigm の合成データ比較（Issue #93）

計測日: 2026-11-12。`docs/research/japanese-full-text-search.md` の「未確認事項」のうち、
合成データでの実測（索引サイズ・書込みコスト・検索品質・検索速度）、権限・期限・削除込みの検索、
WAL と復元（R15 相当）、rootless Podman での動作、英字部分一致のトークナイザー設定を確かめた。
再現は `scripts/ja-fts-bench/bench.sh all`（`scripts/ja-fts-bench/README.md` 参照）。

## 1. 環境

| 項目 | 値 |
|---|---|
| マシン | Linux 24 コア / 38 GiB RAM（rootless Podman 4.9.3） |
| PostgreSQL | 16.15（公式 postgres:16-alpine イメージ、両者共通） |
| PGroonga | 4.1.0（Groonga 16.1.3、pgroonga/docker の alpine/16 Dockerfile から自前ビルド） |
| pg_bigm | v1.2-20250903（公式イメージが存在しないため postgres:16-alpine 上にソースビルド） |
| データ | 合成チャット 300,000 件。本文は漢字語・カタカナ語・製品名・コード断片・英文の 11 テンプレート組み合わせ（決定的）。10% 期限切れ、5% 削除済み（alive は約 85.5%）。テンプレートの一部を半角カナ・全角英字に置換した表記ゆれ行と打ち間違い語（進捙 など）を混入 |
| 検索の正解 | alive な行の `strpos(body, 語) > 0`（生の部分一致）で機械定義。評価クエリ 17 種（漢字・カタカナ・製品名・コード・打ち間違い・表記ゆれ・英字部分一致） |

チャネル 50・メンバー 200 の `channel_members`（参加率 30%）で権限を表し、
「削除されず（`deleted_at IS NULL`）・期限切れでなく（`expires_at`）・参加チャネル（EXISTS）」を
検索 SQL に組み込んだ形で計測する（13 の R03/D01、12 の現 scope 絞り込み相当）。

## 2. rootless Podman での動作

- PGroonga の公式配布イメージは本環境から pull できなかった（ghcr.io が 403、Docker Hub `pgroonga/pgroonga` は存在せず）。pgroonga/docker の alpine/16 Dockerfile をコピーして rootless `podman build` したところ正常にビルド・起動し、拡張・索引・検索がすべて動いた。
- ビルドは Groonga をソースビルドするため、rootless ネットワーク（既定）ではバンドルの MeCab ダウンロード（`download_mecab.rb`）が packages.groonga.org へ接続タイムアウトした。`podman build --network host` で解決。
- pg_bigm は公式イメージが存在しないためソースビルド。PG16 のサーバヘッダが `unicode/ucol.h`（icu-dev）と `clang21`/`llvm21`（bitcode 生成）を要求する点を Dockerfile に反映した。
- **結論: どちらも rootless Podman で動く。PGroonga は「pull できる公式イメージ」ではなく「公式 Dockerfile からビルドする」運用になる。**

## 3. インデックスの大きさ・書き込みの負荷

| 項目 | pg_bigm（GIN） | PGroonga（TokenBigram+NormalizerNFKC） |
|---|---|---|
| テーブル | 35.1 MB | 35.1 MB |
| 全文検索索引 | 19.4 MB（`pg_relation_size`） | 172.1 MB（`PGDATA/base/<dboid>/pgrn.*` の合計。`pg_relation_size` は 0 を返す） |
| 索引/テーブル比 | 0.55 倍 | 4.9 倍 |
| CREATE INDEX（300k 件） | 3 秒 | 2 秒 |
| 10,000 件追記（索引なし） | 5 ms | 6 ms |
| 10,000 件追記（索引あり） | 51 ms | 105 ms |

- PGroonga の索引は 5,000 件でも 164 MB、300,000 件でも 172 MB と、件数に対する増え方が小さい（Groonga のセグメント事前確保が支配的）。小規模では無視できず、大きくなると比率は下がると予想される。
- 追記コストは PGroonga が pg_bigm の約 2 倍。チャットの書き込みペースでは問題になる水準ではない。

## 4. 検索の質（再現率・追加ヒット・上位10件）

正解 = alive かつ `strpos(body, 語) > 0`。n_hit には正規化による追加ヒットを含む。
pg_bigm は LIKE と同義なので、部分一致正解に対して常に再現率 100%・追加 0（norm 系を除く）。

| 種類 | 例 | pg_bigm の再現率 | PGroonga の再現率 | PGroonga の追加ヒット |
|---|---|---|---|---|
| 漢字語（2〜3字） | 資料 / 会議 / 進捗 / 不具合 / 確認 | 100% | 100% | なし |
| カタカナ語 | スケジュール / フィードバック / パフォーマンス | 100% | 100% | なし |
| 製品名（大文字） | PostgreSQL | 100% | 100% | なし |
| 製品名（小文字） | postgres（正解 0） | — | 正解 0 に 1,911 件ヒット | `PostgreSQL` 行を大小正規化で拾う（正しい挙動） |
| コード断片 | pgroonga.enable_wal / cargo clippy | 100% | 100% | なし |
| 打ち間違い | 進捙（正解は誤字を含む行のみ） | 100%（誤字行のみ） | 100%（同） | なし。正字クエリ「進捗」が誤字行を拾うことは両者ともない || 英字部分一致 | pp（pp を含む行 = Apple 等 11,620 件） | 100%（11,620/11,620） | **0%**（0/11,620） | なし。既定 TokenBigram は ASCII を語単位トークン化するため語途中に当たらない |
| 表記ゆれ（半角カナ） | クエリ「スケジュール」/ 正解は ｽｹｼﾞｭｰﾙ 行 1,155 件 | **0%** | **100%**（1,155/1,155） | 全角カナ行も含む計 7,000 件ヒット |
| 表記ゆれ（全角英字） | クエリ「postgres」/ 正解は ｐｏｓｔｇｒｅｓ 行 1,911 件 | 0% | 100% | — |

上位 10 件の品質（pg_bigm は id 順、PGroonga は `pgroonga_score` 順）: 通常語では両者とも
正解のみを上位に返した。PGroonga で「正解外」が上位に混ざったのは大小正規化・表記ゆれの
追加ヒットのみ（例: コンプライアンスの上位 10 件中 2 件が半角カナ行）で、検索者の意図に
合う追加である。

## 5. 検索の速さ（alive 判定のみ / 権限・期限・削除込み）

EXPLAIN ANALYZE の Execution Time 中央値（3 回）。単位 ms。
権限・期限・削除込みは `EXISTS (channel_members)` + alive を同一 SQL に含む。

| クエリ（種類） | pg_bigm（単体） | PGroonga（単体） | pg_bigm（ACL 込み） | PGroonga（ACL 込み） |
|---|---|---|---|---|
| 資料（13,241 件） | 10.6 | 57.0 | 5.5 | 57.5 |
| 会議（6,888 件） | 8.8 | 56.7 | 3.9 | 58.8 |
| 確認（35,889 件） | 14.6 | 19.4 | 10.7 | 26.9 |
| スケジュール（5,845 件） | 9.0 | 57.5 | 5.0 | 58.3 |
| PostgreSQL（5,824 件） | 10.0 | 59.1 | 6.0 | 57.0 |
| pgroonga.enable_wal（5,816 件） | 13.4 | 57.7 | 9.1 | 55.8 |
| pp 部分一致（11,620 件） | 9.7 | 0.34（ヒット 0 のため） | 5.7 | 0.22（同） |

- 本計測では PGroonga が pg_bigm の約 5〜6 倍の時間がかかった（数十 ms オーダー）。
  ヒット件数が多い「確認」では差は縮まる。どちらも 100 ms 未満であり、チャット検索の
  UI では体感差は小さいと予想するが、要求次第で pg_bigm の速度優位は残る。
- 両者とも索引・正本が同じ Postgres 内にあるため、権限・期限・削除を同じ SQL に
  組み込んだ速度劣化は軽微（pg_bigm はむしろプランの違いで速い結果になった）。

## 6. WAL と復元（R15 のバックアップ訓練相当）

| 試験 | pg_bigm | PGroonga |
|---|---|---|
| pg_dump → 別コンテナへリストア → 検索一致 | 一致（6,888 件）。CREATE INDEX ごと再構築、REINDEX 不要 | 一致（6,888 件）。CREATE INDEX ごと再構築、REINDEX 不要 |
| kill -9（postmaster を SIGKILL）→ 再起動 → 検索一致 | 一致。REINDEX 不要 | 一致（`pgroonga.enable_wal` off / on 両方で確認）。REINDEX 不要 |
| pg_basebackup → 第二インスタンスで復元 → 検索一致 | 一致 | 一致 |

- `pgroonga.enable_wal` は PGroonga の .so をロードしたバックエンドでしか登録されない
  GUC で、`psql` の同一セッション内で検索を一度実行してから `ALTER SYSTEM SET` する
  必要があった（運用上の注意として記録する）。
- 今回の kill -9 試験（コミット済み INSERT 直後に強制停止）では `enable_wal` off でも
  索引破損は再現しなかった。PGroonga の索引更新はコミット時に同期完了するためと
  推測するが、crash safe（`pgroonga_crash_safer`、experimental）を使わない運用では、
  破損時の REINDEX による復旧（`pgroonga_list_broken_indexes()` で検出）を手順に
  組んでおく必要がある。
- **結論: 論理バックアップ（pg_dump）と物理バックアップ（pg_basebackup）のどちらでも、
  PGroonga の索引は復元後に検索可能になり、R15 の日次 snapshot と復元演習の前提を満たす。
  pg_dump リストアでは索引の再構築が自動で走る（データ量に比例した時間がかかる）。**

## 7. 英字・コードの部分一致（「pp」で「Apple」）

3 行の小表（Apple Silicon / PostgreSQL / grep rg）に既定 TokenBigram 索引と
`TokenBigramSplitSymbolAlpha` 索引を両方作り、`pgroonga_condition(query, index_name => ...)`
で索引を指定して比較した。

| クエリ | 既定 TokenBigram + NormalizerNFKC | TokenBigramSplitSymbolAlpha |
|---|---|---|
| pp → Apple | 0 件 | 1 件 |
| ostgre → PostgreSQL | 0 件 | 1 件 |
| ilico → Silicon | 0 件 | 1 件 |

- 調査ドキュメントの「『pp』で『Apple』は当たらない」は再現した。**トークナイザーを
  `TokenBigramSplitSymbolAlpha` に変えることで直る。**
- 注意: 両方の索引が存在する状態で索引を指定せず `body &@~ 'pp'` を実行すると、
  プランナが SplitSymbolAlpha 側を選んでヒットする（見かけ上「当たる」）。
  挙動を確認するときは索引の指定が要る。
- なお Groonga 16.1.3 に `TokenBigramSplitKeyword` は存在しない
  （`pgroonga_command('tokenizer_list')` で確認。SplitSymbol / SplitSymbolAlpha /
  SplitSymbolAlphaDigit などが実在する）。
- 300k 件の本命データでは、pp 部分一致（正解 11,620 件）を pg_bigm は 100% 拾い、
  PGroonga 既定索引は 0 件だった。英字識別子の部分一致を PGroonga で通すには、
  追加の索引（SplitSymbolAlpha）を用意するか、pg_bigm を併用する設計になる。
- おまけ: `pgroonga_condition('進捙', fuzzy_max_distance_ratio => …)`（誤字許容、
  3.1.6+）を 0.34 / 0.5 / 0.6 の 3 段階で試したが、いずれもヒットは誤字を含む行
  13,221 件のまま（正字「進捗」を含む行 13,162 件は拾えない）。2 文字の bigram
  トークンでは fuzzy による置換 1 文字の許容が効かなかった。誤字対策としての
  fuzzy への期待はこの用途では薄い。

## 8. まとめ（#92 の「02 の採否」への入力）

- **PGroonga が pg_bigm に勝る点**: 表記ゆれの正規化（半角カナ・全角英字・大小を
  正規形クエリで拾える。pg_bigm は 0%）、スコア順位・強調表示・snippet、
  索引サイズの件数非依存性、CREATE INDEX の速さ。
- **pg_bigm が PGroonga に勝る点**: 索引サイズ（0.55 倍 vs 4.9 倍）、書き込み負荷
  （約 1/2）、検索速度（本計測で約 5〜6 倍速い）、英字部分一致を特別な設定なしで
  100% 拾える（LIKE と同義のため）。
- **どちらでも変わらない点**: alive・権限を同じ SQL で判定する構造、pg_dump /
  pg_basebackup による復元、kill -9 復帰、rootless Podman での運用。
- 調査ドキュメントの推奨（A: PGroonga を入口に、B を後退先）に対し、本計測は
  「PGroonga の速度・索引サイズの代償は実用範囲内で、正規化と順位付けの利得が
  ある」ことと「英字部分一致だけは追加索引か併用が必要」ことを示す。
  最終判断（#92 の採否）はこの数字を前提に人間が行う。

## 9. 制約（結果の解釈の範囲）

- 合成データはテンプレート組み合わせであり、実チャットの語彙分布・長さ分布とは
  異なる。特に語彙数が少なく、同一語のヒット件数が均一になりやすい。
- 検索速度は 1 マシンのコンテナ内実測。PGroonga は実運用でメモリ上に載る範囲が
  大きいほど速くなる性質があるため、データ量を増やした再計測で傾向が変わる可能性。
- kill -9 試験はコミット済みトランザクションの消失を含まない（同期コミット後の
  強制停止）。WAL 生成中の破損パターンは再現できていない。
- 本レポートの PR はベンチマーク基盤の PR を base にした stacked PR として出す。
  CI（verify.yml）は base が main の pull_request でだけ走るため、この間は CI が
  走らない（実際の差分は `scripts/pr-size.sh --base <base ブランチ>` で確認できる）。
  base が main に切り替わった後に CI が走り、pr-size は +178 に収まる。
