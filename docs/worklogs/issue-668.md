# Issue #668 の作業ログ

## 実装者メモ（round 1）

対象 head: 0d5eab5f6b88e6ee105267a6d89bf33f8d62ebc0

1. 詰まった箇所: 変更内容 4 の「棚卸し担当は提案（巡回記録への判断案と期限の記入）まで」が、`command-team-details.md` の既存文（「作り直す」「閉じる」の操作は人間に依頼する＝PR への依頼コメント＋部屋での知らせ）とどう両立するか。「提案まで」に PR への依頼コメントが含まれるか判断が要った。
2. 曖昧だった文: 「棚卸し担当は提案（巡回記録への判断案と期限の記入）までで、ラベル・マージ・スレッドへの指示はしない」（Issue #668 本文「変更内容」4）
3. 仮定と解釈: 禁止されているのは ラベル・マージ・スレッドへの指示の 3 つだけと読み、PR への依頼コメントと部屋での知らせは提案の伝達手段として棚卸し担当が続ける、と解釈した（`docs/agents/human-actions.md` H26 の「記録を PR にコメント」する条件を維持するため）。選ばなかった解釈: 巡回記録への記入だけで PR コメントを書かない案は、H26 の条件が二度と満たされなくなるため捨てた。あわせて H26 の実行者表記を「棚卸し担当または Grok Bot」に直した（Issue の対象ファイル一覧に無いが、実行者変更に伴うずれの是正）。棚卸し担当の provider・モデルは未決事項の既定案（Devin CLI `swe-2-high`）をそのまま runbook に書いた。
4. 足りなかった情報・検証手段: 棚卸し担当スレッドに `runtimeMode` を指定できるか。Devin CLI のスレッドは権限を指定できず Autonomous 一択のため、`full-access` にならないことを「レビューを始める」の Devin CLI の記述から裏付けた。
5. 改善案: `docs/agents/human-actions.md` の出どころの `パス:行番号` は基準コミット（`3afed983`）の行番号と明記されているため、human-actions.md 自体の編集ではずれない。実際にずれるのは main の参照先ファイルが編集されたときで、参照は基準コミットの古い位置を指し続ける。基準コミットを更新して出どころを再生成する契機（例: 定期的な再集計、または参照先の改訂に追従するルール）を「数えた基準」の節に書くことを、別 Issue で検討する。

使ったスキル: implement-issue、ja-text-communication

## 実装者メモ（round 2）

対象 head: 2a0d737b8c09d89fe3054ef9e3f53c59ec7eba6a

レビュー r1（CHANGES_REQUESTED）の must-fix 3 件と [文書のずれ] nit への対応。

1. 詰まった箇所: must-fix 1 の指摘どおり、round 1 のメモ項目 4 で「Devin CLI のスレッドは権限を指定できず Autonomous 一択」と裏付けた根拠が `devin_review`（レビュー用 sandbox）と `acpRegistry_devin`（`devin-impl` ラッパー）の混同だった。正本（`docs/superpowers/specs/agent-team.md` の制約: `mode` は `smart` まで・`bypass` 禁止）と `docs/dev-environment.md` の「実装担当の sandbox」の記述に合わせて runbook を書き直した。非 Devin provider は `t3 thread start` に権限フラグが無いため、権限を指定できる経路（T3 Connect の MCP・`t3_thread_launch` の `auto-accept-edits`）を指定し、立てられない場合はエスカレーションにする形にした。
2. must-fix 2・3: 起動者を指揮役（前回の棚卸しから 7 日以上経った巡回）と定め、成果の受け渡しは「`docs/worklogs/<日付>-inventory.md` と `<日付>-curation.md` を載せた報告用 Draft PR → 指揮役が巡回記録へ転記・部屋で通知 → 期限切れは次回を待たず runbook 7 のエスカレーション」に設計した。棚卸し担当は巡回上、実装・調査と同じ `state:` 行の分類で扱い、settle は報告 PR のマージ・クローズに紐づける。
3. 確かめ方を弱める変更: 無し。
4. 検証: `mise run verify` は `deny-check`（advisories）で失敗。原因は新規公開の RUSTSEC-2026-0592（`url` 2.5.8。修正は 2.5.9）で、変更を退避したベース（ccafd3e9）でも再現する環境起因。`scripts/script-test.sh` は 45/46 通過、失敗の `verify-pelway/verify-run.test.sh` もベースで同様に失敗（sandbox の /tmp 挙動）。変更は .md のみで Cargo.lock/deny.toml 系は無変更。
5. レビュー指摘の対応内訳: nit「round 1 メモ項目 5 の前提誤り」は項目 5 を正しい前提（行番号は基準コミットに固定・ずれるのは参照先の改訂時）に書き換えた。PR 本文に `Closes #668` と `Implementer:` 行が無かった件は、本文をテンプレート構成に整えて追記した。

