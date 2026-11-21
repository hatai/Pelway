---
name: verify-pelway
description: "Pelway のデモのバックエンド（pelway-http の HTTP API と runner-lite。チャンネル #案件B、ハルへのメンション、A/B 案、判断待ち、決定、決定後のメール下書き）を実際に起動して利用者と同じ道筋で動かし、証拠を残して合否を出す。Use when asked to verify Pelway by running it, check the demo flow end to end, or collect run evidence for a PR or QA. Keywords: verify-pelway, 実際に動かして確かめる, デモの流れ, 端から端まで, run の証拠, QA 確認"
---

# Pelway を実際に動かして確かめる

デモの流れ（チャットで人と AI が案を出し合い、決まった内容からメールの下書きを作る）を、利用者と同じ HTTP の道筋で動かし、証拠を残して合否を出す。機械の部分はこのファイルと同じ場所の `verify-run` が 1 回の run で受け持つ。

```bash
.agents/skills/verify-pelway/verify-run --feature decision   --evidence-root <リポジトリ外の絶対パス>
.agents/skills/verify-pelway/verify-run --feature mail-draft --evidence-root <リポジトリ外の絶対パス>
```

- `--feature` は `features/README.md` の表の名前。省略すると `decision`。
- `--evidence-root` の既定は `${XDG_STATE_HOME:-$HOME/.local/state}/pelway-verify`。リポジトリの中は拒否する（終了コード 2）。sandbox が既定の場所への書き込みを拒否したら、書ける場所（エージェントの scratchpad など、リポジトリ外）を渡す。
- 終了コード: 0 = 全段 PASS、1 = どこかが FAIL（`summary.txt` に理由）、2 = 引数の誤り。
- run は前景の 1 コマンドで終わる。サーバーを run の外で動かし続けない（`nohup`・`setsid`・`systemd-run` を使わない）。

各章の「失敗」に当たったら、その run の合否は FAIL。通すために確かめ方を弱めない（待ち時間を延ばす、段を飛ばす、`--allow-dirty` の run を証拠に使う、など）。前提が足りない流れは `features/<名前>.md` の `Gotchas` に理由を書き、「失敗（理由）」として報告する。

## 1. 起動

- 依存の取得: `mise run fetch`（初回だけ。`verify-run` は `--offline --locked` でビルドする）。
- ビルドと起動: `verify-run` が `cargo build --offline --locked -p pelway-http --example demo-serve` を実行し、できた `demo-serve` を `env -u PELWAY_LLM_API_KEY` で子プロセスとして起動する。起動口 `crates/pelway-http/examples/demo-serve.rs` は 127.0.0.1 の空きポートで待ち受け、LLM は台本モード（`ScriptGateway`）に固定され、環境変数を読まない。
- 空のデータから使える状態まで: マイグレーションは無い（保存はメモリだけ）。テスト用データは起動時のデモ seed（`DataStore::new_demo_seeded`: ワークスペース `ws-demo-01`、チャンネル `ch-demo-anken-b`「案件B」、田中 `m-tanaka`・山本 `m-yamamoto`・佐藤 `m-sato`・ハル `m-hal`（bot）、過去の発言 3 件）。
- 使える合図: 標準出力（証拠の `server.stdout`）の 1 行 `demo-serve ready addr=127.0.0.1:<port> pid=<pid> llm=script`。`pid` が起動した子の PID と一致することまで確かめる。ポートが開いただけでは合図にしない。

失敗: ビルドが通らない（`build.log`）。5 秒以内に ready 行が出ない、または ready 行の `pid` が起動した子と違う。

## 2. 環境チェック

`verify-run` の `env_check` が、読み取りだけで「このインスタンスを動かしてよいか」に答え、`env-check.txt` に残す。

- プロセス: `/proc/<pid>/exe` がこの run でビルドした `demo-serve` で、その sha256 が起動前に測った値と同じ。
- ビルドと sha: `git rev-parse HEAD` が run の開始時と同じ。作業ツリーに未コミットの変更があれば起動前に FAIL（`--allow-dirty` は手元の試しだけに使い、その run は証拠にしない）。
- ポート: `ss -tlnp` で、この PID の待ち受けが ready 行の `addr` だけ。
- 認証: デモは認証なし。操作者は合成ヘッダー `x-pelway-member-id`・`x-pelway-member-kind` で名乗る。LLM キーはプロセスの環境に無いこと（`PELWAY_LLM_API_KEY` の名前の有無だけを見て、値は記録しない）。
- 二重起動・共有インスタンス: 起動前に、同じ `demo-serve` バイナリのプロセスが動いていれば FAIL にして止まり、そのプロセスには触らない。接続先は自分の ready 行の `addr` だけで、ほかのポートのインスタンスには要求を送らない。

