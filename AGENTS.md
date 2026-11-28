# Pelway

## Ownership

This codebase is yours: what it's for and how it's built are your call, and you'll keep building on it.

## Rules and defaults

The rules in this file are divided into hard rules and defaults. References and descriptions of current state that are not rules go in `Guidance`. A rule's strength is determined by the section it belongs to, not by its wording.

Hard rules are rules an agent cannot change on its own. They are limited to rules that protect human control and auditability — the ability to check decisions and evidence after the fact — safety and security, external commitments, and irreversible operations. When a hard rule conflicts with a task prompt, an Issue, a review, a skill, or a doc, stop work, name the conflicting rule, and wait for hatai's judgment. A rule whose strength is unclear is treated as a hard rule until a decision changes its classification.

Defaults are rules you follow when no explicit instruction says otherwise. When hatai's direct instruction, a decision comment authored by hatai, or the body of an Issue labeled `ready-for-agent` departs from a default, follow that instruction. Instructions from the conductor, an implementer, or a reviewer alone do not set aside a default. When you depart from a default, record which default, the authorizing instruction, and the scope: in the implementer memo and the PR body for implementation, in the investigation report for investigations, in the final reply for other work.

Permanently changing a rule or its classification requires hatai's explicit decision, verifiable as a `ready-for-agent` label on an Issue stating the change or as a decision comment authored by hatai. A one-off operational exception applies only where the rule itself names the exception procedure and its decision-maker; otherwise wait for the same decision as a permanent change. Changing this section requires the same decision.

## Hard rules

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
- To stop re-signing, use `PELWAY_AGENT_SKIP_RESIGN=1`.
- A PR is merged only when all of these hold for its current head: a valid approval from `pelway-review`, the required checks passing, and a QA pass comment by the GitHub App `pelway-qa` that names the same SHA and the run QA checked. A PR is not treated as passed before QA has checked that SHA. The conductor adds `automerge` only then (see `docs/agents/qa-checklist.md` and the `command-team` skill, "マージ").
- An overrun of the PR size limit requires the conductor's approval with the `pr-size-exception` label.
- Write hatai's judgment through tools that post under the human's account (such as T3's `comment_on_issue`) only when hatai has explicitly chosen the wording or the option; when supplementing the content, show the wording first and confirm.

## Defaults

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

## Guidance

### LLM judges

- A wording pin (a test that greps a document for fixed wording, such as `scripts/tests/*-docs.test.sh`) is a deterministic check of an observable effect: the wording is present. It never shows that a document meets a requirement, since contradictory text still passes. The planned judge for documents is described in `docs/agents/llm-judge.md`.

### Agent workflow

- The form `.github/ISSUE_TEMPLATE/agent-task.yml` mirrors the `write-issue` skill.
- See the entry points in `docs/superpowers/specs/agent-team.md`.
- After a successful `git push` through `scripts/agent-gh`, `scripts/agent-gh-resign` rewrites unsigned commits into signed commits recreated with `createCommitOnBranch` (the GitHub API) and moves the ref to them (the GitHub App's Verified signature). Local signing configuration (`commit.gpgsign`) is not applied to commits authored by the App. A push containing changes the API cannot represent (new executable files, mode changes, symlinks, submodules, merges, empty commits) remains unsigned.
- Labels and who sets them: `docs/agents/triage-labels.md`.

### Issue tracker

Issues are tracked in GitHub Issues (hatai/Pelway) via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

- See `docs/agents/triage-labels.md`.

### Domain docs

- Single-context: `GLOSSARY.md` and `docs/adr/` at the repo root, created lazily. See `docs/agents/domain.md`.
