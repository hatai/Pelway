#!/usr/bin/env bash
# モデル利用量の実測と推定の区別（Issue #197）の技術仕様の
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

# 条件1: 利用量・費用の表示規則（実測と推定の区別、推定の根拠の示し方）が技術仕様に
# 明記されている。実測は提供元から届いた使用量、推定は§9.4 の仮定からの計算で、
# 実測が届いたら推定を置き換える。推定には根拠（仮定と単価の出典）を示す。
check cond1_actual_vs_estimated docs/technical.md \
  '実測（reported）' \
  '推定（estimated）' \
  '置き換えます' \
  '根拠' \
  '§9.4' \
  '§9.2'

# 条件2: ルーティング階層ごとの集計（どの階層のモデルでいくら使ったか）が
# 記録項目として定義されている。階層は§5 の作業の重さによる振り分け先。
check cond2_tier_aggregation docs/technical.md \
  '階層ごとに集計' \
  '記録項目' \
  '階層' \
  '提供元・モデル' \
  'トークン数' \
  '単価' \
  '§5'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "usage-actual-vs-estimate-docs: ok"
