# Pelway

## Ownership

This codebase is yours: what it's for and how it's built are your call, and you'll keep building on it.

## Prior art first

Before implementing a feature you are unfamiliar with, search GitHub for open-source projects that already solve it (`gh search repos`, `gh search code`) and compare at least two of their approaches. Then present the approaches to the user as options, each with what to adopt and what to drop and why, and wait for their choice. Implement the chosen approach in this codebase's own code: borrow the design, write the code fresh.

## Code over docs

When a repository's documentation and its existing code disagree, follow the conventions used in the existing code, and in your final reply point out each inconsistency you found between the documentation and the code.

## Facts over history

Skills and docs state facts only: rules, procedures, definitions, current state. Do not write chronological information — dates, event narratives, what-happened-when — in them. Chronological information lives in `git log` and in documents where the timeline itself is the point, such as worklogs under `docs/worklogs/`.

## Agent skills

### Issue tracker

Issues are tracked in GitHub Issues (hatai/Pelway) via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Default canonical labels: needs-triage, needs-info, ready-for-agent, ready-for-human, wontfix. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `GLOSSARY.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.
