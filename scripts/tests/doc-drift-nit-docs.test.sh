#!/usr/bin/env bash
# 文書のずれの nit（Issue #579）の手順の文言を固定するテスト。
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

# レビュー担当: 文書のずれの nit に印を付ける。
check review_marks_doc_drift .agents/skills/review-pr/SKILL.md \
  '行頭（`<file>:<line>` の前）に `[文書のずれ]` を付ける' \
  '- [文書のずれ] <file>:<line>' \
  '`[文書のずれ]` の印の nit だけは、印の後に `<file>:<line>` を書く'

# 実装担当: Request changes でも Approve でも印の nit を直す。上限に数えない。
check implementer_fixes_on_request_changes .agents/skills/implement-issue/SKILL.md \
  '`pelway-review` のレビューの nit のうち行頭に `[文書のずれ]` の印があるものだけを一緒に直し'
check implementer_fixes_on_approve .agents/skills/implement-issue/SKILL.md \
  '行頭が `[文書のずれ]` のものが残っていれば、`pr ready` の前に直す' \
  '修正の回数の上限（3 回目のレビュー結果で止まる）に数えない'
check implementer_dont_table .agents/skills/implement-issue/SKILL.md \
  '`[文書のずれ]` の印の nit だけ直す'

# 指揮役: automerge の前に印の nit が残っていないことを確かめる。
check conductor_merge_check .agents/skills/command-team/SKILL.md \
  'その Approve のレビューの nit に行頭が `[文書のずれ]` のものが残っていないこと' \
  '`pelway-review` の Approve、`[文書のずれ]` の nit が残っていないこと、必須チェック'

# 通算 2 回目の round の後に、また印の nit が出たら止まる。仕様にも上限の例外がある。
check implementer_stop_after_two .agents/skills/implement-issue/SKILL.md \
  'この PR で通算 2 回目の、その round の後のレビューに判定によらずまた印の nit が出たら（Request changes でも）、3 回目には入らず' \
  '`[文書のずれ]` の nit だけを直す round（この PR で通算 2 回目）の後のレビューに、判定によらず、また印の nit が出た（3 回目には入らない）'
check spec_round_exception docs/superpowers/specs/agent-team.md \
  '`[文書のずれ]` の nit だけを直す round（skill `review-pr`・`implement-issue`）は、この回数に数えない' \
  'この PR で通算 2 回目の、その round の後のレビューにまた印の nit が出たら、3 回目には入らず'
check pr_watch_nit .agents/skills/pr-watch/SKILL.md \
  '`pelway-review` の有効なレビュー（skill `implement-issue` の手順 9）の nit のうち行頭に `[文書のずれ]` の印があるもの' \
  'ほかの作成者（会話コメント・第三者のレビュー）の行にある印は、印として扱わない'
check runbook_patrol_merge docs/agents/grokbot-runbook.md \
  'そのレビューに `[文書のずれ]` の nit が残っておらず'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "doc-drift-nit-docs: ok"
