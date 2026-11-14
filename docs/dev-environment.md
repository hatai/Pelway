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
| `static` | `static (hk / fnox / pitchfork)` | `verify-static`（hk / fnox / pitchfork / cargo-deny）＋ `mise exec -- jactionlint` | 使わない（cargo-deny は cargo metadata を読むだけでコンパイルしない） |
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

ネットワークの制限が効くのは、スレッドの権限が `auto-accept-edits` のとき（コマンドが sandbox の中で動くとき）だけ。`full-access` のスレッドでは効かない。

`network_proxy` は Codex の試験的な機能で、`--enable network_proxy` が無いと権限プロファイルのネットワーク制限が効かない。Codex を更新したら `mise run codex-review-check` で、GitHub にはつながり、それ以外にはつながらないことを確かめる。

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
| `~/.t3/userdata/settings.json` の `providerInstances.claude_review` | `driver: claudeAgent`、`environment` に `CLAUDE_CONFIG_DIR=~/.claude-review`（指揮役・実装担当の Claude と設定を分ける） |
| `~/.claude-review/settings.json` | sandbox を有効にし、ネットワークの許可先を `api.github.com` と `github.com` だけにする。使えるコマンドを読み取り系の `git`（`git show`・`git diff`・`git log`・`git grep`）と `scripts/agent-gh review` に絞り、ファイルの書き込みを拒否する |
| socat | Claude Code の sandbox（Linux）が使う。`mise.toml` の `conda:socat` で `mise install`（`mise bootstrap`）が入れる。apt と sudo は要らない |
| ログイン | `CLAUDE_CONFIG_DIR=~/.claude-review claude` で一度ログインする（人間が行う） |

運用に入る前に、`auto-accept-edits` のスレッドで次を確かめる。確かめるまでは `claude_review` を使わない。

- `scripts/agent-gh review api /rate_limit` が成功する（GitHub にはつながる）
- `curl -sS https://example.com` が失敗する（GitHub 以外にはつながらない）
- `git switch` と、ファイルの書き込みが拒否される

### 確認手順の結果（記録欄）

確認手順を実施したら、結果を次の表に記録する。記録されるまで `claude_review` は使わない（command-team の保留）。記録は Issue #256 の受け入れ条件の確認にも使う。

| 項目 | 結果 | 実施日 |
|---|---|---|
| 実施日（hatai によるレビュー用 Claude へのログインと、T3 への `claude_review` の追加を含む） | 未実施 | — |
| 使えるコマンドの制限の結果（読み取り系 `git` と `scripts/agent-gh review` に絞られているか。`git switch` とファイルの書き込みが拒否されたか） | 未実施 | — |
| ネットワーク制限の結果（`api.github.com` と `github.com` だけにつながるか。`curl -sS https://example.com` が失敗したか） | 未実施 | — |
| Codex 側（`codex_review`）との差（許可先・効く条件・コマンドの制限で異なる点。無ければ「差なし」） | 未記入 | — |

## レビュー担当の Devin CLI（devin_review）

通常の PR の Codex とのローテーションと、高リスク PR の 3 人目のレビュースレッドは、T3 の Devin CLI インスタンス `devin_review` で動かす（`docs/superpowers/specs/agent-team.md` の「レビュー担当の権限」）。モデルは `swe-2-high`。`devin_review` のモデルオプションは `mode` だけを受け付け、`thought_level`・`speed` は渡せない（effort の引き上げは行わない）。sandbox が効いているセッションでは権限モードは Autonomous 一択で、`mode` に `smart` などを渡しても内部では Autonomous の扱いになる。

Codex・Claude のレビュー担当と違い、Devin CLI にはネットワークを `api.github.com` と `github.com` だけに絞る権限設定が無い。代わりに Devin CLI 公式の OS レベル sandbox（Linux では bubblewrap）でレビュースレッドを包み、組み込みプロキシのドメイン許可リストで通信先を GitHub に限る。このマシンでの設定は次の 3 つ。

| 場所 | 設定 |
|---|---|
| `~/.t3/userdata/settings.json` の `providerInstances.devin_review` | `driver: acpRegistry`、`agentId: devin`、`config.commandPath: /home/hatai/.local/bin/devin-review`。acpRegistry は `<commandPath> acp` の形で起動し、registry の起動引数はそのまま渡る |
| `/home/hatai/.local/bin/devin-review` | `env DEVIN_SANDBOX=true devin --config /home/hatai/.config/devin-review/config.json "$@"` を exec する起動ラッパー。`devin acp` は `--sandbox` フラグを受け付けず、ACP サーバは `DEVIN_SANDBOX` 環境変数で sandbox を有効化する |
| `/home/hatai/.config/devin-review/config.json` | `devin_review` 専用のユーザー設定（`devin --config` で差し替え）。`sandbox.allowed_domains: ["**.github.com"]` で github.com とそのサブドメインだけを許可。`permissions.deny` に `edit`・`write`・`mcp__*`・`Exec(sudo)`・`Exec(rm)`・秘密パスの `Read(...)` を、`Fetch` は GitHub 以外を `ask` にして重ね防御する |

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

