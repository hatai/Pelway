# Issue #651 の作業ログ

## 実装者メモ（round 1）

対象 head: 9352adb9c13cabb3ec897413ec507a3093f565d3

1. 詰まった箇所: `scripts/tests/conductor-watch.test.sh`: t3 モックが読む `threads.json` を作っておらず、既存試験の全巡回がスレッド一覧の取得失敗のまま走っていた。失敗中は conductor を implement・review に戻さない決まりを足すと 5 件が落ちたため、`mock_threads`（空の一覧）を最初に置いた。
2. 曖昧だった文: 「取得失敗のループではスレッド依存の判定（THREAD・STALL・stalled-45m・bad-title）を行わず、PR・ラベルの処理は継続される」（Issue #651「受け入れ条件」）。止まりで conductor にした対象をラベルから implement に戻すのがラベルの処理か、スレッド依存の判定かが読み取れなかった。
3. 仮定と解釈: 人間との事前の詰め（グリル、Q1〜Q10 すべて推奨案）で決めた。失敗中は前回 conductor でラベル由来が implement・review の対象だけ conductor を保つ。そのほかのラベル・PR 由来の判定は続ける。runbook には「見張りの行に基づくスレッドの判断を保留」と書き、巡回が MCP の `t3_thread_list` で点検する経路までは止めない解釈を選んだ。
4. 足りなかった情報・検証手段: 再通知の間隔を確かめる試験は実時間に頼る（間隔 5 秒・待ち 5.2 秒）。スクリプトに時計の差し替え口が無く、負荷の高い機械で 2 秒間隔にすると 13 回中 2 回揺らいだ。
5. 改善案: `scripts/conductor-watch.sh` の python 側に `PELWAY_WATCH_NOW`（テスト用の現在時刻）を足すと、HEARTBEAT・再通知・45 分停滞の試験を実時間に頼らず書ける。

使ったスキル: implement-issue、grilling、pstack poteto-mode（Feature の手順）

## 実装者メモ（round 2）

対象 head: 476619b7cefc768b60096ab62e091cfb728401e5

1. 詰まった箇所: Codex のレビュー委託が `verdict: failed` で、必須チェック未登録を理由にレビュー未投稿のまま must-fix 2 件だけ届いた。投稿前の指摘でも、実害（一覧の要素がスレッドの形でないと `t["threadId"]` で python が落ち、WATCHER 行も出ず巡回が止まる）を自分で確かめてから直した。
2. 曖昧だった文: 「それ以外は 1 回だけ再委託する（2 回連続で止まる条件）」（skill implement-issue の手順 9「レビュー結果を処理する」）。`verdict: failed` の報告に添えられた must-fix を、再委託の前に直してよいかが読み取れなかった。
3. 仮定と解釈: 指摘は GitHub のレビューとして投稿されていないが、自分で再現確認できる具体的な欠陥として扱い、再委託の前に直す解釈を選んだ。Claude 側の委託はプロバイダの週次上限で失敗したため「使えない担当」とみなし、回し先を Codex にした。devin に回す案は、Devin で実装を続けた PR に Devin のレビューを付けない決まりの意図に沿って退けた。
4. 足りなかった情報・検証手段: `mise run verify` が `.agents/skills/verify-pelway/verify-run.test.sh`（この環境では /tmp の所有者が uid 65534 で `dir_chain_safe` の所有者条件を満たせず常に失敗）と、本試験の「still-failing line after the interval」（実時間依存の既知の揺らぎ。単独では 3/3 回通過）で落ち、緑の verify が採れなかった。
5. 改善案: `docs/dev-environment.md` の環境の節に「WSL 環境によっては /tmp の所有者が uid 65534 で、`verify-run.test.sh` の `dir_chain_safe` 系のチェックが落ちる」と 1 行足すと、同じ失敗の切り分けが速い。再通知試験の時計差し替え（`PELWAY_WATCH_NOW`）は引き続き改善候補として残す。

使ったスキル: implement-issue

## 実装者メモ（round 3）

対象 head: dbb937f（このファイルの追加分を除く実装の head）

1. 詰まった箇所: 利用量の上限で failed した Codex の委託 2 件が、上限解除後に自動で再開されて両方レビュー（request-changes）を投稿した。停止・解放の手続き（`needs-human`・`agent:working` 解除・PR の watch 解除）を済ませた直後だったため、前提の消えた停止を Issue のコメントで明示して取り直した。
2. 曖昧だった文: 「巡回時刻を指定できるようにし、同じ間隔の直前・到達時・直後に巡回させて回数を確かめてください」（Codex の must-fix）。1 回の `run_cycles` 起動は複数巡回を連続で回すため、巡回ごとに時刻を変える口が必要かが読み取れなかった。
3. 仮定と解釈: `PELWAY_WATCH_NOW` を「epoch 秒を書いたファイルのパス」とし、python が巡回ごとに読み直す形にした。試験側は巡回の間にファイルを書き換えて時刻を進める。これにより再通知間隔の期限前・到達・直後だけでなく、45 分停滞の期限越えも実時間に頼らず置けると判断した。スレッドの形の検査は、後の処理が型を前提とする `title`・`updatedAt`・`status` を「無いか null か文字列」に限る厳格な側を選んだ（T3 の実出力は文字列で、型が壊れた一覧は取得失敗として通知した方が運用で分かる）。
4. 足りなかった情報・検証手段: STALL・stalled-45m の抑止を検証するには「成功時ならイベントが出る入力」が失敗時の応答に必要で、stalled-45m は 45 分の経過が要るため時計の差し替えが先決だった。`verify-run.test.sh` の /tmp 所有者の環境起因は引き続き残り、`git stash` で差分を外しても同じ失敗になることを確認済み。
5. 改善案: `docs/dev-environment.md` の環境の節に /tmp 所有者の件を書く改善案は round 2 と同じ（この PR では扱わない）。

使ったスキル: implement-issue
