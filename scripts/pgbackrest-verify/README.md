# pgbackrest-verify: pgBackRest の保持・暗号化・復元の検証

Issue #101 の検証一式。R15（作成から最大 14 日）の保持、暗号化とチェックサム
検証つきの復元、12 §8.2 手順 6 の削除記録の再適用を確かめる。
結果と読み方は
`docs/research/pgbackrest-retention-restore-verification.md`。

## 前提

- rootless Podman（`podman run` が非 root で動くこと）
- ネットワーク: `docker.io`（ベースイメージ）、`dl-cdn.alpinelinux.org`（apk）、
  `github.com`（pgBackRest の tarball）
- ディスク: イメージとリポジトリで 1GiB 程度

## 使い方

```console
$ bash scripts/pgbackrest-verify/verify.sh all      # ビルドから検証まで一括
$ bash scripts/pgbackrest-verify/verify.sh setup    # フェーズ単独実行も可
$ bash scripts/pgbackrest-verify/verify.sh cleanup  # コンテナ・ボリューム削除
```

フェーズ: `build` → `setup` → `retention-count` → `retention-time` →
`expire-count` → `delete-reapply` → `restore-checksum`（`all` はこの順に
全部）。結果は `work/`（git 管理外）。

各フェーズは判定を満たすと `work/checks/<name>.ok` を作り、満たさないと
exit 1 で止まる。フェーズは単体で再実行できる（`retention-count` と
`retention-time`、`expire-count` は対象リポジトリを初期化してから始める）。

## 構成

| ファイル | 役割 |
|---|---|
| `Dockerfile` | postgres:16-alpine + pgBackRest 2.60.0（release tarball を sha256 検証して meson でビルド）+ libfaketime |
| `verify.sh` | 検証本体。保持方式・暗号化・復元・削除記録の再適用 |

## 検証の設計（要点）

- **保持**: 件数指定（`repo-retention-full=14`、count 型）と日数指定
  （`repo-retention-full-type=time`）を別リポジトリで比べた。件数指定は
  14 世代だけ残り、日数指定は 14 日超の 1 個が残る。
- **件数指定 + `expire --set`**: 日次バックアップが 7 日分欠けた状況
  （フル 14 世代で件数上限ちょうど）を count 型リポジトリに作り、
  残った 14 日超の世代を `expire --set` で個別に消して復旧対象が
  14 日以内に収まることを確かめる（#150）。
- **時刻の制御**: pgBackRest のフルバックアップのタイムスタンプは DB サーバ
  の時計から取る（`src/command/backup/db.c.inc` の `backupTime` はオンライン時
  `dbTimeMSec` を使う）。したがって「何日前のバックアップ」を作るには
  postgres 自体を libfaketime 付きで起動し直す。expire 側の判定は
  `time(NULL)` なので、backup 側の時刻を偽れば判定材料が実測できる。
- **暗号化**: `repo-cipher-type=aes-256-cbc`。平文がリポジトリに現れないこと、
  `pgbackrest verify` が通ること、subkey が弱くないこと（公式手順の
  ゼロバイト数）を確かめる。
- **チェックサム検証つきの復元**: 復元の前後で probe テーブルの md5 を比べ、
  実体ファイルを 1 バイト改変すると restore が失敗することを確かめる。
- **削除記録の再適用**: 削除を DB 外（別ボリューム）へ保全してから復旧し、
  復旧直後に削除済みデータが復活すること、記録を先に再適用すると復活し
  ないことを確かめる。
