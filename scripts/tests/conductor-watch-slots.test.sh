#!/usr/bin/env bash
# scripts/conductor-watch.sh の SLOTS 行と slots.json（Issue #714）のテスト。巡回ごとに枠の使用数を
# `SLOTS <担当> <使用数>/<上限>`（超過は over、未確定は error）で watch.log に出し、同じ判定を slots.json（正本）に書く。
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ROOT="$(mktemp -d)"
WPID=""; failures=()
trap '[[ -n "${WPID:-}" ]] && kill "$WPID" 2>/dev/null; rm -rf "$ROOT"' EXIT

MOCK="$ROOT/mock"
mkdir -p "$ROOT/repo/scripts" "$ROOT/bin" "$MOCK" "$ROOT/state/pelway" "$ROOT/repo/.git/pelway" "$ROOT/queue"

cp "$REPO/scripts/"{conductor-watch.sh,agent-team-config,patrol-state-update} "$ROOT/repo/scripts/"  # 本物を置く偽リポジトリ（agent-gh はモックに差し替える）
git init -q "$ROOT/repo"
STATE="$ROOT/repo/.git/pelway/patrol-state.json"; ISTATE="$ROOT/repo/.git/pelway/patrol-items.json"; SLOTS="$ROOT/state/pelway/conductor-watch/slots.json"

cat > "$ROOT/repo/scripts/agent-gh" <<'EOF'  # モック: 呼び出しを記録し、issue list / pr list / label-events の読み取りだけに答える（issues.json が無いときは失敗―取得失敗の再現。読み取り以外は exit 9）
#!/usr/bin/env bash
echo "$*" >> "$MOCK/gh-calls.log"
[[ "$2 $3" == "issue list" ]] && {
  # 取得の開始と後続の巡回処理で時計が進む形を作るフラグ（updated_at の確認に使う）。
  [[ -f "$MOCK/advance-now" ]] && echo $(( $(cat "$PELWAY_WATCH_NOW") + 120 )) > "$PELWAY_WATCH_NOW"
  cat "$MOCK/issues.json"; exit $?
}
[[ "$2 $3" == "pr list" ]] && { cat "$MOCK/prs.json"; exit $?; }
[[ "$2 $3" == "api /repos/hatai/Pelway/issues/events?per_page=100" ]] && { echo '[]'; exit 0; }
exit 9
EOF
chmod +x "$ROOT/repo/scripts/agent-gh"

cat > "$ROOT/bin/gh" <<'EOF'  # 素の gh のモック: 呼ばれたことを記録して失敗する（実運用のキュー・実際の gh に触れないことの確認）
#!/usr/bin/env bash
echo "$*" >> "$MOCK/gh-direct-calls.log"
exit 9
EOF
chmod +x "$ROOT/bin/gh"

cat > "$ROOT/bin/t3" <<'EOF'
#!/usr/bin/env bash
[[ "$1 $2" == "thread list" ]] && {
  # T3 の一覧の取得中に時計が進む形を作るフラグ（updated_at が最も古い取得開始時刻であることの確認に使う）。
  [[ -f "$MOCK/advance-threads" ]] && echo $(( $(cat "$PELWAY_WATCH_NOW") + 90 )) > "$PELWAY_WATCH_NOW"
  cat "$MOCK/threads.json"; exit 0
}
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

