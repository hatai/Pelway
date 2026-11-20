#!/usr/bin/env bash
# 審査系AIの実行環境の隔離原則（Issue #193）の技術仕様の
# 必須文言を固定するテスト。
# 位置づけ: このスクリプトは「必須文言が技術仕様に存在する」という観測可能な
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

# 条件1: 審査系AIの隔離原則（専用の読み取り中心の環境、被審査側の指示・スキル・記憶を
# 持ち込まない、持ち込む情報を列挙する）が技術仕様に明記されている。
check cond1_dedicated_env docs/technical.md \
  '審査専用' \
  '読み取り中心'
check cond1_no_context docs/technical.md \
  'プロジェクト指示' \
  'スキル' \
  '記憶' \
  '持ち込まない'
check cond1_enumerated_inputs docs/technical.md \
  '持ち込む情報'

# 条件2: 番人（#189）と承認の審査にこの原則が適用されることが書かれている。
check cond2_watchdog docs/technical.md \
  '番人' \
  '#189' \
  '承認'

# 出典の明記（採用する方式が Buzz 由来であること）。
check source_buzz docs/technical.md \
  'Buzz'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "review-isolation-docs: ok"
