#!/usr/bin/env bash
# scripts/conductor-watch.sh の未配送指示（pending_thread_sends）の滞留検知（Issue #669）のテスト。
# 巡回時刻を PELWAY_WATCH_NOW で差し替え、patrol-state.json の pending_thread_sends の
# 最古の since からの経過がしきい値を超えたときの WATCHER 行・HEARTBEAT_SECS ごとの
# 再通知・解消の回復行・読めない/キーが無いときの誤検知なしを確かめる。
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ROOT="$(mktemp -d)"
WPID=""
trap '[[ -n "${WPID:-}" ]] && kill "$WPID" 2>/dev/null; rm -rf "$ROOT"' EXIT
failures=()

MOCK="$ROOT/mock"
export MOCK  # モックの t3・sleep が応答ファイルを見つけるために子へ引き継ぐ
T0=1764000000  # 巡回時刻の基準（PELWAY_WATCH_NOW に書き込む epoch 秒）
mkdir -p "$ROOT/repo/scripts" "$ROOT/bin" "$MOCK" "$ROOT/state/pelway" "$ROOT/queue"
cp "$REPO/scripts/conductor-watch.sh" "$ROOT/repo/scripts/"
git init -q "$ROOT/repo"
PELWAY="$ROOT/repo/.git/pelway"
STATE="$PELWAY/patrol-state.json"

cat > "$ROOT/repo/scripts/agent-gh" <<'EOF'
#!/usr/bin/env bash
[[ "$2 $3" == "issue list" ]] && { cat "$MOCK/issues.json"; exit 0; }
[[ "$2 $3" == "pr list" ]] && { cat "$MOCK/prs.json"; exit 0; }
[[ "$2 $3" == "api /repos/hatai/Pelway/issues/events?per_page=100" ]] && { echo '[]'; exit 0; }
exit 9
EOF
chmod +x "$ROOT/repo/scripts/agent-gh"
# モックの t3: 空の一覧に成功するだけ（この試験ではスレッド側の材料は使わない）
cat > "$ROOT/bin/t3" <<'EOF'
#!/usr/bin/env bash
[[ "$1 $2" == "thread list" ]] && { echo '[]'; exit 0; }
exit 9
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

issues() { printf '%s\n' "$1" > "$MOCK/issues.json"; }
prs() { printf '%s\n' "$1" > "$MOCK/prs.json"; }
# 巡回の時刻を差し替える（PELWAY_WATCH_NOW）。実時間に依存しない間隔の試験に使う
now_is() { printf '%s\n' "$1" > "$MOCK/now"; }
# pend <since-epoch>... — patrol-state.json の pending_thread_sends を、since が各引数の
# 要素（to は mcp:t0 からの連番）で書き直す。引数が無ければ空の配列を置く
pend() {
  python3 -I - "$STATE" "$*" <<'EOF'
import json, sys
from datetime import datetime, timezone
p, args = sys.argv[1], sys.argv[2].split()
elems = [{"to": f"mcp:t{i}", "what": "wake", "since": datetime.fromtimestamp(int(a), tz=timezone.utc).isoformat()}
         for i, a in enumerate(args)]
try: d = json.load(open(p))
except Exception: d = {}
if not isinstance(d, dict): d = {}
d["pending_thread_sends"] = elems
json.dump(d, open(p, "w"), ensure_ascii=False, indent=2, sort_keys=True)
EOF
}
# pend_to <since-epoch> <to> — pending_thread_sends を宛先を指定した 1 要素で書き直す
pend_to() {
  python3 -I - "$STATE" "$1" "$2" <<'EOF'
import json, sys
from datetime import datetime, timezone
p, a, to = sys.argv[1], int(sys.argv[2]), sys.argv[3]
try: d = json.load(open(p))
except Exception: d = {}
if not isinstance(d, dict): d = {}
d["pending_thread_sends"] = [{"to": to, "what": "wake",
    "since": datetime.fromtimestamp(a, tz=timezone.utc).isoformat()}]
json.dump(d, open(p, "w"), ensure_ascii=False, indent=2, sort_keys=True)
EOF
}
# pend_iso <to> <since-string> — pending_thread_sends を任意の since 文字列の 1 要素で書き直す
pend_iso() {
  python3 -I - "$STATE" "$1" "$2" <<'EOF'
import json, sys
p, to, since = sys.argv[1], sys.argv[2], sys.argv[3]
try: d = json.load(open(p))
except Exception: d = {}
if not isinstance(d, dict): d = {}
d["pending_thread_sends"] = [{"to": to, "what": "wake", "since": since}]
json.dump(d, open(p, "w"), ensure_ascii=False, indent=2, sort_keys=True)
EOF
}
# state_has <key> — patrol-state.json のトップレベルにキーがあるか（読めなければ unreadable）
state_has() {
  python3 -I -c 'import json,sys
print("yes" if sys.argv[2] in json.load(open(sys.argv[1])) else "no")' "$STATE" "$1" 2>/dev/null || echo unreadable
}
# pending_thread_sends キーが無い patrol-state.json にする
pend_none() {
  python3 -I -c 'import json,sys
p = sys.argv[1]
try: d = json.load(open(p))
except Exception: d = {}
if not isinstance(d, dict): d = {}
d.pop("pending_thread_sends", None)
json.dump(d, open(p, "w"), ensure_ascii=False, indent=2, sort_keys=True)' "$STATE"
}
pend_count() { # patrol-state.json の pending_thread_sends の要素数（キーが無ければ "none"）
  python3 -I -c 'import json,sys
d = json.load(open(sys.argv[1]))
arr = d.get("pending_thread_sends")
print("none" if not isinstance(arr, list) else str(len(arr)))' "$STATE" 2>/dev/null || echo unreadable
}

