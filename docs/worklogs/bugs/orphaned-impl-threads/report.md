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

T3 のスレッドは pull 型である。1 回の run が `completed` で終わると、そのスレッドは新しいメッセージ（`t3_thread_send`）が届くまで二度と起きない。指示の配送は指揮役の仕事であり、settle（整理済みにする操作）は手動である。したがって「孤立」とは、**open PR や作業中の Issue を持つ完了済みスレッドに、次の指示も settle も届かず、誰の監視対象にもなっていない状態**を指す。

13 件（指揮役スレッド run 6 の棚卸しで確定。ユーザー指摘の 5 本＋同条件での追加 8 本）:

| # | スレッド | 紐付く Issue/PR | 最終更新（UTC） | 分類 |
|---|---|---|---|---|
| 1 | `mcp:970f6c1e-…` `[実装] #333` | #333 / PR #560 | 2026-11-21T15:40:50Z | **真の孤立**（push 指示が `pending_thread_sends` に滞留・最古件） |
| 2 | `mcp:dab971b2-…` `[実装] #306` | #306 / PR #567 | 2026-11-21T14:59:36Z | 意図的保留（needs-human・経路 C の人間判断待ち）。レビュー r2 未回付の孤立面もあり |
| 3 | `mcp:28d8f688-…` `[実装] #480` | #480 / PR #482 | 2026-11-21T13:04:20Z | 意図的保留（競合解消を別途委託中・人間マージ待ち） |
| 4 | `mcp:3f022128-…` `[実装] #490` | #490 / PR #497 | 2026-11-21T07:18:41Z | 意図的保留（hatai の再承認待ち・ホールド指示済み） |
| 5 | `[実装] #569`（スナップショット範囲外） | #569 / PR #600 | — | needs-human ラベル付き。APPROVED・Draft のまま次工程（QA）の回付が行われていない孤立面もあり |
| 6 | `mcp:launch:caa0ff74…` `[実装] #602` | #602 / PR #610 | 2026-11-22T02:11:14Z | settle 負債（後継スレッド `mcp:launch:2e87e67b…` が running で継続中） |
| 7 | `mcp:launch:3bcaf0e3…` `[実装] #613` | #613 / PR #624 | 2026-11-21T16:33:25Z | **真の孤立**（REVIEW_REQUIRED・レビュー未回付） |
| 8 | `mcp:launch:9c9657f9…` `[実装] #615` | #615 / PR #625 | 2026-11-21T16:04:22Z | **真の孤立**（REVIEW_REQUIRED・レビュー未回付。needs-human も併存） |
| 9 | `mcp:5c599260-…` `[修正] #481` | #481 / PR #530 | 2026-11-21T15:14:24Z | **真の孤立**（CHANGES_REQUESTED の must-fix 指示が send 障害で未配送→旧指揮役が PR コメント issuecomment-7072879032 で代替配信） |
| 10 | `mcp:5fc0e3f5-…` `[実装] #295` | #295 / PR #463 | 2026-11-21T13:10:43Z | 意図的保留（CONFLICTING・#581 の設計判断待ち） |
| 11 | `mcp:16efc920-…` `[実装] #457` | #457 / PR #464 | 2026-11-21T05:45:42Z | needs-human。QA 経路（pelway-qa）待ちの孤立面もあり |
| 12 | `mcp:aec2522e-…` `[検討] #588` | #588 / PR なし | 2026-11-21T08:22:03Z | settle 負債（完了済み検討・PR なし） |
| 13 | `cb1a1122-…` `CI セキュリティチェックの検出事項の修正` | PR なし | — | settle 負債（人間起動・completed） |

内訳: **真の孤立（起こす手段が壊れていて作業が止まったもの）は最大 6 件**（1・2・7・8・9・11 のうち実際に送るべき指示があったもの）。意図的保留 4〜5 件、settle 負債 3 件。「13 件すべてが孤立」ではなく、3 種類の状態が `completed` 1 つのステータスに潰れていたことが把握を困難にした。

## 3. 根本原因（確認済み）

孤立は単一の故障ではなく、独立した 2 つの経路障害と 2 つの構造的弱点の重なりで起きた。

### 3.1 監視側の経路が死んでいた（一次原因）

`conductor-watch` は本来この孤立を検知する仕組みを持っている（Issue #504 で実装）。`scripts/conductor-watch.sh` は `[実装] #<N>` スレッドが `completed/failed/interrupted/cancelled` になり受信箱の状態行が `working` か無記録のままだと `STALL #<N>` イベントを出し `waiting_on=conductor` に倒す（同ファイル 284-297 行）。

しかし 2026-11-20 00:50 JST 以降、スレッド列挙が失敗し続けた:

- `t3 thread list --json` が `No running T3 Code server uses /home/hatai/.t3` で失敗（`patrol-state.json` の `watcher_blind` 記録、`recorded_at: 2026-11-21T01:50:00+09:00`）。
- 一方で T3 サーバー（pid 309767）は稼働中で、MCP 経由の `t3_thread_list` は正常だった。つまり **CLI 検出経路だけが壊れ、サーバー自体は生きていた**。

沈黙のしかたが致命的だった。`scripts/conductor-watch.sh` 54 行:

```sh
"$T3_BIN" thread list --json > "$D/threads.json" 2>/dev/null || true
```

- リダイレクトで `threads.json` が空に切り詰められ、stderr は捨てられ、失敗は `|| true` で握り潰される。
- Python 側（87 行）`try: threads = json.load(...) except Exception: threads = []` が「列挙失敗」と「スレッド 0 件」を区別できない。
- 結果、THREAD/STALL/停滞検知がすべて止まるのに、HEARTBEAT 行（10 分ごと）だけは出続け、**生きているように見える完全な盲状態**が約 25 時間続いた。`watcher_blind` の記録が翌日 01:50 なのは、巡回が手で気づくまで検知手段がなかったため。

