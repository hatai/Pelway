#!/usr/bin/env bash
# pr-size 上限超過時の止まり方（Issue #806）の文言を固定するテスト。
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

# check_absent <name> <file> <pattern> — pattern が file に無ければ合格。
check_absent() {
  local name="$1" file="$2" pattern="$3"
  if grep -qF -- "$pattern" "$ROOT/$file"; then
    failures+=("$name: $file に残っている: $pattern")
  fi
}

# 実装担当は上限超過を push せず、分割案・手元の head・diffstat を Issue に書いて止まる。
check implement_stop_row .agents/skills/implement-issue/SKILL.md \
  'push せずに止まる' \
  '手元の head の SHA' \
  '`git diff --stat origin/main...HEAD` の出力'
check implement_step6 .agents/skills/implement-issue/SKILL.md \
  '上限を超えたときは、例外が認められる前に push しない' \
  '`scripts/agent-gh-pr-size-gate` は例外の無い上限超過の push を止める'
check_absent implement_no_consult_push .agents/skills/implement-issue/SKILL.md \
  '相談のため'

# 例外が認められたあとは、ラベルとコメントを確かめてから push し、メモと必須チェックを経て手順 8 へ戻る。
check implement_exception_resume .agents/skills/implement-issue/SKILL.md \
  '例外が認められたあとに再開するときは' \
  'PR に `pr-size-exception` ラベルがあり' \
  'hatai の行頭が `<!-- pr-size-exception -->` のコメントがあること' \
  '必須チェックを確かめて手順 8 から再開する'

# 指揮役側の分割と例外の節も、push せずに止まる実装担当の止まり方に合わせる。
check command_team_exception .agents/skills/command-team/SKILL.md \
  '実装担当は例外が認められる前の上限超過の差分を push せずに止まる' \
  'PR がまだ無いときは例外を選ばず分割を指示する'
check command_team_split .agents/skills/command-team/SKILL.md \
  '分割案・手元の head の SHA・`git diff --stat origin/main...HEAD` の出力'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "pr-size-stop-docs: ok"
