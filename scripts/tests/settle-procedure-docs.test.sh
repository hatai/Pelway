#!/usr/bin/env bash
# レビュースレッドの settle 手順（Issue #470）が skill と runbook に落ちているかのテスト。
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

# 条件4: runbook の片付けの列挙に、次のラウンドに置き換わった前ラウンドの
# レビュースレッドが入っている（skill に足す手順と runbook の対応付け）。
check cond4_runbook "$RB" \
  '次のラウンドに置き換わった前ラウンドのレビュースレッド'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "settle-procedure-docs: ok"
