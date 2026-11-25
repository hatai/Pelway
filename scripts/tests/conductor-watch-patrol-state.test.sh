#!/usr/bin/env bash
# scripts/conductor-watch.sh の patrol-state.json の読み書き（Issue #699）のテスト。
# 補助記録は sandbox の中からも書ける共有 git dir にあるため、巡回は symlink をたどらず、
# FIFO で止まらず、通常ファイルで 4 MiB 以下のときだけ読み（#680 の read_ps）、書き戻しは
# 同じディレクトリの一時ファイルからの置き換えで行う（symlink の先や FIFO に書かない）。辞書でない記録は空として扱う。
# 旧置き場（$XDG_STATE_HOME/pelway/patrol-state.json）からの移行も同じ検査で読み（通常ファイル以外は
# 開かずに飛ばす）、新置き場に何も無いときだけ作る。記録の親ディレクトリ（共有 git dir の pelway/）は
# symlink をたどらずに開いて確かめ、確かめられない回は読まず書かない。移行と書き戻しは記録を 0600 で作り、
# 書き戻しが上限を超える回は置き換えずに読めていた記録を保つ。止まりの検知のテスト（conductor-watch-stall.test.sh）と同じく、
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
[[ "$2 $3" == "issue list" ]] && { [[ -e "$MOCK/issues-fail" ]] && exit 1; cat "$MOCK/issues.json"; exit 0; }
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
# seed <path> [埋め草のバイト数 | near] — 引き継がれたかを見分けられる記録を書く（埋め草で大きさを変える）。
# near は、読み取り上限（4 MiB）の 64 バイト手前まで埋めた詰めた JSON で、巡回の書き戻し（字下げと
# トップレベルの updated_at）では上限を超える大きさになる。
seed() {
  python3 -I - "$1" "${2:-0}" "$SEEDED_SINCE" <<'EOF'
import json, sys
p, pad, since = sys.argv[1], sys.argv[2], sys.argv[3]
d = {"subjects": {"61": {"waiting_on": "implement", "waiting_since": since,
                         "next_owner": "implement thread (#61)", "next_check_at": "2099-01-01T00:00:00+09:00",
                         "updated_at": since, "labels": ["agent:working"]}},
     "scheduled_task_id": "seeded"}
if pad == "near":
    compact = lambda: json.dumps(d, ensure_ascii=False, separators=(",", ":"), sort_keys=True)
    d["pad"] = ""
    d["pad"] = "x" * (4194304 - 64 - len(compact().encode("utf-8")))
    open(p, "w", encoding="utf-8").write(compact())
else:
    if int(pad): d["pad"] = "x" * int(pad)
    json.dump(d, open(p, "w"), ensure_ascii=False, indent=2, sort_keys=True)
EOF
}

