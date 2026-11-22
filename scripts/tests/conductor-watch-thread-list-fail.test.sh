#!/usr/bin/env bash
# scripts/conductor-watch.sh の t3 thread list の取得失敗の扱い（Issue #651）のテスト。
# モックの t3 を成功・失敗に切り替え、WATCHER 行、threads.json の保持、失敗中にスレッドに
# 基づく判定が止まること、止まりで conductor にした対象を戻さないことを確かめる。
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ROOT="$(mktemp -d)"
WPID=""
trap '[[ -n "${WPID:-}" ]] && kill "$WPID" 2>/dev/null; rm -rf "$ROOT"' EXIT
failures=()

MOCK="$ROOT/mock"
mkdir -p "$ROOT/repo/scripts" "$ROOT/bin" "$MOCK"
cp "$REPO/scripts/conductor-watch.sh" "$ROOT/repo/scripts/"
git init -q "$ROOT/repo"
PELWAY="$ROOT/repo/.git/pelway"
STATE="$PELWAY/patrol-state.json"
INBOX="$PELWAY/conductor-inbox.jsonl"
SNAP="$ROOT/state/pelway/conductor-watch/threads.json"

cat > "$ROOT/repo/scripts/agent-gh" <<'EOF'
#!/usr/bin/env bash
[[ "$2 $3" == "issue list" ]] && { cat "$MOCK/issues.json"; exit 0; }
[[ "$2 $3" == "pr list" ]] && { cat "$MOCK/prs.json"; exit 0; }
[[ "$2 $3" == "api /repos/hatai/Pelway/issues/events?per_page=100" ]] && { echo '[]'; exit 0; }
exit 9
EOF
chmod +x "$ROOT/repo/scripts/agent-gh"
# モックの t3: $MOCK/t3-mode で応答を切り替える
cat > "$ROOT/bin/t3" <<'EOF'
#!/usr/bin/env bash
[[ "$1 $2" == "thread list" ]] || exit 9
case "$(cat "$MOCK/t3-mode")" in
  ok) cat "$MOCK/threads.json" ;;
  fail) echo '[]'; echo 'No running T3 Code server uses /home/x/.t3' >&2; exit 1 ;;
  silent) exit 3 ;;
  empty) ;;
  invalid) echo '{not json' ;;
  notlist) echo '{"threads":[]}' ;;
  long) printf 'x%.0s' {1..200} >&2; printf '\nsecond line\n' >&2; exit 1 ;;
esac
EOF
chmod +x "$ROOT/bin/t3"
cat > "$ROOT/bin/sleep" <<'EOF'
#!/usr/bin/env bash
n=$(( $(cat "$MOCK/sleeps" 2>/dev/null || echo 0) + 1 ))
echo "$n" > "$MOCK/sleeps"
if [[ -n "${SLEEP_MAX:-}" ]] && (( n >= SLEEP_MAX )); then kill -TERM "$PPID"; exit 0; fi
exec /bin/sleep 0.03
EOF
chmod +x "$ROOT/bin/sleep"

