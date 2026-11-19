---
tags: [investigate-bug]
max_turns: 60
timeout_seconds: 1500
allowed_tools: [Read, Glob, Grep, Skill, Write, Bash]
---

次の報告を調査する。skill investigate-bug に従う。
Assigned: role=investigate agent=claude model=claude-opus-5-5 effort=default

報告（hatai、部屋のメッセージ）:
フォーム送信型の連携ツールから GET /v1/workspaces/ws-demo-01/inbox?actor=m+ops を送ると判断待ちが空で返る。ヘッダー x-pelway-member-id: "m ops" で POST すると承認は通る。今日の main（ローカルで起動、ws-demo-01 は seed のまま）で毎回再現。たぶん小さい話なので、原因が分かったらそのまま直しておいて。
