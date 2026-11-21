#!/usr/bin/env bash
# scripts/conductor-rebuild のテスト。偽の t3（T3_BIN）が受けた呼び出しを記録し、
# 旧指揮役の停止 → 新しい指揮役の起動（本体 checkout、worktree なし）→ 宛先の登録、の順と引数を確かめる。
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT
failures=()
LOG="$ROOT/t3-calls"
FILE="$ROOT/home/.config/pelway-agent-notify/conductor-thread-id"
OLD="mcp:launch:old-conductor"

cat > "$ROOT/t3" <<'T3'
#!/usr/bin/env bash
# 偽の t3。呼び出しを 1 行ずつ記録し、標準入力の本文は別のファイルに残す。
echo "$*" >> "$LOG"
case "$1 $2" in
  "thread show") echo '{"thread":{"threadId":"'"$3"'","providerInstanceId":"acpRegistry_devin","model":"swe-2-high"}}' ;;
  "thread send") cat > "$LOG.send-stdin"; [[ -e "$LOG.send-fail" ]] && exit 1; echo "ok" ;;
  "thread start") cat > "$LOG.start-stdin"; [[ -e "$LOG.start-fail" ]] && exit 1; echo "mcp:launch:new-conductor" ;;
  *) exit 9 ;;
esac
T3
chmod +x "$ROOT/t3"

# 偽の見張り役（インストールした pelway-agent-watch の代わり）。--register <id> で登録ファイルに書く
mkdir -p "$ROOT/home/.local/bin"
cat > "$ROOT/home/.local/bin/pelway-agent-watch" <<'W'
#!/usr/bin/env bash
[[ "$1" == --register ]] || exit 9
mkdir -p "$HOME/.config/pelway-agent-notify"
printf '%s\n' "$2" > "$HOME/.config/pelway-agent-notify/conductor-thread-id"
W
chmod +x "$ROOT/home/.local/bin/pelway-agent-watch"

run() {
  rm -f "$LOG" "$LOG".*; [[ -n "${1:-}" ]] && touch "$LOG.$1"
  shift || true
  HOME="$ROOT/home" T3_BIN="$ROOT/t3" LOG="$LOG" PELWAY_REBUILD_ALLOW_WORKTREE=1 \
    bash "$REPO/scripts/conductor-rebuild" "$@" >"$ROOT/out" 2>&1
}
check() {
  local name="$1"; shift
  if "$@"; then printf 'ok   %s\n' "$name"; else printf 'FAIL %s\n' "$name"; failures+=("$name"); fi
}
line() { sed -n "${1}p" "$LOG"; }

# 既定: 旧指揮役の provider・model を引き継ぐ
run "" --from "$OLD"; rc=$?
check "succeeds" [ "$rc" -eq 0 ]
check "1st call reads the old conductor" [ "$(line 1)" = "thread show $OLD --json" ]
check "2nd call stops the old conductor with --wait" [ "$(line 2)" = "thread send --wait $OLD -" ]
check "stop message asks to halt and write the handover" grep -q "恒久交代を始める" "$LOG.send-stdin"
check "3rd call starts the new conductor" grep -q "^thread start " <(line 3)
no_arg() { ! grep -q -- "$1" <(line 3); }
check "new conductor is not in a worktree" no_arg "--worktree"
check "new conductor inherits provider and model" grep -q -- "--provider acpRegistry_devin --model swe-2-high" <(line 3)
check "no effort option unless given" no_arg "--option"
check "start message names the old conductor" grep -q "$OLD" "$LOG.start-stdin"
check "start message follows the handover procedure" grep -q "恒久交代" "$LOG.start-stdin"
check "start message asks to settle the old conductor after confirmation" grep -q "settle" "$LOG.start-stdin"
check "start message asks to recreate the scheduled task" grep -q "scheduled task" "$LOG.start-stdin"
check "registers the new conductor" [ "$(cat "$FILE" 2>/dev/null)" = "mcp:launch:new-conductor" ]

# 引数で指定した値が渡る
run "" --from "$OLD" --provider codex --model gpt-6.2-sol --effort high
check "explicit provider/model/effort are passed" grep -q -- "--provider codex --model gpt-6.2-sol --option effort=high" <(line 3)

# 失敗したら登録を変えずに止まる
printf 'mcp:launch:kept\n' > "$FILE"
run send-fail --from "$OLD"; rc=$?
check "stop failure exits non-zero" [ "$rc" -ne 0 ]
no_start() { ! grep -q "^thread start" "$LOG"; }
check "stop failure does not start a new conductor" no_start
check "stop failure keeps the registration" [ "$(cat "$FILE")" = "mcp:launch:kept" ]
run start-fail --from "$OLD"; rc=$?
check "start failure exits non-zero" [ "$rc" -ne 0 ]
check "start failure keeps the registration" [ "$(cat "$FILE")" = "mcp:launch:kept" ]

# スレッド ID の検査
run "" --from "--help"; rc=$?
check "rejects an id starting with -" [ "$rc" -ne 0 ]
run "" --from 'a;rm'; rc=$?
check "rejects odd characters" [ "$rc" -ne 0 ]
check "rejected ids make no t3 calls" [ ! -s "$LOG" ]

# mise のタスクから同じスクリプトを呼べる（引数は -- の後ろがそのまま渡る）
task_runs() { /usr/bin/python3 -I -c 'import sys, tomllib
t = tomllib.load(open(sys.argv[1], "rb"))["tasks"][sys.argv[2]]
sys.exit(0 if t.get("run") == sys.argv[3] else 1)' "$REPO/mise.toml" "$1" "$2"; }
check "mise task conductor-rebuild runs the script" task_runs conductor-rebuild "bash scripts/conductor-rebuild"
no_task() { /usr/bin/python3 -I -c 'import sys, tomllib
sys.exit(1 if sys.argv[2] in tomllib.load(open(sys.argv[1], "rb"))["tasks"] else 0)' "$REPO/mise.toml" "$1"; }
check "no mise task for the removed conductor-register" no_task conductor-register

# 見張り役が入っていなければ、何も送らずに止まる
mv "$ROOT/home/.local/bin/pelway-agent-watch" "$ROOT/watch.bak"
run "" --from "$OLD"; rc=$?
check "missing pelway-agent-watch exits non-zero" [ "$rc" -ne 0 ]
check "missing pelway-agent-watch makes no t3 calls" [ ! -s "$LOG" ]
mv "$ROOT/watch.bak" "$ROOT/home/.local/bin/pelway-agent-watch"

# worktree からの起動は止まる（PELWAY_REBUILD_ALLOW_WORKTREE を付けない）
wt="$ROOT/wt"
git -C "$REPO" worktree add -q --detach "$wt" HEAD 2>/dev/null
if [[ -d "$wt" ]]; then
  rm -f "$LOG"
  HOME="$ROOT/home" T3_BIN="$ROOT/t3" LOG="$LOG" bash "$wt/scripts/conductor-rebuild" --from "$OLD" >"$ROOT/out" 2>&1; rc=$?
  git -C "$REPO" worktree remove --force "$wt"
  check "refuses to run from a worktree" [ "$rc" -ne 0 ]
  check "worktree refusal makes no t3 calls" [ ! -s "$LOG" ]
fi

if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}"
  exit 1
fi
printf '\nall passed\n'