UMASK0="$(umask)"
reset() {
  umask "$UMASK0"
  rm -rf "$PELWAY" "$ROOT/state/pelway" "$ROOT/target" "$ROOT/old-target" "$ROOT/outside" "$MOCK/issues-fail"
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
# top <key> — 記録のトップレベルの項目の値（無ければ空）
top() {
  regular || { echo not-regular; return; }
  python3 -I -c 'import json,sys
try: print(json.load(open(sys.argv[1])).get(sys.argv[2], ""))
except Exception: print("")' "$STATE" "$1"
}
task_id() { top scheduled_task_id; }
mode() { stat -c %a "$1" 2>/dev/null; }

check() {
  local name="$1"; shift
  if "$@"; then printf 'ok   %s\n' "$name"; else printf 'FAIL %s\n' "$name"; failures+=("$name"); fi
}
eq() { [[ "$1" == "$2" ]]; }
has() { grep -qF -- "$2" "$1"; }
lacks() { ! grep -qF -- "$2" "$1"; }
stamped() { [[ "$(top updated_at)" == 20* ]]; }  # 巡回がこの記録を書き戻した（トップレベルの updated_at）
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

# 試験 11: 記録が JSON として読めても辞書でない（[]）なら、空の記録として扱い、巡回を止めずに書き戻す。
reset; printf '[]\n' > "$STATE"
run_cycles 1
check "11: the cycle finishes within 10 seconds" in_time
check "11: a non-object patrol-state is replaced by a fresh record" written

# 試験 12: 旧置き場が通常ファイルでない（Unix ドメインソケット。開くと ENXIO になる種類）なら、
# 移行せずに理由を残し、見張りは止まらずに巡回を始める。bind はパスの長さの上限を避けて相対パスで行う。
reset
(cd "$ROOT/state/pelway" && python3 -I -c 'import socket; socket.socket(socket.AF_UNIX).bind("patrol-state.json")')
run_cycles 1
check "12: a socket is placed at the old place" test -S "$OLD_STATE"
check "12: the cycle finishes within 10 seconds" in_time
check "12: a socket at the old place does not stop the watcher" written
check "12: the skipped migration is logged" has "$ROOT/out.log" "PATROL-STATE not-migrated $OLD_STATE reason=not-regular"

# 試験 13: umask 022 でも、移行・書き戻し（一時ファイルからの置き換え）・新しい記録の作成は、記録の
# 許可ビットを 0600 より広げない。移行だけを見る回は、Issue の一覧の取得を失敗させて書き戻しを止める。
reset; seed "$OLD_STATE"; chmod 600 "$OLD_STATE"; : > "$MOCK/issues-fail"
umask 022; run_cycles 1; umask "$UMASK0"
check "13: the 0600 old record is migrated" eq "$(task_id)" "seeded"
check "13: the patrol does not write back while the issue list fails" eq "$(top updated_at)" ""
check "13: migration keeps the record at 0600 under umask 022" eq "$(mode "$STATE")" "600"
reset; seed "$STATE"; chmod 600 "$STATE"
umask 022; run_cycles 2; umask "$UMASK0"
check "13: the 0600 record is written back" stamped
check "13: the written-back record carries the conductor's items over" eq "$(task_id)" "seeded"
check "13: write-back keeps the record at 0600 under umask 022" eq "$(mode "$STATE")" "600"
reset
umask 022; run_cycles 1; umask "$UMASK0"
check "13: a new record is written" written
check "13: a new record is created at 0600 under umask 022" eq "$(mode "$STATE")" "600"

# 試験 14: 記録の親ディレクトリ（共有 git dir の pelway/）が symlink なら、その指す先の記録を読まず、
# 指す先に書かない（一時ファイルも移行先も作らない）。その回は書き戻さず、理由の行を出す。
# 指す先の記録は 30 分より前から未配送の指示を持つので、読まれれば pending-sends-stalled の行が出る。
reset; rm -rf "$PELWAY"; mkdir -p "$ROOT/outside"
printf '%s\n' '{"scheduled_task_id":"outside","pending_thread_sends":[{"to":"mcp:x","what":"w","since":"2020-01-01T00:00:00+09:00"}]}' \
  > "$ROOT/outside/patrol-state.json"
cp "$ROOT/outside/patrol-state.json" "$ROOT/outside.orig"
ln -s "$ROOT/outside" "$PELWAY"
run_cycles 1
check "14: the cycle finishes within 10 seconds" in_time
check "14: the record behind a symlinked directory is not read" lacks "$ROOT/out.log" "pending-sends-stalled"
check "14: the record behind a symlinked directory is left byte-identical" \
  cmp -s "$ROOT/outside/patrol-state.json" "$ROOT/outside.orig"
check "14: no file is created behind a symlinked directory" eq "$(find "$ROOT/outside" -mindepth 1 | wc -l)" "1"
check "14: the skipped write-back is logged with its reason" has "$ROOT/out.log" "reason=dir-invalid"
reset; rm -rf "$PELWAY"; mkdir -p "$ROOT/outside"; ln -s "$ROOT/outside" "$PELWAY"; seed "$OLD_STATE"
run_cycles 1
check "14: the cycle finishes within 10 seconds (migration)" in_time
check "14: migration creates nothing behind a symlinked directory" eq "$(find "$ROOT/outside" -mindepth 1 | wc -l)" "0"
check "14: the invalid directory is logged at startup" has "$ROOT/out.log" "PATROL-STATE dir-invalid $PELWAY"

# 試験 15: 上限（4 MiB）付近の有効な記録で、書き戻すと上限を超えるなら、置き換えずに読めていた記録を保つ。
# 3 周回しても記録はそのまま引き継がれ（指揮役の項目と待ち始めた時刻が残る）、理由の行は 1 回だけ出る。
# 記録が上限に収まる大きさに戻れば、書き戻しが戻ったことを示す行が出る。
reset; seed "$STATE" near; cp "$STATE" "$ROOT/near.orig"
check "15: the near-limit record is within the read limit" test "$(stat -c %s "$STATE")" -le 4194304
run_cycles 3
check "15: the cycles finish within 10 seconds" in_time
check "15: the near-limit record is kept byte-identical" cmp -s "$STATE" "$ROOT/near.orig"
check "15: the conductor's items survive three cycles" eq "$(task_id)" "seeded"
check "15: waiting_since survives three cycles" eq "$(since61)" "$SEEDED_SINCE"
check "15: the skipped write-back is logged once" \
  eq "$(grep -c 'WATCHER patrol-state-write-skipped .*reason=too-large' "$ROOT/out.log")" "1"
check "15: no temporary file is left" \
  eq "$(find "$PELWAY" -mindepth 1 -maxdepth 1 ! -name patrol-state.json | wc -l)" "0"
seed "$STATE"
run_cycles 1
check "15: the write-back resumes once the record fits" has "$ROOT/out.log" "WATCHER patrol-state-write-recovered"
check "15: the resumed write-back is written" stamped
check "15: the resumed write-back keeps the conductor's items" eq "$(task_id)" "seeded"

if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}"
  exit 1
fi
printf '\nall passed\n'
