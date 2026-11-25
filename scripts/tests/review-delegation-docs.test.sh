#!/usr/bin/env bash
# レビューと委託の手順（Issue #594）の文言を固定するテスト。
# 位置づけ: 文言が文書に存在することだけを確かめる決定的な検査（wording pin、存在の確認）であり、
# 手順の意味や整合性は評価しない（AGENTS.md「LLM judges」。意味の確認は PR 本文での引用と
# skill review-pr のレビューで行う）。文言を変えるときはこのテストと同じ PR で直す。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
failures=()

# check <name> <file> <pattern...> — すべての pattern が file にあれば合格。
check() {
  local name="$1" file="$2"
  shift 2
  local p
  for p in "$@"; do
    if ! grep -qF -- "$p" "$ROOT/$file"; then
      failures+=("$name: $file に無い: $p")
    fi
  done
}

# 2 回目以降のレビューは前回の sha からの差分と前回の must-fix を見る。差分の外は nit か PR 外の所見。
check review_rereview_scope .agents/skills/review-pr/SKILL.md \
  '前回レビューした sha（`Reviewed:` の行）から今の head までの差分と、前回の must-fix が直ったかを見る' \
  '差分の外で新しく見つけたことは nit か PR 外の所見にする。must-fix にしてよいのは、この PR が入れた正しさ・安全性の欠陥だけ'

# PR の範囲外の安全上の所見は PR に書かず、委託者への返答に書く。
check review_out_of_scope .agents/skills/review-pr/SKILL.md \
  'PR の範囲外の安全上の所見は PR に書かず、委託者への返答（手順 5 の `out-of-scope:` の行）に書く'

# 最初の手順で実行環境を確かめる。
check review_runtime_check .agents/skills/review-pr/SKILL.md \
  '### 1. 実行環境を確かめ、材料を集める' \
  '`verdict: failed`、`error: wrong runtime` で報告して止まる'

# チェックの完了を待ってからレビューする。
check review_waits_checks .agents/skills/review-pr/SKILL.md \
  '`checks pending` で失敗を返さず、skill `pr-watch` の「待ち方」で待ってからレビューする' \
  'error: checks timeout'
check pr_watch_reviewer_row .agents/skills/pr-watch/SKILL.md \
  'レビュー担当（skill `review-pr`。委託の子もトップレベルのスレッドも）' \
  'scripts/agent-gh review pr checks <N> --required --watch --interval 30'
check implementer_checks_timeout .agents/skills/implement-issue/SKILL.md \
  '委託した子が `error: checks timeout` を報告したときも、同じラウンドとして数えずに完了を待って再委託する'
check runbook_checks_timeout docs/agents/grokbot-runbook.md \
  '`error: checks timeout` なら'

# 実装担当は PR 外の所見を終わりの報告に書き、公開の Issue を立てない。
check implementer_out_of_scope .agents/skills/implement-issue/SKILL.md \
  'PR 外の所見を受けたら、終わりの報告（指揮役か hatai あて）の `out-of-scope:` の行に書き、公開の Issue は立てない'

# delegate_task の呼び出しを丸ごと写せるブロック。
check implementer_delegate_block .agents/skills/implement-issue/SKILL.md \
  '"runtimeMode": "auto-accept-edits"' \
  '"isolation": "worktree"' \
  '"role": "review"' \
  '"providerInstanceId": "<codex_review|claude_review|devin_review>"'
check conductor_delegate_block .agents/skills/command-team/SKILL.md \
  '"runtimeMode": "auto-accept-edits"' \
  '"isolation": "worktree"' \
  '"role": "review"' \
  '"providerInstanceId": "<codex_review|claude_review|devin_review>"'

# 人間のアカウントで投稿する道具で hatai の判断を書く条件。
check agents_human_account AGENTS.md \
  '人間のアカウントで投稿する道具（T3 の `comment_on_issue` など）で hatai の判断を書くのは、hatai がその文面か選択肢を明示して選んだときだけ'

