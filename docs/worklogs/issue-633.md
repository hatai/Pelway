# Issue #633 の作業ログ

## 未決事項の結論（`t3 thread show --json` に worktree 準備中を示す項目があるか）

項目は無いと判断した。確かめた範囲:

- `t3 thread show --help`: フラグ（`--base-dir`・`--messages`・`--json`）だけで、出力項目の記載は無い。
- 手元の T3 Code のソース（`~/.t3/source/dfd2187ccb0f7d16b0ff6d13f79eb5236ca9c46f`）: `t3 thread show --json` は MCP の `t3_thread_read` を呼び、`thread` を `OrchestratorMcpThreadDetail` で返す（`apps/server/src/cli/thread.ts`）。その項目は `threadId`・`status`・`runtimeMode`・`branch`・`worktreePath` などで、worktree を作っている途中を示す項目は無い。
- サーバー側には `WorktreeSetupSnapshot`（`phase: "running"` と段階の状態）があるが、WebSocket の `worktreeSetup` 通知向けで、`t3_thread_read` には出ない。CLI に `worktree` のサブコマンドも無い（`t3 --help`）。
- `ThreadLaunchService` は worktree を作り終えてから `thread.metadata.update` で `branch`・`worktreePath` を書くため、準備中は `worktreePath` が null のまま — worktree なしで立てたスレッドと見分けが付かない。`status`（`idle`/`preparing`/`starting`/...）は run の状態で、worktree なしのスレッドでも同じ値を通るため区別に使えない。

そのため時間切れの行は「worktree がまだ無い（準備中か、worktree なしで立てられた）」と両方を書き、T3 の画面で新指揮役の worktree を確かめる案内にした。

## 実装者メモ（round 1）

対象 head: 555aa264c41947857da25a1151760c94549e0120

1. 詰まった箇所: `docs/agents/conductor-context.md`: 「恒久交代」の「手順 2」の指す箇所。#632 で番号が 1 ずつ繰り上がり、受け入れ条件の「手順 2」は #632 前の番号では権限確かめの段（現行の項目 1）だった。待ちの記述は項目 1 に置き、導入文に lock のやり直しを足した。
2. 曖昧だった文: 「`docs/agents/conductor-context.md` の「恒久交代」の手順 2 に、worktree を上限付きで待つことが書かれている」（Issue #633「受け入れ条件」）
3. 仮定と解釈: 「手順 2」は #632 前の番号（権限確かめ）と解釈し、現行の項目 1 に書いた（worktree の確かめはそこにしか無いため）。もう 1 つの解釈（現行の項目 2＝停止文の段）は意味的に置き場が無い。「最大 3 回やり直し」は初回 + 3 回（計 4 回）と読んだ（「3 回のやり直しも失敗したら」とあるため）。時間切れの「待った秒数」は実際に待った秒数（`SECONDS` の差）を出し、上限も併記した。
4. 足りなかった情報・検証手段: `t3 thread show --json` の出力項目は CLI の `--help`・文書に無く、T3 Code のソース（`packages/contracts/src/orchestratorMcp.ts` の `OrchestratorMcpThreadDetail`）を読んで確かめた。`t3 --help` や CLI 文書に JSON の項目一覧があれば調べなくて済む。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「手順 1」の「文書の節（見出しを `grep`）」の行に「箇条書きの項目は番号（手順 N）ではなく見出し・節名で指す（番号は編集でずれる）」を足す。

使ったスキル: .agents/skills/implement-issue、.agents/skills/diagnosing-bugs（`verify-pelway/verify-run.test.sh` の失敗の切り分けに Phase 2。`scripts/red-check origin/main` で同じ失敗を再現し、環境起因と確かめた。この環境は `/tmp`・`/home` が nobody:nogroup の所有で、`dir_chain_safe` の「置き場より上は自分か root の持ち物」に反するため、書き込める場所の階層では必ず落ちる）。`mise run verify` は `script-test` 1/55（この試験のみ）で落ちたが、ほかの全段は通った。

