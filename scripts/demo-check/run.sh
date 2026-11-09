#!/usr/bin/env bash
# demo-check: デモ受入前の 3 段階検査の土台（ランナー）。
#   (a) loopback      : デモの待受が 127.0.0.1 だけであること
#   (b) leak-phrases  : ビルド成果物・ログに禁止語（漏えい時に問題になる語句）が無いこと
#   (c) external-refs : デモ資産に外部 URL・外部ホスト参照が無いこと
# 未実装の段階は "NOT IMPLEMENTED" を出して非 0 で終わる。
# 全段階を最後まで実行し、1 段でも失敗すれば全体も非 0 で終わる（未実装を合格にしない）。
# 使い方: mise run demo-check                 # 全段階
#         bash scripts/demo-check/run.sh loopback  # 段階を指定
set -u

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
all_stages=(loopback leak-phrases external-refs)

if [ "$#" -gt 0 ]; then
  stages=("$@")
else
  stages=("${all_stages[@]}")
fi

failed=()
for s in "${stages[@]}"; do
  script="${here}/${s}.sh"
  echo "== demo-check: ${s} =="
  if [ ! -f "${script}" ]; then
    echo "-- ${s}: FAIL (unknown stage; expected one of: ${all_stages[*]})" >&2
    failed+=("${s}")
    continue
  fi
  if bash "${script}"; then
    echo "-- ${s}: PASS"
  else
    rc=$?
    echo "-- ${s}: FAIL (exit ${rc})" >&2
    failed+=("${s}")
  fi
done

echo "== demo-check summary: ${#stages[@]} stage(s), ${#failed[@]} failed =="
if [ "${#failed[@]}" -gt 0 ]; then
  echo "failed: ${failed[*]}" >&2
  exit 1
fi
exit 0
