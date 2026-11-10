# Triage Labels

The skills speak in terms of five canonical triage roles. This file maps those roles to the actual label strings used in this repo's issue tracker.

| Label in mattpocock/skills | Label in our tracker | Meaning                                  |
| -------------------------- | -------------------- | ---------------------------------------- |
| `needs-triage`             | `needs-triage`       | Maintainer needs to evaluate this issue  |
| `needs-info`               | `needs-info`         | Waiting on reporter for more information |
| `ready-for-agent`          | `ready-for-agent`    | Fully specified, ready for an AFK agent  |
| `ready-for-human`          | `ready-for-human`    | Requires human implementation            |
| `wontfix`                  | `wontfix`            | Will not be actioned                     |

When a skill mentions a role (e.g. "apply the AFK-ready triage label"), use the corresponding label string from this table.

Edit the "Label in our tracker" column to match whatever vocabulary you actually use.

## Operational labels

The AI dev team (see `docs/superpowers/specs/agent-team.md`) tracks a task's state with these labels. Review, fix and merge states are not labels: they come from the PR's draft state, review state and CI result.

| Label | Set by | Removed by | Meaning |
| ----- | ------ | ---------- | ------- |
| `ready-for-agent` | Human, after checking an `agent-task` issue | Grok Bot, when it starts the task | Fully specified, ready for an agent |
| `agent:working` | Grok Bot, when it starts the task | Grok Bot, when the PR merges or `needs-human` is added | An agent is working on it |
| `needs-human` | Grok Bot or an agent, when it stops | Human, after deciding | The agent stopped and needs a human decision |
