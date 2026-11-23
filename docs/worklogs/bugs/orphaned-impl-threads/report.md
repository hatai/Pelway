# 調査報告: 実装スレッド 13 件の孤立（completed・未 settle で滞留）

調査日: 2026-11-22（JST）
調査者: 指揮役スレッド `mcp:57618b05-7233-4524-9910-75946dd4a367`（Devin・恒久交代後の現指揮役）
依頼: 「13件が孤立した原因をより詳細に調査し、issue を立ててください」（指揮役スレッドへの人間からの指示）

## 1. 報告の整理

- 期待: 完了した実装・レビュースレッドは、次の指示（レビュー回付・push ゲート・修正依頼）か settle によって回収され、open PR を持つスレッドが通知なしに放置されない。
- 実際: `status: completed`・未 settle・トップレベルのスレッドが 13 本滞留した。うち 11 本が open PR、2 本が PR なし（指揮役スレッド run 6 の棚卸し結果）。
- 対象: Pelway の AI 開発チームの T3 実装・レビュー・検討スレッド。
- 場所: T3 Code（プロジェクト `e73c7c04-fd96-49c9-a3d0-71aa74f9ee3f`）、監視はホストの `conductor-watch` デーモンと指揮役の定期巡回。
- 時刻: 滞留は 2026-11-20 以降に累積。ユーザーによる指摘・確認は 2026-11-22 11 時台 JST。
- 頻度: 常時（完了したスレッドが次々に滞留に加わった）。

## 2. 「孤立」の定義と 13 件の内訳

T3 のスレッドは pull 型である。1 回の run が `completed` で終わると、通常の指示配送では新しいメッセージ（`t3_thread_send`）が届くまで起きない（指示配送以外の起動 — スケジュールタスクの起床や中断後の自動起き直り — については 6 節の未確定事項を参照）。指示の配送は指揮役の仕事であり、settle（整理済みにする操作）は手動である。したがって「孤立」とは、**open PR や作業中の Issue を持つ完了済みスレッドに、次の指示も settle も届かず、誰の監視対象にもなっていない状態**を指す。

13 件（2026-11-22 11 時台 JST・指揮役スレッド `mcp:57618b05` run 6 の `t3_thread_list` スナップショットで確定。記録は `docs/worklogs/2026-11-22-conductor-substitute-11.md` の「13件孤立の詳細調査」節。ユーザー指摘の 5 本＋同条件での追加 8 本。表の「最終更新」はスナップショット時点の `updatedAt`）:

| # | スレッド | 紐付く Issue/PR | 最終更新（UTC） | 分類 |
|---|---|---|---|---|
| 1 | `mcp:970f6c1e-…` `[実装] #333` | #333 / PR #560 | 2026-11-21T15:40:50Z | **真の孤立**（push 指示が `pending_thread_sends` に滞留・最古件） |
| 2 | `mcp:dab971b2-…` `[実装] #306` | #306 / PR #567 | 2026-11-21T14:59:36Z | 意図的保留（needs-human・経路 C の人間判断待ち）。レビュー r2 未回付の孤立面もあり |
| 3 | `mcp:28d8f688-…` `[実装] #480` | #480 / PR #482 | 2026-11-21T13:04:20Z | 意図的保留（競合解消を別途委託中・人間マージ待ち） |
| 4 | `mcp:3f022128-…` `[実装] #490` | #490 / PR #497 | 2026-11-21T07:18:41Z | 意図的保留（hatai の再承認待ち・ホールド指示済み） |
| 5 | `[実装] #569`（スナップショット範囲外・スレッド ID は未特定。6 節参照） | #569 / PR #600 | — | needs-human ラベル付き。APPROVED・Draft のまま次工程（QA）の回付が行われていない孤立面もあり |
| 6 | `mcp:launch:caa0ff74…` `[実装] #602` | #602 / PR #610 | 2026-11-22T02:11:14Z | settle 負債（後継スレッド `mcp:launch:2e87e67b…` が継続中。別の巡回記録では元スレッドが running と観測された時点もあるため、棚卸し時点の `completed` は以後に変化している） |
| 7 | `mcp:launch:3bcaf0e3…` `[実装] #613` | #613 / PR #624 | 2026-11-21T16:33:25Z | **真の孤立**（REVIEW_REQUIRED・レビュー未回付） |
| 8 | `mcp:launch:9c9657f9…` `[実装] #615` | #615 / PR #625 | 2026-11-21T16:04:22Z | **真の孤立**（REVIEW_REQUIRED・レビュー未回付。needs-human も併存） |
| 9 | `mcp:5c599260-…` `[修正] #481` | #481 / PR #530 | 2026-11-21T15:14:24Z | **真の孤立**（CHANGES_REQUESTED の must-fix 指示が send 障害で未配送→旧指揮役が PR コメント issuecomment-7072879032 で代替配信） |
| 10 | `mcp:5fc0e3f5-…` `[実装] #295` | #295 / PR #463 | 2026-11-21T13:10:43Z | 意図的保留（CONFLICTING・#581 の設計判断待ち） |
| 11 | `mcp:16efc920-…` `[実装] #457` | #457 / PR #464 | 2026-11-21T05:45:42Z | needs-human。QA 経路（pelway-qa）待ちの孤立面もあり |
| 12 | `mcp:aec2522e-…` `[検討] #588` | #588 / PR なし | 2026-11-21T08:22:03Z | settle 負債（完了済み検討・PR なし） |
| 13 | `cb1a1122-…` `CI セキュリティチェックの検出事項の修正` | PR なし | — | settle 負債（人間起動・completed） |