reset() {
  rm -rf "$PELWAY" "$ROOT/state/pelway/conductor-watch"
  mkdir -p "$PELWAY"; : > "$ROOT/out.log"
  now_is "$T0"
}

# run_cycles <n> [heartbeat secs] [pending しきい値秒] — n 回巡回して止める。出力は out.log に足す
run_cycles() {
  echo 0 > "$MOCK/sleeps"
  PELWAY_WATCH_NOW="$MOCK/now" SLEEP_MAX="$1" \
    PELWAY_WATCH_HEARTBEAT_SECS="${2:-600}" PELWAY_WATCH_PENDING_SENDS_SECS="${3:-1800}" \
    XDG_STATE_HOME="$ROOT/state" \
    PATH="$ROOT/bin:$PATH" T3_BIN="$ROOT/bin/t3" PELWAY_BOARD_QUEUE_DIR="$ROOT/queue" \
    bash "$ROOT/repo/scripts/conductor-watch.sh" >>"$ROOT/out.log" 2>&1 &
  WPID=$!
  wait "$WPID" 2>/dev/null; WPID=""
}

check() {
  local name="$1"; shift
  if "$@"; then printf 'ok   %s\n' "$name"; else printf 'FAIL %s\n' "$name"; failures+=("$name"); fi
}
eq() { [[ "$1" == "$2" ]]; }
has() { grep -q -- "$2" "$1"; }
nhas() { ! grep -qE -- "$2" "$1"; }
count() { grep -c -- "$2" "$1" || true; }
since_of() { grep -o -m1 -- "$2 since=[^ ]*" "$1" | sed 's/.* since=//'; }

issues '[]'; prs '[]'

# 試験 1: 最古の since からの経過がしきい値（既定 1800 秒）を超えると stalled 行が出る。
# 行は最古の since・経過秒数・件数・最古の宛先を名乗る。しきい値内の 2 件目は数に数えられるだけ。
reset
pend $((T0 - 1900)) $((T0 - 100))
run_cycles 1
check "1: stalled line for the oldest pending send over the threshold" \
  has "$ROOT/out.log" "WATCHER pending-sends-stalled since=20[^ ]* for=1900s count=2 oldest_to=mcp:t0"
check "1: stalled line is printed once" eq "$(count "$ROOT/out.log" 'WATCHER pending-sends-stalled ')" "1"

# 試験 2: 最古の since がしきい値内なら行は出ない。巡回の書き戻しで要素は消えない。
reset
pend $((T0 - 1799))
run_cycles 1
check "2: no watcher line within the threshold" nhas "$ROOT/out.log" "WATCHER pending-sends"
check "2: pending_thread_sends survives the patrol's write-back" eq "$(pend_count)" "1"

# 試験 2b: 経過がしきい値と等しい巡回では行は出ない（通知は超えたときだけ）。1 秒超えれば出る。
reset
pend $((T0 - 1800))
run_cycles 1
check "2b: no stalled line at exactly the threshold" nhas "$ROOT/out.log" "WATCHER pending-sends"
pend $((T0 - 1801))
run_cycles 1
check "2b: stalled line just past the threshold" \
  has "$ROOT/out.log" "WATCHER pending-sends-stalled since=20[^ ]* for=1801s count=1 oldest_to=mcp:t0"