## 実装者メモ（round 2）

対象 head: 5f4c96059214e4deb05cfc08845affc83ba3be29
PR: #735

レビュー指摘（run 7180293697 codex request-changes、head 12fee3a33b43095e2f9c42d3febb30159685cab1）への対応:

1. 詰まった箇所: なし
2. must-fix の対応:
   - `scripts/conductor-rebuild`:133 期限を超えて返った `show` の結果にパスがあっても処理を進めていた → ループの先頭で `SECONDS > end` なら `worktreePath` を空に戻して待ちを抜ける形にした（読み取りの返りが期限をまたぐのは、間隔 `WT_INTERVAL` の超過と `show` 内の lock のやり直しの両方でありうる）。`runtimeMode` は進む・止まるの判定を変えないため、最後に読んだ値のままログに出す。試験を 2 件追加（`worktree after the limit`: 間隔 3 秒で 2 回目の読み取りにパスが入っても期限超過で止まる、`lock retry past the limit`: lock のやり直しが期限をまたいだ読み取りを受け入れない）
3. nit の対応: なし（nit 0 件）。`[文書のずれ]` の印も無し

## 実装者メモ（round 3）

対象 head: 4c8e332d23a82a816f99dee74e9be74af63216ec
PR: #735

レビュー指摘（run 7180639512 codex request-changes、head b3ee1180e5913022c4684d17b4ed29615bf35b95）への対応:

1. 詰まった箇所: なし
2. must-fix の対応:
   - `scripts/conductor-rebuild`:138 `SECONDS` は整数秒なので、実際の経過が上限を超えても `SECONDS == end` の間は読み取りを受け入れていた → 期限の判定を `EPOCHREALTIME`（秒.マイクロ秒を整数化）に替え、上限・開始・待った秒数の計算をすべてマイクロ秒に揃えた。上限は awk で小数の `WT_WAIT` も扱える計算にした。試験を 1 件追加（`worktree just past the limit`: 上限 1 秒・間隔 1.1 秒で、整数秒では `SECONDS == end` に見えるが実経過は超過している読み取りを受け入れない）。既存の「worktree after the limit」「lock retry past the limit」の待ち時間も短くし、全体を約 7 秒に保った
3. nit の対応: なし（nit 0 件）。`[文書のずれ]` の印も無し

## 実装者メモ（round 4）

対象 head: 8405822e40014bc37de284c00e57500c39dd8de2
PR: #735

レビュー指摘（run 7181000574 codex request-changes、head 305d5bf894ab190cb73d9e63ff39f5422db54406）への対応。round 3 の結果に must-fix が残ったため一度止まったが、hatai の例外判断（レビューのラウンド上限なし）により round 4 として修正に入った。

1. 詰まった箇所: 読み取りが期限で打ち切られたとき `runtimeMode` が空のまま権限の確かめに進み、時間切れではなく権限不整合（exit 3）で止まる状態がテストで見つかった → 期限での打ち切り（`aborted`）では権限の確かめを省き、時間切れの報告に進む形にした。
2. must-fix の対応:
   - `scripts/conductor-rebuild`:134 `EPOCHREALTIME` はロケールの小数点を使い、コンマ小数点の環境では整数化に使えず期限を検出できなかった → 経過時間を `/usr/bin/python3` の `time.monotonic()`（ロケールに依存しない単調時計。JSON の読み取りに python3 は既に前提）に替え、`now_us()` でマイクロ秒を返す形に統一した。
   - `scripts/conductor-rebuild`:137 期限の確認より先に `show` の終了を待つため、待機そのものに上限が無かった（期限を超えて読み取り・やり直しを始め、間隔・やり直しの待ちが残り時間をはみ出した）→ 期限を過ぎたら次の読み取りを始めず（1 回目は必ず読む）、読み直しの間隔と `show` 内のやり直しの待ちを `sleep_min_us` で残り時間に切り詰め、`SHOW_DEADLINE_US` で `show` のやり直しを期限で打ち切る形にした。