### 3.2 起こす側の経路が死んでいた（二次原因）

- `pending_thread_sends` の最古件は 2026-11-22 02:45 JST（#333 実装スレッド宛の push ゲート指示）。`mcp_down.since: 2026-11-22T10:45+09:00` で send/schedule 系が全失敗として記録。
- 2026-11-22 11 時台 JST の現指揮役巡回でも `t3_thread_send` 再送が 2 回連続「Failed to connect to MCP server」。同じ時刻に `t3_thread_read`・`t3_thread_list`・`list_scheduled_tasks` 等の read 系は成功していた。**読み取り系と送信系で障害の振る舞いが違う**ことが確認済みの事実（原因の下層＝なぜ MCP の send ルートだけ落ちるかは未確定、T3 プラットフォーム側）。
- 滞留中の未配送 3 件: #333 宛 push 指示・#568 宛締切共有・PR #631 宛競合解消指示（#631 はその後マージ済みのため指示は陳腐化）。

### 3.3 構造的弱点 1: 「次の連絡」を追う帳簿が無い

`patrol-state.json` は Issue/PR ごとの `waiting_on` を追うが、「スレッドごとの次の連絡予定・最終接触時刻」は保持しない。そのため「completed だが open PR を持ち、waiting_on も conductor に倒れていない」スレッドは、監視が死ぬと誰にも数えられない。前日の棚卸しでも「未 settle 13 件」は数えられたが「明確な待ち」の有無だけ見ており、起こす経路の死活は確認対象に含まれていなかった。

### 3.4 構造的弱点 2: 指揮役交代と共有状態の競合

- 約 34 時間で指揮役が 4 代交代（Devin 8→9→10→11、主立った交代は 11-22 01:45・10:41・11:15 JST）。各交代で保留キューの引き継ぎとスケジュールタスクの作り直しが発生し、MCP 障害期と重なって送り損ないが累積した。
- さらに旧指揮役スレッド（`mcp:launch:61092c…`）が settle 後も 10 回以上自動再起動して巡回を続行し、`patrol-state.json` を上書きした（`watch_log_offset` が 507473 → 3796 に逆戻り、`last_patrol_note` が旧指揮役の巡回記録で上書きされたことを確認）。共有状態ファイルが複数の指揮役から同時に書かれ得る設計になっている。
- 現指揮役の巡回スケジュールタスク `scheduled-task:fe5a87db-…` も `enabled:false` で無効化されているのを確認（誰が/いつ無効化したかは未確認）。

## 4. 証拠一覧（一次記録）

| 主張 | 証拠 |
|---|---|
| 13 件の構成 | 指揮役スレッド run 6 の棚卸しメッセージ（`completed`・未 settle・トップレベルで 13 本と明記） |
| 監視の失明の期間と症状 | `patrol-state.json` `watcher_blind`（since 2026-11-20T00:50+09:00、`recorded_at` 11-21 01:50） |
| 失明のコード上の原因 | `scripts/conductor-watch.sh` 54 行の失敗握り潰し、87 行の `except: threads = []` |
| STALL 検知の存在と要件 | `scripts/conductor-watch.sh` 284-297 行（Issue #504 由来の実装スレッド停止検知） |
| send 障害 | `patrol-state.json` `mcp_down.since: 2026-11-22T10:45+09:00`、`pending_thread_sends` 3 件（最古 11-22 02:45）、本調査中の `t3_thread_send` 2 回連続失敗（read 系は成功） |
| 旧指揮役の自動再起動と共有状態の上書き | `conductor-watch/watch.log` の 61092c の running 遷移、`patrol-state.json` の offset 逆行 |
| 巡回の実値 | `docs/worklogs/2026-11-22-conductor-substitute-11.md`（PR 12・agent:working 9・needs-human 7） |

## 5. 最小の修正案

この調査の引き渡し先の Issue で直すべき範囲（実装はしない）:

1. `scripts/conductor-watch.sh` で `t3 thread list` の失敗を検知して `WATCHER thread-list-failed` 行を watch.log に出す（終了コードと stderr を保持、成功した最後のスナップショットは失敗で上書きしない）。
2. 同様に `pending_thread_sends` の最古件がしきい値を超えた滞留を知らせる手段を持つ（巡回プロンプトまたは監視側）。
3. 回帰試験: 列挙失敗をモックして「イベントが出る・前回スナップショットが残る・HEARTBEAT だけで見逃されない」ことを `scripts/tests/conductor-watch*.test.sh` の流儀で検証する。

T3 プラットフォーム側の send 障害そのもの（MCP の send ルートが読み取り系と別に落ちる件）はリポジトリで直せない。ホスト側の対応（T3 アプリ再起動等）は人間に依頼済み。

## 6. 未確定・残課題

- `t3 thread list` が `No running T3 Code server` を返しつつ MCP `t3_thread_list` が通る理由（CLI のサーバー検出方式の差）は未確定。`watcher_blind` 記録の推定どおりホスト側の検出不良が濃厚だが、T3 の内部は検証していない。
- `t3_thread_send` の障害が read 系と切り分けられる内部理由は未確定。
- 旧指揮役が settle 後に自動再起動したトリガー（残存スケジュールか、T3 側の再試行か）は未確定。当時のスケジュールタスク `a3fcb862` は削除済みだが、削除前後で挙動が続いた記録がある。
- 「13 件」のうち #569 の実装スレッド ID は監視スナップショットの範囲外で特定できていない（タイトル検索に非一致）。
