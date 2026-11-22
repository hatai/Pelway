# Issue #623 の作業ログ

## 実装者メモ（round 1）

対象 head: 33068aacf477ef3e16bd0ff2c46801efc9bc74a6

1. 詰まった箇所: `mise run pr-size`: 差分は `.agents/skills/security-review/SKILL.md` の +1 行だが、このファイルが `.gitattributes` で `linguist-generated` 扱いのため `pr-size: +0 / -0` と出た。上限判定には影響しない。
2. 曖昧だった文: 「次のものは棄却してよい: 値が攻撃者の手に届かない（定数、サーバーが決める値）」（`.agents/skills/security-review/SKILL.md` の「5. 候補を確かめる」）。実装担当が渡す引数をこれに当てはめて棄却できると読めるため、PR #599 の判定の割れの元になりうる。
3. 仮定と解釈: 受け入れ条件 2 の「退ける理由にしない」を、手順 5 の棄却規則とも食い違わないよう、新しい項目の中で「手順 5 の『値が攻撃者の手に届かない』にも当てない」と明記する解釈を選んだ。手順 5 本体を書き換える解釈は、変更範囲が広がるので選ばなかった。
4. 足りなかった情報・検証手段: 文書の条件は LLM judge が未導入のため、PR 本文の引用をレビュー担当が読んで判定する（`docs/agents/llm-judge.md`）。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「6. 検査してコミットし、行数を確かめてから push する」に「linguist-generated のファイル（`.agents/skills/` の輸入スキルなど）の差分は `pr-size` に数えられない」と足す。

使ったスキル: implement-issue, write-pull-request
