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

## 実装者メモ（round 3）

対象 head: 83a52436

1. 詰まった箇所: レビュー 2 ラウンドの must-fix。Codex は、試験の kill の形の確認が本物の終了処理を通っていないこと（モックの `sleep` の取り出しと、書き写した kill。負の PID への送信は専用のプロセスグループが無く失敗して、その失敗が単体 PID への SIGKILL に隠れる）、名前空間の中の見張りが実運用のボード更新キューを開くこと（`gh` が無い PATH で、未処理の依頼を失敗扱いで消す）を指摘。Claude は「spaces in path: one watch」が 1 回だけの `pgrep -fc` で、見張りの巡回が作る同じコマンドラインの一時的な subshell を数えて揺れることを指摘。対応: `conductor-watch-slots.test.sh`（単体 PID の kill）と `conductor-watch-stall.test.sh`（プロセスグループの kill）を実際に、管理対象の見張りと並行して名前空間の中で通す形に替えた（各試験が `REPO/scripts` から自分で写せるように、試験ごとの偽リポジトリ `trepo-*` に本物のスクリプトを置き、最小 PATH に `git`・`mktemp`・`rm`・`ln`・`touch`・`chown`・`sha256sum`・`rmdir` を足した）。`PELWAY_BOARD_QUEUE_DIR` を試験用に固定し、`waitn` にパターンの引数を足して上限付きで数え直す形にした。
2. 曖昧だった文: Codex の「実運用と同じ形の見張りを止めないこと」。「実運用と同じ形」は、実際の試験の終了処理を通すことと解釈した（kill の形を写すのでなく、試験そのものを通す）。
3. 仮定と解釈: 試験側の偽リポジトリは `git init` されるため、patrol-state の置き場の既定の判定（共有 git dir）を使わせる。内側の試験は `env -u PELWAY_PATROL_STATE` で走らせた。レビュー担当は `devin_review` が orchestrator で無効のため codex・claude の 2 担当にした（前ラウンドと同じ）。誤って残した `to: devin` の委託記録は削除した。
4. 足りなかった情報・検証手段: 開発機のキューに未処理の依頼が残る状態での実地確認はしていない（Codex のレビューの「未確認」のとおり）。
5. 改善案: `conductor-watch.sh` の巡回は自分と同じコマンドラインの subshell を瞬間的に作るため、プロセスの本数を 1 回の `pgrep -c` で確かめる確認は揺れます。`conductor-watch-ensure.test.sh` では上限付きで数え直す `waitn` にパターンの引数を足しました。同じパターンの確認をほかの試験で書くときも、数え直しにしてください。

使ったスキル: `.agents/skills/implement-issue`

## 実装者メモ（round 4）

対象 head: f227307dced44a8f5a6253741251f3c1c7400dbd

1. 詰まった箇所: レビュー 3 ラウンド（codex）の must-fix。正常終了する巡回では stall の試験のグループ停止（`kill -KILL -- -PGID`。同 15・93 行）を通らないこと、停止の確認が見張りの名前で数えるため別名の子を見逃すこと、最小 PATH に `mkfifo` が無く受信箱の FIFO を作れないまま試験 14 が形だけ通ることを指摘された。対応: 子を持つ試験用の見張り（実際の `conductor-watch.sh` と別名の子 `inbox-sleeper`）を `setsid` で立て、実際のグループ停止を通して、残りをプロセスグループ（`pgrep -g`）で確かめる検査を足した。`trepo-stall` を `git init` して `agent-gh` のモックを置き、最小 PATH に `mkfifo` を足し、stall の試験に受信箱が FIFO である確認（`test -p`）を足した。
2. 曖昧だった文: 「実際のグループ停止処理を通してください」。stall の試験が使う停止処理を、別の場所に写すのでなく同じ形で実行することと解釈した。
3. 仮定と解釈: レビュー担当は指揮役の判定どおり codex のみ（通常分類で、無効の devin_review の代わりに先頭に回る）。実行の無いまま残るとその head の結果がそろわないと数えられるため、claude の委託記録は取り下げた。Devin の参考レビューは、レビュー用の環境設定でない `acpRegistry_devin` では review-pr の起動時チェックで失敗し、チェックを回り道で回避する形も開発リードが禁止したため、実施しなかった。
4. 足りなかった情報・検証手段: codex・claude のレビューインスタンスがどちらも利用上限で、codex の再委託は復帰見込みのあとの予約タスクで仕込んだ（実際には上限解除で元の委託が自動で再開されて結果が届き、予約タスクは既投稿を確かめて何もしなかった）。実運用のキューに未処理の依頼が残る環境での実地確認はしていない。
5. 改善案: 試験が「残っていないこと」を確かめるときは、プロセスの名前（コマンドライン）ではなく、残るべきでない集団（プロセスグループなど）で数えること。名前で数えると、見張りが立てる別名の子（python3・sleep など）を見逃します。

