# AI 開発チーム 導入段階 1（リポジトリ側）Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** エージェントが Issue から Draft PR・GitHub 上のレビューまでを人間の中継なしで進められるよう、リポジトリ側のルール・道具・保護設定をそろえる。

**Architecture:** 作業ルールは AGENTS.md と skill（`implement-issue`・`review-pr`）に置き、Grok Bot のプロンプトは Issue 番号と skill 名だけにする。GitHub の名義は `scripts/agent-gh` で役割（dev / review）ごとに切り替える。PR の大きさは `scripts/pr-size.sh` で測り、CI の必須チェックにする。人間の承認は CODEOWNERS とブランチ保護で強制する。

**Tech Stack:** bash、mise tasks、GitHub Actions、gh CLI、gh 拡張 `Link-/gh-token`、fnox、GitHub Issue Forms、GitHub Projects

**Spec:** `docs/superpowers/specs/agent-team.md`

## Global Constraints

- PR 1 本の差分上限は 800 行。`.gitattributes` で `linguist-generated` を付けたファイルは数えない
- 実装担当の名義は `pelway-agent`、レビュー担当は GitHub App `pelway-review`
- ラベルは `ready-for-agent`・`agent:working`・`needs-human` の 3 つを運用に使う
- ブランチ名は `agent/<issue番号>-<slug>`
- 秘密値は fnox で管理し、リポジトリにもログにも出さない
- skill と docs には事実だけを書き、日付や経緯を書かない（AGENTS.md「Facts over history」）
- 新しい skill は `.agents/skills/<name>/SKILL.md` に置き、`.claude/skills/<name>` からシンボリックリンクを張る（`write-pull-request` と同じ形）
- 本文は日本語。既存の docs と同じ文体

## Review Focus

1. 実装スレッドが `scripts/agent-gh dev` を使わずに素の `gh` で PR を作ると、PR の作成者が `hatai` になる。`hatai` は自分の PR を Approve できないので、マージできなくなる。`implement-issue` skill の最初の手順で `scripts/agent-gh dev api user --jq .login` が `pelway-agent` を返すことを確かめさせる（Task 6）
2. ファイル名に空白を含むファイルや、リネームしたファイルがあっても、`pr-size.sh` が正しく数える（Task 4 のテスト）
3. fnox に値がないとき、`agent-gh` は秘密値を出さずにエラーで止まる。誤って `hatai` の名義で処理を続けない（Task 5 のテスト）
4. mise-bump が毎週作る PR（作成者 `github-actions[bot]`）も、ブランチ保護のもとで `hatai` の Approve だけでマージできる（Task 7 の確認手順）
5. GitHub の画面から Issue Form を開いたとき、必須項目を空のままでは Issue を作れない（Task 3 の確認手順）

---

### Task 0: 既存の worktree とブランチの片付け

**Files:** なし（リポジトリの外の操作）

- [ ] **Step 1: 削除候補の一覧を出す**

`git worktree list --porcelain` と `gh pr list --state all --limit 300 --json number,headRefName,state` を突き合わせ、次の 2 種類を TSV（worktree パス、ブランチ、理由）で一覧にする。
- PR が MERGED か CLOSED のブランチ
- PR がなく、最後のコミットが 7 日以上前のブランチ

ただし #59 と #62 の head ブランチ、`client-ui`・`client-gdpr-13` を含むブランチ、`main` は除く。リモートブランチも同じ条件で別の一覧にする。

- [ ] **Step 2: 一覧を人間に見せ、削除の承認を得る**

承認が得られるまで Step 3 に進まない。

- [ ] **Step 3: 承認された分だけ削除する**

`git worktree remove <path>` → `git branch -D <branch>` の順に実行する。リモートは `git push origin --delete <branch>`。未コミットの変更がある worktree は削除せずに報告する。

- [ ] **Step 4: 結果を確かめる**

Run: `git worktree list | wc -l` と `git branch | wc -l`
Expected: 削除した数だけ減っている

---

### Task 1: AGENTS.md と docs の食い違いを直す

**Files:**
- Modify: `AGENTS.md`
- Create: `CLAUDE.md`
- Move: `docs/worklog/backend.md` → `docs/worklogs/backend.md`
- Modify: `docs/dev-environment.md`（CI 節の「2026-11-10 に GitHub API で公式タグと一致することを確認」の日付と経緯）

- [ ] **Step 1: `CLAUDE.md` を作る**

