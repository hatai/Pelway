#!/usr/bin/env bash
# Issue #799 の採点表と不具合報告項目が各スキルにあることを確かめるテスト。
# 位置づけ: 文言が各スキルに存在することだけを確かめる決定的な presence check であり、
# 採点や報告の意味の正しさは評価しない（AGENTS.md「LLM judges」。意味の確認は PR 本文での引用と
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

# triage: 要望の採点表（項目と 0〜3 の基準）、根拠の確かさ、unknown、合計の扱い。
check triage_feature_scoring .agents/skills/triage/SKILL.md \
  '| Dimension | 0 | 1 | 2 | 3 |' \
  '| Impact |' \
  '| Reach |' \
  '| Fit |' \
  '| UX |' \
  '| Cost |' \
  'Cost is inverse: 3 means low cost.' \
  '`confirmed`, `assumed`, or `unknown`' \
  '**Value score:** incomplete — material evidence is missing.' \
  'comparison aid only, not a priority' \
  '## Feature assessment'

# investigate-bug: 手順 1 と手順 5 の両方に報告の読み取りの項目がある。
check investigate_bug_fields .agents/skills/investigate-bug/SKILL.md \
  '- 版:' \
  '- 回帰か:' \
  '- 最後に動いた版:' \
  '- 重さ:' \
  '- 頻度:' \
  '- 報告の読み取り:' \
  '致命的' \
  '合理的に確かめられる' \
  '手段を試した後だけにする'

# review-pr: 調査報告 PR の確認項目が investigate-bug の新しい形式と同期している。
check review_pr_report_items .agents/skills/review-pr/SKILL.md \
  '報告の読み取り（期待、実際、対象、場所、版、時刻、頻度、回帰か、最後に動いた版、重さ）' \
  '使ったスキル'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "triage-investigate-docs: ok"
