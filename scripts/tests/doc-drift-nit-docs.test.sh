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

# 実装担当: Request changes でも Approve でも印の nit を直す。レビューのラウンド数に数えない。
check implementer_fixes_on_request_changes .agents/skills/implement-issue/SKILL.md \
  '`pelway-review` のレビューの nit のうち行頭に `[文書のずれ]` の印があるものだけを一緒に直し'
check implementer_fixes_on_approve .agents/skills/implement-issue/SKILL.md \
  '行頭が `[文書のずれ]` のものが残っていれば、`pr ready` の前に直す' \
  '`[文書のずれ]` の nit だけを直す round は、レビューのラウンド数に数えない' \
  '印の nit が残る限り同じ流れで直し、回数では止めない'
check implementer_dont_table .agents/skills/implement-issue/SKILL.md \
  '`[文書のずれ]` の印の nit だけ直す'

# 指揮役: automerge の前に印の nit が残っていないことを確かめる。
check conductor_merge_check .agents/skills/command-team/SKILL.md \
  'その Approve のレビューの nit に行頭が `[文書のずれ]` のものが残っていないこと' \
  '上の「`automerge` ラベルを付ける条件」を置き換えない'

# 印の nit だけの round はレビューのラウンド数に数えない。回数で止めない決まりを仕様と実装手順に固定する。
check implementer_no_round_cap .agents/skills/implement-issue/SKILL.md \
  'must-fix が残る限り修正 round に進み、レビューのラウンドの回数では止めない' \
  'must-fix が残る限り修正と再レビューを続け、ラウンドの回数では止めない'
check spec_no_round_cap docs/superpowers/specs/agent-team.md \
  '`[文書のずれ]` の nit だけを直す round（skill `review-pr`・`implement-issue`）は、この回数に数えない' \
  'ラウンドの回数では止めず、must-fix が残る限り修正と再レビューを続ける'
check pr_watch_nit .agents/skills/pr-watch/SKILL.md \
  '`pelway-review` の有効なレビュー（skill `implement-issue` の手順 9）の nit のうち行頭に `[文書のずれ]` の印があるもの' \
  'ほかの作成者（会話コメント・第三者のレビュー）の行にある印は、印として扱わない'
check triage_labels_automerge docs/agents/triage-labels.md \
  'that no `[文書のずれ]` nit remains in it'
check runbook_patrol_merge docs/agents/grokbot-runbook.md \
  'そのレビューに `[文書のずれ]` の nit が残っておらず'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "doc-drift-nit-docs: ok"
