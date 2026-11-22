# Pelway

## Ownership

This codebase is yours: what it's for and how it's built are your call, and you'll keep building on it.

## Prior art first

Before implementing a feature you are unfamiliar with, search GitHub for open-source projects that already solve it (`gh search repos`, `gh search code`) and compare at least two of their approaches. Then present the approaches to the user as options, each with what to adopt and what to drop and why, and wait for their choice. Implement the chosen approach in this codebase's own code: borrow the design, write the code fresh.

`gh search repos` and `gh search code` are allowed even where an agent's network access is otherwise restricted. An agent running autonomously (no user in the session) waits for the choice this way: it posts the comparison as a comment on the issue, adds the `needs-human` label, and stops.

## Code over docs

When a repository's documentation and its existing code disagree, follow the conventions used in the existing code, and in your final reply point out each inconsistency you found between the documentation and the code.

## Facts over history

Skills and docs state facts only: rules, procedures, definitions, current state. Do not write chronological information — dates, event narratives, what-happened-when — in them. Chronological information lives in `git log` and in documents where the timeline itself is the point, such as worklogs under `docs/worklogs/`.

## LLM judges

Where an evaluation turns on the meaning of natural language or on factual claims, judge it with an LLM and an explicit, version-controlled rubric, calibrated on reviewed examples — never by scoring the text with keyword lists or regular expressions. Deterministic assertions are limited to observable effects and structured output contracts. Judge calls are reported with their cost stated separately, and a judge error counts as "cannot judge", never as a pass.

- `evals/agent-team` cases follow this: `judge = "llm"` when natural-language evaluation is needed (see `evals/agent-team/README.md`).
- A wording pin (a test that greps a document for fixed wording, such as `scripts/tests/*-docs.test.sh`) is a deterministic check of an observable effect: the wording is present. It never shows that a document meets a requirement, since contradictory text still passes. Name and describe a wording pin as a presence check; the meaning is judged by the LLM judge or, until one runs for that document, by the reviewer reading the quoted passage. The planned judge for documents is described in `docs/agents/llm-judge.md`.

## Agent workflow

The AI dev team (the conductor, T3 Code, opencode, Codex, Claude) follows `docs/superpowers/specs/agent-team.md`.

- Issues for the team are written with the `write-issue` skill (the form `.github/ISSUE_TEMPLATE/agent-task.yml` mirrors it).
- Bug and incident reports whose cause is not yet understood go through an investigation thread (the `investigate-bug` skill) before filing: the report is saved under `docs/worklogs/bugs/<slug>/report.md` and published as a report-only Draft PR on the branch `agent/investigate-<slug>` (the conductor reads the report on that PR), the conductor decides whether to file, and the human approves with `ready-for-agent`. The report PR is reviewed and merged regardless of the filing decision, so the report stays on main. See the entry points in `docs/superpowers/specs/agent-team.md`.
- Implementer threads follow the `implement-issue` skill and leave an implementer memo in `docs/worklogs/issue-<N>.md`; reviewer threads follow the `review-pr` skill.
- The conductor (Grok Bot, or a substitute agent named by the human) follows `docs/agents/grokbot-runbook.md` for events and the `command-team` skill for routing, escalation, merging, the improvement loop, and substitution.
- Shared rules and skills live only where every tool reads them: `AGENTS.md`, `.agents/skills/`, `docs/`. Each `.claude/skills/<name>` is a symlink to `.agents/skills/<name>`; after a change, `diff -r .claude/skills/<name> .agents/skills/<name>` must be empty.
- Agents run `gh` and `git` against GitHub through `scripts/agent-gh <dev|review>`, never through plain `gh`, so that PRs are authored by the GitHub App `pelway-dev[bot]` and reviews by the GitHub App `pelway-review[bot]`. The exceptions are the human's fallback `gh pr merge --admin` — merges are done by Kodiak, and a manual merge is used only when the queue cannot merge, needing admin rights that the Apps do not have — and the conductor's marker comments (`<!-- pr-size-exception -->`, `<!-- split-plan -->`), which reviewers trust only when authored by `hatai` (see the `command-team` skill).
- `scripts/agent-gh` の `git push` は、成功後に `scripts/agent-gh-resign` が署名なしコミットを `createCommitOnBranch`（GitHub API）で作り直した署名付きコミットへ ref を付け替える（GitHub App の Verified 署名）。ローカルの署名設定（`commit.gpgsign`）は App 名義のコミットには適用しない。API で表現できない変更（新規の実行ファイル・mode 変更・symlink・submodule・マージ・空コミット）を含む push は署名なしのままになる。止めるときは `PELWAY_AGENT_SKIP_RESIGN=1`。
- A PR is merged only when all of these hold for its current head: a valid approval from `pelway-review`, the required checks passing, and a QA pass comment by the GitHub App `pelway-qa` that names the same SHA and the run QA checked. A PR is not treated as passed before QA has checked that SHA. The conductor adds `automerge` only then (see `docs/agents/qa-checklist.md` and the `command-team` skill, "マージ").
- A PR's diff stays within 800 added lines and 2000 deleted lines as counted by `mise run pr-size`, unless the conductor approved the overrun with the `pr-size-exception` label.
- 人間のアカウントで投稿する道具（T3 の `comment_on_issue` など）で hatai の判断を書くのは、hatai がその文面か選択肢を明示して選んだときだけで、中身を補うときは先に文面を見せて確かめる。
- Labels and who sets them: `docs/agents/triage-labels.md`.
- Worklogs live under `docs/worklogs/`.

## Agent skills

### Issue tracker

Issues are tracked in GitHub Issues (hatai/Pelway) via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Default canonical labels: needs-triage, needs-info, ready-for-agent, ready-for-human, wontfix. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `GLOSSARY.md` and `docs/adr/` at the repo root, read when they exist (they are created lazily). See `docs/agents/domain.md`.

### Skill helpers

A skill's verifiable steps may ship as an executable helper with its test, bundled in the same directory as `SKILL.md` (local-managed skills only). Scope, test placement and execution, and treatment in both skill trees follow `docs/agents/skill-helpers.md`.
