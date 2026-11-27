# `mise run verify` のスクリプトの試験が、指揮役の巡回の開始と重なったときだけ落ちる

調査の対象は Issue #808 です。起票時に挙がっていた失敗は 3 つで、この報告では次の名前で呼びます。

- 「slots の試験」: `scripts/tests/conductor-watch-slots.test.sh`
- 「shellcheck の試験」: `scripts/tests/shellcheck.test.sh`
- 「shellcheck タスク」: `mise run shellcheck`（本体は `scripts/shellcheck.sh`）

## 結論

3 つの失敗の原因は同じ 1 か所です。指揮役（チームのエージェントに Issue を割り当てるスレッド）が巡回の最初に毎回実行する `scripts/conductor-watch-ensure.sh`（以下「ensure」）が、コマンドラインに `conductor-watch.sh` という文字列を含むプロセスをすべて「見張り」とみなし、SIGTERM で終了させています。見張りとは `scripts/conductor-watch.sh`（T3 のスレッドと PR の状態変化を出し続ける常駐スクリプト）です。

巻き込まれるのは、見張りではないのに同じ文字列をコマンドラインに持つ 2 種類のプロセスです。

1. shellcheck の本体。`scripts/shellcheck.sh` が検査対象のスクリプトのパスを引数に並べて起動するため、`scripts/conductor-watch.sh` がコマンドラインに含まれます。
2. conductor-watch の試験が一時ディレクトリに置いた、見張りの写し。写しのファイル名は本物と同じ `conductor-watch.sh` です。

`mise run verify` の並列数、負荷、時間切れは関係しません。同じ verify の中で並列に走る他のタスクも関係しません。関係するのは、ensure が実行された瞬間に上の 2 種類のプロセスが生きていたかどうかだけです。

この報告の根拠は、この調査で自分が実行した試験と観測のログだけです（「証拠のログ」の節）。ほかのエージェントの出力は根拠に使っていません（開発リードの指示による）。

## 報告の読み取り

- 期待: `mise run verify` が通る。
- 実際: 3 つの失敗は、それぞれ単独で回すと通り、全体を回し直すと通った（Issue #808 の本文）。落ちたときの出力は、slots の試験では `FAIL SLOTS lines return after toml fixed` と `FAIL no stale usage on fetch failure`、shellcheck タスクでは `Terminated`（指摘 0 件）でした。
- 対象: 開発機 minisforum（WSL2、Ubuntu 24.04、論理 CPU 24）。複数のエージェントのスレッドが同時に `mise run verify` を回し、指揮役が約 8 分ごとに巡回している環境。
- 場所: `mise run verify` の `shellcheck` タスクと `script-test` タスク。
- 時刻: 報告は 2026-11-26 です（Issue #808 の起票）。この調査の再現と観測は 2026-11-26 19:58 から 2026-11-27 10:58 まで（JST）に行いました。
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

ensure が SIGTERM を送ることは、修正前の ensure を自分で実行して確かめました（「修正後の確認」の修正前後の比較。修正前の ensure は 201 回の実行のうち 149 回で、試験の見張りと shellcheck を kill しました）。

開発機で動いている指揮役の ensure が同じことをしているかは、次の観測で確かめようとしました。2026-11-27 10:46:42 から 11:03:42（JST）まで、コマンドラインに `scripts/conductor-watch.sh` を持つ自分のプロセスを 2 つ（shellcheck と同じ形、リポジトリの外の写しの見張りと同じ形）立てて SIGTERM を受け取った時刻を記録し、同時に `/proc` を 0.1 秒ごとに見て、コマンドラインの末尾が `conductor-watch-ensure.sh` のプロセスを見つけた時刻を記録しました。この 17 分間では、2 つのプロセスに SIGTERM は届かず、指揮役による ensure の実行も見つかりませんでした。見つかった 6 件は、ほかのスレッドの verify が名前空間の中で走らせた試験の写しです（名前空間の中からは自分のプロセスは見えず、信号も送れません）。この窓の外で起きた kill の回数と送信元は、未集計です。

