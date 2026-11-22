# Issue #657 の作業ログ

## 実装者メモ（round 1）

対象 head: 496cf5b09b586491fba0417368e95f0c393541f1

1. 詰まった箇所: `scripts/agent-gh`: Issue の取得経路は `scripts/agent-gh <role> api ...` だが、`qa` ロールは `issue comment` しか受け付けない（`qa_check_args`）。QA の委託先が contents API を読む名義を `dev` と決めて本文に書いた。
2. 曖昧だった文: 「`.agents/skills/command-team/SKILL.md` の委託の節に、再委託・遅延実行タスクの定型文として」（Issue #657 の受け入れ条件）。command-team に「委託の節」という見出しは無く、関連情報の「委託の表のある節」から「レビュー担当と effort」と読んだ。
3. 仮定と解釈: 定型文は delegate_task の task の 2 行（skill 指示と `Assigned:`）を変えず、3 行目の `Guard:` の行として足すと解釈した。review-pr は `Assigned:` を 2 行目として読むため、既存の 2 行を書き換える解釈は選ばなかった。成果の「未投稿」は、指定 SHA に向いた `pelway-review` のレビューか `pelway-qa` の判定が無いことと置いた。
4. 足りなかった情報・検証手段: 遅延実行タスク（`schedule_task`）を誰がどの文面で組むかの正本が無く、定型文を入れる場所を「task の 2 行の後」とだけ書いた。
5. 改善案: `.agents/skills/command-team/SKILL.md` の「レビュー担当と effort」に「`schedule_task` で再委託を組むときは、task の全文を上のブロックから写す」を足す。

使ったスキル: implement-issue、write-pull-request