# 試験 3: 滞留の間は HEARTBEAT_SECS ごとに再通知する（間隔の直前・到来・直後を確かめる）。
reset
pend $((T0 - 2000))
run_cycles 1 5
check "3: first stall is not a re-notify" eq "$(count "$ROOT/out.log" 'WATCHER pending-sends-still-stalled')" "0"
STALLED_SINCE="$(since_of "$ROOT/out.log" pending-sends-stalled)"
now_is $((T0 + 4)); : > "$ROOT/out.log"
run_cycles 1 5
check "3: no re-notify before the interval" eq "$(count "$ROOT/out.log" 'WATCHER pending-sends-still-stalled')" "0"
now_is $((T0 + 5)); : > "$ROOT/out.log"
run_cycles 1 5
check "3: still-stalled line at the interval" \
  eq "$(count "$ROOT/out.log" "WATCHER pending-sends-still-stalled since=$STALLED_SINCE for=2005s count=1 oldest_to=mcp:t0")" "1"
now_is $((T0 + 6)); : > "$ROOT/out.log"
run_cycles 1 5
check "3: no re-notify right after one" eq "$(count "$ROOT/out.log" 'WATCHER pending-sends-still-stalled')" "0"

# 試験 4: 配列が空になると回復の行を出す（after は最初の滞留の始まりからの秒数）。
pend
now_is $((T0 + 100)); : > "$ROOT/out.log"
run_cycles 1 5
check "4: recovery line when the array becomes empty" \
  has "$ROOT/out.log" "WATCHER pending-sends-recovered since=$STALLED_SINCE after=2100s"
check "4: no stalled line after the recovery" nhas "$ROOT/out.log" "pending-sends-stalled"

# 試験 5: 最古の since が新しい値に変わると、前の滞留の回復と（しきい値を超えていれば）
# 新しい滞留の 1 行を出す。しきい値内に収まるなら回復だけ。
reset
pend $((T0 - 2000)) $((T0 - 1900))
run_cycles 1
OLD_SINCE="$(since_of "$ROOT/out.log" pending-sends-stalled)"
pend $((T0 - 1900))
now_is $((T0 + 10)); : > "$ROOT/out.log"
run_cycles 1
check "5: recovery line for the replaced oldest" \
  has "$ROOT/out.log" "WATCHER pending-sends-recovered since=$OLD_SINCE after=2010s"
check "5: new stall line for the new oldest over the threshold" \
  has "$ROOT/out.log" "WATCHER pending-sends-stalled since=20[^ ]* for=1910s count=1 oldest_to=mcp:t0"
reset
pend $((T0 - 2000))
run_cycles 1
OLD_SINCE="$(since_of "$ROOT/out.log" pending-sends-stalled)"
pend $((T0 - 100))
now_is $((T0 + 10)); : > "$ROOT/out.log"
run_cycles 1
check "5: recovery only when the new oldest is within the threshold" \
  has "$ROOT/out.log" "WATCHER pending-sends-recovered since=$OLD_SINCE after=2010s"
check "5: no new stall within the threshold" nhas "$ROOT/out.log" "WATCHER pending-sends-stalled"

# 試験 6: patrol-state.json が読めないときは滞留なしとして扱い、誤検知しない。
# 滞留の最中に読めなくなった前回の滞留は回復として扱う。
reset
echo '{not json' > "$STATE"
run_cycles 1
check "6: unreadable state from the start never stalls" nhas "$ROOT/out.log" "WATCHER pending-sends"
reset
pend $((T0 - 2000))
run_cycles 1
OLD_SINCE="$(since_of "$ROOT/out.log" pending-sends-stalled)"
echo '{not json' > "$STATE"
now_is $((T0 + 10)); : > "$ROOT/out.log"
run_cycles 1
check "6: unreadable state does not re-notify" nhas "$ROOT/out.log" "pending-sends-still-stalled"
check "6: unreadable state counts as no stall (recovered)" \
  has "$ROOT/out.log" "WATCHER pending-sends-recovered since=$OLD_SINCE after=2010s"

# 試験 7: pending_thread_sends キーが無いときも滞留なしとして扱う。
reset
pend_none
run_cycles 1
check "7: missing key never stalls" nhas "$ROOT/out.log" "WATCHER pending-sends"
reset
pend $((T0 - 2000))
run_cycles 1
OLD_SINCE="$(since_of "$ROOT/out.log" pending-sends-stalled)"
pend_none
now_is $((T0 + 10)); : > "$ROOT/out.log"
run_cycles 1
check "7: missing key counts as no stall (recovered)" \
  has "$ROOT/out.log" "WATCHER pending-sends-recovered since=$OLD_SINCE after=2010s"

