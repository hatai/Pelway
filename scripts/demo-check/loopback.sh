#!/usr/bin/env bash
# demo-check 段階 (a) loopback: デモの待受・接続先が 127.0.0.1 か ::1 だけであることの検査。
# 許可値はリード確定 2026-11-09 15:01/15:03: 127.0.0.1 と ::1 の完全一致だけ（localhost・0.0.0.0・127.0.0.10 などは不可）。
#
# static（実施・失敗で exit 1）:
#   1. mise.toml・直下の *.env* ・crates 配下の *.toml にある PELWAY_BIND_ADDR の値が
#      127.0.0.1:<port> か [::1]:<port> であること。<port> は 0〜65535（0 は自動割り当て）。
#      mise.toml に無い場合も失敗。
#   2. 製品ソース・資産（lib.sh の走査範囲）に全アドレス待受の記述が無いこと:
#      0.0.0.0 / "::" / [::] / Ipv4Addr::UNSPECIFIED / Ipv6Addr::UNSPECIFIED / INADDR_ANY / in6addr_any
#      （// で始まるコメント行も対象。誤検出の場合は QA と合意のうえ除外を追加する）
#   3. PELWAY_BIND_ADDR を経由しない bind(...) の引数も許可値と完全一致で照合する（DC-01）。
#      例: TcpListener::bind("localhost:8080") や 127.0.0.1 以外の IP は違反。
#      引数が "..." の文字列リテラルで書かれている場合だけが対象（変数は静的に見えないため）。
#      同じ行に複数の bind(...) があっても、すべての呼び出しを検査する（slot5 指摘1）。
#      対象は関連付け関数の形 ::bind("..."）だけ（Issue #143 の移植で絞った）:
#      sqlx などのメソッド形式 .bind("...") はソケットの待受・接続先ではないため対象外。
#      メソッド形式のソケット bind に文字列リテラルを渡す形は検出しない（既知の限界）。
# runtime（未実装）:
#   デモ起動中の待受ソケットを列挙して許可値以外を検出する検査は、デモの起動方法が決まってから実装する。
#   既定では "runtime: PENDING (NOT IMPLEMENTED)" と表示し、static 合格なら exit 0（runtime は未検査）。
#   DEMO_CHECK_ACCEPTANCE=1 のときは runtime PENDING を受入不合格として exit 1 にする。
# exit 0 = static 合格（runtime は未検査、ACCEPTANCE 未設定時）/ 1 = static 違反 or ACCEPTANCE で runtime PENDING /
# 2 = 検査異常（リスト欠落・設定やソースの走査失敗・bind 走査の停滞）
set -uo pipefail
# shellcheck source=lib.sh
. "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

hits=0
report() { echo "$1: $2"; hits=$((hits + 1)); }