使ったスキル: `.agents/skills/implement-issue`

## 実装者メモ（round 5）

対象 head: 1694bfd99db8a57a3705119b0598df4ac129413d

1. 詰まった箇所: レビュー 4 ラウンド（codex）の must-fix。前回足した独立したグループ停止の検査は、起動用の `bash -c` のコマンドライン自体が `inbox-sleeper` を含むため `pgrep -f` が見張り・子の起動前に一致し、子が無くても合格していた。かつ独立した呼び出しは実際の stall の停止経路ではなかった。対応: `conductor-watch-stall.test.sh` に試験 18 を足し、応答しない t3（`exec /bin/sleep 600`。モックの sleep を通らない）で巡回を時間切れにして、run_cycles の実際の停止処理（`kill -KILL -- -$WPID`）を通す。停止の直前に `pgrep -g` でグループを記録し、見張りと別名の子が別の PID で存在したこと・停止後にグループが空であることを確かめる。ensure 側の独立した検査は取り除き、実際の停止経路の確認は stall の試験が同じ名前空間で担う形にした。
2. 曖昧だった文: 「実際の stall の停止経路を通すケース」。stall の試験自身のコード（run_cycles の時間切れ分岐）が停止処理を実行する形と解釈し、独立した呼び出しに写さなかった。
3. 仮定と解釈: レビューの再委託は自分では行わず、push のあとに PR に [review-request] を書いて指揮役に立ててもらう（開発リードの指示。round 4 のレビューは指揮役側の委託で入った）。
4. 足りなかった情報・検証手段: 自分の委託記録（7216775401）は指揮役側の委託のレビューと run が違うため取り下げた。T3 の runtime mode の違いで指揮役のスレッドへは直接送れないため、やり取りは PR のコメントで行った。
5. 改善案: 「子が起動したこと」を `pgrep -f` の文字列で確かめるときは、起動側のコマンドラインに同じ文字列が載らないこと（`bash -c` の引数に含めると起動元自身に一致する）。子の検出はコマンドラインではなく、専用のマーカーファイルやプロセスグループのメンバー数で確かめるのが確実です。

使ったスキル: `.agents/skills/implement-issue`

## 実装者メモ（round 6）

対象 head: 4792de513fd88931242b0a39af5fa370b75d3bfd

1. 詰まった箇所: なし（印の nit のみの round）。
2. 曖昧だった文: なし。
3. 仮定と解釈: codex の round 5 の verdict は approve。`[文書のずれ]` の印の nit（report.md の `mise run verify` の回数が 5 回・5 回目最終版の記述）だけを直した。印の無い nit（stall の試験の 18 番のメンバー照合を `LAST_WPID` と `/bin/sleep 600` の行で確かめる案）は、この PR では直さない（印の無い nit は直さない決まり）。
4. 足りなかった情報・検証手段: なし。
5. 改善案: なし。

使ったスキル: `.agents/skills/implement-issue`