write_toml() {  # 試験用の agent-team.toml（opencode 3 枠・devin 6 枠。検証を通る最小の形）
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

mock_issues() { printf '%s\n' "$1" > "$MOCK/issues.json"; }; mock_prs() { printf '%s\n' "$1" > "$MOCK/prs.json"; }; mock_threads() { printf '%s\n' "${1:-[]}" > "$MOCK/threads.json"; }; mock_threads
iss() { python3 -I -c 'import json,sys
print(json.dumps({"number": int(sys.argv[1]), "state": sys.argv[2],
                  "labels": [{"name": l} for l in sys.argv[3:]]}))' "$@"; }

jarr() { local out="" x; for x in "$@"; do out+="${out:+,}$x"; done; printf '[%s]' "$out"; }; thread_item() { printf '{"threadId":"%s","title":"%s","status":"%s","updatedAt":"%s"}' "$1" "$2" "$3" "${4:-2026-01-01T00:00:00Z}"; }  # t3 thread list の要素（題名の形は「[実装] #<番号> 」）

NOW="$ROOT/now"; echo 1700000000 > "$NOW"; set_now() { echo "$1" > "$NOW"; }

start_watch() {
  MOCK="$MOCK" SLEEP_MAX="${SLEEP_MAX:-}" XDG_STATE_HOME="$ROOT/state" \
  PATH="$ROOT/bin:$PATH" T3_BIN="$ROOT/bin/t3" PELWAY_BOARD_QUEUE_DIR="$ROOT/queue" \
  PELWAY_WATCH_NOW="$NOW" PELWAY_WATCH_HEARTBEAT_SECS=600 \
    bash "$ROOT/repo/scripts/conductor-watch.sh" >>"$ROOT/out.log" 2>&1 &
  WPID=$!
}

run_cycles() {  # 各ケースの新規出力だけを対象にするため、巡回の前にログを切り詰める
  local want="$1"
  : > "$ROOT/out.log"
  echo 0 > "$MOCK/sleeps"
  SLEEP_MAX="$want" start_watch
  wait "$WPID" 2>/dev/null; WPID=""
}

check() { local name="$1"; shift; if "$@"; then printf 'ok   %s\n' "$name"; else printf 'FAIL %s\n' "$name"; failures+=("$name"); fi; }

has() { grep -Eq "$2" "$1"; }; nhas() { ! grep -Eq "$2" "$1"; }; calls() { wc -l < "$MOCK/gh-calls.log"; }
get() { python3 -I -c 'import json,sys; d=json.load(open(sys.argv[2]));
for k in sys.argv[1].split("."): d=d[k]
print(d)' "$1" "$2"; }  # 記録の <key> の値を取り出す（jq 風のパス）
slot_eq() { [[ "$(get "$1" "$SLOTS")" == "$2" ]]; }; ps_eq() { [[ "$(get "$1" "$STATE")" == "$2" ]]; }

jput() { python3 -I -c 'import json,sys
p,n,k,v=sys.argv[1:5]
try: d=json.load(open(p))
except Exception: d={}
if not isinstance(d,dict): d={}
it=d.setdefault("items",{}).setdefault(n,{})
it[k] = it.setdefault(k,[])+[json.loads(v)] if k=="implement_thread" else json.loads(v)
json.dump(d,open(p,"w"),ensure_ascii=False,indent=2,sort_keys=True)' "$1" "$2" "$3" "$4"; }  # <file> の items[<issue>] に <key>=<JSON> を書く（implement_thread は一覧に追加）

mark()         { jput "$ISTATE" "$1" work_state "{\"state\": \"$2\"}"; }  # 作業状態の印（bind/set_rollback も patrol-items.json に書く。litem だけが旧置き場の patrol-state.json）
bind()         { jput "$ISTATE" "$1" implement_thread "{\"pr\": \"${3:-none}\", \"thread_id\": \"$2\", \"branch\": \"agent/x\"}"; }; set_rollback() { jput "$ISTATE" "$1" rollback "{\"assignment_id\": \"a\", \"started_at\": \"t\", \"steps_done\": []}"; }; litem() { jput "$STATE" "$1" work_state "{\"state\": \"$2\"}"; jput "$STATE" "$1" implement_thread "{\"pr\": \"none\", \"thread_id\": \"$3\", \"branch\": \"agent/x\"}"; }

: > "$ROOT/out.log"; : > "$MOCK/gh-calls.log"

# --- 1. agent:working の open Issue が 2 件（agent:devin なし）で上限 3 → SLOTS opencode 2/3 ---
mock_issues "$(jarr "$(iss 11 OPEN agent:working)" "$(iss 12 OPEN agent:working)")"
mock_prs "[]"
run_cycles 1
check "SLOTS opencode 2/3" has "$ROOT/out.log" '^SLOTS opencode 2/3$'
check "SLOTS devin 0/6" has "$ROOT/out.log" '^SLOTS devin 0/6$'
check "slots.json status ok" slot_eq status ok
check "slots.json usage opencode used 2" slot_eq usage.opencode.used 2
check "slots.json usage opencode limit 3" slot_eq usage.opencode.limit 3
check "slots.json updated_at is this patrol" slot_eq updated_at 1700000000.0

# --- 2. agent:devin 付きは devin に数え、opencode に数えない。閉じた Issue は数えない ---
mock_issues "$(jarr "$(iss 11 OPEN agent:working)" "$(iss 12 OPEN agent:working)" "$(iss 13 OPEN agent:working agent:devin)" "$(iss 14 CLOSED agent:working agent:devin)")"
run_cycles 1
check "devin-labelled issue counted to devin" has "$ROOT/out.log" '^SLOTS devin 1/6$'
check "devin-labelled issue not counted to opencode" has "$ROOT/out.log" '^SLOTS opencode 2/3$'
check "closed issue not counted" nhas "$ROOT/out.log" '^SLOTS devin 2/6$'

# --- 3. 印が停止中・再開処理中・再開待ちで、現在の割り当てに結び付いたスレッドが動いていないと確かめられたものだけが枠を空ける ---
# （起動を試みる前の待機の印は枠を消費しない。起動を試みた回は巻き戻し中の印が残るため使用中）
mark 11 "停止中";   bind 11 t1
mark 13 "再開待ち"; bind 13 t2
mark 15 "再開処理中"; bind 15 t3
mock_threads "$(jarr "$(thread_item t1 '[実装] #11 a' completed)" "$(thread_item t2 '[実装] #13 b' failed)" "$(thread_item t3 '[実装] #15 c' interrupted)")"
mock_issues "$(jarr "$(iss 11 OPEN agent:working)" "$(iss 12 OPEN agent:working)" "$(iss 13 OPEN agent:working agent:devin)" "$(iss 15 OPEN agent:working)" "$(iss 16 OPEN agent:working)")"
run_cycles 1
check "stopped and pre-attempt-resume issues excluded" has "$ROOT/out.log" '^SLOTS opencode 2/3$'
check "pre-attempt resume marks free the slot" has "$ROOT/out.log" '^SLOTS devin 0/6$'

# --- 3b. 印だけでは由来を確かめられない: 組の無いもの・動いているスレッドが残るものは使用中として数える ---
mark 16 "停止中"   # implement_thread の組もスレッドも無い → 確かめられない → 使用中
mark 17 "停止中"; bind 17 t4   # 組のスレッドが動いている → 使用中
mark 18 "停止中"; bind 18 t5   # 組のスレッドは止まっているが一覧に動いているものが残る → 使用中
mark 19 "再開待ち" # 組が無い再開の印は由来を確かめられない → 使用中
mock_threads "$(jarr \
  "$(thread_item t1 '[実装] #11 a' completed)" \
  "$(thread_item t2 '[実装] #13 b' failed)" \
  "$(thread_item t3 '[実装] #15 c' interrupted)" \
  "$(thread_item t4 '[実装] #17 d' running)" \
  "$(thread_item t5 '[実装] #18 e' completed)" \
  "$(thread_item t6 '[実装] #18 f' running)")"
mock_issues "$(jarr "$(iss 11 OPEN agent:working)" "$(iss 12 OPEN agent:working)" "$(iss 13 OPEN agent:working agent:devin)" "$(iss 15 OPEN agent:working)" "$(iss 16 OPEN agent:working)" "$(iss 17 OPEN agent:working)" "$(iss 18 OPEN agent:working)" "$(iss 19 OPEN agent:working)")"
run_cycles 1
check "unverified marks counted as used" has "$ROOT/out.log" '^SLOTS over opencode 5/3$'
check "devin resume mark still free" has "$ROOT/out.log" '^SLOTS devin 0/6$'

# --- 3c. 起動を試みる前の「再開待ち」は枠を空ける（組のスレッドが一覧に無いものは使用中） ---
mark 20 "再開待ち"; bind 20 t-old-20
mark 22 "停止中";   bind 22 t-missing-22
mock_threads "$(jarr "$(thread_item t-old-20 '[実装] #20 old' completed)")"
mock_issues "$(jarr "$(iss 12 OPEN agent:working)" "$(iss 20 OPEN agent:working)" "$(iss 22 OPEN agent:working)")"
run_cycles 1
check "pre-attempt resume mark frees the slot" has "$ROOT/out.log" '^SLOTS opencode 2/3$'

# --- 3d. 起動を試みたあと（rollback が残るもの）は、組のスレッドが止まっていても使用中（再開待ちでも同じ） ---
mark 21 "停止中"; bind 21 t-rb; set_rollback 21
mark 23 "再開待ち"; bind 23 t-rb2; set_rollback 23
mock_threads "$(jarr "$(thread_item t-rb '[実装] #21 g' completed)" "$(thread_item t-rb2 '[実装] #23 h' completed)")"
mock_issues "$(jarr "$(iss 12 OPEN agent:working)" "$(iss 21 OPEN agent:working)" "$(iss 23 OPEN agent:working)")"
run_cycles 1
check "post-attempt assignments counted as used" has "$ROOT/out.log" '^SLOTS opencode 3/3$'

# --- 3e. 巡回の書き戻しは patrol-items.json を一度も書き換えない ---
items_hash_before="$(sha256sum "$ISTATE" | cut -d' ' -f1)"; run_cycles 1
check "watch never rewrites patrol-items.json" bash -c "[[ \$(sha256sum '$ISTATE' | cut -d' ' -f1) == '$items_hash_before' ]]"

# --- 3f. patrol-items.json が読めない回は、旧置き場の古い記録で枠を空けない（使用中に倒す） ---
litem 24 "停止中" t-24
echo '{' > "$ISTATE"
mock_threads "$(jarr "$(thread_item t-24 '[実装] #24 i' completed)")"
mock_issues "$(jarr "$(iss 12 OPEN agent:working)" "$(iss 24 OPEN agent:working)")"
run_cycles 1
check "unreadable patrol-items keeps slot counted" has "$ROOT/out.log" '^SLOTS opencode 2/3$'
# --- 3g. patrol-items.json が無い回は、旧置き場が残っていても読まない（旧記録は復活しうるため） ---
rm -f "$ISTATE"
litem 24 "停止中" t-24; litem 25 "再開待ち" t-25
mock_threads "$(jarr "$(thread_item t-24 '[実装] #24 i' completed)" "$(thread_item t-25 '[実装] #25 i' completed)")"
mock_issues "$(jarr "$(iss 12 OPEN agent:working)" "$(iss 24 OPEN agent:working)" "$(iss 25 OPEN agent:working)")"
run_cycles 1
check "missing patrol-items never reads legacy" has "$ROOT/out.log" '^SLOTS opencode 3/3$'
check "missing patrol-items publishes all-used usage" slot_eq "usage.opencode.used" "3"
# --- 3h. 旧置き場も無い回も同じくすべて使用中（直前と同じ値で行は出ないため記録で確かめる） ---
python3 -I -c 'import json,sys; p=sys.argv[1]; d=json.load(open(p)); d.pop("items",None)
json.dump(d,open(p,"w"),ensure_ascii=False,indent=2,sort_keys=True)' "$STATE"
run_cycles 1
check "post-migration loss counts all used" slot_eq "usage.opencode.used" "3"

# --- 4. 上限超過は SLOTS over ---
mark 16 "稼働中"; mark 17 "稼働中"; mark 18 "稼働中"; mark 19 "稼働中"
mark 11 "稼働中"
mark 15 "稼働中"
mock_issues "$(jarr "$(iss 11 OPEN agent:working)" "$(iss 12 OPEN agent:working)" "$(iss 13 OPEN agent:working agent:devin)" "$(iss 15 OPEN agent:working)" "$(iss 16 OPEN agent:working)")"
run_cycles 1
check "over-limit becomes SLOTS over" has "$ROOT/out.log" '^SLOTS over opencode 4/3$'

# --- 5. 使用数が変わらない巡回には出ず、HEARTBEAT の巡回には出る ---
run_cycles 2
check "no SLOTS on unchanged patrol" nhas "$ROOT/out.log" '^SLOTS '
set_now 1700000700   # heartbeat_at から 600 秒超 → 次の巡回は HEARTBEAT を出す
run_cycles 1
check "HEARTBEAT emitted" has "$ROOT/out.log" '^HEARTBEAT '
check "SLOTS re-emitted on heartbeat patrol" has "$ROOT/out.log" '^SLOTS over opencode 4/3$'

# --- 6. Issue を持たないスレッドは使用数に含めない ---
mock_threads '[{"threadId":"t-noissue","status":"running","title":"[調査] 入口B の件","createdBy":"agent","archived":false,"itemCount":3,"updatedAt":"2026-11-25T00:00:00Z"}]'
set_now 1700001400   # ふたたび HEARTBEAT の巡回（SLOTS の行が必ず出る）
run_cycles 1
check "issue-less thread not counted" has "$ROOT/out.log" '^SLOTS over opencode 4/3$'
check "issue-less thread not added" nhas "$ROOT/out.log" '^SLOTS over opencode 5/3$'
mock_threads

# --- 7. agent-team.toml が検証を通らない → SLOTS error。見張りは止まらずほかの行も出る ---
cat > "$ROOT/repo/agent-team.toml" <<'EOF'
[agents.opencode]
EOF
set_now 1700002100   # HEARTBEAT の巡回
before=$(calls)
run_cycles 2
check "invalid toml emits SLOTS error" has "$ROOT/out.log" '^SLOTS error '
check "no usage emitted on error" nhas "$ROOT/out.log" '^SLOTS (over )?[a-z]+ [0-9]+/[0-9]+$'
check "heartbeat still emitted on invalid toml" has "$ROOT/out.log" '^HEARTBEAT '
check "watch kept cycling on invalid toml" bash -c "(( \$(wc -l < '$MOCK/gh-calls.log') - $before == 6 ))"
check "slots.json status error" slot_eq status error

# --- 8. 検証を直すと SLOTS の行に戻る ---
write_toml
run_cycles 1
check "SLOTS lines return after toml fixed" has "$ROOT/out.log" '^SLOTS over opencode 4/3$'

# --- 9. issue 一覧の取得が失敗した巡回は SLOTS error。直前の巡回の行が残ったまま使われない ---
rm "$MOCK/issues.json"
run_cycles 1
check "issue fetch failure emits SLOTS error" has "$ROOT/out.log" '^SLOTS error '
check "no stale usage on fetch failure" nhas "$ROOT/out.log" '^SLOTS (over )?[a-z]+ [0-9]+/[0-9]+$'
check "slots.json status error on fetch failure" slot_eq status error
mock_issues "$(jarr "$(iss 11 OPEN agent:working)" "$(iss 12 OPEN agent:working)" "$(iss 13 OPEN agent:working agent:devin)" "$(iss 15 OPEN agent:working)" "$(iss 16 OPEN agent:working)")"  # 取得に戻った巡回は、あらためて最新の使用数を出す
run_cycles 1
check "fresh usage re-emitted after recovery" has "$ROOT/out.log" '^SLOTS over opencode 4/3$'
check "slots.json back to ok" slot_eq status ok

# --- 10. issue 一覧が件数の上限（1000 件）に達した巡回は SLOTS error ---
python3 -I - > "$MOCK/issues.json" <<'EOF'
import json
print(json.dumps([{"number": i, "state": "OPEN", "labels": [{"name": "agent:working"}]} for i in range(1000)]))
EOF
run_cycles 1
check "truncated issue list emits SLOTS error" has "$ROOT/out.log" '^SLOTS error '
check "no usage emitted on truncated list" nhas "$ROOT/out.log" '^SLOTS (over )?[a-z]+ [0-9]+/[0-9]+$'
mock_issues "$(jarr "$(iss 11 OPEN agent:working)" "$(iss 12 OPEN agent:working)" "$(iss 13 OPEN agent:working agent:devin)" "$(iss 15 OPEN agent:working)" "$(iss 16 OPEN agent:working)")"

# --- 11. キューの要求のファイル名が watch.log に SLOTS の行を作れても、slots.json は実測のまま ---
# （ファイル名に / は使えないため `SLOTS error` 系の行を偽装できる。判断が slots.json で行われるのはこの経路のため）
touch "$ROOT/queue/"$'\n'"SLOTS error injected"$'\n'".json"
run_cycles 1
check "queue filename can forge a SLOTS error line" has "$ROOT/out.log" '^SLOTS error injected$'
check "slots.json status stays ok despite forged line" slot_eq status ok
check "slots.json keeps real usage despite forged line" slot_eq usage.opencode.used 4

# --- 12. agent-team.toml が作業ツリーに無い → SLOTS の行を出さず、見張りは止まらない ---
rm "$ROOT/repo/agent-team.toml"
set_now 1700002800   # HEARTBEAT の巡回
before=$(calls)
run_cycles 1
check "no SLOTS lines without toml" nhas "$ROOT/out.log" '^SLOTS '
check "other lines still emitted without toml" has "$ROOT/out.log" '^HEARTBEAT '
check "watch kept cycling without toml" bash -c "(( \$(wc -l < '$MOCK/gh-calls.log') - $before == 3 ))"
check "slots.json status unavailable without toml" slot_eq status unavailable

# --- 13. slots.json を書けない巡回でも見張りは止まらず、巡回の残りの処理を続ける ---
write_toml
rm "$SLOTS"; mkdir "$SLOTS"   # 置き換え先をディレクトリにして、前回記録の除去（os.unlink）も公開（os.replace）も失敗させる
set_now 1700003500
: > "$ROOT/out.log"
before=$(calls)
run_cycles 1
check "watch kept cycling when slots.json unwritable" bash -c "(( \$(wc -l < '$MOCK/gh-calls.log') - $before == 3 ))"
check "other lines still emitted when slots.json unwritable" has "$ROOT/out.log" '^HEARTBEAT '
# 公開に失敗した巡回でも、公開より後の巡回処理（補助記録の更新）まで進むことを確かめる
check "patrol-state updated when slots.json unwritable" \
  ps_eq updated_at "$(python3 -I -c 'import datetime; print(datetime.datetime.fromtimestamp(1700003500).isoformat(timespec="seconds"))')"
check "slots.json not a readable file when blocked" bash -c "[[ ! -f '$SLOTS' ]]"
rmdir "$SLOTS"
run_cycles 1
check "slots.json resumes after path restored" slot_eq status ok
# 読み手の規約（runbook）の基礎: 公開できた巡回は slots.json と state.json の patrol_at が一致する
check "record patrol_at matches state.json" \
  python3 -I -c 'import json,sys
s=json.load(open(sys.argv[1])); w=json.load(open(sys.argv[2])); sys.exit(0 if s["patrol_at"]==w["patrol_at"] else 1)' "$SLOTS" "$(dirname "$SLOTS")/state.json"

# --- 13b. 読めるまま残った古い記録は patrol_at で見抜ける ---
# 公開に失敗した回に前回の記録が読めるまま残った形を作る（書き手側の失敗経路では作れない
# ため、同じ内容の記録を書いて読み手側の判定を確かめる）。
python3 -I - "$SLOTS" <<'EOF'
import json, sys
s = json.load(open(sys.argv[1]))
s["patrol_at"] -= 60      # 残留物: state.json の巡回より前の記録
s["updated_at"] -= 10     # 60 秒の鮮度の内側に収まる
json.dump(s, open(sys.argv[1], "w"), ensure_ascii=False)
EOF
check "residue is still a readable ok record" slot_eq status ok
check "residue passes updated_at freshness" \
  python3 -I -c 'import json,sys,time
s=json.load(open(sys.argv[1])); w=json.load(open(sys.argv[2])); sys.exit(0 if w["patrol_at"]-s["updated_at"] < 60 else 1)' "$SLOTS" "$(dirname "$SLOTS")/state.json"
check "stale readable record detectable via patrol_at" \
  python3 -I -c 'import json,sys
s=json.load(open(sys.argv[1])); w=json.load(open(sys.argv[2])); sys.exit(0 if s["patrol_at"] < w["patrol_at"] else 1)' "$SLOTS" "$(dirname "$SLOTS")/state.json"
run_cycles 1
check "republished record patrol_at matches state.json" \
  python3 -I -c 'import json,sys
s=json.load(open(sys.argv[1])); w=json.load(open(sys.argv[2])); sys.exit(0 if s["patrol_at"]==w["patrol_at"] else 1)' "$SLOTS" "$(dirname "$SLOTS")/state.json"

# --- 13c. updated_at は Issue 一覧の取得開始時刻（後続の処理が遅れても記録の時刻を押し上げない） ---
touch "$MOCK/advance-now"
set_now 1700003800
run_cycles 1
rm -f "$MOCK/advance-now"
check "updated_at is issue-list fetch start" slot_eq updated_at 1700003800.0
check "patrol work runs on the later clock" \
  ps_eq updated_at "$(python3 -I -c 'import datetime; print(datetime.datetime.fromtimestamp(1700003920).isoformat(timespec="seconds"))')"
check "patrol_at is the patrol's processing time" slot_eq patrol_at 1700003920.0

# --- 13d. updated_at は使用数の根拠のうち最も古い取得開始時刻（T3 一覧の取得が遅れた回も古い時刻を保つ） ---
touch "$MOCK/advance-threads"
set_now 1700004000
run_cycles 1
rm -f "$MOCK/advance-threads"
check "updated_at is T3-list fetch start" slot_eq updated_at 1700004000.0
check "patrol work runs on the post-T3 clock" slot_eq patrol_at 1700004090.0

# --- 13e. 一覧の取得開始時刻が記録できなかった巡回は使用数を公開しない ---
# （threads-at の置き場をディレクトリにして書き込みだけを失敗させ、遅い T3 の取得を重ねる）
rm -f "$(dirname "$SLOTS")/threads-at"
mkdir "$(dirname "$SLOTS")/threads-at"
touch "$MOCK/advance-threads"
set_now 1700005000
run_cycles 1
rmdir "$(dirname "$SLOTS")/threads-at"
rm -f "$MOCK/advance-threads"
check "unrecorded fetch time emits SLOTS error" has "$ROOT/out.log" '^SLOTS error fetch-time-missing$'
check "slots.json status error on unrecorded fetch time" slot_eq status error
run_cycles 1
check "slots.json back to ok when fetch time recorded" slot_eq status ok

# --- 13f. 残った時刻ファイルが読めても、今回の巡回で書けていなければ使わない（古い値で鮮度を誤魔化さない） ---
TSA="$(dirname "$SLOTS")/threads-at"
if (( EUID == 0 )); then rm -f "$TSA"; ln -s /dev/full "$TSA"; else rm -f "$TSA"; echo 0 > "$TSA"; chmod 0444 "$TSA"; fi
touch "$MOCK/advance-threads"
set_now 1700006000
run_cycles 1
rm -f "$MOCK/advance-threads" "$TSA"
check "stale readable fetch time still errors" has "$ROOT/out.log" '^SLOTS error fetch-time-missing$'
check "slots.json status error with stale fetch time" slot_eq status error
run_cycles 1
check "slots.json recovers after stale file removed" slot_eq status ok

# --- 14. 見張りは GitHub に書き込まない（agent-gh は読み取りだけ、素の gh も呼ばない） ---
if grep -vE "^(dev issue list |dev pr list |dev api /repos/hatai/Pelway/issues/events)" "$MOCK/gh-calls.log" | grep -q .; then
  printf 'FAIL %s\n' "no non-read agent-gh calls"
  failures+=("no non-read agent-gh calls")
  grep -vE "^(dev issue list |dev pr list |dev api /repos/hatai/Pelway/issues/events)" "$MOCK/gh-calls.log" | head -5
else
  printf 'ok   %s\n' "no non-read agent-gh calls"
fi
check "no direct gh calls (board queue isolated)" bash -c "[[ ! -s '$MOCK/gh-direct-calls.log' ]]"

if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}"
  exit 1
fi
printf '\nall passed\n'
