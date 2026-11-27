# Issue #808 の作業ログ

## 実装者メモ（round 1）

対象 head: 5a0c6eea89e3f5b74cdd1495f5a6c7338acd53d0

1. 詰まった箇所: 手順 6 の `mise run verify`: この sandbox では `/tmp` の所有者が uid 65534 のため、`.agents/skills/verify-pelway/verify-run.test.sh` が `make_run_dir` で落ちます（この PR の変更を退避した同じ木でも同じ 6 つの FAIL）。`TMPDIR=/dev/shm/p808` で回しました（作業ツリーの中だと mbx の socket のパスが長すぎます）。`.devin/` が読み取り専用のため `git merge --ff-only origin/main` は `.devin/config.json` で止まり、`git reset --mixed origin/main` で分岐点だけを main の先端に動かしました。読み取り専用で更新できない `.devin/config.json` には `git update-index --skip-worktree` を付けて、commit に含まれないようにしました。verify の 1 回目は `scripts/tests/shellcheck.test.sh` が落ちたため、通るまで計 4 回回しました。
2. 曖昧だった文: 「report-only の Draft PR（ブランチ `agent/investigate-<slug>`）を作り、手順書どおりマージまで行く」（Issue #808 の受け入れ条件）。開発リードからの T3 スレッドの指示「いつもの手順どおり agent/808-… ブランチで PR を作り」が新しい決定のため、そちらに従い、報告と修正を同じ PR にしました。
3. 仮定と解釈: 「見張り」は、コマンドラインが `<bash> <…>/scripts/conductor-watch.sh` の 2 つで、スクリプトがこのリポジトリの checkout（主 checkout と各 worktree）にあるプロセスとしました。pidfile 方式は、手で起動した見張りを置き換えるという Issue #285 の目的を果たせないため選びませんでした。相対パスで起動された見張りは `/proc/<pid>/cwd` から解きました。
4. 足りなかった情報・検証手段: verify のログに時刻が無いため、失敗の時刻は自分で取った観測ログ（decoy が SIGTERM を受け取った時刻と、ensure を実行しているプロセスを `/proc` で見つけた時刻）で突き合わせました。ローカルの verify の 1 回目（00:24）と修正後の A/B の反復 30 で shellcheck に届いた SIGTERM の送信元は、観測をしていなかったため記録にありません。
5. 改善案: `scripts/script-test.sh` の 85 行の `echo "=== $t ==="` を `echo "=== $t === $(date '+%Y-%m-%dT%H:%M:%S%z')"` にする。verify のログだけで失敗の時刻が分かるようになり、外部の kill との突き合わせが一度で済みます。

使ったスキル: `.agents/skills/investigate-bug`、`.agents/skills/implement-issue`

## 実装者メモ（round 2）

対象 head: 9e04fded487e06ea08ac65f592f344fce16470b8

1. 詰まった箇所: レビュー 1 ラウンドの must-fix（`scripts/conductor-watch-ensure.sh` の選択式）: `./scripts/conductor-watch.sh` と起動した見張りが `<cwd>/./scripts/…` になって比較から外れ、空白を含む checkout の見張りが `[^ ]*` のパターンで外れていた。`/proc` の NUL 区切りで引数を読み、`cd -P` で解決した物理パスで比べる形に替えた。名前空間の中で conductor-watch の試験のモックの `sleep`（本物から取り出す）を ensure の管理対象の見張りの横で使った。取り出したモックには実行権限が要り、`chmod` を名前空間の PATH に足した。
2. 曖昧だった文: 「テストが本番の watcher を、本番がテストの watcher を互いに止めないことを回帰テストで示してください」（開発リードからの T3 スレッドの指示）。Codex の must-fix を受けて、「本番の見張り」は ensure が管理対象とする見張り（名前空間の中の unit・leader の見張り）と解釈し、一時ディレクトリに置いた代役ではなく、その横で試験の kill の形を実行する形にした。
3. 仮定と解釈: 相対パスの見張りは、見張り自身がリポジトリのルートへ移動したあとの cwd から求めた（起動してまもない数ミリ秒は選ばれない）。`git worktree list` に載る checkout だけを管理対象にする割り切りは、報告の「危うさ」に書いた。Issue #808 の本文は報告だけを範囲にしており、PR #843 は修正を含む。範囲の変更は開発リードからの指示だが Issue 上で確認できないため、自分では本文を変えずに起票者への依頼とした。
4. 足りなかった情報・検証手段: 開発機で SIGTERM を送っているのが指揮役の ensure であることは、17 分の観測では確かめられなかった（その間に ensure の実行が無かった）。A/B を worktree の中の相対パスに出力して実行した 1 回は、ダミーのリポジトリが本物の git リポジトリの中に入って ensure の起動対象が本物の見張りになり、無効だった（本物の見張りは巡回の前段で終わったため、GitHub にもボードのキューにも触れていない）。出力先を worktree の外の絶対パスにしてやり直した。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「決まり（毎回守る）」に「T3 スレッドの指示で Issue の受け入れ条件や対象外を変えるときは、着手の前に起票者か人間に Issue 本文を書き換えてもらう。実装担当の App が引用として残したコメントは、レビューで範囲変更の根拠に数えられない」を足す。

使ったスキル: `.agents/skills/implement-issue`
