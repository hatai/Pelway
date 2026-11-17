# Issue #404 の作業ログ

## 実装者メモ（round 1）

対象 head: c8e38353919ab546b5b67efe0f6ba265c00444f2

1. 詰まった箇所: mise run verify: `scripts/tests/conductor-watch-ensure.test.sh` の behavior-fallback が失敗（失敗箇所は実行ごとに `watch is running` / `extra leader present` / `fragile watch present` と変動）。`git stash` して HEAD（main と同一コミット 3884375）でも再現したため環境起因と判断した。cargo test（全ワークスペース）・clippy・fmt・hk・deny は全部合格を確認して進めた（検証の内訳は PR 本文に記録）。
2. 曖昧だった文: 「クエリの `key` / `token` / `api_key` / `access_token` などの値は伏せる（キー名は残してよい）」（Issue #404 本文「やること」）
3. 仮定と解釈: 「など」に `secret`・`password` と、キー名の正規化（小文字化・`_`/`-` 除去）で `api-key`・`apiKey`・`Access-Token` 等の表記ゆれを含めた。判定は正規化名が `key`/`token`/`secret`/`password` を**含む**（過剰側）で判定する解釈を採り、列挙名との完全一致の解釈は採らなかった（取りこぼしのリスクを優先）。伏せ文字は URL クエリでパーセントエンコードされない `***` とした（リポジトリ既存の `<redacted>` リテラルは form-urlencoded の再エンコードで化けるため）。
4. 足りなかった情報・検証手段: `eprintln!` の stderr 出力そのものをテストから捕捉する手段。テストは起動時ログの 1 行を組み立てる `DemoLlmConfig::endpoint_log_line`（demo.rs）を確かめる形にした。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「やること」の書き方に「列挙に『など』を使うときは一致の規則（含む一致か完全一致か）まで書く」を足す。

使ったスキル: implement-issue

## 実装者メモ（round 2）

対象 head: b376d6231dba88bdb31d497d5d84e1a99bded15d

1. 詰まった箇所: `scripts/agent-gh dev api PATCH`: skill write-pull-request の記載形 `api PATCH /repos/...` が現在の gh では `accepts 1 arg(s), received 2` で拒否され、`api --method PATCH` に読み替えた。
2. 曖昧だった文: 「fragment は OAuth implicit flow の `access_token` など secret を載せる代表的な場所で、Issue #404 の『秘密がログに残る』という趣旨では userinfo・クエリと同じ伏せ対象」（PR #417 round 1 の Devin レビューの must-fix）
3. 仮定と解釈: must-fix 本文の「`parsed.set_fragment(None)` を足し」を「fragment は中身を見ずに丸ごと落とす（無害な fragment も落とす）」と解釈した。値の部分だけ `***` に置き換える解釈は採らなかった（fragment の形式はクエリと限らず、見て判断する余地を残さないほうが漏れがないため。Claude レビューの nit も同案）。
4. 足りなかった情報・検証手段: なし（must-fix に修正方針とテストの構造まで指定があった）。
5. 改善案: `.agents/skills/write-pull-request/SKILL.md` の「手順 3」の `api PATCH /repos/...` の記載を `api --method PATCH /repos/...` に直す。

使ったスキル: implement-issue