# レビュー委託の標準手順に fail-closed のツール一覧チェックが明記されている（Issue #581）。
# チェックに失敗したレビュー（error: が 設定の不備: で始まる verdict: failed）は結果を採用しない。
# 投稿済みのレビューも「有効な Approve」に数えない（投稿の後に 設定の不備: で失敗を報告したレビュー担当の投稿）。
check fail_closed_review_pr .agents/skills/review-pr/SKILL.md \
  '自分に渡されたツール名の一覧を最後の返答より前のメッセージに記録' \
  '設定の不備: ツール名の一覧が読めない'
check fail_closed_implement_issue .agents/skills/implement-issue/SKILL.md \
  'fail-closed のツール一覧チェック' \
  '書き込み系ツールが含まれる、または一覧が読めないときは何もせず終了する' \
  '結果を採用しない' \
  '投稿済みのレビュー'
check fail_closed_command_team .agents/skills/command-team/SKILL.md \
  'fail-closed のツール一覧チェック' \
  '書き込み系ツールが含まれる、または一覧が読めないときは何もせず終了する' \
  '結果を採用しない' \
  '投稿したレビュー担当が後から `error:` が `設定の不備:` で始まる `verdict: failed` を報告したレビューは' \
  '投稿済みのレビューも'
check fail_closed_runbook docs/agents/grokbot-runbook.md \
  'fail-closed のツール一覧チェック' \
  '書き込み系ツールが含まれる、または一覧が読めないときは何もせず終了する' \
  '結果を採用しない' \
  '投稿済みのレビュー'
check fail_closed_spec docs/superpowers/specs/agent-team.md \
  'fail-closed のツール一覧チェック' \
  '結果を採用しない' \
  '投稿済みのレビュー'

# 投稿後の設定不備は [review-invalid] の記録として PR に残し、実装担当の再開手順と
# 指揮役の承認判定の両方で照合する（Issue #581 r2）。記録として数えるのは pelway-dev[bot] 作のコメントだけで、
# 記録するレビューは本文の Reviewer: agent= の行で失敗した担当のもの（と担当を見分けられないもの）を id ごとに選ぶ。
# 未解消の記録が 1 件でもあるあいだは、その PR のどの Approve も数えない（Issue #581 r3）。
# いずれも文言の存在確認で、意味はレビューで引用を読んで判定する。
check fail_closed_record_implement .agents/skills/implement-issue/SKILL.md \
  '[review-invalid] head: <SHA> review:' \
  'pullrequestreview-<id> to: <担当>' \
  '投稿後の `設定の不備:` の記録' \
  'REST の `user.login` が `pelway-dev[bot]` のコメントだけ' \
  'run: ($last | capture(" run=(?<id>[0-9]+)$").id // null)' \
  '未解消の記録が PR に 1 件でもあるあいだは、その PR のどのレビューも'
check fail_closed_record_command_team .agents/skills/command-team/SKILL.md \
  '[review-invalid]' \
  '`pelway-dev[bot]` だけ' \
  'その PR のどの Approve も有効な Approve に数えない'
check fail_closed_record_runbook docs/agents/grokbot-runbook.md \
  '[review-invalid]' \
  '`pelway-dev[bot]` のコメントだけ' \
  'その PR のどの Approve も有効な Approve に数えず'

# 担当は本文の末尾行だけで見分け、対象のレビューは委託の時刻で絞る。投稿を特定できないときも
# review: unknown の記録を残し、PR に needs-human を付ける。解消は hatai の手での dismiss で確かめ、
# 解消の後の再レビューは解消より後の委託記録の後のレビューだけを数える。Guard (b) は DISMISSED を数えない。
# 結果の確定前の投稿はキュー投入の承認に使わず、記録を見つけた PR の automerge は外す（Issue #581 r5）。
# いずれも文言の存在確認で、意味はレビューで引用を読んで判定する。
check fail_closed_r5_implement .agents/skills/implement-issue/SKILL.md \
  'レビューの担当は本文の最後の空でない行（末尾行' \
  '.submitted_at >= "<T0>"' \
  '投稿前と確認' \
  '`review: unknown` の記録を 1 つ残す' \
  'scripts/agent-gh dev pr edit <M> --add-label needs-human' \
  'by_push: (.dismissed_review.dismissal_commit_id != null)' \
  'すべての記録の解消の時刻より後に残した委託記録'
