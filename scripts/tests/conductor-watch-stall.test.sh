#!/usr/bin/env bash
# scripts/conductor-watch.sh の止まりの検知（Issue #504）のテスト。
# モックの t3・agent-gh で、実装スレッドの状態と GitHub の状態を返し、指揮役の受信箱
# （conductor-inbox.jsonl。Issue #555）に実装担当の状態の行（skill implement-issue の
# 「ターンの終わりの状態の行」）を置いて、補助記録の waiting_on と watch.log の行を確かめる。
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ROOT="$(mktemp -d)"
WPID=""
trap '[[ -n "${WPID:-}" ]] && kill "$WPID" 2>/dev/null; rm -rf "$ROOT"' EXIT
failures=()

MOCK="$ROOT/mock"
mkdir -p "$ROOT/repo/scripts" "$ROOT/bin" "$MOCK" "$ROOT/state/pelway"
cp "$REPO/scripts/conductor-watch.sh" "$ROOT/repo/scripts/"
git init -q "$ROOT/repo"
PELWAY="$ROOT/repo/.git/pelway"
STATE="$PELWAY/patrol-state.json"
INBOX="$PELWAY/conductor-inbox.jsonl"

# モックの agent-gh: issue list / pr list だけに答える。それ以外の呼び出しは記録して失敗する
# （書き込み系の呼び出しが無いことを確かめるため）。
cat > "$ROOT/repo/scripts/agent-gh" <<'EOF'
#!/usr/bin/env bash
[[ "$2 $3" == "issue list" ]] && { cat "$MOCK/issues.json"; exit 0; }
[[ "$2 $3" == "pr list" ]] && { cat "$MOCK/prs.json"; exit 0; }
[[ "$2 $3" == "api /repos/hatai/Pelway/issues/events?per_page=100" ]] && { echo '[]'; exit 0; }
echo "agent-gh $*" >> "$MOCK/other-calls"; exit 9
EOF
chmod +x "$ROOT/repo/scripts/agent-gh"
cat > "$ROOT/bin/t3" <<'EOF'
#!/usr/bin/env bash
[[ "$1 $2" == "thread list" ]] && { cat "$MOCK/threads.json"; exit 0; }
# 作業ツリーの共有の検知（Issue #643）が呼ぶ読み取り。共有の無い空の作業ツリーを返す。
[[ "$1 $2" == "thread show" ]] && { echo '{"thread":{"worktreePath":""}}'; exit 0; }
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

