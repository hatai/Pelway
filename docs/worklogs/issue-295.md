# Issue #295 の作業ログ

## 実装者メモ（round 1）

対象 head: 2cbbfe8fdd71792bf013b93b418e2a338af49ac9

1. 詰まった箇所: `mise run verify` の script-test: `scripts/tests/conductor-watch-ensure.test.sh` だけが実行ごとに違うサブテストで失敗する（`foreign watch present`・`watch is running`・`ensure restarts watch` など）。`git stash` で差分を除いた main の tree でも同様に失敗することを確かめ、本差分とは無関係の環境依存と判断して進めた。
2. 曖昧だった文: 「review-pr スキルが実際に使う読み取り系（`gh api`, `gh pr diff/view/checks`, `git show/diff/log/grep` 等）に限定される」（Issue #295「受け入れ条件」）
3. 仮定と解釈: 「読み取り系」の集合は、例示の列挙ではなく `docs/dev-environment.md`「レビュー担当の Claude」設定表の許可形（gh: `api`・`issue view`・`pr view`・`pr diff`・`pr checks`・`pr review`）と review-pr・security-review が使う git 読み取り系（`show`・`diff`・`log`・`blame`・`grep`、skill の列挙に `blame` を含むため追加）と解釈した。選ばなかった解釈: `api` を GET 限定にする引数検査（#305 が hatai の選択待ちと明記）と、読み取り系 git の `--output`・`-O` の拒否（#306 の範囲。引数形の deny は省略形・束ね書きで外れやすいと #306 が結論付け済み）。拒否の終了コードは既存の 2〜5 に続く 6 とした。
4. 足りなかった情報・検証手段: review 名義の正当な呼び出しの全網羅リストは無く、リポジトリ内の `agent-gh review` 出現箇所（review-pr・security-review・dev-environment.md の確認手順）を grep で洗って許可集合を決めた。conductor-watch-ensure.test.sh の不安定さを切り分ける確実な再現条件（本番の conductor-watch.sh が常駐する環境で名前空間内の pgrep カウントがどうずれるか）は未解明。
5. 改善案: `scripts/tests/conductor-watch-ensure.test.sh` の動作検査の先頭に「unshare 内の `pgrep -f 'conductor-watch\.sh'` が実環境の常駐 watch を数えていないか」を確かめる事前条件を足し、混ざる環境では動作検査を skip と明記する。

使ったスキル: `.agents/skills/implement-issue`、`.agents/skills/write-pull-request`

## 実装者メモ（round 2）

対象 head: 0df36c311d0acdbdaf8b2841ad0b1f8ec85b6b03

1. 詰まった箇所: `scripts/agent-gh` の `review_api_args_allowed`: `gh api` のフラグ形を pflag の受け取り方に合わせて拾う必要があった（`-XDELETE`・`-fkey=...` の束ね書き、`--method=...`、値を取る `--method` と `--input`）。
2. 曖昧だった文: 「HTTP メソッドを GET に限定し、変更系メソッドを拒否してください」（PR #463 の codex レビュー must-fix、scripts/agent-gh:39）
3. 仮定と解釈: 「GET に限定」は `-X`/`--method` の値検査だけでなく、body を付けると `gh api` が暗黙に POST になる `-f`/`--field`・`-F`/`--raw-field`・`--input` の拒否を含むと解釈した。明示の `-X GET`・`--method get`（大小文字不問）は許可とした。選ばなかった解釈: `-X`/`--method` だけを検査して body 形を残す案（POST が残り must-fix を満たさない）。`api` の許可集合が狭まる影響は、リポジトリ内の `agent-gh review api` 呼び出しがすべて GET 形（`api /rate_limit`・`api /installation/repositories`・`api repos/...`）で投稿が `pr review` 経由のため無しと判断した。
4. 足りなかった情報・検証手段: `gh api` のフラグ一覧と pflag の束ね書きの受け取り方は `gh api --help` 相当の情報がリポジトリ内になく、コード内コメントでカバーする形を自前で列挙した。実機の `gh api` での拒否確認はレビュー側の実測に委ねる形になった。
5. 改善案: `docs/dev-environment.md`「レビュー担当の Claude」の設定表の読み取り系 `git` の列挙に `blame` を足す（review-pr が `git blame` を使うが、表の列挙には無い。round 1 の claude レビュー nit でも指摘された齟齬）。

使ったスキル: `.agents/skills/implement-issue`