3. nit の対応:
   - `[文書のずれ]` `scripts/conductor-rebuild`:105 t3 が終了コード 0 で JSON にならない出力に lock の文字を含む場合もやり直していた → やり直すのは t3 コマンド自体の失敗に限定（t3 成功・JSON 壊れはやり直さず止まる）。
4. 仮定と解釈（同じ種類の must-fix が続いたため、その種類の全経路に当てる決まり）: 「worktree 待ちの上限」に関わる経過時間の確かめは全て、期限を超える処理（読み取り・やり直し・間隔の待ち）を始めないか残り時間に切り詰め、期限後に返った結果は判定に使わない。経過時間はロケールに依存しない単調時計で測る。この決まりの及ぶ範囲は worktree 待ちのループと `show` のやり直し（`thread send --wait` の待ちや旧指揮役の読み取りには上限の意味が無いため含まない）。
5. 足りなかった情報・検証手段: この環境にはコンマ小数点のロケールが無く（`locale -a` が C/POSIX 系のみ）、実ロケールでの確認はできなかった。試験は対象ロケールがある環境でのみ走る形にした（機構上は書式付き時刻の解析を一切していないためロケールに依存しない）。`date +%s%N`（壁時計）や `/proc/uptime`（カーネル稼働時間）も候補だったが、レビュー指摘の「単調増加時計」に素直に合う `time.monotonic()` を採った。

試験の更新・追加: 「interval past the limit」（間隔が残り時間を超えるとき期限後の読み取りを始めない。読み取り 1 回）、「read finishes past the limit」（応答が期限後に返る読み取りを受け入れない。偽 t3 の `NEW_SHOW_DELAY`）、「lock retry past the limit」（やり直しの待ちを残り時間に収めて期限で打ち切る。読み取り 1 回）、「corrupt output」（t3 成功・壊れた JSON ではやり直さない。偽 t3 の `NEW_CORRUPT`）、コンマ小数点ロケールの条件付き試験。`scripts/red-check` で前の実装（305d5bf）だと期限後の読み取り開始・期限を超えるやり直し・壊れた JSON でのやり直しの 3 件が落ちることを確かめた。

## 実装者メモ（round 5）

対象 head: 73d300a3645a2ece3e3a2809e2713312802efdbd
PR: #735

レビュー指摘（run 7183286832 codex request-changes、head f51c65991743e99e0a6e7858bcc108286bc397e9。併せて run 7183287312 claude approve・`[文書のずれ]` の印付き nit 1 件）への対応。判定が割れたため request-changes 側を優先した。

1. 詰まった箇所: なし
2. must-fix の対応:
   - `scripts/conductor-rebuild`:125（前回指摘の残り）`show` の中の `t3` の呼び出し自体に上限が無く、応答が期限より遅い・固まると待機が 120 秒を超え、時間切れの案内も出なかった → `SHOW_DEADLINE_US` がある呼び出しは `timeout` に残り時間を秒で渡して包み（初回も再試行も）、打ち切られた読み取りは失敗として期限を確かめ、時間切れを報告する経路に進む。初回の読み取りと再試行の読み取りのそれぞれで、期限に戻ること・時間切れを報告して送信・登録変更をしないことを試験した（偽 t3 に `NEW_SHOW_DELAY_FROM` を足し、やり直し側の呼び出しだけ遅らせられるようにした）。戻り時刻の判定は `SECONDS` の整数秒で、打ち切り 0.6 秒・遅延 4 秒の間を確かめられる幅にした
   - `scripts/conductor-rebuild`:172（security-review の HIGH も同項）`aborted` が未初期化のため、起動環境に同名の値があると正常に読み取れた経路でも権限の確かめが飛ばされ、権限の合わないスレッドに引き継ぎの指示が届きうる → 待ちループの前に `aborted=` を初期化。注入の回帰試験を追加（`aborted=1` の環境で full-access は拒否・送信なし・登録不変、通常権限では普通に進む）。時間切れの分岐は送信に届く前に必ず止まる形を維持
