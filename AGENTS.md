# Pelway

## Ownership

This codebase is yours: what it's for and how it's built are your call, and you'll keep building on it.

## 規則と既定

このファイルの規則は「変えてはいけない規則」と「既定」に分かれる。規則ではない参照と現状の説明は「案内」に置く。規則の強さは文言ではなく所属する節で決まる。

「変えてはいけない規則」は、エージェントが自分では変えられない規則である。人間の統制と監査可能性（判断と証跡をあとから確かめられること）、安全とセキュリティ、外部との約束、不可逆な操作を守る。task prompt、Issue、レビュー、skill、docs と衝突したときは作業を止め、衝突した規則を示して hatai の判断を待つ。強弱が読めない規則は、分類を変える決定があるまで「変えてはいけない規則」として扱う。

「既定」は、明示された指示がないときに従う規則である。hatai の直接指示、hatai 名義の決定コメント、または `ready-for-agent` の付いた Issue の本文が既定と違うときは、その指示に従う。指揮役・実装担当・レビュー担当の指示だけでは既定を外れない。既定を外れたときは、外した既定・根拠・範囲を書く。実装では実装者メモと PR 本文、調査では調査報告、それ以外の作業では最終回答に書く。

規則とその分類を恒久的に変えるには、hatai の明示した決定が要る。決定は、変更内容を明示した Issue への `ready-for-agent`、または hatai 名義の決定コメントで確認できるようにする。一回の運用例外は、その規則が例外手続きと決定者を明記しているときだけ使う。明記がなければ恒久的な変更と同じ決定を待つ。この節を変えるときも同じ決定が要る。

## 変えてはいけない規則

### Prior art first

- Before implementing a feature you are unfamiliar with, present the approaches to the user as options, each with what to adopt and what to drop and why, and wait for their choice.
- Implement the chosen approach in this codebase's own code: borrow the design, write the code fresh.
- `gh search repos` and `gh search code` are allowed even where an agent's network access is otherwise restricted.
- An agent running autonomously (no user in the session) waits for the choice this way: it posts the comparison as a comment on the issue, adds the `needs-human` label, and stops.

### Code over docs

- When a repository's documentation and its existing code disagree, in your final reply point out each inconsistency you found between the documentation and the code.

### LLM judges

- Where an evaluation turns on the meaning of natural language or on factual claims, judge it with an LLM and an explicit, version-controlled rubric, calibrated on reviewed examples — never by scoring the text with keyword lists or regular expressions.
- Deterministic assertions are limited to observable effects and structured output contracts.
- Judge calls are reported with their cost stated separately, and a judge error counts as "cannot judge", never as a pass.
- Name and describe a wording pin as a presence check; the meaning is judged by the LLM judge or, until one runs for that document, by the reviewer reading the quoted passage.

### Agent workflow

- The AI dev team (the conductor, T3 Code, opencode, Codex, Claude) follows `docs/superpowers/specs/agent-team.md`.
- Issues for the team are written with the `write-issue` skill.
- Bug and incident reports whose cause is not yet understood go through an investigation thread (the `investigate-bug` skill) before filing.
- The report is published as a report-only Draft PR, and the conductor reads the report on that PR.
- The conductor decides whether to file, and the human approves with `ready-for-agent`.
- The report PR is reviewed and merged regardless of the filing decision, so the report stays on main.
- Implementer threads follow the `implement-issue` skill and leave an implementer memo; reviewer threads follow the `review-pr` skill.
- The conductor (Grok Bot, or a substitute agent named by the human) follows `docs/agents/grokbot-runbook.md` for events and the `command-team` skill for routing, escalation, merging, the improvement loop, and substitution.
- Shared rules and skills live only where every tool reads them: `AGENTS.md`, `agent-team.toml`, `.agents/skills/`, `docs/`. After a change, `diff -r .claude/skills/<name> .agents/skills/<name>` must be empty.
- Agents run `gh` and `git` against GitHub through `scripts/agent-gh <dev|review>`, never through plain `gh`, so that PRs are authored by the GitHub App `pelway-dev[bot]` and reviews by the GitHub App `pelway-review[bot]`. The exceptions are the human's fallback `gh pr merge --admin` — merges are done by Kodiak, and the manual merge applies only to the exception PRs hatai judges (the canonical rule is the `command-team` skill's "マージ") — and the conductor's marker comments (`<!-- pr-size-exception -->`, `<!-- split-plan -->`), which reviewers trust only when authored by `hatai` (see the `command-team` skill).
- 署名付け替えを止めるときは `PELWAY_AGENT_SKIP_RESIGN=1` を使う。
- A PR is merged only when all of these hold for its current head: a valid approval from `pelway-review`, the required checks passing, and a QA pass comment by the GitHub App `pelway-qa` that names the same SHA and the run QA checked. A PR is not treated as passed before QA has checked that SHA. The conductor adds `automerge` only then (see `docs/agents/qa-checklist.md` and the `command-team` skill, "マージ").
- An overrun of the PR size limit requires the conductor's approval with the `pr-size-exception` label.
- 人間のアカウントで投稿する道具（T3 の `comment_on_issue` など）で hatai の判断を書くのは、hatai がその文面か選択肢を明示して選んだときだけで、中身を補うときは先に文面を見せて確かめる。

