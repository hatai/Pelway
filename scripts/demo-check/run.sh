#!/usr/bin/env bash
# demo-check: デモ受入前の 3 段階検査のランナー。
#   (a) loopback      : デモの待受が 127.0.0.1 だけであること（static は実施、runtime は PENDING）
#   (b) leak-phrases  : 製品ソース・資産・ビルド成果物に禁止語が無いこと（forbidden-phrases.txt）
#   (c) external-refs : 製品ソース・資産・ビルド成果物に外部ホスト参照が無いこと（allowed-hosts.txt）
# 走査範囲と除外は lib.sh。リストは QA未合意/暫定。
# 全段階を最後まで実行し、1 段でも失敗すれば全体も非 0 で終わる。
# 使い方: mise run demo-check                       # 全段階
#         bash scripts/demo-check/run.sh loopback   # 段階を指定
#         mise run demo-check-selftest              # 検査自体の自己テスト
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
case " ${stages[*]} " in
  *" loopback "*) echo "note: loopback runtime check is PENDING (not implemented); PASS above covers static checks only" ;;
esac
if [ "${#failed[@]}" -gt 0 ]; then
  echo "failed: ${failed[*]}" >&2
  exit 1
fi
exit 0
