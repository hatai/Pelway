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
export MOCK  # モックの t3・sleep が応答ファイルを見つけるために子へ引き継ぐ
T0=1764000000  # 巡回時刻の基準（PELWAY_WATCH_NOW に書き込む epoch 秒）
mkdir -p "$ROOT/repo/scripts" "$ROOT/bin" "$MOCK"
cp "$REPO/scripts/conductor-watch.sh" "$ROOT/repo/scripts/"
cp "$REPO/scripts/patrol-state-update" "$ROOT/repo/scripts/"
git init -q "$ROOT/repo"
PELWAY="$ROOT/repo/.git/pelway"
STATE="$PELWAY/patrol-state.json"
INBOX="$PELWAY/conductor-inbox.jsonl"
SNAP="$ROOT/state/pelway/conductor-watch/threads.json"

cat > "$ROOT/repo/scripts/agent-gh" <<'EOF'
#!/usr/bin/env bash
gf="$(cat "$MOCK/gh-fail" 2>/dev/null || true)"
[[ "$2 $3" == "issue list" ]] && { case "$gf" in issue|both) exit 9;; esac; cat "$MOCK/issues.json"; exit 0; }
[[ "$2 $3" == "pr list" ]] && { case "$gf" in pr|both) exit 9;; esac; cat "$MOCK/prs.json"; exit 0; }
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
  # 失敗でも標準出力に一覧を残す（取得結果を読む実装への回帰で、失敗中にスレッドに
  # 基づく行が出ることを確かめられる形にする）
  fail) cat "$MOCK/threads.json" 2>/dev/null; echo 'No running T3 Code server uses /home/x/.t3' >&2; exit 1 ;;
  silent) exit 3 ;;
  empty) ;;
  invalid) echo '{not json' ;;
  notlist) echo '{"threads":[]}' ;;
  badlist) echo '[{"title":"no id"},{"threadId":5}]' ;;
  # フィールドを 1 つずつ壊した入力（どの検査が欠けても試験で分かる形）
  badtitle) echo '[{"threadId":"t-1","title":5,"status":"running","updatedAt":"2026-11-20T10:00:00Z"}]' ;;
  badupdated) echo '[{"threadId":"t-1","title":"x","status":"running","updatedAt":123}]' ;;
  badstatus) echo '[{"threadId":"t-1","title":"x","status":5,"updatedAt":"2026-11-20T10:00:00Z"}]' ;;
  # JSON 外のリテラル（Python の json.loads は既定で読めてしまう）
  nanlist) echo '[{"threadId":"t-1","title":"x","status":"running","updatedAt":"2026-11-20T10:00:00Z","itemCount":NaN}]' ;;
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
# GitHub の一覧取得の失敗を切り替える: off / issue / pr / both
ghfail() { echo "$1" > "$MOCK/gh-fail"; }
issues() { printf '%s\n' "$1" > "$MOCK/issues.json"; }
prs() { printf '%s\n' "$1" > "$MOCK/prs.json"; }
threads() { printf '%s\n' "$1" > "$MOCK/threads.json"; }
# 巡回の時刻を差し替える（PELWAY_WATCH_NOW）。実時間に依存しない間隔の試験に使う
now_is() { printf '%s\n' "$1" > "$MOCK/now"; }
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
  now_is "$T0"
}

