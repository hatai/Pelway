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
  'round 4 以降に達したかは、現在のラウンド数ではなく、連続したラウンドサマリーの履歴に残る最大の `<R>` で判定する' \
  '実装担当の終わりの報告のラウンド数ではなく、ラウンドサマリーの履歴で確かめる' \
  '初回の付与と付け直しの前に' \
  '`created_at` と `updated_at` が同じ（未編集の）コメント' \
  '最大 `<R>` が 4 以上なら round 4 以降に達した PR とする' \
  '1 から最大 `<R>` までの番号に欠けがあるときは、`automerge` を付けずに `needs-human` で人間に確かめる' \
  '編集されたラウンドサマリーが 1 件でもある、または 1 から最大 `<R>` までの番号に欠けがあるときは、`automerge` を付けずに `needs-human` で人間に確かめる' \
  'round 4 以降のレビューでセキュリティ領域（上のリスト）の must-fix が出たら、そのラウンドのサマリーを PR に残してから止めて'

# 実装担当のいない PR でも、指揮役が同じ記録を残す。
check conductor_round_summary docs/agents/grokbot-runbook.md \
  '結果を実装スレッドに送るときは実装担当が残し、結果を送らずに止めるときと実装スレッドに送らない PR では指揮役が残す' \
  'ラウンドサマリーはレビューのラウンドの記録と round 4 到達の記録に使うため、省略しない'

# 停止経路でもラウンドの記録を残し、番号は履歴から続ける。
check stop_paths_keep_round_record .agents/skills/implement-issue/SKILL.md \
  '止まる前に手順 9 の形のラウンドサマリーを PR に残し、`must-fix:` に未対応のまま止まる旨を書く' \
  'レビューの結果への対応の途中でこの表の条件に当たったときは、止まる前に手順 9 の形のラウンドサマリーを PR に残す' \
  'ラウンドサマリーの `<R>` は、PR の未編集のラウンドサマリー履歴に残る最大 `<R>` から続ける' \
  'REST の `user.login` が `pelway-dev[bot]` で `user.type` が `Bot`、`created_at` と `updated_at` が同じ（未編集の）もので、1 行目が `### review round <R>` のものだけに限る' \
  '`[文書のずれ]` の nit だけを直す再対応のラウンドサマリーでは、直前の同じラウンドの `<R>` を使う' \
  '新しいラウンドでは最大 `<R>` + 1 を使い、コンフリクトの解消を含まない main の取り込みだけで同じラウンドをやり直すときは同じ `<R>` を使う' \
  '履歴を読めない・編集されたサマリーがある・番号の欠けがあるときは、`<R>` を推測せず、「止まる条件」の表の履歴の行で止まる'

check conductor_stop_keeps_round_record docs/agents/grokbot-runbook.md \
  '修正を中継しないときは、指揮役がラウンドサマリーを残してから hatai の判断に戻す'

# QA の依頼は admin マージの判定から独立させ、QA の合格後にマージ方法を分ける。
check qa_before_merge_method docs/agents/grokbot-runbook.md \
  '正本の条件のうち「admin マージの対象に当たらないこと」を除く条件が、QA の合格以外はそろっている PR は QA 待ちである' \
  '同じ SHA の QA の合格が出たら、admin マージの対象に当たる PR は hatai の admin マージに回し' \
  '正本の条件のうち「admin マージの対象に当たらないこと」を除く条件が QA の合格以外そろっていれば QA 待ちとして扱う'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "review-round-record-docs: ok"
