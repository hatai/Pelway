# レビュー担当を専用 home に移したときの確認（2026-11-21）

`docs/dev-environment.md` の「専用 home での確認」と「レビュー担当の Claude」の確認項目を実施した記録。このファイルの確認は、どれも同じ版で行った（`codex-cli 0.171.0`、Claude Code `2.1.330`、T3 `0.1.10-nightly.20261115.4736`。この日の作業の間に版は変わっていない）。

## codex_review

`codex_review` に委託した確認用の子タスク（`runtimeMode: "auto-accept-edits"`）で実施した。`homePath` を変えた後、T3 を再起動してから行った。

| 項目 | 結果 |
|---|---|
| 渡るツールに `t3`・`codex_apps`・`github` を含む名前が無いか（`ALL_TOOLS` の名前） | 合格（該当なし） |
| `CODEX_HOME`・`GH_CONFIG_DIR`・`PELWAY_AGENT_FNOX_CONFIG` | `CODEX_HOME=/home/hatai/.codex-review`、`GH_CONFIG_DIR=/home/hatai/.config/pelway-gh-empty`、`PELWAY_AGENT_FNOX_CONFIG` あり（後の 2 つは `scripts/agent-gh review` が実際に使う値ではない） |
| `scripts/agent-gh review api /rate_limit` | 合格（exit 0） |
| GitHub 以外への通信（`agent-gh review api --hostname example.com /`、`curl https://example.com`） | 合格（どちらも sandbox の許可先で遮断） |
| 秘密の置き場 4 パスの `test -r` | 合格（4 つとも 1） |
| worktree への書き込み（`touch`） | 書き込めた。権限プロファイル `review` の「`/tmp` だけ書ける」と食い違う。home を分ける前から同じ（Issue #589） |
| `mise run codex-review-check` | 合格（GitHub 2 件が 200、`example.com` は通常・`--noproxy` とも 000） |
| `~/.codex-review` への書き込み（確認用の子タスク、`touch ~/.codex-review/config.toml` と新しいファイル） | 合格（どちらも `Read-only file system` で拒否。同じ子タスクで worktree と `/tmp` には書けた） |
| `codex sandbox -P review` での書き込み（`CODEX_HOME=~/.codex-review`、cwd は worktree） | `~/.codex-review/config.toml` と `~/.codex-review` の新しいファイル、worktree はどれも `Read-only file system` で拒否。`/tmp` は書けた |

`web_search` を確かめる項目に足した後、`main` のコミットから委託した確認用の子タスクで、モデルに渡るツールを全部挙げさせた。直接呼べるツール（`functions.exec`・`functions.wait`・`collaboration.*` など）と `exec` から呼べるツール（`apply_patch`・`exec_command`・`list_mcp_resources`・`image_gen__imagegen` など）に、MCP サーバーのツールとウェブ検索（`web_search`・`web.run`）は無く、ウェブ検索は使えなかった。ウェブ検索の実効の設定は、`~/.codex-review/config.toml` の `web_search = "disabled"` で、`launchArgs` に上書きする `-c` は無い。

## claude_review

`~/.claude-review/settings.json` の `permissions.deny` に `mcp__t3-code` と `mcp__t3-code-direct` を足した後、`claude_review` の子タスクで確かめた。

| 項目 | 結果 |
|---|---|
| ツールの一覧に `t3` を含むツールが無いか | 合格（該当なし） |
| `scripts/agent-gh review api /rate_limit` | 合格（exit 0） |

`env` に `ENABLE_CLAUDEAI_MCP_SERVERS=false` を足した後、`main` のコミットから委託した `claude_review` の子タスクで確かめた。

| 項目 | 結果 |
|---|---|
| `mcp__` で始まるツール（遅延読み込みの一覧を含む） | 0 個。claude.ai のコネクタのツールは出なかった |
| 設定に残る MCP サーバー | アカウントから同期されたプラグインの MCP サーバー（GitHub・1Password ほか）。どれも接続に失敗したか、認証が要る状態で、ツールは 0 個 |
| `scripts/agent-gh review api /rate_limit --jq .rate.limit` | 合格（`5000`） |

