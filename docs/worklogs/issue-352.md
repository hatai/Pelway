# Issue #352 の作業ログ

## 実装者メモ（round 1）

対象 head: daeb1bf34bd5eccdc6cb4ca5ec2ad3d4d39c2dca（この round の実装の実体を含むコミット。`scripts/agent-gh-resign` による署名付け替え後の SHA。このメモを含むコミットの親）

1. 詰まった箇所: 環境: このスレッドの sandbox は `/home` を read-only マウントするため `mise run verify` が mbx（`~/.cache/mbx` 書込不可）と `scripts/tests/agent-sandbox.test.sh`（`$HOME` 直下への fixture 作成）で失敗。前者は `XDG_CACHE_HOME=/tmp/devin-xdgcache` へのリダイレクトで通した。後者は `HOME=/tmp/fakehome` でも試したが、sandbox-shell が実 HOME 前提のため同テストはこの環境では構造的に実行不可（差分とは無関係。それ以外の script-test 全件は個別実行で全パス、verify-lint・cargo test・verify-static も通過済み）
2. 曖昧だった文: 「Status 名がそのプロジェクトの選択肢に無い、対象 item が解決できない等の不正要求は `gh` を呼ばず失敗記録に回す」（Issue #352 本文「受け入れ条件」）
3. 仮定と解釈: 「`gh` を呼ばず」は、ファイル形式・Status 名・対象指定の検証で落ちる要求は gh を一切呼ばない、と解釈した（第 1 段をローカル検証にし、Issue 番号→item id の解決だけは `item-list` の読み取りが必須なため、解決不能系の失敗では読み取り系 gh のみ呼ぶ）。選択肢名は runbook の 8 値の固定集合で検証したうえで実プロジェクトの `field-list` にも照合する二段とした。`item_id` 指定の対象存在確認は実施せず、`item-edit` の失敗を `.error` に回す設計とした
4. 足りなかった情報・検証手段: エージェント環境とホストで `/tmp` が同一かは、この環境からは `df /tmp` が `/dev/sde`（tmpfs でない ext4）で Issue の前提と一致することまでしか確かめられなかった。ホスト側の実測は Issue の未決事項どおり報告に回した
5. 改善案: `scripts/tests/agent-sandbox.test.sh` に、`$HOME` が書き込み不可な環境（`/home` 自体が ro マウント）ではスキップするガードを足すと、tmpfs マスク環境でも script-test が完走できる

使ったスキル: `.agents/skills/implement-issue`、`.agents/skills/improve-skills`

## 実装者メモ（round 2）

対象 head: 919979de41e2db871ba44f984474d69cff690649

1. 詰まった箇所: PR #355 レビュー（codex must-fix 1・claude must-fix 2）: (a)「不正要求は `gh` を呼ばず」の厳格解釈（読み取り系 gh も不可）は、Status 名→選択肢 id・Issue 番号→item id の解決に読み取り gh が必須で実装不能のため、2 段解釈（ローカル拒否は gh 無呼出・解決失敗は item-edit 不呼出）を Issue コメントと runbook に明文化して仕様・実装を一致させた。(b) `.done.tmp`/`.error.tmp` への symlink で人間名義の任意ファイル上書き、(c) キューディレクトリの symlink 差し替えによるホストファイル削除と FIFO による巡回停止は、dir_fd ベースの O_NOFOLLOW 操作（ディレクトリ検証・要求の S_ISREG・上限・O_NONBLOCK、一時ファイルはランダム名 O_EXCL|O_NOFOLLOW）に作り替え、テスト 4 群を足した
2. 曖昧だった文: 「静的検証だけで拒否できる条件と、プロジェクト状態の照会が必要な条件を Issue と合意したうえで仕様・実装を一致させてください」（PR #355 codex レビュー must-fix）
3. 仮定と解釈: 「Issue と合意」は起票者の返答を待つ意味ではなく、Issue のコメントに解釈を記録して進める形と解釈した（implement-issue の「判断とその理由は PR 本文か Issue のコメントに書いて進める」に従う）。claude の nit 2 件（repository 絞り・使ったスキル欄）は must-fix だけ直す決まりにより未対応とした
4. 足りなかった情報・検証手段: エージェント sandbox がホストと同じ uid で `/tmp` 直下に書けるかはこの環境からは未確認（claude レビューの「未確認」に同じ）
5. 改善案: `scripts/tests/agent-sandbox.test.sh` の ro マウント環境でのスキップガード（round 1 と同じ）

使ったスキル: `.agents/skills/implement-issue`
