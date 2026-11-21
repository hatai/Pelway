#!/usr/bin/env bash
# レビュースレッドの settle 手順（Issue #470）の skill と runbook の
# 必須文言を固定するテスト。
# 位置づけ: このスクリプトは「必須文言がskill と runbookに存在する」という観測可能な
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

CT=.agents/skills/command-team/SKILL.md
II=.agents/skills/implement-issue/SKILL.md
RB=docs/agents/grokbot-runbook.md

# 条件1: command-team に、指揮役が自分で立てたレビュースレッドの settle と、
# 次のラウンドに置き換わった前ラウンドのレビュースレッドの settle が書かれている。
check cond1_command_team "$CT" \
  '### レビュースレッドの settle' \
  'verdict を読んだら、指揮役が自分で立てたレビュースレッド' \
  '`t3_thread_organize`（`action: settle`）' \
  '次のラウンドのレビュースレッドを立てたら、置き換わった前ラウンドのレビュースレッドを settle する'

# 条件2: implement-issue に、PR を作るたびの全 PR 紐付け（link_pull_requests）と、
# 次のラウンドを委託したときの前ラウンドのレビュースレッド（子タスク）の settle が書かれている。
check cond2_implement_issue_link "$II" \
  'link_pull_requests' \
  'PR を作るたびに' \
  '最後の 1 つだけでなく'
check cond2_implement_issue_settle "$II" \
  '次のラウンドのレビューを委託したら、前ラウンドのレビュースレッド（子タスク）を settle する'

# 条件3: settle しない条件（needs-human 中・止まっている・迷走中）が両方の skill に書かれている。
check cond3_ct_no_settle "$CT" \
  '`needs-human` 中のスレッドと、止まっている・迷走中のスレッドは settle しない'
check cond3_ii_no_settle "$II" \
  '`needs-human` 中・止まっている・迷走中のときは settle しない'

# 条件4: runbook の片付けの settle の条件に、次のラウンドに置き換わった
# レビューの子タスクが入っている（skill に足す手順と runbook の対応付け。
# 条件は Issue #526 で種類の列挙から状態の条件に置き換わった）。
check cond4_runbook "$RB" \
  'レビューの子タスクで、次のラウンドの委託に置き換わったか、結果を読み終えたもの'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "settle-procedure-docs: ok"
