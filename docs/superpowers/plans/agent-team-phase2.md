# AI 開発チーム 導入段階 2（見張り役と通知）Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** エージェントの状態の通知を、worktree ごとのフックから、T3 を見張る常駐プロセス 1 つに置き換える。あわせて GitHub からの通知を Grok Bot が動く必要のあるものに絞る。

**Architecture:** Rust のバイナリ `pelway-agent-watch` が 30 秒ごとに `t3 thread list --json` を読み、前回の状態（JSON ファイル）と比べて変化したスレッドだけ `t3 thread show --json` で詳しく見る。判定ロジックは外部コマンドを呼ばない純粋関数に分け、単体テストで固める。外部とのやりとり（t3・gh・fnox・curl）は `main.rs` 側の薄い層に置く。

**Tech Stack:** Rust 1.96（ワークスペースの既存ツールチェーン）、serde / serde_json、pitchfork、GitHub Actions

**Spec:** `docs/superpowers/specs/agent-team.md`（「見張り役と通知」「GitHub からの通知」）

## Global Constraints

- クレートは `crates/pelway-agent-watch`（ワークスペースの `members = ["crates/*"]` に入る）。依存は `serde`（derive）と `serde_json` だけ。時刻の処理も HTTP も自前か外部コマンドで行い、クレートを足さない
- 外部コマンド: `t3`、`gh`、`fnox`、`curl`。秘密（Webhook の URL と鍵）は `fnox -c ~/.config/pelway-agent-notify/fnox.toml get PELWAY_NOTIFY_WEBHOOK_URL` / `PELWAY_NOTIFY_WEBHOOK_KEY` から取り、curl には `-K -`（標準入力）で渡す。引数・ログに出さない
- 秘密が取れないときは送らずにログへ `dry-run` と書く（`notify.sh` と同じ）
- 状態ファイルは `~/.local/state/pelway/agent-watch.json`（`XDG_STATE_HOME` を優先）
- payload の schema は `pelway.agent-watch/v1`。キーは `schema, event, urgency, thread_id, provider, model, branch, issue_number, round, report, stall_reason, link, host, occurred_at` の 14 個で固定
- `report` は最後の assistant 返答が「`キー: 値` の行だけで 5 行以内（キーは英小文字・数字・`_`・`-`）」のときだけその本文、それ以外は `null`
- しきい値: ポーリング 30 秒、`no-activity` 45 分、CI 確認 5 分
- ドキュメントは事実だけを書く（AGENTS.md「Facts over history」）

## Review Focus

1. 見張り役を初めて起動したとき（状態ファイルがない）、既存の 140 本近い完了済みスレッドについて `finished` を送ってはいけない。最初の 1 回は状態を記録するだけにする（Task 2 のテスト `first_run_is_baseline`）
2. 見張り役が止まっている間に完了したスレッドは、再起動後に 1 回だけ `finished` を送る（Task 2 のテスト `missed_completion_after_restart`）
3. 状態ファイルが壊れている・書き込めないときに、全スレッドを再通知したり落ちたりしない。壊れていれば初回扱いにしてログに残す（Task 4）
4. `t3` が一時的に失敗したとき（サーバー停止中など）、全スレッドが消えたと誤解して何かを送らない。そのサイクルは何もしない（Task 4）
5. 報告行に見える文字列でも、秘密やコードの断片が入りうる長い返答は `report` に入れない（Task 1 のテスト）

---

### Task 1: クレートと純粋関数（時刻・ブランチ・報告行）

**Files:**
- Create: `crates/pelway-agent-watch/Cargo.toml`
- Create: `crates/pelway-agent-watch/src/lib.rs`
- Create: `crates/pelway-agent-watch/src/parse.rs`
- Modify: `Cargo.toml`（コメントのクレート一覧を実態に合わせる）