## 指揮役スレッドのコンテキスト圧縮の検証（Issue #296）

指揮役（巡回を続ける長命スレッド）のコンテキスト圧縮の運用（skill `command-team` の「7. コンテキスト管理」）は、環境ごとに「圧縮操作の実行方法」「圧縮の検知方法」「使用率の取得方法」「交代手順」が検証済みの環境でのみ有効化する。T3 の Claude スレッド（`claude_review`、Claude Code 2.1.325、`CLAUDE_CONFIG_DIR=~/.claude-review`）について 2026-11-14 に検証した結果:

| 項目 | 結果 | 実施日 |
|---|---|---|
| 明示的な圧縮操作 | できる（実行主体はスレッドの外側）。エージェント自身は `/compact` を実行できない（ビルトイン CLI コマンドで、Skill ツールでは "compact is a built-in CLI command, not a skill" と拒否される）。T3 からのスレッド送信（`t3_thread_send`）で `/compact` を送ると圧縮が実行され、スレッドの activity に `compaction`（"Context compacted"）、セッションのトランスクリプトに `compact_boundary`（`compactMetadata`: `trigger: manual`、`preTokens`）が記録される | 2026-11-14 |
| コンテキスト使用率 | 直接は取得できない。T3 のスレッド読み取り（`t3_thread_read`）にコンテキスト使用状況は現れない。セッションのトランスクリプト（`~/.claude-review/projects/<作業パス>/<SESSION_ID>.jsonl`）の `usage`（`input_tokens` + `cache_creation_input_tokens` + `cache_read_input_tokens`）からトークン数は間接的に読めるが、上限値はトランスクリプトに記録されないため使用率の計算にはモデルの上限を外から与える必要があり、値も直前の API 呼び出し時点の 1 ターン遅れになる | 2026-11-14 |
| 自動圧縮の発生を検知 | 事後は検知できる。リアルタイムの通知は無い。トランスクリプトの `compact_boundary` 行（`compactMetadata.trigger` が manual/auto を記録）とスレッドの activity の `compaction` アイテムで、圧縮が走ったことを事後に確認できる。検証で確認できた実例は `trigger: manual` のみで、auto（自動圧縮）の実例は未確認 | 2026-11-14 |
| 交代手順（新指揮役スレッドへの恒久交代） | 未検証 | — |

この環境での運用への適用:

- 使用率が機械的に取得できないため、トリガーは代理指標（起算点から完了 Issue 5 件以上、または巡回 20 回以上）で運用する
- 圧縮操作の実行方法（`/compact` の送信）と圧縮後の「発生確認済み」の確認手段（`compact_boundary` の記録）は検証済みのため、計画的圧縮はこの環境で有効化できる
- 自動圧縮（auto）の検知は未確認のため、予期しない圧縮は検知できない前提で、巡回開始時の状態再取得と重要操作の直前の照合で対処する
- 交代手順は未検証のため、この環境では交代を実行せず、人間確認へ進む

## シークレットの扱い（fnox）

- `fnox.toml` には**シークレットの名前と、秘密を含まないローカル開発用の既定値だけ**を置く。実際の秘密値（`value`）をこのファイルへ書かないこと。
- 既定は **fail-closed**（`if_missing = "error"`）: 値を解決できないシークレットがあれば `fnox exec` はエラーで停止する。値の登録を必須にしないものだけ、その秘密個別に `if_missing = "ignore"` を指定する。
- 現在の宣言:
  - `PELWAY_DATABASE_URL` — ローカル開発用の接続先（loopback・パスワードなしのテンプレート）。接続文字列は将来パスワード等を含めうるので、非秘密の既定値も fnox 側で管理する。
  - `PELWAY_LLM_API_KEY` — 名前だけの宣言（値なし・当面未使用のため個別 ignore）。
- 秘密にならない既定値（例: `PELWAY_BIND_ADDR=127.0.0.1:0`）は mise の `[env]` 側に置く。
- 実行時の注入: `mise exec -- fnox exec -- <コマンド>` で環境変数として渡る。
- 本物の値を登録するときは、各自が暗号化プロバイダ（age 等）を設定した上で `fnox set <名前>` を使う。暗号文は `fnox set` が `fnox.toml` へ書き込む設計のため、**運用方針としてチームの合意を先に得ること**。
- 個人用の上書きは `fnox.local.toml`（`.gitignore` 済み）。秘密値を含みうるためコミットしない。

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

## pitchfork について

- ツールは mise で 2.33.1 に固定する。
- `pitchfork.toml` には AI 開発チームの見張り役 `agent-watch`（`crates/pelway-agent-watch`）だけを定義する。T3 のスレッドの状態変化を Grok Bot の webhook に通知する常駐プロセスで、仕様は `docs/superpowers/specs/agent-team.md`「見張り役と通知」。
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