# 試験 8: しきい値は PELWAY_WATCH_PENDING_SENDS_SECS で上書きできる。
reset
pend $((T0 - 100))
run_cycles 1 600 60
check "8: threshold overridable by the environment" \
  has "$ROOT/out.log" "WATCHER pending-sends-stalled since=20[^ ]* for=100s count=1 oldest_to=mcp:t0"

# 試験 9: 間隔の設定値が数値でないときは設定名を警告して既定値で巡回を続ける
# （python が途中で落ちて HEARTBEAT だけが出続ける状態にならない）。
reset
pend $((T0 - 1900))
run_cycles 1 bad abc
check "9: bad heartbeat interval warns with the setting name" \
  has "$ROOT/out.log" "WATCHER bad-config PELWAY_WATCH_HEARTBEAT_SECS"
check "9: bad pending-sends interval warns with the setting name" \
  has "$ROOT/out.log" "WATCHER bad-config PELWAY_WATCH_PENDING_SENDS_SECS"
check "9: heartbeat still emitted on the default" has "$ROOT/out.log" "HEARTBEAT"
check "9: stall detection still works on the default threshold" \
  has "$ROOT/out.log" "WATCHER pending-sends-stalled since=20[^ ]* for=1900s count=1 oldest_to=mcp:t0"

# 試験 10: 宛先がスレッド ID の形でないときは固定値に置き換える。形は conductor-rebuild の
# valid_id が受け付ける文字種と長さに限って合わせた（英数字と :_.%- で 256 文字以内。
# valid_id が別に持つ先頭ハイフンの拒否までは真似ない）。改行入りの宛先で watch.log に偽の行を
# 書かせない。% を含む正規の宛先（委託スレッドの ID）はそのまま出る。
reset
pend_to $((T0 - 1900)) $'x\nTHREAD shared-worktree path=/w threads=a,b'
run_cycles 1
check "10: invalid to is replaced with a fixed value" \
  has "$ROOT/out.log" "WATCHER pending-sends-stalled since=20[^ ]* for=1900s count=1 oldest_to=invalid"
check "10: stalled line is still one line" eq "$(count "$ROOT/out.log" 'WATCHER pending-sends-stalled ')" "1"
check "10: no forged line from the to value" nhas "$ROOT/out.log" "THREAD shared-worktree"
reset
pend_to $((T0 - 1900)) 'thread:delegated-task:command%3Amcp%3Aabc123'
run_cycles 1
check "10: percent-encoded to is shown as-is" \
  has "$ROOT/out.log" "WATCHER pending-sends-stalled since=20[^ ]* for=1900s count=1 oldest_to=thread:delegated-task:command%3Amcp%3Aabc123"

# 試験 11: fromisoformat では読めるが地方時では年の範囲を外れる since は、読めないものと
# 同じく数に入れない。置かれても巡回が止まらず、後段（記録の書き戻し）が動く。
reset
pend_iso mcp:t0 "0001-01-01T00:00:00+23:59"
run_cycles 1
check "11: no traceback from an out-of-range since" nhas "$ROOT/out.log" "Traceback"
check "11: out-of-range since is not counted" nhas "$ROOT/out.log" "WATCHER pending-sends"
check "11: patrol still writes the state back" eq "$(state_has updated_at)" "yes"

# 試験 12: patrol-state.json が通常ファイルでない（symlink）・読み取り上限を超えるときは、
# 読めないものと同じく滞留を数えず、後段（記録の書き戻し）まで止まらない。
# （待ちを起こし得るファイル種の扱いは別の検討とし、ここでは symlink とサイズで確かめる）
reset
pend_to $((T0 - 1900)) mcp:t0
mv "$STATE" "$ROOT/real-state.json" && ln -s "$ROOT/real-state.json" "$STATE"
run_cycles 1
check "12: symlinked patrol-state is not read for pending-sends" \
  nhas "$ROOT/out.log" "WATCHER pending-sends-stalled"
check "12: patrol still writes the state back over a symlink" eq "$(state_has updated_at)" "yes"
reset
pend_to $((T0 - 1900)) mcp:t0
python3 -I - "$STATE" <<'EOF'
import json, sys
p = sys.argv[1]
d = json.load(open(p))
d["pad"] = "x" * 5_000_000  # 読み取り上限（4 MiB）を超える有効な JSON にする
json.dump(d, open(p, "w"), ensure_ascii=False)
EOF
run_cycles 1
check "12: oversized patrol-state is not counted" \
  nhas "$ROOT/out.log" "WATCHER pending-sends-stalled"
check "12: patrol still writes the state back over an oversized file" \
  eq "$(state_has updated_at)" "yes"

if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}"
  exit 1
fi
printf '\nall passed\n'
