#!/usr/bin/env bash
# 指揮役のコンテキスト圧縮の仕組み化（Issue #296）が正本 3 文書と検証記録に落ちているかのテスト。
# 対象は prose の差分なので、各受け入れ条件を grep したときの必須文言で確かめる。
# 文言を変えるときはこのテストと同じ PR で直す。
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

CT=.agents/skills/command-team/SKILL.md
RB=docs/agents/grokbot-runbook.md
AT=docs/superpowers/specs/agent-team.md
DE=docs/dev-environment.md

# 条件1: command-team に「コンテキスト管理」の節があり、計画的圧縮と予期しない圧縮の
# 2 経路、区切りの定義、結果不明の自発行操作が残る間は計画的圧縮を延期すること。
check cond1_two_routes "$CT" \
  '## 7. コンテキスト管理' \
  '計画的圧縮' \
  '予期しない圧縮' \
  '区切り' \
  '結果不明の自発行操作が残っている間は計画的圧縮を延期'

# 条件2: トリガーが暫定の観測基準（使用率 70% 以上、代理指標は完了 Issue 5 件以上
# または巡回 20 回以上）で、起算点と集計定義（完了の数え方、巡回は開始時点で 1 回）
# が定められていること。
check cond2_trigger "$CT" \
  '使用率が取得できる環境では使用率 70% 以上（代理指標は使わない）' \
  '完了 Issue 5 件以上、または巡回 20 回以上' \
  '起算点' \
  '巡回は開始時点で 1 回'

# 条件3: 圧縮の打ち切り（圧縮後も 70% を下回なければその区切りで再圧縮しない、
# 圧縮操作が 2 回連続で失敗したら繰り返さず交代または人間確認へ）。
check cond3_abort "$CT" \
  'その区切りで再圧縮しない' \
  '2 回連続で失敗したら繰り返さず、交代または人間確認へ進む'

# 条件4: 圧縮前の外部化チェックがチェックリストとして書かれ、ボード Status の確認は
# #290 正本化後は導出規則を参照し、正本化前は暫定確認に限ることが明記されていること。
check cond4_externalize "$CT" \
  '外部化チェック' \
  '割り当てと次アクション' \
  'patrol-state の 3 項目' \
  '未記録の判断' \
  'ボード Status' \
  '復元検証待ち' \
  '圧縮未実施' \
  '「ステータスの導出規則」を参照' \
  '正本化前は agent-team.md の既存の決まりに基づく暫定確認に限る'

# 条件5: 操作発行時の記録が常時ルールとして書かれていること。
check cond5_oplog "$CT" \
  '操作発行時の記録' \
  '発行前に種類・対象・照合識別子' \
  '結果確定後に追記' \
  '事前に識別子を確保できない操作は実行先での照合方法'

# 条件6: 復元検証の手順（入口 3 つ、情報種別ごとの照合先、食い違いの解消と保留）。
check cond6_restore "$CT" \
  '圧縮直後' \
  '巡回開始時' \
  '異常検知時' \
  '状態=GitHub' \
  '判断=巡回記録' \
  '待ち=patrol-state' \
  '解消不能なら依存操作を保留'

# 条件7: 復元検証の終了の「完了」と「部分復旧」の区別。
check cond7_restore_end "$CT" \
  '部分復旧' \
  '未解決事項' \
  '停止対象' \
  '解除条件' \
  '毎巡回の開始時に継続照合'

# 条件8: 結果不明の操作の照合方法。
check cond8_unknown "$CT" \
  '結果不明の操作の照合' \
  '照合手段がない操作は結果不明のまま保留' \
  '依存する後続操作を止める' \
  '再実行しない'

# 条件9: 圧縮の結果の区分と、起算点の更新・成功回数の計上の条件。
check cond9_result "$CT" \
  '発生確認済み' \
  '未実施・失敗' \
  '発生不明' \
  '起算点の更新と圧縮成功回数の計上は「発生確認済み」の場合だけ'

# 条件10: 予期しない圧縮への対処。
check cond10_unexpected "$CT" \
  '通常操作を止めて' \
  '検知方法が未検証の環境では' \
  'の直前に正本・巡回記録・実行先の状態を照合する' \
  '有効化対象にしない'

# 条件11: 交代手順（操作の停止、権限の移譲点、確認失敗時の不移転、同時稼働の禁止）。
check cond11_handover "$CT" \
  '恒久交代' \
  '案件を進行させる操作を止める' \
  '引き継ぎ確認の完了時点で操作権限が移る' \
  '権限を移さず' \
  '同時稼働の禁止'

# 条件12: 導入条件（検証済みの環境でのみ有効化、未検証では外部化して停止し人間に確認）。
check cond12_adoption "$CT" \
  '導入条件' \
  '検証済みであること' \
  '未検証の環境では' \
  '人間に確認する'

# 条件13: runbook の巡回開始手順の復元検証の判定と、巡回の区切りの圧縮要否の判定。
check cond13_runbook "$RB" \
  '未完了の復元検証待ち記録があれば復元検証から始める' \
  '圧縮要否の判定'

# 条件14: agent-team.md に原則が 1〜2 行で書かれていること。
check cond14_agent_team "$AT" \
  '圧縮' \
  'skill `command-team` の「コンテキスト管理」'

# 条件15: 観測と見直し（各イベントの記録と、見直しの条件）。
check cond15_observe "$CT" \
  'トリガー発生' \
  '延期' \
  '異常検知' \
  '交代' \
  '発動理由' \
  '圧縮成功 10 回' \
  '1 か月'

# 条件16: T3 の全プロバイダ（opencode・Devin CLI・Codex・Claude）の検証結果が
# dev-environment 側の記録に残っていること。
check cond16_verification "$DE" \
  '明示的な圧縮操作' \
  'コンテキスト使用率' \
  '自動圧縮の有無と検知' \
  '### opencode（GLM、zai-coding-plan/glm-5.3、OpenCode server 2.3.0）' \
  '### Devin CLI（acpRegistry_devin、swe-2-high' \
  '### Codex（codex_review、gpt-6-luna）' \
  '### Claude（claude_review、claude-opus-5-5'

# 条件17（hatai の指示による拡張）: プロバイダごとの差分が command-team の該当節に
# 明記されていること（トリガー・予期しない圧縮・導入条件の 3 箇所）。
check cond17_provider_diff "$CT" \
  'プロバイダごとに異なる' \
  'プロバイダ単位で行い'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "conductor-compaction-docs: ok"
