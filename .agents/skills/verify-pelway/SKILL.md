---
name: verify-pelway
description: "Pelway のデモのバックエンド（pelway-http の HTTP API と runner-lite。チャンネル #案件B、ハルへのメンション、A/B 案、判断待ち、決定、決定後のメール下書き）を実際に起動して利用者と同じ道筋で動かし、証拠を残して合否を出す。Use when asked to verify Pelway by running it, check the demo flow end to end, or collect run evidence for a PR or QA. Keywords: verify-pelway, 実際に動かして確かめる, デモの流れ, 端から端まで, run の証拠, QA 確認"
---

# Pelway を実際に動かして確かめる

デモの流れ（チャットで人と AI が案を出し合い、決まった内容からメールの下書きを作る）を、利用者と同じ HTTP の道筋で動かし、証拠を残して合否を出す。機械の部分はこのファイルと同じ場所の `verify-run` が 1 回の run で受け持つ。

```bash
.agents/skills/verify-pelway/verify-run --feature decision   --label <Issue 番号など>
.agents/skills/verify-pelway/verify-run --feature mail-draft --label <Issue 番号など>
```

- `--feature` は `features/README.md` の表の名前。省略すると `decision`。
- 証拠の置き場は、共有 git dir（`git rev-parse --path-format=absolute --git-common-dir`）の `pelway/evidence/<label>-<sha>/<run-id>/`。実装役も QA もここに揃える（Issue #568 なら `--label 568` で `pelway/evidence/568-<sha>/`）。`~/.local/state` と `/tmp` は使わない。`--evidence-root` で変えられるが、作業ツリーの中（共有 git dir を除く）は拒否する（終了コード 2）。
- 終了コード: 0 = 全段 PASS、1 = どこかが FAIL か、未対応・道具が無い（`summary.txt` か標準エラーに理由）、2 = 引数の誤り。
- run は前景の 1 コマンドで終わる。サーバーを run の外で動かし続けない（`nohup`・`setsid`・`systemd-run` を使わない。pitchfork にも登録しない）。

前提（run の前に 1 回。WSL（Ubuntu）で通す。コンテナは使わない）:

1. `mise trust`（trust 済みの checkout・worktree だけで行う。ユーザーの mise 状態を書き換えるため）→ `mise install --locked` → `mise run fetch`（crates.io に接続する準備。run 本体は `--offline --locked`）
2. 道具: `git`・`mise`・`rustup`・`jq`・`ss`（iproute2）・`pgrep`/`ps`（procps）・`sha256sum`・`realpath`・`readlink`・`comm`・`timeout`・`/proc`・bash の `/dev/tcp`。欠けていれば `verify-run` は「失敗（道具が無い）」で止まる。勝手に入れない。使った版は `tools.txt` に残る。
3. `fnox exec` の下で動かさない。`verify-run` は自分の環境に fnox の秘密の変数があると止まる。
4. Linux 以外（macOS など）では `verify-run` は「未対応（理由）」で終了コード 1 になる（`/proc`・`ss` が無い）。Mac での手順は未検証で、置き換え先は `lsof -p`・`lsof -iTCP -a -p <pid>`・`shasum -a 256`。

守ること:

- AI は台本モードだけで動かす。`PELWAY_LLM_API_KEY`・`PELWAY_LLM_CUSTOM_BASE_URL`・`PELWAY_LLM_BASE_URL` は設定しないし、参照もしない（`verify-run` は起動の前に、自分の環境にこれらと fnox の秘密が無いことを記録する）。`crates/pelway-runner/tests/real_llm.rs` は使わない。外部への実 I/O（AI・メール）と費用は発生させない。
- テスト用データは `DataStore::new_demo_seeded` の合成 seed だけ。実在の人・会社・アドレスは足さない。
- ログと証拠に、投稿の本文や秘密を出さない（`verify-run` は証拠の `body`・`question` を長さだけ残して伏せる）。

