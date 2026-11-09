#!/usr/bin/env bash
# demo-check 段階 (c) external-refs: 製品ソース・資産・ビルド成果物に外部参照が無いことの検査。
# 許可ホストは allowed-hosts.txt（リード確定 2026-11-09 15:01/15:03。127.0.0.1 と ::1 だけ。localhost は不可）。
# 角括弧なしの "localhost" のような点を含まない語はホスト名リテラルとしては検出しない（URL 内の localhost は検出する）。
# 検出するもの:
#   1. http:// https:// ws:// wss:// の URL（ホスト部を取り出して許可リストと照合）
#   2. スキーム省略の //host:
#      a) HTML/CSS/JS/SVG の src= href= url( @import の直後
#      b) 引用符（" ' `）の直後の //ホスト名（JS などの文字列の中。DC-02）
#         ホスト部は「ドット付き TLD で終わる」か IPv4 の形だけに限る（//localhost のような
#         点なし語や path//file は対象外。行頭の // コメントは引用符が無いため対象外）
#   3. 文字列リテラル "…" 全体がホスト名（例 "fonts.example.com" "api.example.com:443"）か IPv4 アドレス
# 例外（ネットワーク参照ではないもの）:
#   xmlns="…" / xmlns:xx="…" 属性の値（XML 名前空間の識別子。取得されない）
#   行コメント（行頭または空白の直後の // から行末まで）。引用符で囲んだ例示 URL を
#   コメントに書いても外部参照に数えない（slot5 指摘3）。引用符の直前の // は
#   文字列内のプロトコル相対参照なので消さない
# 同じファイル:行:ホストの重複報告は 1 件にまとめる（複数の規則が同じものを見つけたとき）。
# 違反は「ファイル:行: ホスト」で出す（URL の残り＝パスやクエリは出さない）。exit 0 合格 / 1 違反 / 2 検査異常。
set -uo pipefail
# shellcheck source=lib.sh
. "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

list="${DC_LIST_DIR}/allowed-hosts.txt"
allowed="$(dc_list_lines "${list}")" || exit 2
dc_load_files files

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

# 文字列内の // の直後に来る語がホスト名の形か（ドット付き TLD で終わる、または IPv4）
is_host_shape() {
  [[ "$1" =~ \.(${tlds})$ ]] || [[ "$1" =~ ^([0-9]{1,3}\.){3}[0-9]{1,3}$ ]]
}

tlds='com|net|org|io|dev|app|ai|jp|co|cloud|info|biz|me|xyz|us|uk|eu|de|cn|ru|so|sh|gg|tv|ly|to|page|site|online|tech'
url_re='(https?|wss?)://[^[:space:]"'"'"'<>)]+'
pr_re='(src=|href=|url\(|@import)[[:space:]]*["'"'"']?//[A-Za-z0-9.-]+'
qpr_re='(^|[[:space:]([{,=])["'"'"'`]//[A-Za-z0-9.-]+'
host_re='"([A-Za-z0-9-]+\.)+('"${tlds}"')(:[0-9]+)?"|"([0-9]{1,3}\.){3}[0-9]{1,3}(:[0-9]+)?"'

hits=0
declare -A seen=()
report() {
  local key="$1|$2"
  [ -n "${seen[${key}]:-}" ] && return
  seen["${key}"]=1
  echo "$1: external host '$2'"
  hits=$((hits + 1))
}

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
  # xmlns 属性を消し、さらに文字列外の行コメント（行頭か空白の直後の // から行末）を消してから
  # 探す（行番号は保たれる）。引用符の直前の // は文字列内の参照なので残る
  stripped="$(sed -E 's/xmlns(:[A-Za-z0-9_.-]+)?[[:space:]]*=[[:space:]]*("[^"]*"|'"'"'[^'"'"']*'"'"')//g' -- "${path}")"
  nocomment="$(printf "%s\n" "${stripped}" | sed -E 's@(^|[[:space:]])//.*$@@')"
  while IFS=: read -r ln m; do
    [ -n "${m:-}" ] || continue
    h="$(url_host "${m}")"
    is_allowed "${h}" || report "${f}:${ln}" "${h}"
  done < <(printf "%s\n" "${nocomment}" | grep -noE "${url_re}")
  while IFS=: read -r ln m; do
    [ -n "${m:-}" ] || continue
    h="${m##*//}"
    is_allowed "${h}" || report "${f}:${ln}" "${h}"
  done < <(printf "%s\n" "${nocomment}" | grep -noE "${pr_re}")
  # 引用符の直後の //ホスト（JS などの文字列内のプロトコル相対参照。DC-02）
  while IFS=: read -r ln m; do
    [ -n "${m:-}" ] || continue
    h="${m##*//}"
    is_host_shape "${h}" || continue
    is_allowed "${h}" || report "${f}:${ln}" "${h}"
  done < <(printf "%s\n" "${nocomment}" | grep -noE "${qpr_re}")
  while IFS=: read -r ln m; do
    [ -n "${m:-}" ] || continue
    h="${m//\"/}"; h="${h%%:*}"
    is_allowed "${h}" || report "${f}:${ln}" "${h}"
  done < <(printf "%s\n" "${nocomment}" | grep -noE "${host_re}")
done

echo "external-refs: scanned ${#files[@]} file(s), ${hits} hit(s)"
[ "${hits}" -eq 0 ]
