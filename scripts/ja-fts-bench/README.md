# ja-fts-bench: PGroonga と pg_bigm の日本語全文検索ベンチマーク

Issue #93 の計測一式。結果と読み方は
`docs/research/ja-fts-pgroonga-pg-bigm-benchmark.md`。

## 前提

- rootless Podman（`podman run` が非 root で動くこと）
- ネットワーク: `docker.io`（ベースイメージ）、`dl-cdn.alpinelinux.org`（apk）、
  `packages.groonga.org`（Groonga/PGroonga ソース）、`github.com`（pg_bigm ソース）
- `--network host` での `podman build` が許可されていること
  （rootless ネットワーク経由だと Groonga バンドルの MeCab ダウンロードが
  タイムアウトするため）

## 使い方

```console
$ bash scripts/ja-fts-bench/bench.sh all      # ビルドから検証まで一括
$ bash scripts/ja-fts-bench/bench.sh setup    # フェーズ単独実行も可
$ N_MESSAGES=300000 bash scripts/ja-fts-bench/bench.sh setup  # 件数指定
$ bash scripts/ja-fts-bench/bench.sh cleanup  # コンテナ・ボリューム削除
```

フェーズ: `build` → `setup` → `quality` → `speed` → `acl` → `backup` →
`partial` → `verify`（`all` はこの順に全部）。結果は `work/`（git 管理外）。

## 構成

| ファイル | 役割 |
|---|---|
| `Dockerfile.pgbigm` | postgres:16-alpine + pg_bigm v1.2-20250903（ソースビルド） |
| `Dockerfile.pgroonga` | postgres:16-alpine + PGroonga 4.1.0 / Groonga 16.1.3。pgroonga/docker の alpine/16 と同じ手順 |
| `build-pgroonga.sh` | 上のビルドスクリプト本体（pgroonga/docker の alpine/build.sh と同一内容） |
| `bench.sh` | 計測本体。合成データ生成・品質・速度・ACL・バックアップ・部分一致 |

## 計測の設計（要点）

- **合成データ**: 日本語チャット風の本文 11 テンプレート × 語彙配列
  （漢字語・カタカナ語・製品名・コード断片・英文）を決定的に組み合わせ、
  10% を期限切れ、5% を削除済みにする。テンプレートの一部を半角カナ・
  全角英字に置き換えた表記ゆれ行と、打ち間違い語（進捙 など）を混ぜる。
- **検索の正解**: alive な行の `strpos(body, needle) > 0`（生の部分一致）で
  機械定義。`LIKE` の `_` ワイルドカード問題を避けるため strpos を使う。
  pg_bigm の `likequery` は記号をエスケップした上で LIKE と同義になる。
- **alive 判定**: `deleted_at IS NULL AND (expires_at IS NULL OR
  expires_at > now())`（13 の R03/D01 相当）。権限は `channel_members` への
  EXISTS で同じ SQL に組み込む。
- **バックアップ訓練（R15 相当）**: pg_dump リストア、kill -9 クラッシュ後の
  再起動（PGroonga は `pgroonga.enable_wal` の off/on 両方）、
  pg_basebackup からの復元を試す。
