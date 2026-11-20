# Issue #526 の作業ログ

## 実装者メモ（round 1）

対象 head: e8eb80fe7ba8feaddf849413cb1138e7ad779095

1. 詰まった箇所: `mise run verify` の script-test: `scripts/tests/settle-procedure-docs.test.sh` の条件 4 が、置き換えた runbook の文「次のラウンドに置き換わった前ラウンドのレビュースレッド」を固定していて落ちた。新しい settle の条件 3 の文を確かめる形に向け直した。`scripts/tests/conductor-compaction-docs.test.sh` の「巡回 20 回以上」も同じく 75 回に直した。
2. 曖昧だった文: 「#548 が未マージの間は、この Issue に着手しない」（Issue #526 の背景・前提）— hatai の「次の Issue に着手して」を新しい指示として扱い、#548 の PR #558 のブランチに積む PR にした（状態の行の語彙を参照するため、#548 の内容の上に書く）。
3. 仮定と解釈: 「チームのスレッド」を、割り当ての記録の表にあるスレッド・指揮役が立てたスレッド・それらの委託の子タスクと定義した。`grokbot` の巡回の目安（`next_check_at` の表）は、スクリプトの `check_hours` と同時に直す #504 に残した（ここで表だけ足すと、スクリプトと表が食い違うため）。
4. 足りなかった情報・検証手段: 受信箱（#555）と見張りの `grokbot` の扱い（#504）は未実装のため、それらを使う手順はこの PR の時点では動かない（文書の上で先に置いた）。
5. 改善案: `.agents/skills/improve-skills/SKILL.md` の手順 5 の探し方に「置き換える文の特徴的な語句は、置き換え前の文のすべての節（読点で区切った各部分）で探す」を足す（今回は列挙の先頭の語句で探し、後ろの語句を固定した試験を見落とした）。

使ったスキル: implement-issue、improve-skills（`.agents/skills/improve-skills/SKILL.md`）、writing-for-agents（`.agents/skills/writing-for-agents/SKILL.md`）、write-pull-request

### improve-skills 手順 7 の引き渡し記録

- 候補の出どころ:
  - `docs/worklogs/2026-11-18-conductor-substitute.md:246`・`:260-262`（未 settle 30 件、誤 settle 2 件）
  - `:253`（#381 の「介入なし」）
  - `docs/worklogs/2026-11-14-conductor-substitute.md:261`（#256 の約 40 分の放置）
  - Issue #504 の背景（#488・#480・#490 の約 2 時間の止まり）
- 対象の改善点: runbook・command-team の巡回の手順。手順の欠落（全スレッドの点検・指示待ちへの指示が無い）と、矛盾（「介入しない」が止まった実装スレッドを待つ根拠になった）
- 期待する変化: 指揮役の側で、指示待ちの放置時間が 2 巡回以内に収まり、毎日のまとめの時点の不要な未 settle が 0 件になる
