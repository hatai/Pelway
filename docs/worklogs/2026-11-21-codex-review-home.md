# レビュー担当を専用 home に移したときの確認（2026-11-21）

`docs/dev-environment.md` の「専用 home での確認」と「レビュー担当の Claude」の確認項目を実施した記録。

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
