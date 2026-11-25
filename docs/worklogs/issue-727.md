# Issue #727 の作業ログ

## 実装者メモ（round 1）

対象 head: afe1714e5b3adfb6a77b2f17ed4302f75862b786

1. 詰まった箇所: 実機確認: `t3 thread list --json` が「No running T3 Code server uses /home/hatai/.t3」でつながらず（runbook にある所有権の衝突。開発リードの指示どおり直さない）、`createdBy` / `creationSource` の実値は確認できなかった。`scripts/conductor-watch.sh` の `createdBy != "agent"` の約束と合成データで実装し、未確認であることは PR 本文に書く。
2. 詰まった箇所: 手順 6: この環境では `/tmp` が nobody（uid 65534）の所有で、`.agents/skills/verify-pelway/verify-run` の `dir_chain_safe` が「置き場より上の要素は自分か root の所有」を要求するため `verify-run.test.sh` が環境起因で落ちた。`TMPDIR=/mnt/pelway-tmp`（自分の所有の 700）で `mise run verify` を回し直すと script-test 55 件すべてを含め通った。
3. 曖昧だった文: 「人間が立てたスレッドかどうかを、T3 が実際に返す項目（`t3 thread list --json` / `show --json` の createdBy など）で見分ける」（Issue #727「やること」）—「実際に返す項目」をこの環境では実機で確かめられなかったため、リポジトリ内の既存の約束（conductor-watch.sh・grokbot-runbook の shared-worktree の項）に合わせた。
4. 仮定と解釈: 「人間が立てた」= `createdBy` が存在し `"agent"` でない（`"user"` など将来の値も人間扱いに含める）。`createdBy` が無い・文字列でないときは「人間が立てた」とは扱わず従来どおり起こす（起こし漏れを避ける側に倒す、という受け入れ条件どおり）。T3 Connect（MCP）経由のスレッドは `createdBy: "agent"`（開発リード確認済み）なのでエージェント扱いになり、扱いは変わらない。
5. 足りなかった情報・検証手段: 動く T3 サーバーへの `t3 thread list --json` / `show --json` での `createdBy`・`creationSource` の実値（人間が UI で立てたスレッドと MCP 経由のスレッドの組み合わせ）。
6. 改善案: `docs/agents/notify-payloads.md` の payload の項に、payload 自体は立て手を持たず起こしの判定では `t3` の `createdBy` を別途見る旨を書くと、受信箱の行だけを見た読み手が立て手を payload に探す迷いが減る（今回は同ファイルの「指揮役の受信箱」の「起こし」の項に書いた）。

使ったスキル: implement-issue、tdd（seam: `decide_wake` に `WakeNotice`（種類・報告の有無・立て手）を渡す引数、および結合試験の fake `t3` 出力の `createdBy` 項目）
## 実装者メモ（round 2）

対象 head: 332c17a56773a4d29708de187e8c57479e5934b6

