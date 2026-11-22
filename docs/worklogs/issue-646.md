# Issue #646 の作業ログ

## 実装者メモ（round 1）

対象 head: b4232add1a689d7ef133128a6269382186f4fc74

1. 詰まった箇所: `docs/agents/human-actions.md` の H33: 「supervisor の PATH（systemd のユーザーユニット）は対象のファイルに書かれておらず」とあり、この変更で事実と合わなくなる。Issue の対象外にも受け入れ条件にも無いので直さず、PR 本文に書いた。
2. 曖昧だった文: 「その手順から調査報告 `docs/worklogs/bugs/agent-watch-not-running/report.md` へのリンクがある」（Issue #646 の受け入れ条件）
3. 仮定と解釈: リンクは `docs/dev-environment.md` の既存のリンク（`](load-testing.md)` など、同じディレクトリからの相対パス）に合わせ、`](worklogs/bugs/agent-watch-not-running/report.md)` の Markdown リンクにした。リポジトリ根からのパスをコードで書くだけの形は選ばなかった。PATH が無いと何が起きるか（exit 127 等）を手順 3 の文中に 1 文で書いた。
4. 足りなかった情報・検証手段: 手順どおりの drop-in が実機で効くか（`systemctl --user show` の出力）は、この worktree からは確かめられない（PC の設定の変更は対象外）。
5. 改善案: `docs/agents/human-actions.md` の H33 の「対象のファイル」の列に `docs/dev-environment.md` の手順 3 を足し、「書かれておらず」の文を消す（フォロー Issue）。

使ったスキル: implement-issue
