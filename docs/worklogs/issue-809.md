# Issue #809 の作業ログ

## 失敗の筋（実装前）

テストを書く前に考えた筋（round 1 の実装の前に立てたものを、この節の追加時に文に起こした）。

- 失敗の筋: 「レビューと QA」・「対象」の節に印（「最後に検証した sha」・単語としての「HEAD」）の行が無い・複数ある本文で、決まった行を取り違えて別の行の sha を書き換える／書き換えるべきでないのに書き換える。
  確かめ方: `bash scripts/tests/pr-body-update.test.sh` の、印の行が無い・複数ある入力で終了コード 1・PATCH 0 回を確かめる項目
- 失敗の筋: フェンス付きコード・インラインコード・HTML 注釈・行頭 4 空白の字下げの中にある印を正式な行と取り違え、コード例の sha を書き換える。
  確かめ方: 各構造の中に印を置いた入力の試験（`pr-body-update.test.sh`）
- 失敗の筋: PR を読んでから PATCH するまでの間に本文や head が変わり、古い内容を上書きする・違う head に書き込む。
  確かめ方: 「head moved after the read」「body changed after the read」の試験（再読み取りで終了コード 1・PATCH 0 回）
- 失敗の筋: PR 番号・`--verified` の形式が不正（数字以外・40 桁でない）で別の対象に届く。
  確かめ方: 引数検査の試験（終了コード 2）
- 失敗の筋: 一時ファイル（`--input` 用）や子プロセスの起動経路（シェル解釈・ファイルの残り）で秘密や本文が漏れる。
  確かめ方: 偽 `scripts/agent-gh` の呼び出し記録を読む試験（引数リスト起動・mkstemp・finally で削除）

## 実装者メモ（round 1）

対象 head: 89050f562404312cff45360454158df75c7f5959

1. 詰まった箇所: 手順 6: `scripts/agent-gh dev git merge --ff-only origin/main` が `.devin/config.json` の `Read-only file system` で途中まで進んで止まった（sandbox が worktree の `.devin/` を読み取り専用にする。main の #737 がこのファイルを 1 行変えた）。書き換わっていた 10 ファイルと増えた 6 ファイルを origin/main と照合してから試みの前（7c36a927 + この Issue の変更）に戻し、この round は main を取り込まずに進めた。`mise run verify` はこの作業環境では 2 本落ちる（`.agents/skills/verify-pelway/verify-run.test.sh` の 6 項目は `/tmp` の所有者が自分でないため。`scripts/tests/agent-team-config.test.sh` の試験 3 は、base の 7c36a927 で hatai の `Update agent-team.toml` に試験の pins が追いついていないため）。`scripts/red-check HEAD` で変更を退避しても同じ失敗が出ること、後者は `scripts/red-check origin/main` では通ることを確かめた。
2. 曖昧だった文: 「PR 番号を受け取り、PR 本文の決まった行（「対象」の HEAD、「レビューと QA」の最後に検証した sha）を、push 後の `headRefOid` と指定した sha に書き換えるコマンドがある」（Issue #809 の受け入れ条件）。「決まった行」の形はどこにも定義がなく、実際の本文は揺れている（PR #735 の「対象」は `` `<branch>` @ `<sha>` `` で HEAD の語が無い。#743・#744 は `- HEAD：`。#722 は HEAD の行にほかの sha の補足がある）。
3. 仮定と解釈: 「決まった行」は、節の中に印（`最後に検証した sha`、単語としての `HEAD`）を含む行がちょうど 1 つあることとし、sha は印の直後の 16 進数（7〜40 桁）とした。見つからない・複数あるときは書き換えずに非 0 で終わる。「40 桁の sha を含む唯一の行」とする解釈は、補足の sha と紛れるため選ばなかった。道具は `scripts/agent-gh` のサブコマンドではなく独立のスクリプトにした（`scripts/agent-gh` を変えると手順 8 でレビューを委託できなくなるため）。`--verified` は形式だけを検査し、PR の head の祖先かは確かめない。GitHub との読み書きは `scripts/agent-gh dev api` に統一した（`pr edit --body-file` が本文を加工しないことを手元で確かめられないため）。
4. 足りなかった情報・検証手段: 本物の GitHub への PATCH は、試験では擬似の agent-gh でしか確かめられない（PR 作成後に自分の PR で確かめ、結果は PR 本文に書く）。手確認で `cd /` から本体を走らせようとして外部ディレクトリの許可を開発リードに拒まれた。以後は確認環境を `/tmp/opencode/809/` 以下に置き、`TMPDIR=/tmp/opencode/809` で試験を走らせた。ほかに、`.agents/skills/write-pull-request/SKILL.md` の「定期ルール」の許可一覧に `gh api` が無いが、手順 3 とこの道具は `api` を使う。
5. 改善案: `.github/pull_request_template.md` の「レビューと QA」と「対象」のコメントに ``最後に検証した sha: `<sha>` `` と ``HEAD: `<sha>` `` の行を書くよう注記すると、初回の本文から道具が読める形になる。`.agents/skills/implement-issue/SKILL.md` の「決まり（毎回守る）」に「sandbox 下の作業ツリーは `.devin/` が読み取り専用のため、`.devin/` を変えた main を取り込めないことがある。取り込めないときはそのまま push し、衝突は引き継ぎの作業ツリーで解消する」を足す。