集計の基準と各スレッドの帰属:

- **真の孤立**（起こすべき指揮役が配送経路の障害または未回付で次の指示を出せていない）: 1・7・8・9 は未配送・未回付が確認済み。2・11 は意図的保留が主だが、保留中に届くはずだったレビュー回付・QA 経路の未回付があり、上限として含めると最大 6 件。
- **意図的保留**（待ち先が人間または別作業で明確）: 3・4・10 と、2・5・11 の保留面。「4〜5 件」は 2・11 を両側に数えるため幅がある。
- **settle 負債**（作業の回収ではなく settle だけが残った）: 6・12・13。

「13 件すべてが孤立」ではなく、3 種類の状態が `completed` 1 つのステータスに潰れていたことが把握を困難にした。

## 3. 根本原因（確認済み）

孤立は単一の故障ではなく、確認できた二つの症状（監視側の失明・起こす側の障害）と 2 つの構造的弱点の重なりで起きた。2 つの症状が独立した障害か同一原因かは、内部原因が未確定のため判断できない（6 節）。

### 3.1 監視側の経路が死んでいた（一次原因）

`conductor-watch` は本来この孤立を検知する仕組みを持っている（Issue #504 で実装）。対象コミット（`c2ac7050`）の `scripts/conductor-watch.sh` では、GitHub の Issue・PR 一覧の取得に成功し、かつ両一覧が件数上限で打ち切られていない場合に限り、open Issue ごとにラベルから `waiting_on` を求め、その値が `implement` か `review` の場合に限って停止スレッド判定に入る。判定では、Issue 番号ごとの最新の「`[実装] #<N>`」題名のスレッドが `completed/failed/interrupted/cancelled` で、受信箱（`conductor-inbox.jsonl`）の最新の状態行が `working` か無記録のとき、`PATROL`/`STALL #<N>` 行を出して `waiting_on=conductor` に倒す（同ファイル 224 行の `STOPPED`、289-297 行の判定と `PATROL`/`STALL` 出力）。同じ `updatedAt` の STALL は再通知しない。`needs-human` の Issue は最初から `waiting_on=human` と決まり、この分岐には入らない（2・5・11 番は列挙障害が無くても STALL の対象にならない）。

しかし 2026-11-20 00:50 JST 以降、スレッド列挙が失敗し続けた:

- `t3 thread list --json` が `No running T3 Code server uses /home/hatai/.t3` で失敗（`patrol-state.json` の `watcher_blind` 記録、`recorded_at: 2026-11-21T01:50:00+09:00`）。
- 一方で T3 サーバー（pid 309767）は稼働中で、MCP 経由の `t3_thread_list` は正常だった。つまり **CLI 検出経路だけが壊れ、サーバー自体は生きていた**。

沈黙のしかたが致命的だった。`scripts/conductor-watch.sh` 54 行:

```sh
"$T3_BIN" thread list --json > "$D/threads.json" 2>/dev/null || true
```

