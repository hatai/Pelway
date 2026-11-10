# Grok Bot が受け取る通知

Grok Bot の webhook には 2 種類の JSON が届く。`schema` で見分ける。運用の仕様は `docs/superpowers/specs/agent-team.md`。

| schema | 送り手 | 内容 |
|---|---|---|
| `pelway.agent-watch/v1` | 見張り役 `agent-watch`（`crates/pelway-agent-watch`、pitchfork で常駐） | T3 のエージェントのスレッドの状態変化 |
| `pelway.github-notify/v2` | `.github/workflows/notify-grokbot.yml` | GitHub の PR・レビュー・CI の出来事 |

どちらも `urgency` を持つ。`immediate` はすぐ対応し、`digest` は朝と夕方のまとめにだけ反映する。

## `pelway.agent-watch/v1`

キーは 14 個で固定。値が無いものは `null`。

| キー | 型 | 意味 |
|---|---|---|
| `schema` | string | `"pelway.agent-watch/v1"` |
| `event` | string | `finished` / `failed` / `cancelled` / `waiting` / `stalled` |
| `urgency` | string | 常に `"immediate"` |
| `thread_id` | string | T3 のスレッド ID（`t3 thread send` の宛先）。`stall_reason` が `ci-failed-twice` のときは空文字 |
| `provider` | string | `opencode`（実装担当）/ `codex`（レビュー担当）など |
| `model` | string | 例: `zai-coding-plan/glm-5.3` |
| `branch` | string / null | スレッドの worktree のブランチ |
| `issue_number` | number / null | `branch` が `agent/<番号>-...` のときの番号 |
| `round` | number | スレッドの実行回数（最初の指示が 1、`t3 thread send` のたびに 1 増える） |
| `report` | string / null | スレッドの最後の返答が `キー: 値` の行だけで 5 行以内のとき、その本文 |
| `stall_reason` | string / null | `event` が `stalled` のとき `no-activity`（実行中のまま 45 分更新なし）か `ci-failed-twice`（`agent/` ブランチで verify が 2 回続けて失敗） |
| `link` | string | T3 の画面でスレッドを開く URL |
| `host` | string | 見張り役が動いているマシン名 |
| `occurred_at` | string | 見張り役が検知した時刻（UTC、`2026-11-10T16:57:09Z` 形式） |

### `event` ごとの意味

| `event` | 起きたこと |
|---|---|
| `finished` | スレッドの実行が完了した。結果は `report` を読む |
| `failed` | スレッドの実行が失敗した |
| `cancelled` | スレッドの実行が取り消された |
| `waiting` | スレッドが承認か入力を待っている |
| `stalled` | `stall_reason` を参照 |

### `report` の形

skill が決めている最後の返答の形。

実装担当（skill `implement-issue`）:

```
issue: #80
pr: https://github.com/hatai/Pelway/pull/81
verify: pass
pr-size: 66 / 800
```

止まったときは `pr:` の代わりに `stopped: <止まった理由>`。

レビュー担当（skill `review-pr`）:

```
pr: #81
verdict: approve
must-fix: 0
```

`verdict` は `approve` / `request-changes` / `failed`。`failed` のときは 4 行目に `error: ...`。

### 例

```json
{"schema":"pelway.agent-watch/v1","event":"finished","urgency":"immediate","thread_id":"mcp:fe379cab-84fc-4bdd-841e-aeea82129d66","provider":"opencode","model":"zai-coding-plan/glm-5.3","branch":"agent/80-agent-gh-help","issue_number":80,"round":1,"report":"issue: #80\npr: https://github.com/hatai/Pelway/pull/81\nverify: pass\npr-size: 66 / 800","stall_reason":null,"link":"https://app.t3.codes/threads/...","host":"mypc","occurred_at":"2026-11-10T17:40:12Z"}
```

## `pelway.github-notify/v2`

キーは 14 個で固定。値が無いものは `null`。

| キー | 意味 |
|---|---|
| `schema` | `"pelway.github-notify/v2"` |
| `urgency` | `digest`: verify の成功、Approve のレビュー。それ以外は `immediate` |
| `repo` | `hatai/Pelway` |
| `event` | `pull_request` / `pull_request_review` / `workflow_run` / `test`（手動実行） |
| `action` | `closed`（`pull_request`）、`submitted`（`pull_request_review`）、`completed`（`workflow_run`） |
| `pr_number` | PR 番号 |
| `head_branch` | PR のブランチ |
| `head_sha` | PR の head のコミット |
| `merged` | `pull_request` の `closed` でマージされたら `true` |
| `conclusion` | `workflow_run` の結果（`success` / `failure` など） |
| `workflow_name` | `workflow_run` の workflow 名（`verify`） |
| `run_url` | Actions の実行の URL |
| `actor` | イベントを起こした GitHub ユーザー |
| `occurred_at` | 送信時刻（UTC） |

送られないもの: PR の作成・push（エージェントの状態は `agent-watch` が送る）、レビューコメント、OWNER / MEMBER / COLLABORATOR 以外（ボットを含む）のレビュー、fork からのイベント。