3. nit の対応:
   - `[文書のずれ]`（claude 側、`docs/agents/conductor-context.md`:120）「待ちそのものも 120 秒に収まる」は、実行中の読み取りに打ち切りが無い実装より強い言い方だった → 読み取りの呼び出し自体を残り時間で打ち切る実装にしたため文書の言い方は実装どおりになった。文書には「実行中の読み取りも残り時間で打ち切る」旨を足した（指摘の提案どおり、書き直すより実装側を合わせた）
   - 印なし nit（codex のロケール名の完全一致、claude の 2 件）は決まりどおり直さない
4. 足りなかった情報・検証手段: なし。打ち切りは `timeout`（coreutils）の TERM で、t3 の `thread show` は読み取りだけなので途中で切っても副作用が無い。打ち切られた呼び出しは lock の文字を出さないため、やり直しの対象にはならずそのまま期限の判定に進む

試験の更新・追加: 「read finishes past the limit」を「read killed at the limit」に替え（打ち切り・戻り時刻・1 回のみの読み取りを確かめる）、「retry read killed at the limit」（1 回目は lock で即失敗・2 回目を遅延で期限にかける）、「ambient aborted」（同名の環境変数でも権限の確かめを必ず通る）を追加。`scripts/red-check f51c659 --keep scripts/tests/conductor-rebuild.test.sh` で前の実装だと 3 グループが落ちることを確かめた

## 実装者メモ（round 6）

対象 head: 6ee3d244b945382bd2eec5a0fc6c6196bcba295b
PR: #735

レビュー指摘（run 7183693439 codex request-changes・run 7183693916 claude request-changes、ともに head f700de548596411eb0bed4e776154f1e52fcf3b0、同一の must-fix 1 件）への対応。

1. 詰まった箇所: なし
2. must-fix の対応:
   - `scripts/conductor-rebuild`:128 残り時間が 0 以下になると `awk` が 0 を返し、GNU `timeout` は期間 0 を「打ち切り無効」と扱うため、まさに打ち切るべき読み取り（残りが尽きた読み取り）だけが上限なしで走っていた。2 経路: `PELWAY_REBUILD_WT_WAIT=0` では 1 回目の読み取りが必ず残り 0 以下、既定でもループ先頭（旧 168 行）や `show` 内の期限確認から残りを測るまでの数十 ms の窓で期限を迎える → `show` の中で残り時間をイテレーションごとに 1 回だけ計測し（確認と呼び出しの窓を閉じる）、残り 0 以下では 2 回目以降のやり直し・呼び出し側の読み直し（`SHOW_REREAD=1` で来る）は呼び出さず失敗を返す。1 回目の読み取りは必ず読む仕様を残し、`PELWAY_REBUILD_SHOW_FIRST_TIMEOUT`（既定 2 秒）の正の下限を `timeout` に渡す形にした（`max(残り, 下限)` ではなく残りが 0 以下のときだけ下限を使い、残りが正なら残りどおりに打ち切る）
3. nit の対応: なし（codex 0 件。claude 2 件は印なし — `timeout -k` と試験経路の欠落 — 決まりどおり直さない）
4. 仮定と解釈: 「1 回目は必ず読む」の下限は、既定 120 秒の待機に対して無視できる大きさで、実際の `t3 thread show`（ローカルの読み取り）に足りる 2 秒とし、試験から縮められるよう環境変数にした。下限で打ち切られた読み取りは lock を出さない失敗として扱われ、時間切れの経路に進む（`WT_WAIT=0` かつ応答なしでも時間切れの案内が出る）

