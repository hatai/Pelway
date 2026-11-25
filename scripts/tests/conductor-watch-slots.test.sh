#!/usr/bin/env bash
# scripts/conductor-watch.sh の SLOTS 行と slots.json（Issue #714）のテスト。
# 巡回ごとに、実装と調査に使っている枠の数を agent-team.toml の上限と比べ、
# `SLOTS <担当> <使用数>/<上限>`（上限超過は `SLOTS over`、確かめられない巡回は `SLOTS error`）を
# watch.log に出し、同じ判定を状態ディレクトリの slots.json に書く。指揮役の判断は
# slots.json で行う（キューの要求のファイル名がそのまま watch.log の行になるため）。
# 出すのは使用数または状態が前回の巡回から変わったときと、HEARTBEAT の行を出した巡回。
# agent-team.toml が作業ツリーに無いときは出さない。見張りは GitHub に書き込まない。
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ROOT="$(mktemp -d)"
WPID=""
trap '[[ -n "${WPID:-}" ]] && kill "$WPID" 2>/dev/null; rm -rf "$ROOT"' EXIT
failures=()

MOCK="$ROOT/mock"
mkdir -p "$ROOT/repo/scripts" "$ROOT/bin" "$MOCK" "$ROOT/state/pelway" "$ROOT/repo/.git/pelway" "$ROOT/queue"

# 本物の conductor-watch.sh と agent-team-config を置く偽リポジトリ
# （agent-gh はモックに差し替える。patrol-state.json の既定の置き場の解決を実際に通す）。
cp "$REPO/scripts/conductor-watch.sh" "$ROOT/repo/scripts/"
cp "$REPO/scripts/agent-team-config" "$ROOT/repo/scripts/"
git init -q "$ROOT/repo"
STATE="$ROOT/repo/.git/pelway/patrol-state.json"
SLOTS="$ROOT/state/pelway/conductor-watch/slots.json"

# モックの agent-gh: 呼び出しをすべて記録し、issue list / pr list / label-events の読み取りだけに答える。
# issues.json が無いときは失敗する（issue 一覧の取得失敗を再現する）。
# 読み取り以外の呼び出しは exit 9 で失敗する（見張りが GitHub に書き込まないことの確認に使う）。
cat > "$ROOT/repo/scripts/agent-gh" <<'EOF'
#!/usr/bin/env bash
echo "$*" >> "$MOCK/gh-calls.log"
[[ "$2 $3" == "issue list" ]] && { cat "$MOCK/issues.json"; exit $?; }
[[ "$2 $3" == "pr list" ]] && { cat "$MOCK/prs.json"; exit $?; }
[[ "$2 $3" == "api /repos/hatai/Pelway/issues/events?per_page=100" ]] && { echo '[]'; exit 0; }
# 印の裏付けに使う Issue のコメント。$MOCK/comments-<N>.json が無いときは失敗する
# （取得失敗 → 印を確かめられず使用中として数える経路の確認に使う）。
[[ "$2" == "api" && "$3" =~ ^/repos/hatai/Pelway/issues/([0-9]+)/comments ]] && { cat "$MOCK/comments-${BASH_REMATCH[1]}.json"; exit $?; }
exit 9
EOF
chmod +x "$ROOT/repo/scripts/agent-gh"

# 素の gh のモック: ボード更新キューの処理が呼ぶ。呼ばれたことを記録して失敗する
# （テスト中の見張りが実運用のキューや実際の gh に触れないことの確認に使う）。
cat > "$ROOT/bin/gh" <<'EOF'
#!/usr/bin/env bash
echo "$*" >> "$MOCK/gh-direct-calls.log"
exit 9
EOF
chmod +x "$ROOT/bin/gh"

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
  PATH="$ROOT/bin:$PATH" T3_BIN="$ROOT/bin/t3" PELWAY_BOARD_QUEUE_DIR="$ROOT/queue" \
  PELWAY_WATCH_NOW="$NOW" PELWAY_WATCH_HEARTBEAT_SECS=600 \
    bash "$ROOT/repo/scripts/conductor-watch.sh" >>"$ROOT/out.log" 2>&1 &
  WPID=$!
}

# 各ケースの新規出力だけを対象にするため、巡回の前にログを切り詰める。
run_cycles() {
  local want="$1"
  : > "$ROOT/out.log"
  echo 0 > "$MOCK/sleeps"
  SLEEP_MAX="$want" start_watch
  wait "$WPID" 2>/dev/null; WPID=""
}

check() {
  local name="$1"; shift
  if "$@"; then printf 'ok   %s\n' "$name"; else printf 'FAIL %s\n' "$name"; failures+=("$name"); fi
}