中身は `@AGENTS.md` の 1 行だけ。

- [ ] **Step 2: AGENTS.md を直す**

- 「Prior art first」: `gh search repos` と `gh search code` は、エージェントのネットワーク制限の例外として許可されると明記する。自律実行中のエージェントは、選択肢を比較した結果を Issue にコメントし、`needs-human` を付けて止まる。それが「人間の選択を待つ」の手順になる
- 「Domain docs」: `GLOSSARY.md` と `docs/adr/` は存在する場合だけ読む、と `docs/agents/domain.md` の記述にそろえる
- 新しい節「Agent workflow」を足す。中身は次の 4 点を指す短い説明にする
  - 仕様 `docs/superpowers/specs/agent-team.md`
  - skill `implement-issue` と `review-pr`
  - `scripts/agent-gh`
  - ラベルの意味（`docs/agents/triage-labels.md`）

- [ ] **Step 3: worklog を `docs/worklogs/` に一本化する**

Run: `git mv docs/worklog/backend.md docs/worklogs/backend.md`
その後 `grep -rn "docs/worklog/" --include=*.md . | grep -v "^./docs/worklogs/"` で残った参照を直す。

- [ ] **Step 4: dev-environment.md から日付と経緯を消す**

該当の文は「lockfile 上のコミットは公式タグと一致する」という事実だけを残す。

- [ ] **Step 5: 検査する**

Run: `mise run hk-check`
Expected: PASS
Run: `test -d docs/worklog && echo NG || echo OK`
Expected: `OK`

- [ ] **Step 6: Commit**

```bash
git add AGENTS.md CLAUDE.md docs/worklogs/backend.md docs/worklog docs/dev-environment.md
git commit -m "docs: align AGENTS.md with actual docs layout and agent workflow"
```

---

### Task 2: ラベル

**Files:**
- Modify: `docs/agents/triage-labels.md`

- [ ] **Step 1: ラベルを作る**

```bash
gh label create "agent:working" --color "fbca04" --description "エージェントが作業中（Grok Bot が付ける）"
gh label create "needs-human" --color "b60205" --description "エージェントが止まり、人間の判断が必要"
```

- [ ] **Step 2: triage-labels.md に 2 つのラベルを足す**

表の下に「運用ラベル」の表を足し、誰が付けて誰が外すかを書く。
- `ready-for-agent`: 人間が付け、Grok Bot が着手時に外す
- `agent:working`: Grok Bot が着手時に付け、PR のマージか `needs-human` で外す
- `needs-human`: Grok Bot かエージェントが付け、人間が外す

- [ ] **Step 3: 確かめる**

Run: `gh label list --search agent: && gh label list --search needs-human`
Expected: 2 つとも表示される

- [ ] **Step 4: Commit**

```bash
git add docs/agents/triage-labels.md
git commit -m "docs: define operational labels for the agent team"
```

---

### Task 3: Issue Form `agent-task`

**Files:**
- Create: `.github/ISSUE_TEMPLATE/agent-task.yml`
- Create: `.github/ISSUE_TEMPLATE/config.yml`（`blank_issues_enabled: true`。計画用の自由記述 Issue を残すため）

- [ ] **Step 1: フォームを書く**

- `name: エージェント向けタスク`
- `labels: []`。`ready-for-agent` は人間が中身を確かめてから手で付ける
- 項目（`id` は固定。Grok Bot と skill がこの見出しで読む）

| id | 種類 | 見出し | 必須 |
|---|---|---|---|
| `goal` | textarea | 目的 | ○ |
| `acceptance` | textarea | 受け入れ条件（テストで確かめられる形で、1 行 1 条件） | ○ |
| `out-of-scope` | textarea | 対象外 | ○ |
| `size` | dropdown（`〜200 行` / `〜800 行` / `800 行超（分割が必要）`） | 規模の見積もり | ○ |
| `approach` | textarea | 採用する方式（Prior art で人間が選んだもの。なければ空） | — |

- [ ] **Step 2: 検査する**