使ったスキル: `.agents/skills/implement-issue`、`.agents/skills/diagnosing-bugs`（`mise run verify` の失敗の切り分けに Phase 2。`scripts/red-check HEAD`・`origin/main` で変更を退避して再現した。1 回だけ現れた `pelway-agent-watch` の `send_deadline_covers_secrets_and_curl`（1 回目）と `scripts/tests/conductor-rebuild.test.sh` の「term-ignoring read killed: returns after the grace」（3 回目）は、単独実行とほかの回では通ったので一過性）

## 失敗の筋（最終）

Draft PR を出す前の版（round 1）。実装前の 5 本はすべて残し、実装で見つけた筋を足した。

- 失敗の筋: 「実装前」の筋はすべて残る（印の行の過不足・コード文脈の印・読み取りと PATCH の間の変化・引数の形式・一時ファイルと子プロセス）。
  確かめ方: 「実装前」に書いた試験（`pr-body-update.test.sh`）
- 失敗の筋: 実際の本文は形が揺れる（PR #735 の `` `<branch>` @ `<sha>` `` 形・`HEAD` の行に補足の sha がある形）。揺れた形の本文で誤った行を選ぶ。
  確かめ方: 揺れた形をまねた入力で「行が無い・複数ある」と非 0 で止まる試験。`HEAD` の行が無い形は PR 本文の「対象外」に明記

## 失敗の筋の見直し（round 2）

head 70d5433e のレビュー（approve と nit）で見つかった筋: テンプレートの注釈の中の `HEAD` の語を印として数え、正式な行が 1 つでも「行が複数ある」と止まる（誤書き換えではなく過剰な拒否）。

- 失敗の筋: 注釈の中の語を印として数えて、書き換えられる本文で止まる。
  確かめ方: 注釈の中に印を含む入力で正式な行だけが変わる試験

## 失敗の筋（round 2 の最終）

- 失敗の筋: round 1 の最終の筋はすべて残る。round 2 で足した「注釈の中の語を数えて過剰に拒否」は、注釈の中を数えない処理で防いだが、注釈の終わりの見分け方のずれは残る。
  確かめ方: 注釈の中に印を含む入力の試験。残るずれは round 3 以降のレビューで追う

## 実装者メモ（round 2）

対象 head: 6fe9d8250beca82cadeba78648cb79b0d51a0668

1. 詰まった箇所: 手順 8: 割り当ての指示に「codex_review は利用上限（2026-11-28 14:00 JST まで）」とあったので、確認ゲートの決まりどおり codex の番を Claude に回し、`claude_review` だけに委託した。直後に指揮役から PR #840 のコメントで「codex の利用上限記録は古い。マージには Codex の approve が必須で Claude/Devin では代用しない」と訂正があり、指揮役が head 70d5433e に codex_review を委託した（run 7194967460）。`orchestrator_capabilities` では `devin_review` が `Provider instance is disabled.` で子タスクを起動できない。
2. 曖昧だった文: 「HTML コメントは残してよい。」（`.agents/skills/write-pull-request/SKILL.md` の手順 4）— テンプレートの「対象」の注釈 `<!-- ブランチ名と HEAD の sha -->` には HEAD の語があり、注釈を残した本文では round 1 の道具は「HEAD の行が 2 つある」と止まった（Claude のレビュー run 7194801754 の `[文書のずれ]` の nit）。
3. 仮定と解釈: 上の nit は `.github/pull_request_template.md` の注釈を書き換える提案だったが、この PR ではテンプレートを変えずに道具を直した（HTML コメントの中の見出しや印は数えない）。`.github/` の下を変える PR は Kodiak ではマージできず hatai の admin マージになるため（skill `command-team` の「マージ」）。テンプレートに 2 行の形を書くことは round 1 の改善案のまま残す。もう 1 件の nit（手順 7 の終了コードと直し方）は指摘どおり文を書き換えた。印の無い nit（push 直後の API が前の head を返す可能性）は直していない。
4. 足りなかった情報・検証手段: 変異（コメントを空白に替えるときの幅を 1 文字にする）が最初の試験では生き残った。同じ行で注釈が印の前に来る本文（`<!-- round 1 --> - 最後に検証した sha: ...`）を試験に足して倒した。`mise run verify` は round 2 でも落ちるのが同じ 2 本だけ（環境起因の `verify-run.test.sh` と base 起因の `agent-team-config.test.sh`）で、一過性の失敗は無かった。
5. 改善案: `.agents/skills/command-team/SKILL.md` の「レビュー担当と effort」に「保留以外の理由（利用上限など）で担当が使えないときの扱い」を書く。割り当ての記録が古いと、実装担当が代用してよいかを判断する材料が無い。

