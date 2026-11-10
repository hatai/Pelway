# 作業ログ: PR のコード品質チェック workflow（platform-pr-ci-r1）

- 日付: 2026-11-10（火）JST
- 担当: Pelway 開発基盤エンジニア（T3 Code / OpenCode + zai-coding-plan/glm-5.3）
- worktree: `~/.t3/worktrees/Pelway/demo-t3-platform-pr-ci-r1`、ブランチ `demo-t3/platform-pr-ci-r1`
- 基点: ローカル `main` = `7c7a35a043f32ebbb4455781ee0b4b36c822e8a0`（GitHub 上の main も同 sha、2026-11-09 23:31 JST。差分なし）

## 目的

`main` 向け Pull Request で、ローカルの公式確認と同じ `mise run verify`（fmt-check / check / clippy / test / build / toolchain-check / hk-validate / hk-check / fnox-check / pitchfork-check）を GitHub Actions で回す。

## 範囲

- 対象: `.github/workflows/verify.yml` の新規作成、`docs/dev-environment.md` への短い説明追加、この作業ログ。
- 対象外: 製品クレートの実装、notify プラグイン、OpenCode 設定、既存の GDPR / UI ブランチ、`docs/technical/v0.7/` と `docs/architecture-v0.7.md`（凍結、不変更）、U01〜U07。
- このラウンドでは push・PR 作成をしない。GitHub 上の実際の run は未検証のまま。

## 成果物

1. `.github/workflows/` — `verify.yml`（PR base=main + `workflow_dispatch`、job は `mise install` → `mise run verify` のみ）
2. `docs/dev-environment.md` — 「CI（GitHub Actions）」節を追加
3. `docs/worklogs/2026-11-10-platform-pr-ci-r1.md` — このログ

## 先行事例の比較（Prior art）

制約上、実装エージェントからは GitHub 検索・Web 参照を行わない（ネット接続禁止）。 therefore 既知の公式パターンで比較し、オンライン再確認が必要な項目は下記「未検証」に列挙した。

| 案 | 内容 | 採否 |
|---|---|---|
| A. `jdx/mise-action`（mise 公式 Action） | checkout 後 `mise-action` で mise 導入 + `mise install`。CI 向けの標準経路 | 不採用（今回は不可）。コミット sha でのピン留めが指示要件だが、オフラインで妥当な sha を検証できず、誤 sha は run を確実に壊す。浮動タグは指示違反 |
| B. 公式スタンドアロンインストーラ（mise.run）+ バージョン断言 | `curl -fsSL https://mise.run \| sh` を `MISE_VERSION` 固定で実行し、インストール後にバージョン不一致で即 fail | **採用**。第三者 Action は `actions/checkout` だけで済む。インストーラがバージョン指定を無視しても断言 step で即座に赤くなる（静かに誤バージョンで走り続けることはない） |
| C. Rust 向け定番 Actions（rust-toolchain 系 + rust-cache 系） | mise を使わず rustc/clippy を直接導入 | 却下。ツールバージョンが `mise.toml` / `mise.lock` の単一管理元から外れる。かつ rust-cache 系はリモートキャッシュで方針違反 |

## ピン留めと出典

| 項目 | 値 | 出典・確認方法 |
|---|---|---|
| `actions/checkout` | `11bd71901bbe5b1630ceea73d27597364c9af683`（v4.2.2） | 広く流布している v4.2.2 のピン値。**オフラインのため tag↔sha 対応を再検証できていない**（QA がオンラインで要確認） |
| mise 本体 | `2026.11.5`（`MISE_VERSION: v2026.11.5`） | 本 worktree と同一ホストで `mise --version` = `2026.11.5 linux-x64 (2026-11-09)` を確認済み。`min_version` 2026.9.2 を満たす |
| ツール一式 | `mise.toml` / `mise.lock`（Rust 1.96.0 + rustfmt/clippy、mr-boxington 1.29.2、hk 2.11.1、fnox 1.47.0、pitchfork 2.33.1） | `mise install` が lockfile の checksum・署名付きで導入。workflow 側ではバージョンを書かない |
| インストーラ URL | `https://mise.run` | mise 公式のスタンドアロン導入経路 |