Run: `mise run hk-check`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add .github/ISSUE_TEMPLATE
git commit -m "feat: add agent-task issue form"
```

- [ ] **Step 4: マージ後に画面で確かめる**

GitHub の New issue からフォームを開き、必須項目を空のまま送信できないことを確かめる。Review Focus 5 の確認。

---

### Task 4: PR サイズ検査 `scripts/pr-size.sh`

**Files:**
- Create: `scripts/pr-size.sh`
- Create: `scripts/tests/pr-size.test.sh`
- Create: `.gitattributes`
- Modify: `mise.toml`（タスク `pr-size` と `script-test` を足し、`script-test` を `verify` の depends に入れる）
- Modify: `.github/workflows/verify.yml`（ジョブ `pr-size` を足す）

**Interfaces:**
- Produces: `scripts/pr-size.sh [--base <ref>] [--limit <n>]`
  - 既定値は `--base origin/main`、`--limit 800`
  - 標準出力の 1 行目に `pr-size: <数えた行数> / <上限> lines` を出す
  - 上限以下なら終了コード 0、超えたら 1、引数の誤りは 2
  - 上限を超えたときは、数えたファイルを行数の多い順に最大 10 件、標準エラーに出す

- [ ] **Step 1: 失敗するテストを書く**

`scripts/tests/pr-size.test.sh` は `mktemp -d` で一時 git リポジトリを作り、ケースごとに base コミットと HEAD コミットを用意して `pr-size.sh --base <base> --limit <n>` を実行する。各ケースの期待値は次のとおり。

| テスト名 | 準備 | 期待 |
|---|---|---|
| `under_limit` | 10 行追加、limit 800 | 終了 0、1 行目 `pr-size: 10 / 800 lines` |
| `over_limit` | 801 行追加、limit 800 | 終了 1 |
| `generated_excluded` | `tokens.json` に 900 行追加、`.gitattributes` に `tokens.json linguist-generated=true`、`a.rs` に 5 行追加 | 終了 0、`pr-size: 6 / 800 lines`（`.gitattributes` の 1 行を含む） |
| `binary_ignored` | バイナリファイルを追加 | 終了 0、`pr-size: 0 / 800 lines` |
| `space_in_name` | `my file.rs` に 3 行追加 | `pr-size: 3 / 800 lines` |
| `rename_counts_changes_only` | 100 行のファイルをリネームし、1 行だけ変更 | `pr-size: 2 / 800 lines` |
| `bad_args` | `--limit abc` | 終了 2 |

テストは失敗したケース名を出して非 0 で終わり、全部成功したら `ok` を出す。

- [ ] **Step 2: テストが失敗することを確かめる**

Run: `bash scripts/tests/pr-size.test.sh`
Expected: FAIL（`scripts/pr-size.sh` がない）

- [ ] **Step 3: `scripts/pr-size.sh` を実装する**

`git diff --numstat -z -M <base>...HEAD` を読む。バイナリ（`-`）は飛ばす。パスごとに `git check-attr linguist-generated -- <path>` が `set` か `true` なら数えない。リネームは新しいパスで判定する。`set -euo pipefail` を使う。

- [ ] **Step 4: テストが通ることを確かめる**

Run: `bash scripts/tests/pr-size.test.sh`
Expected: `ok`

- [ ] **Step 5: `.gitattributes` と mise タスク、CI を足す**

- `.gitattributes`: `Cargo.lock`、`mise.lock`、`skills-lock.json`、`.github/workflows/actions.lock`、`.agents/skills/**`、`.claude/skills/**` に `linguist-generated=true` を付ける
- `mise.toml`
  - `[tasks.pr-size]` は `run = "bash scripts/pr-size.sh"`
  - `[tasks.script-test]` は `run = "bash scripts/tests/pr-size.test.sh && bash scripts/tests/agent-gh.test.sh"`。`agent-gh.test.sh` は Task 5 で作るので、それまでは `pr-size.test.sh` だけにする
  - `script-test` を `verify` の depends に足す
- `verify.yml`
  - ジョブ `pr-size`（`name: pr-size`）を足す。`pull_request` のときだけ動かす
  - `actions/checkout` は `fetch-depth: 0` にする
  - `bash scripts/pr-size.sh --base origin/${{ github.base_ref }}` を実行する
  - mise のセットアップは不要。git と bash だけで動く
  - Action の参照は既存ジョブと同じ sha 固定に従う

- [ ] **Step 6: 検査する**

Run: `mise run verify`
Expected: PASS
Run: `mise exec -- jactionlint .github/workflows/verify.yml`
Expected: 指摘なし

- [ ] **Step 7: Commit**

```bash
git add scripts .gitattributes mise.toml .github/workflows/verify.yml
git commit -m "feat: add PR size check with an 800-line limit"
```

---

### Task 5: 名義の切り替え `scripts/agent-gh`

**Files:**
- Create: `scripts/agent-gh`（実行権限付き）
- Create: `scripts/tests/agent-gh.test.sh`
- Modify: `fnox.toml`
- Modify: `mise.toml`（`script-test` に `agent-gh.test.sh` を足す）

**Interfaces:**
- Consumes: fnox のシークレット `PELWAY_AGENT_GH_TOKEN`、`PELWAY_REVIEW_APP_ID`、`PELWAY_REVIEW_APP_INSTALLATION_ID`、`PELWAY_REVIEW_APP_KEY`（PEM）。gh 拡張 `Link-/gh-token`（`gh token generate`）
- Produces:
  - `scripts/agent-gh <dev|review> <gh の引数...>`: 役割の名義で `gh` を実行する
  - `scripts/agent-gh <dev|review> git <git の引数...>`: その名義の認証で `git` を実行する（`-c credential.helper=` で既存の設定を消してから `-c 'credential.helper=!gh auth git-credential'` を足す）。`dev` のときは `-c user.name=pelway-agent -c user.email=<fnox の PELWAY_AGENT_GIT_EMAIL>` も付ける
  - 終了コード: 役割の誤りは 2、トークンが取れないときは 3、それ以外は `gh` か `git` の終了コードをそのまま返す

- [ ] **Step 1: gh-token のフラグを確かめる**

Run: `gh extension install Link-/gh-token && gh token generate --help`
`--app-id`、`--installation-id`、鍵の渡し方（`--key` か `--base64-key`）、トークンだけを出すフラグ（`--token-only`）が実在するか確かめる。違っていたら、以降の手順を実在するフラグに読み替える。

- [ ] **Step 2: 失敗するテストを書く**

`scripts/tests/agent-gh.test.sh` は、一時ディレクトリに偽の `fnox`・`gh`・`git` を置いて `PATH` の先頭にする。偽の `gh` と `git` は、受け取った `GH_TOKEN`（伏せずに）と引数をファイルに書き出す。偽の `gh token generate` は `review-token` を返す。

| テスト名 | 準備 | 期待 |
|---|---|---|
| `dev_sets_token` | fnox が `dev-token` を返す。`agent-gh dev pr list` | 偽の gh が `GH_TOKEN=dev-token` と `pr list` を受け取る |
| `review_mints_token` | `agent-gh review pr review 1 --approve` | 偽の gh が `GH_TOKEN=review-token` を受け取る |
| `dev_git_identity` | `agent-gh dev git commit -m x` | 偽の git が `user.name=pelway-agent` と `credential.helper=!gh auth git-credential` を受け取る |
| `bad_role` | `agent-gh admin pr list` | 終了 2 |
| `missing_token_fails_closed` | fnox が空文字を返して終了 1 | 終了 3。偽の gh は呼ばれない |
| `token_not_leaked` | `dev_sets_token` と同じ | `agent-gh` 自身の標準出力・標準エラーに `dev-token` が含まれない |

- [ ] **Step 3: テストが失敗することを確かめる**

Run: `bash scripts/tests/agent-gh.test.sh`
Expected: FAIL

- [ ] **Step 4: `scripts/agent-gh` を実装する**

トークンは変数に入れて `GH_TOKEN=... exec gh ...` で渡す。`set -x` は使わない。`review` の鍵は一時ファイル（`mktemp`、`chmod 600`、`trap` で消す）か標準入力で渡す。

- [ ] **Step 5: テストが通ることを確かめる**

Run: `bash scripts/tests/agent-gh.test.sh`
Expected: `ok`

- [ ] **Step 6: fnox.toml にシークレットの名前を宣言する**

5 つすべて `if_missing = "ignore"` にし、description に用途を書く（`PELWAY_AGENT_GIT_EMAIL` は秘密ではないが、置き場所をそろえるためここに置く）。値は書かない。

- [ ] **Step 7: 検査する**

Run: `mise run verify`
Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add scripts/agent-gh scripts/tests/agent-gh.test.sh fnox.toml mise.toml
git commit -m "feat: add agent-gh wrapper to run gh/git as pelway-agent or pelway-review"
```

---

### Task 6: skill `implement-issue` と `review-pr`

**Files:**
- Create: `.agents/skills/implement-issue/SKILL.md`
- Create: `.agents/skills/review-pr/SKILL.md`
- Create: `.claude/skills/implement-issue` → `../../.agents/skills/implement-issue`（シンボリックリンク）
- Create: `.claude/skills/review-pr` → `../../.agents/skills/review-pr`（シンボリックリンク）
- Modify: `.agents/skills/write-pull-request/SKILL.md`

**Interfaces:**
- Consumes: `scripts/agent-gh`（Task 5）、`scripts/pr-size.sh`（Task 4）、Issue Form の見出し（Task 3）

`superpowers:writing-skills` に従って書く。どちらの skill も frontmatter の `description` は「Use when ...」で始める。

- [ ] **Step 1: `implement-issue` を書く**

skill に入れる決まりは次のとおり。
1. 最初に `scripts/agent-gh dev api user --jq .login` が `pelway-agent` を返すことを確かめる。違えば止まり、Issue に `needs-human` を付ける
2. `scripts/agent-gh dev issue view <N> --json title,body,labels,comments` で Issue を読む。受け入れ条件がテストで確かめられない形なら、その理由を Issue にコメントし、`needs-human` を付けて止まる
3. 慣れない機能で、Issue の「採用する方式」が空なら、AGENTS.md の Prior art first に従う。`gh search` で 2 つ以上の方式を比べた結果を Issue にコメントし、`needs-human` を付けて止まる
4. ブランチは `agent/<N>-<slug>`。受け入れ条件ごとにテストを先に書く（TDD）
5. コミットの後に `mise run verify` と `mise run pr-size` を実行する（pr-size は `base...HEAD` を数えるため、コミット後でないと差分に入らない）。pr-size が 800 行を超えたら、分割の案を Issue にコメントし、`needs-human` を付けて止まる
6. コミットと push は `scripts/agent-gh dev git ...` で行う
7. PR は `write-pull-request` skill に従い、`scripts/agent-gh dev pr create --draft` で作る。本文に `Closes #<N>` を入れる
8. 修正の指示（`t3 thread send`）を受けたら、同じブランチに積む。新しいブランチは作らない
9. 迷っても質問で止まらない。判断と理由は PR 本文の「対象外」か Issue のコメントに書く。止まってよいのは 1・2・3・5 の場合だけ

- [ ] **Step 2: `review-pr` を書く**

skill に入れる決まりは次のとおり。
1. 入力は PR 番号。`gh pr diff <N>` と、リンクされた Issue の受け入れ条件を読む
2. ファイルは書き換えない。実行してよいのは `mise run verify` と `mise run pr-size` だけ
3. 指摘は「must-fix」と「nit」に分ける。must-fix が 1 つでもあれば Request changes にする
4. 投稿は `scripts/agent-gh review pr review <N> --approve|--request-changes --body-file -` で行う。行単位の指摘は `scripts/agent-gh review api repos/{owner}/{repo}/pulls/<N>/reviews` の `comments` で付ける
5. 本文の先頭は `Verdict: approve` か `Verdict: request-changes`。その後に must-fix と nit を箇条書きにする
6. PR が Draft のまま Approve したら、`scripts/agent-gh review pr ready <N>` で Ready for review にする

- [ ] **Step 3: `write-pull-request` を直す**

- 「定期ルール」の使ってよい `gh` の一覧に `gh search repos`・`gh search code`・`gh issue view`・`gh issue comment`・`gh issue edit` を足す
- PR の作成と更新は `scripts/agent-gh dev` 経由で行う、と書く
- エージェントが作る PR は Draft にする、と書く

- [ ] **Step 4: シンボリックリンクを張り、検査する**

Run: `ls -l .claude/skills/implement-issue .claude/skills/review-pr && mise run hk-check`
Expected: 両方とも `.agents/skills/...` を指していて、hk-check が PASS

- [ ] **Step 5: Commit**

```bash
git add .agents/skills/implement-issue .agents/skills/review-pr .agents/skills/write-pull-request .claude/skills/implement-issue .claude/skills/review-pr
git commit -m "feat: add implement-issue and review-pr skills for the agent team"
```

---

### Task 7: アカウント・ブランチ保護・Projects ボード

**Files:**
- Create: `.github/CODEOWNERS`（`* @hatai` の 1 行）
- Create: `scripts/setup/branch-protection.sh`
- Create: `scripts/setup/agent-accounts.sh`（`wizard` skill で生成する、人間が実行する手順書）

**Interfaces:**
- Consumes: fnox のシークレット名（Task 5）、CI のジョブ名 `mise run verify` と `pr-size`（Task 4）

- [ ] **Step 1: `scripts/setup/branch-protection.sh` を書く**

`gh api --method PUT repos/hatai/Pelway/branches/main/protection` に次の JSON を送る。何度実行しても同じ結果になるようにする。

```json
{
  "required_status_checks": { "strict": false, "contexts": ["mise run verify", "pr-size"] },
  "enforce_admins": false,
  "required_pull_request_reviews": {
    "required_approving_review_count": 1,
    "require_code_owner_reviews": true,
    "dismiss_stale_reviews": true
  },
  "restrictions": null
}
```

`enforce_admins: false` にするのは、`hatai` が緊急時に保護を迂回できるようにするため。

- [ ] **Step 2: `wizard` skill で `scripts/setup/agent-accounts.sh` を作る**

人間にしかできない手順を、この順で案内する。
1. マシンユーザー `pelway-agent` を作り、`hatai/Pelway` に Write 権限の collaborator として招待し、招待を受ける
2. `pelway-agent` で PAT を作る。まず fine-grained PAT（Contents・Pull requests・Issues が Read and write）を試す。collaborator のリポジトリが対象に選べなければ、classic PAT の `repo` スコープにする。このアカウントが触れるのは Pelway だけなので、影響はそこに限られる
3. `fnox set PELWAY_AGENT_GH_TOKEN` と `fnox set PELWAY_AGENT_GIT_EMAIL`（`<id>+pelway-agent@users.noreply.github.com`）
4. GitHub App `pelway-review` を作る（権限は Pull requests: Read and write、Contents: Read、Metadata: Read。Webhook は無効）。秘密鍵を発行し、`hatai/Pelway` にだけインストールする
5. `fnox set` で App の ID、Installation ID、秘密鍵を登録する
6. `scripts/setup/branch-protection.sh` を実行する
7. GitHub Projects のボード「Pelway agent team」を作り、列を Todo / Working / In review / Changes requested / Ready to merge / Done にする。組み込みワークフローを次のとおり有効にする
   - Auto-add: `is:issue label:ready-for-agent`
   - Item closed → Done
   - Pull request merged → Done
   - Code changes requested → Changes requested
   - Code review approved → Ready to merge
8. 確かめ: `scripts/agent-gh dev api user --jq .login` が `pelway-agent` を返し、`scripts/agent-gh review api /installation/repositories --jq '.repositories[].full_name'` が `hatai/Pelway` を返す

- [ ] **Step 3: Commit（ブランチ保護を有効にする前に main へ入れる）**

```bash
git add .github/CODEOWNERS scripts/setup
git commit -m "chore: add CODEOWNERS, branch protection script and agent account wizard"
```

- [ ] **Step 4: 人間が `scripts/setup/agent-accounts.sh` を実行する**

- [ ] **Step 5: 保護の設定を確かめる**

Run: `gh api repos/hatai/Pelway/branches/main/protection --jq '{checks: .required_status_checks.contexts, codeowners: .required_pull_request_reviews.require_code_owner_reviews}'`
Expected: `{"checks":["mise run verify","pr-size"],"codeowners":true}`

次に開かれる mise-bump の PR が、`hatai` の Approve だけでマージできることを確かめる。Review Focus 4 の確認。

---

### Task 8: 通しの試運転

**Files:** なし

- [ ] **Step 1: 小さい Issue を Issue Form で作り、`ready-for-agent` を付ける**

例: `docs/agents/triage-labels.md` の表に、ラベルの色を書く列を足す。

- [ ] **Step 2: 短いプロンプトで実装スレッドを立てる**

Run: `t3 thread start --provider opencode --model zai-coding-plan/glm-5.3 --worktree main --branch agent/<N>-label-colors . "Issue #<N> を実装する。skill implement-issue に従う。"`

- [ ] **Step 3: 結果を確かめる**

- PR の作成者が `pelway-agent` で、Draft になっている
- 本文に `Closes #<N>` がある
- CI の `pr-size` と `mise run verify` が成功している

- [ ] **Step 4: レビュースレッドを立てる**

Run: `t3 thread start --provider codex --model gpt-6-luna . "PR #<M> をレビューする。skill review-pr に従う。"`
Expected: `pelway-review[bot]` 名義のレビューが付き、Projects のボードでカードが In review か Changes requested に移る

- [ ] **Step 5: 人間が Approve してマージする**

Expected: Issue が閉じ、ボードのカードが Done に移る