## 既定

### Prior art first

- Before implementing a feature you are unfamiliar with, search GitHub for open-source projects that already solve it (`gh search repos`, `gh search code`) and compare at least two of their approaches.

### Code over docs

- When a repository's documentation and its existing code disagree, follow the conventions used in the existing code.

### Facts over history

Skills and docs state facts only: rules, procedures, definitions, current state. Do not write chronological information — dates, event narratives, what-happened-when — in them. Chronological information lives in `git log` and in documents where the timeline itself is the point, such as worklogs under `docs/worklogs/`.

### LLM judges

- `evals/agent-team` cases follow this: `judge = "llm"` when natural-language evaluation is needed (see `evals/agent-team/README.md`).

### Agent workflow

- The report is saved under `docs/worklogs/bugs/<slug>/report.md`, and the Draft PR uses the branch `agent/investigate-<slug>`.
- Implementer memos go in `docs/worklogs/issue-<N>.md`.
- Each `.claude/skills/<name>` is a symlink to `.agents/skills/<name>`.
- A PR's diff stays within 800 added lines and 2000 deleted lines as counted by `mise run pr-size`.
- Worklogs live under `docs/worklogs/`.

### Triage labels

- Default canonical labels: needs-triage, needs-info, ready-for-agent, ready-for-human, wontfix.

### Domain docs

- Read `GLOSSARY.md` and `docs/adr/` when they exist.

### Skill helpers

- A skill's verifiable steps may ship as an executable helper (local-managed skills only).
- A skill's helper ships with its test, bundled in the same directory as `SKILL.md` (local-managed skills only).
- Scope, test placement and execution, and treatment in both skill trees follow `docs/agents/skill-helpers.md`.

## 案内

### LLM judges

- A wording pin (a test that greps a document for fixed wording, such as `scripts/tests/*-docs.test.sh`) is a deterministic check of an observable effect: the wording is present. It never shows that a document meets a requirement, since contradictory text still passes. The planned judge for documents is described in `docs/agents/llm-judge.md`.

### Agent workflow

- The form `.github/ISSUE_TEMPLATE/agent-task.yml` mirrors the `write-issue` skill.
- See the entry points in `docs/superpowers/specs/agent-team.md`.
- `scripts/agent-gh` の `git push` は、成功後に `scripts/agent-gh-resign` が署名なしコミットを `createCommitOnBranch`（GitHub API）で作り直した署名付きコミットへ ref を付け替える（GitHub App の Verified 署名）。ローカルの署名設定（`commit.gpgsign`）は App 名義のコミットには適用しない。API で表現できない変更（新規の実行ファイル・mode 変更・symlink・submodule・マージ・空コミット）を含む push は署名なしのままになる。
- Labels and who sets them: `docs/agents/triage-labels.md`.

### Issue tracker

Issues are tracked in GitHub Issues (hatai/Pelway) via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

- See `docs/agents/triage-labels.md`.

### Domain docs

- Single-context: `GLOSSARY.md` and `docs/adr/` at the repo root, created lazily. See `docs/agents/domain.md`.
