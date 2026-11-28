# Issue #795 の作業ログ

## 失敗の筋（実装前）

- 失敗の筋: PR テンプレートに欄だけが増え、`write-pull-request` が書き方を説明しないため、今後の PR 本文で空欄や形式だけの記入が残る
  確かめ方: `.github/pull_request_template.md` と `.agents/skills/write-pull-request/SKILL.md` の該当文を読み合わせ、`scripts/tests/pr-failure-risk-docs.test.sh` で必須文言の存在を確かめる
- 失敗の筋: `implement-issue` に実装前の記録手順が入らず、失敗の筋が PR 作成時の後付けになる
  確かめ方: `.agents/skills/implement-issue/SKILL.md` の手順 4 と手順 7 の該当文を読み、`scripts/tests/pr-failure-risk-docs.test.sh` で必須文言の存在を確かめる
- 失敗の筋: レビュー規則の境界が曖昧で、網羅性の好みまで must-fix になり、修正 round が終わらない
  確かめ方: `.agents/skills/review-pr/SKILL.md` に must-fix の対象と nit の境界が書かれていることを読み、`scripts/tests/pr-failure-risk-docs.test.sh` で必須文言の存在を確かめる
- 失敗の筋: リスク欄の案内が不十分で、攻撃手順や未修正の弱点の詳細が公開の PR 本文に書かれる
  確かめ方: `.github/pull_request_template.md` と `.agents/skills/write-pull-request/SKILL.md` に詳細を書かない旨があることを読み、`scripts/tests/pr-failure-risk-docs.test.sh` で必須文言の存在を確かめる
- 失敗の筋: `write-pull-request` の評価ケースが 6 セクションのまま残り、スキルと評価の期待値が食い違う
  確かめ方: `.agents/skills/write-pull-request/evals/` を検索し、6 セクションの期待値が残っていないことを確かめる
- 失敗の筋: PR #840 と同じ箇所を変えるため、マージ順によって `write-pull-request` のセクション一覧や sha 行の規則が片方だけの姿に戻る
  確かめ方: この PR のレビュー時点で main と PR #840 の状態を読み、冲突があれば最終差分で両方の規則が残っていることを読み合わせる

## 失敗の筋（最終）

- 失敗の筋: PR テンプレートに欄だけが増え、`write-pull-request` が書き方を説明しないため、今後の PR 本文で空欄や形式だけの記入が残る
  確かめ方: `.github/pull_request_template.md` に 2 つの欄と必須形式、`.agents/skills/write-pull-request/SKILL.md` に書き方と例があることを読み、`bash scripts/tests/pr-failure-risk-docs.test.sh` で必須文言の存在を確かめる
- 失敗の筋: `implement-issue` に実装前の記録手順が入らず、失敗の筋が PR 作成時の後付けになる
  確かめ方: `.agents/skills/implement-issue/SKILL.md` の手順 4 と手順 7 に実装前・round ごとの見直し・最終版の記録があることを読み、`bash scripts/tests/pr-failure-risk-docs.test.sh` で必須文言の存在を確かめる
- 失敗の筋: レビュー規則の境界が曖昧で、網羅性の好みまで must-fix になり、修正 round が終わらない
  確かめ方: `.agents/skills/review-pr/SKILL.md` に構造的な欠落と diff から示せる矛盾だけを must-fix にし、網羅性の好みを nit にする境界があることを読み、`bash scripts/tests/pr-failure-risk-docs.test.sh` で必須文言の存在を確かめる
- 失敗の筋: リスク欄の案内が不十分で、攻撃手順や未修正の弱点の詳細が公開の PR 本文に書かれる
  確かめ方: `.github/pull_request_template.md` と `.agents/skills/write-pull-request/SKILL.md` に攻撃手順や未修正の弱点の詳細を書かない旨があることを読み、`bash scripts/tests/pr-failure-risk-docs.test.sh` で必須文言の存在を確かめる
- 失敗の筋: `write-pull-request` の評価ケースが 6 セクションのまま残り、スキルと評価の期待値が食い違う
  確かめ方: `rg -n '6セクション|テンプレート6' .agents/skills/write-pull-request/evals/` が一致を返さないことを確かめる
