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

## 実装者メモ（round 3）

対象 head: 06d5e8228f9dbe9e4be120b87e7b8ab7db717cd7

1. 詰まった箇所: 手順 6（push）後: PR #351 の main マージで `mise.toml` の script-test 定義が glob 収集（`scripts/script-test.sh`）に置き換わり、PR #355 が mergeState=DIRTY になった。`Merge branch 'main'` で取り込み、`mise.toml` は main 側を採用（我々のテストは `scripts/tests/*.test.sh` で自動収集されるため明示登録は不要になった）
2. 曖昧だった文: 「解消後はラウンドを進めてレビューを再度委託してください（承認後 push は再レビュー対象）」（修正依頼）
3. 仮定と解釈: 取り込み方法は rebase ではなく `Merge branch 'main'`（main の履歴に同形のマージコミットがある先例に合わせた）。マージコミットは agent-gh-resign で表現できないため署名なしのまま（#351 の `58720b3` と同じ状態）。effort は 3 回目のレビューのため codex は `xhigh`・claude は `max` に 1 段上げる（claude の一段上は文書に先例がないため、devin と同じ medium|high|max の段と読んで `max` とした）
4. 足りなかった情報・検証手段: claude_review の effort 有効値の一覧（docs/dev-environment.md に値域の記載なし）
5. 改善案: `docs/dev-environment.md` の「レビュー担当の Claude」に `effort` の有効値（low|medium|high|max 相当）を明記する

使ったスキル: `.agents/skills/implement-issue`

## 実装者メモ（round 4）

対象 head: 3f514a96c3d238cba7f722e8003234ede396c852

1. 詰まった箇所: scripts/conductor-watch.sh:241: round 3 の静的レビュー（codex xhigh）で `{"issue":null}` / `{"item_id":null}` が検証を通り phase 2 の gh 呼び出しに進む指摘。値が null のとき `iss is not None` 条件で検証を素通りしていたため、キーの存在に基づく型検証（`"issue" in req and not iss_ok`）に直し、null・空文字・両方 null のケースを gh ゼロ回の assert つきでテスト追加
2. 曖昧だった文: 「キーの存在に基づいて型を検証し（issue は有限の数値、item_id は空でない文字列、jq の型チェックで null を弾く）」（round 3 レビュー must-fix）
3. 仮定と解釈: 「有限の数値」は正の整数（bool・float 不可）の既存検証で十分と解釈（JSON に inf/nan は来ない）。「jq の型チェック」は実装が Python のため isinstance 検証と読み替えた
4. 足りなかった情報・検証手段: レビュー投稿が checks pending ルールで未投稿のため、GitHub 上のレビュー本文は未確認（指摘文は依頼者経由で入手）
5. 改善案: なし（`is not None` で検証を素通りする穴は初版の検証ロジックにあったもの。null 系のテストを先に書いていれば防げた）

使ったスキル: `.agents/skills/implement-issue`
