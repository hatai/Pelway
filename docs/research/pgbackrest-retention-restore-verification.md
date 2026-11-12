# pgBackRest の保持（14日）と復元の検証（Issue #101）

`docs/research/oss-data-and-collaboration.md` §7 が挙げた「MVP 前に検証」の項目
2（pgBackRest）を実機（rootless Podman + PostgreSQL 16）で確かめた。
再現は `scripts/pgbackrest-verify/verify.sh all`
（`scripts/pgbackrest-verify/README.md` 参照）。

検証日: 2026-11-12。対象は pgBackRest 2.60.0（release/2.60.0 の
ディストリビューション tarball、sha256 検証済み）と PostgreSQL 16.15。

## 1. 環境と構成

| 項目 | 値 |
|---|---|
| マシン | Linux 24 コア / 38 GiB RAM（rootless Podman 4.9.3） |
| PostgreSQL | 16.15（公式 postgres:16-alpine イメージ） |
| pgBackRest | 2.60.0（meson + ninja でソースビルド、libpq・OpenSSL・libxml2・lz4・bz2・zlib は必須、zstd は任意） |
| リポジトリ | POSIX 型（`repo1-type=posix`）。2 つ用意して保持方式を分けて測った |
| 暗号化 | `repo-cipher-type=aes-256-cbc`（両リポジトリ） |
| データ | 合成チャット 200 行（`messages` テーブル）、復元照合用の `probe` 5 行 |
| WAL | `archive_mode=on` + `archive_command=pgbackrest archive-push`。archive-push は設定済みの全リポジトリに push する |

## 2. 保持方式: 件数指定と日数指定の比較

### 2.1 件数指定（count 型、`repo-retention-full=14`）

毎日 02:00 のフルバックアップを 15 日分作った（15 個目を作った時点で最古の
1 個が expire される）。結果:

| 項目 | 実測 |
|---|---|
| 作った世代数 | 15 |
| 残った世代数 | 14 |
| 最も古い残存世代の経過日数 | 13 日 |
| 残存ラベルの連続性 | 20261030-020001F 〜 20261112-020000F（欠けなし） |
| 残った差分・増分 | 0 件（full が消えるのに連動して消える） |

**結論: 件数指定 14 が保証するのは「世代数の上限 14」だけ。** 毎日フル
バックアップのシナリオでは、実測のとおり復旧対象は 14 日以内に収まり、
R15 の「snapshot 作成から最大 14 日で復旧対象外」を満たす。ただし
件数指定は世代数しか制限しないので、日次バックアップが欠けると
14 日を超えた世代が残り得る（欠落日を含むシナリオは未検証、#150）。
「最大 14 日」を件数指定だけで保証することはできない（候補の仕組みは
§2.4）。

ただし定常の保持容量とは別に、バックアップ作成中の一時容量が要る。新しい
フルバックアップの作成中は、最古の世代が expire されるまで `retention + 1`
個（15 世代）が一時的に並存する。13 章 C06 の「14 世代 = 168GiB」は定常の
14 世代分なので、これに作成中の 1 世代分を足した容量を見ておく。

### 2.2 日数指定（time 型、`repo-retention-full-type=time` + `repo-retention-full=14`）

20 日前・16 日前・3 日前のフルバックアップを 3 つ作った。結果:

| 項目 | 実測 |
|---|---|
| 14 日超のバックアップの残存 | 1 件（20261027-020000F = 16 日前） |
| 自動で消えたもの | 20 日前の世代（最古の 1 個） |
| `expire --set=<ラベル>` の効果 | 14 日超の 1 個を個別に消せた（残り 1 世代） |

**結論: 日数指定だけでは「作成から最大 14 日」を守れない。** 14 日超の
バックアップが 1 個残る。

理由は仕様どおりで、日数指定は「14 日より古いもの」を消すのではなく、
**「14 日より古いものの中で最も新しい 1 個は残す」**（復旧範囲の最低限を保証
する）設計になっている。公式設定リファレンスの例（30 日指定で 25 日前と
35 日前のバックアップがあるときどちらも消えない）と同じ挙動で、
実装（`expireTimeBasedBackup`）も一致した。