**Interfaces:**
- Produces（`parse.rs`、`lib.rs` から `pub use`）:
  - `fn parse_utc_epoch(s: &str) -> Option<i64>`: `2026-11-10T16:57:09.187Z` / `2026-11-10T16:57:09Z` 形式を UNIX 秒にする。それ以外は `None`
  - `fn issue_number(branch: &str) -> Option<u64>`: `agent/80-agent-gh-help` → `Some(80)`。`agent/` で始まらない・番号がない → `None`
  - `fn extract_report(text: &str) -> Option<String>`: 上の Global Constraints の条件を満たせば前後の空白を除いた本文

- [ ] **Step 1: 失敗するテストを書く**（`parse.rs` の `#[cfg(test)] mod tests`）

| テスト名 | 入力 | 期待 |
|---|---|---|
| `epoch_with_millis` | `"2026-11-10T16:57:09.187Z"` | `Some(1794329829)` |
| `epoch_without_millis` | `"1970-01-01T00:00:00Z"` | `Some(0)` |
| `epoch_rejects_offset` | `"2026-11-10T16:57:09+09:00"` | `None` |
| `issue_from_agent_branch` | `"agent/80-agent-gh-help"` | `Some(80)` |
| `issue_none_for_other_branch` | `"review/81"`、`"agent/x-1"` | `None` |
| `report_four_lines` | `"issue: #80\npr: https://github.com/hatai/Pelway/pull/81\nverify: pass\npr-size: 66 / 800"` | `Some(同じ文字列)` |
| `report_rejects_prose` | `"レビューを確認します。"` | `None` |
| `report_rejects_six_lines` | `a: 1` を 6 行 | `None` |
| `report_rejects_long_value` | 値が 300 文字の 1 行 | `None`（1 行 200 文字まで） |

`epoch_with_millis` の期待値は実装前に `date -u -d 2026-11-10T16:57:09Z +%s` で確かめて書く。

- [ ] **Step 2: 失敗を確かめる**

Run: `cargo test -p pelway-agent-watch --locked`
Expected: コンパイルエラーか FAIL

- [ ] **Step 3: 実装する**

日付は days-from-civil（Howard Hinnant のアルゴリズム）で計算する。報告行の 1 行の上限は 200 文字。

- [ ] **Step 4: 通ることを確かめる**