# 待受・接続先の宛先（PELWAY_BIND_ADDR の値と bind("...") の引数の両方）の許可判定。
# ホストは 127.0.0.1 か ::1（[::1] の角括弧つきも可）の完全一致、ポートは 0〜65535（DC-01/DC-06）。
ok_bind_target() {
  local a="$1" host port
  if [[ "${a}" =~ ^\[([0-9A-Fa-f:.]+)\]:([0-9]{1,5})$ ]]; then
    host="${BASH_REMATCH[1]}"; port="${BASH_REMATCH[2]}"
  elif [[ "${a}" =~ ^([0-9A-Za-z.-]+):([0-9]{1,5})$ ]]; then
    host="${BASH_REMATCH[1]}"; port="${BASH_REMATCH[2]}"
  else
    return 1
  fi
  [ "${host}" = "127.0.0.1" ] || [ "${host}" = "::1" ] || return 1
  (( 10#${port} <= 65535 ))
}

# 1. PELWAY_BIND_ADDR（走査に失敗したら exit 2）
cfgtmp="$(mktemp)" || exit 2
(
  cd "${DC_ROOT}" || exit 2
  st=0
  [ -f mise.toml ] && printf "mise.toml\0"
  find . -maxdepth 1 -type f \( -name ".env" -o -name ".env.*" -o -name "*.env" \) -printf "%P\0" || st=2
  if [ -d crates ]; then
    find crates -type f -name "*.toml" -print0 | while IFS= read -r -d '' f; do
      dc_is_excluded "${f}" || printf "%s\0" "${f}"
    done || st=2
  fi
  exit "${st}"
) > "${cfgtmp}" || { rm -f "${cfgtmp}"; echo "loopback: config scan failed" >&2; exit 2; }
mapfile -d '' -t cfgs < "${cfgtmp}"
rm -f "${cfgtmp}"
found_in_mise=0
for f in "${cfgs[@]}"; do
  while IFS=: read -r ln rest; do
    [ -n "${ln:-}" ] || continue
    v="$(printf "%s" "${rest}" | sed -E 's/^[^=]*=[[:space:]]*//; s/[[:space:]]*(#.*)?$//; s/^["'"'"']//; s/["'"'"']$//')"
    [ "${f}" = "mise.toml" ] && found_in_mise=1
    if ! ok_bind_target "${v}"; then
      report "${f}:${ln}" "PELWAY_BIND_ADDR is not 127.0.0.1:<port 0-65535> or [::1]:<port 0-65535>"
    fi
  done < <(grep -nE '^[[:space:]]*(export[[:space:]]+)?PELWAY_BIND_ADDR[[:space:]]*=' -- "${DC_ROOT}/${f}")
done
[ "${found_in_mise}" -eq 1 ] || report "mise.toml" "PELWAY_BIND_ADDR not set (expected 127.0.0.1:<port> or [::1]:<port>)"

# 2. 全アドレス待受の記述、3. bind(...) の引数
any_re='(^|[^0-9.])0\.0\.0\.0([^0-9.]|$)|"::"|\[::\]|Ipv4Addr::UNSPECIFIED|Ipv6Addr::UNSPECIFIED|INADDR_ANY|in6addr_any'
bind_scan_re='::[[:space:]]*bind[[:space:]]*\([[:space:]]*"'
bind_cap_re='::[[:space:]]*bind[[:space:]]*\([[:space:]]*"([^"]*)"'
dc_load_files files
for f in "${files[@]}"; do
  dc_is_wasm "${f}" && continue
  dc_is_text "${f}" || continue
  while IFS= read -r ln; do
    [ -n "${ln}" ] && report "${f}:${ln}" "unspecified (all-interfaces) address"
  done < <(grep -nE "${any_re}" -- "${DC_ROOT}/${f}" | cut -d: -f1)
  # PELWAY_BIND_ADDR を経由しない bind("...") の宛先。同じ行に複数あってもすべて検査する（指摘1）。
  # 判定関数内の [[ =~ ]] が BASH_REMATCH を上書きするため、一致全体と引数は先にコピーして使う
  # （Codex r2 指摘1）。次の検索位置は「rest 内の一致全体の直後」とする: マッチ長だけ先頭から
  # 進めると一致の開始位置に届かず、同じ bind を再検査して多重報告する。進まなければ exit 2。
  while IFS=: read -r ln line; do
    [ -n "${line:-}" ] || continue
    rest="${line}"
    while [[ "${rest}" =~ ${bind_cap_re} ]]; do
      m="${BASH_REMATCH[0]}"
      arg="${BASH_REMATCH[1]}"
      ok_bind_target "${arg}" || report "${f}:${ln}" "bind argument is not 127.0.0.1:<port 0-65535> or [::1]:<port 0-65535>"
      nxt="${rest#*"${m}"}"
      if [ "${#nxt}" -eq "${#rest}" ]; then
        echo "loopback: bind scan did not advance at ${f}:${ln}" >&2
        exit 2
      fi
      rest="${nxt}"
    done
  done < <(grep -nE "${bind_scan_re}" -- "${DC_ROOT}/${f}")
done

echo "loopback: static scanned ${#cfgs[@]} config + ${#files[@]} source file(s), ${hits} hit(s)"
echo "loopback: runtime: PENDING (NOT IMPLEMENTED; listening sockets are not checked yet)"
if [ "${hits}" -ne 0 ]; then
  exit 1
fi
if [ "${DEMO_CHECK_ACCEPTANCE:-}" = "1" ]; then
  echo "loopback: ACCEPTANCE MODE: runtime PENDING is a failure" >&2
  exit 1
fi
echo "loopback: static PASS (runtime unchecked)"
exit 0