各章の「失敗」に当たったら、その run の合否は FAIL。通すために確かめ方を弱めない（待ち時間を延ばす、段を飛ばす、`--allow-dirty` の run を証拠に使う、など）。前提が足りない流れは `features/<名前>.md` の `Gotchas` に理由を書き、「失敗（理由）」として報告する。

## 1. 起動

- 依存の取得: `mise run fetch`（初回だけ。`verify-run` は `--offline --locked` でビルドする）。
- 二重起動の確認（ビルドの前、run の最初）: まず `<共有 git dir>/pelway/locks/<作業ツリーの実体のパスの sha256 の先頭 12 桁>` を `mkdir` で原子的に取り、取れなければ（同じ作業ツリーの別の run が実行中）FAIL で止まる。ロックの場所と鍵は `--evidence-root` やビルド（mbx が `target` をリンクにする）で変わらない。続けて、同じ target（`${CARGO_TARGET_DIR:-<top>/target}` の実体）の下の `demo-serve` が動いていれば（手で起動したものなど）、触らずに FAIL で止まる（止めるかは人が決める）。ビルドの後にも、同じ実行ファイルの `demo-serve` が動いていないかをもう一度見る（新しい worktree の最初の run では、ビルドの前に `target` の実体が決まっていないため）。動いているほうの exe はビルドし直すと ` (deleted)` が付くので、外して比べる。ロックは後片付けで外す。
- ビルドと起動: `verify-run` が `RUSTC=<mise.toml の rust のツールチェーンの rustc> mise exec -- cargo build --offline --locked -p pelway-http --example demo-serve --message-format=json-render-diagnostics` を実行し（rustc は `mise current rust` と `rustup which --toolchain` で決める。`mise exec` の PATH では、ほかの mise 呼び出しと重なると rustc が `/usr/bin` の別の版に解決されることがあるため）、cargo が報告した実行ファイル（mr_boxington のときは `~/.cache/mbx/targets/...` の下）を `env -i PATH=/usr/bin:/bin HOME=$HOME` の子として起動する。子の標準入力は run が持つパイプで、run が終われば（SIGKILL でも）閉じる。起動口 `crates/pelway-http/examples/demo-serve.rs` は標準入力の EOF で終わる。`127.0.0.1:0` で待ち受け（mise の `PELWAY_BIND_ADDR` は読まない）、LLM の設定を「どの変数も無い」写像で組み立てる（環境変数を参照しない）ので、台本モード（`ScriptGateway`）になる。
- 空のデータから使える状態まで: マイグレーションは無い（保存はメモリだけ）。テスト用データは起動時のデモ seed（ワークスペース `ws-demo-01`、チャンネル `ch-demo-anken-b`「案件B」、田中 `m-tanaka`・山本 `m-yamamoto`・佐藤 `m-sato`・ハル `m-hal`（bot）、過去の発言 3 件）。
- 使える合図: 実際に待ち受けたポートを書いた標準出力（証拠の `server.stdout`）が 1 行だけで `demo-serve ready addr=127.0.0.1:<1〜65535> pid=<pid> llm=script` の形（`pid` が起動した子の PID と一致すること）、2 章の環境チェック（PID・バイナリ・待ち受けの照合）を通ったこと、そのあと seed を HTTP で読めること（`GET /v1/workspaces/ws-demo-01/channels?actor=m-tanaka` がチャンネル 1 件でメンバーの ID が `m-hal,m-sato,m-tanaka,m-yamamoto`、`GET /v1/channels/ch-demo-anken-b/messages?actor=m-tanaka&after_seq=0` が 3 件）。ポートが開いただけでは合図にしない。

失敗: 道具が無い。同じ target の `demo-serve` が動いている、またはロックが取れない。ビルドが通らない（`build.log`）。5 秒以内に ready 行が出ない、または ready 行の `pid` が起動した子と違う。seed のチャンネル・メンバーの ID・件数が違う（テスト用データが無い）。