試験の更新・追加: 「first read at zero limit」（上限 0・遅延 4 秒・下限 1 秒で、読み取りが下限で打ち切られ時間切れを報告して戻る。読み取り 1 回・送信なし・登録不変）、「reread killed near the limit」（期限直前に始まった読み直しも残り時間で打ち切る。読み取り 2 回）。`scripts/red-check f700de5 --keep scripts/tests/conductor-rebuild.test.sh` で前の実装だと「first read at zero limit」が落ちることを確かめた（「reread killed」は旧実装でも打ち切りが効く経路のため回帰固定として残る）

## 実装者メモ（round 7）

対象 head: 7804757d032a36eb79870e5ad3a1e7c3cd1ee6d9
PR: #735

レビュー指摘（run 7184068054 codex request-changes・must-fix 2、run 7184068644 claude approve・印付き nit 1 件。ともに head 6f1f666ca2fa873624f73cfdd3a2581d99751eb7。両者の印付き nit は同一項目）への対応。判定が割れたため request-changes 側を優先した。

1. 詰まった箇所: なし
2. must-fix の対応:
   - `scripts/conductor-rebuild`:135 (a) `PELWAY_REBUILD_SHOW_FIRST_TIMEOUT=0` を指定すると `timeout` の期間 0（打ち切り無効）に戻る → 起動時に正の有限値か検査し、0・負・非数・inf は読み取りの前に拒否（`awk` で `v+0 > 0 && v+0 < 1e308` 以外を die）。(b) `timeout` は TERM しか送らず、TERM を無視する t3 なら戻らない → 全ての打ち切り付き読み取りに `timeout -k 1`（TERM の 1 秒後に KILL）を付けた
   - `scripts/conductor-rebuild`:171 `end_us` の算出を `awk` の `%d` で出しており、mawk では符号付き 32 ビットに頭打ち（稼働約 36 分で時計が上限を超え、期限内の結果をすべて拒否する）→ `%.0f` で出力。試験から時計をずらせるよう `PELWAY_REBUILD_NOW_US_OFFSET`（μs の加算、試験用）を `now_us` に足した
3. nit の対応:
   - `[文書のずれ]`（両者同一、`scripts/conductor-rebuild`:22）環境変数一覧に `PELWAY_REBUILD_SHOW_FIRST_TIMEOUT` が無い → 一覧に追加（併せて新設の `PELWAY_REBUILD_NOW_US_OFFSET` も記載）
   - 印なし nit（claude の 1 件 — 1 回目読み取りの下限による最大待ちの非単調性）は、待ちが 120 秒を超えないための下限であり意図どおりのため直さない
4. 仮定と解釈: `timeout -k` の猶予は 1 秒とし、待ちの上限は「120 秒＋猶予」と文書に書いた（codex の指摘どおり 1 回目の読み取りの例外も併記）。KILL 猶予の経路を試験するため偽 t3 に `NEW_TERM_IGNORE`（TERM を無視する）を足した。`%.0f` は gawk では `%d` と同じ結果になるためこのホストでは差が出ないが、mawk 環境での 32 ビット頭打ちへの移植性の修正として入れた（codex の must-fix どおり）

試験の更新・追加: 「zero first timeout」（下限 0 を拒否。読み取り・送信なし・登録不変）、「term-ignoring read killed」（TERM を無視する偽 t3 でも KILL 猶予で期限に戻る。戻り時刻を `SECONDS` で検査）、「clock past int32」（時計を +3×10^9 μs ずらしても期限内の worktree を受理して進む）。`scripts/red-check 6f1f666 --keep scripts/tests/conductor-rebuild.test.sh` で前の実装だと「zero first timeout」4 件と「term-ignoring read killed: returns after the grace」が落ちることを確かめた（「clock past int32」はこのホストの awk が gawk のため落ちない回帰固定。「first read at zero limit: returns at the floor」は旧実装に `-k` が無く TERM 到達のタイミング次第で遅れる経路が残るため、稀に落ちる。新実装では KILL で打ち切られるため確定的）

## 実装者メモ（round 8・印の nit だけを直す round。通算 1 回目）

