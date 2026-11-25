#!/usr/bin/env bash
# ボード（GitHub Projects）主導の処理の流れ（Issue #290）の正本 5 文書の
# 必須文言を固定するテスト。
# 位置づけ: このスクリプトは「必須文言が正本 5 文書に存在する」という観測可能な
# 事実だけを確かめる決定的な検査（wording pin）であり、受け入れ条件の意味や仕様の整合性は評価しない
# （AGENTS.md「LLM judges」: 自然言語の意味の評価は rubric 付きの LLM judge が担い、
# 決定的な検査は観測可能な効果に限る。語句が残っていれば矛盾した記述でも通るため、
# 意味の確認は PR 本文での引用と skill review-pr のレビューで行う）。
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

# 条件3: command-team の条件つき詳細文書 — hatai の指摘の伝達に Changes requested への変更と
# 対応済みの記録、「hatai 待ち」を Ready to merge に置く規則、報告 PR に Part of #<番号> を書く義務。
check cond3_command_team_hatai docs/agents/command-team-details.md \
  'Changes requested' \
  '対応済み'
check cond3_command_team_hatai_wait docs/agents/command-team-details.md \
  'Ready to merge' \
  '[review-request-human]'
check cond3_command_team_report_pr docs/agents/command-team-details.md \
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

# 条件6（Issue #715）: 上限と登録情報の正本が agent-team.toml であることが、
# 対象の 5 文書のそれぞれに参照として書かれている。
check cond6_toml_ref_spec docs/superpowers/specs/agent-team.md \
  'agent-team.toml'
check cond6_toml_ref_runbook docs/agents/grokbot-runbook.md \
  'agent-team.toml'
check cond6_toml_ref_command_team .agents/skills/command-team/SKILL.md \
  'agent-team.toml'
check cond6_toml_ref_implement_issue .agents/skills/implement-issue/SKILL.md \
  'agent-team.toml'
check cond6_toml_ref_details docs/agents/command-team-details.md \
  'agent-team.toml'

# 条件6b: command-team-details の調査の数え方に、停止中・再開処理中・再開待ちの
# 除外と agent:devin による内訳がある。
check cond6b_details_count docs/agents/command-team-details.md \
  '停止中・再開処理中・再開待ち' \
  'agent:devin'

# 条件6c: runbook の「3. 同時数と順番待ち」で、実装・調査の枠の使用数は
# watch.log の SLOTS の行で見る形になっている。
check cond6c_runbook_slots docs/agents/grokbot-runbook.md \
  '`SLOTS`' \
  '[developers.<login>.slots]'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "board-patrol-docs: ok"