- リダイレクトで `threads.json` が空に切り詰められ、stderr は捨てられ、失敗は `|| true` で握り潰される。
- Python 側（87 行）`try: threads = json.load(...) except Exception: threads = []` が「列挙失敗」と「スレッド 0 件」を区別できない。
- 結果、**スレッド一覧に依存する検知**（THREAD イベント・上記の停止スレッド判定（PATROL/STALL））が止まる一方、GitHub の取得結果で動く PR・ラベル監視（LABEL/unbacked 系）は別経路のため動き続け、HEARTBEAT 行（10 分ごと）も出続けた。前回スナップショットを残す保護（読み取り失敗時に前回の `threads`/`prs` を維持する処理）により全スレッドを「新規」と誤報することはなかったが、**生きているように見える部分盲状態**が、記録で確認できる範囲では 2026-11-20 00:50 JST（`watcher_blind.since`）から同 11-21 01:50（`recorded_at`）までの約 25 時間続いた。復旧時刻は記録に残っておらず未確定（2026-11-23 時点では `threads.json` が正常に更新されており、いずれかの時点で復旧済み）。`watcher_blind` の記録が翌日 01:50 なのは、巡回が手で気づくまで検知手段がなかったため。

### 3.2 起こす側の経路が死んでいた（二次原因）

- `pending_thread_sends` の最古件は 2026-11-22 02:45 JST（#333 実装スレッド宛の push ゲート指示）。旧指揮役側の記録 `mcp_down.since: 2026-11-22T10:45+09:00` は「send/schedule 系が失敗」と記されていたが、操作単位で見ると失敗が確認できたのは send 系である:
  - 旧指揮役（`mcp:launch:61092c…`）: `t3_thread_send` の失敗が 10:45 JST 以降に記録（`mcp_down`）
  - 現指揮役（`mcp:57618b05…`）: 11:05-11:15 JST 台に `t3_thread_list`・`t3_thread_read`・`t3_thread_organize`・`schedule_task`・`t3_thread_interrupt` が疎通し（旧側の「schedule 系も不通」はここでは再現せず）、巡回タスク `fe5a87db` を 11:14 に作成成功。一方で直後の巡回 1（同日 11 時台 JST）で `t3_thread_send` 再送は 2 回連続「Failed to connect to MCP server」
  - つまり「send 系」だけが断続的に落ち、schedule/read 系はスレッドによっては通る。**読み取り系・schedule 系と送信系で障害の振る舞いが違う**ことが確認済みの事実（原因の下層＝なぜ MCP の send ルートだけ落ちるかは未確定、T3 プラットフォーム側）。
- 滞留中の未配送 3 件: #333 宛 push 指示・#568 宛締切共有・PR #631 宛競合解消指示（#631 はその後マージ済みのため指示は陳腐化）。

### 3.3 構造的弱点 1: 「次の連絡」を追う帳簿が無い

`patrol-state.json` は Issue/PR ごとの `waiting_on` を追うが、「スレッドごとの次の連絡予定・最終接触時刻」は保持しない。そのため「completed だが open PR を持ち、waiting_on も conductor に倒れていない」スレッドは、監視が死ぬと誰にも数えられない。前日の棚卸しでも「未 settle 13 件」は数えられたが「明確な待ち」の有無だけ見ており、起こす経路の死活は確認対象に含まれていなかった。

### 3.4 構造的弱点 2: 指揮役交代と共有状態の競合

- 約 34 時間で指揮役が 4 代交代（Devin 8→9→10→11、主立った交代は 11-22 01:45・10:41・11:15 JST）。各交代で保留キューの引き継ぎとスケジュールタスクの作り直しが発生し、MCP 障害期と重なって送り損ないが累積した。
- さらに旧指揮役スレッド（`mcp:launch:61092c…`）が settle・中断の後も自動起き直りして巡回を続行し、`patrol-state.json` を上書きした（`watch_log_offset` が 507473 → 3796 に逆戻り、`last_patrol_note` が旧指揮役の巡回記録で上書きされたことを巡回で実測）。`conductor-watch/watch.log` では当該スレッドの `prev=completed` から `running` への遷移が計 22 回記録されているが、定期巡回の起床を含むため settle 後に限った回数は分離できていない。割り込み後の自動起き直りそのものは、現指揮役の interrupt 記録（同 worklog「run 22・23。23 は割り込み後に自動起き直りが発生し、再度中断で停止」）で実測されている。共有状態ファイルが複数の指揮役から同時に書かれ得る設計になっている。
- 現指揮役の巡回スケジュールタスク `scheduled-task:fe5a87db-…` も `enabled:false` で無効化されているのを確認（誰が/いつ無効化したかは未確認）。

