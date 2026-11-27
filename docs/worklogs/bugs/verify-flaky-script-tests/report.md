# `mise run verify` のスクリプトの試験が間欠的に落ちる調査: conductor-watch-ensure が見張り以外にも SIGTERM を送る

調査の対象は Issue #808 です。起票時に挙がっていた失敗は 3 つで、この報告では次の名前で呼びます。

- 「slots の試験」: `scripts/tests/conductor-watch-slots.test.sh`
- 「shellcheck の試験」: `scripts/tests/shellcheck.test.sh`
- 「shellcheck タスク」: `mise run shellcheck`（本体は `scripts/shellcheck.sh`）

## 結論

確かめた事実は次のとおりです。

1. 修正前の `scripts/conductor-watch-ensure.sh`（以下「ensure」。指揮役が巡回の最初に実行する、見張りの起動と置き換えのスクリプト）は、コマンドラインに `conductor-watch.sh` という文字列を含むプロセスをすべて「見張り」とみなし、SIGTERM で終了させます。見張りとは `scripts/conductor-watch.sh`（T3 のスレッドと PR の状態変化を出し続ける常駐スクリプト）です。
2. この選択は、見張りではないのに同じ文字列をコマンドラインに持つ 2 種類のプロセスも選びます。(1) shellcheck の本体。`scripts/shellcheck.sh` が検査対象のスクリプトのパスを引数に並べて起動するためです。(2) conductor-watch の試験が一時ディレクトリに置いた見張りの写し。ファイル名は本物と同じです。
3. この 2 種類のプロセスに SIGTERM が届くと、Issue が報告した 3 つの失敗と同じ出力で落ちます（「再現」の 1.）。
4. 並列の数が 1 で、負荷をかけない状態でも再現しました。時間切れの設定は関与していません（`mise.toml` に timeout の設定はなく、失敗の出力に timeout を示す文字はありません）。
5. 修正前の ensure を試験と同じ時に実行すると、毎回同じ落ち方をしました（「修正後の確認」の修正前後の比較）。

推定は次のとおりです。直接に観測できていない部分です。

- Issue で報告された失敗と、開発機で自分が見た失敗（「再現」の 2. とローカルの verify の 1 回目）について、SIGTERM を送ったのは指揮役（チームのエージェントに Issue を割り当てるスレッド）が巡回の最初に実行した ensure である、という推定です。根拠と限界は「再現」の 3. に書いてあります。
- 負荷が、SIGTERM が届くと落ちる時間の長さを伸ばすなら、負荷は落ちる頻度に影響しうる、という推定です。時間の長さへの影響も頻度への影響も測っていません（「再現」の 4. には負荷別の測定値はありません）。

この報告の根拠は、この調査で自分が実行した試験と観測のログだけです（「証拠のログ」の節）。ほかのエージェントの出力は根拠に使っていません（開発リードの指示による）。

## 報告の読み取り

- 期待: `mise run verify` が通る。
- 実際: 3 つの失敗は、それぞれ単独で回すと通り、全体を回し直すと通った（Issue #808 の本文）。落ちたときの出力は、slots の試験では `FAIL SLOTS lines return after toml fixed` と `FAIL no stale usage on fetch failure`、shellcheck タスクでは `Terminated`（指摘 0 件）でした。
- 対象: 開発機 minisforum（WSL2、Ubuntu 24.04、論理 CPU 24）。複数のエージェントのスレッドが同時に `mise run verify` を回している環境。
- 場所: `mise run verify` の `shellcheck` タスクと `script-test` タスク。
- 時刻: 報告は 2026-11-26 です（Issue #808 の起票）。この調査の再現と観測は 2026-11-26 19:58 から 2026-11-27 15:40 まで（JST）に行いました。
- 頻度: Issue では 3 失敗とも「単独では通り、全体の回し直しで通った」とあります。この調査で測った頻度は「再現」の節のとおりです。

## 修正済みか

該当なし。選択式はコミット `ee15dae0`（PR #286、Issue #285、2026-11-14）で入り、kill する部分は `28ac875d`（同日）で入りました。Issue と PR を「flaky」「揺れ」「pgrep」「conductor-watch-ensure」で検索しましたが、この原因を扱ったものは #808 のみでした。`docs/worklogs/bugs/` に同じ症状の過去の調査はありません。

## 再現

### 1. 報告されたのと同じ出力を、SIGTERM で出す

