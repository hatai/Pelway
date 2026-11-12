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
