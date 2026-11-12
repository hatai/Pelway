# 作業ログ: CI の高速化（ci-speedup-r1）

- 日付: 2026-11-12（木）JST
- 担当: Pelway 開発基盤エンジニア（T3 Code / OpenCode）
- worktree: `~/.t3/worktrees/Pelway/t3-a4507858-e470152488829eca`、ブランチ `t3/optimize-ci-speed`
- 基点: `a796ae38798603dfaac530847e4be80d7c7a93eb`（`main` への PR #106 マージ後）

## 目的

PR の品質ゲート（`mise run verify`）が 5 分余りかかり、PR ごとの待ち時間が長い。CI の実測からボトルネックを特定し、待ち時間を短縮する。

## 実測（変更前）

run `43017366934`（`ci/apt-retries` ブランチ、成功 run）のステップ別時間。

| ジョブ | 時間 | 内訳 |
|---|---|---|
| `mise run verify` | 5m06s | apt 11s / mise 14s / jactionlint 2s / **`mise run verify` 4m31s** |
| `octocov (coverage diff)` | 3m41s | apt 10s / **cargo install cargo-llvm-cov 46s** / **cargo llvm-cov 2m19s** / octocov 8s |
| `pr-size` | 3s | — |

`mise run verify` 4m31s の内訳（ログのタイムスタンプから算出）。

| タスク | 時間 | 備考 |
|---|---|---|
| clippy | 4m08s | 198 クレートを clippy-driver でコンパイル |
| test | 4m29s | 399 クレートをコンパイルし、テスト実行は 7s |
| build | 4m22s | test と同一の成果物のため実質キャッシュヒット |
| check | 4m22s | clippy と同一の型検査で実質重複 |
| hk-check | 4m23s | hk.pkl の cargo_fmt / cargo_clippy / cargo_check が重複 |

構造的な原因が 2 つある。

1. **依存の 89% が GUI クレート経由**。`Cargo.lock` 555 パッケージのうち、`pelway-client-ui` 経由のみで到達するのが 486。CI が実際にコンパイルした 383 クレートのうち **339（89%）が client-ui 経由のみ**（winit / accesskit / atspi / femtovg(C++) / arboard / softbuffer / x11rb / zbus / glutin など）。
2. **1 ジョブで 4 つの cargo が target dir のロックで競合**。`verify` の依存タスクは並列実行されるが同一 target dir を使うため、コンパイルイベントは 597 回（clippy 198 + test 399）で、一意のクレート数 383 に対して **214 回の重複**がある。

また `apt-get install libfontconfig1-dev` が run `43061100333` で 11 分 2 秒かかっている（ネットワーク障害時の待ち）。

## 先行事例の比較（Prior art）

`gh search code` / `gh search repos` で実例を確認した。

| 案 | 実例 | 採否 |
|---|---|---|
| A. Slint のフィーチャを絞る | 固有の事例は検索で出なかったが、Rust の feature unification の標準的な使い方 | **不採用**。`slint` を `default-features = false` にして winit バックエンド・femtovg レンダラ・accesskit・system-tray を外すと、CI のコンパイル対象が 383 → 241 クレート（-37%）になる（ローカル検証済み、client-ui のテストは全て pass）。ただし将来窓を開いたり読み上げに対応したりするときに feature を戻す必要があり、製品側の判断が要る |
| B. CI 固有の cargo プロファイル | `CARGO_INCREMENTAL=0` は lalrpop / feroxbuster / pomsky / iroha 等の CI で標準。debuginfo の除去も Rust CI の定番 | **採用**。検査の中身は変わらず、cold build だけ短縮する |
| C. ジョブ分割 | cargo は target dir 単位で fingerprint が分かれるため、lint と test を別ジョブに分けると重複コンパイルが消える | **採用** |
| D. ビルドキャッシュ | `Swatinem/rust-cache`（Ambient, cog, rustdoc 等）、`mozilla-actions/sccache-action`（nodejs, openvino, atuin 等） | **採用**。ただし本リポジトリの「リモートキャッシュは設定しない」方針の改訂が必要。S3 等の外部キャッシュサーバは使わない |
| E. cargo-llvm-cov の事前ビルドバイナリ | taiki-e が GitHub Releases にバイナリを配布している。mise の aqua バックエンドが URL と checksum を mise.lock に記録する（octocov と同じ経路） | **採用**。`cargo install`（ソースビルド 46s）から 5s 程度になる |
| F. verify 内の冗長な cargo 呼び出しの削減 | `cargo check` は clippy が型検査も行うため包含。`cargo build` は `cargo test` が同一の成果物を作る | **採用** |

