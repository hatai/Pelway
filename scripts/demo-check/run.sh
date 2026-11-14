#!/usr/bin/env bash
# demo-check: デモ受入前の 4 段階検査のランナー。
#   (a) loopback      : 待受・接続先が 127.0.0.1 / ::1 だけであること（static は常に実施。
#                       runtime は DEMO_CHECK_RUNTIME=1 で ss ベースの検査。未設定時は PENDING）
#   (b) leak-phrases  : 製品ソース・資産・ビルド成果物に禁止語・秘密値の形が無いこと（forbidden-phrases.txt）
#   (c) external-refs : 製品ソース・資産・ビルド成果物に外部ホスト参照が無いこと（allowed-hosts.txt）
#   (d) canary        : ビルド成果物・ログ・端末保存領域に合成データの目印が無いこと（canaries.txt）
# 走査範囲と除外は lib.sh。
# 引数は上の段階名の完全一致だけ受け付ける。それ以外（パス・未知の名前）が 1 つでもあれば
# 何も実行せずに exit 2 で終わる（任意のスクリプトを実行させないため）。
# 段階を指定した実行は PARTIAL RUN で、受入には使えない。受入は引数なしの全段階実行だけ。
# DEMO_CHECK_ACCEPTANCE=1 のとき（受入モード）:
#   - まとめに ACCEPTANCE MODE と表示する
#   - loopback の runtime PENDING（DEMO_CHECK_RUNTIME 未設定）は exit 1（段階失敗）
#   canary の走査 0 件（no artifacts to scan）は未検証として、受入・通常ともに canary が
#   exit 1 になり全体も exit 1（slot6）。
#   既定（未設定）の挙動は変えない。DEMO_CHECK_REQUIRE_RUNTIME は作らない。
# 全段階を最後まで実行し、1 段でも失敗すれば全体も非 0 で終わる。
# 使い方: mise run demo-check                       # 全段階（受入に有効）
#         DEMO_CHECK_ACCEPTANCE=1 mise run demo-check
#         bash scripts/demo-check/run.sh loopback   # 段階を指定（PARTIAL RUN）
#         mise run demo-check-selftest              # 検査自体の自己テスト
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
all_stages=(loopback leak-phrases external-refs canary)

is_stage() {
  local s
  for s in "${all_stages[@]}"; do [ "$1" = "${s}" ] && return 0; done
  return 1
}

bad=0
for a in "$@"; do
  if ! is_stage "${a}"; then
    echo "demo-check: invalid stage argument: '${a}' (allowed: ${all_stages[*]})" >&2
    bad=1
  fi
done
if [ "${bad}" -ne 0 ]; then
  echo "demo-check: nothing was run" >&2
  exit 2
fi

partial=0
if [ "$#" -gt 0 ]; then
  stages=("$@")
  for s in "${all_stages[@]}"; do
    case " $* " in *" ${s} "*) ;; *) partial=1 ;; esac
  done
else
  stages=("${all_stages[@]}")
fi
[ "${partial}" -eq 1 ] && echo "PARTIAL RUN: not valid for acceptance (selected: ${stages[*]})"
[ "${DEMO_CHECK_ACCEPTANCE:-}" = "1" ] && echo "ACCEPTANCE MODE"

failed=()
for s in "${stages[@]}"; do
  echo "== demo-check: ${s} =="
  if bash "${here}/${s}.sh"; then
    echo "-- ${s}: PASS"
  else
    rc=$?
    echo "-- ${s}: FAIL (exit ${rc})" >&2
    failed+=("${s}")
  fi
done

echo "== demo-check summary: ${#stages[@]} stage(s), ${#failed[@]} failed =="
if [ "${DEMO_CHECK_ACCEPTANCE:-}" = "1" ]; then
  echo "ACCEPTANCE MODE"
fi
if [ "${partial}" -eq 1 ]; then
  echo "PARTIAL RUN: not valid for acceptance; acceptance requires a full run (mise run demo-check with no arguments)"
else
  echo "full run: all ${#all_stages[@]} stages"
fi
case " ${stages[*]} " in
  *" loopback "*)
    if [ "${DEMO_CHECK_RUNTIME:-}" = "1" ]; then
      echo "note: loopback runtime check ran (ss-based; set by DEMO_CHECK_RUNTIME=1)"
    elif [ "${DEMO_CHECK_ACCEPTANCE:-}" = "1" ]; then
      echo "note: loopback runtime PENDING fails under ACCEPTANCE MODE"
    else
      echo "note: loopback runtime check is PENDING (set DEMO_CHECK_RUNTIME=1 to enable the ss-based check)"
    fi ;;
esac
if [ "${#failed[@]}" -gt 0 ]; then
  echo "failed: ${failed[*]}" >&2
  exit 1
fi
exit 0