Run: `cargo test -p pelway-agent-watch --locked`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock crates/pelway-agent-watch
git commit -m "feat: add pelway-agent-watch crate with time, branch and report parsing"
```

---

### Task 2: 状態の比較とイベント生成

**Files:**
- Create: `crates/pelway-agent-watch/src/watch.rs`

**Interfaces:**
- Consumes: Task 1 の 3 関数
- Produces:
  - `struct ListedThread { id: String, status: String, updated_at: String }`（`t3 thread list --json` の 1 要素から読む。`threadId` / `status` / `updatedAt`）
  - `struct ThreadDetail { id, status, updated_at, branch: Option<String>, provider: String, model: String, link: String, run_count: u32, pending_requests: u32, last_assistant_text: Option<String> }`（`t3 thread show --json` の `thread` と `messages` から読む）
  - `struct ThreadMemo { status: String, updated_at: String, run_count: u32, pending_requests: u32, stalled_for: Option<String> }`（`stalled_for` は `no-activity` を送ったときの `updated_at`）
  - `struct State { initialized: bool, threads: BTreeMap<String, ThreadMemo>, ci_notified: BTreeMap<String, String> }`（serde で JSON に読み書き）
  - `fn needs_detail(state: &State, listed: &ListedThread) -> bool`: 初回でなく、（未知のスレッド）または（status か updated_at が変わった）または（status が `running`）なら true
  - `enum Event { Finished, Failed, Cancelled, Waiting, Stalled(StallReason) }`、`enum StallReason { NoActivity, CiFailedTwice }`
  - `fn step(state: &mut State, details: &[ThreadDetail], listed: &[ListedThread], now: i64) -> Vec<(Event, ThreadDetail)>`

規則（`step`）:
- `state.initialized == false` のときは、全スレッドを `threads` に記録して `initialized = true` にし、イベントは返さない
- 前回 `running`（または未知）→ 今回 `completed` / `failed` / `cancelled` で、それぞれ `Finished` / `Failed` / `Cancelled`
- 前回が終わった状態でも、`run_count` が増えて今回また終わった状態なら（見張りの間に 1 往復した）、同じく終了イベントを出す
- `pending_requests` が 0 → 1 以上で `Waiting`
- `running` かつ `now - updated_at >= 45 * 60` かつ `stalled_for != Some(updated_at)` で `Stalled(NoActivity)`、`stalled_for` に `updated_at` を記録
- `listed` に無いスレッドは `threads` から消さない（t3 の一時的な欠落で再通知しないため）

- [ ] **Step 1: 失敗するテストを書く**

| テスト名 | 準備 → 操作 | 期待 |
|---|---|---|
| `first_run_is_baseline` | 未初期化の State、完了済み 3 本 | イベント 0、`initialized == true`、3 本記録 |
| `running_to_completed_emits_finished` | 前回 running → 今回 completed | `Finished` 1 件 |
| `completed_stays_completed_is_silent` | 前回 completed → 今回 completed（同じ updated_at） | 0 件 |
| `missed_completion_after_restart` | 前回 completed run_count 1 → 今回 completed run_count 2 | `Finished` 1 件 |
| `failed_and_cancelled` | running → failed、running → cancelled | `Failed`、`Cancelled` |
| `pending_request_emits_waiting_once` | pending 0 → 1 → 1 | 1 回目だけ `Waiting` |
| `no_activity_stall_once` | running、updated_at が 46 分前、2 サイクル | 1 回目だけ `Stalled(NoActivity)` |
| `no_activity_rearms_after_update` | stall 後に updated_at が変わり、再び 46 分 | もう 1 回 `Stalled(NoActivity)` |
| `missing_from_list_is_kept` | 前回あったスレッドが今回の listed に無い | イベント 0、`threads` に残る |
| `new_thread_finished_between_polls` | 未知のスレッドがいきなり completed | `Finished` 1 件 |

- [ ] **Step 2〜4: 失敗 → 実装 → 成功**（Run: `cargo test -p pelway-agent-watch --locked`）

- [ ] **Step 5: Commit**

```bash
git add crates/pelway-agent-watch
git commit -m "feat: detect agent thread transitions and stalls from T3 snapshots"
```

---

### Task 3: CI の連続失敗と payload

**Files:**
- Create: `crates/pelway-agent-watch/src/ci.rs`
- Create: `crates/pelway-agent-watch/src/payload.rs`

**Interfaces:**
- Consumes: Task 1・2
- Produces:
  - `struct RunInfo { conclusion: String, head_sha: String }`（`gh run list --json conclusion,headSha` の要素）
  - `fn ci_failed_twice(runs_newest_first: &[RunInfo]) -> Option<String>`: 直近 2 件がどちらも `failure` で `head_sha` が異なれば、最新の `head_sha` を返す
  - `fn ci_should_notify(state: &mut State, branch: &str, runs: &[RunInfo]) -> bool`: `ci_failed_twice` が `Some(sha)` で、`state.ci_notified[branch] != sha` なら記録して true
  - `fn build_payload(event: &Event, d: &ThreadDetail, host: &str, occurred_at: &str) -> serde_json::Value`: Global Constraints の 14 キー。`urgency` は常に `"immediate"`、`issue_number` は `branch` から、`round` は `run_count`、`report` は `extract_report(last_assistant_text)`、`stall_reason` は `"no-activity"` / `"ci-failed-twice"` / `null`

- [ ] **Step 1: 失敗するテストを書く**

| テスト名 | 期待 |
|---|---|
| `ci_two_failures_distinct_sha` | `Some(新しい sha)` |
| `ci_same_sha_rerun_is_not_twice` | 同じ sha の 2 回失敗 → `None` |
| `ci_failure_then_success` | `None` |
| `ci_notify_once_per_sha` | 同じ runs で 2 回呼ぶ → true, false |
| `payload_has_fixed_keys` | キー集合がちょうど 14 個で、`schema == "pelway.agent-watch/v1"` |
| `payload_report_null_for_prose` | 最後の返答が文章 → `report == null` |
| `payload_issue_and_round` | branch `agent/80-x`、run_count 2 → `issue_number == 80`、`round == 2` |

- [ ] **Step 2〜4: 失敗 → 実装 → 成功**

- [ ] **Step 5: Commit**

```bash
git add crates/pelway-agent-watch
git commit -m "feat: detect repeated CI failures and build agent-watch payloads"
```

---

### Task 4: 実行層（main）

**Files:**
- Create: `crates/pelway-agent-watch/src/main.rs`
- Create: `crates/pelway-agent-watch/src/io.rs`

**Interfaces:**
- Consumes: Task 1〜3
- Produces: バイナリ `pelway-agent-watch [--once] [--dry-run] [--project <path>]`
  - 既定はループ（30 秒）。`--once` は 1 サイクルで終了、`--dry-run` は送らずに payload を標準出力へ
  - `--project` の既定は実行ファイルの位置から辿れないので、カレントディレクトリ（pitchfork がリポジトリで起動する）

`io.rs` に置くもの（外部コマンドの呼び出しだけ。テストは結合テストで行う）:
- `fn t3_list(project: &Path) -> Result<Vec<ListedThread>, String>`、`fn t3_show(id: &str) -> Result<ThreadDetail, String>`
- `fn gh_runs(branch: &str) -> Result<Vec<RunInfo>, String>`（`gh run list --workflow verify.yml --branch <b> --limit 2 --json conclusion,headSha`）
- `fn open_agent_branches() -> Result<Vec<String>, String>`（`gh pr list --state open --json headRefName` から `agent/` のもの）
- `fn send(payload: &Value) -> SendResult`（fnox で秘密を取り、curl に `-K -` で渡す。HTTP ステータスだけ返す）
- `fn load_state(path) -> State`（壊れていれば `State::default()` を返してログ）、`fn save_state(path, &State)`（一時ファイルに書いて rename）

1 サイクルの流れ: `t3_list` が失敗したら何もせず次へ（Review Focus 4）→ `needs_detail` のものだけ `t3_show` → `step` → 5 分ごとに CI を確認 → 送信 → `save_state`。ログは標準エラーに 1 行ずつ（秘密・本文を出さない。`event` `thread_id` `branch` `http` だけ）。

- [ ] **Step 1: 結合テストを書く** `crates/pelway-agent-watch/tests/once.rs`

偽の `t3` / `gh` / `fnox` を一時ディレクトリに置いて `PATH` の先頭にし、`--once --dry-run` で実行する。

| テスト名 | 偽コマンドの準備 | 期待 |
|---|---|---|
| `once_first_run_prints_nothing` | 状態ファイルなし、完了 2 本 | 標準出力が空、状態ファイルができる |
| `once_emits_finished_with_report` | 状態ファイルで running の 1 本 → 偽 t3 が completed と 4 行の返答 | 標準出力に `"event":"finished"` と `"report":"issue: #80` を含む JSON 1 行 |
| `once_t3_failure_keeps_state` | 偽 t3 が終了コード 1 | 標準出力が空、状態ファイルは変わらない |
| `once_corrupt_state_is_baseline` | 状態ファイルが `{` | 標準出力が空、状態ファイルが正しい JSON に置き換わる |