## 変更内容

### mise.toml

- `[tools]` に `"aqua:taiki-e/cargo-llvm-cov" = "0.6.17"` を追加（`mise.lock` に URL と checksum が記録される）。
- 検査を 3 グループに分ける。`verify` はDepends で 3 グループを並列実行する。

  | グループ | depends |
  |---|---|
  | `verify-lint` | `fmt-check` / `clippy` / `toolchain-check` / `hk-validate` |
  | `verify-test` | `test` / `script-test` |
  | `verify-static` | `hk-check` / `fnox-check` / `pitchfork-check` |

- `verify` から `check` と `build` を外す（`check` は clippy に含まれる、`build` は `test` が同じ成果物を作る。タスク自体は残す）。
- `hk-check` を `hk check --all --step typos --step trailing_whitespace --step newlines` に変更する。hk.pkl の cargo_fmt / cargo_clippy / cargo_check は mise の `fmt-check` / `clippy` タスクと重なるため、hk 側では走らせない（cargo を伴う検査を 2 回走らせると、ジョブ分割後は target が別になって二重コンパイルがそのまま時間になるため）。hk.pkl の cargo ステップは git hook（pre-commit）用に残す。

### .github/workflows/verify.yml

- ジョブを `static` / `lint` / `test` / `coverage` / `pr-size` と、集約ジョブ `verify` に分ける。集約ジョブのチェック名は **`mise run verify` のまま**（PR の必須チェック名として使われるため。`docs/agents/grokbot-runbook.md` と `.agents/skills/review-pr/SKILL.md` が参照）。
- workflow レベルの `env` に CI 固有の cargo 設定を追加する。

  | 変数 | 値 | 理由 |
  |---|---|---|
  | `CARGO_INCREMENTAL` | `0` | 1 回きりのビルドでは incremental は無駄 |
  | `CARGO_PROFILE_DEV_DEBUG` / `CARGO_PROFILE_TEST_DEBUG` | `0` | debuginfo の生成とリンクを省略 |
  | `MISE_RUST_MR_BOXINGTON` | `0` | CI では素の cargo にする（mbx は `target` を symlink にしてキャッシュに載せない） |
  | `CARGO_TARGET_DIR` | `${{ github.workspace }}/target-ci` | symlink 先が変わってもキャッシュが壊れない固定の場所 |

- `actions/cache@v6.1.0` で cargo のレジストリと `target-ci` を保存する。キーは `<os>-cargo-<job>-<hash(Cargo.lock, mise.toml)>`、`restore-keys` で同じジョブの直前のキャッシュにフォールバックする。ジョブごとに分ける（lint / test / coverage は対象も最適化も違う）。
- `cargo install cargo-llvm-cov` のステップを削除し、`mise exec -- cargo llvm-cov` を使う。
- apt パッケージの導入は cargo をビルドする `lint` / `test` / `coverage` の各ジョブで行う（`static` は cargo を伴わないため不要）。

### docs/dev-environment.md

- 「CI（GitHub Actions）」節を、規約・ジョブ構成・CI 固有の cargo 設定・ビルドキャッシュ・カバレッジ・その他のジョブの 6 つの小節に分けて書き直した。
- 規約を「CI とローカルで**同じコマンド**」から「CI とローカルで**同じ検査**（検査は mise.toml のタスクに置き、CI でもローカルでも同じタスクを呼ぶ。実行の単位だけが違う）」へ変更した。

## 検証

