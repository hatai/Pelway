# implement-issue から diagnosing-bugs と mutation-testing を参照する（2026-11-20）

出どころ: 人間からの直接依頼（T3 セッション「diagnosing-bugs と mutation-testing が効果的に参照されるようにして」）。`docs/agents/external-skills.md` の適用範囲の規定（「効果の判定と移行は hatai が別に判断する」）における hatai の移行判断として、両スキルの implement-issue からの参照を通常運用に移す。Issue を経由していない判断は人間の指示による。

## 根拠の対応表

| 変更 | 根拠（一次記録） | 期待する変化 |
|---|---|---|
| implement-issue 手順 4 に「テストが red にならない・揺らぐときは diagnosing-bugs の Phase 1〜2 で切り分ける」を追加 | 実装者が red の切り分けを都度独自に組み立てている: `docs/worklogs/issue-411.md`（並行実行時のみ失敗・単体では再現せず）、`docs/worklogs/issue-437.md`（初回 verify の `[test]` のみ競合で失敗・再実行で通過） | red が作れない・揺らぐ場面での切り分けが手順に固定される |
| implement-issue 手順 6 に「verify の無関係そうな失敗は diagnosing-bugs の Phase 2（退避しての再現）で切り分ける。環境起因なら内訳と再現確認を PR 本文に書いて先へ進めてよい」を追加 | 同種の切り分けが 6 round で反復: `docs/worklogs/issue-404.md`（失敗箇所が実行ごとに変動・stash で main でも再現）、`issue-405.md`（main で 6 回中 3 回失敗）、`issue-411.md`、`issue-437.md`、`issue-440.md`（toolchain 再インストール・「よくある失敗」への1行追加が改善案として積まれたまま）、`issue-470.md`。いずれも stash して main で再現確認する同じ方法を独自に再発明している | 環境起因の verify 失敗での止まり・手戻りの減少と、切り分け記録の形式の一定化 |
| implement-issue 手順 6 に「対象 crate のロジックに触れたら `mise run mutants:diff` を回し結果を実装者メモに書く。生き残りの分析は mutation-testing」を追加 | #439 本文の implement-issue に足す 1 行（「ロジックを変えたら、対象の crate で mise run mutants:diff を回し、結果を作業ログに書く」）。実行基盤は存在: `mise.toml` の `tasks.mutants` / `tasks."mutants:diff"`（#437 で導入・対象は `.cargo/mutants.toml` の examine_globs）。人間の直接指示により #439 のうち本項のみ前倒し（#439 の残り項目は Issue に残す） | ミューテーションテスト対象 crate のロジック変更で生き残りの確認と記録が義務化される |
| 輸入スキル節に diagnosing-bugs・mutation-testing の適用ルールを追加（tdd は試行枠のまま） | `docs/agents/external-skills.md` の表との整合（適用ルールは役割スキル側に置く決まり）。mutation-testing は mewt/muton 用のため、使う範囲を結果の解釈の節に限定し cargo-mutants と読み替える適用ルールが必要 | 参照の解決先と使う範囲が一意に定まる |
| `docs/agents/external-skills.md` の表更新（diagnosing-bugs 行への implement-issue 追記・mutation-testing 行の新設・移行済み一覧への追記） | 同文書が輸入スキル採用の正本であること（取り込みの discipline の構成） | 正本と役割スキルの参照関係の一致 |

## 想定と実績の区別

- 裏付け済み: 切り分けの反復（上記 6 round）、mutants 基盤の存在、#439 の本文。
- 想定: 参照によって切り分けの質が上がること自体は未測定（効果測定は command-team 手順 4 へ引き継ぐ。観察する指標: 実装者メモの「詰まった箇所」での切り分け記述の一貫化、環境起因失敗での needs-human 化の減少）。

## #439 との関係

本変更は #439 の「やること」のうち implement-issue への 1 行に相当する。review-pr への行・手順書・QA 受け入れ条件は含まないため #439 はクローズせず、PR で該当箇所を実施した旨を #439 にコメントして残りを可視化する。

## 実装者メモ（round 1）

対象 head: cdfbd638950f91d4709874e750317da3701dbad2（push 後の署名付け替え後の SHA。ローカルの eac3820 から付け替え）

1. 詰まった箇所: `docs/agents/external-skills.md`: 適用範囲の bullet は mattpocock/skills 節の中にあり、trailofbits 由来の mutation-testing の移行記録をそこに書けない。非 mattpocock 表（役割スキルから参照する外部スキル）の行として追加し、移行記録には diagnosing-bugs だけを書いた。
2. 曖昧だった文: 「適用範囲: 通常運用への移行が決まるまで、試行の対象と指揮役が試行として明示した割り当てに限る。」（`docs/agents/external-skills.md`）— 人間の今回の指示が「移行の判断」に当たるかは、指示が hatai 本人から来ていることと規定の主語（hatai が別に判断する）から判断した。
3. 仮定と解釈: mutation-testing の適用範囲を「結果の解釈の節に限定」としたのは、本文が mewt/muton のルーティング構造のため。cargo-mutants の出力（caught/missed/timeout）は同節の概念（TestFail/Uncaught/Timeout）に対応するという解釈で書いた。
4. 足りなかった情報・検証手段: cargo-mutants の生存ミュータント分析をこの適用ルールで実際に回した実績（#439 の残りと合わせて初回実施時に確認する）。
5. 改善案: なし。

使ったスキル: find-skills（ユーザーレベル）、write-pull-request（`.agents/skills/write-pull-request/SKILL.md`）
