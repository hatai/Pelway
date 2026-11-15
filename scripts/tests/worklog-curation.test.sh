#!/usr/bin/env bash
# 作業ログの定期キュレーション（Issue #308）が正本に落ちているかのテスト。
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
RB=docs/agents/grokbot-runbook.md

# 条件1: command-team の改善ループに、定期キュレーションの位置づけが書かれていること
# （レビュー指摘を入口とする既存ループとの役割分担、提案までで正本を直接編集しないこと）。
check cond1_positioning "$CT" \
  '### 定期キュレーション' \
  'レビュー指摘' \
  '候補' \
  'Issue 化を人間に提案' \
  '直接編集しない'

# 条件2: runbook に実行手順があること（頻度＝週1回、対象＝worklog 群、
# 観点＝繰り返しの型・正本と実態のずれ・長期間動いていない待ち、出力＝worklog への候補リスト）。
check cond2_procedure "$RB" \
  '## 12. 作業ログの定期キュレーション' \
  '週 1 回' \
  'worklog 群' \
  '繰り返し' \
  '正本' \
  '長期間動いていない待ち' \
  '候補リスト'

# 条件3: 初回の走査が実施され、候補リスト（候補が無い場合はその旨）が
# worklog に残っていること（docs/worklogs/<日付>-curation.md）。
curation_file=""
for f in "$ROOT"/docs/worklogs/*-curation.md; do
  [[ -e "$f" ]] || continue
  if grep -q '候補' "$f"; then
    curation_file="$f"
    break
  fi
done
if [[ -z "$curation_file" ]]; then
  failures+=("cond3_scan: docs/worklogs/ に候補リストを含む <日付>-curation.md が無い")
fi

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "worklog-curation: ok"