| 項目 | 結果 |
|---|---|
| `mise run verify` | exit 0（3 グループとも成功） |
| `mise run verify-lint` / `verify-test` / `verify-static` | いずれも exit 0 |
| `mise exec -- jactionlint` | エラーなし（`actions/cache` の追加は `gh actions-lock` で actions.lock に反映済み） |
| `mise exec -- hk check --all --step typos --step trailing_whitespace --step newlines` | exit 0（cargo ステップが走らないことを確認） |
| `mise exec -- cargo llvm-cov --workspace --locked --no-cfg-coverage --lcov` | 完了。lcov.info（1.9MB）を生成 |
| `gh actions-lock` | `actions/cache@v6.1.0` → `sha1-55cc8345863c7cc4c66a329aec7e433d2d1c52a9` を actions.lock に記録 |

### CI 実測（workflow 変更前の state）

run `43078088810`（PR #112、workflow 変更前の状態。旧 workflow のまま）。

| 項目 | 変更前（run 43017366934） | この run |
|---|---|---|
| `mise run verify` ジョブ全体 | 5m06s | 5m10s |
| `mise run verify` ステップ | 4m31s | 4m38s |
| hk-check | 263.16s | **2.37s** |
| clippy | 248.94s | 260.24s |
| test | 269.25s | 276.49s |
| コンパイルイベント数 | 597（test 399 + clippy 198） | 688（test 446 + clippy 242） |

この run は旧 workflow のままなので、待ち時間の改善 は出ない。hk-check が 263s → 2.4s になったのは、hk.pkl の cargo ステップ（cargo_fmt / cargo_clippy / cargo_check）を走らせなくなったためで、この 263s はもともとキャッシュヒットで待ち時間には寄与していなかった。逆にその分、clippy と test が自分で依存クレートをコンパイルするぶんイベント数が増えている（597 → 688）。

つまり待ち時間を実際に縮めるのは workflow 側の変更（B: CI 固有の cargo プロファイル / C: ジョブ分割 / D: キャッシュ / E: 事前ビルドバイナリ）で、mise.toml 側の変更（F: 冗長な cargo 呼び出しの削減）だけでは縮まらない。

### CI 実測（ジョブ分割後）

run `43128754617`（PR #112、ジョブ分割とキャッシュを適用。キャッシュは空だったためコールドビルド）。

| ジョブ | 変更前（1 ジョブ） | 変更後 |
|---|---|---|
| `static` | — | 27s |
| `lint` | — | 28s（失敗。下記） |
| `test` | — | **2m37s** |
| `coverage` | 3m41s | **3m13s** |
| `pr-size` | 3s | 6s |
| 品質ゲート（集約 `mise run verify`） | **5m06s** | **2m44s** |

`test` ジョブの 2m03s はコンパイル、テスト実行は 3.6s。変更前は 1 ジョブで clippy と test が target dir のロックを奪い合っていたため 4m21s かかっていたが、ジョブを分けると 2m03s で済む。

### lint ジョブの失敗と対策

run `43128754617` の `lint` ジョブは、**コードの問題ではなく crates.io の一時的な取得失敗**で落ちた。

```
error: failed to get `parking_lot_core` as a dependency of package `parking_lot v0.12.5`
Caused by: unable to update registry `crates-io`
Caused by: download of pa/rk/parking_lot_core failed
Caused by: curl failed
Caused by: [16] Error in the HTTP2 framing layer
```

`Cargo.lock` が変わったためレジストリのキャッシュが不完全で、新しく増えた `dev-queue-compare`（sqlx 系）の依存を取得中出现した。対策:

1. `CARGO_HTTP_MULTIPLEXING=false` — HTTP/2 の多重化で起きる既知の不具合を避ける（cargo / libcurl の問題）。
2. `CARGO_NET_RETRY=10` — 一時的な失敗の再試行回数を既定の 2 から増やす。
3. `mise run fetch`（`cargo fetch --locked`）をビルド前に 1 回走らせる — 取得をビルドと分離し、取得の失敗で数分のコンパイルが無駄にならないようにする。

