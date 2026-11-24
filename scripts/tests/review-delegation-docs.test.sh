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
check fail_closed_review_pr .agents/skills/review-pr/SKILL.md \
  '自分に渡されたツール名の一覧を返答に記録' \
  '設定の不備: ツール名の一覧が読めない'
check fail_closed_implement_issue .agents/skills/implement-issue/SKILL.md \
  'fail-closed のツール一覧チェック' \
  '書き込み系ツールが含まれる、または一覧が読めないときは何もせず終了する' \
  '結果を採用しない'
check fail_closed_command_team .agents/skills/command-team/SKILL.md \
  'fail-closed のツール一覧チェック' \
  '書き込み系ツールが含まれる、または一覧が読めないときは何もせず終了する' \
  '結果を採用しない'
check fail_closed_runbook docs/agents/grokbot-runbook.md \
  'fail-closed のツール一覧チェック' \
  '書き込み系ツールが含まれる、または一覧が読めないときは何もせず終了する' \
  '結果を採用しない'
check fail_closed_spec docs/superpowers/specs/agent-team.md \
  'fail-closed のツール一覧チェック' \
  '結果を採用しない'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "review-delegation-docs: ok"
