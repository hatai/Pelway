# Issue #295 の作業ログ

## 実装者メモ（round 1）

対象 head: 2cbbfe8fdd71792bf013b93b418e2a338af49ac9

1. 詰まった箇所: `mise run verify` の script-test: `scripts/tests/conductor-watch-ensure.test.sh` だけが実行ごとに違うサブテストで失敗する（`foreign watch present`・`watch is running`・`ensure restarts watch` など）。`git stash` で差分を除いた main の tree でも同様に失敗することを確かめ、本差分とは無関係の環境依存と判断して進めた。
2. 曖昧だった文: 「review-pr スキルが実際に使う読み取り系（`gh api`, `gh pr diff/view/checks`, `git show/diff/log/grep` 等）に限定される」（Issue #295「受け入れ条件」）
3. 仮定と解釈: 「読み取り系」の集合は、例示の列挙ではなく `docs/dev-environment.md`「レビュー担当の Claude」設定表の許可形（gh: `api`・`issue view`・`pr view`・`pr diff`・`pr checks`・`pr review`）と review-pr・security-review が使う git 読み取り系（`show`・`diff`・`log`・`blame`・`grep`、skill の列挙に `blame` を含むため追加）と解釈した。選ばなかった解釈: `api` を GET 限定にする引数検査（#305 が hatai の選択待ちと明記）と、読み取り系 git の `--output`・`-O` の拒否（#306 の範囲。引数形の deny は省略形・束ね書きで外れやすいと #306 が結論付け済み）。拒否の終了コードは既存の 2〜5 に続く 6 とした。
4. 足りなかった情報・検証手段: review 名義の正当な呼び出しの全網羅リストは無く、リポジトリ内の `agent-gh review` 出現箇所（review-pr・security-review・dev-environment.md の確認手順）を grep で洗って許可集合を決めた。conductor-watch-ensure.test.sh の不安定さを切り分ける確実な再現条件（本番の conductor-watch.sh が常駐する環境で名前空間内の pgrep カウントがどうずれるか）は未解明。
5. 改善案: `scripts/tests/conductor-watch-ensure.test.sh` の動作検査の先頭に「unshare 内の `pgrep -f 'conductor-watch\.sh'` が実環境の常駐 watch を数えていないか」を確かめる事前条件を足し、混ざる環境では動作検査を skip と明記する。

使ったスキル: `.agents/skills/implement-issue`、`.agents/skills/write-pull-request`
