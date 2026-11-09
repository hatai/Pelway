#!/usr/bin/env bash
# demo-check 段階 (a) loopback: デモの待受・接続先が 127.0.0.1 か ::1 だけであることの検査。
# 許可値はリード確定 2026-11-09 15:01/15:03: 127.0.0.1 と ::1 の完全一致だけ（localhost・0.0.0.0・127.0.0.10 などは不可）。
#
# static（実施・失敗で exit 1）:
#   1. mise.toml・直下の *.env* ・crates 配下の *.toml にある PELWAY_BIND_ADDR の値が
#      127.0.0.1:<port> か [::1]:<port> であること（ポート 0 = 自動割り当ても可）。mise.toml に無い場合も失敗。
#   2. 製品ソース・資産（lib.sh の走査範囲）に全アドレス待受の記述が無いこと:
#      0.0.0.0 / "::" / [::] / Ipv4Addr::UNSPECIFIED / Ipv6Addr::UNSPECIFIED / INADDR_ANY / in6addr_any
#      （// で始まるコメント行も対象。誤検出の場合は QA と合意のうえ除外を追加する）
# runtime（未実装・失敗扱いにしない）:
#   デモ起動中の待受ソケットを列挙して許可値以外を検出する検査は、デモの起動方法が決まってから実装する。
#   それまでは "runtime: PENDING (NOT IMPLEMENTED)" と必ず表示し、static の合格を runtime の合格と見なさない。
# exit 0 = static 合格（runtime は未検査）/ 1 = static 違反 / 2 = 検査異常
set -u
# shellcheck source=lib.sh
. "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

hits=0
report() { echo "$1: $2"; hits=$((hits + 1)); }
bind_re='^(127\.0\.0\.1|\[::1\]):[0-9]{1,5}$'

# 1. PELWAY_BIND_ADDR
mapfile -t cfgs < <(
  cd "${DC_ROOT}" || exit 2
  [ -f mise.toml ] && echo mise.toml
  find . -maxdepth 1 -type f \( -name ".env" -o -name ".env.*" -o -name "*.env" \) -printf "%P\n"
  [ -d crates ] && find crates -type f -name "*.toml" | while IFS= read -r f; do dc_is_excluded "${f}" || echo "${f}"; done
)
found_in_mise=0
for f in "${cfgs[@]}"; do
  while IFS=: read -r ln rest; do
    [ -n "${ln:-}" ] || continue
    v="$(printf "%s" "${rest}" | sed -E 's/^[^=]*=[[:space:]]*//; s/[[:space:]]*(#.*)?$//; s/^["'"'"']//; s/["'"'"']$//')"
    [ "${f}" = "mise.toml" ] && found_in_mise=1
    if [[ ! "${v}" =~ ${bind_re} ]]; then
      report "${f}:${ln}" "PELWAY_BIND_ADDR is not 127.0.0.1:<port> or [::1]:<port>"
    fi
  done < <(grep -nE '^[[:space:]]*(export[[:space:]]+)?PELWAY_BIND_ADDR[[:space:]]*=' -- "${DC_ROOT}/${f}")
done
[ "${found_in_mise}" -eq 1 ] || report "mise.toml" "PELWAY_BIND_ADDR not set (expected 127.0.0.1:<port> or [::1]:<port>)"

# 2. 全アドレス待受の記述
any_re='(^|[^0-9.])0\.0\.0\.0([^0-9.]|$)|"::"|\[::\]|Ipv4Addr::UNSPECIFIED|Ipv6Addr::UNSPECIFIED|INADDR_ANY|in6addr_any'
mapfile -t files < <(dc_all_files)
for f in "${files[@]}"; do
  dc_is_wasm "${f}" && continue
  dc_is_text "${f}" || continue
  while IFS= read -r ln; do
    [ -n "${ln}" ] && report "${f}:${ln}" "unspecified (all-interfaces) address"
  done < <(grep -nE "${any_re}" -- "${DC_ROOT}/${f}" | cut -d: -f1)
done

echo "loopback: static scanned ${#cfgs[@]} config + ${#files[@]} source file(s), ${hits} hit(s)"
echo "loopback: runtime: PENDING (NOT IMPLEMENTED; listening sockets are not checked yet)"
if [ "${hits}" -eq 0 ]; then
  echo "loopback: static PASS (runtime unchecked)"
  exit 0
fi
exit 1
