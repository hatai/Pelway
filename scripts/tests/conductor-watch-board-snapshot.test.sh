#!/usr/bin/env bash
# scripts/conductor-watch.sh のボード読み取りスナップショット（Issue #814）のテスト。
# モック gh が item-list の応答を返し、見張りが人間名義の gh でボードを読んで
# スナップショット JSON を書き出すこと、取得に失敗した巡回では前の書き出しが
# 残ること（消えない・空にならない）、新しい書き出しがあるうちは取り直さない
# ことを確かめる。本物のボードには触れない。
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ROOT="$(mktemp -d)"
WPID=""
trap '[[ -n "${WPID:-}" ]] && kill "$WPID" 2>/dev/null; rm -rf "$ROOT"' EXIT
failures=()

MOCK="$ROOT/mock"
QUEUE="$ROOT/queue"
SNAP="$ROOT/snapshot.json"
mkdir -p "$ROOT/repo/scripts" "$ROOT/bin" "$MOCK" "$ROOT/state" "$QUEUE"

cp "$REPO/scripts/conductor-watch.sh" "$ROOT/repo/scripts/"
cp "$REPO/scripts/patrol-state-update" "$ROOT/repo/scripts/"
cat > "$ROOT/repo/scripts/agent-gh" <<'MOCK1'
#!/usr/bin/env bash
# モック: 巡回の取得は常に空（スナップショットとは無関係）。
if [[ "$2 $3" == "issue list" || "$2 $3" == "pr list" ]]; then echo '[]'; exit 0; fi
exit 9
MOCK1
chmod +x "$ROOT/repo/scripts/agent-gh"

cat > "$ROOT/bin/t3" <<'MOCK2'
#!/usr/bin/env bash
[[ "$1 $2" == "thread list" ]] && { echo '[]'; exit 0; }
exit 9
MOCK2
chmod +x "$ROOT/bin/t3"

cat > "$ROOT/bin/sleep" <<'MOCK3'
#!/usr/bin/env bash
n=$(( $(cat "$MOCK/sleeps" 2>/dev/null || echo 0) + 1 ))
echo "$n" > "$MOCK/sleeps"
if [[ -n "${SLEEP_MAX:-}" ]] && (( n >= SLEEP_MAX )); then
  kill -TERM "$PPID"; exit 0
fi
exec /bin/sleep 0.03
MOCK3
chmod +x "$ROOT/bin/sleep"

cat > "$ROOT/bin/gh" <<'MOCK4'
#!/usr/bin/env bash
# モック gh: 呼び出しを記録し、item-list の固定応答を返す。
# MOCK_READ_FAIL があるとき item-list は失敗する。
echo "$*" >> "$MOCK/gh-calls.log"
if [[ "$1 $2" == "project item-list" ]]; then
  if [[ -n "${MOCK_READ_FAIL:-}" ]]; then echo "boom" >&2; exit 1; fi
  cat "$MOCK/item-list.json"; exit 0
fi
echo "unexpected gh call: $*" >&2
exit 9
MOCK4
chmod +x "$ROOT/bin/gh"

# item-list の返却。content 型のほか status・labels・assignees を項目ごとに持つ
cat > "$MOCK/item-list.json" <<'MOCK5'
{"items": [
  {"id": "PVTI_a", "content": {"type": "Issue", "number": 810, "repository": "hatai/Pelway", "title": "a"},
   "status": "Todo", "labels": ["ready-for-agent"], "assignees": []},
  {"id": "PVTI_b", "content": {"type": "Issue", "number": 811, "repository": "hatai/Pelway", "title": "b"},
   "status": "Todo", "labels": ["ready-for-agent", "P0"], "assignees": ["hatai"]},
  {"id": "PVTI_c", "content": {"type": "PullRequest", "number": 812, "repository": "hatai/Pelway", "title": "c"},
   "status": "In review", "labels": [], "assignees": []}
], "totalCount": 3}
MOCK5

