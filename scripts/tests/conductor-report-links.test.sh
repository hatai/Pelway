#!/usr/bin/env bash
# 指揮役の人間あて報告での Issue・PR 番号リンク化（Issue #303）の正本の
# 必須文言を固定するテスト。
# 位置づけ: このスクリプトは「必須文言が正本に存在する」という観測可能な
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

CT=.agents/skills/command-team/SKILL.md
RB=docs/agents/grokbot-runbook.md

# 条件1: command-team にルールが明記されていること（対象メッセージの範囲、
# Issue と PR のリンク形式の例、GitHub の自動リンクに頼らない旨、
# 繰り返し出る場合もその都度リンクにする旨）。
check cond1_rule "$CT" \
  '## 人間あての報告・連絡のリンク' \
  'マージ報告' \
  '巡回・ダイジェスト報告' \
  'エスカレーション' \
  '引き継ぎ報告' \
  '判断を求める' \
  '[#<番号>](https://github.com/hatai/Pelway/issues/<番号>)' \
  '[PR #<番号>](https://github.com/hatai/Pelway/pull/<番号>)' \
  'GitHub の自動リンクに頼らない' \
  'その都度リンクにする'

# 条件2: command-team と、報告の例文（runbook のまとめ）に、素の #<番号> 表記の
# 人間あて報告の例が残っていないこと。リンク形式に揃っていることを確認する。
if ! grep -qF '[#80](https://github.com/hatai/Pelway/issues/80)' "$ROOT/$RB"; then
  failures+=("cond2_example: $RB に無い: まとめの例文が Issue のリンク形式になっていない")
fi
if ! grep -qF 'マージ待ち: [PR #77](https://github.com/hatai/Pelway/pull/77)' "$ROOT/$RB"; then
  failures+=("cond2_example: $RB に無い: まとめの例文が PR のリンク形式になっていない")
fi
# まとめの例文の中に、URL を伴わない素の番号表記が残っていないこと。
if sed -n '/【まとめ/,/^```$/p' "$ROOT/$RB" | grep -nE '#[0-9]+' | grep -v 'github.com/hatai/Pelway' | grep -q .; then
  failures+=("cond2_example: $RB のまとめの例文に素の番号表記が残っている")
fi

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "conductor-report-links: ok"