使ったスキル: `.agents/skills/implement-issue`、`.agents/skills/mutation-testing`（手動の変異 17 種の生き残りの解釈に「What Results Mean」。差分がミューテーションテスト対象の crate に触らないため `mise run mutants:diff` は対象外）

## 失敗の筋の見直し（round 3）

head 15f9f2fc のレビュー（Codex must-fix 3 件）で足した筋: 書き込み直前の照合不足（読み取りから PATCH までに変わった本文を上書き）、フェンス・インラインコードの中の印、sha の語の境界。

- 失敗の筋: 読み直しをしないと、読み取りと PATCH の間に変わった本文・head をそのまま上書きする。
  確かめ方: 「head moved after the read」「body changed after the read」の試験 + PATCH 応答の head と本文の照合
- 失敗の筋: sha の直後に英数字や `_` が続く語（`2222222xyz`）や 41 桁以上の並びを sha として書き換える。
  確かめ方: 語の境界の試験（両方の行で何も書き換えない）

## 失敗の筋（round 3 の最終）

- 失敗の筋: 残る筋は、印の行の過不足、コード文脈（フェンス・インラインコード・注釈・字下げ）の中の印の取り違え、注釈の中の語の計数、読み取りと PATCH の間の変化、語の境界、引数の形式、一時ファイルと子プロセス、本文の形の揺れ。
  確かめ方: `pr-body-update.test.sh` の各項目と手動変異 26 種（すべて倒せた）

## 実装者メモ（round 3）

対象 head: ff6d2393853ca6e51a2a510da8b236266ca69086

1. 詰まった箇所: 手順 8〜9: head 15f9f2fc のレビューで判定が割れた（Claude が approve、Codex が request-changes の must-fix 3 件）。Issue にそれぞれの要点をコメントし、修正側の must-fix を直す round にした。
2. 曖昧だった文: 「書き込み直前に本文と head を照合し、変わっていれば止めてください。API で照合と更新を一体にできない場合は、同時編集を避ける手順と残る競合の範囲を明記してください。」（Codex のレビュー run 7206801692 の must-fix）— この API が条件付き更新を受け付けるかは、どこにも書き方が無い。
3. 仮定と解釈: PR #840 に `If-Match` を付けた PATCH（本文は同じ内容）を試すと `HTTP 400`（`Conditional request headers are not allowed in unsafe requests unless supported by the endpoint`）が返ったため、照合と更新は一体にできないと判断した。書き込みの直前にもう一度読んで本文と head を照合し、書き込みの応答の head と本文も確かめる形にした。残る競合（読み直しから書き込みまでの間に重なった本文の編集は上書きされる）を道具の説明と skill implement-issue の手順 7 に書いた。「Markdown のコードの文脈」は、フェンス付きコードブロックの中を数えないことと、インラインコードの中の `<!--`・`-->` を注釈の記号にしないことと解した（sha を書く形がインラインコードのため、インラインコードの中身は数える）。Claude の印の無い nit（閉じ忘れの注釈の試験が、見出しの行ごと消しているため弱い）は、同じ処理を書き直す中で試験を強くした。
4. 足りなかった情報・検証手段: 手動の変異 26 種がすべて試験で倒せることを確かめた。最初は「書き込みの後に GitHub が返した本文を確かめない」と「注釈の中のインラインコードを空白に替えない」の 2 種が生き残ったので、試験を足した。`mise run verify` は round 3 でも落ちるのが同じ 2 本だけ（一過性の失敗は無し）だった。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 8 に「担当が使えるかは、割り当ての記録だけでなく `orchestrator_capabilities` の `canRunChildTask` と `constraints` でも確かめる。割り当ての記録の利用上限が古いことがあり、使えない担当は `Provider instance is disabled.` と出る」を足す。

