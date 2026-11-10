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
| [pitchfork](https://pitchfork.jdx.dev) | ローカル開発用プロセスマネージャー | 採用するが設定ファイルは置かない。動かすプロセス（server・runner・DB 等）が存在しないため。ツールのみ mise で固定 |

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
- rust のツール定義には `components = ["rustfmt", "clippy"]` を明示する。成分の存在は `mise run toolchain-check` で検査する。
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
| `mise run toolchain-check` | rustc / cargo / rustfmt / clippy の存在とバージョン確認（rustfmt・clippy を含まない最小構成だと失敗する） |
| `mise run hk-validate` / `hk-check` | `hk.pkl` の検証 / リポジトリ全体の読み取り専用検査 |
| `mise run fnox-check` | `fnox list`（定義の一覧確認。秘密値の取得・表示はしない） |
| `mise run pitchfork-check` | pitchfork のバージョン確認と、`pitchfork.toml` が未配置であることの確認 |
| `mise run verify` | 上記すべての一括確認（依存タスクは**並列**実行） |
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

- `.github/workflows/verify.yml` が、`main` 向け Pull Request と手動実行（`workflow_dispatch`）で、ローカルと同じ `mise run verify` を GitHub-hosted runner（`ubuntu-24.04`）で実行する。
- CI で実行するのはツール導入（`mise install --locked` 相当）と `mise run verify`（一括確認）に加え、クライアントのビルドに必要な Linux システムパッケージの導入である。verify / coverage の両ジョブで `apt-get` により `libfontconfig1-dev` を入れる（Ubuntu 24.04 では `libfontconfig-dev` を引く移行パッケージ。`fontconfig.pc` を提供する）。`mise bootstrap`（apt:podman・sudo）は使わない。ツールのバージョンは `mise.toml` / `mise.lock` に従う。
- mise の導入は公式 Action（`jdx/mise-action`）で行う。`version: v2026.11.5`・`install_args: --locked`・`cache: false` を指定し、導入後にバージョンを検証している。mise-action のツールキャッシュは復元内容を `mise.lock` と突き合わせない（公式の注意）ため無効化し、ダウンロード用のキャッシュも足さない。Action 参照の sha 固定には GitHub 公式 lockfile（`.github/workflows/actions.lock`、担当が `gh actions-lock` で生成）を使う。workflow 側の参照は `actions/checkout@v4.2.2` と `jdx/mise-action@v2.4.4`。lockfile 上のコミットはそれぞれ `11bd71901bbe5b1630ceea73d27597364c9af683` と `c37c93293d6b742fc901e1406b8f764f6fb19dac`で、どちらも公式タグが指すコミットと一致する。
- リモートキャッシュ（GitHub Actions cache・S3 等）はローカルの mbx と同じ方針で設定しない。CI のビルドはランナー内のローカルキャッシュのみで完結する。
- この workflow に秘密は不要（`permissions: contents: read`）。`fnox-check` は `fnox list`（定義一覧の確認）のみで、秘密値の取得・表示はしない。
- カバレッジの増減計測は同じ workflow の `coverage` ジョブで行う（品質ゲートとは独立）。cargo-llvm-cov（バージョン指定・`--no-cfg-coverage` 付き。`cfg(coverage)` で nightly 専用 `#![feature]` を有効にする依存があると stable で E0554 になるため）で LCOV を `lcov.info` に出し、`.octocov.yml` の `comment:`（`if: is_pull_request`、`updatePrevious: true`）に従って octocov が PR へカバレッジコメントを投稿する。PR 本文への挿入（`body:`）は使わない。octocov は `mise.toml` の `[tools]` で `"aqua:k1LoW/octocov" = "0.83.5"` と宣言し、`mise.lock` が URL と checksum で取得物を固定する（CI は `mise exec -- octocov`）。比較用レポートの保存は GitHub Actions の artifact のみ。rust にはカバレッジ計測用の `llvm-tools-preview` 成分を `mise.toml` の `components` で宣言する。`toolchain-check` が sysroot 上の `llvm-profdata` / `llvm-cov` の存在を確認し、欠けていれば失敗する（暗黙の rustup 自動ダウンロードには頼らない）。CI の verify / coverage 両ジョブは `mise install` のあと、この検査を先に実行する。
- workflow の静的検査は jactionlint（`mise.toml` で `2.5.0` をピン）を CI の mise-action のあとに実行する。jactionlint は actions.lock による sha 固定を受け付ける。lock 生成後、暫定の inline ignore は撤去済み。
- `.github/workflows/mise-bump.yml` が、毎週月曜（UTC）と手動実行で、`mise upgrade --bump --no-global --minimum-release-age 7d --yes` により `[tools]` のバージョンを更新し、`chore/mise-tools-bump` ブランチのローリング PR を1本作る・更新する。dependabot は mise.toml を解釈できないため mise 自身に更新させる（rust の inline table を含む mise.toml と mise.lock を一貫して書き換える）。`--minimum-release-age 7d` で公開直後の版を除外する。mise 本体（`min_version`・mise-action の `version` ピン）は対象外。PR 作成は `GITHUB_TOKEN` + `gh` CLI で、専用 action は使わない。workflow レベルの `permissions` は空で、bump ジョブにだけ `contents: write` / `pull-requests: write` を付与する。GITHUB_TOKEN 製 PR の `pull_request` run は承認待ちになるため、`verify` が自動で走らない場合は PR の checks から `Approve workflows to run` を一度押す。
- 2026-11-10 時点で GitHub 上での実際の run は未確認（ブランチの push・PR 作成の承認待ち。作業ログ: `docs/worklogs/2026-11-10-platform-pr-ci-r1.md`）。

## GitHub 通知（Grok Bot webhook・notify-grokbot）

`.github/workflows/notify-grokbot.yml` が、PR 系イベント（pull_request / review / review comment）と verify workflow の完了（`workflow_run`）、手動実行（`workflow_dispatch`）を Grok Bot の webhook に POST する。送るのはメタデータのみで、PR タイトル・本文、コメント本文、レビュー本文、ラベル、メールアドレス等は送らない。

- Secret（リポジトリの Settings → Secrets and variables → Actions → New repository secret で登録。値はドキュメント・コードに置かない）:
  - `GROKBOT_WEBHOOK_URL` — webhook の URL
  - `GROKBOT_WEBHOOK_AUTH` — `Authorization` ヘッダーの値全体
- 送る JSON のキーは 13 個固定: `schema`（`"pelway.github-notify/v1"`）, `repo`, `event`, `action`, `pr_number`, `head_branch`, `head_sha`, `merged`, `conclusion`, `workflow_name`, `run_url`, `actor`, `occurred_at`。値の無いものは `null`。
- 送らない条件（通知をスキップし、ジョブは成功で終わる。CI は落ちない）:
  - 上記 Secret のどちらかが未設定のとき
  - fork 由来のイベント（PR 系は head リポジトリ、workflow_run は head_repository の full_name がこのリポジトリと違うとき）
  - コメント系イベント（pull_request_review / pull_request_review_comment）で、投稿者の `author_association` が `OWNER` / `MEMBER` / `COLLABORATOR` のいずれでもないとき（ボット例外なし。octocov 等の結果は verify の `workflow_run` 通知で把握する）
- 対象外: PR の会話欄に書いた普通のコメント（issue_comment）は通知しない。fork から来た PR へのコメントを安全に見分けられないため。あとで足す場合は、`pull-requests: read` で PR を調べて fork を除く形にし、別の PR で行う。
- 手動実行（workflow_dispatch）は `event: "test"` のテスト通知を 1 通送る。送信の失敗（タイムアウト・HTTP エラー）で CI は落ちない。

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
- リモートキャッシュ（GitHub Actions cache・S3 等）は設定しない。

## pitchfork について（採用済み・設定保留）

- ツールは mise で 2.33.1 に固定する。loopback 限定の smoke（HTTP サーバーの起動・readiness・ログ・停止）で動作を確認済み。
- v0.7 の05章が想定する起動単位（postgres・server・runner・integration gateway・sandbox controller・stub inference・dummy mail・task sandbox）が実装された時点で、loopback 限定の `pitchfork.toml` を追加する。動かない設定だけを残さないため、実装前に置かない。`mise run pitchfork-check` が未配置を検査する。

## 既知の事項・未解決

1. **WSL の `/` shared mount 警告**: podman の rootless 実行時に出ることがある。smoke には影響しないが、bind mount を使う場面への影響は未確認。実使用時に検証する。
2. `docs/architecture-v0.7.md` の実行ビット（100755）は意図的にそのまま（凍結文書のため変更禁止）。
3. macOS（Apple container）の手順は未検証。Intel Mac は対象外。
4. `mise activate`（シェル統合）は `~/.bashrc` 等の変更が必要なため設定していない。必要なら利用者が各自で設定する（[シェル設定](https://mise.jdx.dev/shell-setup.html)）。それまでは `mise exec --` を使う。
