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

# 節に、新しいスレッド・子への依頼文に PR ブランチ名と fetch・push の形を入れることがある。
check handoff_instruction "$RB" \
  '新しいスレッドへの最初の指示と、修正 round の子への依頼文には、PR ブランチ名と、取り込み・push の形'

# runbook の再開手順の 5 と手当スレッドの項が新しい節を指す。
check resume_pointer "$RB" \
  'open な主 PR があるときはその PR ブランチで引き継ぐ（立て方は「4. 仕事の始め方」の「既存の PR ブランチを引き継ぐ」）'
check assist_pointer "$RB" \
  'その PR のブランチで起動する（立て方は「4. 仕事の始め方」の「既存の PR ブランチを引き継ぐ」）'

# 再開手順の 5 が、PR が無いときも引き継ぐ成果が残っていれば同じ節の形で
# 引き継ぎ、新規の立て方を引き継ぐ成果が無い場合に限定している。
check resume_no_pr_takeover "$RB" \
  '引き継ぐ成果が残っていれば同じ節の形でその作業ブランチを引き継ぐ' \
  '引き継ぐ成果が無いときだけ、実装スレッドの通常の立て方（「実装を始める」）で起動する'

# 再開手順の 5 が、取得コメントを pelway-dev[bot] 名義のものだけに限り、止まったスレッドの
# branch（別名 -r<N> でありうる）を使わず、特定できないときは確認待ちにする。
check claim_author "$RB" \
  '取得コメントは REST の `user.login` が `pelway-dev[bot]` のものだけを使い、ほかの作成者のコメントは読まない' \
  '止まったスレッドの `branch` は使わない' \
  '成果が無いと判断せず起動しない（作業状態の印を確認待ちにして hatai に報告する）'

# 節に、どの経路で得たブランチ名も agent/<N>- と英小文字・数字・ハイフンだけかを照合し、
# 外れたら止め、照合を通らない名前をコマンドに入れない決まりがある。
check branch_name_check "$RB" \
  '`agent/<N>-` で始まり、続きが英小文字・数字・ハイフンだけであることを確かめる。外れたら起動・委託を止める' \
  '照合を通らない名前は、成果の有無を確かめる git のコマンド・`t3 thread start`・`git branch --set-upstream-to`・push のどの行にも入れない'

# 引き継ぎ時に items の implement_thread へ PR 番号・thread_id・push 先の組を PR ごとに記録し、
# レビュー結果を送る直前に対象 PR の組で担当を確かめ、確定できなければ保留する決まりがある。
check implement_thread_record "$RB" \
  '手当てする PR の番号（PR がまだ無ければ `none`）・新しいスレッドの `thread_id`・push 先の `<PRブランチ>` の組を書く' \
  '組は PR ごとに持ち' \
  '`implement_thread` で、その PR の番号の組を選び' \
  'ちょうど 1 本のときだけ送って、その PR の組を書く。0 本か 2 本以上で担当を確定できないときは送らずに保留し' \
  'そのスレッドの `thread_id` で `implement_thread` の組を書き換える'

# 節に、起動・委託の前に PR の head リポジトリと ref を照合し、外れれば止める決まりがある。
check head_repo_guard "$RB" \
  '--json isCrossRepository,headRefName' \
  '`isCrossRepository` が `true`、または `headRefName` が `<PRブランチ>` と違うときは、起動・委託を止める'

# implement-issue の手順 6 に、別名ブランチのときは両方の push を HEAD:<PRブランチ> に
# 置き換える決まりがある。
check implement_push_target "$II" \
  '`scripts/agent-gh dev git push origin HEAD:<PRブランチ>` に置き換える'

# implement-issue の手順 9 に、修正 round を子に任せるときは runbook の節に従う 1 文がある。
check implement_issue_pointer "$II" \
  '修正 round を子に任せるときは、`docs/agents/grokbot-runbook.md` の「4. 仕事の始め方」の「既存の PR ブランチを引き継ぐ」の節に従う'

# command-team の「手当て・引き継ぎの前の確認」に、runbook の節の形で立てる 1 文がある。
check command_team_pointer "$CT" \
  'PR がすでにあり、そのブランチが `origin` にある引き継ぎ・再開・手当てのスレッドは、runbook の「4. 仕事の始め方」の「既存の PR ブランチを引き継ぐ」の節の形で立てる'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "takeover-worktree-docs: ok"
