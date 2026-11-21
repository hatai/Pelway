# Issue #602 の作業ログ

## 実装者メモ（round 1）

対象 head: d7abbed28d7e8a7935df9f8fe5ea40fc4f1c1423

1. 詰まった箇所: `scripts/conductor-watch.sh` の `"$T3_BIN" thread list --json > threads.json || true`: t3 が失敗しても空の一覧と同じに読まれ、登録があると `thread-missing` の誤った格上げになる。`|| rm -f` に替えて取得失敗を区別した。`gh issue edit --add-label needs-human --remove-label needs-decision` が 1 回の不可分な更新かを確かめられなかったため、`needs-human` を付けてから `needs-decision` を外す 2 回の呼び出しに分けた（途中で失敗してもどちらのラベルも無い状態にしない）。
   登録ファイルを最初は中身全体の strip で読んでいたが、`crates/pelway-agent-watch/src/inbox.rs` の `parse_conductor_file` は 1 行目だけを使う。2 行以上のファイルで動いている指揮役を `thread-missing` と読み違えるため、1 行目だけを読むように直した（場面 19）。
2. 曖昧だった文: 「`t3 thread list` の取得に失敗した周は判定せず、次の周に回す（指揮役がいるのかいないのか分からないまま替えない）」（Issue #602 の「背景・前提」）。登録ファイルが無ければ、一覧が無くても T3 の指揮役がいないことは分かる。
3. 仮定と解釈: 受け入れ条件の無条件の文（「取得に失敗した周では格上げせず」）に合わせ、登録ファイルが無くても一覧の取得に失敗した周は格上げしないと解釈した。選ばなかった解釈は「登録が無ければ一覧なしでも格上げする」。この解釈のままだと、t3 が失敗し続けるホストでは格上げが起きない。目印の名前は未決事項の起票者の案で進めた。`grokbot` の目安 24 時間は指揮役が書く待ちに残し、`needs-decision` から導く待ちだけを `labeled_at` + 15 分にした。
4. 足りなかった情報・検証手段: `needs-decision` のラベルが GitHub にまだ無いため、実際の `issue edit --remove-label needs-decision` と、`pelway-dev[bot]` 名義のコメントの `@hatai` で通知が届くことは、モックの `agent-gh` でしか確かめていない（通しの訓練 #605 で確かめる）。
5. 改善案: `docs/agents/triage-labels.md` の `needs-human` の行の Who adds に「conductor-watch, when `needs-decision` stays 15 minutes and no conductor is running」を足す（#603 で `needs-decision` の行を足すときに一緒に）。

使ったスキル: `implement-issue`、`write-pull-request`

## 実装者メモ（round 2）

対象 head: 97557b9e69dc860c3d777411d48b4c393e7ac82a

1. 詰まった箇所: 手順 3（修正依頼は今のブランチのまま）: `agent/602-decision-escalation` が round 1 の worktree（`.claude/worktrees/agent-af759d14a5b9ce62e`）に checkout されたままで、同じ名前のブランチに切り替えられなかった。別名のローカルブランチ `agent/602-decision-escalation-r2` に `--track origin/agent/602-decision-escalation` を付け、`scripts/agent-gh dev git push origin HEAD:agent/602-decision-escalation` で push した（`agent-gh-resign` は `@{u}` で同じブランチを解く）。
2. 曖昧だった文: 「追加を先に行う順序は保ち、見張り自身が始めた同じ付与の格上げだと確認できる場合に限って、残った削除を再試行できるようにしてください。」（PR #610 の codex のレビューの must-fix 3）。外し直しに、指揮役が動いていないことを条件に付けるかが書かれていない。
3. 仮定と解釈: Issue #602 の対象外「指揮役が動いているときに conductor-watch がラベルを替えること」と、runbook の「1. 役割と触ってよい範囲」の文を優先し、外し直しも指揮役が動いていない周だけ行うと解釈した（`DECISION escalated` の `reason=` も今の 3 語のまま出せる）。選ばなかった解釈は「指揮役の有無によらず外し直す」。指揮役が動き始めた後に残った両方のラベルは、`step=label` の行を読んだ指揮役が外すと runbook に書いた。外し直しは、受け入れ条件「`needs-human` が付いていれば、コメントもラベルの変更もしない」の例外になる（PR 本文の「ずらした点」）。外し直しが失敗し続ける間に人間が `needs-human` を外すと、次の周で通知なしに `needs-human` が付き直す挙動は残る。重複の判定は、`api` で読み返した `pelway-dev[bot]` のコメント（Issue #602 の取得コメント）が LF の改行と末尾の改行を保っていたので、正規化しない完全一致にした。
4. 足りなかった情報・検証手段: 読み直しの間の付け直しは、モックの `agent-gh` が Issue の読み直しに応えるときに `labeled` のイベントを足す形でしか試せていない。本物の GitHub で events に反映されるまでの遅れは確かめていない。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「3. ブランチを用意する」に「修正依頼のブランチが別の worktree に checkout されたままなら、別名のローカルブランチに `--track origin/<ブランチ>` を付け、`scripts/agent-gh dev git push origin HEAD:<ブランチ>` で push する」を足す。

使ったスキル: `implement-issue`、`write-pull-request`

## 実装者メモ（round 3）

対象 head: 693e27801afca34980b8c039cb8109519215e9f5

