# Issue #555 の作業ログ

## 実装者メモ（round 1）

対象 head: 65a6532d09e85f2f541a5245b0ae3817f31040a1

1. 詰まった箇所: `crates/pelway-agent-watch/tests/once.rs` の Sandbox: 偽の git が `--git-common-dir` に相対の `.git` を返すため、そのままでは送信ありの既存の試験が crate の下に受信箱を作る。既定を sandbox の中の絶対パス（`gitcommon`）にした。起こしの `t3` は、この PC で動いている本物の T3 サーバーを呼ばないよう、試験では常に `T3_BIN` を偽物に向けた。
2. 曖昧だった文: 「`patrol-state.json` が読めない（無い・壊れている）ときは、その回は起こしを送らず、受信箱への追記と webhook は行う」（Issue #555 の受け入れ条件）— 「無い」は、Grok Bot が指揮役のときの普通の状態なので、ログを出さずに起こさない扱いにした。「壊れている」（書きかけを含む）は `WAKE skipped reason=patrol-state-unreadable` を 1 行出す。
3. 仮定と解釈:
   - `conductor_thread_id` は sandbox から書けるファイルの値で、ホスト側のコマンドの引数になる。そこで、先頭の `-`、英数字と `:`・`_`・`-`・`.`・`%` 以外の文字、256 文字を超える長さを拒むことにした（Issue に無い検査を足した）。
   - 受信箱への追記は、知らせを作った時点の 1 回だけにした。送信待ちの列の再送では書かない。
4. 足りなかった情報・検証手段:
   - 本物の `t3 thread send` で、動いている指揮役が次のターンとして起きることは、この環境では確かめていない（試験は偽の `t3`）。
   - 差分は `.cargo/mutants.toml` の `examine_globs` の外なので、`mise run mutants:diff` は回していない。
5. 改善案: `docs/agents/grokbot-runbook.md` の「指揮役の定期巡回（スケジュール起床）」の「`watch.log` からスレッドへの転送スクリプト（…）は使わない」の項に、#555 の起こし（サーバーと同じ `t3`、毎回読み直す宛先、決まった文）は別扱いであることを足す（#526 の PR #559 で直す）。

使ったスキル: implement-issue、write-pull-request

## 実装者メモ（round 2）

対象 head: b2637cc0bb9256f5d359b5a349e9e89497612a99

1. 詰まった箇所: `crates/pelway-agent-watch/src/io.rs`: round 1 は、共有の git dir の `pelway/`（sandbox から書ける）のファイルと、sandbox から書ける場所の実行ファイルを、ホストの権限で確かめずに使っていた（Claude の must-fix 2 件）。直し方は次のとおり。
   - `pelway/` を `O_DIRECTORY | O_NOFOLLOW` で開き、fd で種類・所有者を確かめる。
   - 中のファイルを `openat` と `O_NOFOLLOW | O_NONBLOCK` で開き、通常ファイル・所有者・リンク数 1・大きさの上限を確かめる。
   - `t3` は `$HOME/.t3/wsl-runtime/` の下の自分の所有のものだけを受け入れる。
2. 曖昧だった文: 「`mise run pr-size` はコミット済みの差分だけを数える。必ずコミットの後、push の前に実行する。」（`.agents/skills/implement-issue/SKILL.md` 手順 6）— 文は明確で、修正の round で守らずに push した（+880 で上限超過）。hatai が「PR サイズは無視しておｋ」と判断したので分割せず、`pr-size-exception` の付与は hatai に頼む。
3. 仮定と解釈: パスで確かめてからパスで開く間の差し替え（自動のセキュリティレビューの指摘）は、決まった順で起こす試験が作れないため、作り（dirfd と `openat`）で塞ぎ、`pelway/` が symlink のときに外へ書かないことは既存の試験で確かめる形にした。nit 3 件（ゾンビ、受信箱の全体の読み直し、残る範囲の書き方）は直していない。
4. 足りなかった情報・検証手段: bubblewrap の中の常駐プロセスの `/proc/<pid>/exe` がホストからどう見えるかは、未確認。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 9「Request changes のとき」の 2 に「手順 6 の `mise run pr-size` を push の前に必ず実行する（修正の round でも省かない）」を足す。