## 検証コマンドと結果（自己検証・この worktree）

| コマンド | 結果 |
|---|---|
| `mise trust` | exit 0（"No untrusted config or environment files found."） |
| `mise run verify` | **exit 0**。全依存タスク（10個）成功。test は `dev_environment_smoke` 1件 pass、mbx は `0 B downloaded, 0 B uploaded`（ローカルキャッシュのみ） |
| `python3 -c "import yaml; yaml.safe_load(...)"`（verify.yml 構文確認） | OK。triggers = `pull_request.branches=[main]` + `workflow_dispatch`（pyyaml の YAML 1.1 で `on:` が true キーに見えるのは既知の挙動） |
| `sha256sum .opencode/plugins/pelway-agent-notify.ts` | `7ca356833d1e170f529522488e585c5d877458524441f4595107be657390ea80`（VERSION 3.5。比較基準 sha は指示になし→記録のみ） |

自己報告であり、上記は「未検証」として扱うこと（特に GitHub 上の run）。

## 守った制約

- ランナーは GitHub-hosted `ubuntu-24.04`（ubuntu-latest を使わず固定）。
- リモートキャッシュなし: `actions/cache` 不使用、mise-action 不使用（キャッシュ機能持ちのため）、mbx はローカルキャッシュのみ（検証ログで `0 B downloaded/uploaded` を確認）。
- CI で `mise bootstrap`・apt・podman・sudo なし。`mise install` と `mise run verify` のみ。
- 秘密: workflow は `permissions: contents: read` のみ。`fnox-check` は `fnox list`（定義一覧）だけ。ログ・ファイルに秘密を書いていない。
- 凍結文書（`docs/technical/v0.7/`、`docs/architecture-v0.7.md`）は未変更。
- 実装エージェントからのネット接続・git push / fetch は一切実行していない。

## 迷った点と安全側の判断

1. **mise-action の sha ピン留めがオフラインで検証不能** → 案 B に切り替え、インストール後のバージョン断言で「静かなバージョンずれ」を構造的に排除した。`MISE_VERSION` の接頭辞（`v` の有無）もオフライン検証できなかったため、断言失敗時は即 fail になる設計で許容した。
2. **`push` を main に足すか** → 同じ検査に限る条件付きで許可されていたが、r1 は最小構成（PR + `workflow_dispatch`）にした。main 直接 push 運用の実態が固まってから足す。
3. **キャッシュ全般** → 指示は mbx / Cargo のリモートキャッシュ禁止だが、mise ツールキャッシュ等も含め「リモートになるものは全部 off」の安全側を採った。CI の実行時間増は許容。
4. **作業ログの場所** → `docs/planning/` は製品計画専用でワークログの慣例がないため、新しく `docs/worklogs/` を切った。
5. **通知プラグイン** → 指示に比較基準 sha が添付されていなかったため、sha を記録して報告のみ（差し替え・指示送信はなし）。
6. **timeout** → 45分。skeleton workspace では過剰だが、依存追加後の初回ビルドを見込んで余裕を持たせた。

## 使った skills

- リポジトリ導入済み `.agents/skills/`（51個: Rust 系列 m01〜m15 / domain-* / penpot-* 等）を確認したが、CI workflow の YAML 作成に該当するものはなく不使用。
- ハーネス側の `verification-before-completion` を最終確認フェーズで使用（検証コマンドの実流出力を確認してから完了宣言）。
- サブエージェント・delegate は不使用（本作業は単独で完結）。

## 未検証（次ラウンド以降）

1. GitHub 上の実際の workflow run（push・PR の許可待ち。QA または基盤が確認）。
2. `actions/checkout` の tag↔sha 対応のオンライン再確認。
3. mise.run インストーラの `MISE_VERSION` 解釈（`v` 接頭辞を含む）の実機確認。
4. GitHub ランナー上での hk 初回実行時の Pkl パッケージ取得（docs/dev-environment.md 記載のとおり初回のみネットワーク使用）。

## 次の一手

リードがブランチを push して PR を作成 → workflow の初回 run を確認（上記「未検証」を潰す）→ 必要なら mise-action の sha ピンへの移行を検討。