したがって「開発機で SIGTERM を送っているのは指揮役の巡回の ensure である」ことは、直接に観測した事実ではなく、次の 3 つからの推定です。(1) skill `command-team` の「1. サイクルの始め」は指揮役に毎回 ensure を実行させる。(2) 上の 2. と、後述のローカルの verify の 1 回目で、自分は kill していないのに 1. と同じ出力で落ちた。(3) 修正前の ensure は、実行されると 1. と同じ SIGTERM を同じ選択で送る。確定させるには、指揮役の巡回と重なる時間帯に `scripts/observe.sh` を走らせて、ensure のプロセスを見つけた時刻と SIGTERM の到達の時刻が一致することを見ます。

### 4. 3 つの失敗の再現条件

| 失敗 | 条件 | 窓の長さ（1 回の verify あたり） | 再現 |
|---|---|---|---|
| shellcheck タスク | verify の開始直後、`--severity=warning` の shellcheck が生きている間に ensure が実行される | 約 5 秒（負荷が高いときは 17 秒まで） | した（1. で 1/1、2. で 1/94。3. の観測では届かなかった） |
| shellcheck の試験 | 手順 5 の `mise run shellcheck` の同じ shellcheck が生きている間に ensure が実行される | 約 5 秒（verify の終わり近く） | した（1. で 1/1） |
| slots の試験 | 節 8 の巡回の python が `SLOTS` の行を出す前に ensure が実行される | 0.3 秒未満（節 8 の見張しの寿命は 0.1〜0.3 秒） | した（1. で 5/5） |

並列の数は 1 でも再現します。上の 1. の再現はすべて、試験を 1 本だけ単独で走らせた状態で行いました。負荷も時間切れも必要ありません。

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

ensure は skill `command-team` の「1. サイクルの始め」の最初の項目で、指揮役が巡回のたびに実行します。unit の外で動く見張しを置き換えるのが ensure の目的です（Issue #285）。その判定を文字列の一致だけで行っているため、見張り以外も選びます。

2026-11-27 11:10:52（JST）に、修正前と修正後の選択式を同じ瞬間に比べました（`scripts/select-compare.sh`）。自分の `bash scripts/shellcheck.sh` と、リポジトリの外に置いた見張りの写しを動かしている状態で、修正前の選択式は 5 個を選びました（unit の見張り 1、自分の shellcheck 1、自分が置いた写し 1、ほかのプロセス 2。ほかのプロセスの中身は記録していません）。修正後の `watch_pids` は unit の見張り（pid 422）だけを選びました。

それぞれの失敗までの経路です。

- shellcheck タスク: `scripts/shellcheck.sh:42` が `shellcheck --severity=warning "${rest[@]}"` を起動します。`rest` は `scripts/` 以下のスクリプトのパスの一覧で、`scripts/conductor-watch.sh` を含みます。shellcheck が SIGTERM で終わすると bash は `Terminated` を出し、`|| rc=1`（39 行・42 行）が続き、45〜46 行が `findings above` を出して終了コード 1 で終わります。指摘は 0 件です。
- shellcheck の試験: 手順 5（`mise run shellcheck`）と手順 7（写しに対する `shellcheck.sh` の 3 回）が同じ形で shellcheck を起動するため、同じように終わります。
- slots の試験: `run_cycles`（124〜130 行）はログを空にしてから見張しを起動し、`wait` で終わりを待ちます。見張しの巡回は同期的な python です（`scripts/conductor-watch.sh:129`）。節 8 の巡回の途中で見張りの bash だけが kill されると、`wait` はすぐに返り、節 8 の確認は `SLOTS` の行がまだ無いまま失敗します。python は孤児として動き続け、あとから `SLOTS` の行をログに書きます。ログは `>>` で開いているため、この書き込みは節 9 が空にしたあとのログに入り、節 9 の「古い使用数の行が残っていないこと」の確認が落ちます。

## 修正案

Issue の対象外だったため調査だけの予定でしたが、開発リードからの指示（T3 スレッドのメッセージ、2026-11-26 21:1x JST 受領）で対策まで進めました。案と、それぞれの利点と危うさは次のとおりです。