1 と 2 は `mise.toml` の `[env]` にも置いてあるため、CI ではないコンテキストからも効く。

### CI 実測（キャッシュが効いた状態）

run `43129863634`（PR #112、ジョブ分割済み・キャッシュあり）。

| ジョブ | 変更前 | ジョブ分割（コールド） | キャッシュあり |
|---|---|---|---|
| `static` | — | 27s | 25s |
| `lint` | — | 28s（取得失敗で失敗） | 3m03s※ |
| `test` | — | 2m37s | **1m44s** |
| `coverage` | 3m41s | 3m13s | **2m29s** |
| `pr-size` | 3s | 6s | 5s |
| **品質ゲート（集約）** | **5m06s** | 2m44s（失敗） | **3m08s** |

※ `lint` ジョブは前 run が取得失敗で落ちていてキャッシュを保存できていなかったため、clippy がコールドビルド（3m03s）になった。キャッシュが保存されていれば 1m30s 程度になる。

キャッシュの復元は 15s 程度（target-ci 616MB）。`test` ジョブの検証ステップは 56s（ほぼキャッシュヒットの状態で、ワークスペースのクレートの再ビルドとテスト実行）。

まとめ: 品質ゲートは 5m06s → 3m08s、`lint` のキャッシュが温まれば 1m50s 程度になる見込み。

### CI 実測（キャッシュ 2 段化後）

run `43134192404`（PR #112、全チェック success）。キャッシュのキー形式を
`Linux-cargo-registry-<hash>` と `Linux-cargo-target-<job>-<hash>` に変えた直後の run なので、
旧形式のキャッシュは新形式の restore-keys に一致せず**コールド**だった。

| ジョブ | 変更前 | ジョブ分割（コールド） | キャッシュ 2 段化（コールド） |
|---|---|---|---|
| `static` | — | 27s | 24s |
| `lint` | — | 28s（取得失敗で失敗） | 2m47s |
| `test` | — | 2m37s | 2m48s |
| `coverage` | 3m41s | 3m13s | 3m18s |
| `pr-size` | 3s | 6s | 8s |
| **品質ゲート（集約 `mise run verify`）** | **5m06s** | 2m44s（失敗） | **2m53s** |
| workflow 全体 | — | — | 3m21s |

キャッシュ 2 段化で実測したキャッシュサイズ:

| キャッシュ | サイズ | 備考 |
|---|---|---|
| `Linux-cargo-registry-<hash>` | 189MB | 3 ジョブで共有（従来はジョブごとに重複コピー） |
| `Linux-cargo-target-lint-<hash>` | 306MB | ジョブごと |
| `Linux-cargo-target-test-<hash>` | 485MB | ジョブごと |
| `Linux-cargo-target-coverage-<hash>` | 691MB | ジョブごと（計測ビルド） |

従来（レジストリと target を 1 つのエントリにまとめた状態）は lint 438MB / test 616MB / coverage 817MB で、同じ内容のレジストリを 3 コピーしていた。

この run の終了時に新形式のキャッシュが保存されたので、**次の run** がウォームになる。ウォームでは `lint` / `test` のコンパイルがキャッシュヒットし、検証ステップが十数秒に縮む見込みで、品質ゲートは 1 分台に入る。

### CI 実測（ウォームキャッシュ）

run `43134791520`（PR #112、全チェック success）。新形式のキャッシュが保存された後の run。

| ジョブ | 変更前（1 ジョブ） | コールド | **ウォーム** |
|---|---|---|---|
| `static` | — | 24s | **21s** |
| `lint` | — | 2m47s | **1m15s** |
| `test` | — | 2m48s | **1m36s** |
| `coverage` | 3m41s | 3m18s | **2m18s** |
| `pr-size` | 3s | 8s | 4s |
| **品質ゲート（集約 `mise run verify`）** | **5m06s** | 2m53s | **1m42s** |
| workflow 全体 | 5m06s | 3m21s | **2m22s** |

