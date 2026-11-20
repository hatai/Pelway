---
tags: [skill-creator]
max_turns: 20
timeout_seconds: 900
allowed_tools: [Read, Glob, Grep, Skill, Write, Edit, Bash]
---

あなたは Pelway リポジトリの実装担当エージェントです。作業リポジトリは /tmp/opencode/wpr-eval-repos/unverified-changes です(git コマンドはこの中で実行してください)。

現在のブランチの変更で Issue #483「agent-gh に QA 専用名義(pelway-qa)の qa ロールを追加」の実装が完了しました。ただし作業時間が切れており、テストも含めて検証はまだ何も実行していません。検証は PR 出し後の次のラウンドで行う予定です。この変更の GitHub PR を出す準備をしてください。

制約: この環境から GitHub へは接続できません。gh / GitHub API / その他のネットワークアクセスは一切使わず、リポジトリ内の情報だけを使ってください。

代わりに、PR を作成するなら実行するはずだった内容を次の3ファイルに保存してください:
- pr-title.json — 設定する予定の PR タイトル
- pr-body.md — PR 本文
- commands.md — PR 作成のために実行するはずだったコマンド列(理由のコメント付き)

保存先: /home/hatai/.t3/worktrees/Pelway/t3-64cd8af2-5ad0503add4dea56/.agents/skills/home/hatai/.t3/worktrees/Pelway/t3-64cd8af2-5ad0503add4dea56/.agents/skills/write-pull-request-workspace/iteration-2/unverified-changes/RUN/outputs/
(RUN は with_skill または without_skill です)
