#!/usr/bin/env bash
# Issue #724 (a): skill command-team の「Codex のモデル」の表で、
# security=yes のレビューに gpt-6.2-sol が選ばれる条件の必須文言を固定するテスト。
# 位置づけ: このスクリプトは「文言が表に存在する」という観測可能な事実だけを確かめる
# 決定的な検査（wording pin）であり、表から一意に読み取れるかという意味の評価はしない
# （AGENTS.md「LLM judges」: 意味の確認は PR 本文での引用と skill review-pr のレビューで行う）。
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

# 条件 (a): security=yes のレビューが gpt-6.2-sol の行に含まれている。
# 委託のたびに使う材料に security の有無が加わっている。
check a_materials .agents/skills/command-team/SKILL.md \
  'security-review を行うかどうか、レビューの回数'
check a_sol_row .agents/skills/command-team/SKILL.md \
  'security=yes のレビュー（Assigned 行に `security=yes` がある、または skill `review-pr` の手順 2 で security-review を行う条件のどれかに当たる）'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "codex-model-security-docs: ok"