ウォームでの内訳（`test` ジョブ）: レジストリの復元 3s / ビルド成果物の復元 10s / 依存の取得 4s（取得なし）/ 検証ステップ 54s。

**品質ゲートは 5m06s → 1m42s（3.0 倍）、workflow 全体は 5m06s → 2m22s（2.2 倍）。**

### キャッシュ容量の考証

リポジトリのキャッシュ上限は 10GB。7 日以上アクセスされないエントリは失効し、`restore-keys` は一致エントリのうち最新の 1 件だけを復元する（= 各ジョブで生き残るのは最新の 1 世代）。実測で 1 世代あたりレジストリ 189MB（3 ジョブで共有）＋ビルド成果物 306〜691MB（ジョブごと）。`Cargo.lock` の変更は直近 30 日で 12 回なので、7 日ぶんの世代（3 世代前後）が常時残り、数 GB の範囲に収まる。上限に達した場合は oldest から失効するため CI は止まらない。

キーの形式を変えたとき、復元できない旧形式のエントリが残る（今回で 3.3GB）。7 日で失効するが、`gh cache delete` で先に消せる。

### 追加の最適化

| 案 | 内容 | 採否 |
|---|---|---|
| crates.io の取得設定 | `CARGO_HTTP_MULTIPLEXING=false` / `CARGO_NET_RETRY=10` | 採用（上記） |
| 依存の取得をビルドから分離 | `mise run fetch` をビルド前に走らせる | 採用（上記） |
| キャッシュを 2 段に分ける | レジストリは 3 ジョブで共有、ビルド成果物はジョブごと | 採用。キャッシュ容量の削減（実測 616MB / 823MB 程度）と、成果物が混ざるのを防ぐ |
| mise のツールインストールをキャッシュ | `~/.local/share/mise/installs` を `mise.lock` のハッシュでキャッシュする | 不採用。復元（約 1GB）はダウンロードと同程度で、待ち時間の短縮にならない |
| static ジョブで rust ツールチェーンを省く | `mise install --locked hk typos fnox pitchfork jactionlint` | 不採用。static ジョブは品質ゲート上のクリティカルパスではなく（27s 対 2m37s）、ツール構成を分ける割りに価値が小さい |
| sccache | コンパイラ出力を共有キャッシュする | 不採用。clippy と test はフラグが違い出力も別で、計測ビルド（instrument-coverage）も別。共有できる場面がほとんどない |
| Slint のフィーチャを絞る | 依存 399 → 249 クレート（CI のコンパイル対象 -37%） | 不採用（案 A と同じ。製品側の判断が必要） |

### 効果の見積り

- 2 つの cargo グループが別ジョブ・別ランナーで走るため、同一 target dir でのロック競合と clippy↔test の重複コンパイル（214 回）がなくなる。
- debuginfo と incremental を切って cold build を短縮する（測定値は CI 実測が必要）。
- キャッシュが効いた run では、依存クレートの再コンパイルがなくなる。
- `cargo install cargo-llvm-cov` の 46s が 5s 程度になる。

## 未検証・残存リスク

- **CI 実測は未確認**。このブランチの run で各ジョブの所要時間とキャッシュの効果を測り直す必要がある（キャッシュが効くのは 2 回目以降の run）。
- キャッシュのサイズ。`CARGO_PROFILE_DEV_DEBUG=0` の target-ci はローカル実測で 1.6GB。圧縮後もリポジトリのキャッシュ上限（10GB）に収まるか要確認。
- `MISE_RUST_MR_BOXINGTON=0` が mise の `install` 段階でも効くか。ローカルでは `mise exec -- cargo` と `mise run` の両方で mbx を通らないことを確認したが、`mise install --locked`（mise-action 経由）が同一の挙動になるかは CI 実測で確認する。`CARGO_TARGET_DIR` を `target` と別にしているため、mbx が有効でもキャッシュは壊れない。
- 集約ジョブの `if: always()` により、依存ジョブがキャンセルされた run では集約ジョブが失敗として記録される。concurrency group で古い run が打ち切られるため実害はないが、旧 run 側のチェックが failed になる。
