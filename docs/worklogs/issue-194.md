# Issue #194 の作業ログ

## 実装者メモ（round 1）

対象 head: 9a7228becdb712059ca6e74cff19858f19cf7514

1. 詰まった箇所: mise run verify: scripts/tests/conductor-watch-ensure.test.sh（script-test 1/22）が失敗。本差分（docs 2 本と新規 docs テストのみ）と無関係な環境起因で、変更退避後の base ツリーでも同一失敗（behavior-fallback、behavior-systemd は実行により揺らぐ）を確認した。verify-lint・verify-static・test・残り 21 の script テスト（routine-safety-docs を含む）は個別実行で合格。Issue #340 round 1 と同じ扱いにし、CI の必須チェックに委ねる
2. 曖昧だった文: 「ルーティンの制限値（最短間隔、件数上限、連続失敗の自動停止回数）が決まり、仕様に明記されている → PRD または技術仕様の該当箇所を目視で確認」（Issue #194 受け入れ条件）。「技術仕様」（v0.7 統合版本文。sources 対応表と verification JSON で管理される保全対象）と PRD のどちらを正本にするかが一意でない
3. 仮定と解釈: 正本は PRD §11 と解釈した。通知方法と断る挙動は使う人から見た規則で PRD の他節と同じ性格であり、v0.7 body ch03 へ新節を足すと verification/section-correspondence.json・current-sections.json・sources 対応表の更新を要するため選ばなかった。判断の記録（OpenBot と Rakazo の比較、調整理由）は #213 と対になる ADR 0011 に書いた。方式は Issue の「採用する方式」が選定済みのため選択待ちには入らない
4. 足りなかった情報・検証手段: OpenBot の「有効な定期実行は20件まで」の数え単位（利用者ごとか Bot ごとか）が docs/competitors/openbot.md に明記されていない。Pelway の権限・費用の単位に合わせて AI Bot 1つにつき20件と解釈し、会社（workspace）単位の上限は未決として ADR 0011 に記録した
5. 改善案: .agents/skills/write-issue/SKILL.md の「関連情報」を書く手順に「参照先の文書に該当節が実在することを確認する」を足す。Issue #194 の関連情報「PRD の定期実行に関する記述」は、prd.md に定期実行の節が存在しない（MVP 除外リストの日次ダイジェストが最も近い）まま書かれているため

使ったスキル: implement-issue、write-pull-request

## 実装者メモ（round 2）

対象 head: 8143e5b8a4e8911bd8669b1eb50e52b32c4267c4

1. 詰まった箇所: scripts/tests/routine-safety-docs.test.sh: PR #410 レビュー round 1 の must-fix。AGENTS.md「LLM judges」が自然言語の意味の正規表現・キーワード採点を禁じているのに、受け入れ条件の文言を grep で判定していた。board-patrol-docs.test.sh・egress-host-docs.test.sh の先例に倣ったが、正本の規則が優先する
2. 曖昧だった文: 「仕様の内容を LLM と明示的な rubric で評価する形にするか、意味を判定するテストを外してください」（PR #410 レビュー本文 must-fix）。LLM 評価の置き場（evals/ は agent-team ケース用で、docs の意味評価の経路が無い）が無く、rubric と判定 call の費用負担も本 PR の範囲外
3. 仮定と解釈: 意味の判定はテストから外し、Issue #194 の受け入れ条件どおり目視確認とレビューに委ねた。代わりに構造だけ（ADR 0011 の実在と prd.md 相対リンクの解決）を残した。壊れたリンク（adr/9999）を一時的に入れて FAIL、戻して ok を確認し、テストが条件を確かめていることを示した。選ばなかった解釈: evals/ に docs 判定の LLM rubric を新設する案（本 Issue の規模見積もり約60行を大きく超え、rubric の較正も review 済み事例が無いまま作ることになるため）
4. 足りなかった情報・検証手段: レビューの nit（メモの対象 head が 395e768 と異なる）は、skill implement-issue 手順 5〜6 の定義（対象 head＝メモコミットの親＝実装コミット 9a7228b、PR head＝メモコミット）どおりの値のため直さず、round サマリーで根拠を返した
5. 改善案: .agents/skills/implement-issue/SKILL.md の手順 4 に「受け入れ条件が目視確認のとき、prose の意味を grep で確かめるテストに置き換えない（AGENTS.md『LLM judges』。構造・リンク整合の検査に限る）」を足す。egress-host-docs.test.sh・board-patrol-docs.test.sh の既存の文言 grep も同じ規則に合わないため、改善ループ（skill improve-skills）で扱う候補

使ったスキル: implement-issue、receiving-code-review、write-pull-request