二重起動を拒むことの確かめ方（QA）:

- 同時に始めた run: `verify-run --label <N> & sleep 3; verify-run --label <N>; wait`。2 つめが `FAIL 起動: 二重起動の確認（別の run がロックを持っている…）` で止まり、1 つめは最後まで通る。
- 先に起動しておいたインスタンス: 一度 run を通してバイナリを作ったあと、`(sleep 120 | target/debug/examples/demo-serve > /dev/null) &` で 1 つ起動してから `verify-run --label <N>`。`FAIL 起動: 二重起動の確認（同じ target の demo-serve が動いている pid=…）` で止まり、先のインスタンスは触られずに動き続ける（`sleep` が終わると標準入力が閉じて自分で終わる）。

## 2. 環境チェック

`verify-run` の `env_check` が、読み取りだけで「このインスタンスを動かしてよいか」に答え、`env-check.txt` に残す。

- プロセス: `/proc/<pid>/exe` がこの run でビルドした `demo-serve` で、その sha256 が起動前に測った値と同じ。使った道具の版は `tools.txt`（`mise`・ツールチェーンの `rustc`・`cargo`・`ss`・`pgrep` など）。
- ビルドと sha: `git rev-parse HEAD` が run の開始時と同じ。作業ツリーに未コミットの変更があれば起動前に FAIL（`--allow-dirty` は手元の試しだけに使い、その run は証拠にしない）。
- ポート: `ss -tlnp` で、この PID の TCP の待ち受けが ready 行の `addr` だけ。`ss -ulnp` で、この PID の UDP のソケットが 0 個。この照合を通るまで HTTP は送らない。
- 環境: 子の環境の変数の名前の数（`env_name_count`）と、fnox の秘密 3 つ（LLM のキー・Penpot MCP のトークン・DB の URL）と LLM の接続先 2 つ（独自の接続先・既知の接続先）が無いこと（`forbidden_env=none`）。名前の有無だけを見て、値は残さない。証拠には変数名も書かずラベルだけ書く（変数名は leak-phrases の禁止語）。
- 台本モード: 標準エラー（`server.stderr`）に接続先を知らせる行 `pelway-runner: LLM endpoint: ...` があり、ready 行が `llm=script`。台本の合成応答が出たことは 3 章の段 4 で確かめる。
- 認証: デモは認証なし。操作者は合成ヘッダー `x-pelway-member-id`・`x-pelway-member-kind` で名乗る。
- 二重起動・共有インスタンス: 起動前に、同じ target の下の `demo-serve` が動いていれば FAIL にして止まり、そのプロセスには触らない（1 章）。接続先は自分の ready 行の `addr` だけで、ほかのポートのインスタンスには要求を送らない。

失敗: 上のどれか 1 つでも合わない（`env-check.txt` の `NG:` 行）。別のインスタンスだった場合も同じ。

## 3. 操作

デモの流れを利用者と同じ HTTP の経路で動かす（手がかりは API の経路と合成ヘッダー。経路の定数は `crates/pelway-contract` の `CHANNEL_MESSAGES_PATH`・`DECISIONS_PATH`・`DECIDE_PATH`）。テスト専用の経路や、内部の値を直接書き換えることはしない。機能ごとの経路・手順・既知の失敗は `features/` にある。`verify-run` が送る要求は次の順:

