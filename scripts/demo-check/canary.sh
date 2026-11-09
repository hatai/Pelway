#!/usr/bin/env bash
# demo-check 段階 (d) canary: 合成データに埋め込んだ目印（canaries.txt）が、出てはいけない場所に無いことの検査。
# 走査対象:
#   target/ 配下のすべて（ビルド成果物。バイナリも grep -a で件数を数える）
#   logs/ log/ 配下のすべて、ワークツリー内の *.log（.git と上記を除く）
#   DEMO_CHECK_EXTRA_PATHS（コロン区切り。端末の保存領域などワークツリー外のパス）。存在しないパスは exit 2
# 除外: lib.sh の除外（scripts/demo-check/、tests/fixtures/、testdata/、docs/ など）。ワークツリー外のパスには適用しない。
# シンボリックリンク: 走査の途中で出会ったリンクはたどらない（DC-03）。
#   暗黙の走査ルート（target/ logs/ log/）が作業ツリーの外を指すリンクのときは、外を読まずに
#   検査異常（exit 2）で止める（slot5 指摘2）。例外は作業ツリー直下の target だけ:
#   実体を解決すると $HOME/.cache/mbx/targets/（mr-boxington のビルドキャッシュ）の下を指す
#   ときは、その中身を走査する（開発リード決定 2026-11-09 17:56。リンクを通した論理パスで報告する）。
#   DEMO_CHECK_EXTRA_PATHS はワークツリー外のパスを明示的に指定するものなので、この検査の対象にしない。
# ファイル名の区切り: 改行を含むファイル名でも壊れないよう NUL 区切りで扱う（DC-04）。
# 走査するものが 1 つも無いときは "scanned 0 file(s)" と "no artifacts to scan" を表示して
#   exit 1（未検証の失敗。受入・通常で共通。slot6）。
# 違反は「ファイル:行: 目印」（バイナリは「ファイル:(binary): 目印 (N件)」）で出す。行の中身は出さない。
# 目印検索の前に、対象ファイルがすべて読めることを確認する。読めなければ exit 2（slot5 指摘4）。
# exit 0 合格 / 1 目印あり or 走査対象ゼロ（未検証） / 2 検査異常（リスト欠落・走査失敗を含む）
set -uo pipefail
# shellcheck source=lib.sh
. "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

list="${DC_LIST_DIR}/canaries.txt"
markers="$(dc_list_lines "${list}")" || exit 2
[ -n "${markers}" ] || { echo "canary: no markers in ${list}" >&2; exit 2; }

implicit_targets=()
[ -e "${DC_ROOT}/target" ] && implicit_targets+=("${DC_ROOT}/target")
for d in logs log; do [ -d "${DC_ROOT}/${d}" ] && implicit_targets+=("${DC_ROOT}/${d}"); done

# 暗黙の走査ルートが作業ツリーの中に解決されることを確かめてから走査する（slot5 指摘2）。
# 例外は作業ツリー直下の target だけ（開発リード決定 2026-11-09 17:56）: 実体を解決すると
# $HOME/.cache/mbx/targets/（mr-boxington のビルドキャッシュ）の下を指すときは走査する。
# それ以外の外を指す走査ルートは、読まずに exit 2 で止める。
root_real="$(realpath -- "${DC_ROOT}")" || { echo "canary: cannot resolve ${DC_ROOT}" >&2; exit 2; }
mbx_root="${HOME}/.cache/mbx/targets"
mbx_real=""
if [ -d "${mbx_root}" ]; then
  mbx_real="$(realpath -- "${mbx_root}")" || mbx_real=""
fi
targets=()
for t in "${implicit_targets[@]}"; do
  t_real="$(realpath -- "${t}")" || { echo "canary: cannot resolve ${t#"${DC_ROOT}"/}" >&2; exit 2; }
  inside=0
  case "${t_real}" in
    "${root_real}" | "${root_real}"/*) inside=1 ;;
  esac
  if [ "${inside}" -eq 0 ] && [ "${t}" = "${DC_ROOT}/target" ] && [ -n "${mbx_real}" ]; then
    case "${t_real}" in
      "${mbx_real}" | "${mbx_real}"/*) inside=1 ;;
    esac
  fi
  if [ "${inside}" -eq 0 ]; then
    echo "canary: scan root resolves outside the work tree (not the mbx cache): ${t#"${DC_ROOT}"/} -> ${t_real}" >&2
    exit 2
  fi
  targets+=("${t}")
done
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
  echo "canary: scanned 0 file(s), 0 hit(s)"
  echo "canary: no artifacts to scan (target/, logs/, log/, *.log and DEMO_CHECK_EXTRA_PATHS are absent or empty)" >&2
  exit 1
fi

# 目印検索の前に、対象ファイルがすべて読めることを確認する（読めなければ検査異常 exit 2。指摘4）
for f in "${files[@]}"; do
  if [ ! -r "${f}" ]; then
    echo "canary: cannot read: ${f#"${DC_ROOT}"/}" >&2
    exit 2
  fi
done

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
