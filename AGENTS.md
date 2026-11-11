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

## Agent workflow

The AI dev team (the conductor, T3 Code, opencode, Codex, Claude) follows `docs/superpowers/specs/agent-team.md`.

- Implementer threads follow the `implement-issue` skill and leave an implementer memo in `docs/worklogs/issue-<N>.md`; reviewer threads follow the `review-pr` skill.
- The conductor (Grok Bot, or a substitute agent named by the human) follows `docs/agents/grokbot-runbook.md` for events and the `command-team` skill for routing, escalation, merging, the improvement loop, and substitution.
- Shared rules and skills live only where every tool reads them: `AGENTS.md`, `.agents/skills/`, `docs/`. Each `.claude/skills/<name>` is a symlink to `.agents/skills/<name>`; after a change, `diff -r .claude/skills/<name> .agents/skills/<name>` must be empty.
- Agents run `gh` and `git` against GitHub through `scripts/agent-gh <dev|review>`, never through plain `gh`, so that PRs are authored by the GitHub App `pelway-dev[bot]` and reviews by the GitHub App `pelway-review[bot]`.
- A PR's diff stays within 800 lines as counted by `mise run pr-size`.
- Labels and who sets them: `docs/agents/triage-labels.md`.
- Worklogs live under `docs/worklogs/`.

## Agent skills

### Issue tracker

Issues are tracked in GitHub Issues (hatai/Pelway) via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Default canonical labels: needs-triage, needs-info, ready-for-agent, ready-for-human, wontfix. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `GLOSSARY.md` and `docs/adr/` at the repo root, read when they exist (they are created lazily). See `docs/agents/domain.md`.
