# Issue #651 の作業ログ

## 実装者メモ（round 1）

対象 head: 9352adb9c13cabb3ec897413ec507a3093f565d3

1. 詰まった箇所: `scripts/tests/conductor-watch.test.sh`: t3 モックが読む `threads.json` を作っておらず、既存試験の全巡回がスレッド一覧の取得失敗のまま走っていた。失敗中は conductor を implement・review に戻さない決まりを足すと 5 件が落ちたため、`mock_threads`（空の一覧）を最初に置いた。
2. 曖昧だった文: 「取得失敗のループではスレッド依存の判定（THREAD・STALL・stalled-45m・bad-title）を行わず、PR・ラベルの処理は継続される」（Issue #651「受け入れ条件」）。止まりで conductor にした対象をラベルから implement に戻すのがラベルの処理か、スレッド依存の判定かが読み取れなかった。
3. 仮定と解釈: 人間との事前の詰め（グリル、Q1〜Q10 すべて推奨案）で決めた。失敗中は前回 conductor でラベル由来が implement・review の対象だけ conductor を保つ。そのほかのラベル・PR 由来の判定は続ける。runbook には「見張りの行に基づくスレッドの判断を保留」と書き、巡回が MCP の `t3_thread_list` で点検する経路までは止めない解釈を選んだ。
4. 足りなかった情報・検証手段: 再通知の間隔を確かめる試験は実時間に頼る（間隔 5 秒・待ち 5.2 秒）。スクリプトに時計の差し替え口が無く、負荷の高い機械で 2 秒間隔にすると 13 回中 2 回揺らいだ。
5. 改善案: `scripts/conductor-watch.sh` の python 側に `PELWAY_WATCH_NOW`（テスト用の現在時刻）を足すと、HEARTBEAT・再通知・45 分停滞の試験を実時間に頼らず書ける。

使ったスキル: implement-issue、grilling、pstack poteto-mode（Feature の手順）
