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
| [shellcheck](https://github.com/koalaman/shellcheck) | シェルスクリプトの静的検査 | 採用。`mise.toml` で 0.11.0 を固定し、`shellcheck` タスク（`verify-static`・CI の static ジョブ）が `scripts/` 以下で 1 行目が `#!` で始まり `bash` か `sh` を含むファイル全部を検査する。demo-check 一式（`scripts/demo-check/*.sh`）は severity 既定、それ以外は `--severity=warning`（info・style 級は対象外。Issue #265・#621） |

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

- `hk install --mise` は `.git/config` に `hook.hk-pre-commit.command`（`test "${HK:-1}" = "0" || mise x -- hk run pre-commit --from-hook`）を書く。pre-commit の実体はこの行で、`mise x` が `hk.pkl` を読む前に動く。コミット時の `mise lock` の一覧や rust の入れ直しの出力は `hk.pkl` ではなく、この `mise x` から出る（`git config --get hook.hk-pre-commit.command` で確かめられる）。
- 導入済みのものはスキップされる（宣言と現在の差分だけを適用）。このため `mise bootstrap` は何度実行してもよく、podman 導入済みの環境では packages 部分で sudo の入力を求められない。
- 各段階は上表のとおり単独のコマンドでも実行できる。
- `mise bootstrap` の tools 部分は `[tools]` に加えてグローバル設定のツールも確認対象にする（mise の通常動作）。

- `mise install` と `mise.lock` の生成は `mise.toml` の `[settings] lockfile = true` でこのプロジェクト自身が指定する（ユーザーのグローバル設定に依存しない）。`mise.lock` と `Cargo.lock` はコミット対象。
- rust のツール定義には `components = ["rustfmt", "clippy", "llvm-tools"]` を明示する（`llvm-tools` は CI のカバレッジ計測が必要とする）。成分名は rustup が `rustup component list --installed` に出す名前で書く。別名（`llvm-tools-preview`）で書くと、mise が導入済みと判定できず、`mise exec` のたびに rust を入れ直して `mise lock` の一覧を出す。成分の存在は `mise run toolchain-check` で検査する。
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
| `mise run toolchain-check` | rustc / cargo / rustfmt / clippy / llvm-tools の存在とバージョン確認（rustfmt・clippy を含まない最小構成だと失敗する）。加えて `mise.toml` の rust の `components` の名前が `rustup component list --installed` の名前（`-<target>` を除く）と一致するかを照合し、ずれた名前を出して失敗する |
| `mise run hk-validate` / `hk-check` | `hk.pkl` の検証 / リポジトリ全体の読み取り専用検査（cargo を伴わないステップだけ。cargo を伴う検査は `fmt-check` / `clippy` が担当する） |
| `mise run fnox-check` | `fnox list`（定義の一覧確認。秘密値の取得・表示はしない） |
| `mise run pitchfork-check` | pitchfork のバージョン確認と、`pitchfork.toml` に `agent-watch` が定義されていることの確認 |
| `mise run conductor-rebuild -- --from <旧指揮役のスレッド ID>` | 指揮役を立て直す（`docs/agents/conductor-context.md` の「恒久交代」）。hatai がホストの本体 checkout で実行する（worktree からは止まる） |
| `mise run deny-check` | `cargo deny check`（依存クレートの検査。`verify-static` に含まれる。設定は `deny.toml`（ゲート）と `deny-warn.toml`（unmaintained / unsound の警告表示）） |
| `mise run shellcheck` | `scripts/shellcheck.sh`（`scripts/` 以下の bash・sh のスクリプト全部の静的検査。`scripts/demo-check/*.sh` は severity 既定、それ以外は `--severity=warning`。指摘 0 件で通り、検査したファイルの数を出す。`verify-static` に含まれる） |
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
| `coverage` | `octocov (coverage diff)` | `coverage` タスク ＋ `k1LoW/octocov-action`（octocov 本体は v0.83.5 に固定） | 計測ビルド |
| `pr-size` | `pr-size` | `scripts/pr-size.sh`（mise を使わない） | 使わない |
| `verify` | `mise run verify` | 上の 3 ジョブの集約 | — |

- `.github/workflows/verify.yml` が、`main` 向け Pull Request と手動実行（`workflow_dispatch`）で実行する。main への push でも実行し、その run が main のカバレッジレポートを artifact に保存する（カバレッジの増減表示はこの保存のあとの PR から出る）。
- クライアント（`yeslogic-fontconfig-sys` など）のビルドに必要な Linux システムパッケージは、cargo をビルドする `lint` / `test` / `coverage` の各ジョブで `apt-get` により `libfontconfig1-dev` を入れる（Ubuntu 24.04 では `libfontconfig-dev` を引く移行パッケージ。`fontconfig.pc` を提供する）。`static` ジョブは cargo を伴わないため要らない。`mise bootstrap`（apt:podman・sudo）は使わない。ツールのバージョンは `mise.toml` / `mise.lock` に従う。
- mise の導入は公式 Action（`jdx/mise-action`）で行う。`version: v2026.11.5`・`install_args: --locked`・`cache: false` を指定し、導入後にバージョンを検証している。mise-action のツールキャッシュは復元内容を `mise.lock` と突き合わせない（公式の注意）ため無効化し、ダウンロード用のキャッシュも足さない。Action 参照の sha 固定には GitHub 公式 lockfile（`.github/workflows/actions.lock`、担当が `gh actions-lock` で生成）を使う。workflow 側の参照は `actions/checkout@v7.0.1`、`jdx/mise-action@v5.2.7`、`actions/cache@v6.1.0`。公式タグが指すコミットはそれぞれ `3d3c42e5aac5ba805825da76410c181273ba90b1`、`b44d12e7d0d8ee7461bb893e1b1993500433185b`、`55cc8345863c7cc4c66a329aec7e433d2d1c52a9`。
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
- rust にはカバレッジ計測用の `llvm-tools` 成分を `mise.toml` の `components` で宣言する。`toolchain-check` が sysroot 上の `llvm-profdata` / `llvm-cov` の存在を確認し、欠けていれば失敗する（暗黙の rustup 自動ダウンロードには頼らない）。`coverage` ジョブは `mise install` のあと、この検査を先に実行する。

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

### セキュリティスキャン（security-scan）

- `.github/workflows/security-scan.yml` が、main 向け Pull Request・main への push・手動実行で、秘密情報・危ないコードのパターン・脆弱な依存関係を検出する（Issue #571）。すべて無料の OSS で、有料サービス・アカウント登録・スキャナー用のトークンは使わない（checkout は GitHub Actions が自動で発行する読み取り用のトークンを使う）。
- 検出の一覧（場所と種類）はジョブサマリーに出す。次の検出が 1 件でもあれば、件数を `::error::` に出してジョブを失敗させる（`scripts/security-scan.sh` の終了コード 4）。それ以外の検出は件数を `::warning::` に出すだけで、ジョブは成功する（報告のみ）。ツール自体の失敗（ネットワーク断・設定の誤りなど）もジョブを失敗させる。必須チェックにはしていない（必須チェックにするかは hatai が決める）。本体は `scripts/security-scan.sh`、テストは `scripts/tests/security-scan.test.sh`。
  - Gitleaks: すべての検出。
  - Semgrep: 重大度が INFO 以外（WARNING・ERROR と、知らない重大度）の検出。INFO（`unsafe` の使用・`std::env::args`・`std::env::temp_dir` など、監査の目印にとどまる規則）は報告のみ。
  - OSV-Scanner: unmaintained の告知（RUSTSEC の `informational` が `unmaintained`。OSV の `affected[].database_specific.informational` で見分ける）以外の勧告が付いたパッケージ。unmaintained は報告のみにする（cargo-deny の第1段が unmaintained を失敗させないのと同じ扱い）。`unsound` など他の告知は失敗させる。サマリーでは、告知の勧告の後ろに種類を書く。
- ツールの診断（標準エラー）: 3 つのツールとも、診断には PR の作者が決められる値（設定の行・パス・取得先の URL など）が入りうり、失敗が終了コードに出ないことがある。そのため次のように扱う。
  - 標準エラーは公開のジョブログに出さず、ランナー上の一時ファイルにだけ書く。ログには失敗の種類と行数だけを出す。中身は手元で `SECURITY_SCAN_OUT=<dir> bash scripts/security-scan.sh <tool>` を実行し、`<dir>/<tool>.stderr` を読む（`MISE_ENV=security mise exec --` の下で実行する。Semgrep はイメージの中で実行する）。
  - Gitleaks と OSV-Scanner はログの水準を error に絞り（`--log-level error`・`--verbosity error`）、標準エラーに 1 行でもあれば、終了コードが検出なし・検出ありでも失敗にする。Gitleaks 8.30.1 は git log の失敗を、OSV-Scanner 2.7.1 はロックファイルの抽出の失敗を、ログに出すだけで終了コードに出さない。
  - Semgrep は解析エラーを JSON の errors に書き、検出の終了コード 1 をエラーより優先して返す。そのため level が error のエラーがあれば失敗にし、level が warn の構文エラーなどは件数だけをサマリーに出す。
- OSV-Scanner は `Cargo.lock` の照合だけにする: `--x-no-default-plugins --x-plugins rust/cargolock` で、抽出器を `Cargo.lock` 用だけにする。既定の抽出器（同梱の C/C++ ソースのパスとハッシュを OSV.dev に送る `directory` など）も、`lockfile` プリセット（`pom.xml`・`go.mod` などマニフェストの抽出器を含む）も使わない。PR の作者が足したマニフェストの依存を OSV.dev に照会しないため。別のエコシステムのロックファイルを入れるときは、その抽出器の名前を `scripts/security-scan.sh` に足す。`--no-resolve` でマニフェスト（`pom.xml` など）の推移的な依存を、PR の作者が指定できるレジストリへ取りに行かない。`--no-call-analysis=all` で呼び出し解析も使わない。
- サマリー: 件数は各ツールの JSON の検出の数から数える。表のセルはコードスパンにする（パスなどは PR の作者が決められるため、リンク・装飾・HTML・メールアドレスの自動リンクとして解釈させない）。改行は空白に、`|` は `\|` にする。ジョブサマリーは 1 ステップ 1 MiB を超えると丸ごと載らないため、一覧の行は UTF-8 のバイト数の合計が 900,000 バイトに収まる先頭の行までにし、省略した件数を書く。

| ジョブ | ツールと版 | 対象 | 導入 |
|---|---|---|---|
| `gitleaks (secrets)` | Gitleaks 8.30.1（MIT） | HEAD から辿れる git 履歴全体。マージコミットの差分（競合解消で足した行）も含める（`--log-opts="-m HEAD"`、checkout は `fetch-depth: 0`） | `mise.security.toml`（`MISE_ENV=security`）と `mise.security.lock` |
| `semgrep (code patterns)` | Semgrep CE 1.186.0（LGPL-2.1） | 作業ツリーのファイル（`tests/` の下を含む。リポジトリ直下の `.semgrepignore` が Semgrep の組み込みの既定の除外パターンを置き換え、今はここでは何も除外しない。ファイルの大きさやバイナリかどうか、未追跡のファイルへの `.gitignore` による Semgrep の選別は残る）。ルールセットは `p/rust`・`p/github-actions`・`r/bash`（`github-actions-mutable-action-tag` を除く） | 公式イメージ `semgrep/semgrep` を digest 固定した job コンテナ |
| `osv-scanner (dependencies)` | OSV-Scanner 2.7.1（Apache-2.0） | 作業ツリーのロックファイル（今は `Cargo.lock`） | `mise.security.toml` と `mise.security.lock` |

- 秘密の値は出さない: Gitleaks は `--redact` で値を伏せ、サマリーには規則・場所・コミットだけを書く。`-m` で同じ値が複数のマージコミットに重ねて出るため、サマリーには規則と場所の重複を除いた数も書く。
- Gitleaks は CLI を直接実行し、`gitleaks-action` は使わない。Action は v2 以降、独自の EULA（改変の禁止、Organization のリポジトリではライセンスキーが必要）で配布されている。CLI 本体は MIT で、版の固定も mise の checksum で揃えられる。
- Semgrep は Registry のルールをログインなしで取得し、`--metrics=off` で利用状況の送信を止める。Registry のルールセットは実行のたびに最新を取り、版は固定されない（イメージの digest が固定するのはエンジンと依存だけ）。同じコミットでも、日によって検出が変わりうる。WARNING 以上の規則が Registry に増えると、コードを変えなくてもジョブが失敗しうる。Registry のルールは Semgrep Rules License v1.0 で配布されている。利用は「自社の内部の業務目的」に限られ、ルールの再配布とサービスとしての提供は禁じられている（出典: https://semgrep.dev/legal/rules-license ）。このリポジトリでは、自分のコードを CI で検査するために使うだけで、ルールを複製してリポジトリに置かない。秘密情報向けのルールセットは入れない（秘密の検出は Gitleaks が担い、Semgrep は一致した行をそのまま出すため）。
- Semgrep を mise で入れない理由: PyPI 配布で、mise の pypi バックエンドが作る依存の固定（uv.lock）が 1800 行を超えるため。イメージの digest は依存を含む中身全体を固定する。
- 版の更新: 週次の `mise-bump.yml` は `MISE_ENV` を付けずに実行するため、`mise.security.toml` の Gitleaks・OSV-Scanner の版は上がらない。Semgrep のイメージの digest も同じ。Gitleaks の検出ルールは本体に入っているため、古い版のままだと新しい種類の秘密を見落とす。今は手で上げる: `MISE_ENV=security mise upgrade --bump aqua:gitleaks/gitleaks aqua:google/osv-scanner`（ツール名を付けないと `mise.toml` の共通のツールも上がる）のあと `MISE_ENV=security mise lock` で `mise.security.lock` を作り直し、Semgrep は新しいタグの digest に `security-scan.yml` を書き換える（workflow の変更は hatai が push する）。
- OSV-Scanner と cargo-deny（`deny-check`）は、どちらも `Cargo.lock` を RustSec の勧告と照合する点で重なる。違いは次のとおり。
  - OSV-Scanner は、OSV データベースの他の出所（GHSA など）も見る（今は `Cargo.lock` だけを照合する）。
  - cargo-deny は `deny.toml` の ignore と警告の扱いを持ち、必須チェックの一部である。
  - OSV-Scanner には `deny.toml` の ignore が効かない。cargo-deny で ignore した勧告のうち告知でないもの（今は rsa の RUSTSEC-2023-0071）は、`osv-scanner.toml` にも同じ理由で書く。
- Semgrep の `yaml.github-actions.security.github-actions-mutable-action-tag.github-actions-mutable-action-tag` は `--exclude-rule` で使わない。この規則は `uses:` がコミット SHA でないことを検出するが、このリポジトリは `uses:` をタグで参照し、SHA を `.github/workflows/actions.lock`（`gh actions-lock` が生成）で固定する。
- 除外（`.gitleaks.toml`・`.semgrepignore`・`osv-scanner.toml`）は、1 件ずつ理由を書いて足す。`.semgrepignore` を消すと、Semgrep は組み込みの既定の除外一覧（`tests/`・`test/`・`vendor/` など）に戻る。そのため、ファイルは中身が空でも残す。既存の検出を理由に、ルールの無効化や除外の範囲を広げない。
  - Gitleaks の除外は `.gitleaks.toml` の `[[allowlists]]` に書く（`[extend] useDefault = true` で組み込みの規則はそのまま使う）。1 つの allowlist は `condition = "AND"` にし、`targetRules`（規則）・`paths`（ダミー値を置いたファイル。`^…$` で 1 ファイルずつ）・`regexes`（既知のダミー値そのもの。`^…$` で値全体を書き、検出された値に当てる）のすべてに合うものだけを許す。行番号を使わないため、ファイルの行がずれても CI は落ちない。同じファイルの別の値は除外されない。値の形（桁数・文字種）だけの除外は書かない。
  - `.gitleaksignore`（「コミット:ファイル:規則:行」や「ファイル:規則:行」の指紋）は使わない。指紋は行番号を含むため、行がずれると同じ値が別の検出として出直す。新しいダミー値を除外するときは、そのファイルとその値を書いた allowlist を足すか、既存の allowlist の値をそのまま使う。
  - `osv-scanner.toml` は `Cargo.lock` と同じディレクトリに置き、`[[IgnoredVulns]]` に `id` と `reason` を書く。`deny.toml` の `ignore` と同じ勧告は同じ理由と Issue を書く。unmaintained の告知は足さない（報告のみのため）。

## GitHub 通知（Grok Bot webhook・notify-grokbot）

`.github/workflows/notify-grokbot.yml` が、Grok Bot が動く必要のあるイベント（PR の作成（`opened`・`ready_for_review`）と push（`synchronize`）、クローズ（マージを含む）、レビューの投稿、verify・nightly workflow の完了（`workflow_run`））と手動実行（`workflow_dispatch`）を Grok Bot の webhook に POST する。エージェントのスレッドの状態は見張り役 `agent-watch` が別に送る（「pitchfork について」参照）。送るのはメタデータのみで、PR タイトル・本文、コメント本文、レビュー本文、ラベル、メールアドレス等は送らない。

- Secret（リポジトリの Settings → Secrets and variables → Actions → New repository secret で登録。値はドキュメント・コードに置かない）:
  - `GROKBOT_WEBHOOK_URL` — webhook の URL
  - `GROKBOT_WEBHOOK_AUTH` — `Authorization` ヘッダーの値全体
- 送る JSON のキーは 14 個固定: `schema`（`"pelway.github-notify/v2"`）, `urgency`（`digest` は workflow_run の成功（verify・nightly）・Approve のレビュー・PR への push（`synchronize`）、それ以外は `immediate`）, `repo`, `event`, `action`, `pr_number`, `head_branch`, `head_sha`, `merged`, `conclusion`, `workflow_name`, `run_url`, `actor`, `occurred_at`。値の無いものは `null`。
- 送らない条件（通知をスキップし、ジョブは成功で終わる。CI は落ちない）:
  - 上記 Secret のどちらかが未設定のとき
  - fork 由来のイベント（PR 系は head リポジトリ、workflow_run は head_repository の full_name がこのリポジトリと違うとき）
  - レビュー（pull_request_review）で、投稿者の `author_association` が `OWNER` / `MEMBER` / `COLLABORATOR` のいずれでもないとき（ボット例外なし。レビュー担当の GitHub App の結果は `agent-watch` がスレッドの報告行として送る）
  - `synchronize` で、同じ PR・同じ head sha の通知をすでに送った過去 run があるとき（dedup。workflow が `actions: read` 権限でこの workflow の過去 run を調べ、`Send notification` step が成功した run があれば送らない。調べられなければ送る側に倒す）
- 対象外: PR の会話欄に書いた普通のコメント（issue_comment）は通知しない。fork から来た PR へのコメントを安全に見分けられないため。あとで足す場合は、`pull-requests: read` で PR を調べて fork を除く形にし、別の PR で行う。
- 手動実行（workflow_dispatch）は `event: "test"` のテスト通知を 1 通送る。送信の失敗（タイムアウト・HTTP エラー）で CI は落ちない。

## レビュー担当の Codex（codex_review）

AI 開発チームのレビュースレッドは、T3 の Codex インスタンス `codex_review` で動かす（`docs/superpowers/specs/agent-team.md` の「レビュー担当の権限」）。`codex_review` はほかの Codex と home（`CODEX_HOME`）を分け、レビュー担当専用の `~/.codex-review` で動かす。レビュー担当（`codex_review`・`claude_review` のどちらも）が `scripts/agent-gh review` で読む fnox の設定は、環境変数 `PELWAY_AGENT_FNOX_CONFIG_REVIEW`（既定 `~/.config/pelway-agent-review/fnox.toml`）。`PELWAY_REVIEW_APP_ID` などの `PELWAY_REVIEW_*` の秘密はこのファイルに載る（dev 用の `PELWAY_AGENT_FNOX_CONFIG`・`~/.config/pelway-agent/fnox.toml` とは分離）。このマシンでの設定は次のとおり。

| 場所 | 設定 |
|---|---|
| `~/.t3/userdata/settings.json` の `providerInstances.codex_review` | `driver: codex`、`config.homePath: /home/hatai/.codex-review`（UI のフィールド名は「CODEX_HOME path」。app-server の環境変数 `CODEX_HOME` になる）、`config.launchArgs: --enable network_proxy -c default_permissions=review`（ほかに `shell_environment_policy.set` で `PELWAY_AGENT_FNOX_CONFIG` と `GH_CONFIG_DIR` を渡す。どちらも `scripts/agent-gh review` が実際に使う値ではない。review ロールの資格情報は `PELWAY_AGENT_FNOX_CONFIG_REVIEW`（未設定なら `~/.config/pelway-agent-review/fnox.toml`）から読み、`GH_CONFIG_DIR` は呼び出しのたびに空の一時ディレクトリへ置き換える） |
| `~/.codex-review/auth.json` | `~/.codex/auth.json` への symlink（ログインは hatai の Codex と共有する） |
| `~/.codex-review/config.toml` の `[permissions.review]` | ファイルは読み取り（`codex sandbox -P review` で動かしたコマンドは `/tmp` だけ書ける。T3 のスレッドでは worktree にも書ける。Issue #589）、ネットワークは `api.github.com` と `github.com` だけ（SOCKS は使わない） |
| `~/.codex-review/config.toml` の `[features]` と `[mcp_servers.*]` | `apps`・`plugins`・`remote_plugin`・`hooks` を `false` にする。`[mcp_servers.t3-code]` と `[mcp_servers.t3-code-direct]` を `enabled = false`（`url` は設定の読み込みのための仮の値 `http://127.0.0.1:1/mcp`）にする。最上位のキーに `web_search = "disabled"` を書く。`[projects.*]` の信頼設定は書かない（worktree の `.codex/` を読ませない） |
| `/etc/codex/requirements.toml`（#449 で追加。hatai が root で作る） | `[permissions.filesystem]` の `deny_read` に秘密パス 5 件（`~/.config/fnox`・`~/.config/pelway-agent`・`~/.config/pelway-agent-qa`・`~/.ssh`・`~/.config/gh`）を列挙する。requirements 層の設定で、ユーザーの `config.toml` に書いても効かない。このマシン（単一ユーザー）の全 codex セッションに適用される |

`deny_read` は sandbox ポリシー（FileSystemSandboxPolicy）に Deny エントリとして渡り、sandboxed なコマンドから当該パスが読めなくなる（codex 0.169.0 の `codex-rs/config/src/filesystem_constraints.rs` と `requirements_layers/permissions.rs` による。パスは絶対パスで書く）。

ネットワークの制限が効くのは、スレッドの権限が `auto-accept-edits` のとき（コマンドが sandbox の中で動くとき）だけ。`full-access` のスレッドでは効かない。

T3 はどのスレッドにも、T3 自身の MCP サーバー（`t3-code`・`t3-code-direct`。PR・Issue・スレッドの操作のツール）を、スレッドの開始時の設定として渡す。そのツールは T3 のサーバーの中で動き、レビュースレッドの sandbox・ネットワーク制限・`GH_CONFIG_DIR` の差し替えが効かないので、`codex_review` では使えなくする。止め方の制約:

- 起動引数の `-c mcp_servers.t3-code.enabled=false` と `disabled_tools` は、T3 がスレッドの開始時に渡す同じ名前の設定に上書きされて効かない。`config.toml` の層に書いた `enabled = false` は残る。
- `~/.codex/config.toml` に書くとほかの Codex のスレッドからも T3 のツールが消えるため、`codex_review` の home を分ける。`shadowHomePath` は `config.toml` を `~/.codex` と共有するので使えない。
- `homePath` を変えたら T3 を再起動する（設定画面でインスタンスを入れ直しても、動いている app-server は起動し直されない）。

Codex の GitHub 連携（`apps`・`plugins`）も、OpenAI 側で GitHub を操作し sandbox の外で動くため、`~/.codex-review/config.toml` で止める。

`network_proxy` は Codex の試験的な機能で、`--enable network_proxy` が無いと権限プロファイルのネットワーク制限が効かない。Codex を更新したら `mise run codex-review-check` で、`api.github.com` と `github.com` には HTTP 200 が返り、`example.com` には HTTP の応答が無いことを確かめる（`CODEX_HOME=~/.codex-review` の `[permissions.review]` を使う）。このコマンドが見るのはこの 3 つの宛先への応答だけで、許可先の設定そのものは読まない。`example.com` に応答が無い原因（sandbox の遮断か、ほかの接続の失敗か）も区別しない。許可先は `~/.codex-review/config.toml` を読んで確かめる。

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

### 専用 home での確認

確かめ終わるまでの間は、この見出しのすぐ下に `保留中: <更新したもの>` の行を置き、確かめ終わったら消す。更新を予定しているときは、その行が main に載ってから更新を始める（載せ方と、載るまでの間の伝え方は skill `command-team` の「レビュー担当と effort」の確認ゲート）。指揮役と実装担当（レビューの委託の担当を決めるとき）は、委託やスレッドの起動・再開の直前に main の最新の版（`scripts/agent-gh dev api -H "Accept: application/vnd.github.raw" "repos/hatai/Pelway/contents/docs/dev-environment.md?ref=main"`）でこの行の有無を見て、`codex_review` を使えるかを判断する。「レビュー担当の Claude」にも、T3・Claude Code・`~/.claude-review` を更新したときに同じ `保留中:` の行を置く。

`codex_review` を初めて設定したときと、Codex・T3・`codex_review` の home のどれかを更新したときは、`codex_review` に委託した確認用の子タスク（`runtimeMode: "auto-accept-edits"`）で次を確かめる。確かめ終わるまで、`codex_review` をレビューに使わない。確認用の子タスクは、`main` のコミットで動くスレッドから委託する（`delegate_task` の子は委託元のコミットを checkout するので、PR のブランチから委託すると、PR が変えた `scripts/agent-gh` や `mise.toml` のタスクを実行しうる）。`config.toml` の層の `enabled = false` が T3 の設定に上書きされないことは Codex の設定の重ね方に、サーバー名 `t3-code`・`t3-code-direct` は T3 に依存するため、どちらを更新しても確かめ直す。結果は `docs/worklogs/` に日付付きの作業ログとして残し、確かめたときの版（`codex --version`、`claude --version`、T3 の `~/.t3/runtime/service-state.json` の `activeVersion`）も書く（初回は `2026-11-21-codex-review-home.md`）。巡回などで今の版が最後の記録と違うと分かったら、更新があったものとして保留にする。

- モデルに渡るツールを全部挙げさせ（直接呼べるツール、`exec` から呼べるツール、遅延読み込みのツールのすべて）、T3（`t3-code`・`t3-code-direct`）・`codex_apps`・GitHub の MCP サーバーのツールと、ウェブ検索（`web_search`・`web.run`）が無い。`ALL_TOOLS` の一覧だけでは足りない（T3 の `t3-code-direct` とサーバー側のウェブ検索はその一覧に載らない）
- ウェブ検索の実効の設定が無効になっている（`~/.codex-review/config.toml` の `web_search = "disabled"` と、`launchArgs` にそれを上書きする `-c` が無いこと）
- `CODEX_HOME` が `/home/hatai/.codex-review` になっている
- `scripts/agent-gh review api /rate_limit` が成功する
- GitHub 以外への通信（`scripts/agent-gh review api --hostname example.com /`、`curl https://example.com`）が遮断される
- 秘密の置き場 5 パス（`/etc/codex/requirements.toml` の `deny_read`）が読めない（`test -r` が 1）。`codex sandbox -P review` は `deny_read` を使わないので、この項目は確認用の子タスクの中で確かめる
- 確認のあとの `~/.codex-review/config.toml` に、`[projects.` で始まる表が無い（T3 と Codex の app-server は sandbox の外でこのファイルに書けるため。信頼設定が入ると worktree の `.codex/` が読まれる）
- `~/.codex-review` に書き込めない（`touch ~/.codex-review/config.toml` と、新しいファイルの `touch` が拒否される）。止める設定がこのファイルに集まっているため、書き込めると次のセッションで設定を戻せる
- `mise run codex-review-check` が ok

どれかが通らなければ `codex_review` をレビューに使わない。T3 のツールや Codex の GitHub 連携が残っていたら `~/.codex-review/config.toml` を直し（`homePath` を変えたときは T3 を再起動し）、全部を確かめ直してから使う。

worktree への書き込みは、今は通る（権限プロファイル `review` の `/tmp` だけの制限が、T3 のスレッドの `workspace-write` では worktree に広がる）。直す方向は Issue #589 で決める。それまでは、worktree に書き込めても `codex_review` を使う（レビュー担当は PR のコードを実行せず、ファイルを書き換えないため。skill `review-pr` の「決まり」）。

## レビュー担当の Claude（claude_review）

高リスクの PR と抜き取りのレビュースレッドは、T3 の Claude インスタンス `claude_review` で動かす（`docs/superpowers/specs/agent-team.md` の「レビュー担当の権限」）。Codex のレビュー担当と同じ条件にそろえる。

| 場所 | 設定 |
|---|---|
| `~/.t3/userdata/settings.json` の `providerInstances.claude_review` | `driver: claudeAgent`、`config` の `homePath` に `/home/hatai/.claude-review`（UI のフィールド名は「CLAUDE_CONFIG_DIR path」。子プロセスの環境変数 `CLAUDE_CONFIG_DIR` になる。`config.environment` は claudeAgent の config スキーマに無いキーで無視される） |
| `~/.claude-review/settings.json` | sandbox を有効にし、ネットワークの許可先を `api.github.com` と `github.com` だけにする。使えるコマンドは読み取り系の `git`（`git show`・`git diff`・`git log`・`git grep`・`git ls-tree`・`git cat-file`）と、`scripts/agent-gh review` を使う gh サブコマンド単位の形（`api`・`issue view`・`pr view`・`pr diff`・`pr checks`・`pr review`・`run list`・`run view`）に絞る（`scripts/agent-gh review *` のワイルドカードは使わない。`agent-gh` が引数を `git`/`gh` にそのまま渡すため、ワイルドカードは `auth token`・`repo clone`・`git` 全般などあらゆる形を許してしまう）。ファイルの書き込みを拒否する。deny は念のためラッパー経由の形でも書く（`git *`・`pr checkout *`・`alias *`・`auth *`・`repo *`・`gist *`）。#449 ではさらに `permissions.deny` に秘密パスの `Read(...)` 4 件（`~/.config/fnox/**`・`~/.config/pelway-agent/**`・`~/.ssh/**`・`~/.config/gh/**`）を足す（QA 用の `Read(~/.config/pelway-agent-qa/**)` も、下の「QA 担当の GitHub App（pelway-qa）」の手順 5 で足してある）。Claude Code 2.1.x は `Read(...)` deny を `sandbox.filesystem.denyRead` にマージし、sandboxed なコマンドにはファイルシステム層（bwrap）でも隠す。#306 では `sandbox.filesystem.denyWrite` に `"~/"` を入れ、ホームの下（worktree・共有 `.git`・`$HOME`・`~/.config`・`~/.claude-review` 自身）への書き込みを sandbox で拒否する。実測で書けたのは sandbox の一時フォルダ（`$TMPDIR`。実体は `/tmp/claude-1000` で、uid ごとに 1 つのためレビュー専用ではない）で、`/tmp` 直下は拒否された。ホームの外のほかの場所（`/dev/shm`・`/var/tmp` など）は測っていない。許可した読み取り系 `git` の `--output=<path>` や `git grep -O<コマンド>` で起動したコマンドも、この拒否を受ける。T3 が渡す MCP サーバーのツールは、`permissions.deny` に `mcp__t3-code` と `mcp__t3-code-direct`（サーバー名だけで、そのサーバーの全ツール）を書いて使えなくする（理由は「レビュー担当の Codex」と同じ）。claude.ai のコネクタ（外部のサービスへ送信できるツールを含む）は、`env` の `ENABLE_CLAUDEAI_MCP_SERVERS=false` で止める。組み込みのツールのうち、ほかのセッションや claude.ai へ送れるもの（`SendMessage`・`RemoteTrigger`・`Artifact`・`ShareOnboardingGuide`・`PushNotification`・`DesignSync`・`PublishPlugin`）は `permissions.deny` に書く（`WebFetch`・`WebSearch` は前から deny にあり、サブエージェントにも効く）。アカウントから同期されるプラグインの MCP サーバーは、`env` の `CLAUDE_CODE_SKIP_PLUGIN_MCP_SERVERS=1` で読み込ませない（接続に失敗しているだけの状態は、セッションの途中で接続が戻りうるので、止めたことにならない）。下の `hooks.PermissionRequest` は承認要求を拒否するが、これに頼ってプラグインのツールを残さない |
| socat | Claude Code の sandbox（Linux）が使う。`mise.toml` の `conda:socat` で `mise install`（`mise bootstrap`）が入れる。apt と sudo は要らない |
| ログイン | `CLAUDE_CONFIG_DIR=~/.claude-review claude` で一度ログインする（人間が行う） |
| `~/.claude-review/settings.json` の `hooks.PermissionRequest` | matcher `*` で `/home/hatai/.claude-review/hooks/deny-permission-request.sh` を呼ぶ。承認要求になる場面ごとに、人間に聞かずに拒否を返す。拒否のメッセージは、sandbox の遮断ではないこと、Bash なら単純なコマンドに分けて送り直すことをモデルに伝える。フックは worktree の外に絶対パスで置く（委託のレビューは PR の作者のコミットの checkout で動くため、リポジトリの中に置くと PR が差し替えられる） |

T3 は `--permission-mode=acceptEdits` を渡すので、`defaultMode: dontAsk` は効かない。許可リストに無いコマンドのうち、Claude Code が読み取り専用と判定できる単純なもの（`echo`・`grep`・`wc`・`git log`・`git rev-parse`・`git merge-base` など）は承認なしで通る。判定できないもの（`git -C <パス>`、許可リストに無いスクリプトやサブコマンド）と、変数の代入・`$(...)`・`$?`・ループ・`<(...)` を含むコマンドは承認要求になる。レビューでよく使う読み取り専用のコマンド（`git ls-tree`・`git cat-file`・`agent-gh review run list`/`run view`）は許可リストに入れて、拒否されないようにしてある。フックはこの承認要求を拒否に変える。許可の範囲は広げない（`allow` を返さない）。フックの中身は次のとおり。

```sh
#!/bin/sh
cat > /dev/null
cat <<'EOF'
{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"レビュー担当では承認要求を出さず、自動で拒否した。sandbox による遮断ではないので、これだけを理由に verdict: failed にしない。Bash なら、変数の代入・$(...)・$?・for/while のループ・<(...) を使わない単純なコマンドに分けて 1 つずつ送り直す（終了コードはツールの結果に出るので echo $? は要らない）。拒否されたのと同じコマンドをそのまま送り直しても、また拒否される。それ以外のツールや、分けても通らない操作は、手順にない操作として行わずに進める。"}}}
EOF
```

戻すときは `hooks.PermissionRequest` を消す。

運用に入る前に、`auto-accept-edits` のスレッドで次を確かめる。確かめるまでは `claude_review` を使わない。

- `scripts/agent-gh review api /rate_limit` が成功する（GitHub にはつながる）
- `curl -sS https://example.com` が失敗する（GitHub 以外にはつながらない）
- `git switch` と、ファイルの書き込みが拒否される
- `agent-gh review` を経由する他の gh サブコマンドが拒否される: `scripts/agent-gh review git switch -c <probe>`・`scripts/agent-gh review repo clone <リポジトリ> <dir>`・`scripts/agent-gh review auth status` が permissions の deny で拒否される（許可は使用するサブコマンド単位に絞ってあり、deny もラッパー経由の形で二重に張ってある）
- 許可経路からの egress が遮断される: `scripts/agent-gh review api --hostname example.com /` が失敗する（sandbox のネットワーク許可先の実測。許可リスト内で GitHub 以外へ出られるコマンド形で試す）
- 新しい `claude_review` のセッションのツールの一覧に、`mcp__` で始まるツールが無い（T3 の `mcp__t3-code`・`mcp__t3-code-direct`、claude.ai のコネクタ、プラグインの MCP サーバー）。どれか 1 つでも残っていれば、呼んで確かめずに `claude_review` をレビューに使わない（確かめるための呼び出しが、承認の設定に不備があればそのまま外部のサービスに届くため）。設定でそのツールを止めてから確かめ直す（プラグインなら `env` の `CLAUDE_CODE_SKIP_PLUGIN_MCP_SERVERS=1` を確かめる）。システムの通知に `plugin:` で始まる MCP サーバーが出ていても、同じく使わない。deny はサーバー名に依存するので、T3・Claude Code・`~/.claude-review` を更新したらこの項目を確かめ直す。確かめ終わるまでは、この節の見出しのすぐ下に `保留中: <更新したもの>` の行を置き、指揮役と実装担当は委託やスレッドの起動・再開の直前に main の最新の版でこの行の有無を見る
- 組み込みのツールの一覧を最後の作業ログの記録と比べ、ほかのセッションや claude.ai へ送れるツールが増えていない（増えていれば `permissions.deny` に足してから使う）
- 許可された形で worktree と共有 `.git` に書けない: `git log -1 --format=%H --output=<worktree のファイル>` と `--output=<共有 .git のファイル>` が `Read-only file system` で失敗する。`git grep -O"bash -c 'echo x > <ファイル>'" -e <パターン>` で起動したコマンドも worktree・`/tmp` 直下に書けず、`git grep -O` で起動した `curl` は GitHub 以外へ接続できない。`mktemp` は `$TMPDIR` に作れる。書式は `%` を含むもの（`%H`）にする（git 2.56 は `--format=x` のような `%` の無い書式を `fatal: invalid --pretty format` で拒否し、`--output` まで届かない）
- 許可にも deny にも無い読み取り系でない形（例: `scripts/agent-gh review release list`）が止まる（承認要求になる、または拒否される）

今の保留の扱い: 上の最後の項目（`release list` が止まる）と、下の「#306 で追っている経路」「確かめ終えていない項目」は確かめ終えていないが、`claude_review` は hatai の決定で運用を続けている。保留を解く条件は、#306 の実測と QA の照合が済むこと。

### 確認手順の結果（記録欄）

確認手順を実施したら、結果を次の表か、`docs/worklogs/` の作業ログに記録する。記録されるまで `claude_review` は使わない（command-team の保留）。T3 の MCP サーバーとコネクタを止めた後の確認は、作業ログ `docs/worklogs/2026-11-21-codex-review-home.md` にある。記録は Issue #256 の受け入れ条件の確認にも使う。

| 項目 | 結果 | 実施日 |
|---|---|---|
| 実施日（hatai によるレビュー用 Claude へのログインと、T3 への `claude_review` の追加を含む） | hatai によるレビュー用 Claude へのログインと `claude_review` の追加は済んでいる（`config.homePath` への修復は指揮役（代行）が hatai の分担を代わりに、許可の gh サブコマンド単位への絞り込みと deny への `git *`・`pr checkout *`・`alias *`・`auth *`・`repo *`・`gist *` の追加は hatai がそれぞれ適用した）。確認は `delegate_task`（target `claude_review`、`auto-accept-edits`、worktree）で実施。`CLAUDE_CONFIG_DIR` の継承は親側で子プロセスの environ（`CLAUDE_CONFIG_DIR=/home/hatai/.claude-review`）を確認して確かめた。許可・拒否のどちらのリストにも無い `echo "$CLAUDE_CONFIG_DIR"` は承認要求になり、承認を経て同じ値を出力した | 2026-11-14 |
| 使えるコマンドの制限の結果（読み取り系 `git` と `scripts/agent-gh review` に絞られているか。`git switch` とファイルの書き込みが拒否されたか） | `scripts/agent-gh review api /rate_limit`・`pr view`・`issue view` は承認なしで成功（許可は使用する gh サブコマンド単位に列挙）。`git switch -c perm-check-256` は permissions の deny で拒否。Write ツールは deny によりセッションで無効化され（`No such tool available`）、ファイルは作られなかった。ラッパー経由の形（`scripts/agent-gh review git switch -c <probe>`・`git -c 'alias.x=!<コマンド>' x`・`pr checkout <N>`・`repo clone <リポジトリ> <dir>`・`auth status`）は許可の列挙に入らず、deny（`git *`・`pr checkout *`・`alias *`・`auth *`・`repo *`・`gist *`）でも拒否される | 2026-11-14 |
| ネットワーク制限の結果（`api.github.com` と `github.com` だけにつながるか。`curl -sS https://example.com` が失敗したか） | `curl -sS https://example.com` は permissions の deny で拒否（実行自体されない）。GitHub API には到達した。egress の実測: 許可経路の `scripts/agent-gh review api --hostname example.com /` は実行されるが sandbox のプロキシで遮断（`host is not on the allow list`）。`scripts/agent-gh review git ls-remote https://example.com/` は deny で拒否 | 2026-11-14 |
| Codex 側（`codex_review`）との差（許可先・効く条件・コマンドの制限で異なる点。無ければ「差なし」） | 許可先は同じく GitHub のみ。遮断の仕組みが異なり、Codex は `network_proxy`＋権限プロファイル、Claude は `~/.claude-review/settings.json` の permissions（allow/deny）＋ sandbox。ファイルの書き込みは Write/Edit ツール自体が無効化される。許可・拒否のどちらにも載っていないコマンドは、T3 が `--permission-mode=acceptEdits` を渡すため承認要求になるのが規則だが（`defaultMode: dontAsk` より acceptEdits が優先される）、#306 の実測では 2 通りに分かれた。変数展開を含む `echo "$CLAUDE_CONFIG_DIR"` は承認要求になった（上の行）。変数展開を含まない `scripts/agent-gh review release list` は、承認要求にならずに実行された（#306。下の行）。許可外のコマンドがどう扱われるかの規則は、上の「T3 は `--permission-mode=acceptEdits` を渡すので」の段落にある。書き込みの遮断も異なり、Codex の権限プロファイルは `/tmp` 以外を読み取り専用にする（ただし T3 のスレッドでは worktree にも書ける。「レビュー担当の Codex」と Issue #589）のに対し、Claude は sandbox の `denyWrite: ["~/"]` でホームの下を拒否する。Claude の実測では `$TMPDIR`（`/tmp/claude-1000`）に書けて、Codex と違い `/tmp` 直下は拒否された（ホームの外のほかの場所は測っていない） | 2026-11-14（書き込みの遮断と、許可にも deny にも無いコマンドの扱いは 2026-11-21） |
| 承認要求を拒否に変えるフック（`hooks.PermissionRequest`）の結果 | `delegate_task`（target `claude_review`、`auto-accept-edits`、worktree）で確認。`H=$(git rev-parse HEAD); …; echo rc=$?` と `for f in …; do wc -l "$f"; done` は承認待ちにならずフックの拒否が返り、モデルは単純なコマンドに分けて送り直して結果を得た。`scripts/agent-gh review api /rate_limit` はそのまま通り、`curl -sS https://example.com` は permissions の deny で拒否のまま。スレッドに承認待ち（`pendingApproval`）は出なかった | 2026-11-21 |
| フック導入後の実レビューの観察 | 導入後 1 時間の `claude_review` の実レビュー 9 セッションで、フックの拒否は 26 件。拒否を理由に `verdict: failed` になったレビューは無かった（`failed` の 1 件は PR のチェックの未報告による）。拒否のうち `git ls-tree`・`git -C`・`agent-gh review run list` は単純な形でも承認要求になっていたため、`git ls-tree`・`git cat-file`・`run list`・`run view` を許可に足した（`claude -p` で通ることと、`run download` は拒否のままであることを確認）。拒否されたのと同じコマンドを 1 回そのまま送り直す例が多かったため、フックのメッセージに 1 文足した | 2026-11-21 |
| 書き込み先の制限の結果（#306。許可された形で worktree と共有 `.git` に書けないか） | `denyWrite: ["~/"]` の適用後、`delegate_task`（target `claude_review`、`auto-accept-edits`、worktree）で実測。`git log -1 --format=%H --output=t306-probe-wt.tmp`（worktree）と `--output=/home/hatai/repos/hatai/Pelway/.git/t306-probe-gitdir.tmp`（共有 `.git`）はどちらも exit 128・`Read-only file system`。`git grep -O"bash -c 'echo x > t306-probe-grep.tmp'" -e mise`（worktree）と同じ形の `/tmp/t306-probe-tmp.tmp`（`/tmp` 直下）は、コマンドは起動するが書き込みが `Read-only file system` で失敗。`git grep -O"bash -c 'mktemp'" -e mise` は `/tmp/claude-1000/tmp.*` を作れた。`git grep -O"bash -c 'curl -sS -m 10 https://example.com -o /dev/null'" -e mise` は sandbox のプロキシで遮断（`CONNECT tunnel failed, response 403`、`host is not on the allow list`）。実測の後、親スレッドから 4 つの書き込み先にファイルが無いことを `ls` で確かめた。別の実測スレッドでは `touch` の結果が worktree・`$HOME`・`$HOME/.config` で拒否、`$TMPDIR` で成功だった | 2026-11-20・2026-11-21 |
| 許可にも deny にも無い読み取り系でない形の結果（#306。`scripts/agent-gh review release list` が止まるか） | 止まらなかった。承認要求にならずに実行され、exit 0・出力なし（リポジトリにリリースが無いため。セッションの記録 `~/.claude-review/projects/<作業パス>/<SESSION_ID>.jsonl` でも、承認を挟まずに `Bash completed with no output` で終わっている）。セッションの `permissionMode` は `acceptEdits`、設定は `defaultMode: dontAsk`。`sandbox.autoAllowBashIfSandboxed` は設定に無い。この項目の既定は false（実測時の Claude Code 2.1.329 の本体に同梱された設定の説明に「Default: false」「still defaults to false」とある。2.1.330 も同じ）ため、この項目は原因ではない。止まらなかった原因（T3 の `auto-accept-edits` が承認要求に自動で許可を返しているのか、Claude Code の permissions の評価なのか）は特定していない。この実測は、承認要求を拒否に変えるフック（`hooks.PermissionRequest`）を入れる前のもの。上の「T3 は `--permission-mode=acceptEdits` を渡すので」の段落は、許可リストに無いスクリプトやサブコマンドは承認要求になる（フックで拒否される）としており、フック導入後の観察では `agent-gh review run list` が承認要求になって拒否された。この記述と、`release list` がフック導入前に承認なしで通った実測は食い違う。フック導入後の測定では拒否された（承認要求は出ず、実行されなかった）が、`cd <repo> && …` の複合形で行ったため、拒否の対象が `release list` 自体かは切り分けられていない。単一形での再測と QA の照合は #306 で待っている | 2026-11-21 |
| 秘密の置き場の読み取り（#306 の決定で、`denyWrite` の適用後に記録する項目。1 = 拒否、0 = 成功） | `test -r` の結果は `~/.config/fnox` 0・`~/.config/pelway-agent` 0・`~/.config/pelway-agent-qa` 0・`~/.config/gh/hosts.yml` 1。期待した「1 のまま」になっていない。`test -r` は access(2) で権限ビットを見るだけのため、denyRead で中身が隠されたディレクトリでも 0 になりうる。中身を読む実測（`wc -c` など）はエージェントには行わせない | 2026-11-20 |

`denyWrite` の差分と戻し方（`~/.claude-review/settings.json`。設定は hatai が手で変える）:

```diff
     "filesystem": {
       "denyRead": [ ...変更なし... ],
+      "denyWrite": [
+        "~/"
+      ]
     }
```

戻すときは、`~/.claude-review/settings.json` の `sandbox.filesystem` から `denyWrite` の項目だけを消す（`denyRead` などほかの項目は残す）。変える直前の控え `~/.claude-review/settings.json.bak-306-20261120` をまるごと戻すと、控えを取った後に足した `hooks.PermissionRequest` や許可リストの変更まで消えるため、控えは差分を確かめるためだけに使う

前からある未確認の経路: 委託のレビューは PR の head の checkout で動くので、PR が足した `.claude/`・`.mcp.json`・`.codex/`・`.devin/`（Devin CLI がプロジェクト層として読む `config.json`・`hooks.v1.json`・`mcp_config.json`・`rules/`・`global_rules.md`・`skills/`）・`.cursor/`・`.cursorrules`・`.windsurf/`・`.windsurfrules`、プロジェクトの指示のファイル（どのディレクトリの `AGENT.md`・`AGENTS.md`・`AGENTS.local.md`・`AGENTS.override.md`・`CLAUDE.md` も）と、PR が変えたレビュー担当のスキル（`.agents/skills/review-pr`・`.agents/skills/security-review`・`.agents/skills/pr-watch`。`.claude/skills/` はその symlink）を読みうる（project の設定は `~/.claude-review/settings.json` より優先され、`env` の上書きや `.devin/hooks.v1.json` のような hooks の実行がありうる）。手当てとして、これらを足す・変える PR はレビューを委託しない（skill `implement-issue` の手順 8）。

未確認の経路: Codex のファイルを書くツール `apply_patch` が、`~/.codex-review` と worktree の外へ書き込めるか。確認用の子タスクのセッションの記録では、承認の方針（`approval_policy`）が `on-request` で、この方針では書き込める範囲の外への patch は拒否ではなく承認要求になる。T3 の `auto-accept-edits` がその承認要求をどう扱うかは確かめていない（`docs/worklogs/2026-11-22-codex-review-apply-patch.md`）。使う前の確認の一覧には入れず、扱いは hatai が決める。

未確認の経路: `Agent` ツールの `isolation: "remote"`（クラウドの環境でエージェントを動かす）が、アカウントの設定で使えるかを確かめていない。使えると、指示文を sandbox の外へ渡せる。扱いは hatai が決める。

残っている既知の経路は hatai の決裁で別 Issue で追う（PR #287 の round 4 で両レビュー担当が request-changes として指摘し、hatai が残リスクを受容）:

- `api -F <key>=@<file>`・`pr review --body-file <file>` で読み取り可能なローカルファイルを GitHub へ送信できる（#305）

#306 で追っている経路（#306 は開いたまま。受け入れ条件は変えず、実測と QA の照合が済んだら閉じる）:

- 許可した `git grep *` の `-O<コマンド>`（`--open-files-in-pager`）で、任意のコマンドを起動できる。起動するのは git で、シェル経由で起動するため、内側のコマンドは permissions の deny（`curl *`・`git *`・`auth *`・`gist *` など）とも、承認要求を拒否に変えるフックとも照らし合わされない。`denyWrite` の適用後は、内側のコマンドも sandbox の中で動く。実測では、ホームの下と `/tmp` 直下への書き込みは拒否され、`$TMPDIR` には書けた（ホームの外のほかの場所は測っていない）。外への接続は GitHub だけになる（記録欄の「書き込み先の制限の結果」の行）。起動したコマンドから何を読めるか、`scripts/agent-gh review` の資格情報を使えるかは確かめていない
- 許可にも deny にも無いコマンド（例: `scripts/agent-gh review release list`）が止まること（受け入れ条件 3）は、未検証。フックの導入前の実測では、承認要求にならずに実行された（記録欄の「許可にも deny にも無い読み取り系でない形の結果」の行）。フックの導入後の測定では拒否されたが、`cd <repo> && …` の複合形で行ったため、拒否の対象が `release list` 自体かは切り分けられていない。単一形での再測と QA の照合を #306 で待っている

`denyWrite` の適用後の確認で、確かめ終えていない、または満たしていない項目（#306）:

- 満たしていない: 書ける一時フォルダがレビュー専用であること。`$TMPDIR` の実体の `/tmp/claude-1000` は uid ごとに 1 つで、同じユーザーのほかのプロセスと共有される
- 確かめていない: 秘密の置き場の読み取りが拒否されること。`test -r` は権限ビットしか見ないため拒否を測れず、中身を読む実測はエージェントには行わせない。人が sandbox の下で、中身を読むコマンドで確かめる
- 確かめていない: `denyWrite` の下で、実際の PR レビュー（`scripts/agent-gh review pr review` による投稿まで）が最後まで終わること

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

レビュー担当（`devin_review`・`claude_review`・`codex_review`）の sandbox から、秘密の置き場を読めないようにする。対象は 4 パス（QA 用の `~/.config/pelway-agent-qa` は、下の「QA 担当の GitHub App（pelway-qa）」の手順 5 で 5 つ目として各担当の拒否リストに足す）: `~/.config/fnox`（age 秘密鍵 `age.txt` と fnox のグローバル設定）・`~/.config/pelway-agent`（dev 用の fnox 設定。`PELWAY_DEV_*` と `PELWAY_LLM_API_KEY` が同じ age 鍵で暗号化されて同居していた）・`~/.ssh`・`~/.config/gh`。これらはすべて実体のあるディレクトリでなければならない（symlink だと devin の bwrap が exec を全滅させる）。

### hatai が手で適用する変更（リポジトリ外の設定。反映は hatai が行う）

**1. `~/.config/devin-review/config.json`** — `permissions.deny` の `Read(...)` に 2 件を追加（`~/.ssh/**`・`~/.config/gh/**` は既存）:

```json
"Read(~/.config/fnox/**)",
"Read(~/.config/pelway-agent/**)",
```

**2. `~/.claude-review/settings.json`** — `permissions.deny` に 4 件を追加（QA 用の 5 件目は下の「QA 担当の GitHub App（pelway-qa）」の手順 5 で足す）（`Read(...)` deny は `sandbox.filesystem.denyRead` にマージされ、sandboxed なコマンドにはファイルシステム層でも隠される。`allowUnsandboxedCommands: false` が既に効いているため全 Bash コマンドが対象）:

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
  "/home/hatai/.config/pelway-agent-qa",
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
| claude_review: 4 パスの `test -r`（すべて非 0 か） | 未実測。実測は #479 で追う（`claude_review` は今は動き、`/rate_limit` は下の行のとおり成功する） | — |
| codex_review: 4 パスの `test -r`（すべて非 0 か） | 合格 — 確認用の子タスクで 4 パス、続けて QA 用を含む 5 パスとも `test -r` が 1（`docs/worklogs/2026-11-21-codex-review-home.md`） | 2026-11-21 |
| devin_review: `scripts/agent-gh review api /rate_limit` 成功 | 未実測（上記の devin_review 停止中と同じ理由。実測は #479 で追う） | — |
| claude_review: `scripts/agent-gh review api /rate_limit` 成功 | 合格 — 確認用の子タスクで exit 0（`5000`）（`docs/worklogs/2026-11-21-codex-review-home.md`） | 2026-11-21 |
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
5. レビュー役の拒否リスト（Codex は `/etc/codex/requirements.toml` の `deny_read`、`~/.claude-review/settings.json`、`~/.config/devin-review/config.json`）に `~/.config/pelway-agent-qa/` を足し、レビュースレッドから秘密を読めなくする

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

### Codex（codex_review。検証時のモデルは gpt-6-luna）

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
- `pitchfork.toml` には AI 開発チームの見張り役 `agent-watch`（`crates/pelway-agent-watch`）と、worktree の片付け `agent-worktree-prune` を定義する。`agent-watch` は T3 のスレッドの状態変化を Grok Bot の webhook に通知し、同じ知らせを指揮役の受信箱に書いて、いまの指揮役のスレッドを起こす常駐プロセスで、仕様は `docs/superpowers/specs/agent-team.md`「見張り役と通知」。
- `agent-worktree-prune` は cron（毎時 0 分）で `scripts/agent-worktree-prune --auto` を走らせる。消すのは `~/.t3/worktrees/Pelway` の下の、settle されたスレッドの（またはスレッドの無い）worktree のうち、main に取り込み済みか PR がマージ済み・クローズ済みのもの。未追跡のファイルは捨て、追跡ファイルに変更が残るものは残す。生きているプロセスのカレントディレクトリ（`/proc/<pid>/cwd`）が中にある worktree と、最後の変更（作業ツリーと、git dir の `HEAD`・`index` の更新時刻のうち新しいもの）から 6 時間以内の worktree も残す。この 2 つは一覧を作るときと消す直前の両方で確かめる（消す直前は、更新時刻の走査の間に始まったプロセスも見逃さないよう、走査の後にもプロセスを確かめ直す）。`t3 thread list` に出ない `delegate_task` の子スレッドの worktree を守るためで、T3 の CLI には子スレッドを列挙する手段が無い。T3 に問い合わせられない回と `/proc` を読めない回は何も消さない。消したあとに `git worktree prune`・`git fetch --prune origin`・`mbx gc` を走らせる。pitchfork の supervisor が動いていれば、`pitchfork start` しなくても cron で動く。supervisor は各 worktree の `pitchfork.toml` の cron も動かすが、`--auto` は main の checkout 以外では何もせずに終わり、同時に 2 つは走らない（`$XDG_STATE_HOME/pelway/agent-worktree-prune.lock`）。

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

- 状態は `$XDG_STATE_HOME/pelway/agent-watch.json`（未設定なら `~/.local/state/pelway/agent-watch.json`）に置く。初めて起動したときは既存のスレッドを記録するだけで、通知しない。webhook に送れなかった通知は状態の送信待ちの列に残り、古い順に送り直す。受け手が直らない失敗（HTTP 400・404・413）を返した通知は、送り直さずに列から外して `mode=send-dropped http=<コード>` を 1 行残し、下の期限が残っていれば同じ周回で次の通知を送る（期限までに送り終えなかった通知は列に残る）。外した通知も指揮役の受信箱には書かれる（受信箱に書けなかった通知は、書けるまで状態に残して書き直す。上限 100 件で、超えたら古いものから捨てる。下の「指揮役の受信箱と起こし」）。送り直せる失敗（ほかの HTTP ステータス・接続の失敗・秘密が無い・下の期限での打ち切り）では、通知を列に残してその周回の送信をやめ、次に送るまで間隔を空ける（`outbox retry_after=<秒>s` を 1 行残す）。間隔は 30 秒から失敗のたびに倍にし、上限は 15 分、送れたら 30 秒に戻す。間隔は失敗が分かった時刻から数える。間隔を空けている間に出た通知も列の後ろに並び、時刻が来たら古い順に送る。間隔と次に送る時刻は状態ファイルに保存するので、再起動しても引き継ぐ。1 周の送信には、送信の開始から 40 秒の期限を置く。秘密を読む `fnox`（1 件につき URL と鍵の 2 回、1 回の上限 30 秒）・curl（1 回の上限 10 秒、2 秒おいて最大 2 回まで送り直す）・送り直しの待ちはすべて期限の内で行い、`fnox` と curl の制限は起動の直前に期限からの残り時間で決め、期限を過ぎたものは止めて、その通知を列に残す（`mode=send-failed reason=deadline` を 1 行残す。期限の直前に送り始めた通知の打ち切りも、送り直せる失敗として間隔を空ける）。残りが 1 ミリ秒未満なら `fnox` と curl を起動せず、次の通知も送り始めない。そのため 1 周の webhook の送信は、列の長さによらず 45 秒以内に収まる。
- 起動してよいのは本体 checkout だけ。worktree にも同じ `pitchfork.toml` の定義があるが、worktree から起動されたときは起動元（実行時のカレントディレクトリ）の `git rev-parse --git-dir` と `--git-common-dir` の差で検出して理由を 1 行ログに出して終わる。見張り対象の `--project` が起動元の checkout の toplevel と一致しないときも、本体から別の checkout を見張る起動になるため同じく理由を 1 行出して終わる。同時に動かせるのは 1 つだけで、状態ディレクトリの `agent-watch.lock`（0600）を flock で取り、取れなければ理由を 1 行出して終わる（いずれも終了コード 0。agent-worktree-prune と同じ決まり。拒否のログにはパスの種類だけを書き、中身や環境変数は出さない）。
- 指揮役の受信箱と起こし（Issue #555）: 知らせを出すたびに、webhook と同じ JSON を共有の git dir の `pelway/conductor-inbox.jsonl` に 1 行ずつ追記する（形は `docs/agents/notify-payloads.md` の「指揮役の受信箱」）。受信箱に書けなくても webhook の送信と見張りは続け、ログに 1 行残す。書けなかった知らせは状態ファイルに残し、次の周回から、その周回の新しい知らせより先に書き直す（上限 100 件。超えたら古いものから捨てる）。その項目（`inbox_pending`）が無い更新前の版の状態ファイルでは、送信待ちの列の知らせを受信箱に書けたかが分からないので、更新後の最初の周回で列の写しも書き直す（書けていた知らせは受信箱に 2 行になる）。`pelway/` は sandbox の中のエージェントも書ける場所で、agent-watch はホストの権限で動くので、`pelway/` が symlink・自分の所有でないディレクトリのとき、受信箱が symlink・FIFO などの通常ファイルでないもの・自分の所有でないもの・ハードリンクが 2 本以上あるもののときは書かない（`O_NOFOLLOW`・`O_NONBLOCK` で開き、開いた fd で種類・所有者・リンク数を確かめる。前からある受信箱に所有者以外の権限があれば書かない（0600 に直しても、すでに他人が開いている fd からは読めるため。webhook の送信は続ける）。`scripts/conductor-watch.sh` のボード Status 更新キューと同じ型）。前回の追記が途中で失敗して改行の無い断片が残っていれば、追記の前に改行で閉じ、次の知らせを独立した 1 行にする（行の形・断片の行・UTF-8・読み手の扱いは `docs/agents/notify-payloads.md` の「指揮役の受信箱」）。受信箱が 256 MiB を超えているときは、書かず、行数も数えず、その回は起こさない（`inbox append failed: inbox is too large` を 1 行残す。webhook の送信は続ける）。`patrol-state.json` は、symlink・FIFO などの種類・所有者・リンク数を同じく確かめ（権限は見ない。conductor-watch が既定の umask で書くため）、256 KiB を超えるものは読まない（`WAKE skipped reason=patrol-state-too-large`）。追記のあと、次の決まりで指揮役を起こす（判定の順は、登録ファイル → own-event → 補助記録 → 既読の位置の範囲 → スレッド ID の形 → 既読が進んだか。ログに出るのは最初に当たった理由だけ）。
  - 宛先は、ホスト側の登録ファイル `~/.config/pelway-agent-notify/conductor-thread-id`（webhook の秘密の設定と同じ置き場で、sandbox の中からは書けない）を毎回読み直す。登録は hatai がホストの端末で、インストールした見張り役で行う: 代行の指揮役を始めるときに `~/.local/bin/pelway-agent-watch --register '<スレッド ID>'`（スレッド ID は、指揮役が伝えた値をそのまま貼らず、hatai がホスト側の T3 の画面か `t3 thread list --json` の `threadId` で確かめ、値が ASCII の英数字と `:`・`_`・`-`・`.`・`%` だけで、先頭が `-` でないことを見てから、単一引用符で囲んで渡す。`--register` の文字の検査は、シェルが行を解釈した後にしか働かないため）、終えるときに `--unregister`、確かめるときに `--show-registration`（見張りは始めない）。worktree の中のスクリプトや `cargo run` は sandbox の中のエージェントが書き換えられるので、ホストでは実行しない（インストールは本体 checkout で `mise run agent-watch-install`。指揮役は本体 checkout の作業ツリーを書けない専用の worktree で立てるが、共有 git dir の `index`・`refs/` は worktree のスレッドからも書けるので、ホストで実行する前の確かめは `docs/agents/conductor-context.md` の「恒久交代」に従う）。登録する値は、受信箱の `thread_id` と同じ文字列（`t3 thread list --json` の `threadId`）にする。`--register` は文字の種類だけを確かめ、表記は確かめない。違う表記を登録すると、知らせの元が指揮役自身かの判定が一致せず、`t3 thread send` がその表記を受け付けるなら、指揮役の巡回の終わりの知らせでも起こすので「起こし → 巡回 → 起こし」が続く（受け付けなければ、起こしは毎回 `WAKE failed` になる）。そうなったら `--unregister` で止め、正しい表記で登録し直す。登録ファイルの位置がディレクトリなら登録は失敗し、それ以外（symlink・FIFO など）はたどらずに置き換える。読むときも symlink をたどらず、FIFO で止まらず、通常ファイル・自分の所有でなければ使わない（`WAKE skipped reason=conductor-file-unreadable`）。ファイルが無いときは起こさない。`patrol-state.json` に `conductor_thread_id` が書かれていても使わない。値の先頭が `-`、または英数字と `:`・`_`・`-`・`.`・`%` 以外を含む、または 256 文字を超えるときは使わない（`WAKE skipped reason=invalid-thread-id`。`--register` も同じ規則で登録を拒む）。`patrol-state.json` は既読の位置（`inbox_offset`）だけに使い、無い・読めない（書きかけ・壊れている・`inbox_offset` が 0 以上の整数でない。項目が無いときは 0 として扱う）ときはその回は起こさず `WAKE skipped reason=patrol-state-unreadable` を 1 行残す。その回の知らせの元がすべて登録された指揮役自身のスレッド（payload の `thread_id` と登録値の完全一致で見る）（指揮役の巡回の終わりなど）のときも起こさない（`WAKE skipped reason=own-event`。起こすと、巡回の終わりと起こしを繰り返すため）。
  - 前回起こしたときの指揮役の既読の位置（`patrol-state.json` の `inbox_offset`）を状態ファイル（`last_wake_inbox_offset`）に持ち、既読の位置がそこから進んでいなければ重ねて起こさない（`WAKE skipped reason=not-yet-read`）。1 行でも読み進めていれば、次の知らせで起こす。基準は起こした宛先と組で持ち、登録が替わったら一度も起こしていない扱いにする。既読の位置が受信箱の行数を超えているとき（書き損じ・書き換え）は、その回は起こさず基準にも残さない（`WAKE skipped reason=offset-out-of-range`）。
  - 送るのは決まった文だけで（`t3 thread send <スレッド> -`、本文は標準入力。知らせの中身は含めない）、実行中のスレッドには次のターンとして届く。一覧・詳細・起こしに使う `t3` は同じもので、`T3_BIN` が指定されていればそれ、無ければ動いている T3 サーバーと同じ実行ファイル、それも無ければ PATH の `t3`（PATH の `t3` は版がサーバーと食い違うと失敗し、`ServerOwnsDatabaseError` で拒まれることもあるため）。サーバーの実行ファイルとして受け入れるのは、`/proc/<pid>/exe` の向き先が `$HOME/.t3/wsl-runtime/` の下の `t3` で、自分の所有の通常ファイルのものだけ（`scripts/conductor-watch.sh` の部分一致の規則より狭い。agent-watch はホストの権限で実行するので、sandbox から書ける場所に置かれた偽物を選ばないため）。
  - `run`・`wake` を通る外部コマンド（`t3`・`gh`・`git`・`hostname`）は `timeout -k 5 30` で包む（30 秒で SIGTERM、さらに 5 秒で SIGKILL。終了処理で固まった子で見張りが止まらないため）。webhook の送信の `fnox` と curl はこれを通らず、`timeout -s KILL` で包む。制限は上の状態の項の 1 周の送信の期限までの残り時間（`fnox` は 30 秒との短い方）で、過ぎたら猶予なしに SIGKILL で止める（curl は加えて `--connect-timeout 2 --max-time 10` で区切る）。
  - 送れなかったら `WAKE failed <理由>`（理由は `exit=<終了コード>`・`exit=signal`・`spawn:`・`stdin:`・`wait:` のどれか）を 1 行残すだけで送り直さず、次の新しい知らせで改めて試みる。知らせは受信箱に残り、指揮役の巡回が読む。
  - `patrol-state.json` は sandbox の中のエージェントが書けるので、`inbox_offset` を書き換えれば重ねての起こしの抑止は外れる。逆に、受信箱の行数以内で実際より先の値に書き換えると、起こしたときにその値が基準に残り、補助記録を正しい値に戻しても、既読の位置がその値を超えるまで起こしは送られない（知らせは受信箱に残り、定期巡回が読む）。その場合も送られるのは、決まった文を、hatai が登録した 1 つのスレッドへ、知らせ 1 件につき最大 1 回だけである。
- 送らずに試すには `cargo run -p pelway-agent-watch -- --once --dry-run`（新しいイベントの payload を標準出力に出す。状態ファイルは読むだけで書き換えず、受信箱にも書かず、起こしも送らない）。
- 再起動のあとの自動起動: `pitchfork.toml` の `agent-watch` には `boot_start = true` を付けてあり、login 時に起動する pitchfork の supervisor（`pitchfork supervisor run --boot`）が agent-watch を起動する。起動時の supervisor は作業ディレクトリを持たず、読むのはユーザーの設定（`~/.config/pitchfork/config.toml`）と、その `[namespaces]` に登録されたプロジェクト（とその全 worktree）の `pitchfork.toml` だけなので、次の 3 つを PC 側で入れておく。3 つともリポジトリの外の設定で、hatai が OK のうえで手で行う。
  1. supervisor の login 時の起動を登録する: `pitchfork boot enable`（sudo なし。`~/.config/systemd/user/pitchfork.service` ができる）。`pitchfork boot status` が `Boot start is enabled` になれば済み。
  2. 本体 checkout を namespace として登録する: `~/.config/pitchfork/config.toml` に次を書く（`dir` は本体 checkout の絶対パス。`git worktree list` の 1 行目）。

     ```toml
     [namespaces.pelway]
     dir = "/home/<ユーザー>/repos/hatai/Pelway"
     ```

  3. supervisor の PATH を足す: login 時に起動する supervisor（`~/.config/systemd/user/pitchfork.service`）の PATH には `git`・`gh`（`/home/linuxbrew/.linuxbrew/bin`）と `t3`（`~/.local/bin`）が無く、agent-watch は `git rev-parse: exit 127` で止まり、`agent-worktree-prune` も `git: command not found` で失敗する（[調査報告](worklogs/bugs/agent-watch-not-running/report.md)の根本原因の 4）。drop-in `~/.config/systemd/user/pitchfork.service.d/path.conf` を次の中身で作る。systemd の `Environment=` は `~` も `$HOME` も展開しないので、ホームは指定子 `%h` で書く。

     ```ini
     [Service]
     Environment=PATH=%h/.local/bin:%h/.local/share/mise/shims:/home/linuxbrew/.linuxbrew/bin:/usr/local/bin:/usr/bin:/bin
     ```

     `systemctl --user daemon-reload` のあと `systemctl --user restart pitchfork.service` で適用する。入ったかの確かめ方: `systemctl --user show pitchfork.service -p Environment` の出力に、上の PATH（`%h` がホームのパスに展開されたもの）が出る。

  - 起動時に読まれるのは、本体 checkout にいまチェックアウトされているブランチの `pitchfork.toml` なので、本体 checkout は `boot_start = true` の入った main に置いておく。
  - 起動時の supervisor は各 worktree の同じ定義からも agent-watch の起動を試みるが、上の決まりで worktree からの起動は理由を 1 行出して終了コード 0 で終わる（pitchfork の再起動は起きない）。動き続けるのは本体 checkout の 1 つだけ。
  - 入ったかの確かめ方（WSL の再起動のあと、手を触れずに、本体 checkout の直下で実行する）: `pitchfork supervisor status` で supervisor が動いている、`pitchfork status agent-watch` が running、`pitchfork logs agent-watch` に周回のログが出ている。worktree 側の起動の拒否が 1 行ずつ出て終わっていることも、同じくログで確かめる。
  - 自動起動だけを止める: 本体 checkout の直下で `pitchfork disable agent-watch`（disabled の daemon は起動時にも起動されない。戻すときは `pitchfork enable agent-watch`）。supervisor ごと止めるときは `pitchfork boot disable`。
  - 止まっていたときの戻し方: 状態ファイル（`$XDG_STATE_HOME/pelway/agent-watch.json`、未設定なら `~/.local/state/pelway/agent-watch.json`）は消さず、`agent-watch.json.bak-<日付>` に名前を変えてから、本体 checkout で `pitchfork start agent-watch` する（状態が無い初回の起動は既存のスレッドを記録するだけで、止まっていた間の古い完了をまとめて送らない）。状態ファイルを動かすのは hatai の OK があるときだけ。
- v0.7 の05章が想定する起動単位（postgres・server・runner など）は、実装された時点で loopback 限定の定義を `pitchfork.toml` に足す。

## 既知の事項・未解決

1. **WSL の `/` shared mount 警告**: podman の rootless 実行時に出ることがある。smoke には影響しないが、bind mount を使う場面への影響は未確認。実使用時に検証する。
2. `docs/architecture-v0.7.md` の実行ビット（100755）は意図的にそのまま（凍結文書のため変更禁止）。
3. macOS（Apple container）の手順は未検証。Intel Mac は対象外。
4. `mise activate`（シェル統合）は `~/.bashrc` 等の変更が必要なため設定していない。必要なら利用者が各自で設定する（[シェル設定](https://mise.jdx.dev/shell-setup.html)）。それまでは `mise exec --` を使う。
