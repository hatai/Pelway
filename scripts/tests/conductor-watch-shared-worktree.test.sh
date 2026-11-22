#!/usr/bin/env bash
# scripts/conductor-watch.sh の作業ツリーの共有の検知（Issue #643）のテスト。
# モックの t3 が thread list と thread show に合成の JSON を返し、watch.log に出る
# THREAD shared-worktree の行と、thread show の呼び出しの記録を確かめる。
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ROOT="$(mktemp -d)"
WPID=""
trap '[[ -n "${WPID:-}" ]] && kill "$WPID" 2>/dev/null; rm -rf "$ROOT"' EXIT
failures=()

MOCK="$ROOT/mock"
mkdir -p "$ROOT/repo/scripts" "$ROOT/bin" "$MOCK/show" "$ROOT/state"
cp "$REPO/scripts/conductor-watch.sh" "$ROOT/repo/scripts/"
git init -q "$ROOT/repo"

# モックの agent-gh: 一覧は空、それ以外は失敗する（この試験はスレッドだけを見る）。
cat > "$ROOT/repo/scripts/agent-gh" <<'EOF'
#!/usr/bin/env bash
[[ "$2 $3" == "issue list" || "$2 $3" == "pr list" ]] && { echo '[]'; exit 0; }
[[ "$2 $3" == "api /repos/hatai/Pelway/issues/events?per_page=100" ]] && { echo '[]'; exit 0; }
exit 9
EOF
chmod +x "$ROOT/repo/scripts/agent-gh"
# モックの t3: thread show <id> --json は $MOCK/show/<id> の作業ツリーを返す。
# ファイルが無いか中身が FAIL のスレッドは失敗する。呼び出しは $MOCK/show-calls に記録する。
cat > "$ROOT/bin/t3" <<'EOF'
#!/usr/bin/env bash
[[ "$1 $2" == "thread list" ]] && { cat "$MOCK/threads.json"; exit 0; }
if [[ "$1 $2" == "thread show" && "${4:-}" == "--json" ]]; then
  echo "$3" >> "$MOCK/show-calls"
  f="$MOCK/show/$3"
  [[ -f "$f" && "$(cat "$f")" != "FAIL" ]] || exit 1
  python3 -I -c 'import json,sys; v=open(sys.argv[1]).read().rstrip("\n")
print(json.dumps({"thread": {"threadId": sys.argv[2], "worktreePath": None if v == "null" else v}}))' "$f" "$3"
  exit 0
fi
echo "t3 $*" >> "$MOCK/other-calls"; exit 9
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

# th <id> <settled> <createdBy> — thread list の 1 要素
th() { printf '{"threadId":"%s","title":"[実装] #1 x","status":"completed","settled":%s,"parentThreadId":null,"createdBy":"%s"}' "$1" "$2" "$3"; }
threads() { local IFS=,; printf '[%s]\n' "$*" > "$MOCK/threads.json"; }
# wt <id> <path|null|FAIL|""> — thread show が返す作業ツリー
wt() { printf '%s\n' "$2" > "$MOCK/show/$1"; }

reset() {
  rm -rf "$ROOT/state/pelway" "$MOCK/show" "$MOCK/show-calls" "$MOCK/other-calls"
  mkdir -p "$MOCK/show"; : > "$ROOT/out.log"
}
# run_cycles <n> — 見張りを n 巡回だけ回す（出力は out.log に足す）
run_cycles() {
  echo 0 > "$MOCK/sleeps"
  MOCK="$MOCK" SLEEP_MAX="$1" XDG_STATE_HOME="$ROOT/state" PATH="$ROOT/bin:$PATH" \
    T3_BIN="$ROOT/bin/t3" PELWAY_BOARD_QUEUE_DIR="$ROOT/queue" \
    bash "$ROOT/repo/scripts/conductor-watch.sh" >>"$ROOT/out.log" 2>&1 &
  WPID=$!
  wait "$WPID" 2>/dev/null; WPID=""
}

check() {
  local name="$1"; shift
  if "$@"; then printf 'ok   %s\n' "$name"; else printf 'FAIL %s\n' "$name"; failures+=("$name"); fi
}
eq() { [[ "$1" == "$2" ]]; }
count() { grep -c -F -- "$2" "$1"; }
shared() { count "$ROOT/out.log" "THREAD shared-worktree"; }

# 試験 1: 2 本が同じ作業ツリー → 1 行（threadId は昇順）。人間が立てたスレッドも数える。
# settle 済み・作業ツリーが空・null のスレッドは数えない。
reset
threads "$(th t-b false agent)" "$(th t-a false human)" "$(th t-c true agent)" \
  "$(th t-d false agent)" "$(th t-e false agent)"
wt t-a /w/one; wt t-b /w/one; wt t-c /w/one; wt t-d ""; wt t-e null
run_cycles 1
check "1: shared worktree is reported once with sorted ids" \
  eq "$(count "$ROOT/out.log" "THREAD shared-worktree path=/w/one threads=t-a,t-b")" "1"
check "1: only one shared-worktree line" eq "$(shared)" "1"

# 試験 2: 同じ組み合わせが続く間は、2 巡目以降に同じ行を出さない。
run_cycles 2
check "2: same combination is not reported again" eq "$(shared)" "1"

# 試験 3: 共有が解消された後に再び共有されたら、また 1 行出す。
wt t-b /w/two
run_cycles 1
check "3: no line while not shared" eq "$(shared)" "1"
wt t-b /w/one
run_cycles 1
check "3: re-sharing is reported again" \
  eq "$(count "$ROOT/out.log" "THREAD shared-worktree path=/w/one threads=t-a,t-b")" "2"

# 試験 4: 組み合わせのスレッドが 2 本から 3 本に増えたら、新しい組み合わせとして 1 行出す。
threads "$(th t-b false agent)" "$(th t-a false human)" "$(th t-c false agent)"
run_cycles 1
check "4: grown combination is reported" \
  eq "$(count "$ROOT/out.log" "THREAD shared-worktree path=/w/one threads=t-a,t-b,t-c")" "1"
check "4: total lines" eq "$(shared)" "3"

# 試験 5: thread show が失敗したスレッドはその巡回で数えず、ほかの判定と周回は続く。
# thread show は settle されていないスレッドだけに、1 巡回につき 1 本 1 回呼ぶ。
reset
threads "$(th t-a false agent)" "$(th t-x false agent)" "$(th t-b false agent)" \
  "$(th t-s true agent)" "$(th t-c false agent)"
wt t-a /w/p; wt t-x FAIL; wt t-b /w/p; wt t-s /w/p; wt t-c /w/q
run_cycles 2
check "5: other threads are still judged when one show fails" \
  eq "$(count "$ROOT/out.log" "THREAD shared-worktree path=/w/p threads=t-a,t-b")" "1"
check "5: failing thread is not counted" eq "$(shared)" "1"
check "5: watcher keeps cycling" eq "$(count "$ROOT/out.log" "HEARTBEAT")" "1"
check "5: second cycle ran" eq "$(cat "$MOCK/sleeps")" "2"
check "5: show is not called for settled threads" eq "$(count "$MOCK/show-calls" "t-s")" "0"
check "5: show is called once per unsettled thread per cycle" \
  eq "$(sort "$MOCK/show-calls" | uniq -c | awk '{print $2"="$1}' | tr '\n' ' ')" "t-a=2 t-b=2 t-c=2 t-x=2 "
check "5: no other t3 calls" eq "$(cat "$MOCK/other-calls" 2>/dev/null)" ""

if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}"
  exit 1
fi
printf '\nall passed\n'
