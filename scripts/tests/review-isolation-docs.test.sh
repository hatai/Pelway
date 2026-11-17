#!/usr/bin/env bash
# 審査系AIの実行環境の隔離原則（Issue #193）が技術仕様に落ちているかのテスト。
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
