# Issue #809 の作業ログ

## 実装者メモ（round 1）

対象 head: 89050f562404312cff45360454158df75c7f5959

1. 詰まった箇所: 手順 6: `scripts/agent-gh dev git merge --ff-only origin/main` が `.devin/config.json` の `Read-only file system` で途中まで進んで止まった（sandbox が worktree の `.devin/` を読み取り専用にする。main の #737 がこのファイルを 1 行変えた）。書き換わっていた 10 ファイルと増えた 6 ファイルを origin/main と照合してから試みの前（7c36a927 + この Issue の変更）に戻し、この round は main を取り込まずに進めた。`mise run verify` はこの作業環境では 2 本落ちる（`.agents/skills/verify-pelway/verify-run.test.sh` の 6 項目は `/tmp` の所有者が自分でないため。`scripts/tests/agent-team-config.test.sh` の試験 3 は、base の 7c36a927 で hatai の `Update agent-team.toml` に試験の pins が追いついていないため）。`scripts/red-check HEAD` で変更を退避しても同じ失敗が出ること、後者は `scripts/red-check origin/main` では通ることを確かめた。
2. 曖昧だった文: 「PR 番号を受け取り、PR 本文の決まった行（「対象」の HEAD、「レビューと QA」の最後に検証した sha）を、push 後の `headRefOid` と指定した sha に書き換えるコマンドがある」（Issue #809 の受け入れ条件）。「決まった行」の形はどこにも定義がなく、実際の本文は揺れている（PR #735 の「対象」は `` `<branch>` @ `<sha>` `` で HEAD の語が無い。#743・#744 は `- HEAD：`。#722 は HEAD の行にほかの sha の補足がある）。
3. 仮定と解釈: 「決まった行」は、節の中に印（`最後に検証した sha`、単語としての `HEAD`）を含む行がちょうど 1 つあることとし、sha は印の直後の 16 進数（7〜40 桁）とした。見つからない・複数あるときは書き換えずに非 0 で終わる。「40 桁の sha を含む唯一の行」とする解釈は、補足の sha と紛れるため選ばなかった。道具は `scripts/agent-gh` のサブコマンドではなく独立のスクリプトにした（`scripts/agent-gh` を変えると手順 8 でレビューを委託できなくなるため）。`--verified` は形式だけを検査し、PR の head の祖先かは確かめない。GitHub との読み書きは `scripts/agent-gh dev api` に統一した（`pr edit --body-file` が本文を加工しないことを手元で確かめられないため）。
4. 足りなかった情報・検証手段: 本物の GitHub への PATCH は、試験では擬似の agent-gh でしか確かめられない（PR 作成後に自分の PR で確かめ、結果は PR 本文に書く）。手確認で `cd /` から本体を走らせようとして外部ディレクトリの許可を開発リードに拒まれた。以後は確認環境を `/tmp/opencode/809/` 以下に置き、`TMPDIR=/tmp/opencode/809` で試験を走らせた。ほかに、`.agents/skills/write-pull-request/SKILL.md` の「定期ルール」の許可一覧に `gh api` が無いが、手順 3 とこの道具は `api` を使う。
5. 改善案: `.github/pull_request_template.md` の「レビューと QA」と「対象」のコメントに ``最後に検証した sha: `<sha>` `` と ``HEAD: `<sha>` `` の行を書くよう注記すると、初回の本文から道具が読める形になる。`.agents/skills/implement-issue/SKILL.md` の「決まり（毎回守る）」に「sandbox 下の作業ツリーは `.devin/` が読み取り専用のため、`.devin/` を変えた main を取り込めないことがある。取り込めないときはそのまま push し、衝突は引き継ぎの作業ツリーで解消する」を足す。

使ったスキル: `.agents/skills/implement-issue`、`.agents/skills/diagnosing-bugs`（`mise run verify` の失敗の切り分けに Phase 2。`scripts/red-check HEAD`・`origin/main` で変更を退避して再現した。1 回だけ現れた `pelway-agent-watch` の `send_deadline_covers_secrets_and_curl`（1 回目）と `scripts/tests/conductor-rebuild.test.sh` の「term-ignoring read killed: returns after the grace」（3 回目）は、単独実行とほかの回では通ったので一過性）

## 実装者メモ（round 2）

対象 head: 6fe9d8250beca82cadeba78648cb79b0d51a0668

