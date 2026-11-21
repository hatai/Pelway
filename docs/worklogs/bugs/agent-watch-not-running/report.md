# agent-watch が動いていない

## 報告の読み取り

- 期待: pitchfork の supervisor が `boot_start` で agent-watch を常駐させる。agent-watch は T3 のスレッドを 30 秒ごとに見て、状態が変わると Grok Bot の webhook に送り、指揮役の受信箱 `<git-common-dir>/pelway/conductor-inbox.jsonl` に書き、指揮役のスレッドを起こす
- 実際: agent-watch のプロセスが無い。受信箱のファイルも無い
- 対象: hatai の PC（WSL）で動く見張り役 `Pelway/agent-watch`
- 場所: pitchfork の supervisor、`~/.local/bin/pelway-agent-watch`
- 時刻: 確認したのは 2026-11-21 の夕方（JST）。最後のログは 12:52:26（JST）
- 頻度: 再起動のたびに起こる（下の根本原因による）

## 修正済みか

該当なし。#511（boot_start、PR #557）と #555（受信箱と起こし、PR #561）は main に入っている。ただし、この PC の本体 checkout とインストール済みのバイナリには、どちらも届いていない（下の 2・3）。

## 再現

再現は試していない。原因はログと設定ファイルで確定した（証拠は下）。

## 根本原因

再起動のあと、agent-watch を起動するものが無かった。原因は次の 3 つで、どれも PC 側の状態であり、リポジトリのコードの欠陥ではない。

1. **namespace が登録されていない。** 起動時の supervisor が読むのは `~/.config/pitchfork/config.toml` の `[namespaces]` に登録したプロジェクトだけ（`docs/dev-environment.md:692-701`）。このファイルが無い。
2. **本体 checkout が古いブランチにある。** 本体 checkout `/home/hatai/repos/hatai/Pelway` は `agent/510-agent-watch-start-guard`（`e91c3f96`）をチェックアウトしていて、`origin/main` より 126 コミット遅れている。そこの `pitchfork.toml` の `[daemons.agent-watch]` には `boot_start = true` が無い。`docs/dev-environment.md:701` は「本体 checkout は `boot_start = true` の入った main に置いておく」としている。
3. **インストール済みのバイナリが古い。** `~/.local/bin/pelway-agent-watch` は 2026-11-11 20:15 のビルドで、#555 の受信箱と起こし、#510 の起動の拒否を含まない。仮に起動しても、受信箱は作られず、指揮役も起こされない。指揮役の登録ファイル `~/.config/pelway-agent-notify/conductor-thread-id` も無い。

トリガーは 12:52〜12:55 の PC（WSL）の再起動で、それまでは手で起動した agent-watch が動いていた。

### 証拠

pitchfork の supervisor のログ（`~/.local/state/pitchfork/logs/pitchfork/pitchfork.log`）:

```
2026-11-21 12:55:41 INFO  Starting supervisor with pid 549
2026-11-21 12:55:41 INFO  Boot start mode enabled, starting boot_start daemons
2026-11-21 12:55:41 INFO  Scanning for boot_start daemons
2026-11-21 12:55:41 INFO  No daemons configured with boot_start = true
```

agent-watch の最後のログ（`pitchfork logs Pelway/agent-watch`）。この行のあとには何も出ていない:

```
11-21 12:52:26 agent-watch: event=finished thread=8036bf10-e8c4-4bd5-999a-fb9b76caa212 branch=agent/555-agent-watch-inbox mode=send http=200
```

`pitchfork list` は `Pelway/agent-watch stopped`。`state.toml` の `retry_count = 0` なので、異常終了からの再起動を使い切ったのではない。

その他の確認結果:

- `pitchfork boot status` → `Boot start is enabled`（supervisor の login 時の起動は登録済み）
- `ls ~/.config/pitchfork/` → `No such file or directory`
- `stat ~/.local/bin/pelway-agent-watch` → `2026-11-11 20:15:40 +0900`

## 修正案

コードの変更は要らない。PC 側で次を行う（`docs/dev-environment.md:692-701` の手順。リポジトリの外の設定なので hatai が行う）。

1. 本体 checkout を main にして最新にする（`git switch main && git pull`）
2. `~/.config/pitchfork/config.toml` に `[namespaces.pelway]` と本体 checkout の `dir` を書く
3. 本体 checkout で `mise run agent-watch-install` を実行してバイナリを入れ直す
4. 本体 checkout で `pitchfork start agent-watch` を実行する
5. 代行の指揮役が動いているなら、ホストの端末で `~/.local/bin/pelway-agent-watch --register '<スレッド ID>'` で登録する（`docs/dev-environment.md:685` の手順）

再発の防止: 見張りが止まったことに誰も気づけなかった。止まりの検知は #512（heartbeat）で扱う。Grok Bot との連携の Issue でも、heartbeat を webhook に送ることを入れる。

## 未解決の問い

- 再起動の前の 12:52 時点でも、webhook は `http=200` を返していた。#505 は Grok 側の受け口 pelway-2 が 11/12 から止まっているとしている。200 を返したのが pelway-2 か、受けたあと Grok Bot が動いたかは、このリポジトリからは確かめられない。
- ログには `skip cycle: t3 thread: exit 1` が多い（12:00〜12:35 だけで 6 回）。T3 の CLI がときどき失敗している。今回の停止とは別の問題で、調べていない。

## 使ったスキル

investigate-bug、write-pull-request