start_watch() {
  local interval="${INTERVAL:-0}" mock_fail="${RFAIL:-}"
  echo 0 > "$MOCK/sleeps"
  MOCK="$MOCK" SLEEP_MAX="$1" MOCK_READ_FAIL="$mock_fail" \
  PELWAY_BOARD_QUEUE_DIR="$QUEUE" XDG_STATE_HOME="$ROOT/state" PATH="$ROOT/bin:$PATH" \
  PELWAY_PATROL_STATE="$ROOT/state/pelway/patrol-state.json" \
  PELWAY_BOARD_SNAPSHOT="$SNAP" PELWAY_BOARD_SNAPSHOT_INTERVAL="$interval" \
  PELWAY_WATCH_NOW="${WATCH_NOW_FILE:-$ROOT/now-absent}" \
    bash "$ROOT/repo/scripts/conductor-watch.sh" >>"$ROOT/out.log" 2>&1 &
  WPID=$!
  wait "$WPID" 2>/dev/null; WPID=""
}

check() {
  local name="$1"; shift
  if "$@"; then printf 'ok   %s\n' "$name"; else printf 'FAIL %s\n' "$name"; failures+=("$name"); fi
}

has() { grep -qF "$2" "$1"; }
nhas() { ! grep -qF "$2" "$1"; }

jqv() { python3 -c 'import json,sys; print(eval(sys.argv[1], {"d": json.load(open(sys.argv[2]))}))' "$1" "$2" 2>/dev/null; }

# 1. 巡回で item-list が読まれ、スナップショットが書き出される
: > "$MOCK/gh-calls.log"
INTERVAL=0 start_watch 2
check "snapshot file written" test -f "$SNAP"
check "snapshot is regular file" test -f "$SNAP" -a ! -L "$SNAP"
check "snapshot version is 1" test "$(jqv 'd["version"]' "$SNAP")" = "1"
check "snapshot has fetched_at" test "$(jqv 'isinstance(d["fetched_at"], (int, float))' "$SNAP")" = "True"
check "snapshot order preserved (810,811,812)" test "$(jqv '[i["number"] for i in d["items"]]' "$SNAP")" = "[810, 811, 812]"
check "snapshot status readable" test "$(jqv 'd["items"][0]["status"]' "$SNAP")" = "Todo"
check "snapshot labels readable" test "$(jqv 'd["items"][1]["labels"]' "$SNAP")" = "['ready-for-agent', 'P0']"
check "snapshot assignees readable" test "$(jqv 'd["items"][1]["assignees"]' "$SNAP")" = "['hatai']"
check "snapshot type readable" test "$(jqv 'd["items"][2]["type"]' "$SNAP")" = "PullRequest"
check "snapshot has no body/title" test "$(jqv '"title" not in d["items"][0] and "body" not in d["items"][0]' "$SNAP")" = "True"

# 2. reader（scripts/board-snapshot-read）がその書き出しを正常と読める
PELWAY_BOARD_SNAPSHOT="$SNAP" python3 "$REPO/scripts/board-snapshot-read" > "$ROOT/read.json"
check "reader accepts written snapshot" test "$(jqv 'd["ok"]' "$ROOT/read.json")" = "True"

# 3. item-list が失敗する巡回では前の書き出しが残る（消えない・空にならない）
before="$(cat "$SNAP")"
INTERVAL=0 RFAIL=1 start_watch 2
check "failed cycle keeps old snapshot" test "$(cat "$SNAP")" = "$before"
check "failed cycle logged" has "$ROOT/out.log" "BOARD snapshot"

# 4. 間隔の制御: 書き出しが新しい（fetched_at が INTERVAL 以内）ときは gh を呼ばない
: > "$MOCK/gh-calls.log"
INTERVAL=86400 start_watch 1
check "fresh snapshot not refetched within interval" nhas "$MOCK/gh-calls.log" "item-list"

# 5. fetched_at が未来の書き出しは「新しい」と扱わず取り直す。未来の時刻を新しいと
#    扱うと、reader が broken と読む書き出しを見張りが更新せず、読み取りが止まる
now_epoch=$(date +%s)
python3 -c 'import json,sys; json.dump({"version": 1, "fetched_at": float(sys.argv[2]) + 3600, "items": []}, open(sys.argv[1], "w"))' "$SNAP" "$now_epoch"
echo "$now_epoch" > "$ROOT/now"
: > "$MOCK/gh-calls.log"
WATCH_NOW_FILE="$ROOT/now" INTERVAL=86400 start_watch 1
check "future fetched_at is refetched" has "$MOCK/gh-calls.log" "item-list"

printf '\n'
if ((${#failures[@]})); then
  printf 'failed: %s\n' "${failures[@]}" >&2
  exit 1
fi
printf 'all tests passed\n'