ensure は動かさず、ensure と同じ選択に当たる自分のプロセスにだけ SIGTERM を送りました。

- shellcheck タスク: `bash scripts/shellcheck.sh` の `--severity=warning` の shellcheck に SIGTERM を送ると、`Terminated` と `shellcheck: findings above (85 files checked)` を出して終了コード 1 になりました（1 回中 1 回）。
- shellcheck の試験: 試験をそのまま走らせ、選択に当たる shellcheck に送り続けると、手順 5 で `FAIL mise run shellcheck failed (exit 1)` と `FAIL mise run shellcheck reported '<none>' files checked, want 85` が出ました。手順 7 の 3 回では `FAIL shellcheck.sh failed on an info-level-only script outside demo-check (want pass): Terminated` と `FAIL shellcheck.sh failed without reporting SC2164 in scripts/sub/warn.sh: Terminated` が出ました（1 回中 1 回）。
- slots の試験: 節 8 の巡回の python が `SLOTS` の行を出す前に、見張りの bash にだけ SIGTERM を送ると、Issue が報告したのと同じ 2 つの FAIL が出ました。時刻で当てる方式で 2 回中 2 回、巡回の途中で止める仕掛けを入れた写しで 3 回中 3 回です。python が始まる前に送ると 1 つ目の FAIL だけが出ました（1 回中 1 回）。ほかの節で送った 7 回は、その節の確認が 1 つだけ落ちました（5 種類の FAIL）。
- kill を送らなかった slots の試験は 148 回すべて通りました。

### 2. 何も kill せずに繰り返すと、落ちるときがある

2026-11-26 20:51:37 から 21:03:12（JST）まで、slots の試験と `bash scripts/shellcheck.sh` を並行して繰り返しました。この間、自分は何も kill していません。

- slots の試験 58 回のうち 1 回が落ちました（20:54:36 開始。`FAIL invalid toml emits SLOTS error` など 4 つの FAIL）。
- shellcheck 94 回のうち 1 回が落ちました（21:02:46 開始。1 行目が `Terminated`）。

落ちた 2 回は、上の「SIGTERM を送ったとき」と同じ種類の出力です。

### 3. SIGTERM を送るのは ensure であること

ensure が SIGTERM を送ることは、修正前の ensure を自分で実行して確かめました（「修正後の確認」の修正前後の比較）。

開発機で動いている指揮役の ensure が同じことをしているかは、次の観測で確かめようとしました。2026-11-27 10:46:42 から 11:03:42（JST）まで、コマンドラインに `scripts/conductor-watch.sh` を持つ自分のプロセスを 2 つ（shellcheck と同じ形、リポジトリの外の写しの見張りと同じ形）立てて SIGTERM を受け取った時刻を記録し、同時に `/proc` を 0.1 秒ごとに見て、コマンドラインの末尾が `conductor-watch-ensure.sh` のプロセスを見つけた時刻を記録しました。この 17 分間では、2 つのプロセスに SIGTERM は届かず、指揮役による ensure の実行も見つかりませんでした。見つかった 6 件は、ほかのスレッドの verify が名前空間の中で走らせた試験の写しです（名前空間の中からは自分のプロセスは見えず、信号も送れません）。この窓の外で起きた kill の回数と送信元は、未集計です。

したがって「開発機で SIGTERM を送っているのは指揮役の巡回の ensure である」ことは、直接に観測した事実ではなく、次の 3 つからの推定です。(1) skill `command-team` の「1. サイクルの始め」は指揮役に毎回 ensure を実行させる。(2) 上の 2. と、後述のローカルの verify の 1 回目で、自分は kill していないのに 1. と同じ出力で落ちた。(3) 修正前の ensure は、実行されると 1. と同じ SIGTERM を同じ選択で送る。確定させるには、指揮役の巡回と重なる時間帯に `scripts/observe.sh` を走らせて、ensure のプロセスを見つけた時刻と SIGTERM の到達の時刻が一致することを見ます。

### 4. 3 つの失敗の再現条件

確かめた条件は「どのプロセスに、どの時点で SIGTERM が届くと落ちるか」です。

