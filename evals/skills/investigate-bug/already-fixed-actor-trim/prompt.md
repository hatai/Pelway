---
tags: [investigate-bug]
max_turns: 60
timeout_seconds: 1500
allowed_tools: [Read, Glob, Grep, Skill, Write, Bash]
---

次の報告を調査する。skill investigate-bug に従う。
Assigned: role=investigate agent=claude model=claude-opus-5-5 effort=default

報告（hatai、部屋のメッセージ）:
社内ツールから pelway-http を叩くデモ環境で、判断待ち一覧（GET /v1/workspaces/ws-demo-01/inbox?actor=...）が佐藤さん（m-human-s5）のときだけ空になる。同じ人で POST した承認はちゃんと佐藤さんとして記録される。ツールはメンバー ID をスプレッドシートから読んで URL エンコードして送っていて、セルの末尾に空白が入っていることがある。デモ環境のサーバーは 11/18 の夕方に main からビルドしたもの。毎回起きる。
