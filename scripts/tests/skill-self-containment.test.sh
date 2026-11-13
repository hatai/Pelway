#!/usr/bin/env bash
# Issue の自己完結性の規則（Issue #253）がスキル本文に落ちているかのテスト。
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

# 条件1: write-issue の関連情報 — リポジトリ外の成果物は URL か判断に足る抜粋を必須。
check cond1_write_issue_external_artifact .agents/skills/write-issue/SKILL.md \
  'リポジトリ外の成果物（素案・調査文書など）を参照するときは、取得可能な URL か、判断に足る抜粋の引用を本文に残す'

# 条件2: write-issue の受け入れ条件の書き方 — 到達点の状態の欄。
check cond2_write_issue_endpoint_state .agents/skills/write-issue/SKILL.md \
  '到達点の状態を 1 行に含める' \
  'Draft PR の存在をもって完了とする' \
  '受け入れ条件に到達点の状態が書かれている'

# 条件3: implement-issue の停止条件の表に 2 行。
check cond3_implement_issue_stop_conditions .agents/skills/implement-issue/SKILL.md \
  'Issue が依存するリポジトリ外の成果物（関連情報の URL・抜粋）に到達できない' \
  'Issue の未決事項に挙がっておらず、避けて進めることもできず、受け入れ条件の成否に関わる判断が必要'

# 条件4: 根拠・事例には一次記録への引用を必須とし、裏付けられない内容は「想定」。
check cond4a_command_team_citation .agents/skills/command-team/SKILL.md \
  '引用で裏付けられない内容は「想定」と明記する'
check cond4b_investigate_bug_citation .agents/skills/investigate-bug/SKILL.md \
  '引用で裏付けられない内容は「想定」と明記する'
check cond4c_write_pr_citation .agents/skills/write-pull-request/SKILL.md \
  '引用で裏付けられない内容は「想定」と明記する'

# 条件5: command-team の手当て・引き継ぎの前の確認（適用範囲 (a)〜(d)、対象外の明記）。
check cond5_command_team_handling_gate .agents/skills/command-team/SKILL.md \
  '手当て・引き継ぎの前の確認' \
  '対象 Issue の受け入れ条件と到達点の状態' \
  '(a) PR の Draft 解除と Draft への復帰' \
  '(b) PR への main 取り込み（update-branch）と base の変更' \
  '(c) Issue のクローズと再オープン' \
  '(d) 受け入れ条件に触れる Issue 本文の変更' \
  'マージとレビュー投稿はこの確認の対象外'

if ((${#failures[@]})); then
  printf 'FAIL %s\n' "${failures[@]}"
  exit 1
fi
echo ok