| 失敗 | SIGTERM が届くと落ちるプロセスと時点 | その時間の長さの目安（1 回の verify あたり） | 再現 |
|---|---|---|---|
| shellcheck タスク | verify の開始直後、`--severity=warning` の shellcheck が生きている間 | 数秒。自分のローカルの verify では shellcheck タスク全体で 10〜14 秒だった（2026-11-26 23:29 から 2026-11-27 00:20 の 3 回） | した（1. で 1/1、2. で 1/94） |
| shellcheck の試験 | 手順 5 の `mise run shellcheck` の同じ shellcheck が生きている間（verify の終わり近く） | 同上 | した（1. で 1/1） |
| slots の試験 | 節 8 の巡回の python が `SLOTS` の行を出す前 | 0.3 秒未満（節 8 の見張りの寿命は 0.1〜0.3 秒） | した（1. で 5/5） |

並列の数は 1 でも再現します。上の 1. の再現はすべて、試験を 1 本だけ単独で走らせた状態で行いました。負荷をかけることも、時間切れも必要ありません。負荷が上の「時間の長さ」を伸ばすなら落ちる頻度に影響しうると考えられますが、時間の長さへの影響も頻度への影響も測っていません。

## 根本原因

`scripts/conductor-watch-ensure.sh` の選択式が広すぎます。`pgrep -f` はコマンドライン全体への部分一致です。

```bash
watch_pids() { pgrep -f "conductor-watch\.sh" | grep -vx -e "$$" -e "${PPID:-0}" || true; }
```
（修正前の `scripts/conductor-watch-ensure.sh:31`）

```bash
  for p in $1; do kill "$p" 2>/dev/null || true; done
```
（修正前の `scripts/conductor-watch-ensure.sh:63`。呼び出し元は 110 行（systemd の経路）と 128 行・135 行（systemd が無い経路））

ensure は skill `command-team` の「1. サイクルの始め」の最初の項目で、指揮役が巡回のたびに実行します。unit の外で動く見張りを置き換えるのが ensure の目的です（Issue #285）。その判定を文字列の一致だけで行っているため、見張り以外も選びます。

2026-11-27 13:20:09（JST）に、修正前と修正後の選択式を同じ瞬間に比べました（`scripts/select-compare.sh`）。自分の `bash scripts/shellcheck.sh` と、リポジトリの外に置いた見張りの写しを動かしている状態で、修正前の選択式は 3 個を選びました（unit の見張り、自分の shellcheck、自分が置いた写し）。修正後の `watch_pids` は unit の見張り（pid 422）だけを選びました。

それぞれの失敗までの経路です。

- shellcheck タスク: `scripts/shellcheck.sh:42` が `shellcheck --severity=warning "${rest[@]}"` を起動します。`rest` は `scripts/` 以下のスクリプトのパスの一覧で、`scripts/conductor-watch.sh` を含みます。shellcheck が SIGTERM で終了すると bash は `Terminated` を出し、`|| rc=1`（39 行・42 行）が続き、45〜46 行が `findings above` を出して終了コード 1 で終わります。指摘は 0 件です。
- shellcheck の試験: 手順 5（`mise run shellcheck`）と手順 7（写しに対する `shellcheck.sh` の 3 回）が同じ形で shellcheck を起動するため、同じように終わります。
- slots の試験: `run_cycles`（124〜130 行）はログを空にしてから見張りを起動し、`wait` で終わりを待ちます。見張りの巡回は同期的な python です（`scripts/conductor-watch.sh:129`）。節 8 の巡回の途中で見張りの bash だけが kill されると、`wait` はすぐに返り、節 8 の確認は `SLOTS` の行がまだ無いまま失敗します。python は孤児として動き続け、あとから `SLOTS` の行をログに書きます。ログは `>>` で開いているため、この書き込みは節 9 が空にしたあとのログに入り、節 9 の「古い使用数の行が残っていないこと」の確認が落ちます。

## 修正案

Issue の対象外だったため調査だけの予定でしたが、開発リードからの指示（T3 スレッドのメッセージ、2026-11-26 21:1x JST 受領）で対策まで進めました。案と、それぞれの利点と危うさは次のとおりです。

