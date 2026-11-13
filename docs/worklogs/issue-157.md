# Issue #157 の作業ログ

## 実装者メモ（round 1）

1. 詰まった箇所: `.github/workflows/verify.yml` coverage ジョブ: `--exclude dev-client-probe`（ディレクトリ名）では除外されず、`crates/*/Cargo.toml` の `name`（`pelway-dev-client-probe` 等）に変えて初めて効いた。加えて、ローカルの `target/llvm-cov-target/` に `--exclude` なし実行の profraw が残っているとマージに混入して除外済みの SF: が復活する（llvm-cov-target を空にしたら再現しなくなった。CI は毎回クリーンなので影響なし）。
2. 曖昧だった文: 「`cargo llvm-cov` 呼び出しへの `--exclude`（dev-client-probe・dev-queue-compare・dev-smoke・pelway-agent-watch・pelway-db）」（Issue #157 本文・目的）— ディレクトリ名で列挙されているが、cargo の `--exclude` が受け取るのはパッケージ名で、この 3 dev crate は `pelway-` 接頭辞が付く。
3. 仮定と解釈: `--ignore-filename-regex` は Issue の grep 検証（`[/\]out[/\]`・`crates/[^/]*/tests/`）に対応する 1 つの正規表現 `([/\\]out[/\\]|crates/[^/]+/tests/)` にまとめた（検証コマンドが 2 列の `\]` を許すのに対し、cargo-llvm-cov に渡す Rust regex では `]` を `\\]` にエスケープせねばならない）。`tests/` は現状の lcov.info には元々 0 件だが、将来の混入防止として入れた。選ばなかった解釈: `out` だけの部分一致（`src/out/` のような本来のソースを巻き込む恐れ）。
4. 足りなかった情報・検証手段: パッケージ名とディレクトリ名の対応（`grep -m1 '^name' crates/*/Cargo.toml` で確認した）。ローカル検証で過去の profraw 混入を防ぐには `target/llvm-cov-target/` の掃除が必要なこと。
5. 改善案: `.agents/skills/write-issue/SKILL.md`の「採用する方式」に「cargo の `--exclude` 等パッケージ指定は crates/*/Cargo.toml の name で書く（ディレクトリ名と一致するとは限らない）」の一文追加。

## 実装者メモ（round 2）

1. 詰まった箇所: PR #177 のレビューコメント「設定ファイルに書くことできない？」: cargo-llvm-cov v0.6.17 は設定ファイルを持たない（README の目次に Configuration 無し、CHANGELOG 全履歴にも設定ファイル機能の追加なし。cargo-config2 は cargo 標準設定の読み取り用）。代わりに mise.toml の `[tasks.coverage]` に引数を括り出し、verify.yml は `mise run coverage` を呼ぶ形にした。
2. 曖昧だった文: 「設定ファイルに書くことできない？」（PR #177 レビューコメント）— 「設定ファイル」が cargo-llvm-cov 自体の設定ファイルか、リポジトリ側の設定（mise 等）かが曖昧。
3. 仮定と解釈: レビュアーの意図は「ワークフローへの長い引数のインライン記述をやめて設定に置きたい」と解釈した。cargo-llvm-cov 自体の設定ファイルは非対応のため、このリポジトリの単一管理元である mise.toml のタスクに集約する解釈を選び、octocov 側（.octocov.yml）での除外は選ばなかった（lcov.info に混入したままでは #157 の受け入れ条件「lcov.info に含まれない」を満たさないため）。
4. 足りなかった情報・検証手段: cargo-llvm-cov の設定ファイル非対応の確認手段（README v0.6.17 と CHANGELOG v0.6.17 の取得元 URL）。
5. 改善案: `docs/dev-environment.md`の「ジョブ構成」の表に coverage ジョブが mise タスクを呼ぶことを反映した（今回の変更に含めた）。
