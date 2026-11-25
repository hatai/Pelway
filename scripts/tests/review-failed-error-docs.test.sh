#!/usr/bin/env bash
# Issue #724 (b): skill review-pr の verdict: failed の報告で、error: 行に
# 失敗したコマンドとエラー（または満たせなかった手順の完了基準）を必ず書かせる
# 必須文言を固定するテスト。
# 位置づけ: このスクリプトは「文言が手順に存在する」という観測可能な事実だけを
# 確かめる決定的な検査（wording pin）であり、報告の形として意味が通るかの評価はしない
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

# 条件 (b)-1: コマンドが失敗していないときの書き方が決まっている。
check b_no_failed_command .agents/skills/review-pr/SKILL.md \
  'コマンドが失敗していないのに手順を完了できないとき'
check b_no_failed_command_form .agents/skills/review-pr/SKILL.md \
  '（失敗したコマンドなし）'

# 条件 (b)-2: 失敗したコマンドを書かせる決まりが error: の行にある。
check b_error_line .agents/skills/review-pr/SKILL.md \
  'error: <コマンド> → <エラーの要点 1 行>'

# 条件 (b)-3: failed の報告の形の例が更新されている（コマンド失敗の例と、
# 失敗したコマンドなしの例の両方）。
check b_example_command .agents/skills/review-pr/SKILL.md \
  'error: scripts/agent-gh review pr diff <N> -- <パス> → accepts at most 1 arg(s), received 2'
check b_example_no_command .agents/skills/review-pr/SKILL.md \
  'error: 手順 <番号> <節の名前>: <満たせなかった完了の基準>（失敗したコマンドなし）'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "review-failed-error-docs: ok"
