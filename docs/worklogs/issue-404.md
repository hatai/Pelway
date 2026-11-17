# Issue #404 の作業ログ

## 実装者メモ（round 1）

対象 head: c8e38353919ab546b5b67efe0f6ba265c00444f2

1. 詰まった箇所: mise run verify: `scripts/tests/conductor-watch-ensure.test.sh` の behavior-fallback が失敗（失敗箇所は実行ごとに `watch is running` / `extra leader present` / `fragile watch present` と変動）。`git stash` して HEAD（main と同一コミット 3884375）でも再現したため環境起因と判断した。cargo test（全ワークスペース）・clippy・fmt・hk・deny は全部合格を確認して進めた（検証の内訳は PR 本文に記録）。
2. 曖昧だった文: 「クエリの `key` / `token` / `api_key` / `access_token` などの値は伏せる（キー名は残してよい）」（Issue #404 本文「やること」）
3. 仮定と解釈: 「など」に `secret`・`password` と、キー名の正規化（小文字化・`_`/`-` 除去）で `api-key`・`apiKey`・`Access-Token` 等の表記ゆれを含めた。判定は正規化名が `key`/`token`/`secret`/`password` を**含む**（過剰側）で判定する解釈を採り、列挙名との完全一致の解釈は採らなかった（取りこぼしのリスクを優先）。伏せ文字は URL クエリでパーセントエンコードされない `***` とした（リポジトリ既存の `<redacted>` リテラルは form-urlencoded の再エンコードで化けるため）。
4. 足りなかった情報・検証手段: `eprintln!` の stderr 出力そのものをテストから捕捉する手段。テストは起動時ログの 1 行を組み立てる `DemoLlmConfig::endpoint_log_line`（demo.rs）を確かめる形にした。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「やること」の書き方に「列挙に『など』を使うときは一致の規則（含む一致か完全一致か）まで書く」を足す。

使ったスキル: implement-issue