1. 詰まった箇所: `scripts/conductor-watch.sh` の登録ファイルの読み取り: round 2 の `os.fdopen` は、登録ファイルの場所がディレクトリだと `IsADirectoryError`（CPython の FileIO はディレクトリを拒む）で python ごと落ちる形だった。fd のまま `os.fstat` と `os.read` で読み、ディレクトリは「通常のファイルでない」として扱うようにした（`registered_thread`、場面 9 の確認を足した）。
2. 曖昧だった文: 「Issue #602 が不在と定める条件を確認できた場合だけ格上げしてください。」（PR #610 の round 2 の codex のレビューの must-fix 1）。自分の所有でない通常ファイルが、Issue の「通常のファイルでない」に入るかが書かれていない。
   「あれば人間が外したので、付け直さない。」（PR #610 の round 2 の claude のレビューの nit 2）。この判定を、見張りの格上げが途中の Issue だけに当てるか、記録の無い Issue にも当てるかが書かれていない。
3. 仮定と解釈: 自分の所有でない通常ファイルは不在の 5 つに当たらないとして保留にした（`crates/pelway-agent-watch` の `read_conductor_file` も Err を返し、agent-watch は起こしを見送る）。選ばなかった解釈は round 2 までの「不在」。付け直さない判定は、前の周に通知を書いた格上げにだけ当てた（`resumed`）。選ばなかった解釈は「記録が無くても書かない」で、受け入れ条件（`needs-human` が無ければ格上げする）から外れ、その Issue を誰も格上げしないまま毎周読み続けるため採らなかった。見張りが `needs-human` を付けた後に人間が外したときは、`needs-decision` だけを外して見張りの操作を終える。選ばなかった解釈は「何もせず指揮役に残す」。
4. 足りなかった情報・検証手段: 自分の所有でない登録ファイルの試験は、別の uid が要るため書いていない。権限の無い登録ファイルの場面 23 は root で流すと確かめにならない（root は読めて、生きている ID を読むので書かない）。CI（`ubuntu-24.04` の `runner`）とローカル（uid 1000）は root でない。events がラベルの変化より遅れて届くときの挙動はモックでしか確かめていない。
5. 改善案: `scripts/conductor-watch.sh` の「ボード Status 更新キュー」の第 1 段に「名前が `[A-Za-z0-9._-]+\.json` に合わない要求ファイルは、名前を行に出さずに `bad-name` で退ける」を足す（エージェントが決められるファイル名で `watch.log` に行を作り込めないようにする。PR #610 の round 2 の codex の security-review の指摘）。

使ったスキル: `implement-issue`、`write-pull-request`、pstack の `poteto-mode`（principle-make-operations-idempotent・principle-laziness-protocol・principle-model-the-domain）

## 実装者メモ（round 4）

対象 head: f2bddb26dbecdc20f5de4c8f52a7717d9bd04043

1. 詰まった箇所: `scripts/conductor-watch.sh` の `escalate`: Issue と events は別々の読み取りなので、どちらを先に読んでも、後の読み取りの間の変化は先の読み取りに出ない。must-fix 2 に合わせて events → Issue の順にし、events の最後の `needs-decision` の付け外しで付与が続いているかも確かめた。この順では Issue の読み取りの間の付け直しが見えないので、場面 21 の前半の差し込みを events の取得に移した（PR 本文の「確かめ方を弱める変更」）。`mise run verify` の `typos` が変数名 `nd` を `and` の誤字と判定して落ちた（名前を変えた）。
2. 曖昧だった文: 「追加の出どころを確認できない場合は、指揮役が確認できる形で保留を記録してください。」（PR #610 の round 3 の codex のレビューの must-fix 3）。付いた `needs-human` がラベル履歴にまだ出ていない場合を「確認できない」に入れるかが書かれていない。
3. 仮定と解釈: 試行の起点を時刻でなく events の ID（`seen`）にした。GitHub の Issue ごとの events は古い順に ID が増え、App のラベル操作の `actor.login` は `pelway-dev[bot]` だった（#602・#571・#488 の events）。時刻は 1 秒の刻みで、試行の直前の解除と自分の付与が同じ秒になると取り違える。`held` は、`seen` より後にほかの名義のイベントだけがあるときと、通知の後で付ける前にほかの誰かの `needs-human` があったときだけにし、イベントがまだ無いときは次の周に回す。選ばなかった解釈は「イベントが無ければ held」で、ラベル履歴の遅れだけで格上げが止まる。試行の後に人間が外した `needs-human` は付け直さず、`needs-decision` だけを外して終える（場面 26 と同じ扱い。選ばなかった解釈は「held にして何もしない」）。
4. 足りなかった情報・検証手段: 応答が失われる・events がラベルより遅れる・保存の前に止まる、はモックでしか確かめていない。同じ App の名義の別のスレッドが試行の後に付けた `needs-human` は、見張りの付与と区別できない。`needs-decision` を外した応答が失われると、GitHub は正しい状態になるが `DECISION escalated` の行は出ない（次の周には Issue が一覧から外れる）。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「止まる条件」の表の 3 回目のレビューの行に「hatai が上限を外したときは、その指示を hatai の名義で Issue にコメントしてもらってから修正 round に入る」を足す（この PR では上限を外した指示が `pelway-dev[bot]` のコメントにしか残っていない）。

使ったスキル: `implement-issue`、`write-pull-request`、pstack の `poteto-mode`（principle-make-operations-idempotent・principle-model-the-domain・principle-separate-before-serializing-shared-state）