- 設定リファレンス: `repo-retention-full-type=time` —
  "full backups older than repo-retention-full will be removed from the
  repository if there is at least one other backup that is equal to or
  greater than the repo-retention-full setting."
- ソース: `src/command/expire/expire.c` の `expireTimeBasedBackup()`

### 2.3 R15 を満たす方式の検討（実測と未検証の仮説）

| 方式 | 復旧対象の最大経過日数 | 判定 |
|---|---|---|
| 件数指定のみ（毎日フルバックアップ、欠落なし） | 14 世代、最古は 13 日前（実測） | 実測で満たす |
| 件数指定のみ（欠落日を含む） | 世代数しか制限しないため 14 日超の世代が残り得る（欠落シナリオ自体は未検証） | 保証できない |
| 日数指定のみ | 14 日を超える場合がある（実測で 16 日前の世代が残存） | **満たさない**（実測） |
| 日数指定 + `expire --set` による追加削除 | 14 日以内に収まる（repo2 で実測） | 実測で満たす（time 型リポジトリのみ） |
| 件数指定 + `expire --set` による 14 日超世代の定期削除 | 14 日以内に収まるはず（欠落日があっても） | **未検証の仮説**（§2.4、検証は #150） |

**採用候補（未検証）: 件数指定（`repo-retention-full=14`、type は既定の
count）で世代数を制限しつつ、14 日を超えた世代を `expire --set` で消す
定期削除を併用する仕組み。** この運用で R15 を満たすかは、count 型
リポジトリでの `expire --set` が未検証なので断定できず、#150 の検証待ち。
実測で 14 日以内に収まった方式は「日数指定 + `expire --set`」だけで、
time 型リポジトリに限られる。日数指定単独は 14 日超の世代を残すので
候補にしない。

### 2.4 「最大 14 日」を保証する候補の仕組み（未検証、検証は #150）

件数指定は世代数しか制限しないので、「最大 14 日」の保証を削除処理側が
担う仕組みを仮説として置く。確かめた事実と未検証の仮説は次のとおり。

1. **14 日を超えた世代を日次で消す削除処理**（仮説）。`pgbackrest
   info --output=json` で世代の開始時刻を読み、14 日を超えたラベルに
   `pgbackrest expire --set=<ラベル>` を実行する。
   - 実測済み: `expire --set` は指定した世代とそれに連なる差分・増分を
     まとめて消す（time 型の repo2、§2.2）。
   - 未検証: count 型リポジトリ（repo1）での `expire --set`、欠落日を
     含むシナリオ、削除処理を定期化した運用。pgBackRest の expire は
     ラベル指定で世代を消すコマンドなので同じ操作になるはず、という
     仮説。検証は #150。
   - この削除処理が止まれば上限は破るため、バックアップとは別の失敗
     として見える形（例: 日次ジョブの成否の監視）で動かす。
2. **最新の復旧点の鮮度は別の問題**。バックアップが数日欠けると、上限は
   守れても最新の復旧点が古くなる。これは expire では解決できないので、
   「毎日フルバックアップが取れていること」を監視し、欠落を検知して
   再開後すぐフルを取る運用が前提になる（こちらも未検証）。

また `expire --set` で最後の世代を消すと復旧点がなくなる。実運用では
「新しいフルができてから古い世代を消す」順序にする。

## 3. 暗号化とチェックサム検証つきの復元

2.59.3 以上（2.60.0）で、暗号化を有効にしたリポジトリで実測した。

| 検査 | 結果 |
|---|---|
| 平文の混入 | なし。合成データの目印文字列 `PELWAY-BACKUP-MARKER` がリポジトリ内のどのファイルにも現れない |
| `pgbackrest verify` | 成功（リポジトリ全体のチェックサム検証） |
| 暗号 subkey の健全性 | ゼロバイト数 0（弱い subkey の判定基準は 6 バイト以上） |
| 復元 | 成功。復元の前後で `probe` 5 行の md5 が一致（`7fc71136f31cc1c97659772b39202edb`） |
| 改変の検知 | 実体ファイルを 1 バイト改変すると restore が失敗（`zlib threw error: [-3] data error`） |

