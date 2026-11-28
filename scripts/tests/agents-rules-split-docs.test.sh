#!/usr/bin/env bash
# AGENTS.md が「規則と既定」「変えてはいけない規則」「既定」「案内」の 4 つの節に
# 分かれていることを固定するテスト（Issue #797）。
# 位置づけ: このスクリプトは「節の見出しが存在する」という観測可能な事実だけを確かめる
# 決定的な検査（presence check）であり、各決まりの分類の意味や分類基準の内容は評価しない
# （AGENTS.md「LLM judges」: 自然言語の意味の評価は rubric 付きの LLM judge が担い、
# 決定的な検査は観測可能な効果に限る。意味の確認は PR 本文での引用とレビューで行う）。
# 節の名前を変えるときはこのテストと同じ PR で直す。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
failures=()

for heading in '## 規則と既定' '## 変えてはいけない規則' '## 既定' '## 案内'; do
  if ! grep -qxF -- "$heading" "$ROOT/AGENTS.md"; then
    failures+=("heading_missing: AGENTS.md に行「${heading}」が無い")
  fi
done

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "agents-rules-split-docs: ok"