- **A. 選択式を「見張りの形」に狭める（採用し、この PR で実装）。** コマンドラインが `<bash のパス> <…>/scripts/conductor-watch.sh` の 2 つだけであること、そしてそのスクリプトがこのリポジトリの checkout（主 checkout と各 worktree）の中にあること、の 2 段で選びます。利点: ensure の元々の目的（unit の外で動く見張しの置き換え）を保ちます。殺されうるプロセスがこのリポジトリの見張りだけに減ります。shellcheck と conductor-watch の試験は変更不要です。危うさ: 相対パスで起動された見張りは `/proc/<pid>/cwd` から解決するため、見張しが `cd` する前の一瞬は選ばれません（次の巡回で選ばれます）。リポジトリの外に置いた写しで動く本物の見張りは置き換えの対象から外れます。symlink を含むパスでは文字列の一致が失敗して選ばれません（kill しない側に倒れます）。
- **B. ensure が記録した pid だけを kill する（pidfile）。** 利点: 選択が最も狭い。危うさ: 手で起動した見張りが対象から漏れ、#285 の目的を果たせません。pid の再利用に弱く、古い記録があると別のプロセスを kill します。
- **C. 被害される側を頑丈にする。** shellcheck.sh が信号で止められたらやり直す、試験を名前空間の中で走らせる、`run_cycles` が外部の kill を検出して明示的に失敗する。利点: 原因がどこにあっても影響が小さい。危うさ: やり直しは本当の失敗を隠します。pid の名前空間は親から見えるため、host 側の pgrep による kill は防げません（`conductor-watch-ensure.test.sh` の見張しが巻き込まれてきた原因はこれだと推測します）。
- **D. 落ちた試験を回し直す仕組みを `script-test` に入れる。** 利点: 変更が小さい。危うさ: Issue が挙げたとおり本当の失敗を隠します。kill は約 8 分ごとに続くため、当たった試験を何度でも落とします。
- **E. 時間切れを見直す。** 該当しません。3 つの失敗に時間切れは関与していません（`mise.toml` に timeout の設定はなく、1. と 2. の失敗の出力に timeout を示す文字はありません）。

### 修正後の確認

- 回帰テスト（`scripts/tests/conductor-watch-ensure.test.sh`）: 名前空間の中で、ensure が (1) `scripts/conductor-watch.sh` を引数に並べるだけのプロセスと (2) リポジトリの外の写しの見張りを選ばないことを、unit の外の見張しを置き換えるのと同じ 1 回の実行で確かめます。修正前は 4 つの確認が落ちて、修正後は全部通りました。名前空間の外では、本番と同じ形で起動したプロセスを立てて conductor-watch の試験をその横で走らせ、そのプロセスが残ることを確かめます。
- 修正前後で同じ実験を比べました。ユーザー・pid・mount の名前空間の中で、slots の試験と `bash scripts/shellcheck.sh` を走らせている間、本物の ensure（`systemctl` はモック）を 0.1 秒間隔で実行し続けます。修正前（main の版）は 12 回の反復すべてで両方が落ちて、ensure は 201 回の実行のうち 149 回で kill しました。修正後は 2 回の実行で 124 回の反復のうち 123 回で両方が通り、ensure は合計 7243 回の実行で kill しませんでした。残る 1 回（1 回目の実行の反復 30）は shellcheck だけが `Terminated` で落ちましたが、名前空間の中の修正後の ensure はその反復で 58 回実行して kill しておらず、SIGTERM は名前空間の外から来たものです（下の「未解決の問い」）。2 回目の実行中の 23:11:42 には、修正がまだ入っていない本物の ensure（指揮役の巡回）が、名前空間の中のモックの unit の見張しを kill しました。名前空間の中の修正後の ensure はそれを検出して立て直し、反復の結果には影響しませんでした。
- ローカルの `mise run verify` は 3 回走らせました（いずれも `TMPDIR=/dev/shm/p808`。理由は PR 本文に書いてあります）。1 回目（2026-11-27 00:20〜00:24）は `shellcheck の試験` だけが `FAIL mise run shellcheck failed (exit 1)` と `FAIL mise run shellcheck reported '<none>' files checked, want 86` で落ちました。出力は 1. と同じ形です（このときは 3. の観測をしていなかったため、送信元は記録されていません）。2 回目（09:48〜09:52）と 3 回目（10:08〜10:12）は exit 0 でした。

