#!/usr/bin/env bash
# scripts/conductor-watch.sh の SLOTS 行（Issue #714）のテスト。
# 巡回ごとに、実装と調査に使っている枠の数を agent-team.toml の上限と比べ、
# `SLOTS <担当> <使用数>/<上限>`（上限超過は `SLOTS over`、検証の失敗は `SLOTS error`）を出す。
# 出すのは使用数が前回の巡回から変わったときと、HEARTBEAT の行を出した巡回。
# agent-team.toml が作業ツリーに無いときは出さない。見張りは GitHub に書き込まない。
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ROOT="$(mktemp -d)"
WPID=""
trap '[[ -n "${WPID:-}" ]] && kill "$WPID" 2>/dev/null; rm -rf "$ROOT"' EXIT
failures=()

MOCK="$ROOT/mock"
mkdir -p "$ROOT/repo/scripts" "$ROOT/bin" "$MOCK" "$ROOT/state/pelway" "$ROOT/repo/.git/pelway"

# 本物の conductor-watch.sh と agent-team-config を置く偽リポジトリ
# （agent-gh はモックに差し替える。patrol-state.json の既定の置き場の解決を実際に通す）。
cp "$REPO/scripts/conductor-watch.sh" "$ROOT/repo/scripts/"
cp "$REPO/scripts/agent-team-config" "$ROOT/repo/scripts/"
git init -q "$ROOT/repo"
STATE="$ROOT/repo/.git/pelway/patrol-state.json"

# モックの agent-gh: 呼び出しをすべて記録し、issue list / pr list / label-events の読み取りだけに答える。
# 読み取り以外の呼び出しは exit 9 で失敗する（見張りが GitHub に書き込まないことの確認に使う）。
cat > "$ROOT/repo/scripts/agent-gh" <<'EOF'
#!/usr/bin/env bash
echo "$*" >> "$MOCK/gh-calls.log"
[[ "$2 $3" == "issue list" ]] && { cat "$MOCK/issues.json"; exit 0; }
[[ "$2 $3" == "pr list" ]] && { cat "$MOCK/prs.json"; exit 0; }
[[ "$2 $3" == "api /repos/hatai/Pelway/issues/events?per_page=100" ]] && { echo '[]'; exit 0; }
exit 9
EOF
chmod +x "$ROOT/repo/scripts/agent-gh"

cat > "$ROOT/bin/t3" <<'EOF'
#!/usr/bin/env bash
[[ "$1 $2" == "thread list" ]] && cat "$MOCK/threads.json" && exit 0
exit 9
EOF
chmod +x "$ROOT/bin/t3"

cat > "$ROOT/bin/sleep" <<'EOF'
#!/usr/bin/env bash
# 巡回の周期を縮める。SLEEP_MAX 回目の呼び出しで親（conductor-watch.sh）を終了させる。
n=$(( $(cat "$MOCK/sleeps" 2>/dev/null || echo 0) + 1 ))
echo "$n" > "$MOCK/sleeps"
if [[ -n "${SLEEP_MAX:-}" ]] && (( n >= SLEEP_MAX )); then
  kill -TERM "$PPID"; exit 0
fi
exec /bin/sleep 0.03
EOF
chmod +x "$ROOT/bin/sleep"

# 試験用の agent-team.toml（opencode 3 枠・devin 6 枠。検証を通る最小の形）。
write_toml() {
  cat > "$ROOT/repo/agent-team.toml" <<'EOF'
[agents.opencode]
instance = { implement = "opencode", investigate = "opencode" }
models = ["m"]
effort = { option = "variant", raise = ["low"] }
assign_label = ""

[agents.devin]
instance = { implement = "acpRegistry_devin" }
models = ["m"]
effort = { option = "thought_level", raise = ["high"] }
assign_label = "agent:devin"

[agents.codex]
instance = { review = "codex_review" }
models = ["m"]
effort = { option = "reasoningEffort", raise = ["low"] }
assign_label = ""

[agents.claude]
instance = { review = "claude_review" }
models = ["m"]
effort = { option = "effort", raise = ["low"] }
assign_label = ""

[developers.hatai]
use = ["opencode", "devin", "codex"]

[developers.hatai.slots]
opencode = 3
devin = 6

[developers.hatai.review_limits]
codex = 2

[developers.hatai.priority.implement]
normal = ["opencode"]

[developers.hatai.priority.review]
normal = ["codex"]
EOF
}
write_toml

