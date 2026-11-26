# Issue #809 の作業ログ

## 実装者メモ（round 1）

対象 head: 89050f562404312cff45360454158df75c7f5959

1. 詰まった箇所: 手順 6: `scripts/agent-gh dev git merge --ff-only origin/main` が `.devin/config.json` の `Read-only file system` で途中まで進んで止まった（sandbox が worktree の `.devin/` を読み取り専用にする。main の #737 がこのファイルを 1 行変えた）。書き換わっていた 10 ファイルと増えた 6 ファイルを origin/main と照合してから試みの前（7c36a927 + この Issue の変更）に戻し、この round は main を取り込まずに進めた。`mise run verify` はこの作業環境では 2 本落ちる（`.agents/skills/verify-pelway/verify-run.test.sh` の 6 項目は `/tmp` の所有者が自分でないため。`scripts/tests/agent-team-config.test.sh` の試験 3 は、base の 7c36a927 で hatai の `Update agent-team.toml` に試験の pins が追いついていないため）。`scripts/red-check HEAD` で変更を退避しても同じ失敗が出ること、後者は `scripts/red-check origin/main` では通ることを確かめた。
2. 曖昧だった文: 「PR 番号を受け取り、PR 本文の決まった行（「対象」の HEAD、「レビューと QA」の最後に検証した sha）を、push 後の `headRefOid` と指定した sha に書き換えるコマンドがある」（Issue #809 の受け入れ条件）。「決まった行」の形はどこにも定義がなく、実際の本文は揺れている（PR #735 の「対象」は `` `<branch>` @ `<sha>` `` で HEAD の語が無い。#743・#744 は `- HEAD：`。#722 は HEAD の行にほかの sha の補足がある）。
3. 仮定と解釈: 「決まった行」は、節の中に印（`最後に検証した sha`、単語としての `HEAD`）を含む行がちょうど 1 つあることとし、sha は印の直後の 16 進数（7〜40 桁）とした。見つからない・複数あるときは書き換えずに非 0 で終わる。「40 桁の sha を含む唯一の行」とする解釈は、補足の sha と紛れるため選ばなかった。道具は `scripts/agent-gh` のサブコマンドではなく独立のスクリプトにした（`scripts/agent-gh` を変えると手順 8 でレビューを委託できなくなるため）。`--verified` は形式だけを検査し、PR の head の祖先かは確かめない。GitHub との読み書きは `scripts/agent-gh dev api` に統一した（`pr edit --body-file` が本文を加工しないことを手元で確かめられないため）。
4. 足りなかった情報・検証手段: 本物の GitHub への PATCH は、試験では擬似の agent-gh でしか確かめられない（PR 作成後に自分の PR で確かめ、結果は PR 本文に書く）。手確認で `cd /` から本体を走らせようとして外部ディレクトリの許可を開発リードに拒まれた。以後は確認環境を `/tmp/opencode/809/` 以下に置き、`TMPDIR=/tmp/opencode/809` で試験を走らせた。ほかに、`.agents/skills/write-pull-request/SKILL.md` の「定期ルール」の許可一覧に `gh api` が無いが、手順 3 とこの道具は `api` を使う。
5. 改善案: `.github/pull_request_template.md` の「レビューと QA」と「対象」のコメントに ``最後に検証した sha: `<sha>` `` と ``HEAD: `<sha>` `` の行を書くよう注記すると、初回の本文から道具が読める形になる。`.agents/skills/implement-issue/SKILL.md` の「決まり（毎回守る）」に「sandbox 下の作業ツリーは `.devin/` が読み取り専用のため、`.devin/` を変えた main を取り込めないことがある。取り込めないときはそのまま push し、衝突は引き継ぎの作業ツリーで解消する」を足す。

使ったスキル: `.agents/skills/implement-issue`、`.agents/skills/diagnosing-bugs`（`mise run verify` の失敗の切り分けに Phase 2。`scripts/red-check HEAD`・`origin/main` で変更を退避して再現した。1 回だけ現れた `pelway-agent-watch` の `send_deadline_covers_secrets_and_curl`（1 回目）と `scripts/tests/conductor-rebuild.test.sh` の「term-ignoring read killed: returns after the grace」（3 回目）は、単独実行とほかの回では通ったので一過性）