対象 head: dd60f92f6cbfb59ab8ca0bb635ece0c7a25a93c0
PR: #735

レビュー指摘（run 7184499800 codex approve・`[文書のずれ]` の印付き nit 1 件、run 7184500403 claude approve・印なし nit 1 件。ともに head 6608d3056c36b977eea17a3b66a72166c28c742c）への対応。両者とも approve のため、印の nit だけを直す round（修正回数の上限に数えない。この PR では通算 1 回目）として入った。

1. 詰まった箇所: なし
2. 印付き nit の対応:
   - `[文書のずれ]`（codex、`scripts/conductor-rebuild`:105）初回の上限は秒数と書いているが、`awk` の `v+0` 変換は文字列の先頭だけで成功するため `1d` が 1 として検査を通り、そのまま `timeout` に渡ると 1 日の期間になる → 接尾辞の付く値を落とせるよう、`[[ =~ ^[0-9]+(\.[0-9]+)?$ ]]` で入力全体を数字だけの形に検査してから `awk` で正の値か確かめる 2 段にした（接尾辞・符号・指数・inf をすべて拒否。`v+0` だけでは接尾辞を落とせない）
3. nit の対応:
   - 印なし nit（codex の `PELWAY_REBUILD_NOW_US_OFFSET` を算術式に入れる前に整数に限定、claude の上限値の例。security-review の `[入力の検査]` の所見も `[文書のずれ]` でないため対象外）は決まりどおり直さない
4. 仮定と解釈: 受理する書式は「数字だけの十進の秒」（`1`・`0.5` など。`.5` や `1.` は形に合わず拒否）とした。`timeout` が受理する `1e3` や `inf` も形に合わないため拒否するが、秒数としての意味は検査と消費側でそろう形に限定する指摘の意図どおり

試験の更新・追加: 「first timeout <値>」（`1d`・`5m`・`abc` を拒否。読み取り・送信なし・登録不変。計 167 件 pass）。`scripts/red-check 6608d30 --keep scripts/tests/conductor-rebuild.test.sh` で前の実装だと `1d`・`5m` の 8 件が落ちることを確かめた（`abc` は旧実装でも `v+0` が 0 になり拒否されるため落ちない。「term-ignoring read killed: returns after the grace」は旧実装に `-k` が無く TERM が効かない読み取りが稀に遅れる経路が残るため、稀に落ちる）

## 実装者メモ（round 9）

対象 head: da926bb76755fa3d73aefb3fa98ddd3657f3fd18
PR: #735

レビュー指摘（run 7184768069 codex request-changes・must-fix 1、run 7184768661 claude approve・印なし nit 1 件。ともに head 0ec024de3094b36656462b3015e3897182d1f3da）への対応。判定が割れたため request-changes 側を優先した。

1. 詰まった箇所: なし
2. must-fix の対応:
   - `scripts/conductor-rebuild`:108 round 8 で `v+0 < 1e308` の上限を落としたため、309 桁以上の数字列が awk で `inf` となり `v+0 > 0` を通り、そのまま `timeout` に渡ると GNU timeout は範囲外の期間を最大値として扱う（実質打ち切りなし）→ 下限に実用的な上限 60 秒を足した（`v+0 > 0 && v+0 <= 60`。1 回のローカルの読み取りの猶予として既定 2 秒の 30 倍の幅。書式の検査はそのまま維持）。試験に 400 桁の数字列と 61 を追加（codex の指摘どおり数値の範囲を超える十進入力を T3 の呼び出し前に拒否することを固定）
3. nit の対応: なし（codex 0 件。claude の 1 件は同じ指摘を含むが印なし — `[文書のずれ]` でないため対象外）
4. 仮定と解釈: 「実用的な上限」は 60 秒とした。下限は残りが尽きたあとの 1 回目の読み取りを締める猶予で、既定 2 秒に対し十分な幅。これ以上を指定する運用は意図が無いため、運用の値ではなく異常系として起動時に拒否する形にした

