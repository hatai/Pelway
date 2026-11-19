#!/usr/bin/env bash
# Issue #490 で足した監査仕様（承認回答の監査事象への記録と、主体を消しても消えない記録）の
# 回帰検査。対象は prose の差分なので、各受け入れ条件に対応する必須文言が技術仕様に
# 残っているかを grep で確かめる。文言を変えるときはこのテストと同じ PR で直す。
# R11/D01・#415 との整合といった自然言語の意味の成否は Issue #490 の受け入れ条件どおり
# 目視確認と skill review-pr のレビューが担い、このテストでは採点しない（AGENTS.md の
# LLM judges がキーワード一致での採点を禁じるため）。
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

DOC03=docs/technical/v0.7/body/03-ai-bot-runtime.md

# 条件1: 承認・却下・失効・取り下げの回答が監査の事象に含まれることが書かれている。
check cond1_responses_are_event_kinds "$DOC03" \
  '承認・却下・失効・取り下げ'
check cond1_each_response_recorded "$DOC03" \
  '独立の監査事象'

# 条件2: 監査の行は主体（利用者・Bot）に外部キーで結ばず、主体を消しても残ることが書かれている。
check cond2_no_fk_to_subjects "$DOC03" \
  '外部キーで結ばない'
check cond2_survives_subject_deletion "$DOC03" \
  '連鎖して消えない'

# 条件3: 承認の記録（ActionSnapshot と回答）は Bot の削除・外す操作で連鎖して消えないことが
# 書かれている。外部キー非依存の文言は既存節（論理操作・固定承認）ではなく監査脈絡でだけ
# 出る組み合わせで確かめる。
check cond3_snapshot_and_decision "$DOC03" \
  '主体の行への外部キーで参照せず'
check cond3_survives_bot_removal "$DOC03" \
  'ワークスペースから外す操作'

# 条件4: 監査の時刻は同じトランザクション内でも順序が付くことが書かれている。
check cond4_monotonic_order "$DOC03" \
  '単調な順序番号'

# 条件5: R11 の保持期間と、消去の依頼（D01）があったときの監査の扱いが矛盾しないことが
# 書かれている（13 側の R11/D01 の定義自体は変えない）。
check cond5_r11_retention_unchanged "$DOC03" \
  'R11（記録から365日）'
check cond5_erasure_request_handling "$DOC03" \
  '消去の依頼'

# 条件7: 削除された主体（消えた利用者・Bot）を表す形が pelway-contract に置かれることが
# 書かれている。
check cond7_deleted_subject_contract "$DOC03" \
  'pelway-contract'

# 条件8: 監査ログには仮の識別子（pseudonymous id）と hash だけを残し、承認カードの本文は
# 残さないことが書かれている（濱田さん決定 2026-11-19 17:23 JST）。
check cond8_pseudonymous_ids_only "$DOC03" \
  '仮名識別子（pseudonymous id）'
check cond8_no_approval_card_body "$DOC03" \
  '承認カードの本文'

# 前提: 保全対象の v0.7 の更新は検証（verification）の手順に従う。検証記録が report に
# 追記されていること（static_pass の実測は report 側の記録で確認する）。
check verify_report_updated docs/technical/v0.7/verification/report-v0.7.md \
  'Issue #490'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "audit-approval-docs: ok"