状態ファイルの場所は環境変数 `XDG_STATE_HOME` で一時ディレクトリに向ける。

- [ ] **Step 2〜4: 失敗 → 実装 → 成功**（Run: `cargo test -p pelway-agent-watch --locked`）

- [ ] **Step 5: 実環境で 1 回だけ dry-run する**

Run: `cargo run -p pelway-agent-watch --locked -- --once --dry-run`（状態ファイルがない状態で 1 回、その後もう 1 回）
Expected: 1 回目も 2 回目も標準出力が空。2 回目のあと状態ファイルに 140 本前後のスレッドが記録されている

- [ ] **Step 6: Commit**

```bash
git add crates/pelway-agent-watch
git commit -m "feat: run agent-watch against t3, gh and the Grok Bot webhook"
```

---

### Task 5: 常駐（pitchfork）と方針の更新

**Files:**
- Create: `pitchfork.toml`
- Modify: `mise.toml`（`pitchfork-check` タスク）
- Modify: `docs/dev-environment.md`（pitchfork の採否の行と手順）

- [ ] **Step 1: pitchfork の設定形式を確かめる**

Run: `pitchfork start --help` と公式ドキュメント（context7 か https://pitchfork.jdx.dev）
デーモンの定義キー（`[daemons.<name>]` の `run` など）、再起動の設定、ログの見方を確かめる。

