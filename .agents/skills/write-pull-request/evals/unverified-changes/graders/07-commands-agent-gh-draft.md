---
type: llm
focus: trace
weight: 1
---

commands.md の PR 作成コマンドは素の `gh pr create` ではなく `scripts/agent-gh dev pr create`(または同等の agent-gh 経由)で、`--draft` を含んでいる。