`permissions.deny` に `SendMessage`・`RemoteTrigger`・`Artifact`・`ShareOnboardingGuide`・`PushNotification` を足した後、`main` のコミットから委託した `claude_review` の子タスクで確かめた（ツールは呼ばず、一覧だけを見た）。

| 項目 | 結果 |
|---|---|
| `SendMessage`・`RemoteTrigger`・`Artifact`・`ShareOnboardingGuide`・`PushNotification`・`WebFetch`・`WebSearch` | どれも一覧に無い |
| `mcp__` で始まるツール | 0 個。GitHub のプラグインの MCP サーバーは、ツールを呼ぶと再接続を試みる状態で、今は認証の形式エラーで失敗している |
| 一覧に残る組み込みのツール | `Agent`・`Bash`・`Read`・`Skill`・`ToolSearch`・`Workflow`・`ScheduleWakeup` ほか、遅延読み込みの `PublishPlugin`・`DesignSync`・`Cron*` など。許可リストに無いものは承認要求になり、フックが拒否する（呼んでは確かめていない） |
| `scripts/agent-gh review api /rate_limit --jq .rate.limit` | 合格（`5000`） |

`[projects.` の表は、`~/.codex-review/config.toml` に無い（ホストで `grep -c '^\[projects\.'` が 0）。

## 4 回目のレビューの後の確認

- `codex_review`：`main` のコミットから委託した確認用の子タスクで、秘密の置き場 5 パス（`~/.config/fnox`・`~/.config/pelway-agent`・`~/.config/pelway-agent-qa`・`~/.ssh`・`~/.config/gh`）の `test -r` がすべて 1 だった（中身は読んでいない）。同じ 5 パスは、ホストで `codex sandbox -P review` を通して試すとすべて 0（読める）だった。`codex sandbox` は `/etc/codex/requirements.toml` の `deny_read` を使わないので、この項目は T3 のスレッドの中で確かめる。
- `claude_review`：`env` に `CLAUDE_CODE_SKIP_PLUGIN_MCP_SERVERS=1` を足した後、`main` のコミットから委託した子タスクで、`mcp__` で始まるツールは 0 個、システムの通知に出る MCP サーバーも 0 個（`plugin:` で始まるものも無い）だった。`scripts/agent-gh review api /rate_limit --jq .rate.limit` は `5000`。

## 5 回目のレビューの後の確認

確かめたときの版: `codex-cli 0.171.0`、Claude Code `2.1.330`、T3 `0.1.10-nightly.20261115.4736`（`~/.t3/runtime/service-state.json` の `activeVersion`）。

`~/.claude-review/settings.json` の `permissions.deny` に `DesignSync`・`PublishPlugin` を足した後、`main` のコミットから委託した `claude_review` の子タスクで、一覧だけを見て確かめた。

| 項目 | 結果 |
|---|---|
| すぐ使える組み込みのツール | `Agent`・`AskUserQuestion`・`Bash`・`ListAgents`・`Read`・`ReportFindings`・`ScheduleWakeup`・`Skill`・`ToolSearch`・`Workflow` |
| 遅延読み込みの組み込みのツール | `CronCreate`・`CronDelete`・`CronList`・`EnterPlanMode`・`EnterWorktree`・`ExitPlanMode`・`ExitWorktree`・`Monitor`・`TaskStop` |
| `DesignSync`・`PublishPlugin`・`SendMessage`・`RemoteTrigger`・`Artifact`・`ShareOnboardingGuide`・`PushNotification`・`WebFetch`・`WebSearch` | どれも一覧に無い |
| `mcp__` で始まるツールと、MCP サーバーの通知 | どちらも無い |
| `scripts/agent-gh review api /rate_limit --jq .rate.limit` | 合格（`5000`） |
