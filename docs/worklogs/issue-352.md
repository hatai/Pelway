# Issue #352 の作業ログ

## 実装者メモ（round 1）

対象 head: 01fd4aafe3205ee559e6aca1d179d3ccc9f1b4eb（この round の実装の実体を含むコミット。このメモを含むコミットの親）

1. 詰まった箇所: 環境: このスレッドの sandbox は `/home` を read-only マウントするため `mise run verify` が mbx（`~/.cache/mbx` 書込不可）と `scripts/tests/agent-sandbox.test.sh`（`$HOME` 直下への fixture 作成）で失敗。前者は `XDG_CACHE_HOME=/tmp/devin-xdgcache` へのリダイレクトで通した。後者は `HOME=/tmp/fakehome` でも試したが、sandbox-shell が実 HOME 前提のため同テストはこの環境では構造的に実行不可（差分とは無関係。それ以外の script-test 全件は個別実行で全パス、verify-lint・cargo test・verify-static も通過済み）
2. 曖昧だった文: 「Status 名がそのプロジェクトの選択肢に無い、対象 item が解決できない等の不正要求は `gh` を呼ばず失敗記録に回す」（Issue #352 本文「受け入れ条件」）
3. 仮定と解釈: 「`gh` を呼ばず」は、ファイル形式・Status 名・対象指定の検証で落ちる要求は gh を一切呼ばない、と解釈した（第 1 段をローカル検証にし、Issue 番号→item id の解決だけは `item-list` の読み取りが必須なため、解決不能系の失敗では読み取り系 gh のみ呼ぶ）。選択肢名は runbook の 8 値の固定集合で検証したうえで実プロジェクトの `field-list` にも照合する二段とした。`item_id` 指定の対象存在確認は実施せず、`item-edit` の失敗を `.error` に回す設計とした
4. 足りなかった情報・検証手段: エージェント環境とホストで `/tmp` が同一かは、この環境からは `df /tmp` が `/dev/sde`（tmpfs でない ext4）で Issue の前提と一致することまでしか確かめられなかった。ホスト側の実測は Issue の未決事項どおり報告に回した
5. 改善案: `scripts/tests/agent-sandbox.test.sh` に、`$HOME` が書き込み不可な環境（`/home` 自体が ro マウント）ではスキップするガードを足すと、tmpfs マスク環境でも script-test が完走できる

使ったスキル: `.agents/skills/implement-issue`、`.agents/skills/improve-skills`