1. （1 章の使える合図）seed の一覧と過去の発言を HTTP で読む（`steps/NN-seed-channels`・`NN-seed-messages`）
2. `POST /v1/channels/ch-demo-anken-b/messages`（田中）`{"client_msg_id": "cm-verify-<run_id>", "body": "@ハル 納期延長の案を2つ作って <canary>", "thread_root": null}` — 200 と `message_id`。`<canary>` は `scripts/demo-check/canaries.txt` の `PELWAY_CANARY_BODY_7f3a`
3. 2 と同じ要求の再送 — 200 と同じ `message_id`
4. `GET /v1/workspaces/ws-demo-01/decisions?actor=m-yamamoto` を最大 10 秒、1 秒ごと — `request_message_id` が 2 の投稿の判断待ちが 1 件だけ。案A・案B が台本の合成応答（「納期を2週間延ばす案」「範囲を絞って納期を守る案」）
5. `GET /v1/channels/ch-demo-anken-b/messages?actor=m-tanaka&after_seq=<2 の seq - 1>` — 依頼の投稿が 1 件だけ（再送で重複しない）、ハルの投稿に `案A: ...` と `案B: ...` がある
6. `POST /v1/decisions/<decision_id>/decide`（山本）`{"decision_id", "version", "option_id": "a"}` — 200 と `selected.option_id == "a"`、`selected.body` が台本の案A
7. 6 と同じ選択を確定後の `version` で再送 — 200 で 6 と同じ結果（確定した決定を 1 件読む GET は無いので、保存された選択 a と案A の本文はこの応答で読み直す）
8. 確定後の `version` で `option_id: "b"` — 409 `already_decided`
9. 決定後の状態の取り直し: `GET .../decisions?actor=m-yamamoto`（確定した決定が無い）と `GET .../messages?actor=m-tanaka&after_seq=0`（依頼の投稿が 1 件だけ保存されている）
10. （`mail-draft` だけ）10 秒待ち、5 の最後の seq より後のハルの投稿に、選んだ案A の本文があるか。あっても、下書きを保存先から取り直す経路が main に無いので合格にしない（失敗（機能なし））

既存テストで同じ道筋を確かめるときは `cargo test -p pelway-http --test runner_mention --test decisions --test channels`（フェイクの LLM。実提供元には接続しない）。テストは run の証拠の代わりにならない。

失敗: 要求が期待と違う状態コード・接続できない、10 秒で判断待ちが現れない、案が台本と違う、再送で投稿・判断待ちが増える、再決定の結果が変わる、別の選択への変更が通る、期待した投稿・状態が API に出ない。

## 4. 証拠

- 置き場: `<共有 git dir>/pelway/evidence/<label>-<HEAD の sha>/<UTC 時刻>-<feature>-<run の PID>/`。作業ツリーの外で、run ごと・sha ごとに分かれる。
- 中身: `summary.txt`（run_id・sha・dirty・feature・段ごとの `PASS`/`FAIL`・verdict）、`steps/NN-<名前>.req` と `.resp`（送った要求と受け取った応答。NN は送った順で、判断待ちを待つ間は `decisions-pending` が複数並ぶ。`body`・`question` は `<redacted len=N>` に伏せる）、`env-check.txt`、`tools.txt`、`build.json`（cargo の報告）、`survivors-before.txt`・`survivors-after.txt`、`network.txt`（要求ごとと 0.1 秒ごとに `ss -tuanp` で取った、この PID と待ち受けのアドレスに関わる TCP・UDP の行と、最後の集計行）、`proc-fd.txt`（開いているファイル）、`server.stdout`・`server.stderr`、`build.log`、`cleanup.txt`、`demo-check.txt` と `demo-check-input/`（demo-check に渡した写し）、`SHA256SUMS`。
- 成功の基準:
  - 利用者の道筋（公開の HTTP 経路と合成ヘッダー）だけで動かす。テスト専用の setter・内部の状態を直接読む経路は使わない。
  - 操作（`.req`）と結果の状態（`.resp`）の両方を残す。
  - 副作用を API で読み直す: 投稿が 1 件だけ保存されたこと、決定が確定して判断待ちから消えたこと、下書きが作られたこと（段 10）。保存はメモリだけなので、決定後の状態は HTTP で取り直したもの（段 9）を証拠にする。
  - 外に出ていないことを観察で確かめる: 操作の間、要求ごとと 0.1 秒ごとに `ss -tuanp` を取り（TCP と UDP。この PID と待ち受けのアドレスに関わる行）、`network.txt` の集計行が「loopback 以外との接続 0 件、DB（:5432）宛て 0 件」で観察が 1 行以上。`proc-fd.txt` は標準入出力と、`ss -tuanpe` の `ino:` で TCP・UDP と確かめられたソケットだけ（確かめられないソケットがあれば FAIL）。`mise run demo-check` の合格だけでは足りない。観察はスナップショットなので、0.1 秒より短い接続と、TCP・UDP 以外の通信路は未検証（観察の限界。合格の行にもそう書く）。
  - 証拠のディレクトリとファイルは所有者だけが読める（`umask 077`）。証拠の写しが全部そろったときだけ demo-check を流し、`SHA256SUMS` は作ったあとに `sha256sum -c` で確かめる。どちらかに失敗したら FAIL か終了コード 1。
  - 後片付けの後に、run のログ・証拠の写しに対して `scripts/demo-check/canary.sh` と `leak-phrases.sh` を `DEMO_CHECK_ROOT` で流し、どちらも exit 0（本文の canary と禁止語が証拠に出ていない）。
