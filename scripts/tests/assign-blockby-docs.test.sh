#!/usr/bin/env bash
# 割り当ての前の blocker（blocked_by）確認の決まり（Issue #408）が正本
# （runbook の「割り当て・停止・再開」）に落ちていて、skill command-team の
# 「2. 振り分け」がそこを参照していることを固定するテスト。
# 位置づけ: このスクリプトは「必須文言が正本に存在し、skill がそれを参照している」
# という観測可能な事実だけを確かめる決定的な検査（wording pin）であり、受け入れ条件の意味や仕様の整合性は評価しない
# （AGENTS.md「LLM judges」: 自然言語の意味の評価は rubric 付きの LLM judge が担い、
# 決定的な検査は観測可能な効果に限る。語句が残っていれば矛盾した記述でも通るため、
# 意味の確認は PR 本文での引用と skill review-pr のレビューで行う）。
# 文言を変えるときはこのテストと同じ PR で直す。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
failures=()

# section <file> <開始見出し（正規表現）> <終了見出し（正規表現）> — 見出し間の中身を
# 標準出力へ出す。終了見出しは開始行自身にはマッチさせない。
section() {
  awk -v s="$2" -v e="$3" '
    $0 ~ s { insec = 1 }
    insec && $0 ~ e && $0 !~ s { exit }
    insec { print }
  ' "$ROOT/$1"
}

# check <name> <text> <pattern...> — すべての pattern が text（節の中身）にあれば合格。
check() {
  local name="$1" text="$2"
  shift 2
  local p
  for p in "$@"; do
    if ! grep -qF -- "$p" <<<"$text"; then
      failures+=("$name: $p")
    fi
  done
}

RUNBOOK="$(section docs/agents/grokbot-runbook.md '^### 割り当て・停止・再開$' '^### ')"
SKILL="$(section .agents/skills/command-team/SKILL.md '^## 2[.] 振り分け$' '^## [0-9]')"

# 必須文言（チーム判断の 10 項目。正本の runbook に載る）
COMMON=(
  'issue_dependencies_summary.blocked_by'
  'total_blocked_by'
  'Blocked by #N'
  '0 件と同じ扱いにしない'
  'needs-human'
  'blocker の番号・状態・取得した時刻'
  '巡回の最初'
  '割り当てる直前'
  'scripts/agent-gh dev api'
  'deps-read'
  'dependencies/blocked_by'
  '--paginate'
  '開発リード'
  '根拠'
  '開き直し'
  'hatai に確認'
  '合成のデータ'
  '本物の Issue'
)

check runbook_blockby "$RUNBOOK" "${COMMON[@]}"
check skill_blockby_pointer "$SKILL" \
  '### 割り当ての可否（blocker の確認）' \
  'runbook の「割り当て・停止・再開」の blocker の確認'

# 節の抽出自体の失敗（見出しの改名で空になった）も検出する
if [[ -z "$RUNBOOK" ]]; then
  failures+=("runbook_section_missing: docs/agents/grokbot-runbook.md の「### 割り当て・停止・再開」が抽出できない")
fi
if [[ -z "$SKILL" ]]; then
  failures+=("skill_section_missing: .agents/skills/command-team/SKILL.md の「## 2. 振り分け」が抽出できない")
fi

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "assign-blockby-docs: ok"