1. 詰まった箇所: 手順 8: 割り当ての指示に「codex_review は利用上限（2026-11-28 14:00 JST まで）」とあったので、確認ゲートの決まりどおり codex の番を Claude に回し、`claude_review` だけに委託した。直後に指揮役から PR #840 のコメントで「codex の利用上限記録は古い。マージには Codex の approve が必須で Claude/Devin では代用しない」と訂正があり、指揮役が head 70d5433e に codex_review を委託した（run 7194967460）。`orchestrator_capabilities` では `devin_review` が `Provider instance is disabled.` で子タスクを起動できない。
2. 曖昧だった文: 「HTML コメントは残してよい。」（`.agents/skills/write-pull-request/SKILL.md` の手順 4）— テンプレートの「対象」の注釈 `<!-- ブランチ名と HEAD の sha -->` には HEAD の語があり、注釈を残した本文では round 1 の道具は「HEAD の行が 2 つある」と止まった（Claude のレビュー run 7194801754 の `[文書のずれ]` の nit）。
3. 仮定と解釈: 上の nit は `.github/pull_request_template.md` の注釈を書き換える提案だったが、この PR ではテンプレートを変えずに道具を直した（HTML コメントの中の見出しや印は数えない）。`.github/` の下を変える PR は Kodiak ではマージできず hatai の admin マージになるため（skill `command-team` の「マージ」）。テンプレートに 2 行の形を書くことは round 1 の改善案のまま残す。もう 1 件の nit（手順 7 の終了コードと直し方）は指摘どおり文を書き換えた。印の無い nit（push 直後の API が前の head を返す可能性）は直していない。
4. 足りなかった情報・検証手段: 変異（コメントを空白に替えるときの幅を 1 文字にする）が最初の試験では生き残った。同じ行で注釈が印の前に来る本文（`<!-- round 1 --> - 最後に検証した sha: ...`）を試験に足して倒した。`mise run verify` は round 2 でも落ちるのが同じ 2 本だけ（環境起因の `verify-run.test.sh` と base 起因の `agent-team-config.test.sh`）で、一過性の失敗は無かった。
5. 改善案: `.agents/skills/command-team/SKILL.md` の「レビュー担当と effort」に「保留以外の理由（利用上限など）で担当が使えないときの扱い」を書く。割り当ての記録が古いと、実装担当が代用してよいかを判断する材料が無い。

使ったスキル: `.agents/skills/implement-issue`、`.agents/skills/mutation-testing`（手動の変異 17 種の生き残りの解釈に「What Results Mean」。差分がミューテーションテスト対象の crate に触らないため `mise run mutants:diff` は対象外）

## 実装者メモ（round 3）

対象 head: ff6d2393853ca6e51a2a510da8b236266ca69086

1. 詰まった箇所: 手順 8〜9: head 15f9f2fc のレビューで判定が割れた（Claude が approve、Codex が request-changes の must-fix 3 件）。Issue にそれぞれの要点をコメントし、修正側の must-fix を直す round にした。
2. 曖昧だった文: 「書き込み直前に本文と head を照合し、変わっていれば止めてください。API で照合と更新を一体にできない場合は、同時編集を避ける手順と残る競合の範囲を明記してください。」（Codex のレビュー run 7206801692 の must-fix）— この API が条件付き更新を受け付けるかは、どこにも書き方が無い。
3. 仮定と解釈: PR #840 に `If-Match` を付けた PATCH（本文は同じ内容）を試すと `HTTP 400`（`Conditional request headers are not allowed in unsafe requests unless supported by the endpoint`）が返ったため、照合と更新は一体にできないと判断した。書き込みの直前にもう一度読んで本文と head を照合し、書き込みの応答の head と本文も確かめる形にした。残る競合（読み直しから書き込みまでの間に重なった本文の編集は上書きされる）を道具の説明と skill implement-issue の手順 7 に書いた。「Markdown のコードの文脈」は、フェンス付きコードブロックの中を数えないことと、インラインコードの中の `<!--`・`-->` を注釈の記号にしないことと解した（sha を書く形がインラインコードのため、インラインコードの中身は数える）。Claude の印の無い nit（閉じ忘れの注釈の試験が、見出しの行ごと消しているため弱い）は、同じ処理を書き直す中で試験を強くした。
4. 足りなかった情報・検証手段: 手動の変異 26 種がすべて試験で倒せることを確かめた。最初は「書き込みの後に GitHub が返した本文を確かめない」と「注釈の中のインラインコードを空白に替えない」の 2 種が生き残ったので、試験を足した。`mise run verify` は round 3 でも落ちるのが同じ 2 本だけ（一過性の失敗は無し）だった。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 8 に「担当が使えるかは、割り当ての記録だけでなく `orchestrator_capabilities` の `canRunChildTask` と `constraints` でも確かめる。割り当ての記録の利用上限が古いことがあり、使えない担当は `Provider instance is disabled.` と出る」を足す。

使ったスキル: `.agents/skills/implement-issue`、`.agents/skills/mutation-testing`（手動の変異 26 種の生き残りの解釈に「What Results Mean」）