# 一覧の材料を作る
issues() { printf '%s\n' "$1" > "$MOCK/issues.json"; }
prs() { printf '%s\n' "$1" > "$MOCK/prs.json"; }
threads() { printf '%s\n' "$1" > "$MOCK/threads.json"; }
# inbox <issue> <report|null> — 受信箱に finished の知らせを 1 行足す
inbox() {
  python3 -I - "$INBOX" "$1" "$2" <<'EOF'
import json, sys, time
p, n, rep = sys.argv[1:4]
line = {"schema": "pelway.agent-watch/v1", "event": "finished", "thread_id": "mcp:impl-" + n,
        "issue_number": int(n), "round": 1, "report": None if rep == "null" else rep,
        "occurred_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())}
open(p, "a").write(json.dumps(line, ensure_ascii=False) + "\n")
EOF
}
# impl <issue> <status> — その Issue の実装スレッド 1 本
impl() { printf '[{"threadId":"mcp:impl-%s","title":"[実装] #%s 何かを直す","status":"%s","updatedAt":"2026-11-20T10:00:00Z","createdBy":"agent"}]' "$1" "$1" "$2"; }
iss() { printf '[{"number":%s,"state":"OPEN","labels":[%s]}]' "$1" "$2"; }
draft_pr() { printf '[{"number":9%s,"state":"OPEN","isDraft":true,"headRefName":"agent/%s-x","reviewDecision":null,"statusCheckRollup":[]}]' "$1" "$1"; }
approved_pr() { printf '[{"number":9%s,"state":"OPEN","isDraft":false,"headRefName":"agent/%s-x","reviewDecision":"APPROVED","statusCheckRollup":[{"name":"verify","conclusion":"SUCCESS"}]}]' "$1" "$1"; }

reset() {
  rm -rf "$PELWAY" "$ROOT/state/pelway/conductor-watch" "$MOCK/other-calls"
  mkdir -p "$PELWAY"; : > "$ROOT/out.log"
}

run_cycles() {
  echo 0 > "$MOCK/sleeps"
  MOCK="$MOCK" SLEEP_MAX="$1" XDG_STATE_HOME="$ROOT/state" PATH="$ROOT/bin:$PATH" \
    T3_BIN="$ROOT/bin/t3" PELWAY_BOARD_QUEUE_DIR="$ROOT/queue" \
    bash "$ROOT/repo/scripts/conductor-watch.sh" >>"$ROOT/out.log" 2>&1 &
  WPID=$!
  wait "$WPID" 2>/dev/null; WPID=""
}

sget() {
  python3 -I -c 'import json,sys
print(json.load(open(sys.argv[1]))["subjects"].get(sys.argv[2], {}).get(sys.argv[3], ""))' "$STATE" "$1" "$2"
}
sset() { # sset <issue> <field> <json value>
  python3 -I -c 'import json,sys
p, n, k, v = sys.argv[1:5]
d = json.load(open(p)); d["subjects"][n][k] = json.loads(v)
json.dump(d, open(p, "w"), ensure_ascii=False, indent=2, sort_keys=True)' "$STATE" "$1" "$2" "$3"
}
nca_minutes() { # next_check_at の現在からのずれ（分）
  python3 -I -c 'import json,sys,datetime,time
s = json.load(open(sys.argv[1]))["subjects"][sys.argv[2]]["next_check_at"]
print(round((datetime.datetime.fromisoformat(s).timestamp() - time.time()) / 60))' "$STATE" "$1"
}

check() {
  local name="$1"; shift
  if "$@"; then printf 'ok   %s\n' "$name"; else printf 'FAIL %s\n' "$name"; failures+=("$name"); fi
}
eq() { [[ "$1" == "$2" ]]; }
has() { grep -q -- "$2" "$1"; }
count() { grep -c -- "$2" "$1"; }
le() { (( $1 <= $2 )); }
no_writes() { [[ ! -s "$MOCK/other-calls" ]]; }

# 試験 1: 実装スレッドが completed で、最後の状態の行が working → 次の計算で conductor。
# 元の値と時刻が watch.log の 1 行に残る。
reset
issues "$(iss 41 '{"name":"agent:working"}')"; prs "$(draft_pr 41)"; threads "$(impl 41 completed)"
inbox 41 $'state: working\nhead: none\nnext: 手順 4'
run_cycles 1
check "1: stopped thread with state=working becomes conductor" eq "$(sget 41 waiting_on)" "conductor"
check "1: original value and time are logged" has "$ROOT/out.log" "PATROL #41 waiting_on=conductor from=implement reason=stopped-thread state=working"
check "1: no writes to GitHub or threads" no_writes

# 試験 2: 状態の行が無い（report が null）ときも conductor。
reset
issues "$(iss 42 '{"name":"agent:working"}')"; prs "$(draft_pr 42)"; threads "$(impl 42 failed)"
inbox 42 null
run_cycles 1
check "2: stopped thread without a state line becomes conductor" eq "$(sget 42 waiting_on)" "conductor"
check "2: logged with state=none" has "$ROOT/out.log" "PATROL #42 waiting_on=conductor from=implement reason=stopped-thread state=none"

# 試験 3: waiting-review は waiting_on を変えず、next_check_at を約 8 分後まで早める。
reset
issues "$(iss 43 '{"name":"agent:working"}')"; prs "$(draft_pr 43)"; threads "$(impl 43 completed)"
inbox 43 $'state: waiting-review\nhead: abc\nnext: 手順 9'
run_cycles 1
check "3: waiting-review keeps waiting_on" eq "$(sget 43 waiting_on)" "implement"
check "3: next_check_at is brought forward to about 8 minutes" le "$(nca_minutes 43)" 8

# 試験 4: done は PR の状態からの計算のまま（Approve 済みなら merge）。
reset
issues "$(iss 44 '{"name":"agent:working"}')"; prs "$(approved_pr 44)"; threads "$(impl 44 completed)"
inbox 44 $'issue: #44\nready: done\nstate: done'
run_cycles 1
check "4: done keeps the PR-based value" eq "$(sget 44 waiting_on)" "merge"

# 試験 5: needs-human は、状態の行がどうであっても human のまま。
reset
issues "$(iss 45 '{"name":"agent:working"},{"name":"needs-human"}')"; prs "$(draft_pr 45)"; threads "$(impl 45 completed)"
inbox 45 $'state: working\nhead: none\nnext: 手順 4'
run_cycles 1
check "5: needs-human stays human" eq "$(sget 45 waiting_on)" "human"
check "5: no stopped-thread rewrite for needs-human" eq "$(count "$ROOT/out.log" 'PATROL #45 waiting_on=conductor')" "0"

# 試験 6: 指揮役が書いた grokbot は、ラベルが変わらない限り上書きしない。
reset
issues "$(iss 46 '{"name":"agent:working"}')"; prs "$(draft_pr 46)"; threads "[]"
run_cycles 1
sset 46 waiting_on '"grokbot"'
run_cycles 1
check "6: grokbot set by the conductor is kept" eq "$(sget 46 waiting_on)" "grokbot"
issues "$(iss 46 '{"name":"agent:working"},{"name":"needs-human"}')"
run_cycles 1
check "6: a label change ends the grokbot wait" eq "$(sget 46 waiting_on)" "human"

# 試験 7: STALL の行は、同じ対象・同じ止まりの時刻につき 1 回だけ。
reset
issues "$(iss 47 '{"name":"agent:working"}')"; prs "$(draft_pr 47)"; threads "$(impl 47 interrupted)"
inbox 47 null
run_cycles 2
check "7: STALL is printed once for the same stop" eq "$(count "$ROOT/out.log" 'STALL #47')" "1"

# 試験 8: 見張りが動いていることを示す HEARTBEAT の行を出す。
check "8: HEARTBEAT line is printed" has "$ROOT/out.log" "HEARTBEAT"
check "1-8: no writes to GitHub or threads" no_writes

# 試験 9: waiting-conductor（レビューを委託しない条件に当たる PR で指揮役のレビュースレッドを待つ。
# Issue #622）は conductor にし、STALL の行は出さない。
reset
issues "$(iss 49 '{"name":"agent:working"}')"; prs "$(draft_pr 49)"; threads "$(impl 49 completed)"
inbox 49 $'state: waiting-conductor\nhead: abc\nnext: 指揮役のレビュースレッドの結果で手順 9 に進む'
run_cycles 2
check "9: waiting-conductor becomes conductor" eq "$(sget 49 waiting_on)" "conductor"
check "9: no STALL for waiting-conductor" eq "$(count "$ROOT/out.log" 'STALL #49')" "0"
check "9: no writes to GitHub or threads" no_writes

if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}"
  exit 1
fi
printf '\nall passed\n'