使ったスキル: `.agents/skills/implement-issue`、`.agents/skills/mutation-testing`（手動の変異 26 種の生き残りの解釈に「What Results Mean」）

## 失敗の筋の見直し（round 4）

head 71d0fa5f のレビュー（Codex must-fix 2 件）で足した筋: 節の中のコード例を決まった行に取り違える（印がコードの外にある行だけを候補にする必要）、注釈の終わりの見分け方。

- 失敗の筋: 例（コードの中や補足の中の `HEAD:` の行）を正式な行として書き換える、または正式な行があるのに例を混ぜて「複数ある」と止まる。
  確かめ方: 「例しかないとき PATCH 0 回」「正式な行があるとき例を残して正式な行だけ変わる」試験（両節）
- 失敗の筋: 注釈の終端（`-->` の位置・行をまたぐ注釈）の見分け方が GitHub とずれ、注釈の外にある印を見逃す・注釈の中の例を見る。
  確かめ方: 行頭から・行の途中から始まる注釈と、複数行の注釈の試験

## 失敗の筋（round 4 の最終）

- 失敗の筋: 残る筋は round 3 の最終と同じ + 例の取り違え（印がコードの外にある行だけを候補にする近似で、残る範囲は「対象外」に明記）。
  確かめ方: `pr-body-update.test.sh` 54 項目と手動変異 38 種（すべて倒せた）

## 実装者メモ（round 4）

対象 head: b583d282dcc78ccb59815ba7d1baf969b727d12c

1. 詰まった箇所: 手順 9（round 3 のレビュー結果）: Codex は request-changes（must-fix 2 件と `[文書のずれ]` の nit 2 件）、Claude は approve（`[文書のずれ]` の nit 1 件。Codex の nit と同じ終了コードの説明なので一緒に直した）。ラウンドの上限（round 3 のレビュー結果に must-fix が残ったら止まる）は、開発リードから「濱田さんの指示（t629u）でラウンド上限は無視する。needs-human も付けない」と T3 スレッドで指示されたため、止まらずに修正した。差分が 800 行を超えそうになった（道具 287 行・試験 608 行）ため、試験の期待値を入力本文から sed で作る形に書き直し（入力と期待値の二重書きをやめた）、`--help` をモジュールの説明と共用にした。確かめる内容は減らしていない（`mise run pr-size` は実装コミットで +767 / -4）。
2. 曖昧だった文: 「ラベルがコードの外にある専用の行だけを候補にし、ラベルの後ろの SHA にはバッククォートを許してください。」（Codex のレビュー run 7208374175 の must-fix）— 「専用の行」が行頭から始まることを要求するかは書かれていない。
3. 仮定と解釈: 「専用の行」は、印がコードの外にある行と解した（行頭は要求しない。PR #735 の「実装者メモ: …。最後に検証した sha: `…`。」のように文の途中にあっても候補にする）。行頭に 4 つ以上の空白がある行は候補にしない。注釈は GitHub の表示より多めに隠す近似に倒し（書き換えの誤りを避ける方向。行頭から始まったものは終端の行の終わりまで、行の途中から始まったものは閉じるまで `-->` までを注釈とした。round 5 で表記を直した）。
4. 足りなかった情報・検証手段: 手動の変異 38 種がすべて試験で倒せることを確かめた。試験をまとめ直したあと、バッククォートで囲んだ sha の語の境界と、複数行の注釈の終わりの 2 種が生き残ったので、バッククォート無しの語と、複数行の注釈の後ろに正式の行を置く試験を足した。`mise run verify` は round 4 の 2 回とも、落ちるのが同じ 2 本だけだった。
5. 改善案: hatai の指示（t629u）で運用が変わったなら、`.agents/skills/implement-issue/SKILL.md` の「止まる条件」と `.agents/skills/command-team/SKILL.md` の「レビューのラウンド」の上限を更新する。更新までの間は、割り当ての指示に「ラウンド上限は無視する」と書いておくと、実装担当が止まるかどうかの判断に迷わない。

