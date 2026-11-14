#!/usr/bin/env bash
# ボード（GitHub Projects）主導の処理の流れ（Issue #290）が正本 5 文書に落ちているかのテスト。
# 対象は prose の差分なので、各受け入れ条件を grep したときの必須文言で確かめる。
# 文言を変えるときはこのテストと同じ PR で直す。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
failures=()

# check <name> <file> <pattern...> — すべての pattern が file にあれば合格。
check() {
  local name="$1" file="$2"
  shift 2
  local p
  for p in "$@"; do
    if ! grep -qF "$p" "$ROOT/$file"; then
      failures+=("$name: $file に無い: $p")
    fi
  done
}

# 条件1: agent-team.md — 人間の操作は 3 つ（タスク定義・Todo への移動・マージの委任）、
# 入口A は 起票＋ready-for-agent で Backlog、人間が Todo へ動かしたものから割り当て。
check cond1_agent_team_human_ops docs/superpowers/specs/agent-team.md \
  '人間が行うのは次の 3 つだけ' \
  'Todo へ動か'
check cond1_agent_team_entry_a docs/superpowers/specs/agent-team.md \
  'Backlog' \
  'Todo のカード'

# 条件2: grokbot-runbook.md — Todo＋ready-for-agent が着手条件、ラベルは保持、
# Todo の並び順に割り当て、巡回の具体手順（導出規則・整合チェック・停止・再開・
# 枠の数え方と除外・補助記録の追加項目）が集約されている。
check cond2_runbook_start_condition docs/agents/grokbot-runbook.md \
  'ready-for-agent` は外さない' \
  '返却順'
check cond2_runbook_patrol docs/agents/grokbot-runbook.md \
  'ステータスの導出規則' \
  '整合チェック' \
  '停止処理' \
  '再開手順' \
  '作業状態の印'
check cond2_runbook_slots docs/agents/grokbot-runbook.md \
  '停止中・再開処理中・再開待ち'
check cond2_runbook_aux docs/agents/grokbot-runbook.md \
  '主 PR' \
  '戻り先' \
  '前回ステータス'

# 条件3: command-team — hatai の指摘の伝達に Changes requested への変更と対応済みの記録、
# 「hatai 待ち」を Ready to merge に置く規則、報告 PR に Part of #<番号> を書く義務。
check cond3_command_team_hatai .agents/skills/command-team/SKILL.md \
  'Changes requested' \
  '対応済み'
check cond3_command_team_hatai_wait .agents/skills/command-team/SKILL.md \
  'Ready to merge' \
  '[review-request-human]'
check cond3_command_team_report_pr .agents/skills/command-team/SKILL.md \
  '`Part of #<番号>`'

# 条件4: implement-issue — 委託・再委託のたびに対象 PR へ委託記録コメント、
# 終わりの報告と実装者メモに対象 head の SHA の明示。
check cond4_implement_issue_review_request .agents/skills/implement-issue/SKILL.md \
  '[review-request] head:'
check cond4_implement_issue_head .agents/skills/implement-issue/SKILL.md \
  '対象 head'

# 条件5: write-issue — ボードには ready-for-agent を付けたものだけが載る
# （サブ Issue の例外を含む）、Todo への移動が人間の合図。
check cond5_write_issue_board .agents/skills/write-issue/SKILL.md \
  'ready-for-agent' \
  'サブ Issue' \
  'Todo'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "board-patrol-docs: ok"