使ったスキル: implement-issue、write-pull-request

## 実装者メモ（round 3）

対象 head: 939b7b5901ec7627518e9591d6603244a5554ef1

1. 詰まった箇所: round 2 のレビュー（Codex・Claude）の must-fix 3 件のうち 2 件は同じ指摘で、受信箱の全体の読み直しに上限が無いことだった。巨大な sparse ファイルを置かれると、webhook の送信の前で止まる。256 MiB の上限で直し、1 TiB の sparse ファイルの試験を足した。残る 1 件は、sandbox から書ける `patrol-state.json` の `conductor_thread_id` で宛先を付け替えられることだった（Codex）。
2. 曖昧だった文: 「Authenticate the registration or read it from a location sandboxed agents cannot modify.」（PR #561 round 2 の Codex のレビュー）— 指揮役のスレッドは `createdBy: agent` で立っていることもあり、スレッドの属性では認証できない。そこで hatai に選んでもらい、後者（sandbox から書けない場所から読む）にした。
3. 仮定と解釈: 登録ファイルは `~/.config/pelway-agent-notify/conductor-thread-id` にし、hatai がホストで `scripts/conductor-register` を実行して書く（hatai の判断）。`patrol-state.json` には既読の位置だけを残す。マージ済みの #526 の手順（指揮役が `patrol-state.json` に書く）は、この PR で直した。
4. 足りなかった情報・検証手段: pr-size は +1109 になり、hatai が Issue のコメントで認めた +922 を超えた。
5. 改善案: `.agents/skills/command-team/SKILL.md` の「マージ」の pr-size の例外に、「例外を認めた後に修正 round で行数が増えたときは、指揮役か hatai がコメントで改めて認める」を足す。

使ったスキル: implement-issue、write-pull-request

## 実装者メモ（round 4）

対象 head: 1f1da91d59dfc08ae75bcc294ae039b64c5ee2b2

1. 詰まった箇所: round 3 のレビュー（Codex `gpt-6.2-sol` max）に must-fix 4 件が残った。hatai の判断で、4 回目の修正として直した（Issue #555 のコメント）。直したことは次の 4 つ。
   - 指揮役自身の知らせだけの回は起こさない（own-event）。
   - 補助記録が無いときは起こさない。
   - 改行の無い断片を、追記の前に閉じる。
   - webhook の失敗の試験を、偽の秘密と HTTP 500 の偽の curl で強めた。
   Claude の round 3 は head 1f1da91 を見て、CI の `test (cargo test)` の失敗を must-fix にした。ログ（`PR_DIGEST_LOG_LINES=3000 pr-digest 561 --logs 136409713448`）では、落ちたのは `scripts/demo-check/selftest.sh` の「loopback: allowed+bad bind (exit 124, reports 0)」（時間切れ）で、この PR の変更と関係が無い。前の head と main では同じジョブが通っていて、手元でも通る。
2. 曖昧だった文: 「一時的な失敗なら、再実行で pass を確かめる」（PR #561 round 3 の Claude のレビュー）— 実装担当の App には run の再実行の権限が無い（`run rerun` は「Resource not accessible by integration」）。そこで、このメモのコミットの push で CI を走らせ直す。
3. 仮定と解釈: Claude の nit 9 件は直していない（implement-issue の「nit は直さない」）。ただし、PR 本文が round 3 より前の設計のまま、という指摘は事実の食い違いなので、本文を今の設計（宛先はホストの登録ファイル、補助記録は既読の位置）に書き直した。
4. 足りなかった情報・検証手段: `scripts/demo-check/selftest.sh` の時間切れの原因（CI の負荷か、試験の待ち時間の短さか）は調べていない。
5. 改善案: `.agents/skills/pr-watch/SKILL.md` の「`--logs`」の説明に「出すのは末尾の `PR_DIGEST_LOG_LINES`（既定 80）行だけ。落ちた試験が前の方にあるときは値を大きくする」を足す。

