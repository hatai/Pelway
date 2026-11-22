#!/usr/bin/env bash
# 同じ種類の must-fix を全経路まとめて扱う手順と、push 前に PR ブランチの先端を取り込む手順
# （Issue #614）の文言を固定するテスト。
# 位置づけ: 文言がスキルに存在することだけを確かめる決定的な検査（wording pin）であり、
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

# 実装担当: 同じ種類の must-fix が 2 round 続いたら、全経路に当てる決まりを決めて直す（手順 9）。
check implementer_same_kind_rule .agents/skills/implement-issue/SKILL.md \
  '同じ種類の must-fix が 2 round 続いたら' \
  'その種類の全経路に当てる決まりを決めてから直し、決めた決まりを実装者メモの「仮定と解釈」に書く'

# レビュー担当: must-fix を 1 件見つけたら、差分の中の同じ種類の経路をすべて洗う（手順 3）。
check reviewer_same_kind_sweep .agents/skills/review-pr/SKILL.md \
  'must-fix を 1 件見つけたら、差分の中の同じ種類の経路をすべて洗い、見つけた分を同じレビューの must-fix に書く'

# 実装担当: push の前に PR ブランチの remote の先端を fetch し、進んでいれば fast-forward で取り込む（手順 6）。
check implementer_ff_branch_tip .agents/skills/implement-issue/SKILL.md \
  'scripts/agent-gh dev git fetch origin <ブランチ>' \
  'scripts/agent-gh dev git merge --ff-only origin/<ブランチ>' \
  'Kodiak が main をマージして PR ブランチの remote の先端が進んでいることがある' \
  '修正を始める前、作業ツリーに未コミットの変更が無いうちに、PR ブランチの remote の先端を fetch し、進んでいれば fast-forward で取り込む' \
  '初回（remote にブランチがまだ無い）はこの段を飛ばす' \
  'push の前の段で PR ブランチの remote の先端をもう一度 fetch し'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "same-kind-must-fix-docs: ok"