1. 詰まった箇所: Codex の must-fix（受信箱の追記の失敗のあとの再試行で立て手を失い、人間スレッドの報告の無い finished を立て手不明として起こしてしまう）。`inbox_pending` を payload そのものの列から `{ "notice": <payload>, "created_by": <立て手 | null> }` の形に変え、その時点で判明していた立て手（詳細 → 一覧）を記録する形にした。前の版の記録（`schema` 項目を持つオブジェクト）は payload としてそのまま読み、立て手はその周の一覧・詳細から取り直す（後方互換）。`docs/worklogs/issue-611.md:19` の round 2 修正も同じ「記録に足りない項目を足し、旧形式も読める形にする」パターンだった。
2. 詰まった箇所: Codex・Claude の共通 must-fix（指揮役自身の知らせと人間の静かな知らせが同じ周回だと起こしてしまう）。`decide_wake` で「起こすに足るか」の判定に入る前に登録された指揮役自身のスレッドの知らせを除き、除いた残りだけで判定する形にした。除外の前後で判定が分かれるのは「自身の知らせだけ」のとき（`own-event` が先に出る）と「自身 + 静かな知らせ」のとき（`human-non-urgent`）で、どちらも起こさない。
3. 仮定と解釈: Codex の「指揮役のスレッドの知らせは除く」は、Claude が引用した `docs/worklogs/issue-555.md` の「指揮役の巡回の終わりの知らせ」だけでなく、登録されたスレッドからのすべての知らせ（`createdBy` が `agent` でも human-quiet でも）を含むと読み、登録値との完全一致で除く形にした（`own-event` の判定と同じ「登録されたスレッドの知らせは数えない」という約束と一致する）。「自身の知らせを数えない」を文書に書くことは Claude の must-fix の直し方の一部として挙がっていた（同ファイルに足すよう挙がっていたが、起こしの決まりの正本は dev-environment.md なので、そちらの「その回の知らせ」の説明に足した）。dev-environment.md の #727 の項には、自身の知らせを数えないことと再試行の記録に立て手を残すことを足した。
4. 足りなかった情報・検証手段: 結合試験での追記の失敗の再現。`io.rs` の「`pelway/` 内のパスは `O_NOFOLLOW` で開く」という約束を使い、受信箱のパスを dangling symlink にすることで `append_inbox` の失敗を確実に再現した。
5. 改善案: `decide_wake` の引数が「登録された指揮役のスレッド ID」を直接受ける形なので、テストで除外の対象を確かめるときに結合試験（登録ファイル + fake t3）まで書かなくても単体試験で確かめられる。ただし単体試験では thread_id の一致だけを確かめ、登録値と一致するか（`io::read_id_file` 経由）は結合試験でしか確かめられない。docs が「判定の順は…知らせの種類と元のスレッドの立て手」と並べるのと、コードが「own か」の判定を `all` と「起こすに足る」判定の両方で使うのが対応を追いづらいので、`decide_wake` の doc に「自身の知らせを除いた残りで判定する」と明記した。

使ったスキル: implement-issue

## 実装者メモ（round 3）

対象 head: ff07e3cfb35ee81db826a824cccf790177e9ef88

1. 詰まった箇所: Codex の must-fix（再試行の記録を `{ "notice": ..., "created_by": ... }` に包んだため、更新前のバイナリに戻すと包んだ形のまま受信箱に書かれ、`conductor-watch.sh` が拾えない行になる）。round 2 は「立て手を記録に残す」だけを見て、戻した版が記録をそのまま書き直す向きの互換を見落とした。記録の要素は payload そのものに戻し、判明していた立て手は状態の別項目 `inbox_pending_creators`（thread_id → createdBy）に持つ形に変えた（レビュー役の提案どおり）。中間の版が書いた包んだ形の記録は payload と立て手に戻して読む。
2. 仮定と解釈: 立て手はスレッドごとに変わらないので、別項目のキーは thread_id で十分とした（同じ thread_id の知らせが複数残っても立て手は同じ）。記録から消えたスレッドの項目は残るが、スレッド数で頭打ちの小さな量なので掃除はしない。`[文書のずれ]` の nit はこの round で直した（Codex 3 件・Claude 3 件。dev-environment.md の条件の書き方と PR 本文の更新は両者が同じ箇所を指していて、残りは notify-payloads.md の「文字列の値を持つ」と、round 2 メモの「実際には無かった指摘への対応を書いた」1 文を消す修正）。`human_waiting_wakes` の waiting と stalled の同居は Codex が「別 Issue」に分類したのでこの PR では直さない。Claude の印の無い nit（サマリーの `<R>` の読み違い。main の skill どうしの食い違いが原因と分類）もこの PR の外なので直さない。
3. 足りなかった情報・検証手段: 戻した版での再試行そのものは結合試験で再現できない（旧版のバイナリが無い）ため、記録が payload そのものであること（`schema` 項目を持ち `notice`・`created_by` を持たない）を状態ファイルを読んで確かめる形で担保した。
4. 改善案: 状態ファイルの「記録の形」は、書き方だけでなく「ひとつ前の版がそのまま解釈したときに何をするか」も設計に含めると、この種の後方互換の見落としが減る。`inbox_pending` の doc に「要素は payload そのものに留める」という約束を書いた。

使ったスキル: implement-issue