使ったスキル: implement-issue、pr-watch（`.agents/skills/pr-watch/SKILL.md`）

## 実装者メモ（round 5）

対象 head: fd5d4cf5（実装のコミット）

1. 詰まった箇所: round 4 のレビュー（Codex `gpt-6.2-sol` max が must-fix 7、Claude `claude-opus-5-5` max が must-fix 3）で、次の 2 つが大きかった。
   - 一覧・詳細を PATH の `t3` で取っていた。版の食い違いで一覧が失敗すると、受信箱・起こし・webhook まで進まない。一覧・詳細・起こしで同じ `t3` を使うようにした。
   - `scripts/conductor-register` を、hatai がホストで worktree の版のまま実行しうる。sandbox の中の指揮役が書き換えた版がホストで動く。インストールした `pelway-agent-watch` に `--register`・`--unregister`・`--show-registration` を持たせ、スクリプトは消した。
2. 曖昧だった文: 「hatai が Issue #555 に、宛先をホストの登録ファイルにすること・`scripts/conductor-register` を足すこと・代行の手順を変えることを認めるコメントを書く」（PR #561 round 4 の Claude のレビュー）— 登録の手段をスクリプトからインストールした見張り役に変えるので、コメントはその形で書いてもらった（Issue #555 の hatai のコメント）。
3. 仮定と解釈: UTF-8 の途中で切れた断片の問題は、書き手では断片を消せない（受信箱は追記だけ）ので、読み手の `scripts/conductor-watch.sh` を直した。LF で分けて 1 行ずつ UTF-8 と JSON として読み、壊れた行の後ろも読む。あわせて、種類・所有者を確かめて開き、末尾の 4 MiB だけを読む（Codex の security-review の「読み手にファイル種別・サイズの検査が無い」）。#504 は main にマージ済みなので、この PR で直した。`timeout -k` で子が止まらない場合の試験は、待つ時間が 35 秒を超えるので書いていない。
4. 足りなかった情報・検証手段: pr-size は +1561 になった（hatai は「PR サイズは無視しておｋ」と判断）。本物の T3 で起こしが届くこと、bubblewrap の中のプロセスの `/proc/<pid>/exe` の見え方は未確認。Claude が PR 外として挙げた「pitchfork が worktree の `pitchfork.toml` の定義を起動・cron で動かすなら、sandbox の中のエージェントがホストでコマンドを動かせる」は、この PR では確かめていない。
5. 改善案: `.agents/skills/command-team/SKILL.md` の代行の節に「ホストで hatai に実行を頼むものは、インストールしたバイナリか本体 checkout の版に限る（worktree の版は sandbox の中から書き換えられる）」を足す。

使ったスキル: implement-issue、write-pull-request

## 実装者メモ（round 6）

対象 head: b6f4819e（実装のコミット）

1. 詰まった箇所: round 5 のレビューで、Codex（`gpt-6.2-sol` max）が must-fix 6、Claude（`claude-opus-5-5` max）が must-fix 1 を出した。うち 3 件は `scripts/conductor-watch.sh` の読み手への指摘（Codex）で、Claude はその読み手の変更自体を Issue #555 の対象外とした。hatai の判断で、読み手の変更はこの PR から外し、#569 に分けた。この PR で直したのは次の 3 つ。
   - 所有者以外の権限があった受信箱には書かない（`fchmod` は、すでに開かれた fd の読み取りを取り消さないため）。
   - 起こしたときの指揮役の既読の位置を持ち、そこから 1 行でも進めば次の知らせで起こす。
   - 受信箱への追記が失敗する試験で、知らせの JSON が `curl` まで渡ることを確かめる。