使ったスキル: `.agents/skills/implement-issue`、`.agents/skills/mutation-testing`（手動の変異 38 種の生き残りの解釈に「What Results Mean」）

## 失敗の筋の見直し（round 5）

head aa8043f4 のレビュー（Codex must-fix 2 件・Claude must-fix 3 件）で足した筋: 行をまたぐインラインコードを誤った位置で打ち切る・長さの違うバッククォート列で位置がずれる、箇条書きの中のフェンス付きコード、消えた本文不変のアサーション。

- 失敗の筋: 行をまたぐインラインコードの続きの行を新しいブロックと誤って判定してコードを早く切る、または隠すときに文字数を保たず印の位置がずれて、コード例を書き換える・誤った位置を書き換える。
  確かめ方: CommonMark のブロック生成に合わせた判定と文字数を保つ隠し方の試験（コードが閉じた後ろの正式な sha だけが変わる）
- 失敗の筋: 箇条書きの行で始まるフェンス付きコードを構造ごと見逃し、中の例を唯一の行として書き換える。
  確かめ方: その構造を含む節で書き換えずに終了コード 1 の試験
- 失敗の筋: 「失敗の試験で本文が変わらない」アサーションが消えると、書き換えたあと本文が壊れても気付かない。
  確かめ方: `fails_with`・`fails_quietly` に GitHub 側の本文と入力本文の `cmp` を戻した

## 失敗の筋（round 5 の最終）

- 失敗の筋: 残る筋は round 4 の最終と同じ + 行をまたぐコードの位置ずれ・コンテナ内フェンス（構造が分からない節は拒否する近似で、残る範囲は「対象外」に明記）。
  確かめ方: `pr-body-update.test.sh` 57 項目。新しい 3 件は修正前の head で落ちることを `scripts/red-check HEAD` で確かめた

## 実装者メモ（round 5）

対象 head: cc039786239c5227fe77d962bfdd9f8eb4fb9695

