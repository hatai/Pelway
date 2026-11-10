# 2026-11-10 platform-coverage-no-cfg-r1

- 開始: 2026-11-10 21:00 JST
- 終了: 2026-11-10 21:04 JST（コミット e036f5d の時刻 21:02:28。作業ログのコミットはこの後）
- ブランチ: `demo-t3/platform-coverage-no-cfg-r1`
- worktree: `/home/hatai/.t3/worktrees/Pelway/demo-t3-platform-covera-3ff70be329dde120`
- 親: `f8ee838`（main。#66 のマージコミット）
- 依頼: coverage ジョブの `cargo llvm-cov` に `--no-cfg-coverage` を付ける（#59・#62 が待っている直し）

## 目的

`cargo llvm-cov` は既定で `--cfg coverage` を付けてビルドする。`cfg(coverage)` で nightly 専用 `#![feature]` を有効にする依存があると、stable ツールチェーン（この repo は 1.96.0）で E0554 になり coverage ジョブが落ちる。`--no-cfg-coverage` で計測ビルドから `cfg(coverage)` を外し、stable のまま通す。

## 変更

- `.github/workflows/verify.yml`（coverage ジョブ「Generate LCOV (cargo llvm-cov)」ステップのみ）:
  - `run:` 行を `cargo llvm-cov --workspace --locked --lcov --output-path lcov.info` から
    `cargo llvm-cov --workspace --locked --no-cfg-coverage --lcov --output-path lcov.info` に変更。
  - 直前のコメントに理由を 2 行追加（stable で `cfg(coverage)` による E0554 を避けるため）。
  - ほかのステップ・トリガー・permissions は変更していない。
- `docs/dev-environment.md`（CI 節の coverage の箇条書き）:
  - 「cargo-llvm-cov（バージョン指定）」を「cargo-llvm-cov（バージョン指定・`--no-cfg-coverage` 付き。`cfg(coverage)` で nightly 専用 `#![feature]` を有効にする依存があると stable で E0554 になるため）」に修正（同じ理由の 1 行）。
- 製品コードは変更していない。

## 検証（コマンドと exit code）

- `mise exec -- jactionlint .github/workflows/verify.yml` → **exit 0**。
  note: shellcheck が無いため 13 個の shell script は検査していない（jactionlint 自体は合格。shellcheck は入れなかった＝ネットから取りにいかない）。
- `cargo llvm-cov --workspace --locked --offline --no-cfg-coverage --lcov --output-path target/lcov-check.info` → **未検証**。
  この環境に cargo-llvm-cov が無い（`mise exec -- cargo llvm-cov --version` → `error: no such command: llvm-cov`）。インストールにはネットワークが必要なため入れなかった。E0554 の再現・解消も未検証。
- GitHub 上での coverage ジョブの実際の run → **未検証**（push しないため）。

## hk pre-commit（コミット e036f5d）

- `trailing_whitespace`（`hk util trailing-whitespace --diff .github/workflows/verify.yml`）✔ 合格
- `newlines`（`hk util end-of-file-fixer --diff .github/workflows/verify.yml`）✔ 合格
- cargo fmt / clippy / check のフックは、ステージされたファイルに Rust ファイルが無いため発火せず。
- `git commit` 全体で exit 0。

## 迷った点（安全側で進めた）

- 依頼文は作業ログの置き場所を `docs/worklog/` と書いているが、`docs/worklog/` にはバックエンド担当の `backend.md` しかなく、platform/CI 系の作業ログは `docs/worklogs/`（複数形）に `YYYY-MM-DD-platform-*.md` の個別ファイルで置く慣習（platform-pr-ci-r1・sysdeps-r1・octocov-comment-r1 が全部そこ）。repo の慣習に従い `docs/worklogs/2026-11-10-platform-coverage-no-cfg-r1.md`（このファイル）に作成した。`docs/worklog/backend.md` には足していない（バックエンド担当以外の節を混ぜない安全側）。
- `--no-cfg-coverage` を付けると、将来 `cfg(coverage)` で計測専用コードを差し替える依存・自前コードを入れた場合、そのコードは計測ビルドで有効にならない。今回の依頼は E0554 回避が目的で、該当する依存・自前コードは今の workspace に無い（grep で `cfg(coverage)` の使用は見つからず）ため、この制限は受け入れられると判断した。

## コミット

1. `e036f5d` `fix(ci): build coverage without cfg(coverage) to avoid E0554 on stable`（verify.yml と docs/dev-environment.md。2 files changed, 4 insertions(+), 2 deletions(-)）
2. この作業ログのコミット `docs(worklog): record coverage no-cfg-coverage r1`

push・fetch はしない（担当が自分で行う）。