試験の更新・追加: 「first timeout <400-digit>」「first timeout 61」（ともに拒否。読み取り・送信なし・登録不変。計 175 件 pass）。`scripts/red-check 0ec024d --keep scripts/tests/conductor-rebuild.test.sh` で前の実装だとこの 2 グループ 8 件が落ちることを確かめた

push 後の CI で既存の「no worktree until the limit: logs the last-read values」が落ちた（red-check でも稀に見えていた境目の競合）: 期限の境目で最後の読み取りが打ち切られると権限は読めずログが `runtimeMode=-` になるが、試験は `auto-accept-edits` に固定していた。読み取りの所要時間は環境差があるため、境目のどちら側に落ちるかは決まらない。`runtimeMode=(auto-accept-edits|-)` のどちらでも受ける形に直した（値は読み取りの結果か - だけで、欠落や別の値は引き続き落ちる）

## 実装者メモ（round 10）

対象 head: e705d4dc1c27de6daec3992bfdcf776545985e9a
PR: #735

レビュー指摘（run 7185187523 codex request-changes・must-fix 1、run 7185188739 claude request-changes・must-fix 1。ともに head 4ab57fba4860afccbf222c3fc3e30b7c184d0380、同一趣旨の指摘）への対応。

1. 詰まった箇所: なし
2. must-fix の対応:
   - `scripts/tests/conductor-rebuild.test.sh`:177（両者同一趣旨）round 9 の CI 失敗の直しで「logs the last-read values」の期待値を固定文字列から `(auto-accept-edits|-)` に広げたため、(a) 最後の読み取りが成功したのに権限を消す回帰を検出できない（codex）、(b) 期待値を広げる「確かめ方を弱める変更」が PR 本文に指定の見出しなし（claude）→ 権限の値を決定的な条件で厳密に確かめる 2 件を追加: 「interval past the limit」（1 回の読み取りが期限より十分前に即応答で終わる → `runtimeMode=auto-accept-edits worktree=-` と厳密比較）、「first read at zero limit」（遅延 4 秒・下限 1 秒で必ず打ち切られる → `runtimeMode=- worktree=-` と厳密比較）。境目のシナリオ側のチェックは値が読み取り結果か - のどちらかだけを受ける形のまま残し、理由と厳密ピンの所在をコメントに書いた。PR 本文の「変更内容」に見出し「確かめ方を弱める変更」を立て、ファイルと行・何をしたか・理由（境目のどちら側になるかは読み取りの所要時間で決まる）と、厳密なピンは 2 件の決定的な試験に残ることを記載した
3. nit の対応: なし（両者 0 件）
4. 仮定と解釈: 「読み取り成功の厳密ピン」には「interval past the limit」を選んだ（1 回の読み取りが残り 0.6 秒で即応答し、以後の読み直しを始めず期限の先で止まる条件のため決定的。「worktree arrives」のピンとは別に、時間切れの経路で読み取れた権限が出ることを確かめる）。境目のシナリオの緩和は codex の案 (c)（緩和を残して理由を記載）を取った — 境目のどちら側になるかを決定的に作る手段（読み取りの所要時間を時計から切り離す仕組み）がなく、固定に戻すと CI が再び不安定になるため

試験の更新・追加: 「interval past the limit: logs the last-read values」「first read at zero limit: logs the killed-read values」を追加（計 177 件 pass）

push 後のローカル verify で `term-ignoring read killed: returns after the grace` が 1 度落ちた（cargo test との並走負荷で、打ち切り経路の戻り ~1.6 秒に対し上限 4 秒がタイトすぎた）→ 5 件の時間チェック（read/retry read/first read/reread killed、term-ignoring）の偽の遅延を 8 秒・戻り上限を 6 秒に広げた。打ち切り経路は遅延の完了を待たないため試験時間は増えず、打ち切られない経路（8 秒超）との区別は保たれる
