#!/usr/bin/env bash
# demo-check 段階 (d) canary: 合成データに埋め込んだ目印（canaries.txt）が、出てはいけない場所に無いことの検査。
# 走査対象:
#   target/ 配下のすべて（ビルド成果物。バイナリも grep -a で件数を数える）
#   logs/ log/ 配下のすべて、ワークツリー内の *.log（.git と上記を除く）
#   DEMO_CHECK_EXTRA_PATHS（コロン区切り。端末の保存領域などワークツリー外のパス）。存在しないパスは exit 2
# 除外: lib.sh の除外（scripts/demo-check/、tests/fixtures/、testdata/、docs/ など）。ワークツリー外のパスには適用しない。
# シンボリックリンク: 走査の途中で出会ったリンクはたどらない（DC-03）。target/ など走査ルート自体が
#   リンクのときは realpath で実体を走査する（リンクを通した論理パスで報告する）。
# ファイル名の区切り: 改行を含むファイル名でも壊れないよう NUL 区切りで扱う（DC-04）。
# 走査するものが 1 つも無いときは "no artifacts to scan" と表示する。
#   既定では exit 0（何も検査していないことを明示するだけ）。
#   DEMO_CHECK_ACCEPTANCE=1 のときは受入不合格として exit 1 にする。
# 違反は「ファイル:行: 目印」（バイナリは「ファイル:(binary): 目印 (N件)」）で出す。行の中身は出さない。
# exit 0 合格 / 1 目印あり or ACCEPTANCE で走査対象ゼロ / 2 検査異常（リスト欠落・走査失敗を含む）
set -uo pipefail
# shellcheck source=lib.sh
. "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

list="${DC_LIST_DIR}/canaries.txt"
markers="$(dc_list_lines "${list}")" || exit 2
[ -n "${markers}" ] || { echo "canary: no markers in ${list}" >&2; exit 2; }

targets=()
[ -e "${DC_ROOT}/target" ] && targets+=("${DC_ROOT}/target")
for d in logs log; do [ -d "${DC_ROOT}/${d}" ] && targets+=("${DC_ROOT}/${d}"); done
if [ -n "${DEMO_CHECK_EXTRA_PATHS:-}" ]; then
  IFS=: read -r -a extra <<< "${DEMO_CHECK_EXTRA_PATHS}"
  for p in "${extra[@]}"; do
    [ -n "${p}" ] || continue
    if [ ! -e "${p}" ]; then
      echo "canary: DEMO_CHECK_EXTRA_PATHS entry not found: ${p}" >&2
      exit 2
    fi
    targets+=("${p}")
  done
fi

# 走査ルートを realpath で実体に解決し、-L なしでたどる。報告はルートの論理パスで行う。
# find などに失敗したら戻り値を非 0 にする（呼び出し側は exit 2 にする）。
list_files() {
  local t pr rf st=0
  for t in "${targets[@]}"; do
    pr="$(realpath -- "${t}")" || return 2
    find "${pr}" -type f -print0 | while IFS= read -r -d '' rf; do
      printf "%s\0" "${t}${rf#"${pr}"}"
    done || st=2
  done
  (
    cd "${DC_ROOT}" || exit 2
    find . \( -path ./.git -o -path ./target -o -path ./logs -o -path ./log \) -prune -o -type f -name "*.log" -print0
  ) | while IFS= read -r -d '' f; do printf "%s\0" "${DC_ROOT}/${f#./}"; done || st=2
  return "${st}"
}

lf="$(mktemp)" || exit 2
if ! list_files > "${lf}"; then
  rm -f "${lf}"
  echo "canary: artifact scan failed" >&2
  exit 2
fi
mapfile -d '' -t files < <(
  sort -z -u "${lf}" | while IFS= read -r -d '' f; do
    rel="${f#"${DC_ROOT}"/}"
    if [ "${rel}" != "${f}" ] && dc_is_excluded "${rel}"; then continue; fi
    printf "%s\0" "${f}"
  done
)
rm -f "${lf}"

if [ "${#files[@]}" -eq 0 ]; then
  echo "canary: no artifacts to scan (target/, logs/, log/, *.log and DEMO_CHECK_EXTRA_PATHS are absent or empty)"
  if [ "${DEMO_CHECK_ACCEPTANCE:-}" = "1" ]; then
    echo "canary: ACCEPTANCE MODE: no artifacts to scan is a failure" >&2
    exit 1
  fi
  echo "canary: PASS with note: nothing was scanned"
  exit 0
fi

pf="$(mktemp)"
trap 'rm -f "${pf}"' EXIT
printf "%s\n" "${markers}" > "${pf}"

hits=0
mapfile -d '' -t hitfiles < <(printf "%s\0" "${files[@]}" | xargs -0 -r grep -laFZ -f "${pf}" --)
for f in "${hitfiles[@]}"; do
  [ -n "${f}" ] || continue
  disp="${f#"${DC_ROOT}"/}"
  while IFS= read -r m; do
    if grep -qI . "${f}"; then
      while IFS= read -r ln; do
        [ -n "${ln}" ] && { echo "${disp}:${ln}: canary ${m}"; hits=$((hits + 1)); }
      done < <(grep -nF -e "${m}" -- "${f}" | cut -d: -f1)
    else
      n="$(grep -a -c -F -e "${m}" -- "${f}" || true)"
      [ "${n:-0}" -gt 0 ] && { echo "${disp}:(binary): canary ${m} (${n}件)"; hits=$((hits + 1)); }
    fi
  done <<< "${markers}"
done

echo "canary: scanned ${#files[@]} file(s), ${hits} hit(s)"
[ "${hits}" -eq 0 ]