2. 曖昧だった文: 「前回の起こしの後に `inbox_offset` が進んでいるとき（または一度も起こしていないとき）」（Issue #555 の受け入れ条件）— round 4 までは「前回起こしたときの受信箱の行数に届くまで」と読んでいた。Codex の指摘のとおり、文は「進んだか」なので、起こした時点の既読の位置と比べる形に直した。
3. 仮定と解釈: 読み手を外したので、壊れた UTF-8 の断片が残ったときに今の `conductor-watch.sh` が受信箱を読めなくなる問題は、#569 がマージされるまで残る。PR 本文の「対象外」に依存を書いた。
4. 足りなかった情報・検証手段: Claude が PR 外として挙げた sandbox からホストでコードが動く経路の可能性（pitchfork の worktree の定義、sandbox から書ける `~/.cargo` などの置き場）は、調査の Issue #570 にした。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「Request changes のとき」に「レビューの指摘が Issue の対象外のファイルに及ぶときは、直す前に指揮役か hatai に、含めるか別の Issue にするかを聞く」を足す。

使ったスキル: implement-issue、write-issue、write-pull-request

## 実装者メモ（round 7）

対象 head: 66e1bf58（実装のコミット）

1. 詰まった箇所: round 6 のレビューで、Codex（`gpt-6.2-sol` max）が must-fix 1 を出した。既読の位置が受信箱の行数を超えていても基準に保存するので、補助記録を直しても以後の起こしが止まり続ける。Codex は `pelway-review[bot]` 名義で投稿できず、結果はスレッドにだけ残った。Claude（`claude-opus-5-5` max）の must-fix 1 は、PR 本文の引用と項目名が round 5 のままだったこと。行数を超える既読の位置は使わず保存もしない形にし、起こしの基準を宛先と組で持つ形にした（Claude の nit）。PR 本文を直した。
2. 曖昧だった文: なし。
3. 仮定と解釈: Claude の nit のうち、own-event の回に未読が残っていても起こさない件と、`pelway/` を開く `unsafe` を `OpenOptions` に置き換える件は直していない（implement-issue の「nit は直さない」）。
4. 足りなかった情報・検証手段: Codex の round 6 の投稿の失敗の原因（T3 の `pull_request_review` で名義を確かめられない）は調べていない。
5. 改善案: `.agents/skills/review-pr/SKILL.md` に「投稿は `scripts/agent-gh review` で行い、T3 の `pull_request_review` は使わない」を足す。

使ったスキル: implement-issue、write-pull-request

## 実装者メモ（round 8）

対象 head: b60027ae（実装のコミット）

1. 詰まった箇所: round 7 は Claude（`claude-opus-5-5` max）が Approve、Codex（`gpt-6.2-sol` max）が must-fix 1。`inbox_offset` が文字列・負数のとき 0 として扱い、初めて起こす回は送っていた。項目が無いときは 0、あるのに 0 以上の整数でないときは読めない扱い（その回は起こさない）に直した。Codex は round 6・7 とも GitHub に投稿できず（`auto` の実行モードで `scripts/agent-gh review` が使えない）、指摘はスレッドから転記した。
2. 曖昧だった文: なし。
3. 仮定と解釈: Codex の文書の指摘（既読の位置を先へ書き換えたときの影響）も `docs/dev-environment.md` に足した。
4. 足りなかった情報・検証手段: Codex のレビューを GitHub に投稿させる設定（実行モード・権限）は hatai に確かめてもらう。
5. 改善案: `.agents/skills/command-team/SKILL.md` の Codex のレビューの委託に、投稿できる実行モードを書く（今は `full-access` が方針で拒まれ、`auto` では投稿できない）。

使ったスキル: implement-issue