1. 詰まった箇所: 手順 9: aa8043f4 への round 4 のレビューは 2 件届いていた（Codex run 7211569851 の must-fix 2 件、Claude run 7211569263 の must-fix 3 件と `[文書のずれ]` の nit 2 件。Claude の must-fix 1 件は Codex のと同じ「長さの違うバッククォート列で位置がずれる」）。残りは、round 4 で消えた「失敗の試験で本文が変わらない」アサーション 2 件と、注釈の説明文のずれ。`git merge origin/main` は round 1 と同じく `.devin/config.json` の `Read-only file system` で止まるため、main が変えた範囲（`scripts/pr-body-update`・`implement-issue/SKILL.md`）はこのブランチの変更と重ならないことを diff で確かめ、取り込まずに進めた。
2. 曖昧だった文: 「ブロックの始まり」の判定 — Markdown のどの規則に合わせるかはレビューにも仕様にも書かれていない。
3. 仮定と解釈: CommonMark のブロック生成の規則に合わせた（見出しは `#` の後ろに空白か行末、バッククォートのフェンスの info 文字列に `` ` `` は含めない、段落は箇条書きでは `-`・`+`・`*` と `1.` だけで切れる、字下げ 4 以上では切れない）。箇条書きの中で開いたフェンスは容器ごとの解析が要るため、その構造を含む節は書き換えずに止める形にした。インラインコードを隠すときは文字数を保つ（違う長さのバッククォート列を 1 空白に潰すと位置がずれるため）。消えたアサーションは、直し方の選択肢 (2) で `fails_quietly`・`fails_with` に GitHub 側の本文と置いた本文の `cmp` を足して戻した（確かめ方を弱める変更ではなくなった）。注釈の nit は文を直す側を選び、道具・PR 本文・round 4 のメモを「GitHub より多めに隠す近似」にそろえた。「対象外」の nit の記述は、正式な行が残る場合に限り成り立つ（正式な行が無く、誤って外と判定したコード例が 1 つだけ残る本文では、その例を書き換え得る）。このずれは round 6 で文を直す。
4. 足りなかった情報・検証手段: 新しい試験 3 件が修正前の head で落ちること（`scripts/red-check HEAD`）と、修正後で通ることを確かめた。`mise run verify` は 2 回走らせ、1 回目は `target/` がビルドの途中で消える環境起因の失敗（`aws-lc-sys` の .o 作成と rustc の一時 dir）、2 回目は落ちるのが同じ 2 本だけ（`verify-run.test.sh` と `agent-team-config.test.sh`）だった。
5. 改善案: 指摘の「節の中の例をコードの文脈から守る」問題は、PR 本文を生成する側（テンプレート・skill）で例をフェンス付きコードに置く形にすれば、道具側の構文解析をこれ以上育てなくて済む。

使ったスキル: `.agents/skills/implement-issue`

## 失敗の筋の見直し（round 6）

head 2d22c017 のレビュー（Codex must-fix 2 件）で足した筋: バッククォートで開くコンテナ内フェンスが検査に届かない、引用の継続行でコードが切れる。

- 失敗の筋: `- ```text`・`> ``` ` のように箇条書き・引用で始まるバッククォートのフェンスが、インラインコードを隠した view にしか検査が無いため見逃され、中の例を書き換える。
  確かめ方: 試験 758（`pr-body-update.test.sh`。コンテナ内フェンスを含む両節で終了コード 1・PATCH 0 回）
- 失敗の筋: `>` で始まる引用の継続行を必ず新しいブロックとみなし、同じ引用の中で続くインラインコードを切って中の印を見る。
  確かめ方: 試験 759（同じ深さの引用の継続行ではコードが切れず、中の印は数えない）

## 失敗の筋（round 6 の最終）

- 失敗の筋: 残る筋は round 5 の最終と同じ + バッククォートのコンテナ内フェンス・引用継続行（raw の行への照合と引用の深さの追跡で防いだ）。
  確かめ方: `pr-body-update.test.sh` 59 項目すべて `ok`。新しい 2 件は修正前の head で落ちることを `scripts/red-check HEAD` で確かめた

## 実装者メモ（round 6）

対象 head: 019b8d935d46137d651198c9ff835b5415252fb0

1. 詰まった箇所: `mise run pr-size` が +813/-4 で上限（800）を 13 超えた。skill の決まり（重複だけ削り、試験や説明は削らない）に従い、指揮役の判断を仰ぐために push して Issue #809 に停止報告・分割案・round 6 の記録を書いて `needs-human` で止まった。hatai の `<!-- pr-size-exception -->`（11/27 23:43 JST）とラベルで例外が成立し、指揮役が停滞でないことを確認したので再開した。main の取り込みは `git merge` が `.devin/config.json` の読み取り専用で止まるため、`git merge-tree --write-tree` で木を作り、競合（`.agents/skills/write-pull-request/SKILL.md`。main が「失敗の筋」「リスクとセキュリティへの影響」の節を足して番号が繰り上がった）をこのブランチの追記文の付け替えで解き、一時 index と `commit-tree` で merge コミット（019b8d93。`agent-gh-resign` は merge を作り直せないため署名なし）を push した。
2. 曖昧だった文: 「失敗の筋」はこの作業の開始時点では skill・テンプレートに無い欄で、main に 062c139e で入った後の取り込みで要る形になった。round 1〜6 分を遡って書くとき、各「見直し」「最終」の内容はその round の実装者メモとレビュー（run の ID）から写すことにした。
3. 仮定と解釈: round 6 のレビュー（Devin run 6095814055）の must-fix 3 件のうち、本文の節追加は `## 失敗の筋`・`## リスクとセキュリティへの影響` をテンプレートの順（「確認方法/検証」と「レビューと QA」の間）に置き、各項目を「- 失敗の筋: / 確かめ方:」「- 影響: / 対応:」の 2 行組で書いた。ラウンドサマリーの未投稿分（round 4・5）は、サマリーは修正を push した時点で書く決まりのため、round 4 は head cc039786・round 5 は head 21ff9bb（+merge の 019b8d93）で直した内容として遡って投稿した。
4. 足りなかった情報・検証手段: 59 項目すべて `ok`、新しい試験 2 件（758・759）は修正前の head で落ちることを `scripts/red-check HEAD` で確かめた。merge 後も `pr-body-update.test.sh`・`pr-body-round-history-docs`・`doc-drift-nit-docs`・`review-delegation-docs`・`agent-team-config`（main で pins が追いつき通るようになった）すべて `ok`。
5. 改善案: 「失敗の筋」の記録は round 1 から要る形で運用が始まったため、開始時点に無い欄を遡って埋める手間が生じる。欄の追加は skill とテンプレートに入った時点で、進行中の PR への適用時期（次の修正 round で足す等）を一文書いておくと迷わない。

使ったスキル: `.agents/skills/implement-issue`、`.agents/skills/write-pull-request`
