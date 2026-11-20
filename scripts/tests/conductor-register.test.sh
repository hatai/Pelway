#!/usr/bin/env bash
# scripts/conductor-register のテスト。HOME を一時ディレクトリに向け、登録ファイル
# （~/.config/pelway-agent-notify/conductor-thread-id）の書き込み・消去・表示と、ID の検査を確かめる。
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT
failures=()
REG="$REPO/scripts/conductor-register"
FILE="$ROOT/.config/pelway-agent-notify/conductor-thread-id"

run() { HOME="$ROOT" bash "$REG" "$@"; }
check() {
  local name="$1"; shift
  if "$@"; then printf 'ok   %s\n' "$name"; else printf 'FAIL %s\n' "$name"; failures+=("$name"); fi
}

run "mcp:fe379cab-84fc-4bdd-841e-aeea82129d66" >/dev/null
check "registers the thread id" [ "$(cat "$FILE")" = "mcp:fe379cab-84fc-4bdd-841e-aeea82129d66" ]
check "file is owner-only" [ "$(stat -c %a "$FILE")" = "600" ]
check "--show prints the id" [ "$(run --show)" = "mcp:fe379cab-84fc-4bdd-841e-aeea82129d66" ]

run "thread:delegated-task:command%3Amcp%3A1c7f63d3" >/dev/null
check "re-registering replaces the id" [ "$(cat "$FILE")" = "thread:delegated-task:command%3Amcp%3A1c7f63d3" ]

for bad in "--help" "-x" "a b" 'a;rm' "" "$(printf 'a%.0s' {1..257})"; do
  run "$bad" >/dev/null 2>&1; rc=$?
  check "rejects '${bad:0:12}' (exit != 0)" [ "$rc" -ne 0 ]
done
check "rejected ids leave the file unchanged" [ "$(cat "$FILE")" = "thread:delegated-task:command%3Amcp%3A1c7f63d3" ]

run --clear >/dev/null
check "--clear removes the file" [ ! -e "$FILE" ]
check "--show without a registration says none" [ "$(run --show)" = "(none)" ]
run >/dev/null 2>&1; rc=$?
check "no argument is a usage error" [ "$rc" -ne 0 ]

if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}"
  exit 1
fi
printf '\nall passed\n'
