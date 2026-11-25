#!/usr/bin/env bash
# scripts/conductor-watch.sh の patrol-state.json の読み書き（Issue #699）のテスト。
# 補助記録は sandbox の中からも書ける共有 git dir にあるため、巡回は symlink をたどらず、
# FIFO で止まらず、通常ファイルで 4 MiB 以下のときだけ読み（#680 の read_ps）、書き戻しは
# 同じディレクトリの一時ファイルからの置き換えで行う（symlink の先や FIFO に書かない）。
# 旧置き場（$XDG_STATE_HOME/pelway/patrol-state.json）からの移行も同じ検査で読み、
# 新置き場に何も無いときだけ作る。止まりの検知のテスト（conductor-watch-stall.test.sh）と同じく、
# 巡回は setsid で動かし、10 秒で終わらなければ止めて「時間内に終わらない」とする。
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ROOT="$(mktemp -d)"
WPID=""
# 巡回は setsid で自分のプロセスグループに入れて動かし、止めるときは python の子ごと止める
trap '[[ -n "${WPID:-}" ]] && kill -KILL -- -"$WPID" 2>/dev/null; rm -rf "$ROOT"' EXIT
failures=()

MOCK="$ROOT/mock"
mkdir -p "$ROOT/repo/scripts" "$ROOT/bin" "$MOCK" "$ROOT/state/pelway"
cp "$REPO/scripts/conductor-watch.sh" "$ROOT/repo/scripts/"
git init -q "$ROOT/repo"
PELWAY="$ROOT/repo/.git/pelway"
STATE="$PELWAY/patrol-state.json"
OLD_STATE="$ROOT/state/pelway/patrol-state.json"

# モックの agent-gh: issue list / pr list だけに答える
cat > "$ROOT/repo/scripts/agent-gh" <<'EOF'
#!/usr/bin/env bash
[[ "$2 $3" == "issue list" ]] && { cat "$MOCK/issues.json"; exit 0; }
[[ "$2 $3" == "pr list" ]] && { echo '[]'; exit 0; }
[[ "$2 $3" == "api /repos/hatai/Pelway/issues/events?per_page=100" ]] && { echo '[]'; exit 0; }
exit 9
EOF
chmod +x "$ROOT/repo/scripts/agent-gh"
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

# Issue 61 は agent:working で PR が無い → waiting_on=implement。記録が読まれれば、待ち始めた時刻
# （SEEDED_SINCE）と指揮役の項目（scheduled_task_id）が引き継がれ、読まれなければ新しくなる。
printf '%s\n' '[{"number":61,"state":"OPEN","labels":[{"name":"agent:working"}]}]' > "$MOCK/issues.json"
SEEDED_SINCE="2020-01-01T00:00:00+09:00"
# seed <path> [埋め草のバイト数] — 引き継がれたかを見分けられる記録を書く（埋め草で大きさを変える）
seed() {
  python3 -I - "$1" "${2:-0}" "$SEEDED_SINCE" <<'EOF'
import json, sys
p, pad, since = sys.argv[1], int(sys.argv[2]), sys.argv[3]
d = {"subjects": {"61": {"waiting_on": "implement", "waiting_since": since,
                         "next_owner": "implement thread (#61)", "next_check_at": "2099-01-01T00:00:00+09:00",
                         "updated_at": since, "labels": ["agent:working"]}},
     "scheduled_task_id": "seeded"}
if pad: d["pad"] = "x" * pad
json.dump(d, open(p, "w"), ensure_ascii=False, indent=2, sort_keys=True)
EOF
}

reset() {
  rm -rf "$PELWAY" "$ROOT/state/pelway" "$ROOT/target" "$ROOT/old-target"
  mkdir -p "$PELWAY" "$ROOT/state/pelway"; : > "$ROOT/out.log"
}

# run_cycles <回数> — 巡回を回す。10 秒以内に終わらなければ、プロセスグループごと止めて
# RUN_RC=1 にする（終わったら 0）。
run_cycles() {
  local i
  echo 0 > "$MOCK/sleeps"
  MOCK="$MOCK" SLEEP_MAX="$1" XDG_STATE_HOME="$ROOT/state" PATH="$ROOT/bin:$PATH" \
    T3_BIN="$ROOT/bin/t3" PELWAY_BOARD_QUEUE_DIR="$ROOT/queue" \
    setsid bash "$ROOT/repo/scripts/conductor-watch.sh" >>"$ROOT/out.log" 2>&1 &
  WPID=$!
  for ((i = 0; i < 100; i++)); do kill -0 "$WPID" 2>/dev/null || break; /bin/sleep 0.1; done
  RUN_RC=0
  if kill -0 "$WPID" 2>/dev/null; then kill -KILL -- -"$WPID" 2>/dev/null; RUN_RC=1; fi
  wait "$WPID" 2>/dev/null; WPID=""
}

# 記録が通常ファイル（symlink でない）のときだけ開く（FIFO のまま残ったときにテスト自身が止まらない）
regular() { [[ -f "$STATE" && ! -L "$STATE" ]]; }
since61() {
  regular || { echo not-regular; return; }
  python3 -I -c 'import json,sys
try: print(json.load(open(sys.argv[1]))["subjects"]["61"]["waiting_since"])
except Exception: print("")' "$STATE"
}
task_id() {
  regular || { echo not-regular; return; }
  python3 -I -c 'import json,sys
try: print(json.load(open(sys.argv[1])).get("scheduled_task_id", ""))
except Exception: print("")' "$STATE"
}