- 報告には `summary.txt` の全行と、最後に出る `evidence=<パス>`・`evidence_sha256=<SHA256SUMS の sha256>` を貼る。

失敗: `summary.txt` に `FAIL` 行がある。loopback 以外との接続か DB 宛ての接続が 1 件でもある、または観察が 0 行。`proc-fd.txt` に標準入出力・ソケット以外のファイルがある。demo-check の canary か leak-phrases が exit 0 以外。

## 5. 後片付け

- `verify-run` の終了時（成功・失敗・中断のどれでも）に、この run が起動した PID だけを止める。まず子の標準入力を閉じ（子は EOF で終わる）、5 秒で終わらなければ SIGTERM、さらに 5 秒で終わらなければ SIGKILL。シグナルを送る前に毎回 `/proc/<pid>/exe` がこの run のバイナリであることを確かめる。プロセス名では kill しない。経過は `cleanup.txt`。
- `verify-run` 自身が SIGKILL されたときは後片付けが動かないが、子の標準入力のパイプが閉じるので子も終わる。ロック（`<共有 git dir>/pelway/locks/` の下）は残るので、次の run は「ロックが取れない」で止まる。持ち主の pid（`locks/<id>/pid`）が無いことを人が確かめてから外す。
- 生き残りの確認: 起動の前と後片付けの後に、`demo-serve`・`pitchfork`・`pelway-agent-watch`・`t3-*` のうち 5 分以上動いているものの PID を取り（`survivors-before.txt`・`survivors-after.txt`）、前にあったものが後にもあることを確かめる。pitchfork の定期ジョブのように run の途中で自然に終わる短命なプロセスは比べない。`pitchfork list` も前後で `pitchfork-before.txt`・`pitchfork-after.txt` に残す。`pitchfork list` は supervisor が止まっていると起動してしまうので、起動しない `pitchfork supervisor status` で動いていると確かめられたときだけ、自動起動を止めて（`PITCHFORK_SUPERVISOR_AUTO_START=false`）呼ぶ。止まっていれば「supervisor が動いていない」とだけ書き、起動しない。
- 一時状態はサーバーのメモリだけ（本文を伏せる前の応答も、`verify-run` のメモリだけで扱いファイルに書かない）。証拠は消さない。
- 後片付けの後に、証拠が残っていることを確かめる: `(cd <evidence> && sha256sum -c SHA256SUMS)` がすべて `OK`。

- 中断（Ctrl-C・SIGTERM・コマンドの時間切れ）でも後片付けは動く。run が最後まで進まなかったときは、記録した段が全部 PASS でも `FAIL run が最後まで進まなかった（中断）` を書き、verdict は FAIL になる。

失敗: run が最後まで進まなかった（中断）。`cleanup.txt` が `NG:`（PID が残っている、exe が違うので送らなかった）。前にあったほかのプロセスが消えている。`sha256sum -c` に `OK` 以外がある。