# run_cycles <n> [heartbeat secs] — n 回巡回して止める。出力は out.log に足す
run_cycles() {
  echo 0 > "$MOCK/sleeps"
  PELWAY_WATCH_NOW="$MOCK/now" SLEEP_MAX="$1" \
    PELWAY_WATCH_HEARTBEAT_SECS="${2:-600}" XDG_STATE_HOME="$ROOT/state" \
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
# #43 は実装スレッドがまだ一覧に見えない（止まりの通知がまだ記録されていない）実装中の対象。
# mcp:run は 45 分変わらない itemCount の実行中スレッド（stalled-45m の材料）。
reset
issues '[{"number":41,"state":"OPEN","labels":[{"name":"agent:working"}]},{"number":42,"state":"OPEN","labels":[{"name":"ready-for-agent"}]},{"number":43,"state":"OPEN","labels":[{"name":"agent:working"}]}]'
prs "$(pr41 true)"
threads "[$(th mcp:impl-41 completed '[実装] #41 何かを直す'),$(th mcp:other running '[実装] #77 別の作業'),$(th mcp:run running '[実装] #88 走り続ける')]"
inbox 41 $'state: working\nhead: none\nnext: 手順 4'
mode ok
run_cycles 1
check "1: stopped impl thread makes #41 conductor before the failure" eq "$(sget 41 waiting_on)" "conductor"
check "1: STALL is printed in the success phase" eq "$(count "$ROOT/out.log" 'STALL #41')" "1"
cp "$SNAP" "$ROOT/snapshot.json"

# 失敗中の応答には、成功時ならイベントが出る入力を置く（mcp:impl-43 の止まりはまだ通知
# していないので、失敗中にスレッドに基づく判定が動く回帰では STALL #43 が出るはず。
# mcp:run は itemCount が変わらないまま時計を進めると stalled-45m が出るはず）。
threads "[$(th mcp:impl-41 completed '[実装] #41 何かを直す'),$(th mcp:other completed '[実装] #77 別の作業'),$(th mcp:bad running 'no-prefix title'),$(th mcp:impl-43 completed '[実装] #43 止まる'),$(th mcp:run running '[実装] #88 走り続ける')]"
mode fail
: > "$ROOT/out.log"; : > "$ROOT/fail.log"
now_is $((T0 + 1))
run_cycles 2 5
cat "$ROOT/out.log" >> "$ROOT/fail.log"
check "1: first failure prints the failed line with the stderr reason" \
  has "$ROOT/out.log" "WATCHER thread-list-failed since=20[^ ]* reason=No running T3 Code server uses /home/x/.t3$"
check "1: failed line is printed once for a run of failures" eq "$(count "$ROOT/out.log" 'WATCHER thread-list-failed')" "1"
check "1: no re-notify before the interval" eq "$(count "$ROOT/out.log" 'thread-list-still-failing')" "0"
FAIL_SINCE="$(since_of "$ROOT/out.log" thread-list-failed)"

# 失敗中に PR が Ready になり、#42 に needs-human が付く。再通知の間隔（HEARTBEAT_SECS=5 秒）
# の期限前・期限到来・直後を、巡回時刻の指定で確かめる。
prs "$(pr41 false)"
issues '[{"number":41,"state":"OPEN","labels":[{"name":"agent:working"}]},{"number":42,"state":"OPEN","labels":[{"name":"needs-human"}]},{"number":43,"state":"OPEN","labels":[{"name":"agent:working"}]}]'
now_is $((T0 + 5))
: > "$ROOT/out.log"
run_cycles 1 5
cat "$ROOT/out.log" >> "$ROOT/fail.log"
check "1: no re-notify right before the interval" eq "$(count "$ROOT/out.log" 'thread-list-still-failing')" "0"
now_is $((T0 + 6))
: > "$ROOT/out.log"
run_cycles 1 5
cat "$ROOT/out.log" >> "$ROOT/fail.log"
check "1: still-failing line at the interval" \
  eq "$(count "$ROOT/out.log" "WATCHER thread-list-still-failing since=$FAIL_SINCE for=5s reason=No running T3 Code server uses /home/x/.t3")" "1"
now_is $((T0 + 7))
: > "$ROOT/out.log"
run_cycles 1 5
cat "$ROOT/out.log" >> "$ROOT/fail.log"
check "1: no re-notify right after one" eq "$(count "$ROOT/out.log" 'thread-list-still-failing')" "0"
# 失敗が続いたまま時計を 45 分を超える位置まで進めると、次の間隔で再通知される
# （stalled-45m の材料もこの時刻で期限を超える: 失敗中に判定が動く回帰ならここで出る）
now_is $((T0 + 3000))
: > "$ROOT/out.log"
run_cycles 1 5
cat "$ROOT/out.log" >> "$ROOT/fail.log"
check "1: still-failing line again at the next interval" \
  eq "$(count "$ROOT/out.log" "WATCHER thread-list-still-failing since=$FAIL_SINCE for=2999s reason=No running T3 Code server uses /home/x/.t3")" "1"
check "1: threads.json keeps the last successful snapshot" same "$SNAP" "$ROOT/snapshot.json"
check "1: PR change is still reported while blind" has "$ROOT/fail.log" "PR #941 agent/41-x OPEN|draft=False"
check "1: label-derived change is still applied while blind" has "$ROOT/fail.log" "PATROL #42 waiting_on=human"
check "1: #41 stays conductor while blind" eq "$(sget 41 waiting_on)" "conductor"
check "1: #43 stays implement while blind" eq "$(sget 43 waiting_on)" "implement"
check "1: no thread-dependent lines while blind" nhas "$ROOT/fail.log" "^THREAD |^STALL |stalled-45m|bad-title|PATROL #41 |PATROL #43 "

mode ok
now_is $((T0 + 3100))
: > "$ROOT/out.log"
run_cycles 1
check "1: recovery line names the first failure time" \
  has "$ROOT/out.log" "WATCHER thread-list-recovered since=$FAIL_SINCE after=3099s$"
check "1: change made while blind is reported on recovery" has "$ROOT/out.log" "THREAD completed mcp:other prev=running"
check "1: bad title is detected on recovery" has "$ROOT/out.log" "THREAD bad-title mcp:bad"
check "1: STALL input that stayed silent while blind fires on recovery" has "$ROOT/out.log" "STALL #43"
check "1: the newly stopped subject becomes conductor on recovery" has "$ROOT/out.log" "PATROL #43 waiting_on=conductor"
check "1: stalled-45m input that stayed silent while blind fires on recovery" has "$ROOT/out.log" "stalled-45m mcp:run"
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

# 試験 3: 失敗の理由。stderr があれば空白だけの行を除いた最初の行（120 文字まで）、無ければ種類。
reason_case() { # reason_case <mode> <expected reason>
  reset; issues '[]'; prs '[]'; mode "$1"
  run_cycles 1
  check "3: $1 -> reason=$2" eq "$(grep -o 'thread-list-failed .*' "$ROOT/out.log" | sed 's/.* reason=//')" "$2"
}
reason_case silent "exit=3"
reason_case empty "empty"
reason_case invalid "invalid-json"
reason_case notlist "invalid-json"
reason_case badlist "invalid-json"
reason_case badtitle "invalid-json"
reason_case badupdated "invalid-json"
reason_case badstatus "invalid-json"
reason_case nanlist "invalid-json"
reason_case long "$(printf 'x%.0s' {1..120})"

# 試験 4: 要素がスレッドの形でない一覧は失敗として扱い、前回のスナップショットを守る。
reset
issues '[]'; prs '[]'
threads "[$(th mcp:ok running '[実装] #1 あ')]"
mode ok
run_cycles 1
cp "$SNAP" "$ROOT/snapshot.json"
mode badlist
run_cycles 1
check "4: malformed list is reported as a fetch failure" has "$ROOT/out.log" "WATCHER thread-list-failed"
check "4: snapshot keeps the last good list on a malformed fetch" same "$SNAP" "$ROOT/snapshot.json"
# 文字列の threadId を持ち項目の型だけが不正な一覧と、JSON 外のリテラルを含む一覧でも、
# 失敗と通知されて前回の一覧が残ることを確かめる
i=1
for m in nanlist badtitle badupdated badstatus; do
  mode "$m"; now_is $((T0 + 700 * i)); i=$((i + 1))
  # 巡回ごとにログを空にし、その回の再通知だけを確かめる（前のケースの行では通さない）
  : > "$ROOT/out.log"
  run_cycles 1
  check "4: $m is reported as a fetch failure" eq "$(count "$ROOT/out.log" 'WATCHER thread-list-still-failing')" "1"
  check "4: snapshot keeps the last good list on $m" same "$SNAP" "$ROOT/snapshot.json"
done

# 試験 5: null のフィールドは空として扱って受理する（壊れた型だけが失敗）。
# 状態変化と 45 分停滞の通知でも title: null を空文字に正規化して巡回が止まらない。
reset
issues '[]'; prs '[]'
threads '[{"threadId":"t-null","title":null,"status":"running","updatedAt":null,"createdBy":"agent","itemCount":3}]'
mode ok
run_cycles 1
check "5: list with null fields is accepted and saved" has "$SNAP" "t-null"
now_is $((T0 + 3000))
run_cycles 1
check "5: stalled-45m line normalizes a null title" has "$ROOT/out.log" "stalled-45m t-null"
threads '[{"threadId":"t-null","title":null,"status":"completed","updatedAt":null,"createdBy":"agent","itemCount":3}]'
run_cycles 1
check "5: status-change line normalizes a null title" has "$ROOT/out.log" "THREAD completed t-null prev=running"

# 試験 6: GitHub の一覧取得に失敗する間、取得件数が上限に達する間も、T3 の通知状態は
# 巡回のたびに保存される。失敗開始・回復の通知は各 1 回だけで、再通知は間隔どおりに出る
# （巡回状態ブロックが動かない回でも state.json が保存されないと、同じ通知が繰り返される）。
gh_outage() { # gh_outage <label> — GitHub 側の状態は呼び出し側で用意しておく
  reset
  threads "[$(th mcp:ok running '[実装] #1 あ')]"
  mode ok
  run_cycles 1
  mode fail
  now_is $((T0 + 1)); : > "$ROOT/out.log"
  run_cycles 2 5
  check "6: $1 failed line is printed once across patrols" \
    eq "$(count "$ROOT/out.log" 'WATCHER thread-list-failed')" "1"
  now_is $((T0 + 6)); : > "$ROOT/out.log"
  run_cycles 1 5
  check "6: $1 still-failing at the interval" \
    eq "$(count "$ROOT/out.log" 'WATCHER thread-list-still-failing')" "1"
  now_is $((T0 + 7)); : > "$ROOT/out.log"
  run_cycles 1 5
  check "6: $1 no extra re-notify before the next interval" \
    eq "$(count "$ROOT/out.log" 'WATCHER thread-list-still-failing')" "0"
  mode ok
  now_is $((T0 + 8)); : > "$ROOT/out.log"
  run_cycles 2 5
  check "6: $1 recovered line is printed once across patrols" \
    eq "$(count "$ROOT/out.log" 'WATCHER thread-list-recovered')" "1"
}
# GitHub の一覧取得が失敗する場合
ghfail both
issues '[]'; prs '[]'
gh_outage "github-outage"
# 取得件数が上限（IL=1000 件）に達する場合
ghfail off
python3 -I -c 'import json; print(json.dumps([{"number": i + 1, "state": "OPEN", "labels": []} for i in range(1000)]))' \
  > "$MOCK/issues.json"
prs '[]'
gh_outage "github-limit"
ghfail off

if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}"
  exit 1
fi
printf '\nall passed\n'
