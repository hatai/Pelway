# ローカル開発環境の構築と手順

- 作成: 2026-11-09（同日レビュー反映版）。対象は Windows 11 + WSL2（Ubuntu 24.04、systemd 有効）上のこのリポジトリ。
- 位置づけ: **開発環境の設定と手順の記録**。製品機能の実装ではない。`crates/dev-smoke` とルートの `Cargo.toml` はビルド・試験の経路確認用スケルトンであり、製品コードではない。
- 上位の設計は [技術仕様 統合版 v0.7 の05章](technical/v0.7/body/05-local-dev.md)（backend に WSL2 内ランタイム、macOS に Apple container を想定した構成案。同文書は実行承認前の設計であり、実施報告ではない）。

## ツール構成と採否

| ツール | 役割 | このリポジトリでの採否 |
|---|---|---|
| [mise](https://mise.jdx.dev) | 言語・ツール・環境変数の管理 | 採用。`mise.toml` がバージョンの単一管理元。グローバル設定（`mise use -g`）は使わない |
| Rust（rustc / cargo / rustfmt / clippy） | 言語とビルド | 採用。mise 経由で 1.96.0 を固定し、`components` に rustfmt・clippy を明示する。`rust-toolchain.toml` は置かない（バージョン宣言を mise.toml に一本化するため） |
| [mr boxington](https://mr-boxington.jdx.dev)（`mbx`） | Cargo ビルドの共有キャッシュ | 採用。mise の `mr_boxington = true` オプションで cargo をラップ（ローカルキャッシュのみ。リモートキャッシュは設定しない） |
| [hk](https://hk.jdx.dev) | git hook の管理 | 採用。`hk.pkl` に手順を定義し、フックはリポジトリローカルの Git 設定にインストール |
| [fnox](https://fnox.jdx.dev) | シークレットの管理 | 採用。`fnox.toml` にはシークレットの名前と秘密を含まないテンプレートだけを置く |
| [pitchfork](https://pitchfork.jdx.dev) | ローカル開発用プロセスマネージャー | 採用するが**現時点では設定ファイルを置かない**。動かすプロセス（server・runner・DB 等）がまだ無いため。ツールのみ mise で固定 |

## セットアップ手順（初回）

前提: mise 2026.9.2 以上が入っていること（`mise.toml` の `min_version` が未満のバージョンを拒否する）。インストールは [公式手順](https://mise.jdx.dev/installing-mise.html) に従う。

```bash
cd ~/repos/hatai/Pelway

# 1. このリポジトリの mise 設定を信頼する（グローバル設定は変更しない）
mise trust

# 2. ツール一式をユーザー領域に入れる（mise.toml のバージョンに固定）
mise install

# 3. git hook をこのリポジトリだけにインストール（--mise で mise 経由で起動）
mise exec -- hk install --mise

# 4. 環境の一括確認（後述の verify タスク）
mise run verify
```

- `mise install` と `mise.lock` の生成は `mise.toml` の `[settings] lockfile = true` でこのプロジェクト自身が指定する（ユーザーのグローバル設定に依存しない）。`mise.lock` と `Cargo.lock` はコミット対象。
- rust のツール定義には `components = ["rustfmt", "clippy"]` を明示している。この環境では既存の 1.96.0 をそのまま使い、再インストールは行っていない（`mise install --dry-run` の「all tools are installed」、および `mise lock` で mise.lock に `components = "clippy,rustfmt"` が記録されることで、mise 2026.11.4 がこの指定を解釈することを確認。いずれも導入なし）。成分の存在は `mise run toolchain-check` で検査する。
- 補足: フックはリポジトリローカルの Git 設定（`hook.hk-pre-commit.command` 等）へ書き込まれる。設定ベースフックに **Git 2.54 以上**が必要とするのは hk 公式ドキュメントの記載（2026-11-09取得）。この環境の Git 2.56.0 では設定への書き込みと `hk run` による手動実行を確認済み。`git commit` 経由での発火はコミットを作っていないため**未確認**。初回の `hk` 実行は `hk.pkl` の `amends` URL から GitHub 経由で Pkl パッケージを取得するため**ネットワークが必要**（2回目以降はキャッシュ）。
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

- `git commit`（pre-commit）: **品質ゲート**。fix を実行せず、check コマンドの失敗でコミットを阻止する（trailing whitespace / 最終改行 / `cargo fmt --check` / `cargo clippy -D warnings` / `cargo check --locked`、いずれもワークスペース全体）。コードの自動書き換えは行わない。
- `git push`（pre-push）: `cargo test --workspace --locked` を実行する。
- 検査に失敗したら `mise run fmt` や `mise exec -- hk fix` で直してから再度コミットする。
- 一時的に無効化するには `HK=0` を付ける（例: `HK=0 git commit ...`）。hk の既定動作。

## シークレットの扱い（fnox）

- `fnox.toml` には**シークレットの名前と、秘密を含まないローカル開発用の既定値だけ**を置く。実際の秘密値（`value`）をこのファイルへ書かないこと。
- 既定は **fail-closed**（`if_missing = "error"`）: 値を解決できないシークレットがあれば `fnox exec` はエラーで停止する。値の登録を当分必須にしないものだけ、その秘密個別に `if_missing = "ignore"` を指定する。
- 現在の宣言:
  - `PELWAY_DATABASE_URL` — ローカル開発用の接続先（loopback・パスワードなしのテンプレート）。接続文字列は将来パスワード等を含めうるので、非秘密の既定値も fnox 側で管理する。
  - `PELWAY_LLM_API_KEY` — 名前だけの宣言（値なし・当面未使用のため個別 ignore）。
- 秘密にならない既定値（例: `PELWAY_BIND_ADDR=127.0.0.1:0`）は mise の `[env]` 側に置く。
- 実行時の注入: `mise exec -- fnox exec -- <コマンド>` で環境変数として渡る。
- 本物の値を登録するときは、各自が暗号化プロバイダ（age 等）を設定した上で `fnox set <名前>` を使う。暗号文は `fnox set` が `fnox.toml` へ書き込む設計のため、**運用方針としてチームの合意を先に得ること**。
- 個人用の上書きは `fnox.local.toml`（`.gitignore` 済み）。秘密値を含みうるためコミットしない。

## コンテナ（Docker を使わない）

方針: Docker Desktop・Docker デーモンには依存しない。Windows は WSL2 内の Linux ランタイム、macOS は Apple container を使う。

### この WSL の確認結果（2026-11-09、このマシンでの観測例。他の環境では異なりうる）

- 非 Docker 系のランタイム（podman・nerdctl・containerd・crun/runc・systemd-nspawn・proot）は**いずれも未導入**（`command -v` で確認）。
- `bwrap`（bubblewrap）はあるが、これはサンドボックス用ツールでありコンテナランタイムではない。
- 非特権ユーザーネームスペースは有効（`unshare -Ur /bin/true` が成功、`max_user_namespaces=159629`）。このため rootless Podman を導入すれば動作できる素地はある。
- **mise 経由の導入は不可（2026-11-09確認）**: mise レジストリの podman（`github:podman-container-tools/podman`）を Linux にインストールすると、配置されるのは `podman-remote`（接続先の podman サービスまたは VM を前提とするクライアント単体）のみで、ローカルでコンテナを起動するエンジン本体は配布されていない。加えて rootless 実行に必要な部品（crun・fuse-overlayfs・pasta/slirp4netns は mise レジストリに未登録、`newuidmap` は setuid バイナリのためユーザー空間では導入・有効化できない）も揃えられない。nerdctl はレジストリにあるが containerd デーモンと同じ rootless 部品群が必要で同じ結論。
- **ブロッカー**: podman の導入にはシステム変更（sudo）が必要。本環境構築ではシステム変更が禁止のため実施していない。mise の bootstrap（`[bootstrap.packages]` の `apt:podman`）も導入手段として使えるが、mise 公式ドキュメント（apt のページ、2026-11-09取得）は「root でない実行時は sudo を使う」と明記し、実際の dry-run（2026-11-09、リポジトリ外の実験設定で実施）は `sudo env DEBIAN_FRONTEND=noninteractive apt-get install -y -- podman` を予定とした。つまり bootstrap は手順の宣言化であって sudo・システム変更の回避手段ではない。承認後は次で導入する:

```toml
# 承認後に mise.toml へ追加する宣言（追加承認が必要・今回未実施）
[bootstrap.packages]
"apt:podman" = "latest"
```

```bash
mise bootstrap packages apply --manager apt --dry-run   # 予定の確認
mise bootstrap packages apply --manager apt             # 適用（sudo のパスワード入力あり）
podman run --rm quay.io/podman/hello                    # hello 程度の smoke
```

### macOS（Apple container）— 未検証

この PC では検証できないため手順のみ記す。Apple silicon + macOS 26 以降が条件（[apple/container](https://github.com/apple/container) の公式 README に基づく）。**Intel Mac は Apple container・mbx とも対象外**のため、本構成の対象外（代替は v0.7 05章のとおり別審査の Podman machine）。

```bash
# GitHub Releases から署名済みインストーラを入手し、指示に従ってインストール（管理者パスワードが必要）
container system start
container run --rm alpine echo hello   # hello 程度の smoke
```

## mr boxington（mbx）について

- `mise.toml` の `rust = { version = "1.96.0", mr_boxington = true }` と `mr-boxington = "1.29.2"` の組で、mise 経由の cargo が mbx を通る（2026-11-09の本環境での検証では、`command -v cargo` が `~/.local/share/mise/command-wrappers/bin/cargo` に解決され、ビルド出力に `mbx[cache]: ...` 行が出ることで確認）。
- キャッシュは `~/.cache/mbx`（ローカルのみ、自動GCあり。2026-11-09確認時は予算50GiB）。`target` は `~/.cache/mbx/targets/...` への symlink になる（`.gitignore` 済み）。
- 2026-11-09の本環境での観測: `mbx doctor` は「Cargo shim is not installed」と警告した。同じ時点で `command -v cargo` は mise のラッパーに解決され、ビルド出力に `mbx[cache]:` 行が出ているため、cargo のラップ自体は機能している。警告の理由について、mbx 公式ドキュメント（Getting started、2026-11-09取得）は、mise ネイティブ統合では doctor の setup 検査が shim 警告を出しうると説明している。
- リモートキャッシュ（GitHub Actions cache・S3 等）は今回設定しない。

## pitchfork について（採用済み・設定保留）

- ツールは mise で 2.33.1 に固定する。2026-11-09にリポジトリ外（`/tmp`）で実施した loopback 限定の smoke（HTTP サーバーの起動・readiness・ログ・停止）で動作を確認した。
- v0.7 の05章が想定する起動単位（postgres・server・runner・integration gateway・sandbox controller・stub inference・dummy mail・task sandbox）が実装された時点で、loopback 限定の `pitchfork.toml` を追加する。現時点で置くと動かない設定だけが残るため置かない。`mise run pitchfork-check` が未配置を検査する。

## 既知の事項・未解決

1. **コンテナランタイムの導入は追加承認が必要**（上記ブロッカー）。
2. `docs/architecture-v0.7.md` の実行ビット（100755）は今回修正していない（変更禁止のため）。報告のみ。
3. macOS（Apple container）の手順は未検証。Intel Mac は対象外。
4. `mise activate`（シェル統合）は `~/.bashrc` 等の変更が必要なため設定していない。必要なら利用者が各自で設定する（[シェル設定](https://mise.jdx.dev/shell-setup.html)）。それまでは `mise exec --` を使う。
