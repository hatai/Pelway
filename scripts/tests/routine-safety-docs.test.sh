#!/usr/bin/env bash
# Issue #194 で足した PRD↔ADR の相互参照の構造検査（ADR の実在と PRD 相対リンクの解決）。
# 仕様の内容（制限値・通知方法・AI の振る舞い）の成否は Issue #194 の受け入れ条件どおり
# 目視確認と skill review-pr のレビューが担う。自然言語の意味をキーワード一致や正規表現で
# 採点することは AGENTS.md の「LLM judges」が禁じるため、このテストでは行わない。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
failures=()

# 条件1（構造）: 判断の記録 ADR 0011 が実在する。
[[ -f "$ROOT/docs/adr/0011-routine-safety-limits.md" ]] ||
  failures+=("adr_0011: docs/adr/0011-routine-safety-limits.md が存在しない")

# 条件2（構造）: docs/prd.md の相対 markdown リンクがすべて解決する
# （本 PR で足した adr/0011・adr/0010 への参照を含む。ドキュメントのリンク整合は
# docs/technical/v0.7 側は verify.py が検査するが、prd.md はここ以外に検査が無いため）。
while IFS= read -r link; do
  if [[ ! -e "$ROOT/docs/$link" ]]; then
    failures+=("prd_link: docs/prd.md のリンク先が無い: $link")
  fi
done < <(grep -oE '\]\([^)#]+\.md' "$ROOT/docs/prd.md" | sed 's/](//' | sort -u)

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "routine-safety-docs: ok"
