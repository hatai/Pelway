#!/usr/bin/env bash
# Issue #256: claude_review の稼働ゲート。
# command-team が Claude のレビュー担当の保留（「確認が済むまで使わない」）を解くときは、
# 解除の行が確認記録を参照し、docs/dev-environment.md「レビュー担当の Claude」の
# 記録欄（確認手順の結果）が全項目記入済みであることが前提。
# 記録を消す・保留の文面だけ戻す変更はここで検出する。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SKILL="$ROOT/.agents/skills/command-team/SKILL.md"
DOC="$ROOT/docs/dev-environment.md"
failures=()

# 1. command-team の「Claude のレビュー担当」の行:
#    - 保留（確認が済むまで使わない）が解けている
#    - 確認記録（dev-environment.md の確認手順の結果）を参照している
claude_line="$(grep -n 'Claude のレビュー担当' "$SKILL")"
[[ -n "$claude_line" ]] || failures+=("command-team: 'Claude のレビュー担当' の行が無い")
if [[ -n "$claude_line" ]]; then
  grep -qF '確認が済むまで使わない' <<<"$claude_line" && \
    failures+=("command-team: Claude の保留（確認が済むまで使わない）が残っている")
  grep -qF '確認手順の結果' <<<"$claude_line" || \
    failures+=("command-team: Claude の行が記録欄（確認手順の結果）を参照していない: $claude_line")
fi

# 2. dev-environment.md「レビュー担当の Claude」節の記録欄:
#    - 4 行分の項目がある
#    - 各項目の結果セル（2 列目）が記入済み（空欄・プレースホルダーでない）
#    - 未実施・未記入が残っていない
claude_section="$(awk '/^## レビュー担当の Claude/{f=1} /^## レビュー担当の Devin CLI/{f=0} f' "$DOC")"
[[ -n "$claude_section" ]] || failures+=("dev-environment.md: 'レビュー担当の Claude' 節が無い")
if [[ -n "$claude_section" ]]; then
  for label in '実施日' '使えるコマンドの制限の結果' 'ネットワーク制限の結果' 'Codex 側（`codex_review`）との差'; do
    row="$(grep -F "| ${label}" <<<"$claude_section" | grep -v '^| 項目' | head -1)"
    if [[ -z "$row" ]]; then
      failures+=("dev-environment.md: 記録欄に項目が無い: $label")
      continue
    fi
    for col in 3 4; do
      cell="$(awk -F'|' -v c="$col" '{print $c}' <<<"$row" | sed 's/^ *//; s/ *$//')"
      if [[ -z "$cell" || "$cell" =~ ^(—|--?|未定|未実施|未記入|TBD|N/?A)$ ]]; then
        failures+=("dev-environment.md: 結果・実施日セルが空欄またはプレースホルダー: $label（${col}列目='$cell'）")
      fi
    done
  done
  grep -nE '未実施|未記入' <<<"$claude_section" >/dev/null && \
    failures+=("dev-environment.md: 記録欄に未実施・未記入が残っている")
fi

if ((${#failures[@]})); then
  printf 'FAIL %s\n' "${failures[@]}"
  exit 1
fi
echo ok
