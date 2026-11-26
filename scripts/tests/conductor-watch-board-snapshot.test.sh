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
# MOCK_TRUNCATE があるとき、--limit が 5 より小さい呼び出しには 3 件だけの
# 打ち切られた応答（totalCount 5）を返し、5 以上なら 5 件の応答を返す。
# MOCK_BAD_ITEM があるとき、番号の無い Issue を含む応答を返す。
echo "$*" >> "$MOCK/gh-calls.log"
if [[ "$1 $2" == "project item-list" ]]; then
  if [[ -n "${MOCK_READ_FAIL:-}" ]]; then echo "boom" >&2; exit 1; fi
  if [[ -n "${MOCK_TRUNCATE:-}" ]]; then
    lim=1000; prev=""
    for a in "$@"; do [[ "$prev" == "--limit" ]] && lim="$a"; prev="$a"; done
    if [[ "$lim" -lt 5 ]]; then cat "$MOCK/item-list-truncated.json"; else cat "$MOCK/item-list-full5.json"; fi
    exit 0
  fi
  if [[ -n "${MOCK_BAD_ITEM:-}" ]]; then cat "$MOCK/item-list-bad.json"; exit 0; fi
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

# 打ち切りの試験用: 3 件だけの応答（totalCount 5）と 5 件の応答
cat > "$MOCK/item-list-truncated.json" <<'MOCK6'
{"items": [
  {"id": "PVTI_a", "content": {"type": "Issue", "number": 810, "repository": "hatai/Pelway", "title": "a"},
   "status": "Todo", "labels": [], "assignees": []},
  {"id": "PVTI_b", "content": {"type": "Issue", "number": 811, "repository": "hatai/Pelway", "title": "b"},
   "status": "Todo", "labels": [], "assignees": []},
  {"id": "PVTI_c", "content": {"type": "Issue", "number": 812, "repository": "hatai/Pelway", "title": "c"},
   "status": "Todo", "labels": [], "assignees": []}
], "totalCount": 5}
MOCK6
cat > "$MOCK/item-list-full5.json" <<'MOCK7'
{"items": [
  {"id": "PVTI_a", "content": {"type": "Issue", "number": 810, "repository": "hatai/Pelway", "title": "a"},
   "status": "Todo", "labels": [], "assignees": []},
  {"id": "PVTI_b", "content": {"type": "Issue", "number": 811, "repository": "hatai/Pelway", "title": "b"},
   "status": "Todo", "labels": [], "assignees": []},
  {"id": "PVTI_c", "content": {"type": "Issue", "number": 812, "repository": "hatai/Pelway", "title": "c"},
   "status": "Todo", "labels": [], "assignees": []},
  {"id": "PVTI_d", "content": {"type": "Issue", "number": 813, "repository": "hatai/Pelway", "title": "d"},
   "status": "Todo", "labels": [], "assignees": []},
  {"id": "PVTI_e", "content": {"type": "Issue", "number": 814, "repository": "hatai/Pelway", "title": "e"},
   "status": "Todo", "labels": [], "assignees": []}
], "totalCount": 5}
MOCK7
cat > "$MOCK/item-list-bad.json" <<'MOCK8'
{"items": [
  {"id": "PVTI_x", "content": {"type": "Issue", "repository": "hatai/Pelway", "title": "x"},
   "status": "Todo", "labels": [], "assignees": []}
], "totalCount": 1}
MOCK8

start_watch() {
  local interval="${INTERVAL:-0}" mock_fail="${RFAIL:-}" mock_truncate="${TRUNC:-}" mock_bad="${BADITEM:-}"
  echo 0 > "$MOCK/sleeps"
  MOCK="$MOCK" SLEEP_MAX="$1" MOCK_READ_FAIL="$mock_fail" MOCK_TRUNCATE="$mock_truncate" MOCK_BAD_ITEM="$mock_bad" \
  PELWAY_BOARD_QUEUE_DIR="$QUEUE" XDG_STATE_HOME="$ROOT/state" PATH="$ROOT/bin:$PATH" \
  PELWAY_PATROL_STATE="$ROOT/state/pelway/patrol-state.json" \
  PELWAY_BOARD_SNAPSHOT="$SNAP" PELWAY_BOARD_SNAPSHOT_INTERVAL="$interval" \
  PELWAY_BOARD_SNAPSHOT_FIRST_LIMIT="${FIRST_LIMIT:-}" \
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

# 4. 間隔の制御: 同じ見張りのプロセスの中では、間隔（INTERVAL 秒）より前に
#    取ったばかりなら取り直さない（1 回の起動で item-list は 1 回だけ）
: > "$MOCK/gh-calls.log"
INTERVAL=86400 start_watch 4
n_calls=$(grep -c "item-list" "$MOCK/gh-calls.log" || true)
check "within interval only one fetch per process" test "$n_calls" -eq 1

# 5. 偽の新しい書き出し（エージェントが共有の置き場に置ける）があっても取り直す。
#    鮮度の判定に共有の書き出しの内容を使わない（使うと、偽の fetched_at で
#    取り直しが止まり、偽のカードが本物に戻らない）
now_epoch=$(date +%s)
python3 -c 'import json,sys; json.dump({"version": 1, "fetched_at": float(sys.argv[2]), "items": [{"id": "FAKE", "type": "Issue", "number": 999, "status": "Todo", "labels": ["ready-for-agent"], "assignees": []}]}, open(sys.argv[1], "w"))' "$SNAP" "$now_epoch"
: > "$MOCK/gh-calls.log"
INTERVAL=86400 start_watch 1
check "forged fresh snapshot does not suppress refetch" has "$MOCK/gh-calls.log" "item-list"
check "forged snapshot replaced with real data" test "$(jqv '[i["number"] for i in d["items"]]' "$SNAP")" = "[810, 811, 812]"

# 6. 書き出し先に FIFO が置かれていても見張りは止まらず、通常ファイルに置き換わる
#    （共有の置き場に別の利用者が名前付きパイプを作っても、巡回がそこで待ち続けない）
rm -f "$SNAP"
mkfifo "$SNAP"
INTERVAL=0 start_watch 2
check "fifo does not stop watch" test -f "$SNAP" -a ! -p "$SNAP"
check "fifo replaced with valid snapshot" test "$(jqv 'd["version"]' "$SNAP")" = "1"

# 7. 打ち切られた取得（totalCount が件数を超える）は件数を上限に取り直し、全件で公開する
rm -f "$SNAP"
: > "$MOCK/gh-calls.log"
TRUNC=1 FIRST_LIMIT=3 INTERVAL=0 start_watch 2
check "truncated fetch refetches with total" test "$(jqv '[i["number"] for i in d["items"]]' "$SNAP")" = "[810, 811, 812, 813, 814]"
n_lim=$(grep -c -- "--limit 5" "$MOCK/gh-calls.log" || true)
check "refetch uses totalCount as limit" test "$n_lim" -ge 1

# 8. 形の違うカード（番号の無い Issue）を含む応答は公開せず、前の書き出しが残る
before="$(cat "$SNAP")"
: > "$ROOT/out.log"
BADITEM=1 INTERVAL=0 start_watch 2
check "bad item not published" test "$(cat "$SNAP")" = "$before"
check "bad item logged" has "$ROOT/out.log" "snapshot-failed"

printf '\n'
if ((${#failures[@]})); then
  printf 'failed: %s\n' "${failures[@]}" >&2
  exit 1
fi
printf 'all tests passed\n'