- 失敗の筋: PR #840 と同じ箇所を変えるため、マージ順によって `write-pull-request` のセクション一覧や sha 行の規則が片方だけの姿に戻る
  確かめ方: PR 作成時に PR #840 が open の Draft で main に未マージであることを `scripts/agent-gh dev pr view 840 --json state,isDraft,mergedAt` で確かめ、main の同箇所にこの変更と冲突する差分がないことを `git diff HEAD...origin/main -- .agents/skills/write-pull-request/SKILL.md` で確かめる

実装前の筋のうち外したものはない。実装中に新たな筋は見つからなかった。

## 失敗の筋の見直し（round 2）

- 失敗の筋: PR #840 と main の確認に使う 2 点 diff が自分の変更を含み、main 側の変更を確かめたことにならない
  確かめ方: `git diff HEAD...origin/main -- .agents/skills/write-pull-request/SKILL.md` が merge-base から main 側への差分だけを示すことを読み、実際の出力を確かめる

## 実装者メモ（round 1）

対象 head: 39e44d8cf76dae9844dd437a7cc0cd671dd62ca1

1. 詰まった箇所: 手順 6: この環境では `/tmp` と `/home` の所有者が root に見えず、既存の verify-run・conductor 系テストが変更なしの経路でも失敗し、clippy は再実行で通った
2. 曖昧だった文: 「失敗が自分の変更と無関係に見えるとき（実行ごとに失敗箇所が変わる・ツールチェーンの再インストールを伴うなど）は、輸入スキル `diagnosing-bugs`（下の「輸入スキル」の節）の Phase 2（再現と最小化）で切り分ける」（.agents/skills/implement-issue/SKILL.md「検査してコミットし、行数を確かめてから push する」）
3. 仮定と解釈: 文書だけの変更であり、失敗した verify-run・conductor 系テストは今回の diff の対象外と仮定した。実行環境由来の失敗として記録して進める解釈を選び、無関係な既存テストをこの Issue で直す解釈は選ばなかった
4. 足りなかった情報・検証手段: root 所有に見える通常の `/tmp` と `/home` を持つ実行環境。`mise run verify` は exit=101、script-test の verify-run 失敗と一度だけの clippy 一時ディレクトリ作成失敗、clippy 再実行は exit=0
5. 改善案: .agents/skills/implement-issue/SKILL.mdの「検査してコミットし、行数を確かめてから push する」に「所有権が root に見えない sandbox では verify-run・conductor 系テストが環境起因で失敗しうる」を足す

使ったスキル: implement-issue, write-pull-request, review-pr, improve-skills

## 失敗の筋（round 2 の最終）

- 失敗の筋: PR #840 と main の確認に使う 2 点 diff が自分の変更を含み、main 側の変更を確かめたことにならない
  確かめ方: `git diff HEAD...origin/main -- .agents/skills/write-pull-request/SKILL.md` を実行し、main 側の差分が無い出力（0 行）を確かめる

round 1 の最終版にあるほかの筋は、この round の差分でも残す。消した筋はない。

## 実装者メモ（round 2）

対象 head: b0d5e1a44ca681b18afed7420e12e6bd27cab01b

1. 詰まった箇所: docs/worklogs/issue-795.md: main 側変更の確認に 2 点 diff を書いており、自分の変更を差分に含んでいた
2. 曖昧だった文: 「main の同箇所にこの変更と冲突する差分がないことを `git diff HEAD..origin/main -- .agents/skills/write-pull-request/SKILL.md` で確かめる」（docs/worklogs/issue-795.md「失敗の筋（最終）」）
3. 仮定と解釈: main 側だけの差分を見る意図と仮定した。3 点 diff で merge-base から main 側を見る解釈を選び、自分の変更を含む 2 点 diff を残す解釈は選ばなかった
4. 足りなかった情報・検証手段: 指摘を受けた時点でコマンドの意味を実際の出力と突き合わせる確認
5. 改善案: docs/worklogs/issue-795.mdの「失敗の筋（最終）」に「main 側だけを見るときは 3 点 diff を使う」を足す

使ったスキル: implement-issue, receiving-code-review
