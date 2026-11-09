#!/usr/bin/env bash
# demo-check 段階 (b) leak-phrases: 製品ソース・資産・ビルド成果物の禁止語検索。
# 禁止語は forbidden-phrases.txt（リード確定 2026-11-09 15:01/15:03）。走査範囲と除外は lib.sh を参照。
# 適用範囲: all / secret / ident = すべてのテキスト、copy = 利用者向けの文言と外向け文書だけ（リストの冒頭に記載）。
# 違反は「ファイル:行: 規則」で出し、行の中身は出さない。exit 0 合格 / 1 違反 / 2 検査異常。
set -u
# shellcheck source=lib.sh
. "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

list="${DC_LIST_DIR}/forbidden-phrases.txt"
rules="$(dc_list_lines "${list}")" || exit 2
[ -n "${rules}" ] || { echo "leak-phrases: no rules in ${list}" >&2; exit 2; }
mapfile -t files < <(dc_all_files)

hits=0
report() { echo "$1: [$2] /$3/"; hits=$((hits + 1)); }

while IFS=$'\t' read -r scope icase pat; do
  [ -n "${pat:-}" ] || { echo "leak-phrases: malformed rule line in ${list}" >&2; exit 2; }
  gflags=(-E)
  [ "${icase}" = "i" ] && gflags+=(-i)
  for f in "${files[@]}"; do
    path="${DC_ROOT}/${f}"
    if dc_is_wasm "${f}"; then
      n="$(grep -a -c "${gflags[@]}" -e "${pat}" -- "${path}" || true)"
      [ "${n:-0}" -gt 0 ] && report "${f}:(binary)" "${scope}" "${pat} (${n}件)"
      continue
    fi
    dc_is_text "${f}" || continue
    case "${scope}" in
      all | ident | secret)
        lines="$(grep -n "${gflags[@]}" -e "${pat}" -- "${path}" | cut -d: -f1)" ;;
      copy)
        if dc_is_copy_file "${f}"; then
          lines="$(grep -n "${gflags[@]}" -e "${pat}" -- "${path}" | cut -d: -f1)"
        elif [[ "${f}" == *.rs ]]; then
          # 各 " の直後に空白を入れてから照合する（"24時間" のように文字列の先頭にある語でも、
          # 規則の「直前が数字でない」条件が効くようにするため。行番号は変わらない）
          lines="$(sed 's/"/" /g' -- "${path}" | grep -n "${gflags[@]}" -e "\"[^\"]*(${pat})[^\"]*\"" \
            | grep -vE '^[0-9]+:[[:space:]]*//' | cut -d: -f1)"
        else
          lines=""
        fi ;;
      *) echo "leak-phrases: unknown scope '${scope}' in ${list}" >&2; exit 2 ;;
    esac
    for l in ${lines}; do report "${f}:${l}" "${scope}" "${pat}"; done
  done
done <<< "${rules}"

echo "leak-phrases: scanned ${#files[@]} file(s), ${hits} hit(s)"
[ "${hits}" -eq 0 ]