### 3.1 2.59.3 未満を避ける理由

pgbackrest.org の「Weak Encryption Subkeys and Salts」によれば、2.59.3 未満は
暗号化用の乱数生成が失敗しても検出せず、弱い subkey と salt を作る可能性が
ある。影響を受けるのは暗号化を有効にしたリポジトリだけ（未暗号化は影響なし）。
subkey は `stanza-create` で一度生成され、その後も同じものが使われるため、
アップグレードしても弱い subkey は置き換わらない。

対策は 2.59.3 以上を使うこと。本検証は 2.60.0 を使い、公式ページの手順
（`repo-get` で `cipher-pass` を取り出し、復号した subkey のゼロバイト数を
数える）で弱さがないことも確認した。

### 3.2 チェックサム検証が効く層

restore は復元の各ファイルについて、manifest に記録された SHA-1 と、復号・
展開後の実データの SHA-1 を照合する
（`src/command/restore/file.c.inc` の
"actual checksum ... does not match expected checksum"）。

実測では、1 バイトの改変は暗号化・圧縮・照合のどの層で先に失敗するかが
改変位置によって変わる。

- 暗号ヘッダ付近を改変 → `cipher header invalid`
- データ中ほどを改変 → `zlib threw error: [-3] data error`

いずれも restore は中断する（成功はしない）。検出層が違っても、
「改変を検知して復元を中断する」ことには変わりない。

なお `checksum-page-error`（2.60.0 で追加）は、ページチェックサムが不正な
ファイルを含むとき restore をエラーで止める。13 章の「残存失敗を隠さない」
方針に沿う設定だが、ページチェックサムは initdb 時に有効化する必要がある
（今回は未検証）。

## 4. 削除の要件（12 §8.2 / 13 章）とバックアップ保持の関係

### 4.1 手順 6 の再適用が合成データで通ること

12 §8.2 手順 6 は「復旧時は認可と外部dispatchを既定停止し、復旧DBと独立して
保全した削除・制限・撤回記録を先に再適用し、現ACLと根拠を再確認する」と
定める。合成データで一連の手順を通した。

| 段階 | 実測 |
|---|---|
| バックアップ取得後の削除 | `messages` の 3 行を `deleted_at` で削除し、ID と削除時刻を DB 外（別ボリューム `/records`）へ保全 |
| クラッシュ相当の復旧 | PGDATA を消してバックアップから restore |
| 復旧直後 | 削除済み 3 件が**復活**している（`deleted_at IS NULL` が 3 件）。認可・dispatch は停止状態（`recovery_guard.reapply_state = 'pending'`） |
| 削除記録の先行再適用 | 独立保全先（`/records/deleted-records.tsv`）の記録を復元後の DB に適用 |
| 再適用後 | 削除済み 3 件は復活しない（`deleted_at IS NULL` が 0 件）。停止解除（`reapply_state = 'done'`） |

**結論: 手順 6 は合成データで通る。** ただし復旧直後に削除済みデータが復活
することから、削除記録の再適用を省略する運用は 12 §8.2 に反する。

### 4.2 保持期間と削除記録の寿命の関係

13 章 R16 は削除・失効・操作抑止の照合記録を「制限発効又は結果確定から
最低 90 日、かつ関連する復元可能なコピーや未完了処理がなくなるまで」保持すると
定める。R15 の 14 日に対し、削除記録の寿命は 90 日で長い側にあるため、
記録の保持がバックアップの保持より短いことはない。

ただし「関連する復元可能なコピーや未完了処理がなくなるまで」という条件は残る。
14 日のバックアップが 1 個でも残っているあいだ、削除記録は残す必要がある。
毎日フルが続く件数指定 14 世代の運用では、バックアップの復旧対象が
14 日で消えるのに対し、削除記録は 90 日まで残す（= バックアップが消えた
後も 76 日余分に保持）。これは 13 章の既存方針と矛盾しない。