mock_issues() { printf '%s\n' "$1" > "$MOCK/issues.json"; }
mock_prs() { printf '%s\n' "$1" > "$MOCK/prs.json"; }
mock_threads() { printf '%s\n' "${1:-[]}" > "$MOCK/threads.json"; }
mock_threads

iss() {
  python3 -I - "$@" <<'EOF'
import json, sys
print(json.dumps({"number": int(sys.argv[1]), "state": sys.argv[2],
                  "labels": [{"name": l} for l in sys.argv[3:]]}))
EOF
}

jarr() { local out="" x; for x in "$@"; do out+="${out:+,}$x"; done; printf '[%s]' "$out"; }

NOW="$ROOT/now"
echo 1700000000 > "$NOW"
set_now() { echo "$1" > "$NOW"; }

start_watch() {
  MOCK="$MOCK" SLEEP_MAX="${SLEEP_MAX:-}" XDG_STATE_HOME="$ROOT/state" \
  PATH="$ROOT/bin:$PATH" T3_BIN="$ROOT/bin/t3" \
  PELWAY_WATCH_NOW="$NOW" PELWAY_WATCH_HEARTBEAT_SECS=600 \
    bash "$ROOT/repo/scripts/conductor-watch.sh" >>"$ROOT/out.log" 2>&1 &
  WPID=$!
}

run_cycles() {
  local want="$1"
  echo 0 > "$MOCK/sleeps"
  SLEEP_MAX="$want" start_watch
  wait "$WPID" 2>/dev/null; WPID=""
}

check() {
  local name="$1"; shift
  if "$@"; then printf 'ok   %s\n' "$name"; else printf 'FAIL %s\n' "$name"; failures+=("$name"); fi
}

has() { grep -q "$2" "$1"; }
nhas() { ! grep -q "$2" "$1"; }
calls() { wc -l < "$MOCK/gh-calls.log"; }

# 作業状態の印を patrol-state.json の items に書く（mark <issue> <印>。巡回が書き戻しても items は残る）。
mark() {
  python3 -I - "$STATE" "$1" "$2" <<'EOF'
import json, sys
p, n, st = sys.argv[1:4]
try: d = json.load(open(p))
except Exception: d = {}
if not isinstance(d, dict): d = {}
items = d.setdefault("items", {})
it = items.setdefault(n, {})
it["work_state"] = {"state": st}
json.dump(d, open(p, "w"), ensure_ascii=False, indent=2, sort_keys=True)
EOF
}

: > "$ROOT/out.log"; : > "$MOCK/gh-calls.log"

# --- 1. agent:working の open Issue が 2 件（agent:devin なし）で上限 3 → SLOTS opencode 2/3 ---
mock_issues "$(jarr "$(iss 11 OPEN agent:working)" "$(iss 12 OPEN agent:working)")"
mock_prs "[]"
run_cycles 1
check "SLOTS opencode 2/3" has "$ROOT/out.log" '^SLOTS opencode 2/3$'
check "SLOTS devin 0/6" has "$ROOT/out.log" '^SLOTS devin 0/6$'

# --- 2. agent:devin 付きは devin に数え、opencode に数えない。閉じた Issue は数えない ---
mock_issues "$(jarr "$(iss 11 OPEN agent:working)" "$(iss 12 OPEN agent:working)" "$(iss 13 OPEN agent:working agent:devin)" "$(iss 14 CLOSED agent:working agent:devin)")"
run_cycles 1
check "devin-labelled issue counted to devin" has "$ROOT/out.log" '^SLOTS devin 1/6$'
check "devin-labelled issue not counted to opencode" has "$ROOT/out.log" '^SLOTS opencode 2/3$'
check "closed issue not counted" nhas "$ROOT/out.log" '^SLOTS devin 2/6$'

# --- 3. 作業状態の印（停止中・再開処理中・再開待ち）は使用数に含めない ---
mark 11 "停止中"
mark 13 "再開待ち"
mark 15 "再開処理中"
mock_issues "$(jarr "$(iss 11 OPEN agent:working)" "$(iss 12 OPEN agent:working)" "$(iss 13 OPEN agent:working agent:devin)" "$(iss 15 OPEN agent:working)")"
run_cycles 1
check "stopped/resuming issues excluded" has "$ROOT/out.log" '^SLOTS opencode 1/3$'
check "resume-pending devin excluded" has "$ROOT/out.log" '^SLOTS devin 0/6$'

