#!/usr/bin/env bash
# 既存の PR ブランチの引き継ぎ手順（Issue #644）の runbook と skill の
# 必須文言を固定するテスト。
# 位置づけ: このスクリプトは「必須文言が文書に存在する」という観測可能な
# 事実だけを確かめる決定的な検査（wording pin）であり、手順の意味や整合性は評価しない
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
    if ! grep -qF -- "$p" "$ROOT/$file"; then
      failures+=("$name: $file に無い: $p")
    fi
  done
}

CT=.agents/skills/command-team/SKILL.md
II=.agents/skills/implement-issue/SKILL.md
RB=docs/agents/grokbot-runbook.md

# runbook に「実装を始める」の後の節「既存の PR ブランチを引き継ぐ」がある。
check section_exists "$RB" \
  '### 既存の PR ブランチを引き継ぐ'

# 節に t3_thread_launch の workspaceStrategy の形がある。
check workspace_strategy "$RB" \
  '"type": "worktree", "baseRef": "origin/<PRブランチ>", "branch": "<PRブランチ>-r<N>", "startFromOrigin": true'

# 節に同じ内容の CLI の形がある。
check cli_form "$RB" \
  '--worktree origin/<PRブランチ> --branch <PRブランチ>-r<N>'

# 節に <N> の決め方がある。
check n_rule "$RB" \
  'ローカルのブランチと `origin` にある `<PRブランチ>-r<数>` の最大の数に 1 を足したもの（無ければ 1）'

# 節に upstream を向ける最初の作業と、push の形がある。
check upstream_and_push "$RB" \
  'git branch --set-upstream-to=origin/<PRブランチ>' \
  'scripts/agent-gh dev git push origin HEAD:<PRブランチ>'

# 節に、前のスレッドの作業ツリーへの bind は使わず新しい作業ツリーを作ることがある。
check no_existing_worktree "$RB" \
  '`existing_worktree`）は引き継ぎに使わない'

# 節に、修正 round の子は delegate_task（isolation: "worktree"）で立て、
# Claude の Agent ツールの worktree 隔離では立てないことがある。
check fix_round_child "$RB" \
  '`delegate_task`（`isolation: "worktree"`）で立て' \
  'Claude の Agent ツールの worktree 隔離では立てない'

# runbook の再開手順の 5 と手当スレッドの項が新しい節を指す。
check resume_pointer "$RB" \
  '同じブランチ／PR で新しいスレッドを起動して引き継ぐ（立て方は「4. 仕事の始め方」の「既存の PR ブランチを引き継ぐ」）'
check assist_pointer "$RB" \
  'その PR のブランチで起動する（立て方は「4. 仕事の始め方」の「既存の PR ブランチを引き継ぐ」）'

# implement-issue の手順 9 に、修正 round を子に任せるときは runbook の節に従う 1 文がある。
check implement_issue_pointer "$II" \
  '修正 round を子に任せるときは、`docs/agents/grokbot-runbook.md` の「4. 仕事の始め方」の「既存の PR ブランチを引き継ぐ」の節に従う'

# command-team の「手当て・引き継ぎの前の確認」に、runbook の節の形で立てる 1 文がある。
check command_team_pointer "$CT" \
  '引き継ぎ・再開・手当てのスレッドは、runbook の「4. 仕事の始め方」の「既存の PR ブランチを引き継ぐ」の節の形で立てる'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "takeover-worktree-docs: ok"