### 4.3 表示の区別（12 §8.2）

12 §8.2 は「backup の 14 日は復旧対象の上限、世代の物理消去 24 時間は内部
目標であることを区別し、全 backup まで消えたとは称しない」と定める。
pgBackRest で観測できることは次の 3 つで、表示の 3 状態に対応する。

| 12 §8.2 の語 | pgBackRest で観測できるもの |
|---|---|
| 復旧対象 | `pgbackrest info` に現れるバックアップ。件数がそのまま復旧対象件数 |
| 世代が対象から外れる | `expire` が `backup.info` からラベルを消した時点 |
| 物理消去 | expire 後に `backup/` 配下の実体が削除される。内部目標 24 時間の対比材料になる |

### 4.4 添付 volume と整合取得点

04 の「DB と添付 volume の backup に同じ整合取得点」は pgBackRest の対象外
である（本検証も対象外）。pgBackRest は PG クラスタ単位のバックアップに
なるため、添付 volume は同一の整合取得点を持つ retention とは別の手段の
設計が要る。この点は `docs/research/oss-data-and-collaboration.md` の
「Pelway への当てはめ」と同じ結論で、今回も変わらない。

## 5. 保守の体制

一次情報（pgbackrest.org/news.html、2026-11-12 取得）で確認した。

| 日付 | 事象 |
|---|---|
| 2026-04-27 | 作者が「pgBackRest Is No Longer Being Maintained」を発表 |
| 2026-05-04 | 「Maintenance Update」。スポンサー連合で継続の資金を確保できそうだと表明 |
| 2026-05-18 | 「pgBackRest Will Continue!」を発表。スポンサー連合が継続のための資金を提供 |
| 2026-10-04 | 2.59.3 リリース。弱い subkey/salt の不具合を修正 |
| 2026-10-18 | 2.60.0 リリース |

スポンサー連合は 2026-05-18 の時点で **AWS、Supabase、pgEdge、Tiger Data、
Percona、Eon** の 6 社。2.59.3 / 2.60.0 のリリースは **Xata、Dalibo、
Data Egret** を加えた 9 社の支援で行われた。

**結論: 保守の継続は、単一のスポンサーではなく 6 社（後に 9 社）の連合に
分散しており、1 社の継続不能による事業継続の中止の影響は小さい
という作者側の説明がある（"a coalition of sponsors has come together to
fund ongoing development. Their support means the project is no longer
reliant on a single sponsor"）。**

ただしこれは 2026-11-12 時点の事実であり、保証ではない。継続が
資金次第という前提は残る。

### 5.1 2.60.0 の Important Notes（運用への影響）

2.60.0 のリリースノートに、13 章の運用方針に直結する変更がある。

- restore は復元したファイルに不正なページチェックサムがあると error で
  止まり、クラスタが不完全な状態で起動できなくなる（restore の再実行が
  必要）。
- expire は、最古の残存バックアップにチェックサムエラーがあると expire を
  拒否する（そのリポジトリの archive も expire されない）。
- `checksum-page-error=n` で error の代わりに warning にできる。

これは「残存失敗を隠さない」（13 章 R15）と「世代を静かに消さない」の
方針に沿う。使うなら既定の error のままにするのが 13 章の意図に合う。

## 6. まとめ（13 章 R15・C06 への入力）

- **「最大 14 日」は未検証の運用では断定しない。** 実測で確かめたのは、
  毎日フルバックアップのもとで件数指定 14 が 14 世代（最古 13 日前）を
  残すこと、日数指定が 14 日超の世代を 1 個残すこと、time 型
  リポジトリで `expire --set` が 14 日超の世代を個別に消せること。
  「件数指定 + 定期 `expire --set`」の仕組み（§2.4）は count 型
  リポジトリでの `expire --set` が未検証のため仮説であり、この運用で
  R15 を満たすかは #150 で検証する。日数指定（time 型）単独は
  14 日超の世代を残すので候補にしない。
