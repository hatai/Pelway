#!/usr/bin/env bash
# Issue #202 で足した本番トポロジ ADR の構造検査（ADR の実在と docs/technical.md 相対リンクの解決）。
# トポロジの内容（構成要素と配置・国内前提・堅牢化方針）の成否は Issue #202 の受け入れ条件どおり
# 目視確認と skill review-pr のレビューが担う。自然言語の意味をキーワード一致や正規表現で
# 採点することは AGENTS.md の「LLM judges」が禁じるため、このテストでは行わない。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
failures=()

# 条件1（構造）: 本番トポロジの判断の記録 ADR 0012 が実在する。
[[ -f "$ROOT/docs/adr/0012-production-topology-single-host.md" ]] ||
  failures+=("adr_0012: docs/adr/0012-production-topology-single-host.md が存在しない")

# 条件2（構造）: docs/technical.md の相対 markdown リンクがすべて解決する
# （本 PR で足した adr/0012 への参照を含む。http(s) の絶対 URL はこの検査の対象外。
# ドキュメントのリンク整合は docs/technical/v0.7 側は verify.py が検査するが、
# technical.md はここ以外に検査が無いため）。
while IFS= read -r link; do
  if [[ ! -e "$ROOT/docs/$link" ]]; then
    failures+=("technical_link: docs/technical.md のリンク先が無い: $link")
  fi
done < <(grep -oE '\]\([^)#]+\.md' "$ROOT/docs/technical.md" | sed 's/](//' | grep -vE '^https?://' | sort -u)

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "production-topology-docs: ok"