check fail_closed_r5_command_team .agents/skills/command-team/SKILL.md \
  'その SHA に向いた `DISMISSED` でないレビュー' \
  'その Approve を投稿したレビューの委託の結果が確定している' \
  'キュー投入の承認に使わない'
check fail_closed_r5_runbook docs/agents/grokbot-runbook.md \
  'Kodiak は PR の `needs-human` でマージを止める' \
  '記録を見つけた PR に `automerge` が付いていれば、指揮役がすぐ外す' \
  'その定義をそのまま当てて判定する'

# 委託記録は起動の前に残し、そのコメント ID を割り当ての行の run= に書いて、投稿の末尾行の run= で実行に結び付ける。
# 対応を確かめられない失敗は review: unknown に進み、unknown と dismiss で確かめられない記録は hatai の解消コメントでだけ解消する。
# マージと終わりの報告は、現在の head の投稿のすべてが報告を受けた実行の投稿であることで結果の確定を確かめる（Issue #581 r6）。
# いずれも文言の存在確認で、意味はレビューで引用を読んで判定する。
check fail_closed_r6_implement .agents/skills/implement-issue/SKILL.md \
  '`delegate_task` を呼ぶ前に、対象 PR に委託記録コメントを残す' \
  'security=yes run=<委託記録のコメント ID>' \
  '実行の投稿: 委託記録 `<ID>` の実行の投稿とは' \
  '`<T0>` を同じ担当の別の委託記録や推測で補わず' \
  '[review-invalid-resolved] <記録のコメントの URL>' \
  '解消コメントで解消した記録では、記録の `head:` によらず現在の head とする' \
  'review: <ラウンド数> → approve（run <ID>[, <ID>...]）'
check fail_closed_r6_review_pr .agents/skills/review-pr/SKILL.md \
  '割り当ての行の末尾の `run=<ID>`（委託記録のコメント ID）も、そのまま写す' \
  'last == "<Reviewer 行>"'
check fail_closed_r6_command_team .agents/skills/command-team/SKILL.md \
  '結果の報告を受けた実行の投稿' \
  '`[review-invalid-resolved]`'
check fail_closed_r6_runbook docs/agents/grokbot-runbook.md \
  '立てる前に、対象 PR に委託記録' \
  '最新のレビューでは照合しない'

# 失敗した実行の投稿は末尾行の run= で選び、判別できない投稿は review: unknown に進める。解消コメントは
# 記録より後に作られ編集されていないものだけを数える。結び付かない投稿（run= の無い従来の書式を含む）は
# hatai の dismiss を待って再委託する。結果の確定は、委託の時点で決まる必要な実行のすべての報告と投稿で
# 確かめ、指揮役のレビュースレッドの結果は run <ID>: <verdict> を添えて実装スレッドに渡す（Issue #581 r7）。
# いずれも文言の存在確認で、意味はレビューで引用を読んで判定する。
check fail_closed_r7_implement .agents/skills/implement-issue/SKILL.md \
  '担当と時刻だけでは分けない' \
  '対応を判別できないもの' \
  '`created_at` と `updated_at` が同じ（投稿の後に編集されていない' \
  '`created_at` が対象の記録の `created_at` より後' \
  '結び付かない投稿の取り消し: ' \
  '必要な実行: ある head のレビューに要る実行の集合は、委託の時点で決まる' \
  '`run <ID>: <approve|request-changes>` の行が添えてあれば' \
  '投稿の無い記録は飛ばす'
check fail_closed_r7_command_team .agents/skills/command-team/SKILL.md \
  '(b) は、末尾行に ` run=<ID>` の無い投稿' \
  '必要な実行のうち報告の無いもの'
check fail_closed_r7_runbook docs/agents/grokbot-runbook.md \
  '最初の `approve` で次に進まず残りの報告を待つ' \
  'run 7140000001: approve' \
  '上の 6 の In review の判定では、その PR の最新の委託記録コメントだけを証跡とする'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "review-delegation-docs: ok"