- [ ] **Step 2: `pitchfork.toml` を書く**

デーモン名 `agent-watch`、`run = "cargo run -p pelway-agent-watch --locked --release"`（mise 経由の cargo が解決できる形にする。Step 1 で確かめた形に従う）。異常終了時は再起動する。

- [ ] **Step 3: `pitchfork-check` を「`pitchfork.toml` があり、`agent-watch` が定義されている」の検査に変える**

Run: `mise run pitchfork-check`
Expected: 成功

- [ ] **Step 4: `docs/dev-environment.md` を直す**

pitchfork の行を「採用。`pitchfork.toml` に見張り役 `agent-watch` を定義」にし、起動・停止・ログの手順（`pitchfork start agent-watch` など Step 1 で確かめたコマンド）を書く。

- [ ] **Step 5: 検査して Commit**

Run: `mise run verify`
Expected: 成功

```bash
git add pitchfork.toml mise.toml docs/dev-environment.md
git commit -m "feat: run agent-watch as a pitchfork daemon"
```

---

### Task 6: GitHub 通知の絞り込み

**Files:**
- Modify: `.github/workflows/notify-grokbot.yml`
- Modify: `docs/dev-environment.md`（GitHub 通知の節）

- [ ] **Step 1: トリガーを絞る**

- `pull_request`: `closed` だけ
- `pull_request_review`: `submitted`（投稿者の判定は今のまま）
- `pull_request_review_comment` は削除
- `workflow_run`（verify の completed）と `workflow_dispatch` は残す

- [ ] **Step 2: payload に `urgency` を足す**

`digest` になるのは、`workflow_run` で `conclusion == success` のときと、`pull_request_review` で `review.state == approved` のとき。それ以外は `immediate`。値は `env:` 経由で渡し、`run:` に `${{ }}` を書かない（既存の方針どおり）。schema は `pelway.github-notify/v2` にし、キーは今の 13 個 + `urgency` の 14 個。

- [ ] **Step 3: 検査**

Run: `mise exec -- jactionlint .github/workflows/notify-grokbot.yml`
Expected: 指摘なし

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/notify-grokbot.yml docs/dev-environment.md
git commit -m "feat: limit Grok Bot GitHub notifications to actionable events"
```

---

### Task 7: payload の説明書と旧来のフックの廃止

**Files:**
- Create: `docs/agents/notify-payloads.md`
- 変更（リポジトリの外。人間の確認を得てから行う）: `~/.codex/hooks.json`、T3 の worktree 作成時スクリプト設定、各 worktree の `.opencode/plugins/pelway-agent-notify.ts`

- [ ] **Step 1: `docs/agents/notify-payloads.md` を書く**

`pelway.agent-watch/v1` と `pelway.github-notify/v2` の全キーの意味と値の例、`event` / `urgency` / `stall_reason` の値の一覧。Grok Bot の runbook（段階 3）がここを参照する。

- [ ] **Step 2: 切り替え前の確認**

pitchfork で `agent-watch` を起動し、小さい T3 スレッドを 1 本動かして、`finished` の通知が Grok Bot に届くこと（ログの `http=2xx`）を確かめる。

- [ ] **Step 3: 旧来のフックを止める（人間の承認を得てから）**

- `~/.codex/hooks.json` の notify.sh の行を外す（元のファイルは `hooks.json.bak` として残す）
- T3 の設定画面で worktree 作成時スクリプトを外す（人間が行う）
- 既存 worktree の `.opencode/plugins/pelway-agent-notify.ts` を `.disabled` に名前を変える

- [ ] **Step 4: Commit**

```bash
git add docs/agents/notify-payloads.md
git commit -m "docs: describe the payloads Grok Bot receives"
```
