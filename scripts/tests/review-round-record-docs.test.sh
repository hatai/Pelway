#!/usr/bin/env bash
# round 4 到達の記録と、QA 依頼・マージ方法の順序（Issue #725 round 5）
# の文言を固定するテスト。
# 位置づけ: 文言が文書に存在することだけを確かめる決定的な検査（wording pin、存在の確認）であり、
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

# round 4 到達は現在のラウンド数と分け、履歴の最大値で記録する。
check round4_record_command_team .agents/skills/command-team/SKILL.md \
  'round 4 以降に達したかは、現在のラウンド数ではなく、ラウンドサマリーの履歴に残る最大の `<R>` で判定する' \
  '実装担当の終わりの報告のラウンド数ではなく、ラウンドサマリーの履歴で確かめる' \
  '初回の付与と付け直しの前に' \
  '`created_at` と `updated_at` が同じ（未編集の）コメント' \
  '最大 `<R>` が 4 以上なら round 4 以降に達した PR とする' \
  '編集されたラウンドサマリーが 1 件でもあるときは、`automerge` を付けずに `needs-human` で人間に確かめる'

# 実装担当のいない PR でも、指揮役が同じ記録を残す。
check conductor_round_summary docs/agents/grokbot-runbook.md \
  '外部投入で実装スレッドに結果を送る PR 以外では' \
  'ラウンドサマリーは現在のラウンド数と round 4 到達の記録に使うため、省略しない'

# QA の依頼は admin マージの判定から独立させ、QA の合格後にマージ方法を分ける。
check qa_before_merge_method docs/agents/grokbot-runbook.md \
  'admin マージの対象に当たらないことと QA の合格' \
  '同じ SHA の QA の合格が出たら、admin マージの対象に当たる PR は hatai の admin マージに回し' \
  '正本の条件のうち admin マージの対象に当たらないことと QA の合格だけが欠けていれば QA 待ちとして扱う'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "review-round-record-docs: ok"