失敗: 上のどれか 1 つでも合わない（`env-check.txt` の `NG:` 行）。別のインスタンスだった場合も同じ。

## 3. 操作

デモの流れを HTTP の経路で動かす（手がかりは API の経路と合成ヘッダー。経路の定数は `crates/pelway-contract` の `CHANNEL_MESSAGES_PATH`・`DECISIONS_PATH`・`DECIDE_PATH`）。機能ごとの経路・手順・既知の失敗は `features/` にある。`verify-run` が送る要求は次の順:

1. `GET /v1/workspaces/ws-demo-01/channels?actor=m-tanaka` — seed の #案件B にハルがいる
2. `POST /v1/channels/ch-demo-anken-b/messages`（田中）`{"client_msg_id": ..., "body": "@ハル 納期延長の案を2つ作って", "thread_root": null}` — 200 と `message_id`
3. `GET /v1/workspaces/ws-demo-01/decisions?actor=m-yamamoto` を最大 10 秒、1 秒ごと — `request_message_id` が 2 の投稿の判断待ち
4. `GET /v1/channels/ch-demo-anken-b/messages?actor=m-tanaka&after_seq=<2 の seq>` — ハルの投稿が 2 件以上（受付の 1 行と A/B 案）
5. `POST /v1/decisions/<decision_id>/decide`（山本）`{"decision_id", "version", "option_id": "a"}` — 200 と `selected.option_id == "a"`
6. `GET .../decisions?actor=m-yamamoto` — 確定した決定が一覧から消える
7. （`mail-draft` だけ）10 秒待ち、4 の最後の seq より後にハルの投稿があるか

既存テストで同じ道筋を確かめるときは `cargo test -p pelway-http --test runner_mention --test decisions --test channels`（フェイクの LLM。実提供元には接続しない）。テストは run の証拠の代わりにならない。

失敗: seed が無い、要求が 200 以外・接続できない、10 秒で判断待ちが現れない、期待した投稿・状態が API に出ない。

## 4. 証拠

- 置き場: `<evidence-root>/<HEAD の先頭 12 桁>/<UTC 時刻>-<feature>-<run の PID>/`。リポジトリの外で、run ごと・sha ごとに分かれる。
- 中身: `summary.txt`（run_id・sha・dirty・feature・段ごとの `PASS`/`FAIL`・verdict）、`steps/NN-<名前>.req` と `.resp`（送った要求と、受け取った応答そのもの。NN は送った順で、判断待ちを待つ間は `decisions-pending` が複数並ぶ）、`env-check.txt`、`network.txt`（要求のたびに取ったこの PID のソケット）、`proc-fd.txt`（開いているファイル）、`server.stdout`・`server.stderr`、`build.log`、`cleanup.txt`、`SHA256SUMS`。
- 成功の基準:
  - 利用者の道筋（公開の HTTP 経路と合成ヘッダー）だけで動かす。テスト専用の setter・内部の状態を直接読む経路は使わない。
  - 操作（`.req`）と結果の状態（`.resp`）の両方を残す。
  - 副作用を API で読み直す: 投稿が保存されたこと（4 で読み戻す）、決定が確定して判断待ちから消えたこと（6）、下書きが作られたこと（7）。
  - 外に出ていないことを観察で確かめる: `network.txt` の行がすべて 127.0.0.1 同士で、`proc-fd.txt` が標準入出力・ソケットだけ。
- 報告には `summary.txt` の全行と、最後に出る `evidence=<パス>`・`evidence_sha256=<SHA256SUMS の sha256>` を貼る。

失敗: `summary.txt` に `FAIL` 行がある、`network.txt` に 127.0.0.1 以外の宛先がある、`proc-fd.txt` に標準入出力・ソケット以外のファイルがある。

## 5. 後片付け

- `verify-run` の終了時（成功・失敗・中断のどれでも）に、この run が起動した PID だけに SIGTERM を送り、終わるのを待つ。送る前に `/proc/<pid>/exe` がこの run のバイナリであることを確かめる。プロセス名では kill しない。
- 一時状態はサーバーのメモリだけで、プロセスの終了と共に消える。証拠のディレクトリは消さない。
- 後片付けの後に、証拠が残っていることを確かめる: `(cd <evidence> && sha256sum -c SHA256SUMS)` がすべて `OK`。

失敗: `cleanup.txt` が `NG:`（PID が残っている）。`sha256sum -c` に `OK` 以外がある。