使ったスキル: implement-issue、ja-text-communication

## 実装者メモ（round 3）

対象 head: `975b522578863ed8391f7e9462437c7b9ac1d7db`

1. 詰まった箇所: r2 must-fix 1（報告 PR が `pr:` 確認・再開指示・Issue 起点のレビュー/QA に載らない）の直し方に 2 案があり、レビュアの第 2 案（指揮役が週ごとに棚卸し Issue を起票し、報告 PR がそれを閉じて既存の実装の手順に載る）を採った。第 1 案（`review-pr`・runbook §6 に `agent/inventory-` の例外を足す）は `.agents/skills/review-pr/` を変える PR になるため本 PR 自体の委託レビューができなくなり（implement-issue 手順 8）、週次の報告 PR がすべて恒久的な例外経路を通る設計になるため退けた。
2. 曖昧だった文: 「例えば review-pr に `agent/inventory-` の例外を足す（ただし `.agents/skills/review-pr/` を変える PR は委託できない。implement-issue 手順 8）。または、指揮役が週ごとに棚卸しの Issue を立て、報告 PR がそれを閉じる形にして、Issue を前提にした既存の手順にそのまま乗せる」（PR #686 の r2 レビュー must-fix 1 の「直し方」）
3. 仮定と解釈: 棚卸し Issue は `ready-for-agent` を付けず（人間の着手の合図を待たない定期作業）、指揮役が `write-issue` の形で起票して `gh project item-add` でボードに載せ、`agent:working`・`agent:devin` を付けてから起動する、と解釈した。同時数の枠は Issue を持つ実装スレッドになったため Devin CLI の実装と同じく数える（旧設計の「枠で数えない」の根拠が消えた）。また must-fix 2 について、「人間が代行を頼んだ」の項目は読む条件（`command-team-details.md` の「代行の開始と終了」の読む条件）と重複するため消し、参照を項目 1 に戻した。nit「報告 PR をデータとして扱う」は、棚卸し担当が対象 PR にコメントを書かない設計（H26 の記録コメントは指揮役が書く）と、runbook 側で報告内容を GitHub の実状態で確かめる段を入れて応えた。
4. 足りなかった情報・検証手段: `ready-for-agent` 無しの Issue がボードに載らない場合の巡回の整合チェックの扱い。ボードへの追加は自動では起きないため、起票後に `gh project item-add` で載せる手順にした（載っていなければ Status 導出の書き込みが失敗しうる）。検証は `.md` のみの変更で、語句ピンのテスト（board-patrol-docs・conductor-compaction-docs・doc-drift-nit-docs・review-delegation-docs・review-isolation-docs・routine-safety-docs・settle-procedure-docs・worklog-curation・pr-body-round-history-docs・reply-guard-docs・same-kind-must-fix-docs）はすべて通過。`mise run verify` の `cargo test` は sandbox の read-only mount（worktree の `target/` に書けない）で失敗、`deny-check` は round 2 と同じ環境起因の既知失敗。
5. 改善案: 週次の棚卸し Issue の本文を指揮役が毎回書く負担を減らすため、本文の形を `docs/agents/command-team-details.md` の「棚卸し Issue」に固定した（今回追加）。これで起票が機械的になる。

使ったスキル: implement-issue、ja-text-communication
