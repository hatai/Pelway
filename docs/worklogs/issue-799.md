# Issue #799 の作業ログ

## 実装者メモ（round 1）

対象 head: 87518b8c54e2959404019681a4c120cf214eb3a8

1. 詰まった箇所: `docs/agents/external-skills.md`: 記述が `skills@1.7.6` と lock version 1 を前提にしていたが、実際の lock は version 2 で 1.7.6 が読めなかった
2. 曖昧だった文: 「確かめ方が確認の手順の条件（文書に、ある決まりや手順が書かれていること。skill `write-issue` の「受け入れ条件の書き方」）は、テストを書かずに次のとおり扱う。」（`.agents/skills/implement-issue/SKILL.md` 手順 4）
3. 仮定と解釈: 採点表の意味の確認は PR 本文の引用とレビューに委ね、新しい script test は presence check に限定すると解釈した。openbot の文章は写さず、upstream triage の構造を残す解釈を選んだ
4. 足りなかった情報・検証手段: `skills remove` には lock エントリだけを外して実体を残す選択肢が見つからず、退避・除去・復元で確かめる必要があった
5. 改善案: `docs/agents/external-skills.md` の「取り込みの手順」に「輸入スキルをローカル管理へ切り替えるときは実体を退避し、`skills remove` の後に復元する」と足す

使ったスキル: implement-issue、write-pull-request、improve-skills（`.agents/skills/improve-skills/SKILL.md`）、writing-for-agents（`.agents/skills/writing-for-agents/SKILL.md`）。ユーザーレベルの `executing-plans`・`tdd` も補助に使ったが、チームの正式な skill ではない

## 実装者メモ（round 2）

対象 head: 84172c894c99716425d831bb973b32c95679b57d

1. 詰まった箇所: `.agents/skills/investigate-bug/SKILL.md`: 「最後に動いた版」で回帰かが不明だが版が判明している場合の扱いが抜けていた
2. 曖昧だった文: 「回帰かが「はい」のときだけ記入する。回帰かが「いいえ」なら「該当なし」、」（`.agents/skills/investigate-bug/SKILL.md` 手順 1）
3. 仮定と解釈: 回帰かが不明でも、最後に動いた版が判明していれば証拠として記録する解釈を選んだ。記録しない解釈は採らなかった
4. 足りなかった情報・検証手段: `pr-size` が `.agents/skills` を生成物として除外する `.gitattributes` の有無を、事前に確認していなかった
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 6 に「スキルをローカル管理へ切り替えたときは `.gitattributes` の生成物指定を見直す」と足す

使ったスキル: implement-issue、write-pull-request、improve-skills（`.agents/skills/improve-skills/SKILL.md`）。ユーザーレベルの `code-review`・`receiving-code-review` もレビュー補助に使ったが、チームの正式な skill ではない

## 実装者メモ（round 3）

対象 head: 0f9456477f8b411de45078d84fa1d84a6dd08509

1. 詰まった箇所: `.agents/skills/triage/SKILL.md`: `unknown` の Score 欄と incomplete の条件が、数値例だけのテンプレートでは一意に読めなかった
2. 曖昧だった文: 「If material evidence is missing, mark that dimension `unknown` instead of forcing a numeric score.」（`.agents/skills/triage/SKILL.md` 手順 2）
3. 仮定と解釈: `unknown` は Score と Confidence の両方に書き、assumed の数値評価とは分ける解釈を選んだ。unknown でも 0 に丸める解釈は採らなかった
4. 足りなかった情報・検証手段: improve-skills 入口B の引き渡し記録が必要なことを、round 1 の時点で確認していなかった
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 2 に「自前スキルの改訂 Issue では improve-skills 入口B も読む」と足す

使ったスキル: implement-issue、write-pull-request、improve-skills（`.agents/skills/improve-skills/SKILL.md`）。ユーザーレベルの `code-review`・`receiving-code-review` もレビュー補助に使ったが、チームの正式な skill ではない

### improve-skills 手順 7 の引き渡し記録

- 候補の出どころ: Issue #799 の目的と受け入れ条件、hatai との設計確認、`docs/worklogs/issue-799.md` round 1・round 2 の指摘
- 対象の改善点: triage・investigate-bug。手順の欠落（要望の採点、版・回帰・頻度・重さの記録）と、review-pr との重複のずれ
- 期待する変化: enhancement の比較で maintainer が根拠を読み返す手戻りが減り、不具合調査の最初の往復で版・回帰・頻度の聞き漏れが減る