check() {
  local name="$1"; shift
  if "$@"; then printf 'ok   %s\n' "$name"; else printf 'FAIL %s\n' "$name"; failures+=("$name"); fi
}
eq() { [[ "$1" == "$2" ]]; }
has() { grep -qF -- "$2" "$1"; }
in_time() { [[ "$RUN_RC" == 0 ]]; }
absent() { [[ ! -e "$1" && ! -L "$1" ]]; }
written() { regular && [[ "$(since61)" == 20* ]]; }  # 巡回が通常ファイルとして書き戻した
# 巡回が通常ファイルとして書き戻し、待ち始めた時刻が引き継がれていない（記録が読まれなかった）
fresh() { written && [[ "$(since61)" != "$SEEDED_SINCE" ]]; }

# 試験 0（対照）: 通常ファイルの記録は読まれ、待ち始めた時刻と指揮役の項目が引き継がれる。
reset; seed "$STATE"
run_cycles 1
check "0: a regular patrol-state carries waiting_since over" eq "$(since61)" "$SEEDED_SINCE"
check "0: a regular patrol-state carries the conductor's items over" eq "$(task_id)" "seeded"

# 試験 1: 記録が symlink なら、指す先を読まず、指す先を書き換えない。記録は通常ファイルに置き換わる。
reset; seed "$ROOT/target"; cp "$ROOT/target" "$ROOT/target.orig"
ln -s "$ROOT/target" "$STATE"
run_cycles 1
check "1: the cycle finishes within 10 seconds" in_time
check "1: a symlinked patrol-state is not read" fresh
check "1: the symlink target is left byte-identical" cmp -s "$ROOT/target" "$ROOT/target.orig"
check "1: the patrol-state is written back as a regular file" written

# 試験 2: 記録が指す先の無い symlink なら、指す先にファイルを作らない。
reset; ln -s "$ROOT/target" "$STATE"
run_cycles 1
check "2: the cycle finishes within 10 seconds" in_time
check "2: no file is created at the dangling symlink target" absent "$ROOT/target"
check "2: the patrol-state is written back as a regular file" written

# 試験 3: 記録が FIFO なら、開いて止まらず、10 秒以内にその回を終え、通常ファイルに置き換える。
reset; mkfifo "$STATE"
run_cycles 1
check "3: a FIFO patrol-state does not block the cycle (10 seconds)" in_time
check "3: the patrol-state is written back as a regular file" written

# 試験 4: 記録が読み取り上限（4 MiB）を超えるなら読まない（正しい JSON でも引き継がない）。
reset; seed "$STATE" $((5 << 20))
run_cycles 1
check "4: the cycle finishes within 10 seconds" in_time
check "4: an oversized patrol-state is not read" fresh
check "4: the oversized patrol-state is replaced by a fresh record" eq "$(task_id)" ""
check "4: the patrol-state is written back as a regular file" written

# 試験 5: 旧置き場があり、新置き場が FIFO なら、移行で止まらない（新置き場に何かあれば移行しない）。
reset; seed "$OLD_STATE"; mkfifo "$STATE"
run_cycles 1
check "5: migration onto a FIFO does not block (10 seconds)" in_time
check "5: the patrol-state is written back as a regular file" written
check "5: nothing is migrated onto an existing FIFO" eq "$(task_id)" ""

# 試験 6: 旧置き場があり、新置き場が指す先の無い symlink なら、移行で指す先にファイルを作らない。
reset; seed "$OLD_STATE"; ln -s "$ROOT/target" "$STATE"
run_cycles 1
check "6: the cycle finishes within 10 seconds" in_time
check "6: migration creates no file at the dangling symlink target" absent "$ROOT/target"
check "6: the patrol-state is written back as a regular file" written

# 試験 7: 旧置き場が symlink なら、指す先を読まない（移行しない）。
reset; seed "$ROOT/old-target"; ln -s "$ROOT/old-target" "$OLD_STATE"
run_cycles 1
check "7: the cycle finishes within 10 seconds" in_time
check "7: a symlinked old patrol-state is not migrated" eq "$(task_id)" ""
check "7: the skipped migration is logged" has "$ROOT/out.log" "PATROL-STATE not-migrated $OLD_STATE reason=symlink"
check "7: the patrol-state is written back as a regular file" written

# 試験 8: 旧置き場が読み取り上限（4 MiB）を超えるなら移行しない。
reset; seed "$OLD_STATE" $((5 << 20))
run_cycles 1
check "8: the cycle finishes within 10 seconds" in_time
check "8: an oversized old patrol-state is not migrated" eq "$(task_id)" ""
check "8: the skipped migration is logged" has "$ROOT/out.log" "PATROL-STATE not-migrated $OLD_STATE reason=too-large"
check "8: the patrol-state is written back as a regular file" written

# 試験 9: 旧置き場が FIFO なら、移行で止まらない（移行の読み取りも待たずに開く）。
reset; mkfifo "$OLD_STATE"
run_cycles 1
check "9: an old FIFO patrol-state does not block the migration (10 seconds)" in_time
check "9: the patrol-state is written back as a regular file" written

# 試験 10: 書き戻しの一時ファイルは、成功しても置き換えに失敗しても（置き場がディレクトリ）残らない。
reset; seed "$STATE"
run_cycles 2
check "10: no temporary file is left after a write-back" \
  eq "$(find "$PELWAY" -mindepth 1 -maxdepth 1 ! -name patrol-state.json | wc -l)" "0"
reset; mkdir "$STATE"
run_cycles 1
check "10: the cycle finishes within 10 seconds" in_time
check "10: no temporary file is left after a failed replace" \
  eq "$(find "$PELWAY" -mindepth 1 -maxdepth 1 ! -name patrol-state.json | wc -l)" "0"

if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}"
  exit 1
fi
printf '\nall passed\n'
