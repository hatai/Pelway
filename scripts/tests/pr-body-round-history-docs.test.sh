#!/usr/bin/env bash
# PR 本文の round の履歴と検証の表の書き方（Issue #595）の文言を固定するテスト。
# 位置づけ: 文言がスキルに存在することだけを確かめる決定的な検査（wording pin）であり、
# 手順の意味や整合性は評価しない（AGENTS.md「LLM judges」。意味の確認は PR 本文での引用と
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

# 実装担当: 「レビューと QA」には round の履歴を書かず、メモを指す 1 行と最後に検証した sha だけ。
check implementer_review_qa .agents/skills/implement-issue/SKILL.md \
  'PR 本文の「レビューと QA」には round の履歴を書かず、実装者メモ（`docs/worklogs/issue-<N>.md`）を指す 1 行と、最後に検証した sha だけを書く' \
  'round の履歴の正本は実装者メモとする'

# 実装担当: 修正の round では「確認方法/検証」の表を最後の round の結果だけに置き換える。
check implementer_verification_table .agents/skills/implement-issue/SKILL.md \
  '修正の round では、PR 本文の「確認方法/検証」の表を、その head の試験の出力から写した最後の round の結果だけに置き換える'

# write-pull-request: 「レビューと QA」の説明が implement-issue と食い違わない。
check write_pr_review_qa .agents/skills/write-pull-request/SKILL.md \
  'round の履歴は書かない。実装担当の PR では、実装者メモ（`docs/worklogs/issue-<N>.md`）を指す 1 行と、最後に検証した sha だけを書く(implement-issue 手順 7)'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "pr-body-round-history-docs: ok"
