#!/usr/bin/env bash
# PR への画像添付の手順（Issue #812）の文言を固定するテスト。
# 位置づけ: 文言が文書とスキルに存在することだけを確かめる決定的な検査（wording pin）であり、
# 手順の意味や整合性は評価しない（AGENTS.md「LLM judges」。意味の確認は PR 本文での引用と
# skill review-pr のレビューで行う）。文言を変えるときはこのテストと同じ PR で直す。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
failures=()

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

check attachment_doc docs/agents/pr-image-attachments.md \
  'GitHub App のトークンでは画像を添付できない' \
  'attaching files requires write access to the repository' \
  '実装担当が画像を用意する' \
  'hatai に hatai 名義のコメントでの添付を確かめる' \
  "gh pr comment <N> --attach '<path>#<alt>'" \
  'PR 本文からそのコメントを指す' \
  '人間のアカウントで投稿する道具は、hatai が文面か選択肢を明示して選んだときだけ使う'

check write_issue_pointer .agents/skills/write-issue/SKILL.md \
  'スクリーンショットを確かめ方にするとき' \
  'docs/agents/pr-image-attachments.md'

check implement_issue_pointer .agents/skills/implement-issue/SKILL.md \
  'docs/agents/pr-image-attachments.md'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "pr-image-attachment-docs: ok"