## 4. 証拠一覧（一次記録）

| 主張 | 証拠 |
|---|---|
| 13 件の構成 | 指揮役スレッド `mcp:57618b05` run 6 の棚卸しメッセージ（`completed`・未 settle・トップレベルで 13 本と明記）と、[同 worklog の記録](https://github.com/hatai/Pelway/blob/679086bb6a5aed752280ed508701fa3336a92240/docs/worklogs/2026-11-22-conductor-substitute-11.md)（「13件孤立の詳細調査」節） |
| 監視の失明の期間と症状 | `patrol-state.json` の `watcher_blind` キー: `{"since":"2026-11-20T00:50:00+09:00","symptom":"t3 thread list --json が \"No running T3 Code server uses /home/hatai/.t3\" で失敗（wsl-runtime 実バイナリでも同じ）。threads.json 空→state.json に現行スレッドなし・watch.log 00:50 以降無記録","recorded_at":"2026-11-21T01:50:00+09:00"}` — 連続失敗は `watch.log 00:50 以降無記録` の記述が示す。終了時刻は未記録 |
| 失明のコード上の原因 | `scripts/conductor-watch.sh` 54 行の失敗握り潰し（`> "$D/threads.json" 2>/dev/null \|\| true`）、87 行の `except: threads = []` |
| STALL 検知の存在と対象・条件 | `scripts/conductor-watch.sh` 224 行（`STOPPED`）・289-297 行（waiting_on が implement/review・最新実装スレッド停止・状態行 `working` か無記録のとき PATROL/STALL・同 `updatedAt` は再通知しない）。`needs-human` は分岐に入らない（3.1 節） |
| send 障害 | `patrol-state.json` `mcp_down.since: 2026-11-22T10:45+09:00`（旧指揮役側の記録）、`pending_thread_sends` 3 件（最古 11-22 02:45）、現指揮役側の巡回 1 で `t3_thread_send` 2 回連続失敗（[同 worklog](https://github.com/hatai/Pelway/blob/679086bb6a5aed752280ed508701fa3336a92240/docs/worklogs/2026-11-22-conductor-substitute-11.md)「巡回 1」節。同日 11:05-11:15 台の `schedule_task` 疎通は同 worklog「引き継ぎ確認」節） |
| 旧指揮役の自動起き直りと共有状態の上書き | `conductor-watch/watch.log` で `mcp:launch:61092c…` の `prev=completed`→`running` 遷移が計 22 回（定期巡回の起床を含む）。割り込み後の起き直りは[同 worklog](https://github.com/hatai/Pelway/blob/679086bb6a5aed752280ed508701fa3336a92240/docs/worklogs/2026-11-22-conductor-substitute-11.md)の中断記録（run 22・23）で実測。上書きは `watch_log_offset` 507473→3796 の逆行と `last_patrol_note` の旧指揮役記録での置き換えを巡回で実測（同 worklog「増幅」の記録） |
| 巡回の実値 | [`docs/worklogs/2026-11-22-conductor-substitute-11.md`](https://github.com/hatai/Pelway/blob/679086bb6a5aed752280ed508701fa3336a92240/docs/worklogs/2026-11-22-conductor-substitute-11.md)（PR 12・agent:working 9・needs-human 7） |

## 5. 最小の修正案

この調査の引き渡し先の Issue で直すべき範囲（実装はしない）:

1. `scripts/conductor-watch.sh` で `t3 thread list` の失敗を検知して `WATCHER thread-list-failed` 行を watch.log に出す（終了コードと stderr を保持、成功した最後のスナップショットは失敗で上書きしない）→ Issue #651 として起票済み、PR #664 で実装済み（7 節参照）。
2. 同様に `pending_thread_sends` の最古件がしきい値を超えた滞留を知らせる手段を持つ（巡回プロンプトまたは監視側）→ Issue #669 に引き継ぎ。
3. 回帰試験: 列挙失敗をモックして「イベントが出る・前回スナップショットが残る・HEARTBEAT だけで見逃されない」ことを `scripts/tests/conductor-watch*.test.sh` の流儀で検証する → PR #664 の `conductor-watch-thread-list-fail.test.sh` で実装済み（7 節参照）。

加えて構造的弱点は: 3.3 節（次の連絡帳簿）→ Issue #670、3.4 節（`patrol-state.json` の多重書き込み競合と offset 巻き戻り保護）→ Issue #671 に引き継ぎ。起票判断は指揮役が [PR コメント](https://github.com/hatai/Pelway/pull/650#issuecomment-7090579245)に記録した。

T3 プラットフォーム側の send 障害そのもの（MCP の send ルートが読み取り系と別に落ちる件）はリポジトリで直せない。ホスト側の対応（T3 アプリ再起動等）は人間に依頼済み。

## 6. 未確定・残課題

- `t3 thread list` が `No running T3 Code server` を返しつつ MCP `t3_thread_list` が通る理由（CLI のサーバー検出方式の差）は未確定。`watcher_blind` 記録の推定どおりホスト側の検出不良が濃厚だが、T3 の内部は検証していない。
- `t3_thread_send` の障害が read 系と切り分けられる内部理由は未確定。
- 旧指揮役が settle 後に自動再起動したトリガー（残存スケジュールか、T3 側の再試行か）は未確定。当時のスケジュールタスク `a3fcb862` は削除済みだが、削除前後で挙動が続いた記録がある。
- 「13 件」のうち #569 の実装スレッド ID は監視スナップショットの範囲外で特定できていない（タイトル検索に非一致）。

## 7. 修正の有無と配備状況

| 項目 | 状態（報告の更新時点・2026-11-23） |
|---|---|
| 修正案 1（thread list 失敗の WATCHER 行） | Issue #651 起票・PR #664 で実装済み。PR #664 はレビュー・QA 済みで `automerge` 付きの kodiak キュー待ち。マージ・本番デーモンへの配備は未確認 |
| 修正案 2（pending_thread_sends 滞留の検知） | Issue #669 に引き継ぎ（needs-triage。実装未着手） |
| 修正案 3（列挙失敗の回帰試験） | PR #664 の `scripts/tests/conductor-watch-thread-list-fail.test.sh` で実装済み。配備は 1 と同じく kodiak キュー待ち |
| 3.3（スレッド別・次の連絡帳簿） | Issue #670 に引き継ぎ（needs-triage。実装未着手） |
| 3.4（patrol-state.json 競合・巻き戻り保護） | Issue #671 に引き継ぎ（needs-triage。実装未着手） |
| T3 プラットフォーム側の障害（send 系・自動起き直り・CLI 検出） | リポジトリで直せない。ホスト側対応は人間に依頼済み。起票の要否は #671 の未決事項として hatai 判断待ち |

## 8. 再現（本調査で試した操作と結果）

再現テストではなく、障害の存否を確かめるために行った操作と結果:

- `t3_thread_send` による `pending_thread_sends` 滞留分の再送 ×2（2026-11-22 巡回 1）→ 2 回とも「Failed to connect to MCP server」。同時刻の `t3_thread_read`・`t3_thread_list`・`list_scheduled_tasks` は成功（3.2 節）
- `t3_thread_list`・`t3_thread_read`・`t3_thread_organize`・`schedule_task`・`t3_thread_interrupt` の疎通（現指揮役スレッド、2026-11-22 11:05-11:15 JST 台）→ すべて成功。`schedule_task` 不通は現指揮役側では再現せず（3.2 節）
- 旧指揮役スレッド `mcp:launch:61092c…` への `t3_thread_interrupt` ×2（run 22・23）→ run 23 は割り込み後に自動起き直り、再度中断で停止（3.4 節）
- `t3_thread_list` のスナップショット棚卸し（指揮役スレッド run 6）→ 13 件の確定（2 節）
- `conductor-watch/watch.log` の `mcp:launch:61092c…` の `prev=completed`→`running` 遷移の集計 → 22 回（定期巡回の起床を含む。3.4 節）
- `patrol-state.json` の `watcher_blind`・`mcp_down`・`pending_thread_sends` の読み取り → 3.1・3.2 節の各記録

CLI の `t3 thread list --json` が落ちる環境条件自体の再現手順は確立していない（11-20 00:50 JST 以降に自然発生し、記録時点では MCP 経由は正常。3.1 節・6 節）。

## 9. 使ったスキル

- `investigate-bug`（報告の形式・`docs/worklogs/bugs/<slug>/report.md` の置き場。同ディレクトリの `agent-watch-not-running/report.md` を形式の先例として参照）
- `write-issue`（#651 の起票。報告からの引き継ぎ先 #669/#670/#671 の起票判断は指揮役が同じ形で実施）
- `command-team`・`docs/agents/grokbot-runbook.md`（巡回記録・`patrol-state.json` の各補助記録の読み方）
