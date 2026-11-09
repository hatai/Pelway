#!/usr/bin/env bash
# demo-check 段階 (c) external-refs: 製品ソース・資産・ビルド成果物に外部参照が無いことの検査。
# 許可ホストは allowed-hosts.txt（リード確定 2026-11-09 15:01/15:03。127.0.0.1 と ::1 だけ。localhost は不可）。
# 角括弧なしの "localhost" のような点を含まない語はホスト名リテラルとしては検出しない（URL 内の localhost は検出する）。
# 検出するもの:
#   1. http:// https:// ws:// wss:// の URL（ホスト部を取り出して許可リストと照合）
#   2. スキーム省略の //host（HTML/CSS/JS/SVG の src= href= url( @import の直後だけ）
#   3. 文字列リテラル "…" 全体がホスト名（例 "fonts.example.com" "api.example.com:443"）か IPv4 アドレス
# 例外（ネットワーク参照ではないもの）:
#   xmlns="…" / xmlns:xx="…" 属性の値（XML 名前空間の識別子。取得されない）
# 違反は「ファイル:行: ホスト」で出す（URL の残り＝パスやクエリは出さない）。exit 0 合格 / 1 違反 / 2 検査異常。
set -u
# shellcheck source=lib.sh
. "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

list="${DC_LIST_DIR}/allowed-hosts.txt"
allowed="$(dc_list_lines "${list}")" || exit 2
mapfile -t files < <(dc_all_files)

is_allowed() {
  local h a
  h="$(printf "%s" "$1" | tr "[:upper:]" "[:lower:]")"
  h="${h#[}"; h="${h%]}"
  while IFS= read -r a; do [ "${h}" = "${a}" ] && return 0; done <<< "${allowed}"
  return 1
}

# URL からホストを取り出す（userinfo・ポート・パスを除く。IPv6 は [..] を保つ）
url_host() {
  local u="${1#*://}"
  u="${u%%[/?#\"\'\\<> ]*}"
  u="${u##*@}"
  if [[ "${u}" == \[* ]]; then
    u="${u%%]*}]"
  else
    u="${u%%:*}"
  fi
  printf "%s" "${u}"
}

tlds='com|net|org|io|dev|app|ai|jp|co|cloud|info|biz|me|xyz|us|uk|eu|de|cn|ru|so|sh|gg|tv|ly|to|page|site|online|tech'
url_re='(https?|wss?)://[^[:space:]"'"'"'<>)]+'
pr_re='(src=|href=|url\(|@import)[[:space:]]*["'"'"']?//[A-Za-z0-9.-]+'
host_re='"([A-Za-z0-9-]+\.)+('"${tlds}"')(:[0-9]+)?"|"([0-9]{1,3}\.){3}[0-9]{1,3}(:[0-9]+)?"'

hits=0
report() { echo "$1: external host '$2'"; hits=$((hits + 1)); }

for f in "${files[@]}"; do
  path="${DC_ROOT}/${f}"
  if dc_is_wasm "${f}"; then
    while IFS= read -r m; do
      h="$(url_host "${m}")"
      is_allowed "${h}" || report "${f}:(binary)" "${h}"
    done < <(grep -a -oE "${url_re}" -- "${path}" | sort -u)
    continue
  fi
  dc_is_text "${f}" || continue
  # xmlns 属性を消してから探す（行番号は保たれる）
  stripped="$(sed -E 's/xmlns(:[A-Za-z0-9_.-]+)?[[:space:]]*=[[:space:]]*("[^"]*"|'"'"'[^'"'"']*'"'"')//g' -- "${path}")"
  while IFS=: read -r ln m; do
    [ -n "${m:-}" ] || continue
    h="$(url_host "${m}")"
    is_allowed "${h}" || report "${f}:${ln}" "${h}"
  done < <(printf "%s\n" "${stripped}" | grep -noE "${url_re}")
  while IFS=: read -r ln m; do
    [ -n "${m:-}" ] || continue
    h="${m##*//}"
    is_allowed "${h}" || report "${f}:${ln}" "${h}"
  done < <(printf "%s\n" "${stripped}" | grep -noE "${pr_re}")
  while IFS=: read -r ln m; do
    [ -n "${m:-}" ] || continue
    h="${m//\"/}"; h="${h%%:*}"
    is_allowed "${h}" || report "${f}:${ln}" "${h}"
  done < <(printf "%s\n" "${stripped}" | grep -noE "${host_re}")
done

echo "external-refs: scanned ${#files[@]} file(s), ${hits} hit(s)"
[ "${hits}" -eq 0 ]
