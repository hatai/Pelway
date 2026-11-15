#!/usr/bin/env bash
# Issue #256 の文面条件のテスト。レビュー担当 Claude（claude_review）の稼働確認が
# docs/dev-environment.md の記録欄に記入され、command-team の保留（確認が済むまで
# 使わない）が確認記録への参照で解けているかを grep で確かめる。
# 文言を変えるときはこのテストと同じ PR で直す。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
failures=()

# dev-environment.md の「レビュー担当の Claude」節（次節の Devin CLI 手前まで）。
section="$(awk '/^## レビュー担当の Claude（claude_review）/{f=1} /^## レビュー担当の Devin CLI/{f=0} f' "$ROOT/docs/dev-environment.md")"
if [[ -z "$section" ]]; then
  failures+=("claude_section_missing: docs/dev-environment.md に「レビュー担当の Claude」節が無い")
fi

# 条件: 確認手順の結果（記録欄）の全行が記入済み（未実施・未記入が残っていない）。
if grep -qE '\| (未実施|未記入) \|' <<<"$section"; then
  failures+=("claude_record_unfilled: docs/dev-environment.md の「レビュー担当の Claude」の記録欄に未実施・未記入が残っている")
fi

# 条件: command-team の Claude の保留が解けている（旧い保留の文が無い）。
if grep -qF 'Claude のレビュー担当は、`docs/dev-environment.md` の「レビュー担当の Claude」の確認が済むまで使わない' "$ROOT/.agents/skills/command-team/SKILL.md"; then
  failures+=("command_team_hold_left: .agents/skills/command-team/SKILL.md に Claude の保留の文が残っている")
fi

# 条件: 保留は確認記録（記録欄）への参照で解けている。
if ! grep -qF 'Claude のレビュー担当の確認は済んでいる（`docs/dev-environment.md` の「レビュー担当の Claude」の記録欄' "$ROOT/.agents/skills/command-team/SKILL.md"; then
  failures+=("command_team_hold_ref: .agents/skills/command-team/SKILL.md に確認記録（記録欄）への参照が無い")
fi

if ((${#failures[@]})); then
  printf '%s\n' "${failures[@]}" >&2
  exit 1
fi
echo "claude-review-gate: ok"