- **A. 選択式を「見張りの形」に狭める（採用し、この PR で実装）。** コマンドラインを `/proc` の NUL 区切りで読み、引数が `<bash のパス> <…>/scripts/conductor-watch.sh` の 2 つだけであること、そしてそのスクリプトがこのリポジトリの checkout（このスクリプトの置き場所のリポジトリ、主 checkout、`git worktree list` に載る各 worktree）の中にあることを、`cd -P` で解決した物理パスで比べます。相対パスで起動された見張りは、起動してすぐリポジトリのルートへ移動するため、その移動後の cwd にある `scripts/conductor-watch.sh` とみなします。利点: ensure の元々の目的（unit の外で動く見張りの置き換え）を保ちます。殺されうるプロセスがこのリポジトリの見張りだけに減ります。shellcheck と conductor-watch の試験は変更不要です。危うさ: `git worktree list` に載る checkout だけを対象にしたため、消えた worktree や、同じリポジトリの別の clone から動いている見張りは置き換えの対象から外れます（修正前は文字列が一致すれば置き換えていました）。unit の見張りと並んで残ることになります。相対パスの見張りは、見張り自身がリポジトリのルートへ移動したあとの cwd から求めるため、別のディレクトリから `../…/scripts/conductor-watch.sh` の形で起動してまもなくの数ミリ秒は選ばれません（次の実行で選ばれます）。
- **B. ensure が記録した pid だけを kill する（pidfile）。** 利点: 選択が最も狭い。危うさ: 手で起動した見張りが対象から漏れ、#285 の目的を果たせません。pid の再利用に弱く、古い記録があると別のプロセスを kill します。
- **C. 被害を受ける側を頑丈にする。** shellcheck.sh が信号で止められたらやり直す、試験を名前空間の中で走らせる、`run_cycles` が外部の kill を検出して明示的に失敗する。利点: 原因がどこにあっても影響が小さい。危うさ: やり直しは本当の失敗を隠します。pid の名前空間は親から見えるため、host 側の pgrep による kill は防げません（`conductor-watch-ensure.test.sh` の見張りが巻き込まれてきた原因はこれだと推測します）。
- **D. 落ちた試験を回し直す仕組みを `script-test` に入れる。** 利点: 変更が小さい。危うさ: Issue が挙げたとおり本当の失敗を隠します。kill は巡回のたびに続くため、当たった試験を何度でも落とします。
- **E. 時間切れを見直す。** 該当しません。3 つの失敗に時間切れは関与していません（結論の 4.）。

### 修正後の確認

- 回帰テスト（`scripts/tests/conductor-watch-ensure.test.sh`）に次の 3 つを入れました。
  1. ensure が、`scripts/conductor-watch.sh` を引数に並べるだけのプロセスと、リポジトリの外の写しの見張りを選ばないこと。unit の外の見張りを置き換えるのと同じ 1 回の実行で確かめます（systemd・fallback の両方）。修正前（main の版）では 4 つの確認が落ちて、修正後は通ります。
  2. `./scripts/conductor-watch.sh` と手で起動した見張りが置き換わること。空白を含む checkout で続けて実行しても見張りが 1 本のままであること。この PR の最初の版（相対パスの前置きと空白を扱えていなかった）では 10 の確認が落ちて、修正後は通ります。
  3. conductor-watch の実際の試験（`conductor-watch-slots.test.sh`：単体の PID への kill。`conductor-watch-stall.test.sh`：`setsid` で起こした見張りのプロセスグループごとの kill）を、ensure の管理対象の見張りと並行して通しても、試験が通り、試験が立てた見張りが残らず、管理対象の見張りが残ること。これは修正の前後で変わらない、将来への見張りです（修正前の ensure でも通ります）。
- 修正前後で同じ実験を比べました（最終版の修正で実施）。ユーザー・pid・mount の名前空間の中で、slots の試験と `bash scripts/shellcheck.sh` を走らせている間、本物の ensure（`systemctl` はモック）を 0.1 秒間隔で実行し続けます。

| 版 | 反復 | slots の試験 | `bash scripts/shellcheck.sh` | ensure の実行 | ensure の kill |
|---|---|---|---|---|---|
| 修正前（origin/main の ensure） | 12 | 12 回すべて失敗 | 12 回すべて `Terminated` | 177 回 | 132 回 |
| 修正後（この PR の最終版） | 64 | 64 回すべて通過 | 64 回すべて通過 | 3494 回 | 0 回 |

- ローカルの `mise run verify` は 5 回走らせました（いずれも `TMPDIR=/dev/shm/p808`。理由は PR 本文に書いてあります）。1 回目（2026-11-27 00:20〜00:24）は `shellcheck の試験` だけが `FAIL mise run shellcheck failed (exit 1)` と `FAIL mise run shellcheck reported '<none>' files checked, want 86` で落ちました。出力は 1. と同じ形です（このときは 3. の観測をしていなかったため、送信元は記録されていません）。2 回目（09:48〜09:52）、3 回目（10:08〜10:12）、4 回目（11:17〜11:20）、5 回目（最終版での実行。時刻は PR 本文に書きます）は exit 0 でした。

