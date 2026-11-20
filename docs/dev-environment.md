# ローカル開発環境の構築と手順

- 対象: Windows 11 + WSL2（Ubuntu 24.04、systemd 有効）上のこのリポジトリ。
- 位置づけ: **開発環境の設定と手順の記録**。製品機能の実装ではない。`crates/dev-smoke` とルートの `Cargo.toml` はビルド・試験の経路確認用スケルトンであり、製品コードではない。
- 上位の設計は [技術仕様 統合版 v0.7 の05章](technical/v0.7/body/05-local-dev.md)（backend に WSL2 内ランタイム、macOS に Apple container を想定した構成案。実行承認前の設計であり、実施報告ではない）。

## ツール構成と採否

| ツール | 役割 | このリポジトリでの採否 |
|---|---|---|
| [mise](https://mise.jdx.dev) | 言語・ツール・環境変数の管理 | 採用。`mise.toml` がバージョンの単一管理元。グローバル設定（`mise use -g`）は使わない |
| Rust（rustc / cargo / rustfmt / clippy） | 言語とビルド | 採用。mise 経由で 1.96.0 を固定し、`components` に rustfmt・clippy を明示する。`rust-toolchain.toml` は置かない（バージョン宣言を mise.toml に一本化するため） |
| [mr boxington](https://mr-boxington.jdx.dev)（`mbx`） | Cargo ビルドの共有キャッシュ | 採用。mise の `mr_boxington = true` オプションで cargo をラップ（ローカルキャッシュのみ。リモートキャッシュは設定しない） |
| [hk](https://hk.jdx.dev) | git hook の管理 | 採用。`hk.pkl` に手順を定義し、フックはリポジトリローカルの Git 設定にインストール |
| [typos](https://github.com/crate-ci/typos) | スペルチェッカ | 採用。`hk.pkl` の `Builtins.typos` ステップ（pre-commit と `hk check --all`＝CI で実行）。ツールは mise で 1.52.0 に固定し、除外は `typos.toml` に置く |
| [fnox](https://fnox.jdx.dev) | シークレットの管理 | 採用。`fnox.toml` にはシークレットの名前と秘密を含まないテンプレートだけを置く |
| [pitchfork](https://pitchfork.jdx.dev) | ローカル開発用プロセスマネージャー | 採用。`pitchfork.toml` に AI 開発チームの見張り役 `agent-watch` を定義する。ツールは mise で固定 |
| [cargo-deny](https://embarkstudios.github.io/cargo-deny/) | 依存クレートの脆弱性（RustSec）・ライセンス・重複・取得元の検査 | 採用。`mise.toml` で 0.20.2 を固定し、ポリシーは `deny.toml` に置く。`verify-static`（CI の static ジョブ）で実行 |
| [shellcheck](https://github.com/koalaman/shellcheck) | シェルスクリプトの静的検査 | 採用。`mise.toml` で 0.11.0 を固定し、`shellcheck` タスク（`verify-static`・CI の static ジョブ）が demo-check 一式（`scripts/demo-check/*.sh`）を検査する。severity は既定のまま（Issue #265） |

## セットアップ手順（初回）

前提: mise 2026.9.2 以上（`mise.toml` の `min_version` が未満のバージョンを拒否する）。インストールは [公式手順](https://mise.jdx.dev/installing-mise.html) に従う。

`mise trust` のあと `mise bootstrap` を1回実行すれば完了する。`mise bootstrap` の実行内容と順序（mise の run order 仕様）:

| 順 | 処理 | 内容 | 単独実行する場合のコマンド |
|---|---|---|---|
| 1 | system packages | `[bootstrap.packages]` の apt:podman（rootless Podman 一式）。確認プロンプトと sudo のパスワード入力がある | `mise bootstrap packages apply --manager apt` |
| 2 | tools | `[tools]` のツール一式（`mise install` 相当） | `mise install` |
| 3 | `bootstrap` タスク | hk の git hook 導入（`hk install --mise`）と環境の一括確認（`mise run verify`） | `mise exec -- hk install --mise` ／ `mise run verify` |

```bash
cd <このリポジトリ>
mise trust        # 初回のみ。プロジェクト設定の信頼確認（グローバル設定は変更しない）
mise bootstrap    # 上の1〜3を順に実行
```

- 導入済みのものはスキップされる（宣言と現在の差分だけを適用）。このため `mise bootstrap` は何度実行してもよく、podman 導入済みの環境では packages 部分で sudo の入力を求められない。
- 各段階は上表のとおり単独のコマンドでも実行できる。
- `mise bootstrap` の tools 部分は `[tools]` に加えてグローバル設定のツールも確認対象にする（mise の通常動作）。

- `mise install` と `mise.lock` の生成は `mise.toml` の `[settings] lockfile = true` でこのプロジェクト自身が指定する（ユーザーのグローバル設定に依存しない）。`mise.lock` と `Cargo.lock` はコミット対象。
- rust のツール定義には `components = ["rustfmt", "clippy", "llvm-tools-preview"]` を明示する（`llvm-tools-preview` は CI のカバレッジ計測が必要とする）。成分の存在は `mise run toolchain-check` で検査する。
- エディタ等の非対話プロセスからは `mise exec -- cargo build` の形で実行する（mise の shim またはラッパー経由で cargo が解決される）。

## 日常のタスク（mise tasks）

| コマンド | 内容 |
|---|---|
| `mise run fmt` | `cargo fmt --all`（整形） |
| `mise run fmt-check` | 整形済みかの検査のみ |
| `mise run check` | `cargo check --workspace --all-targets --locked` |
| `mise run clippy` | `cargo clippy --workspace --all-targets --locked -- -D warnings` |
| `mise run test` | `cargo test --workspace --locked` |
| `mise run build` | `cargo build --workspace --locked` |
| `mise run fetch` | 依存クレートの取得（`cargo fetch --locked`）。ビルド前に取得を済ませる |
| `mise run toolchain-check` | rustc / cargo / rustfmt / clippy の存在とバージョン確認（rustfmt・clippy を含まない最小構成だと失敗する） |
| `mise run hk-validate` / `hk-check` | `hk.pkl` の検証 / リポジトリ全体の読み取り専用検査（cargo を伴わないステップだけ。cargo を伴う検査は `fmt-check` / `clippy` が担当する） |
| `mise run fnox-check` | `fnox list`（定義の一覧確認。秘密値の取得・表示はしない） |
| `mise run pitchfork-check` | pitchfork のバージョン確認と、`pitchfork.toml` に `agent-watch` が定義されていることの確認 |
| `mise run deny-check` | `cargo deny check`（依存クレートの検査。`verify-static` に含まれる。設定は `deny.toml`（ゲート）と `deny-warn.toml`（unmaintained / unsound の警告表示）） |
| `mise run shellcheck` | `shellcheck scripts/demo-check/*.sh`（demo-check 一式の静的検査。指摘 0 件で通る。`verify-static` に含まれる） |
| `mise run demo-check` | デモ受入前の 4 段階検査（`scripts/demo-check/run.sh`。受入は引数なしの全段階実行だけ有効。canary 段階は成果物が無いと未検証で失敗するため、製品をビルドしたあとに実行する） |
| `mise run demo-check-selftest` | demo-check の自己テスト（`script-test` 経由で `verify-test` にも含まれる） |
| `mise run verify-lint` / `verify-test` / `verify-static` | 検査の 3 グループ（cargo でコンパイルするのは `verify-lint` と `verify-test` の 2 つだけ） |
| `mise run verify` | 上記 3 グループの一括確認（依存タスクは**並列**実行） |
| `mise exec -- hk fix` | 修正の適用（ワークツリー。ステージはしない） |

Cargo 系タスクはすべて `--locked` 付き。`Cargo.lock` が更新を要求する状態（依存追加後の未コミットなど）では失敗する。その場合は依存を意図した変更か確認してコミットすること。

## git hook の挙動

- フックはリポジトリローカルの Git 設定（`hook.hk-pre-commit.command` 等）へ書き込まれる。設定ベースフックには **Git 2.54 以上**が必要（hk 公式ドキュメントの要件）。この環境の Git 2.56.0 では `git commit` からの発火を確認済み。
- `git commit`（pre-commit）: **品質ゲート**。fix を実行せず、check コマンドの失敗でコミットを阻止する（trailing whitespace / 最終改行 / typos によるスペル検査 / `cargo fmt --check` / `cargo clippy -D warnings` / `cargo check --locked`、いずれもワークスペース全体）。コードの自動書き換えは行わない。typos の検査範囲は `hk.pkl` の `exclude`（docs/**・assets/**）と `typos.toml` の `extend-exclude`（外部管理のベンダー込みスキル `.agents/skills`・`.claude/skills`）を除くリポジトリ全体。
- `git push`（pre-push）: `cargo test --workspace --locked` を実行する。
- 検査に失敗したら `mise run fmt` や `mise exec -- hk fix` で直してから再度コミットする。
- 一時的に無効化するには `HK=0` を付ける（例: `HK=0 git commit ...`）。hk の既定動作。
- 初回の `hk` 実行は `hk.pkl` の `amends` URL から GitHub 経由で Pkl パッケージを取得するため**ネットワークが必要**（2回目以降はキャッシュ）。

## CI（GitHub Actions）

### 規約：CI とローカルで同じ検査を行う

- CI とローカルで**一致させるのは検査の中身**であり、実行の単位ではない。検査は `mise.toml` のタスクに置き、CI でもローカルでも同じタスクを呼ぶ。ローカルでは `mise run verify` が 3 グループ（`verify-lint` / `verify-test` / `verify-static`）を並列実行し、CI では 1 グループを 1 ジョブで走らせる。どちらも同じタスクなので、検査の追加・変更は `mise.toml` の 1 箇所だけを直せば済む。
- 品質ゲートのチェック名は `mise run verify` のままにする（PR の必須チェック名として使う。`docs/agents/grokbot-runbook.md` と `.agents/skills/review-pr/SKILL.md` が参照する）。集約ジョブは `static` / `lint` / `test` の 3 ジョブの成否を集約し、1 つでも失敗すれば失敗する。
- cargo でコンパイルするグループは `verify-lint` と `verify-test` の 2 つだけに絞る。同じジョブで clippy と test を走らせると、同じ依存クレートを 2 回コンパイルすることになるため（cargo は target を共有する）。
- `cargo check` は clippy が型検査も行うため、`verify` には含めない。`cargo build` は `cargo test` が同じ成果物を作るため、`verify` には含めない。`check` / `build` のタスク自体は残してあり、単独で実行できる。

### ジョブ構成

| ジョブ | チェック名 | 実行する mise タスク | cargo |
|---|---|---|---|
| `static` | `static (hk / fnox / pitchfork)` | `verify-static`（hk / fnox / pitchfork / cargo-deny / shellcheck）＋ `mise exec -- jactionlint` | 使わない（cargo-deny は cargo metadata を読むだけでコンパイルしない） |
| `lint` | `lint (fmt / clippy)` | `verify-lint` | clippy |
| `test` | `test (cargo test)` | `verify-test` | test |
| `coverage` | `octocov (coverage diff)` | `coverage` タスク ＋ `mise exec -- octocov` | 計測ビルド |
| `pr-size` | `pr-size` | `scripts/pr-size.sh`（mise を使わない） | 使わない |
| `verify` | `mise run verify` | 上の 3 ジョブの集約 | — |

- `.github/workflows/verify.yml` が、`main` 向け Pull Request と手動実行（`workflow_dispatch`）で実行する。main への push でも実行し、その run が main のカバレッジレポートを artifact に保存する（カバレッジの増減表示はこの保存のあとの PR から出る）。
- クライアント（`yeslogic-fontconfig-sys` など）のビルドに必要な Linux システムパッケージは、cargo をビルドする `lint` / `test` / `coverage` の各ジョブで `apt-get` により `libfontconfig1-dev` を入れる（Ubuntu 24.04 では `libfontconfig-dev` を引く移行パッケージ。`fontconfig.pc` を提供する）。`static` ジョブは cargo を伴わないため要らない。`mise bootstrap`（apt:podman・sudo）は使わない。ツールのバージョンは `mise.toml` / `mise.lock` に従う。
- mise の導入は公式 Action（`jdx/mise-action`）で行う。`version: v2026.11.5`・`install_args: --locked`・`cache: false` を指定し、導入後にバージョンを検証している。mise-action のツールキャッシュは復元内容を `mise.lock` と突き合わせない（公式の注意）ため無効化し、ダウンロード用のキャッシュも足さない。Action 参照の sha 固定には GitHub 公式 lockfile（`.github/workflows/actions.lock`、担当が `gh actions-lock` で生成）を使う。workflow 側の参照は `actions/checkout@v4.2.2`、`jdx/mise-action@v2.4.4`、`actions/cache@v6.1.0`。lockfile 上のコミットはそれぞれ `11bd71901bbe5b1630ceea73d27597364c9af683`、`c37c93293d6b742fc901e1406b8f764f6fb19dac`、`55cc8345863c7cc4c66a329aec7e433d2d1c52a9` で、どれも公式タグが指すコミットと一致する。
- この workflow に秘密は不要。workflow 全体の既定の権限は `contents: read` のみで、`coverage` ジョブだけがジョブ単位で `pull-requests: write`（PR コメント投稿）と `actions: write`（octocov の base ブランチの過去レポート artifact の取得と、保存のたびの前レポート削除）を追加する。`fnox-check` は `fnox list`（定義一覧の確認）のみで、秘密値の取得・表示はしない。

### CI 固有の cargo 設定

workflow レベルの `env` で、ローカルの開発環境には影響させずに次を設定する。

| 変数 | 値 | 理由 |
|---|---|---|
| `CARGO_INCREMENTAL` | `0` | 1 回きりのビルドでは incremental は無駄なオーバーヘッドになる |
| `CARGO_PROFILE_DEV_DEBUG` | `0` | debuginfo の生成とリンクを省いてビルドを短縮する |
| `CARGO_PROFILE_TEST_DEBUG` | `0` | 同上（test プロファイル） |
| `MISE_RUST_MR_BOXINGTON` | `0` | CI では mbx を使わず素の cargo にする。mbx は `target` を `~/.cache/mbx` への symlink にするため、次のキャッシュに `target` を保存できない |
| `CARGO_TARGET_DIR` | `${{ github.workspace }}/target-ci` | ビルド成果物の置き場所。mbx の symlink 先が変わってもキャッシュが壊れないよう `target` とは別の固定の場所に置く |
| `CARGO_HTTP_MULTIPLEXING` | `false` | crates.io の取得で起きる既知の不具合（HTTP/2 の多重化による "Error in the HTTP2 framing layer"）を避ける |
| `CARGO_NET_RETRY` | `10` | crates.io の一時的な障害で取得が失敗したときの再試行回数を増やす（既定は 2） |

debuginfo を落としても検査の中身（clippy の警告もテストの結果も）は変わらない。

`CARGO_HTTP_MULTIPLEXING` と `CARGO_NET_RETRY` は `mise.toml` の `[env]` にも同じ値を置いてある。CI ではないコンテキスト（エディタや補助スクリプト）から cargo を直接呼ぶときにも効かせるためで、ローカルでも取得の失敗を減らす。

### 依存の取得

- `mise run fetch`（`cargo fetch --locked`）で依存クレートをビルド前に 1 回だけ取得する。取得の一時失敗（crates.io の障害など）が数分のビルドの途中で起きるのを避けるため。取得は cargo の設定（`CARGO_HTTP_MULTIPLEXING=false` / `CARGO_NET_RETRY`）で再試行する。

### ビルドキャッシュ

- `actions/cache` を 2 段に分ける。cargo のレジストリ（`~/.cargo/registry`・`~/.cargo/git`）は 3 つの cargo ジョブで共有し、ビルド成果物（`target-ci`）はジョブごとに分ける。
  | 対象 | キャッシュキー | 共有 |
  |---|---|---|
  | レジストリ | `<os>-cargo-registry-<hash(Cargo.lock)>` | 3 ジョブで共有 |
  | ビルド成果物 | `<os>-cargo-target-<job>-<hash(Cargo.lock, mise.toml)>` | ジョブごと |
- 依存やツールのバージョンが変わるとキーが変わるため、古いキャッシュを黙って使い回さない。`restore-keys` で同じジョブ（レジストリは全体）の直前のキャッシュにフォールバックする。
- clippy と test は成果物が別（clippy はメタデータ、test はコード生成）なので、ビルド成果物のキャッシュを共有しない。
- mise-action のツールキャッシュは有効化しない（復元内容を `mise.lock` と突き合わせないため。上の注意を参照）。
- ローカルの mbx はローカルキャッシュ（`~/.cache/mbx`）のみで完結する。S3 等の外部キャッシュサーバは使わない。
- キャッシュの容量を考える。リポジトリのキャッシュ上限は 10GB で、7 日以上アクセスされないエントリは失効する。`restore-keys` は一致するエントリのうち**最新**の 1 件だけを復元するため、各ジョブで生き残るのは最新の 1 世代だけである。実測で 1 世代あたりレジストリ約 250MB・ビルド成果物約 400MB（ジョブごと）。`Cargo.lock` が変わると 1 世代増えるので、依存が増えるペース（直近 30 日で 12 回）と合わせて数 GB の範囲に収まる。上限に達した場合 oldest から失効するため、CI は止まらない（キャッシュが効かなくなるだけ）。

### カバレッジ

- カバレッジの増減計測は同じ workflow の `coverage` ジョブで行う（品質ゲートとは独立）。cargo-llvm-cov（`--no-cfg-coverage` 付き。`cfg(coverage)` で nightly 専用 `#![feature]` を有効にする依存があると stable の E0554 になるため）で LCOV を `lcov.info` に出し、`.octocov.yml` の `comment:`（`if: is_pull_request`、`updatePrevious: true`）に従って octocov が PR へカバレッジコメントを投稿する。PR 本文への挿入（`body:`）は使わない。
- coverage ジョブは PostgreSQL のサービスコンテナ（nightly の test ジョブと同じ 16 系・digest 固定）に `PELWAY_DATABASE_URL`（スーパーユーザーの `postgres` ユーザー）で接続し、pelway-db の RLS テストを実行したうえで計測する（#161。#157 の `--exclude pelway-db` を解除）。ロール（`pelway_app`・`pelway_admin`・`pelway_audit`）はマイグレーションが作るため、ジョブでのロール作成ステップは無い。RLS テストの失敗は coverage ジョブに閉じ、品質ゲート（`static` / `lint` / `test`）の必須チェックには影響しない。
- cargo-llvm-cov は `mise.toml` の `[tools]` で `"aqua:taiki-e/cargo-llvm-cov" = "0.6.17"` と宣言し、`mise.lock` が URL と checksum で取得物を固定する（CI は `mise run coverage`。mise.toml のタスクが cargo-llvm-cov を呼ぶ）。ソースから `cargo install` するとビルドに 45 秒ほどかかるため、事前ビルド済みのバイナリを使う。
- octocov も `mise.toml` の `[tools]` で `"aqua:k1LoW/octocov" = "0.83.5"` と宣言し、`mise.lock` が URL と checksum で取得物を固定する。CI での実行は公式の `k1LoW/octocov-action`（actions.lock で sha 固定）経由で行う。レポートの artifact 保存に必要な `ACTIONS_RUNTIME_TOKEN` は `run:` ステップには渡されず action のコンテキストでしか取れないためです（octocov 本体は action の `version` + `checksum` 入力で v0.83.5 に固定。checksum は `mise.lock` が固定するのと同一のリリースアセット `octocov_v0.83.5_linux_amd64.tar.gz` の sha256）。レポートの保存と比較は `.octocov.yml` の `report.datastores` / `diff.datastores`（GitHub Actions の artifact のみ）で行い、PR コメントに base ブランチ（main）の前回レポートとの増減を表示する。増減は main でレポートが保存されたあとの PR から出る（それまではカバレッジ値と patch coverage のみ）。
- rust にはカバレッジ計測用の `llvm-tools-preview` 成分を `mise.toml` の `components` で宣言する。`toolchain-check` が sysroot 上の `llvm-profdata` / `llvm-cov` の存在を確認し、欠けていれば失敗する（暗黙の rustup 自動ダウンロードには頼らない）。`coverage` ジョブは `mise install` のあと、この検査を先に実行する。

### その他のジョブ

- workflow の静的検査は jactionlint（`mise.toml` で `2.5.0` をピン）を `static` ジョブの mise-action のあとに実行する。jactionlint は actions.lock による sha 固定を受け付ける。lock 生成後、暫定の inline ignore は撤去済み。
- `.github/workflows/mise-bump.yml` が、毎週月曜（UTC）と手動実行で、`mise upgrade --bump --no-global --minimum-release-age 7d --yes` により `[tools]` のバージョンを更新し、`chore/mise-tools-bump` ブランチのローリング PR を1本作る・更新する。dependabot は mise.toml を解釈できないため mise 自身に更新させる（rust の inline table を含む mise.toml と mise.lock を一貫して書き換える）。`--minimum-release-age 7d` で公開直後の版を除外する。mise 本体（`min_version`・mise-action の `version` ピン）は対象外。PR 作成は `GITHUB_TOKEN` + `gh` CLI で、専用 action は使わない。workflow レベルの `permissions` は空で、bump ジョブにだけ `contents: write` / `pull-requests: write` を付与する。GITHUB_TOKEN 製 PR の `pull_request` run は承認待ちになるため、`verify` が自動で走らない場合は PR の checks から `Approve workflows to run` を一度押す。
- GitHub 上での実際の run は未確認（作業ログ: `docs/worklogs/2026-11-10-platform-pr-ci-r1.md`）。

### nightly（main の総点検）

- `.github/workflows/nightly.yml` が、毎日 21:00 UTC（06:00 JST）と手動実行（`workflow_dispatch`）で main に対して実行する。PR の verify では回せない重い検査を対象にし、merge 後の main の劣化を翌朝の指揮役の巡回までに検出する（Issue #180）。
- ジョブ構成:

| ジョブ | 実行するもの | PR の verify との違い |
|---|---|---|
| `test` | `mise run verify-test`（`cargo test --workspace`＋script-test）を PostgreSQL ありで実行 | PR では `PELWAY_DATABASE_URL` 未設定のため飛ぶ DB 結合テスト（pelway-db の RLS、dev-queue-compare のキュー比較）が実際に走る |
| `hk-all` | `mise exec -- hk check --all`（cargo を伴うステップを含む全ステップ） | PR の hk-check タスクは cargo を伴わない 3 ステップ（typos / trailing_whitespace / newlines）だけ |
| `deps` | `mise run deny-check`（cargo-deny） | コードが変わらなくても新規 advisory で落ちうるため、main に対して毎晩確かめ直す |
| `docs-v07` | `python3 docs/technical/v0.7/verification/verify.py` | PR では実行していない文書検証 |

- fmt / clippy / fnox / pitchfork / jactionlint 等は PR の verify が各 PR で検査済みのため nightly では走らせない。
- `test` ジョブの PostgreSQL は `postgres:16-alpine` を digest 固定した service コンテナ（`PELWAY_DATABASE_URL` はスーパーユーザーの `postgres` ユーザー）。scratch database はテストごとに作られ、終了時に消える。
- v0.7 の verify.py は、原本（`/workspace/pelway-technical-docs`）と凍結 ZIP が原本作成環境にだけ存在するため、それらを `not-run` として結果 JSON に記録し、リポジトリ内で確かめられる検査（コピーの hash・相対リンク・節対応・アンカー・UI 文言）だけで合否を判定する。
- 通知は `notify-grokbot.yml` の `workflow_run`（`workflows` に `nightly` を登録）が担う。成功は `digest`、失敗は `immediate` で Grok Bot に届き、`run_url` から失敗ジョブを確認できる。
- 負荷試験の実行と収集するメトリクスの方針は [負荷試験とメトリクス](load-testing.md) に置く。nightly への組み込みは基盤の実装時に判断する。

## GitHub 通知（Grok Bot webhook・notify-grokbot）

`.github/workflows/notify-grokbot.yml` が、Grok Bot が動く必要のあるイベント（PR のクローズ（マージを含む）、レビューの投稿、verify・nightly workflow の完了（`workflow_run`））と手動実行（`workflow_dispatch`）を Grok Bot の webhook に POST する。エージェントのスレッドの状態は見張り役 `agent-watch` が別に送る（「pitchfork について」参照）。送るのはメタデータのみで、PR タイトル・本文、コメント本文、レビュー本文、ラベル、メールアドレス等は送らない。

- Secret（リポジトリの Settings → Secrets and variables → Actions → New repository secret で登録。値はドキュメント・コードに置かない）:
  - `GROKBOT_WEBHOOK_URL` — webhook の URL
  - `GROKBOT_WEBHOOK_AUTH` — `Authorization` ヘッダーの値全体
- 送る JSON のキーは 14 個固定: `schema`（`"pelway.github-notify/v2"`）, `urgency`（`digest` は workflow_run の成功（verify・nightly）と Approve のレビュー、それ以外は `immediate`）, `repo`, `event`, `action`, `pr_number`, `head_branch`, `head_sha`, `merged`, `conclusion`, `workflow_name`, `run_url`, `actor`, `occurred_at`。値の無いものは `null`。
- 送らない条件（通知をスキップし、ジョブは成功で終わる。CI は落ちない）:
  - 上記 Secret のどちらかが未設定のとき
  - fork 由来のイベント（PR 系は head リポジトリ、workflow_run は head_repository の full_name がこのリポジトリと違うとき）
  - レビュー（pull_request_review）で、投稿者の `author_association` が `OWNER` / `MEMBER` / `COLLABORATOR` のいずれでもないとき（ボット例外なし。レビュー担当の GitHub App の結果は `agent-watch` がスレッドの報告行として送る）
- 対象外: PR の会話欄に書いた普通のコメント（issue_comment）は通知しない。fork から来た PR へのコメントを安全に見分けられないため。あとで足す場合は、`pull-requests: read` で PR を調べて fork を除く形にし、別の PR で行う。
- 手動実行（workflow_dispatch）は `event: "test"` のテスト通知を 1 通送る。送信の失敗（タイムアウト・HTTP エラー）で CI は落ちない。

## レビュー担当の Codex（codex_review）

AI 開発チームのレビュースレッドは、T3 の Codex インスタンス `codex_review` で動かす（`docs/superpowers/specs/agent-team.md` の「レビュー担当の権限」）。このマシンでの設定は次の 2 つ。

| 場所 | 設定 |
|---|---|
| `~/.t3/userdata/settings.json` の `providerInstances.codex_review` | `driver: codex`、`config.launchArgs: --enable network_proxy -c default_permissions=review` |
| `~/.codex/config.toml` の `[permissions.review]` | ファイルは読み取り（`/tmp` だけ書ける）、ネットワークは `api.github.com` と `github.com` だけ（SOCKS は使わない） |
| `/etc/codex/requirements.toml`（#449 で追加。hatai が root で作る） | `[permissions.filesystem]` の `deny_read` に秘密パス 4 件（`~/.config/fnox`・`~/.config/pelway-agent`・`~/.ssh`・`~/.config/gh`）を列挙する。requirements 層の設定で、ユーザーの `config.toml` に書いても効かない。このマシン（単一ユーザー）の全 codex セッションに適用される |

`deny_read` は sandbox ポリシー（FileSystemSandboxPolicy）に Deny エントリとして渡り、sandboxed なコマンドから当該パスが読めなくなる（codex 0.169.0 の `codex-rs/config/src/filesystem_constraints.rs` と `requirements_layers/permissions.rs` による。パスは絶対パスで書く）。

ネットワークの制限が効くのは、スレッドの権限が `auto-accept-edits` のとき（コマンドが sandbox の中で動くとき）だけ。`full-access` のスレッドでは効かない。

`network_proxy` は Codex の試験的な機能で、`--enable network_proxy` が無いと権限プロファイルのネットワーク制限が効かない。Codex を更新したら `mise run codex-review-check` で、GitHub にはつながり、それ以外にはつながらないことを確かめる。

gh 2.91 以降はテレメトリ（使用状況の送信）が初期設定で有効で、実行のごく一部で `cafe.github.com` へ送信する。許可先に無いため sandbox が通信を止め、gh の実行ごと失敗する。`scripts/agent-gh` は空の `GH_CONFIG_DIR` で gh を動かすので `gh config set telemetry disabled` は効かず、代わりにどのロールでも `GH_TELEMETRY=false` と `DO_NOT_TRACK=1` を渡して送信を止める（#527。許可先は増やさない）。

### レビュー委託（delegate_task）での確認

通常の PR のレビューは、実装担当のスレッドが `delegate_task` で `codex_review` の子タスクとして委託する（`docs/superpowers/specs/agent-team.md` の「レビュー担当の権限」）。委託の子がスレッドと同じ制限（権限プロファイル `review`、ネットワークは `api.github.com` と `github.com` だけ）を継承するかを、実 PR での初回委託により確かめる。初回の委託の指示には、次の確認を報告に含めるよう足す。記録されるまでは、2 回目以降の委託によるレビューを行わない。

- `scripts/agent-gh review api /rate_limit` が成功する（GitHub にはつながる）
- `curl -sS https://example.com` が失敗する（GitHub 以外にはつながらない）
- worktree の外への書き込みと `git switch` が拒否される
- `scripts/agent-gh review pr review` でレビューを投稿できる

| 項目 | 結果 | 実施日 |
|---|---|---|
| 実施日（初回委託を行った PR） | PR #275（round 1。`checks pending` による再委託 1 回は回数に数えていない） | 2026-11-14 |
| ネットワーク制限の結果（GitHub にはつながり、それ以外にはつながらないか） | 合格（`scripts/agent-gh review api /rate_limit` 成功、`curl -sS https://example.com` 拒否） | 2026-11-14 |
| 書き込み・checkout の制限の結果 | 合格（worktree の外への書き込み拒否、`git switch` 拒否） | 2026-11-14 |
| レビュー投稿の結果（`agent-gh review pr review` が通ったか） | 合格（PR #275 に `pelway-review` 名義で Approve を投稿、`Reviewed:` の sha が head と一致） | 2026-11-14 |

`claude_review`・Devin CLI をレビュー委託で使うときも、それぞれの確認手順に加えてこの確認を済ませる。

## レビュー担当の Claude（claude_review）

高リスクの PR と抜き取りのレビュースレッドは、T3 の Claude インスタンス `claude_review` で動かす（`docs/superpowers/specs/agent-team.md` の「レビュー担当の権限」）。Codex のレビュー担当と同じ条件にそろえる。

| 場所 | 設定 |
|---|---|
| `~/.t3/userdata/settings.json` の `providerInstances.claude_review` | `driver: claudeAgent`、`config` の `homePath` に `/home/hatai/.claude-review`（UI のフィールド名は「CLAUDE_CONFIG_DIR path」。子プロセスの環境変数 `CLAUDE_CONFIG_DIR` になる。`config.environment` は claudeAgent の config スキーマに無いキーで無視される） |
| `~/.claude-review/settings.json` | sandbox を有効にし、ネットワークの許可先を `api.github.com` と `github.com` だけにする。使えるコマンドは読み取り系の `git`（`git show`・`git diff`・`git log`・`git grep`）と、`scripts/agent-gh review` を使う gh サブコマンド単位の形（`api`・`issue view`・`pr view`・`pr diff`・`pr checks`・`pr review`）に絞る（`scripts/agent-gh review *` のワイルドカードは使わない。`agent-gh` が引数を `git`/`gh` にそのまま渡すため、ワイルドカードは `auth token`・`repo clone`・`git` 全般などあらゆる形を許してしまう）。ファイルの書き込みを拒否する。deny は念のためラッパー経由の形でも書く（`git *`・`pr checkout *`・`alias *`・`auth *`・`repo *`・`gist *`）。#449 ではさらに `permissions.deny` に秘密パスの `Read(...)` 4 件（`~/.config/fnox/**`・`~/.config/pelway-agent/**`・`~/.ssh/**`・`~/.config/gh/**`）を足す。Claude Code 2.1.x は `Read(...)` deny を `sandbox.filesystem.denyRead` にマージし、sandboxed なコマンドにはファイルシステム層（bwrap）でも隠す |
| socat | Claude Code の sandbox（Linux）が使う。`mise.toml` の `conda:socat` で `mise install`（`mise bootstrap`）が入れる。apt と sudo は要らない |
| ログイン | `CLAUDE_CONFIG_DIR=~/.claude-review claude` で一度ログインする（人間が行う） |

運用に入る前に、`auto-accept-edits` のスレッドで次を確かめる。確かめるまでは `claude_review` を使わない。

- `scripts/agent-gh review api /rate_limit` が成功する（GitHub にはつながる）
- `curl -sS https://example.com` が失敗する（GitHub 以外にはつながらない）
- `git switch` と、ファイルの書き込みが拒否される
- `agent-gh review` を経由する他の gh サブコマンドが拒否される: `scripts/agent-gh review git switch -c <probe>`・`scripts/agent-gh review repo clone <リポジトリ> <dir>`・`scripts/agent-gh review auth status` が permissions の deny で拒否される（許可は使用するサブコマンド単位に絞ってあり、deny もラッパー経由の形で二重に張ってある）
- 許可経路からの egress が遮断される: `scripts/agent-gh review api --hostname example.com /` が失敗する（sandbox のネットワーク許可先の実測。許可リスト内で GitHub 以外へ出られるコマンド形で試す）

### 確認手順の結果（記録欄）

確認手順を実施したら、結果を次の表に記録する。記録されるまで `claude_review` は使わない（command-team の保留）。記録は Issue #256 の受け入れ条件の確認にも使う。

| 項目 | 結果 | 実施日 |
|---|---|---|
| 実施日（hatai によるレビュー用 Claude へのログインと、T3 への `claude_review` の追加を含む） | hatai によるレビュー用 Claude へのログインと `claude_review` の追加は済んでいる（`config.homePath` への修復は指揮役（代行）が hatai の分担を代わりに、許可の gh サブコマンド単位への絞り込みと deny への `git *`・`pr checkout *`・`alias *`・`auth *`・`repo *`・`gist *` の追加は hatai がそれぞれ適用した）。確認は `delegate_task`（target `claude_review`、`auto-accept-edits`、worktree）で実施。`CLAUDE_CONFIG_DIR` の継承は親側で子プロセスの environ（`CLAUDE_CONFIG_DIR=/home/hatai/.claude-review`）を確認して確かめた。許可・拒否のどちらのリストにも無い `echo "$CLAUDE_CONFIG_DIR"` は承認要求になり、承認を経て同じ値を出力した | 2026-11-14 |
| 使えるコマンドの制限の結果（読み取り系 `git` と `scripts/agent-gh review` に絞られているか。`git switch` とファイルの書き込みが拒否されたか） | `scripts/agent-gh review api /rate_limit`・`pr view`・`issue view` は承認なしで成功（許可は使用する gh サブコマンド単位に列挙）。`git switch -c perm-check-256` は permissions の deny で拒否。Write ツールは deny によりセッションで無効化され（`No such tool available`）、ファイルは作られなかった。ラッパー経由の形（`scripts/agent-gh review git switch -c <probe>`・`git -c 'alias.x=!<コマンド>' x`・`pr checkout <N>`・`repo clone <リポジトリ> <dir>`・`auth status`）は許可の列挙に入らず、deny（`git *`・`pr checkout *`・`alias *`・`auth *`・`repo *`・`gist *`）でも拒否される | 2026-11-14 |
| ネットワーク制限の結果（`api.github.com` と `github.com` だけにつながるか。`curl -sS https://example.com` が失敗したか） | `curl -sS https://example.com` は permissions の deny で拒否（実行自体されない）。GitHub API には到達した。egress の実測: 許可経路の `scripts/agent-gh review api --hostname example.com /` は実行されるが sandbox のプロキシで遮断（`host is not on the allow list`）。`scripts/agent-gh review git ls-remote https://example.com/` は deny で拒否 | 2026-11-14 |
| Codex 側（`codex_review`）との差（許可先・効く条件・コマンドの制限で異なる点。無ければ「差なし」） | 許可先は同じく GitHub のみ。遮断の仕組みが異なり、Codex は `network_proxy`＋権限プロファイル、Claude は `~/.claude-review/settings.json` の permissions（allow/deny）＋ sandbox。ファイルの書き込みは Write/Edit ツール自体が無効化される。許可・拒否のどちらにも載っていないコマンドは、T3 が `--permission-mode=acceptEdits` を渡すため承認要求になる（`defaultMode: dontAsk` より acceptEdits が優先される）。書き込みの遮断も異なり、Codex の権限プロファイルは `/tmp` 以外を読み取り専用にするが、Claude の OS sandbox は worktree と共有 `.git` への書き込みを許すため、permissions の deny が漏れる形（許可リスト経由）では書き込みが止まらない | 2026-11-14 |

残っている既知の経路は hatai の決裁で別 Issue で追う（PR #287 の round 4 で両レビュー担当が request-changes として指摘し、hatai が残リスクを受容）:

- `api -F <key>=@<file>`・`pr review --body-file <file>` で読み取り可能なローカルファイルを GitHub へ送信できる（#305）
- 読み取り系 `git` の `--output=<path>` での書き込みと `git grep -O<cmd>` での実行が、sandbox が worktree・共有 `.git` への書き込みを許すことと重なって残る（#306）

## レビュー担当の Devin CLI（devin_review）

通常の PR の Codex とのローテーションと、高リスク PR の 3 人目のレビュースレッドは、T3 の Devin CLI インスタンス `devin_review` で動かす（`docs/superpowers/specs/agent-team.md` の「レビュー担当の権限」）。モデルは `swe-2-high`。`devin_review` のモデルオプションは `mode` だけを受け付け、`thought_level`・`speed` は渡せない（effort の引き上げは行わない）。sandbox が効いているセッションでは権限モードは Autonomous 一択で、`mode` に `smart` などを渡しても内部では Autonomous の扱いになる。

Codex・Claude のレビュー担当と違い、Devin CLI にはネットワークを `api.github.com` と `github.com` だけに絞る権限設定が無い。代わりに Devin CLI 公式の OS レベル sandbox（Linux では bubblewrap）でレビュースレッドを包み、組み込みプロキシのドメイン許可リストで通信先を GitHub に限る。このマシンでの設定は次の 3 つ。

| 場所 | 設定 |
|---|---|
| `~/.t3/userdata/settings.json` の `providerInstances.devin_review` | `driver: acpRegistry`、`agentId: devin`、`config.commandPath: /home/hatai/.local/bin/devin-review`。acpRegistry は `<commandPath> acp` の形で起動し、registry の起動引数はそのまま渡る |
| `/home/hatai/.local/bin/devin-review` | `env DEVIN_SANDBOX=true devin --config /home/hatai/.config/devin-review/config.json "$@"` を exec する起動ラッパー。`devin acp` は `--sandbox` フラグを受け付けず、ACP サーバは `DEVIN_SANDBOX` 環境変数で sandbox を有効化する |
| `/home/hatai/.config/devin-review/config.json` | `devin_review` 専用のユーザー設定（`devin --config` で差し替え）。`sandbox.allowed_domains: ["**.github.com"]` で github.com とそのサブドメインだけを許可。`permissions.deny` に `edit`・`write`・`mcp__*`・`Exec(sudo)`・`Exec(rm)`・秘密パスの `Read(...)` を、`Fetch` は GitHub 以外を `ask` にして重ね防御する。#449 では `Read(...)` の列挙に `~/.config/fnox/**` と `~/.config/pelway-agent/**` を足す（`~/.ssh/**`・`~/.config/gh/**` は当初からあり）。deny に入れたパスは bwrap がコマンドから隠すため `test -r` も非 0 になる |

sandbox の前提ツールは `bwrap`（linuxbrew の `/home/linuxbrew/.linuxbrew/bin/bwrap`）と `socat`（`mise.toml` の `conda:socat`）で、どちらか欠けるとセッションは起動を拒否する（fail-closed）。sandbox 配下では exec が許可リスト経由の組み込みプロキシを通り、ワークスペース以外のファイルシステムは読み取り専用になり、`Read(...)` deny のパスはコマンドから隠される。

`Read(...)` deny に symlink を含めると、bwrap がリンク先にマウントできず exec が全滅する（例: `~/.aws` が `/mnt/c/...` への symlink の環境では `bwrap: Can't mount on symlink destination` になる）。deny に入れるパスは実体のあるものに限る。

運用に入る前に、`devin_review` のレビュースレッドを立て、次を確かめる。確かめるまではレビューに `devin_review` を使わない。

- `scripts/agent-gh review api /rate_limit` が成功する（GitHub にはつながる）
- `curl -sS https://example.com` が失敗する（GitHub 以外にはつながらない）
- 遮断する仕組みを迂回する任意コマンド（ワークスペース外への書き込みなど、Autonomous で自動許可されるが OS が拒否する実行）が拒否される
- PR のファイルを読み、`verdict:` を含む報告と `Reviewer:` の行を正しい形で出せる
- `scripts/agent-gh review pr review` でレビュー（Approve・Request changes）を投稿できる

### 確認手順の結果（記録欄）

確認手順を実施したら、結果を次の表に記録する。記録されるまで `devin_review` のレビュー担当は使わない（command-team の保留）。

| 項目 | 結果 | 実施日 |
|---|---|---|
| 実施日（レビュー用の Devin CLI スレッドの初回起動を含む） | `devin_review` のレビュースレッド（PR #283 への検証委託）が起動し、確認コマンドを実行できた | 2026-11-14 |
| 遮断する仕組み（名前と設定場所。無ければ「仕組みなし」で確認は打ち切り） | Devin CLI 公式の OS レベル sandbox（`DEVIN_SANDBOX=true`、Linux では bwrap + socat）＋ `~/.config/devin-review/config.json` の `sandbox.allowed_domains: ["**.github.com"]` と `permissions.deny`。起動ラッパー `/home/hatai/.local/bin/devin-review` | 2026-11-14 |
| ネットワーク制限の結果（`api.github.com` と `github.com` だけにつながるか。`curl -sS https://example.com` が失敗したか） | `curl -sS https://example.com` が exit 28（タイムアウト）で失敗。GitHub API には到達できた | 2026-11-14 |
| コマンドの制限の結果（遮断を迂回する任意コマンドが拒否されたか） | `touch $HOME/devin-outside-marker-$$` が `Read-only file system` で拒否（ファイル未作成） | 2026-11-14 |
| `scripts/agent-gh review` が通ること（`/rate_limit` の成功） | `scripts/agent-gh review api /rate_limit` 成功（rate limit の JSON を返却） | 2026-11-14 |
| レビューの形（報告の `verdict:`、`Reviewer:` の行、`agent-gh review pr review` の投稿） | `verdict: approve` と `Reviewer: agent=devin model=swe-2-high effort=high` の行を報告し、PR #283 に APPROVED として投稿済み（`pelway-review`） | 2026-11-14 |
| Codex・Claude 側との差（許可先・効く条件・コマンドの制限で異なる点。無ければ「差なし」） | 許可先は同じく GitHub のみ。遮断の効き方が異なり、Codex は `network_proxy`、Devin CLI は OS sandbox ＋ドメイン許可リスト＋ `permissions.deny`（直接編集・MCP・秘密パスを塞ぐ）。権限モードは Devin では Autonomous 一択 | 2026-11-14 |

## レビュー担当の秘密ファイルの遮断と鍵の分割（#449）

レビュー担当（`devin_review`・`claude_review`・`codex_review`）の sandbox から、秘密の置き場を読めないようにする。対象は 4 パス: `~/.config/fnox`（age 秘密鍵 `age.txt` と fnox のグローバル設定）・`~/.config/pelway-agent`（dev 用の fnox 設定。`PELWAY_DEV_*` と `PELWAY_LLM_API_KEY` が同じ age 鍵で暗号化されて同居していた）・`~/.ssh`・`~/.config/gh`。これらはすべて実体のあるディレクトリでなければならない（symlink だと devin の bwrap が exec を全滅させる）。

### hatai が手で適用する変更（リポジトリ外の設定。反映は hatai が行う）

**1. `~/.config/devin-review/config.json`** — `permissions.deny` の `Read(...)` に 2 件を追加（`~/.ssh/**`・`~/.config/gh/**` は既存）:

```json
"Read(~/.config/fnox/**)",
"Read(~/.config/pelway-agent/**)",
```

**2. `~/.claude-review/settings.json`** — `permissions.deny` に 4 件を追加（`Read(...)` deny は `sandbox.filesystem.denyRead` にマージされ、sandboxed なコマンドにはファイルシステム層でも隠される。`allowUnsandboxedCommands: false` が既に効いているため全 Bash コマンドが対象）:

```json
"Read(~/.config/fnox/**)",
"Read(~/.config/pelway-agent/**)",
"Read(~/.ssh/**)",
"Read(~/.config/gh/**)",
```

**3. `/etc/codex/requirements.toml`**（新規作成。root 権限が必要）— `deny_read` は requirements 層の設定で、ユーザーの `~/.codex/config.toml` に書いても効かない。このマシンの全 codex セッション（hatai の対話利用を含む）に適用される:

```toml
[permissions.filesystem]
deny_read = [
  "/home/hatai/.config/fnox",
  "/home/hatai/.config/pelway-agent",
  "/home/hatai/.ssh",
  "/home/hatai/.config/gh",
]
```

### 適用の順序（番号付き・橋渡し付き）

PR #462 のマージ前は、main 起点のレビュースレッドは**旧 `scripts/agent-gh`**（review ロールが `PELWAY_AGENT_FNOX_CONFIG`＝既定 `~/.config/pelway-agent/fnox.toml` を読む）を実行する。鍵の分割と deny を順序を誤って適用すると、旧 agent-gh は (a) 旧設定から `PELWAY_REVIEW_*` を削除した時点、(b) deny で `~/.config/pelway-agent` が読めなくなった時点のどちらでも exit 3（fail-closed）で止まる。そこで**橋渡し**（旧 agent-gh の `PELWAY_AGENT_FNOX_CONFIG` を review 用設定に向ける環境変数）を挟み、旧設定からの削除は最後に行う:

1. **鍵の分割（登録まで）**: 下の「鍵の分割」のコマンドのうち `fnox remove` のループ**以外**を実行する（review 用の鍵ペアと設定を作り、`PELWAY_REVIEW_*` を review 用設定へ登録する）。この時点で旧設定にも review 用設定にも `PELWAY_REVIEW_*` がある
2. **橋渡し**: レビュー用 CLI の起動時に `PELWAY_AGENT_FNOX_CONFIG=$HOME/.config/pelway-agent-review/fnox.toml` を渡す（`devin_review` は起動ラッパー `/home/hatai/.local/bin/devin-review` の `env` に足す。`claude_review`・`codex_review` は T3 側の起動設定かラッパーで同じ環境変数を渡す）。旧 agent-gh の review ロールが review 用設定を読むようになり、main 起点のレビュースレッドが動き続ける
3. **deny の追加**: 上の 3 担当の変更（devin・claude・`/etc/codex/requirements.toml`）を適用する。レビューの sandbox から `~/.config/pelway-agent` が読めなくなっても、橋渡しにより旧 agent-gh は review 用設定を読む
4. **実測と記録**: 3 担当のレビュー委託スレッドで 4 パスの `test -r`（すべて非 0）と `scripts/agent-gh review api /rate_limit`、review 用設定の `fnox list`（下の確かめ方）を実施し、下の記録欄に記録する
5. **PR #462 のマージ**: マージ後の `scripts/agent-gh` の review ロールは `PELWAY_AGENT_FNOX_CONFIG_REVIEW`（既定 `~/.config/pelway-agent-review/fnox.toml`）を直接読む
6. **旧設定からの削除と橋渡しの解除**: 下の「鍵の分割」の `fnox remove` のループを実行して旧設定から `PELWAY_REVIEW_*` を削除し、橋渡しの `PELWAY_AGENT_FNOX_CONFIG` を元に戻す

橋渡しを採って deny・削除の後倒しを採らない理由: 記録欄の実測（ステップ 4）は deny 適用の後・マージの前に行う必要がある（Issue #449 の受け入れ条件が「記録欄への記録と PR のマージまで」の順を要求するため）。deny をマージまで後倒しすると実測がマージ前に済ませられない。

### 鍵の分割（review 用の fnox 設定と age 鍵）

review 用の秘密（`PELWAY_REVIEW_*`）を dev とは別の fnox 設定・別の age 鍵ペアに移す。以降 `scripts/agent-gh review` は `PELWAY_AGENT_FNOX_CONFIG_REVIEW`（既定 `~/.config/pelway-agent-review/fnox.toml`）だけを読む。値は一切表示しない（`fnox get` の出力はパイプで次の `fnox set` に渡す）。**適用の順序（上の番号付き）のステップ 1 として、まず登録のループだけを実行する**:

```bash
mkdir -p ~/.config/pelway-agent-review
(umask 077 && age-keygen -o ~/.config/pelway-agent-review/age.txt)   # review 専用の鍵ペア（秘密鍵は表示しない）
review_recipient="$(age-keygen -y ~/.config/pelway-agent-review/age.txt)"
(umask 077 && cat >~/.config/pelway-agent-review/fnox.toml <<EOF
# レビュー担当の GitHub 名義（scripts/agent-gh review が読む）。値は review 専用の age 鍵で暗号化されている。
[providers.review_age]
type = "age"
key_file = "/home/hatai/.config/pelway-agent-review/age.txt"
recipients = ["$review_recipient"]

[secrets]
EOF
)
for name in PELWAY_REVIEW_APP_ID PELWAY_REVIEW_APP_INSTALLATION_ID PELWAY_REVIEW_APP_KEY; do
  fnox -c ~/.config/pelway-agent/fnox.toml get "$name" |
    fnox -c ~/.config/pelway-agent-review/fnox.toml set --provider review_age "$name" >/dev/null
done
```

旧設定からの削除は**ステップ 6（マージ後）**に実行する:

```bash
for name in PELWAY_REVIEW_APP_ID PELWAY_REVIEW_APP_INSTALLATION_ID PELWAY_REVIEW_APP_KEY; do
  fnox -c ~/.config/pelway-agent/fnox.toml remove "$name" >/dev/null
done
```

provider を `age` ではなく `review_age` という名前にするのは、fnox のグローバル設定（`~/.config/fnox/config.toml`）の `[providers.age]`（鍵ファイルの指定が無い）とのマージで鍵の指定が曖昧にならないため。`key_file` を明示するので、review の sandbox で `~/.config/fnox` が読めなくても復号できる。

**確かめ方（名前とパスだけ出す。値は出さない）**:

```bash
fnox -c ~/.config/pelway-agent-review/fnox.toml list   # PELWAY_REVIEW_* の 3 名だけが載る
fnox -c ~/.config/pelway-agent/fnox.toml list           # PELWAY_DEV_* と PELWAY_DEV_GIT_EMAIL だけが載る
scripts/agent-gh review api /rate_limit                 # review 用の設定だけでトークンが発行できる
```

**戻し方**: 橋渡しの `PELWAY_AGENT_FNOX_CONFIG` を元に戻し、review 用の設定から旧設定へ同じ形で移し戻してから、review 用の設定と鍵を消す:

```bash
for name in PELWAY_REVIEW_APP_ID PELWAY_REVIEW_APP_INSTALLATION_ID PELWAY_REVIEW_APP_KEY; do
  fnox -c ~/.config/pelway-agent-review/fnox.toml get "$name" |
    fnox -c ~/.config/pelway-agent/fnox.toml set --provider age "$name" >/dev/null
done
rm -f ~/.config/pelway-agent-review/fnox.toml ~/.config/pelway-agent-review/age.txt
```

（`scripts/setup/agent-accounts.sh` の新規セットアップは 2026-11-18 以降この分割構成で作る。ウィザードの「旧設定に残った review 秘密を移す」stage は登録と削除をまとめて行うため、PR #462 のマージ前に既存環境を移行するときは上の番号付きの手順（削除はステップ 6）を使う。）

### 確認手順の結果（記録欄）

3 担当それぞれのレビュー委託スレッドで、4 パスに `test -r <パス>; echo $?` を実行しすべて非 0 になったことと、塞いだ後も `scripts/agent-gh review api /rate_limit` が成功することを記録する。未実施の行が残る担当については、その担当の遮断を「反映済み・未実証」として扱う（`devin_review` は #449 の実測記録までレビュー担当に使わない hatai 決定のまま）。

| 項目 | 結果 | 実施日 |
|---|---|---|
| devin_review: 4 パスの `test -r`（すべて非 0 か） | 未実測 — devin_review は停止中のまま（hatai 決定 2026-11-18・#449 の実測が済むまで使用停止。実測を委託するスレッドを立てられない）。未実測のまま残すことは hatai の決定（2026-11-19 12:58 JST・例外）。実測は #479 で追う | — |
| claude_review: 4 パスの `test -r`（すべて非 0 か） | 未実測 — claude_review は env の橋渡し（`PELWAY_AGENT_FNOX_CONFIG`）が未導入のため実測できない（review 用の fnox 設定を読めずスレッドが動かない。PR #473 の Claude レビューも同じ理由で実施不可）。実測は #479 で追う | — |
| codex_review: 4 パスの `test -r`（すべて非 0 か） | 未実施 — 2026-11-19 10:33 JST の codex_review での実測（下記）は環境変数・`fnox list`・`gh extension list`・`/rate_limit`・`codex features list` の 5 項目で、4 パスの `test -r` は含んでいない | — |
| devin_review: `scripts/agent-gh review api /rate_limit` 成功 | 未実測（上記の devin_review 停止中と同じ理由。実測は #479 で追う） | — |
| claude_review: `scripts/agent-gh review api /rate_limit` 成功 | 未実測（上記の env の橋渡し未導入と同じ理由。実測は #479 で追う） | — |
| codex_review: `scripts/agent-gh review api /rate_limit` 成功 | 合格 — exit 0。起動引数変更後の codex_review スレッド（環境変数 `PELWAY_AGENT_FNOX_CONFIG`=set・`FNOX_CONFIG_DIR`=/nonexistent・`GH_CONFIG_DIR`=set）での実測。同じスレッドで `gh extension list` が exit 4・「To get started with GitHub CLI…」の認証なし応答（sandbox にログイン状態が残留しない形）も確認 | 2026-11-19 |
| review 用設定の `fnox list`（`PELWAY_REVIEW_*` だけか。`PELWAY_DEV_*`・`PELWAY_LLM_API_KEY` が載らないか） | 合格 — `fnox -c "$PELWAY_AGENT_FNOX_CONFIG" list` が exit 0 で、review 用設定には `PELWAY_REVIEW_APP_ID`・`PELWAY_REVIEW_APP_INSTALLATION_ID`・`PELWAY_REVIEW_APP_KEY` の 3 名だけが載る（sandbox 側は `FNOX_CONFIG_DIR`=/nonexistent でグローバル設定も載らない。ホスト側の list には fnox グローバル設定由来の `PELWAY_PENPOT_MCP_TOKEN` が追加で現れるが、review 用設定のファイルには含まれない）。旧設定 `~/.config/pelway-agent/fnox.toml` から `PELWAY_REVIEW_*` は削除済み（`PELWAY_DEV_*`・`PELWAY_DEV_GIT_EMAIL` のみ）で、`PELWAY_DEV_*`・`PELWAY_LLM_API_KEY` は review 用設定に載らない | 2026-11-19 |
| dev 用と review 用の age 鍵のパス（値は書かない）と、review の sandbox から dev 用の鍵が `test -r` 非 0 | 鍵のパス: dev=`~/.config/fnox/age.txt`・review=`~/.config/pelway-agent-review/age.txt`（別ファイル・両方作成済み）。review の sandbox から dev 用の鍵の `test -r` は未実測（今回の実測 5 項目に含まない） | — |

出典: codex_review での実測は指揮役の作業ログ `docs/worklogs/2026-11-19-conductor-substitute.md`（2026-11-19 10:33 JST・codex_review スレッド 1 run・読み取りのみ・値は測定スレッド側が伏せ済み）。PR #462 の round 3 の Codex レビュー（head `1b112fa`）が code-mode-host のエラーなしに 1 run で完走したことも同ログに記録済み（`agent-gh review` によるトークン発行からレビュー投稿まで通ることの追加の傍証）。旧設定の `fnox list` は 2026-11-19 に実装担当の worktree でも確認（名前のみ・値は出していない）。

## QA 担当の GitHub App（pelway-qa）

QA の合格コメントは QA 専用の名義で書く（#480 の決定）。そのための GitHub App `pelway-qa` で、権限は Issues: Read and write・Pull requests: Read-only・Metadata: Read-only のみ、webhook はオフ、hatai/Pelway にだけインストールする。Pull requests が Read-only なので Approve・マージは 403 になる。ラベル操作など権限上は通りうる操作も、`scripts/agent-gh qa` 側の許可リストが `comment` の 1 形しか通さないため、ラッパー経由では実行できない。

### 名義の作り方

`scripts/setup/agent-accounts.sh` は dev / review 専用で、この名義は対象に入っていない。次の手順を人間が手で行う（値はリポジトリに書かない）。

1. GitHub App `pelway-qa` を作る（webhook オフ。権限は上記の 3 つだけ）
2. Private key を発行する
3. hatai/Pelway にだけインストールし、installation ID を控える
4. `~/.config/pelway-agent-qa/fnox.toml`（provider `qa_age`）に、次の 3 つの名前で登録する
   - `PELWAY_QA_APP_ID`・`PELWAY_QA_INSTALLATION_ID`・`PELWAY_QA_PRIVATE_KEY`
   - dev / review 用の秘密（`PELWAY_DEV_APP_KEY`・`PELWAY_REVIEW_APP_KEY` など）と名前の付け方が違うので、`scripts/agent-gh` の `qa` ロールが読む名前どおりに登録する
5. レビュー役の拒否リスト（Codex の権限プロファイル、`~/.claude-review/settings.json`、`~/.config/devin-review/config.json`）に `~/.config/pelway-agent-qa/` を足し、レビュースレッドから秘密を読めなくする

### 使い方

`scripts/agent-gh qa comment <番号> --body-file <path|->` の 1 形だけが使える。中身は `POST /repos/hatai/Pelway/issues/<番号>/comments`（`gh issue comment`）だけで、それ以外の呼び出しはトークン発行の前に終了コード 2 で拒否される（許可リスト方式）。dev / review と違い `PELWAY_AGENT_FNOX_CONFIG` に既定値を持たないため、QA 用の設定ファイルを明示して呼ぶ。

```bash
PELWAY_AGENT_FNOX_CONFIG=~/.config/pelway-agent-qa/fnox.toml \
  scripts/agent-gh qa comment <番号> --body-file -
```

### 確かめ方

値を出さずに次で確かめる。

- `fnox -c ~/.config/pelway-agent-qa/fnox.toml list` で 3 つの名前が出る
- `PELWAY_AGENT_FNOX_CONFIG` を外して `scripts/agent-gh qa comment` を呼ぶと終了コード 3 で止まる
- 許可外の呼び出し（`scripts/agent-gh qa git push`、`scripts/agent-gh qa pr review 1 --approve` など）が終了コード 2 で拒否される。拒否がネットワークの前であることは `scripts/tests/agent-gh.test.sh` が確かめる（fnox・gh・git が呼ばれないことまで検査）
- コメントを実際に投稿したときの作者が `pelway-qa[bot]` であること、Approve・マージが 403 になることなどの実測は、Issue #483 の「実測」の節に沿ってマージ後に行う

### 戻し方

1. GitHub App の設定から hatai/Pelway のインストールを解除し、App を削除する
2. `~/.config/pelway-agent-qa/` を消す
3. レビュー役の拒否リストから `~/.config/pelway-agent-qa/` の項目を外す

## 実装担当の sandbox（agent-sandbox）

実装スレッド（T3 の worktree スレッドで動く opencode と Devin CLI）は、Issue #151 の対応として OS レベルの sandbox の中で動かす。`scripts/agent-sandbox/` に方式を同梱し、`scripts/setup/agent-sandbox.sh` でホストへ導入する。

### 方式

遮断は bubblewrap（`bwrap`）による mount 名前空間で行う。エージェント側の権限設定（T3 の runtimeMode・各 CLI の permissions）は補助層で、境界は mount 側が担う。

| 場所 | 設定 |
|---|---|
| `scripts/agent-sandbox/sandbox-shell` | 共通の bwrap ラッパー。コマンドを「worktree と必要なキャッシュだけ書き込み可」の名前空間で実行する。`-c` 形式（シェル）と `--wrap` 形式（プロセスごと包む）の 2 つの呼び方がある |
| `scripts/agent-sandbox/opencode-plugin/index.js` | opencode のグローバルプラグイン（V2 API、`id: pelway-sandbox`）。`~/.t3/worktrees/` 配下のセッションだけに適用され、(a) shell フックで全コマンドを `sandbox-shell` 経由にする、(b) ファイルツール（write/edit 等）の worktree 外への書き込みと秘匿パスの読み取りを拒否する（bwrap を通らない経路のため）。検査は実体パス（symlink 解決後。新規ファイルは最深の既存祖先まで解決）に対して行い、worktree 内 symlink 越しの脱走も防ぐ |
| `scripts/agent-sandbox/devin-impl` + `devin-config.json` + `.devin/hooks.v1.json` | `acpRegistry_devin` の `config.commandPath` に指定する起動ラッパーと専用設定。`devin acp` プロセス自体を `sandbox-shell --wrap` で包み、さらに PreToolUse フック（`sandbox-shell --hook`）が exec のコマンドをネストした sandbox に送る。devin プロセスは `credentials.toml` を読めるが、agent のコマンドは `~/.local/share/devin` がマスクされた namespace で動くため資格情報を分離できる。Devin 内蔵の `--sandbox` は使わない（後述） |
| リポジトリの `opencode.json`・`.opencode/`・`.devin/` | sandbox が read-only でマウントする同梱ファイル。sandbox 自身を無効化する設定（`plugins: ["-pelway-sandbox"]` や `.devin` の permissions/hooks の変更）を worktree 内から作れなくする |

bwrap が作る名前空間の要点:

- `/usr`・`/etc`・`/home` 等は read-only。`/` 全体をミラーする `--ro-bind /` は使わない（WSL の `/mnt/c` 等の submount まで写し、Windows 側ファイルが見えてしまうため、必要な top-level だけを明示的にマウントする）
- 書き込み可は、セッションの worktree・共有 git dir・`/tmp`・ツールキャッシュ（`~/.cargo`・`~/.rustup`・`~/.local/{share,state}/mise`・`~/.cache/{mise,pkl,fnox}`・`~/.pkl`・`~/.npm`・`~/.cache/agent-sandbox`）のみ。エージェント運用の状態ファイルで sandbox 内からも読み書きが必要なもの（巡回状態の補助記録 `patrol-state.json`、`pr-digest` の既出の指摘の記録）は、書き込み可の共有 git dir（`git rev-parse --git-common-dir`）内の `pelway/` に置く（git の管理対象外。`~/.local/state` は sandbox から書けないため、Issue #477）
- 共有 git dir（worktree の commondir）はコミットのため書き込み可だが、`config`・`config.worktree`・`hooks/` には read-only を重ねてマウントする（url rewrite や hook の仕込みを防ぐ）
- 秘匿パス（`~/.ssh`・`~/.gnupg`・`~/.docker`・`~/.config/gh`・`~/.config/opencode`・`~/.config/devin`・`~/.t3/userdata` 等）は tmpfs で隠すかマウントしない。no-new-privileges により `sudo`/`doas`/`pkexec` も実行できない
- ネットワークは遮断しない（ホストの経路をそのまま使う）。Issue #151 で hatai が「厳しくしすぎると調査が難しくなるので現時点では許可リストにしない」と指示したため
- シェル環境から `T3_ACP_MCP_*`・`T3CODE_*` を落とし、次を渡す: `MISE_RUST_MR_BOXINGTON=0`（mbx 無効化の意図の宣言。cargo コマンドラッパー生成済みの環境では実行時に効かず cargo は mbx を通るが、そのキャッシュは下の `XDG_CACHE_HOME` により sandbox 専用の書き込み可ディレクトリに移る。worktree の `target` symlink は実ディレクトリに戻される）、`PELWAY_AGENT_SANDBOX=1`（sandbox 内であることを示す目印。ホスト専用のテストがこれを見て skip する）、`XDG_CACHE_HOME`（`~/.cache/agent-sandbox`。mbx・hk 等の `~/.cache` 配下のキャッシュの向き先。sandbox 専用なのでホストの `~/.cache/mbx` は見せないままにできる。`$HOME/.cache` が read-only で作れないネスト実行では `/tmp/pelway-sbx-cache-<uid>` に退避する）、`GIT_CONFIG_COUNT` 系の `commit.gpgsign=false`（`~/.ssh` を隠すため SSH 署名は必ず失敗する。push 後に `scripts/agent-gh-resign` が署名を作り直すので問題ない）
- `/etc/resolv.conf` が symlink のとき（WSL では `/mnt/wsl/resolv.conf`）、実体を正本パスに read-only で重ねてマウントし、リンク切れによる DNS 不通を防ぐ。このとき `/mnt/wsl` が名前空間に見えるが、見えるのはリンク先のファイルだけで `/mnt/c` 等の Windows 側は見えない。実ファイルなら `/etc` の ro-bind に含まれるため手当ては要らない
- bwrap は広い bind を後から重ねると子の ro-bind が実質無効化されるため、マウントは「広い→狭い」の順で組み立てる（`scripts/tests/agent-sandbox.test.sh` で検証）

### Devin 内蔵 sandbox を使わない理由

`DEVIN_SANDBOX=true`（公式の bwrap+sandbox）を実装スレッドで検証したところ、`allowed_domains` を空にしても exec の通信がプロキシ経由で github.com 系のみになり、crates.io・npm・rustup の取得や一般サイトの調査ができなかった（`allowed_domains` 未指定・セクション自体の削除でも同じ。`*`/`**.**`/`*.**` 等のワイルドカードは全拒否になる）。これは hatai の「許可リストにしない」方針と両立しないため、Devin 側は内蔵 sandbox を使わず `sandbox-shell --wrap` で包む方式にした（wrap 方式ではネットワークは開放される）。その代わり、wrap 方式ではファイルツール経由の書き込みも同じ mount 境界に入るため、内蔵方式より境界が厳しい面もある。

### 導入手順

1. `bwrap` を導入する（このマシンでは linuxbrew の `/home/linuxbrew/.linuxbrew/bin/bwrap`）
2. `bash scripts/setup/agent-sandbox.sh` を実行する（プラグイン・wrapper・devin-impl・設定ファイルをホストに配置する）
3. `~/.t3/userdata/settings.json` の `providerInstances.acpRegistry_devin.config` に `"commandPath": "/home/hatai/.local/bin/devin-impl"` を追加し、T3 Code を再起動する
4. 次の確認手順を実施する

### 確認手順

- `mise run script-test`（`scripts/tests/agent-sandbox.test.sh`）で wrapper の性質を検査する。worktree 内の書き込み・コミット成功、ホーム・`/etc` への書き込み拒否、秘匿パスの遮蔽、`sudo` 失敗、git の `config`/`hooks` の read-only、sandbox 無効化パスの read-only、環境変数の除去、`--wrap` モード、プラグインの実体パス検査（symlink 越しの拒否）、`XDG_CACHE_HOME` の向き先と書き込み可、ホストの `~/.cache/mbx` の read-only、cargo が mbx を通っても lock 取得に失敗しないこと、sandbox 内の DNS 解決を確認する。sandbox 内の実装スレッドではこのテスト自体は実行できない（`PELWAY_AGENT_SANDBOX` を検出して理由を出して skip する）
- 導入後の初回の実装スレッド（opencode と Devin それぞれ）で、スレッド内から次を実行して結果を記録する
  - `scripts/agent-gh dev api /rate_limit` が成功する（GitHub と fnox/age 鍵経路が通る）
  - `touch "$HOME/sbx-marker"` が失敗する（worktree 外への書き込み拒否）
  - `sudo -n true` が失敗する
  - `curl -sS https://example.com` が**成功する**（ネットワークは開放であることを確認。遮断が意図通り働いていないときの検知にもなる）
  - `getent hosts api.github.com` が成功する（DNS が通る。`/etc/resolv.conf` が symlink のホストでは実体の ro-bind で解決できる）
  - `ls /mnt/c` が失敗する（`/mnt` 自体は resolv.conf のリンク先だけが見える。Windows 側は見えない）
- Devin 側は `acpRegistry_devin` のスレッドで同様に確認する（`devin-impl` wrapper 経由で起動されているかは、スレッド内で `findmnt` や `touch $HOME/...` の拒否で確かめられる）

### 確認手順の結果（記録欄）

| 項目 | 結果 | 実施日 |
|---|---|---|
| `sandbox-shell` の単体検査（`scripts/tests/agent-sandbox.test.sh`） | 合格。PR head `d3f0bf6dbfe492ba3ce29efc89917ba26f37ce7a` の版（#325 の新規検査を含む）を指揮役がホストで通し実行し `PASS: agent-sandbox`・exit=0（Issue #325 コメント 6907015476）。ホストの `/etc/resolv.conf` は `/mnt/wsl/resolv.conf` への symlink のため symlink-target の検査も走った。CI の `test (cargo test)` ジョブでは bwrap 不在で `SKIP: bwrap not installed` になるため、CI の pass は当該検査の実行を意味しない（run 43743202520・job 132900858476 のログで確認） | 2026-11-16 |
| opencode プラグインのフック検査（`ctx.location` が `~/.t3/worktrees` 配下でのみ shell 差し替え・ファイルツール拒否が発火することを node で直接検証） | 合格 | 2026-11-14 |
| Devin wrap 方式の実セッション検査（`sandbox-shell --wrap -- devin --config <devin-impl> -p` 内で実施） | `curl https://example.com` 成功（200）、`scripts/agent-gh dev api /rate_limit` 成功、`touch $HOME/...` が `Read-only file system` で拒否、worktree 書込成功 | 2026-11-14 |
| Devin 内蔵 sandbox の検査（比較のため） | `allowed_domains` 空・未指定で exec が github.com 系のみになり、crates.io・example.com タイムアウト（要件不適合のため不採用） | 2026-11-14 |
| opencode の実セッション（worktree で `opencode run`） | シェル経由の `$HOME` 書込が `Read-only file system`、`sudo -n true` 失敗、worktree 書込成功。write ツールの worktree 外書込が `pelway-sandbox: write: worktree 外への書き込みは禁止` で拒否 | 2026-11-14 |
| T3 の実スレッド（`acpRegistry_devin`）での確認 | 未実施（settings の `commandPath` 反映と T3 再起動後の初回スレッドで実施） | — |

### 既知の残件

- opencode のファイルツール遮断はプラグイン（プロセス内フック）で行い、OS 境界はシェル側の bwrap が担う。プラグイン自体の無効化は、worktree 内の `opencode.json[c]`・`.opencode/` を sandbox が read-only 化して塞いでいる
- `sandbox-shell` はマスク対象パスが存在しない場合に tmpfs でマウントするが、bwrap はマウントポイントをホスト側のファイルシステムに作成する。Pelway は対象ファイルをリポジトリに同梱して常に ro-bind にしている。他リポジトリの worktree では、初回の sandbox 実行で空の `opencode.json`・`.opencode/`・`.devin/` が作られる
- Devin の wrap 方式では `~/.local/share/devin/credentials.toml` は devin プロセス自身（outer namespace）から読める必要があるため、`agent-gh` が使う `~/.config/fnox/age.txt`・`~/.config/pelway-agent/fnox.toml` と同様、ネットワークが開放である以上、理論上の外部送信経路は残る。agent の exec コマンドは `.devin/hooks.v1.json` のフックでネストした namespace（`~/.local/share/devin` は tmpfs マスク）に入るため、コマンドからは資格情報に届かない。共有 git dir も全体が書き込み可（`config`・`hooks/` は read-only で保護）
## 指揮役スレッドのコンテキスト圧縮の検証（Issue #296）

指揮役（巡回を続ける長命スレッド）のコンテキスト圧縮の運用（skill `command-team` の「7. コンテキスト管理」）は、指揮役を動かす環境（プロバイダを含む）ごとに「圧縮操作の実行方法」「圧縮の検知方法」「使用率の取得方法」「交代手順」が検証済みであることでのみ有効化する。圧縮の仕組み（明示的な圧縮の可否・使用率の取得方法・自動圧縮の有無と検知）はプロバイダごとに異なるため、チームが使う全プロバイダ（opencode・Devin CLI・Codex・Claude）について 2026-11-14 に検証した結果を以下に置く。検証は各プロバイダの T3 スレッドでの報告と、T3 からの `/compact`・`/context` 送信の実測による。

共通の結果: エージェント自身（スレッドの中のエージェント）が自分のセッションを圧縮する専用のツール・スラッシュコマンドは、検証したどのプロバイダにも無かった。圧縮の実行は T3 からスレッドへ `t3_thread_send` で `/compact` を送る形（ホスト側の操作）で行い、実行されるとスレッドの activity に `compaction`（"Context compacted"）アイテムが記録される（Claude・Codex・Devin CLI で実測）。opencode では、これに加えてエージェント自身が OpenCode サーバーの HTTP API で圧縮を実行できる（下の表）。

### opencode（GLM、zai-coding-plan/glm-5.3、OpenCode server 2.3.0）

| 項目 | 結果 | 実施日 |
|---|---|---|
| 明示的な圧縮操作 | できる（エージェント自身も実行できる唯一のプロバイダ）。ツール一覧・スラッシュコマンドに compact 系は無いが、OpenCode サーバーの HTTP API `POST /api/session/{sessionID}/compact`（認証は Basic `opencode:<OPENCODE_PASSWORD>`。環境変数でサーバーから渡される）をエージェントが直接呼んで圧縮完了まで実証した（`reason=manual` の compaction メッセージが生成、サマリーは構造化 Markdown）。公式 CLI `opencode api` はこのサンドボックスでは読み取り専用 FS のログ書き込みで全コマンドが失敗するため、curl + Basic 認証が唯一の道だった。T3 からの `/compact` 送信の可否は未検証 | 2026-11-14 |
| コンテキスト使用率 | 取得できる（計算を伴う）。`GET /api/session/{id}/context` で各メッセージの `tokens`（input・output・reasoning・cache）が返る。glm-5.3 の context window（1,000,000）で割って使用率を計算（実測: 約 119,246 トークン ≈ 12%）。使用率%・残り容量をそのまま返すエンドポイントは無く、モデルの window 値（サーバー埋め込みの models.dev データ）からの自行計算が必要 | 2026-11-14 |
| 自動圧縮の有無と検知 | 仕様上存在する（`compaction.auto` はデフォルト true、`buffer` デフォルト 15% = window の 85% 到達で自動圧縮。auto の実際の発火は未観測）。検知はできる: SSE イベント `session.compaction.started` / `session.compaction.ended` が `reason: "auto" | "manual"` を区別して流れ、durable イベントログ（`GET /api/experimental/session/{id}/log`）とセッションのメッセージ一覧の `compaction` メッセージにも残るためポーリング検知も可能。サーバーログファイルには出力されない（実測 0 件）。イベント購読の `types` パラメータはカンマ区切り不可（1 リクエスト 1 型） | 2026-11-14 |

### Devin CLI（acpRegistry_devin、swe-2-high、Devin CLI 3000.12.43）

| 項目 | 結果 | 実施日 |
|---|---|---|
| 明示的な圧縮操作 | できる（実行主体はスレッドの外側）。エージェント用のツール一覧に compact 系は無く、起動フラグ `enable-devin-cli-swe2-self-compaction=false` でモデル主導の自己圧縮は無効。T3 からの `/compact` 送信で圧縮が実行され、スレッドの activity に `compaction` が記録される（エージェントが自分のスレッドへの `t3_thread_send` で注入できることも確認） | 2026-11-14 |
| コンテキスト使用率 | 取得できる。T3 からの `/context` 送信に `72.0k/262.0k tokens (27.5%)` の形（System prompt・Tools・Messages・Free の内訳付き）で応答が返る。`~/.local/share/devin/cli/sessions.db` の `message_nodes.metadata`（`num_tokens_preceding`）からもトークン数の絶対値は読める | 2026-11-14 |
| 自動圧縮の有無と検知 | 自動圧縮は有効（起動フラグ `enable-devin-cli-cloud-compaction-schedule=true`、設定 `agent.compaction_threshold_tokens` で閾値を調整可）。エージェントの会話内では検知できない（changelog に「Automatic context compaction is no longer surfaced in the scrollback or the ACP conversation view」と明記）。環境側では検知できる: ライフサイクルフック `PreCompact` / `PostCompact` / `PostCompaction`（manual/auto を区別、`PostCompaction` は summary を受け取る）、OTel イベント `compaction`、sessions.db の `summarized_from`、`~/.local/share/devin/cli/logs/` | 2026-11-14 |

### Codex（codex_review、gpt-6-luna）

| 項目 | 結果 | 実施日 |
|---|---|---|
| 明示的な圧縮操作 | できる（実行主体はスレッドの外側）。エージェント用のツール一覧に compact 系は無く、`codex debug` 等の CLI にも現在の T3 スレッドを圧縮する操作は無い。T3 からの `/compact` 送信で圧縮が実行され、スレッドの activity に `compaction` が記録される | 2026-11-14 |
| コンテキスト使用率 | 取得できない。ツール一覧にも `codex debug --help` / `codex app-server --help` / `codex features` にも、セッションの使用トークン数・上限・残量を返す機能は見当たらない（`get_goal` のトークン情報は Goal 予算でありセッションの使用量ではない） | 2026-11-14 |
| 自動圧縮の有無と検知 | 不明。auto-compact を制御・観測する機能や圧縮通知・イベント・ログを読む機能がセッション内に無く、圧縮を発生させる試験も未実施 | 2026-11-14 |

### Claude（claude_review、claude-opus-5-5、Claude Code 2.1.325）

| 項目 | 結果 | 実施日 |
|---|---|---|
| 明示的な圧縮操作 | できる（実行主体はスレッドの外側）。エージェント自身は `/compact` を実行できない（ビルトイン CLI コマンドで、Skill ツールでは "compact is a built-in CLI command, not a skill" と拒否される）。T3 からの `/compact` 送信で圧縮が実行され、スレッドの activity に `compaction`、セッションのトランスクリプトに `compact_boundary`（`compactMetadata`: `trigger: manual`、`preTokens`）が記録される | 2026-11-14 |
| コンテキスト使用率 | 直接は取得できない。T3 のスレッド読み取り（`t3_thread_read`）にコンテキスト使用状況は現れない。セッションのトランスクリプト（`~/.claude-review/projects/<作業パス>/<SESSION_ID>.jsonl`）の `usage`（`input_tokens` + `cache_creation_input_tokens` + `cache_read_input_tokens`）からトークン数は間接的に読めるが、上限値はトランスクリプトに記録されないため使用率の計算にはモデルの上限を外から与える必要があり、値も直前の API 呼び出し時点の 1 ターン遅れになる | 2026-11-14 |
| 自動圧縮の有無と検知 | 事後は検知できる。リアルタイムの通知は無い。トランスクリプトの `compact_boundary` 行（`compactMetadata.trigger` が manual/auto を記録）とスレッドの activity の `compaction` アイテムで、圧縮が走ったことを事後に確認できる。検証で確認できた実例は `trigger: manual` のみで、auto（自動圧縮）の実例は未確認 | 2026-11-14 |

### 交代手順

新指揮役スレッドへの恒久交代（skill `command-team` の「7. コンテキスト管理」の恒久交代）は未検証（全プロバイダ共通）。

### 運用への適用（プロバイダごと）

- 明示的な圧縮の実行方法: Devin CLI・Codex・Claude は T3 からの `/compact` 送信（検証済み）。opencode は OpenCode サーバーの HTTP API（エージェント自身・外部のどちらからでも）。圧縮後の「発生確認済み」の確認手段（activity の `compaction`、Claude ではトランスクリプトの `compact_boundary`、Devin CLI ではフック・sessions.db、opencode は SSE イベントとメッセージ一覧）もある
- 使用率が機械的に取得できるのは Devin CLI（`/context`）と opencode（context API の tokens 値と window からの計算）で、いずれも計算・応答形式にクセがある。Codex・Claude では代理指標（起算点から完了 Issue 5 件以上、または巡回 20 回以上）でトリガーを運用する
- 自動圧縮の検知: opencode は SSE イベント（auto/manual を区別）とメッセージ一覧で可能。Devin CLI は環境側の手段（フック・OTel・sessions.db）で可能（未適用）。Claude は事後検知の仕組みがあるが auto の実例は未確認。Codex は不明。いずれのプロバイダでも auto の実際の発火は未観測のため、検知の実効性は運用で確認する。検知方法が未検証のプロバイダでは、予期しない圧縮は検知できない前提で、巡回開始時の状態再取得と重要操作の直前の照合で対処する
- 交代手順は未検証のため、どのプロバイダでも交代を実行せず、人間確認へ進む

## シークレットの扱い（fnox）

- `fnox.toml` には**シークレットの名前と、秘密を含まないローカル開発用の既定値だけ**を置く。実際の秘密値（`value`）をこのファイルへ書かないこと。
- 既定は **fail-closed**（`if_missing = "error"`）: 値を解決できないシークレットがあれば `fnox exec` はエラーで停止する。値の登録を必須にしないものだけ、その秘密個別に `if_missing = "ignore"` を指定する。
- 現在の宣言:
  - `PELWAY_DATABASE_URL` — ローカル開発用の接続先（loopback・パスワードなしのテンプレート）。接続文字列は将来パスワード等を含めうるので、非秘密の既定値も fnox 側で管理する。
  - `PELWAY_LLM_API_KEY` — LLMゲートウェイの API キー（#330）。未登録・送信に使えない形のときは runner が台本モードで起動するため個別 ignore。値は環境変数1本で渡し、リポジトリ・CI には置かない。
- 秘密にならない既定値（例: `PELWAY_BIND_ADDR=127.0.0.1:0`）は mise の `[env]` 側に置く。
- 実行時の注入: `mise exec -- fnox exec -- <コマンド>` で環境変数として渡る。
- 本物の値を登録するときは、各自が暗号化プロバイダ（age 等）を設定した上で `fnox set <名前>` を使う。暗号文は `fnox set` が `fnox.toml` へ書き込む設計のため、**運用方針としてチームの合意を先に得ること**。
- 個人用の上書きは `fnox.local.toml`（`.gitignore` 済み）。秘密値を含みうるためコミットしない。

### デモの LLM 接続（#330）

- デモの runner（`pelway-runner`）は環境変数で LLM 接続を組み立てる（`DemoLlmConfig::from_env`。定数名は `crates/pelway-runner/src/demo.rs` の定数を参照）:
  - `PELWAY_LLM_API_KEY` — キー（fnox から注入。下記）。`PELWAY_LLM_KEY_ENV` で読む変数名だけ差し替えられる。
  - `PELWAY_LLM_PROVIDER` / `PELWAY_LLM_BASE_URL` / `PELWAY_LLM_CHAT_MODEL` / `PELWAY_LLM_EMBED_MODEL` / `PELWAY_LLM_RESPONSE_FORMAT` — 提供元の接続設定。既定は Z.ai 汎用エンドポイント + `glm-5.3-flash`（GLM-5.3 Flash。Z.ai 公式のモデルコード）。Coding Plan 向け endpoint（`/api/coding/paas/v4`）は自前アプリからの呼び出しが認められないため使わない。
  - `PELWAY_LLM_BASE_URL` が受ける接続先は Z.ai・OpenAI の 2 つの endpoint に限る（https 必須に加えて既知の提供元に限る。QA 点検 2026-11-17）。それ以外の接続先（ローカルのモック等）は `PELWAY_LLM_CUSTOM_BASE_URL` で明示したときだけ使い、その値が `PELWAY_LLM_BASE_URL` より優先して接続先になる。gateway 組み立て時に接続先（提供元名と base URL。キーは出さない。URL の userinfo とクエリの key/token などの値は `redact_url_for_log` で伏せる）を stderr に出す。
  - `PELWAY_LLM_TIMEOUT_SECS` — 1呼び出しのタイムアウト秒（既定 30）。
  - `PELWAY_LLM_BASIS` — LLM を呼ぶ根拠の目的 ID（P01〜P08。既定 `P06`。BYOK のため。hatai 決定 2026-11-16）。`DataStore::new_demo_seeded_with_basis` と `RunnerConfig.llm_basis` が受け取れる（コードには固定しない）。現状この値を起動側へ渡すのは tests/real_llm.rs のみで、`serve*` のデモは `new_demo_seeded`（P06 固定）を使う。
- 実キーでの実行: `mise exec -- fnox exec -- <デモ起動コマンド>`。キーが未設定・空白のみ・ヘッダーに載せられない文字を含むときは、落ちずに台本モード（固定の合成応答）で起動する。実提供元の呼び出しが失敗（タイムアウト・検証失敗の再試行尽き・拒否など）した呼び出しも「台本の応答で進めます」の進捗行を出して台本へ落ちる。
- 予備の提供元（GPT Luna。自動切替ではなく設定の差し替え。ADR 0008）: `PELWAY_LLM_PROVIDER=openai PELWAY_LLM_BASE_URL=https://api.openai.com/v1 PELWAY_LLM_CHAT_MODEL=gpt-6-luna PELWAY_LLM_RESPONSE_FORMAT=json_schema`（キーは同じ `PELWAY_LLM_API_KEY`）。
- `mise run demo-check` との関係: 許可接続先は 127.0.0.1 と ::1 のみで、実 LLM（z.ai）への接続は検査の対象外とする。提供元 endpoint の宣言は `crates/pelway-llm/src/config.rs` の1ファイルに集約し、そのファイルだけを external-refs 段階の除外とした（`scripts/demo-check/lib.sh` の除外コメント）。他の製品ソースに endpoint を書くと検査が落ちる。`DEMO_CHECK_ACCEPTANCE=1` の runtime loopback 検査は実 LLM 実行環境では成立しないため、実キーでの完走確認は demo-check の合格とは別に手動で記録する。

## 依存クレートの検査（cargo-deny）

- ツールは `mise.toml` の `[tools]` で `aqua:EmbarkStudios/cargo-deny`（0.20.2）を固定する（`mise.lock` が URL と checksum で取得物を固定。crates.io の同名パッケージを `cargo install` するとソースビルドになるため、公式の事前ビルド済みバイナリを使う）。
- 設定はリポジトリ直下の `deny.toml`（ゲート）と `deny-warn.toml`（unmaintained / unsound の警告表示）。実行は `mise run deny-check` で、`verify-static`（CI の `static` ジョブ）に含まれる。cargo をコンパイルに使わない（`cargo metadata` と `Cargo.lock` を読むだけ）。警告行への組み立て（advisory の ID の抜き出しを含む）は `scripts/deny-warn-extract.sh` が担い、`scripts/tests/deny-warn-extract.test.sh`（`script-test` 経由）が cargo-deny の実際の出力を写した固定サンプルで確かめる。
- 検査内容とポリシー（詳細は `deny.toml` のコメント）:
  - **advisories**: RustSec の脆弱性（vulnerability）が 1 件でもあれば失敗。yanked 版も失敗。unmaintained / unsound は検査を失敗させず、`deny-check` タスクの第2段（`deny-warn.toml` で all として検出し、結果を警告として表示して終了コード 0 で終える）で毎回警告に出す（0.20.2 の設定スキーマに警告レベルが無いための代替。既知の 5 件は `ignore` の台帳 + フォロー Issue #229 で管理）。
  - **licenses**: allow リストは現在の依存ツリーで実際に使われているライセンスだけ（棚卸し表: `docs/worklogs/issue-168.md`）。MPL-2.0 と slint の `LicenseRef-Slint-Royalty-free-2.0` は hatai の判断で許可（#168。GPL 系は許可しない）。ワークスペース内クレート（license 宣言なし・`publish = false`）は `[licenses.private]` で対象外。
  - **bans**: 同一クレートの複数版は警告（解消は上流の更新待ち）。ワイルドカード指定（`version = "*"`、version なしの path 依存）は失敗。ワークスペース内の path 依存には `version = "0.0.0"` を付ける。
  - **sources**: 取得元は crates.io のみ。git 依存・別のレジストリは失敗。
- **例外の足し方**: `deny.toml` の `ignore`（advisories）・allow リスト（licenses）に足すときは、理由と解消用のフォロー Issue の番号を必ず添える。`unused-ignored-advisory = "allow"` は `unmaintained = "none"` と組み合わせた台帳運用のため、エントリが未使用になっても警告が出ない。フォロー Issue を閉じたらエントリを削除する。ライセンスを追加するときは worklog の棚卸し表も更新する。
- CI の `static` ジョブのチェック名 `static (hk / fnox / pitchfork)` は workflow 側の表示名。実行内容の単一の管理元は `mise.toml` の `verify-static`（#168 では workflow 表示名の変更は対象外）。

## コンテナ（Docker を使わない）

方針: Docker Desktop・Docker デーモンには依存しない。Windows は WSL2 内の Linux ランタイム、macOS は Apple container を使う。

### WSL: rootless Podman

- **導入**: `mise.toml` の `[bootstrap.packages]`（`apt:podman`）で宣言する。適用されるのは `mise bootstrap` または `mise bootstrap packages apply` を明示的に実行したときだけ（日常の `mise install`・`mise run` では発動しない）。mise は root でない実行時に sudo を使う（mise 公式ドキュメントの apt ページの記載）。sudo のパスワード入力ができる利用者が対話的に実行する:

```bash
mise bootstrap packages apply --manager apt --dry-run   # 予定の確認（何も実行しない）
mise bootstrap packages apply --manager apt             # 適用（sudo のパスワード入力あり）
podman run --rm quay.io/podman/hello                    # hello 程度の smoke（quay.io からイメージを取得）
```

- **導入されるもの**: podman（Ubuntu noble のパッケージ。この環境では 4.9.3）と、rootless 実行に必要な依存一式（uidmap・crun・passt・slirp4netns・fuse-overlayfs・netavark・aardvark-dns・conmon・catatonit・buildah）。
- **動作状態の確認方法**: `podman info` で `rootless: true`・ociRuntime `crun`・networkBackend `netavark`（＋aardvark-dns）・cgroup v2 であることを確認できる。`mise bootstrap packages status` が導入状態を一覧する（sudo 不要）。
- **mise の `[tools]`（レジストリ）経由では導入できない**: mise レジストリの podman（`github:podman-container-tools/podman`）が Linux に配布するのは `podman-remote`（接続先の podman サービスまたは VM を前提とするクライアント単体）のみで、ローカルでコンテナを起動するエンジン本体を含まない。nerdctl も containerd デーモンと同じ rootless 部品群が必要で同じ結論。
- **既知の警告**: rootless 実行の初回に `WARN[0000] "/" is not a shared mount, this could cause issues or missing mounts with rootless containers` が出ることがある。hello の smoke には影響しない。bind mount を使う場面への影響は**未確認**。

### macOS（Apple container）— 未検証

この構成では検証できないため手順のみ記す。Apple silicon + macOS 26 以降が条件（[apple/container](https://github.com/apple/container) の公式 README に基づく）。**Intel Mac は Apple container・mbx とも対象外**のため、本構成の対象外（代替は v0.7 05章のとおり別審査の Podman machine）。

```bash
# GitHub Releases から署名済みインストーラを入手し、指示に従ってインストール（管理者パスワードが必要）
container system start
container run --rm alpine echo hello   # hello 程度の smoke
```

## mr boxington（mbx）について

- `mise.toml` の `rust = { version = "1.96.0", mr_boxington = true }` と `mr-boxington = "1.29.2"` の組で、mise 経由の cargo が mbx を通る。
- ラップの確認方法: `command -v cargo` が `~/.local/share/mise/command-wrappers/bin/cargo` に解決されること、ビルド出力に `mbx[cache]:` 行が出ること。
- キャッシュは `~/.cache/mbx`（ローカルのみ、自動GCあり。予算は `mbx settings ls` で確認）。`target` は `~/.cache/mbx/targets/...` への symlink になる（`.gitignore` 済み）。
- `mbx doctor` の setup 検査は「Cargo shim is not installed」と警告しうる。mise ネイティブ統合では shim を使わないためで、mbx 公式ドキュメント（Getting started）に説明がある。ラップの実態は上の確認方法で判断する。
- mbx 自体にはリモートキャッシュを設定しない（S3 等の外部キャッシュサーバを使わない）。CI の `actions/cache` は mbx とは別の仕組みで、CI ジョブでは mbx を有効にしない（`MISE_RUST_MR_BOXINGTON=0`）。CI 側の説明は「CI（GitHub Actions）」節にある。
- 実装担当の sandbox（「実装担当の sandbox（agent-sandbox）」参照）は `/home` を読み取り専用にし、書き込み可能なパスをツールキャッシュの列挙（`~/.cargo`・`~/.rustup`・`~/.cache/{mise,pkl,fnox}`・`~/.cache/agent-sandbox` など）に限る。`~/.cache/mbx` はハードリンクキャッシュ経由の書き換え経路として意図的に含めない。cargo コマンドラッパー生成済みの環境では `MISE_RUST_MR_BOXINGTON=0` は実行時に効かず cargo は mbx を通るため、sandbox は代わりに `XDG_CACHE_HOME` を `~/.cache/agent-sandbox` に向ける。mbx・hk 等のキャッシュはこの sandbox 専用ディレクトリに移り、lock 取得とキャッシュ書き込みが通る（Issue #325）。

## pitchfork について

- ツールは mise で 2.33.1 に固定する。
- `pitchfork.toml` には AI 開発チームの見張り役 `agent-watch`（`crates/pelway-agent-watch`）と、worktree の片付け `agent-worktree-prune` を定義する。`agent-watch` は T3 のスレッドの状態変化を Grok Bot の webhook に通知する常駐プロセスで、仕様は `docs/superpowers/specs/agent-team.md`「見張り役と通知」。
- `agent-worktree-prune` は cron（毎時 0 分）で `scripts/agent-worktree-prune --auto` を走らせる。消すのは `~/.t3/worktrees/Pelway` の下の、settle されたスレッドの（またはスレッドの無い）worktree のうち、main に取り込み済みか PR がマージ済み・クローズ済みのもの。未追跡のファイルは捨て、追跡ファイルに変更が残るものは残す。T3 に問い合わせられない回は何も消さない。消したあとに `git worktree prune`・`git fetch --prune origin`・`mbx gc` を走らせる。pitchfork の supervisor が動いていれば、`pitchfork start` しなくても cron で動く。supervisor は各 worktree の `pitchfork.toml` の cron も動かすが、`--auto` は main の checkout 以外では何もせずに終わり、同時に 2 つは走らない（`$XDG_STATE_HOME/pelway/agent-worktree-prune.lock`）。

  | コマンド | 内容 |
  |---|---|
  | `pitchfork supervisor status` | supervisor が動いているかの確認（定期実行の前提） |
  | `pitchfork status agent-worktree-prune` | 前回と次回の実行時刻 |
  | `pitchfork logs agent-worktree-prune` | ログ（消した・残した worktree と理由） |
  | `mise run agent-worktree-prune -- --auto` | 同じ処理をすぐ走らせる（main の checkout で実行する） |

- 起動・停止・ログ（リポジトリの直下で実行する）:

  | コマンド | 内容 |
  |---|---|
  | `mise run agent-watch-install` | 見張り役を `~/.local/bin/pelway-agent-watch` にインストールする。コードを変えたら実行し、`pitchfork restart agent-watch` で入れ替える |
  | `pitchfork start agent-watch` | 起動（異常終了したら 10 回まで再起動する） |
  | `pitchfork stop agent-watch` | 停止 |
  | `pitchfork status agent-watch` | 状態の確認 |
  | `pitchfork logs agent-watch` | ログ（送ったイベントと HTTP ステータス。秘密と返答の本文は出さない） |

- 状態は `$XDG_STATE_HOME/pelway/agent-watch.json`（未設定なら `~/.local/state/pelway/agent-watch.json`）に置く。初めて起動したときは既存のスレッドを記録するだけで、通知しない。送れなかった通知は状態の送信待ちの列に残り、次の周回で送り直す。
- 送らずに試すには `cargo run -p pelway-agent-watch -- --once --dry-run`（新しいイベントの payload を標準出力に出す。状態ファイルは読むだけで書き換えない）。
- v0.7 の05章が想定する起動単位（postgres・server・runner など）は、実装された時点で loopback 限定の定義を `pitchfork.toml` に足す。

## 既知の事項・未解決

1. **WSL の `/` shared mount 警告**: podman の rootless 実行時に出ることがある。smoke には影響しないが、bind mount を使う場面への影響は未確認。実使用時に検証する。
2. `docs/architecture-v0.7.md` の実行ビット（100755）は意図的にそのまま（凍結文書のため変更禁止）。
3. macOS（Apple container）の手順は未検証。Intel Mac は対象外。
4. `mise activate`（シェル統合）は `~/.bashrc` 等の変更が必要なため設定していない。必要なら利用者が各自で設定する（[シェル設定](https://mise.jdx.dev/shell-setup.html)）。それまでは `mise exec --` を使う。