has() { grep -Eq "$2" "$1"; }
nhas() { ! grep -Eq "$2" "$1"; }
calls() { wc -l < "$MOCK/gh-calls.log"; }
# slots.json の <key> の値を取り出す（status・usage.<担当>.used などの jq 風のパス）
slot() { python3 -I -c 'import json,sys; d=json.load(open(sys.argv[2]));
for k in sys.argv[1].split("."): d=d[k]
print(d)' "$1" "$SLOTS"; }
# slot_eq <パス> <期待値> — slots.json の値が期待値と等しい
slot_eq() { [[ "$(slot "$1")" == "$2" ]]; }

# patrol-state.json の <key> の値を取り出す（slots.json の公開より後の巡回処理の効果を見る）。
psval() { python3 -I -c 'import json,sys; d=json.load(open(sys.argv[2]));
for k in sys.argv[1].split("."): d=d[k]
print(d)' "$1" "$STATE"; }
ps_eq() { [[ "$(psval "$1")" == "$2" ]]; }

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

# --- 3. 作業状態の印＋裏付けのコメント（[stop-processing] が最新の記録）は使用数に含めない ---
mark 11 "停止中"
mark 13 "再開待ち"
mark 15 "再開処理中"
printf '%s\n' '[{"user":{"login":"pelway-dev[bot]"},"body":"[stop-processing] sp-1"}]' > "$MOCK/comments-11.json"
printf '%s\n' '[{"user":{"login":"pelway-dev[bot]"},"body":"[stop-processing] sp-2"}]' > "$MOCK/comments-13.json"
printf '%s\n' '[{"user":{"login":"pelway-dev[bot]"},"body":"[stop-processing] sp-3"}]' > "$MOCK/comments-15.json"
mock_issues "$(jarr "$(iss 11 OPEN agent:working)" "$(iss 12 OPEN agent:working)" "$(iss 13 OPEN agent:working agent:devin)" "$(iss 15 OPEN agent:working)")"
run_cycles 1
check "stopped/resuming issues excluded" has "$ROOT/out.log" '^SLOTS opencode 1/3$'
check "resume-pending devin excluded" has "$ROOT/out.log" '^SLOTS devin 0/6$'

# --- 3b. 印だけでは由来を確かめられない: 裏付けが無い・取得失敗・その後の claim がある印は使用中として数える ---
mark 16 "停止中"   # comments-16.json 無し（取得失敗）→ 使用中
mark 17 "停止中"
printf '%s\n' '[]' > "$MOCK/comments-17.json"   # 取得できるが記録無し → 使用中
mark 18 "再開待ち"
printf '%s\n' '[{"user":{"login":"pelway-dev[bot]"},"body":"[stop-processing] sp-4"},{"user":{"login":"pelway-dev[bot]"},"body":"<!-- claim branch=agent/18-x -->"}]' > "$MOCK/comments-18.json"   # 停止の後に取得 → 使用中
mark 19 "停止中"
printf '%s\n' '[{"user":{"login":"someone"},"body":"[stop-processing] forged"}]' > "$MOCK/comments-19.json"   # pelway-dev[bot] 以外 → 使用中
mock_issues "$(jarr "$(iss 11 OPEN agent:working)" "$(iss 12 OPEN agent:working)" "$(iss 13 OPEN agent:working agent:devin)" "$(iss 15 OPEN agent:working)" "$(iss 16 OPEN agent:working)" "$(iss 17 OPEN agent:working)" "$(iss 18 OPEN agent:working)" "$(iss 19 OPEN agent:working)")"
run_cycles 1
check "unverified marks counted as used" has "$ROOT/out.log" '^SLOTS over opencode 5/3$'
check "unverified devin mark counted" has "$ROOT/out.log" '^SLOTS devin 0/6$'

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
# 取得に戻った巡回は、あらためて最新の使用数を出す
mock_issues "$(jarr "$(iss 11 OPEN agent:working)" "$(iss 12 OPEN agent:working)" "$(iss 13 OPEN agent:working agent:devin)" "$(iss 15 OPEN agent:working)" "$(iss 16 OPEN agent:working)")"
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
# （ファイル名に / は使えないため `SLOTS error` 系の行を偽装できる。使用数の行 <n>/<m> は
# ファイル名だけでは作れない。判断が slots.json で行われるのはこの偽装経路のため）
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
check "watch kept cycling when slots.json unwritable" bash -c "(( \$(wc -l < '$MOCK/gh-calls.log') - $before == 4 ))"
check "other lines still emitted when slots.json unwritable" has "$ROOT/out.log" '^HEARTBEAT '
# 公開に失敗した巡回でも、公開より後の巡回処理（補助記録の更新）まで進むことを確かめる
check "patrol-state updated when slots.json unwritable" \
  ps_eq updated_at "$(python3 -I -c 'import datetime; print(datetime.datetime.fromtimestamp(1700003500).isoformat(timespec="seconds"))')"
check "slots.json not a readable file when blocked" bash -c "[[ ! -f '$SLOTS' ]]"
rmdir "$SLOTS"
run_cycles 1
check "slots.json resumes after path restored" slot_eq status ok
# 読み手の鮮度判定（runbook）の基礎: 公開できた巡回は slots.json と state.json の更新時刻が同じ巡回になる
check "slots.json mtime tracks state.json mtime" \
  bash -c "(( \$(stat -c %Y '$(dirname "$SLOTS")/state.json') - \$(stat -c %Y '$SLOTS') < 30 ))"

# --- 14. 見張りは GitHub に書き込まない（agent-gh は読み取りだけ、素の gh も呼ばない） ---
if grep -vE "^(dev issue list |dev pr list |dev api /repos/hatai/Pelway/issues/(events|[0-9]+/comments))" "$MOCK/gh-calls.log" | grep -q .; then
  printf 'FAIL %s\n' "no non-read agent-gh calls"
  failures+=("no non-read agent-gh calls")
  grep -vE "^(dev issue list |dev pr list |dev api /repos/hatai/Pelway/issues/(events|[0-9]+/comments))" "$MOCK/gh-calls.log" | head -5
else
  printf 'ok   %s\n' "no non-read agent-gh calls"
fi
check "no direct gh calls (board queue isolated)" bash -c "[[ ! -s '$MOCK/gh-direct-calls.log' ]]"

if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}"
  exit 1
fi
printf '\nall passed\n'