修正が実際の kill を止めるには、指揮役が実行する ensure の版がこの修正を含む必要があります。マージのあとに、指揮役が使う checkout で main を取り込んでください。それまでは、開発機では従来どおりの巻き込みが続くと推定されます。

## 証拠のログ

ログは開発機のこの worktree の `target-ci/evidence-808/` に置いています（`.gitignore` の `/target-ci` に当たるため、PR には含まれません）。どれも自分が実行したコマンドの出力です。

| ファイル | 何のログか |
|---|---|
| `regression-red-check-vs-main.log` | `scripts/red-check origin/main --keep scripts/tests/conductor-watch-ensure.test.sh -- bash scripts/tests/conductor-watch-ensure.test.sh`。修正前（main）の ensure に新しい試験を当てたもので、終了コード 1 |
| `regression-red-check-vs-round1.log` | 同じ試験を、この PR の最初の版の ensure に当てたもの。終了コード 1 |
| `regression-green-round2.out` | 最終版での `bash scripts/tests/conductor-watch-ensure.test.sh`。`all checks passed` |
| `repro/shellcheck-task.log`、`repro/shellcheck-test.out`、`repro/slots-gated-{1,2,3}.out`、`repro/slots-timed-{1,19}.out` | 1. の出力 |
| `repeat-without-kill-results.txt` | 2. の反復ごとの終了コードと FAIL の行 |
| `observation.log`、`decoy-events.log`、`observation-summary.txt` | 3. の観測 |
| `selection-compare-round2.log` | 根本原因の節の選択式の比較 |
| `ab3/before-fix-results.txt`、`ab3/before-fix-kills.log`、`ab3/after-fix-results.txt` | 修正後の確認の修正前後の比較（最終版） |
| `verify-*.log` | ローカルの `mise run verify` |
| `scripts/` | 上のログを取るのに使ったスクリプト |

## 同じ欠陥が他の場所に

- ensure の systemd が無い経路（128 行・135 行）も同じ `watch_pids` を使うため、同じ巻き込みがありました。この修正で同時に狭まります。
- `conductor-watch-ensure.test.sh` が「環境の負荷で間欠失敗する」と作業ログに繰り返し書かれています（`docs/worklogs/issue-161.md`、`issue-202.md`、`issue-265.md`、`issue-330.md`、`issue-405.md`、`issue-411.md`、`issue-470.md`）。この試験の見張りも同じ文字列を持ち、pid の名前空間は親から見えるため、同じ原因である可能性があります。確かめてはいません。
- `scripts/tests/conductor-watch-ensure.test.sh` は `PELWAY_BOARD_QUEUE_DIR` を設定しません。試験の見張りが本物のボード Status 更新のキュー（`/tmp/pelway-board-queue`）を見ます。2026-11-26 22:30 時点のキューには `gh-read` による失敗の記録はありませんでした（完了 11 件、失敗 1 件は `bad-shape`）。この修正には含めていません。

## 未解決の問い

- 開発機で SIGTERM を送っているのが指揮役の ensure であることは、推定のままです（「再現」の 3.）。
- 修正後の A/B の 1 回目の実行の反復 30（2026-11-26 21:59:05）で shellcheck に届いた SIGTERM の送信元は、特定できませんでした。ensure は `flock` で直列化されるため、記録に残らない別の ensure の実行が先に kill した可能性があります。推測です。
- ローカルの `mise run verify` の 1 回目（2026-11-27 00:24）で `shellcheck の試験` を落とした SIGTERM の送信元も、観測をしていなかったため記録にありません。出力は 1. と同じ形です。
- 次は同じ機構では説明できず、調査していません。`.agents/skills/verify-pelway/verify-run.test.sh` の `make_run_dir` と `locks_dir_ready` の失敗（この sandbox では `/tmp` の所有者が uid 65534 のため、単独でも落ちます）。
- 指揮役が ensure をどの checkout から実行しているかは、確かめていません（3. の観測では、そのプロセスの cwd は読めませんでした）。

## 使ったスキル

`.agents/skills/investigate-bug`、`.agents/skills/implement-issue`