修正が実際の kill を止めるには、指揮役が実行する ensure の版がこの修正を含む必要があります。マージのあとに、指揮役が使う checkout で main を取り込んでください。それまでは、開発機では従来どおりの巻き込みが続きます。

## 証拠のログ

ログは開発機のこの worktree の `target-ci/evidence-808/` に置いています（`.gitignore` の `/target-ci` に当たるため、PR には含まれません）。どれも自分が実行したコマンドの出力です。

| ファイル | 何のログか |
|---|---|
| `regression-red-check.log` | `scripts/red-check HEAD --keep scripts/tests/conductor-watch-ensure.test.sh -- bash scripts/tests/conductor-watch-ensure.test.sh`。修正前の ensure に新しい試験を当てたもので、終了コード 1 |
| `regression-green.log` | 修正後の `bash scripts/tests/conductor-watch-ensure.test.sh`。`all checks passed` |
| `repro/shellcheck-task.log`、`repro/shellcheck-test.out`、`repro/slots-gated-{1,2,3}.out`、`repro/slots-timed-{1,19}.out` | 1. の出力 |
| `repeat-without-kill-results.txt` | 2. の反復ごとの終了コードと FAIL の行 |
| `observation.log`、`decoy-events.log` | 3. の観測 |
| `selection-compare.log` | 根本原因の節の選択式の比較 |
| `ab/before-fix-results.txt`、`ab/before-fix-kills.log`、`ab/after-fix-results-{1,2}.txt` | 修正後の確認の修正前後の比較 |
| `verify-2026-11-27T0020-hit.log`、`verify-2026-11-27T0948.log`、`verify-2026-11-27T1008.log` | ローカルの `mise run verify` 3 回 |
| `scripts/` | 上のログを取るのに使ったスクリプト |

## 同じ欠陥が他の場所に

- ensure の systemd が無い経路（128 行・135 行）も同じ `watch_pids` を使うため、同じ巻き込みがありました。この修正で同時に狭まります。
- `conductor-watch-ensure.test.sh` が「環境の負荷で間欠失敗する」と作業ログに繰り返し書かれています（`docs/worklogs/issue-161.md`、`issue-202.md`、`issue-265.md`、`issue-330.md`、`issue-405.md`、`issue-411.md`、`issue-470.md`）。この試験の見張りも同じ文字列を持ち、pid の名前空間は親から見えるため、同じ原因である可能性があります。確かめてはいません。
- `scripts/tests/conductor-watch-ensure.test.sh` は `PELWAY_BOARD_QUEUE_DIR` を設定しません。試験の見張りが本物のボード Status 更新のキュー（`/tmp/pelway-board-queue`）を見ます。2026-11-26 22:30 時点のキューには `gh-read` による失敗の記録はありませんでした（完了 11 件、失敗 1 件は `bad-shape`）。この修正には含めていません。

## 未解決の問い

- 修正後の A/B の 1 回目の実行の反復 30（2026-11-26 21:59:05）で shellcheck に届いた SIGTERM の送信元は、特定できませんでした。ensure は `flock` で直列化されるため、記録に残らない別の ensure の実行が先に kill した可能性があります。推測です。
- ローカルの `mise run verify` の 1 回目（2026-11-27 00:24）で `shellcheck の試験` を落とした SIGTERM の送信元も、観測をしていなかったため記録にありません。出力は 1. と同じ形です。
- 次は同じ機構では説明できず、調査していません。`.agents/skills/verify-pelway/verify-run.test.sh` の `make_run_dir` と `locks_dir_ready` の失敗（この sandbox では `/tmp` の所有者が uid 65534 のため、単独でも落ちます）。
- 指揮役が ensure をどの checkout から実行しているかは、確かめていません（3. の観測では、そのプロセスの cwd は読めませんでした）。

## 使ったスキル

`.agents/skills/investigate-bug`、`.agents/skills/implement-issue`