- **定常は 14 世代、バックアップ作成中は 15 世代分の容量が要る。** 定常の
  保持は 14 世代で、13 章 C06 の「14 世代 = 168GiB」がそのまま定常容量に
  相当する。新しいフルバックアップの作成中は、最古が expire されるまで
  `retention + 1` の 15 世代が一時的に並存するので、容量は 15 世代分
  見ておくと安全（実測でも 15 個目を作った時点で最古が消えた）。
- **暗号化は 2.59.3 以上で有効にする。** 2.59.3 未満は弱い subkey を
  作る可能性がある。`stanza-create` で subkey は一度しか生成されないので、
  バージョンを見ずに既存リポジトリを引き継がない。
- **復旧時は削除記録の再適用を省略しない。** 復旧直後には削除済み
  データが復活する（実測）。12 §8.2 手順 6 の独立保全が前提になる。
- **削除記録の保持（90 日）はバックアップの保持（14 日）を下回らない。**
- **保守はスポンサー連合（6 社、2.59.3 以降は 9 社）により継続している。**
  継続は資金次第という前提は残る。

## 7. 制約（結果の解釈の範囲）

- 合成データは 200 行の小さなテーブルで、実運用のデータ量・WAL 量・圧縮率
  は測っていない。リポジトリの size は 14 世代で 125.7MiB（実データ 29.5MiB
  に対し backup set size は各 3.9MiB 前後）。C06 の容量試算には、圧縮率と
  WAL 量の実測が必要で、今回の値はそのまま使えない。
- 「欠落日を含む」シナリオは未検証。14 日超世代の `expire --set` は
  time 型リポジトリ（repo2）でのみ実測。count 型リポジトリでの同操作と、
  削除処理を定期化した運用は仮説（§2.4）で、検証は #150 に切り出した。
- 保持の検証は POSIX 型のローカルリポジトリで行った。S3 互換
  オブジェクトストアでも retain の判定はリポジトリ内の backup.info に基づいて
  同じだが、削除（物理消去）の遅延はストアによって異なる。
- 時刻は libfaketime で偽装して作った。実際の backup は DB サーバの時計を
  使う（`src/command/backup/db.c.inc` の `backupTime` はオンライン時
  `dbTimeMSec` を使う）ため、postgres 自体を偽装時刻で起動し直している。
  時計が不正なホストでの挙動は別途確認していない。
- 添付 volume、バックアップと添付の整合取得点、実ドメインの保存先
  （国内 Amazon S3 相当）、障害注入（リポジトリの破損、応答遅延）は
  対象外。
- `checksum-page-error` の実挙動は未検証（ページチェックサムを有効化した
  クラスタを作っていない）。

## 8. 出典

[pgBackRest News, 2026/11] pgBackRest. "News." https://pgbackrest.org/news.html
[pgBackRest Configuration Reference, 2026/11] pgBackRest. "Configuration reference." https://pgbackrest.org/configuration.html
[pgBackRest User Guide, 2026/11] pgBackRest. "User Guide." https://pgbackrest.org/user-guide.html
[pgBackRest 2.60.0, 2026/11] pgBackRest. "pgbackrest-2.60.0.tar.gz." GitHub. https://github.com/pgbackrest/pgbackrest/releases/download/release/2.60.0/pgbackrest-2.60.0.tar.gz
[pgBackRest Source expire.c, 2026/11] pgBackRest. "src/command/expire/expire.c." GitHub. https://github.com/pgbackrest/pgbackrest/blob/release/2.60.0/src/command/expire/expire.c
[pgBackRest Source restore/file.c.inc, 2026/11] pgBackRest. "src/command/restore/file.c.inc." GitHub. https://github.com/pgbackrest/pgbackrest/blob/release/2.60.0/src/command/restore/file.c.inc
[pgBackRest Source backup/db.c.inc, 2026/11] pgBackRest. "src/command/backup/db.c.inc." GitHub. https://github.com/pgbackrest/pgbackrest/blob/release/2.60.0/src/command/backup/db.c.inc