mode() { echo "$1" > "$MOCK/t3-mode"; }
issues() { printf '%s\n' "$1" > "$MOCK/issues.json"; }
prs() { printf '%s\n' "$1" > "$MOCK/prs.json"; }
threads() { printf '%s\n' "$1" > "$MOCK/threads.json"; }
inbox() {
  python3 -I - "$INBOX" "$1" "$2" <<'EOF'
import json, sys, time
p, n, rep = sys.argv[1:4]
line = {"schema": "pelway.agent-watch/v1", "event": "finished", "thread_id": "mcp:impl-" + n,
        "issue_number": int(n), "round": 1, "report": rep,
        "occurred_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())}
open(p, "a").write(json.dumps(line, ensure_ascii=False) + "\n")
EOF
}
# th <id> <status> <title> — スレッド 1 本
th() { printf '{"threadId":"%s","title":"%s","status":"%s","updatedAt":"2026-11-20T10:00:00Z","createdBy":"agent","itemCount":3}' "$1" "$3" "$2"; }
pr41() { printf '[{"number":941,"state":"OPEN","isDraft":%s,"headRefName":"agent/41-x","reviewDecision":null,"statusCheckRollup":[]}]' "$1"; }

reset() {
  rm -rf "$PELWAY" "$ROOT/state"
  mkdir -p "$PELWAY"; : > "$ROOT/out.log"
}

# run_cycles <n> [heartbeat secs] — n 回巡回して止める。出力は out.log に足す
run_cycles() {
  echo 0 > "$MOCK/sleeps"
  MOCK="$MOCK" SLEEP_MAX="$1" PELWAY_WATCH_HEARTBEAT_SECS="${2:-600}" XDG_STATE_HOME="$ROOT/state" \
    PATH="$ROOT/bin:$PATH" T3_BIN="$ROOT/bin/t3" PELWAY_BOARD_QUEUE_DIR="$ROOT/queue" \
    bash "$ROOT/repo/scripts/conductor-watch.sh" >>"$ROOT/out.log" 2>&1 &
  WPID=$!
  wait "$WPID" 2>/dev/null; WPID=""
}

sget() {
  python3 -I -c 'import json,sys
print(json.load(open(sys.argv[1]))["subjects"].get(sys.argv[2], {}).get(sys.argv[3], ""))' "$STATE" "$1" "$2"
}

check() {
  local name="$1"; shift
  if "$@"; then printf 'ok   %s\n' "$name"; else printf 'FAIL %s\n' "$name"; failures+=("$name"); fi
}
eq() { [[ "$1" == "$2" ]]; }
has() { grep -q -- "$2" "$1"; }
nhas() { ! grep -qE -- "$2" "$1"; }
count() { grep -c -- "$2" "$1"; }
same() { cmp -s "$1" "$2"; }
since_of() { grep -o -m1 -- "$2 since=[^ ]*" "$1" | sed 's/.* since=//'; }

# 試験 1: 成功 → 失敗の連続 → 回復。
# #41 は実装スレッドが止まっていて conductor（STALL 済み）。#42 は順番待ちの conductor。
reset
issues '[{"number":41,"state":"OPEN","labels":[{"name":"agent:working"}]},{"number":42,"state":"OPEN","labels":[{"name":"ready-for-agent"}]}]'
prs "$(pr41 true)"
threads "[$(th mcp:impl-41 completed '[実装] #41 何かを直す'),$(th mcp:other running '[実装] #77 別の作業')]"
inbox 41 $'state: working\nhead: none\nnext: 手順 4'
mode ok
run_cycles 1
check "1: stopped impl thread makes #41 conductor before the failure" eq "$(sget 41 waiting_on)" "conductor"
check "1: STALL is printed in the success phase" eq "$(count "$ROOT/out.log" 'STALL #41')" "1"
cp "$SNAP" "$ROOT/snapshot.json"

# 失敗中に起きたスレッドの変化（見張りには見えない）
threads "[$(th mcp:impl-41 completed '[実装] #41 何かを直す'),$(th mcp:other completed '[実装] #77 別の作業'),$(th mcp:bad running 'no-prefix title')]"
mode fail
: > "$ROOT/out.log"; : > "$ROOT/fail.log"
run_cycles 2
cat "$ROOT/out.log" >> "$ROOT/fail.log"
check "1: first failure prints the failed line with the stderr reason" \
  has "$ROOT/out.log" "WATCHER thread-list-failed since=20[^ ]* reason=No running T3 Code server uses /home/x/.t3$"
check "1: failed line is printed once for a run of failures" eq "$(count "$ROOT/out.log" 'WATCHER thread-list-failed')" "1"
check "1: no re-notify before the interval" eq "$(count "$ROOT/out.log" 'thread-list-still-failing')" "0"
FAIL_SINCE="$(since_of "$ROOT/out.log" thread-list-failed)"

# 失敗中に PR が Ready になり、#42 に needs-human が付く。re-notify の間隔は 5 秒に縮める（負荷で 2 回目の巡回が遅れても間隔に届かない幅）
prs "$(pr41 false)"
issues '[{"number":41,"state":"OPEN","labels":[{"name":"agent:working"}]},{"number":42,"state":"OPEN","labels":[{"name":"needs-human"}]}]'
/bin/sleep 5.2
: > "$ROOT/out.log"
run_cycles 2 5
cat "$ROOT/out.log" >> "$ROOT/fail.log"
check "1: still-failing line after the interval, once per interval" \
  eq "$(count "$ROOT/out.log" "WATCHER thread-list-still-failing since=$FAIL_SINCE for=[0-9]*s reason=No running T3 Code server uses /home/x/.t3")" "1"
check "1: threads.json keeps the last successful snapshot" same "$SNAP" "$ROOT/snapshot.json"
check "1: PR change is still reported while blind" has "$ROOT/out.log" "PR #941 agent/41-x OPEN|draft=False"
check "1: label-derived change is still applied while blind" has "$ROOT/out.log" "PATROL #42 waiting_on=human"
check "1: #41 stays conductor while blind" eq "$(sget 41 waiting_on)" "conductor"
check "1: no thread-dependent lines while blind" nhas "$ROOT/fail.log" "^THREAD |^STALL |stalled-45m|bad-title|PATROL #41 "

mode ok
: > "$ROOT/out.log"
run_cycles 1
check "1: recovery line names the first failure time" \
  has "$ROOT/out.log" "WATCHER thread-list-recovered since=$FAIL_SINCE after=[0-9]*s$"
check "1: change made while blind is reported on recovery" has "$ROOT/out.log" "THREAD completed mcp:other prev=running"
check "1: bad title is detected on recovery" has "$ROOT/out.log" "THREAD bad-title mcp:bad"
check "1: unchanged thread is not re-reported (state kept while blind)" nhas "$ROOT/out.log" "^THREAD .*mcp:impl-41"
check "1: STALL is not repeated for the same stop" nhas "$ROOT/out.log" "STALL #41"
check "1: no PATROL line for #41 on recovery" nhas "$ROOT/out.log" "PATROL #41 "
check "1: threads.json is replaced on success" has "$SNAP" "mcp:bad"

# 試験 2: state.json の無い初回から失敗していても failed 行を出し、threads.json を作らない。
reset
issues '[]'; prs '[]'; mode fail
run_cycles 1
check "2: first run already failing prints the failed line" has "$ROOT/out.log" "WATCHER thread-list-failed since="
check "2: threads.json is not created by a failed fetch" eq "$(test -e "$SNAP" && echo yes)" ""
# 終了コード 0 の空配列は成功（スレッド 0 本）として回復を出す
threads '[]'; mode ok
: > "$ROOT/out.log"
run_cycles 1
check "2: empty array with exit 0 is a success" has "$ROOT/out.log" "WATCHER thread-list-recovered"

# 試験 3: 失敗の理由。stderr があればその 1 行目（120 文字まで）、無ければ種類。
reason_case() { # reason_case <mode> <expected reason>
  reset; issues '[]'; prs '[]'; mode "$1"
  run_cycles 1
  check "3: $1 -> reason=$2" eq "$(grep -o 'thread-list-failed .*' "$ROOT/out.log" | sed 's/.* reason=//')" "$2"
}
reason_case silent "exit=3"
reason_case empty "empty"
reason_case invalid "invalid-json"
reason_case notlist "invalid-json"
reason_case long "$(printf 'x%.0s' {1..120})"

if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}"
  exit 1
fi
printf '\nall passed\n'