# --- 4. 上限超過は SLOTS over ---
mark 11 "稼働中"
mark 15 "稼働中"
mock_issues "$(jarr "$(iss 11 OPEN agent:working)" "$(iss 12 OPEN agent:working)" "$(iss 13 OPEN agent:working agent:devin)" "$(iss 15 OPEN agent:working)" "$(iss 16 OPEN agent:working)")"
run_cycles 1
check "over-limit becomes SLOTS over" has "$ROOT/out.log" '^SLOTS over opencode 4/3$'

# --- 5. 使用数が変わらない巡回には出ず、HEARTBEAT の巡回には出る ---
: > "$ROOT/out.log"
run_cycles 2
check "no SLOTS on unchanged patrol" nhas "$ROOT/out.log" '^SLOTS '
set_now 1700000700   # heartbeat_at から 600 秒超 → 次の巡回は HEARTBEAT を出す
run_cycles 1
check "HEARTBEAT emitted" has "$ROOT/out.log" '^HEARTBEAT '
check "SLOTS re-emitted on heartbeat patrol" has "$ROOT/out.log" '^SLOTS over opencode 4/3$'

# --- 6. Issue を持たないスレッドは使用数に含めない ---
mock_threads '[{"threadId":"t-noissue","status":"running","title":"[調査] 入口B の件","createdBy":"agent","archived":false,"itemCount":3,"updatedAt":"2026-11-25T00:00:00Z"}]'
: > "$ROOT/out.log"
set_now 1700001400   # ふたたび HEARTBEAT の巡回（SLOTS の行が必ず出る）
run_cycles 1
check "issue-less thread not counted" has "$ROOT/out.log" '^SLOTS over opencode 4/3$'
check "issue-less thread not added" nhas "$ROOT/out.log" '^SLOTS over opencode 5/3$'
mock_threads

# --- 7. agent-team.toml が検証を通らない → SLOTS error。見張りは止まらずほかの行も出る ---
cat > "$ROOT/repo/agent-team.toml" <<'EOF'
[agents.opencode]
EOF
: > "$ROOT/out.log"
set_now 1700002100   # HEARTBEAT の巡回
before=$(calls)
run_cycles 2
check "invalid toml emits SLOTS error" has "$ROOT/out.log" '^SLOTS error '
check "no usage emitted on error" nhas "$ROOT/out.log" '^SLOTS opencode'
check "heartbeat still emitted on invalid toml" has "$ROOT/out.log" '^HEARTBEAT '
check "watch kept cycling on invalid toml" bash -c "(( \$(wc -l < '$MOCK/gh-calls.log') - $before == 6 ))"

# --- 8. 検証を直すと SLOTS の行に戻る ---
write_toml
run_cycles 1
check "SLOTS lines return after toml fixed" has "$ROOT/out.log" '^SLOTS over opencode 4/3$'

# --- 9. agent-team.toml が作業ツリーに無い → SLOTS の行を出さず、見張りは止まらない ---
rm "$ROOT/repo/agent-team.toml"
: > "$ROOT/out.log"
set_now 1700002800   # HEARTBEAT の巡回
before=$(calls)
run_cycles 1
check "no SLOTS lines without toml" nhas "$ROOT/out.log" '^SLOTS '
check "other lines still emitted without toml" has "$ROOT/out.log" '^HEARTBEAT '
check "watch kept cycling without toml" bash -c "(( \$(wc -l < '$MOCK/gh-calls.log') - $before == 3 ))"

# --- 10. 見張りは GitHub に書き込まない（モックが記録した呼び出しは読み取りだけ） ---
if grep -vE "^(dev issue list |dev pr list |dev api /repos/hatai/Pelway/issues/events)" "$MOCK/gh-calls.log" | grep -q .; then
  printf 'FAIL %s\n' "no non-read GitHub calls"
  failures+=("no non-read GitHub calls")
  grep -vE "^(dev issue list |dev pr list |dev api /repos/hatai/Pelway/issues/events)" "$MOCK/gh-calls.log" | head -5
else
  printf 'ok   %s\n' "no non-read GitHub calls"
fi

if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}"
  exit 1
fi
printf '\nall passed\n'
