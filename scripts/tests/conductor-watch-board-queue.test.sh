#!/usr/bin/env bash
# scripts/conductor-watch.sh のボード Status 更新キュー（/tmp/pelway-board-queue 相当）のテスト。
# モック gh が gh project 系（view / field-list / item-list / item-edit）の応答を返し、
# 呼び出しを 1 行ずつ記録する。不正な要求で item-edit が呼ばれないこと、
# 要求の文字列がシェルに実行されないことを確かめる。
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ROOT="$(mktemp -d)"
WPID=""
trap '[[ -n "${WPID:-}" ]] && kill "$WPID" 2>/dev/null; rm -rf "$ROOT"' EXIT
failures=()

MOCK="$ROOT/mock"
QUEUE="$ROOT/queue"
mkdir -p "$ROOT/repo/scripts" "$ROOT/bin" "$MOCK" "$ROOT/state" "$QUEUE"

# 本物の conductor-watch.sh をコピーした偽リポジトリ（scripts/agent-gh をモックに差し替える）
cp "$REPO/scripts/conductor-watch.sh" "$ROOT/repo/scripts/"
cat > "$ROOT/repo/scripts/agent-gh" <<'EOF'
#!/usr/bin/env bash
# モック: 巡回の取得は常に空（キュー処理とは無関係）。
if [[ "$2 $3" == "issue list" || "$2 $3" == "pr list" ]]; then echo '[]'; exit 0; fi
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
# 巡回の周期を縮める。SLEEP_MAX 回目の呼び出しで親（conductor-watch.sh）を終了させる。
n=$(( $(cat "$MOCK/sleeps" 2>/dev/null || echo 0) + 1 ))
echo "$n" > "$MOCK/sleeps"
if [[ -n "${SLEEP_MAX:-}" ]] && (( n >= SLEEP_MAX )); then
  kill -TERM "$PPID"; exit 0
fi
exec /bin/sleep 0.03
EOF
chmod +x "$ROOT/bin/sleep"

cat > "$ROOT/bin/gh" <<'EOF'
#!/usr/bin/env bash
# モック gh: 呼び出しを gh-calls.log に 1 行ずつ記録し、project 系の固定応答を返す。
# MOCK_EDIT_FAIL があるとき item-edit は失敗する。
echo "$*" >> "$MOCK/gh-calls.log"
case "$1 $2" in
  "project view") cat "$MOCK/project-view.json"; exit 0 ;;
  "project field-list") cat "$MOCK/field-list.json"; exit 0 ;;
  "project item-list") cat "$MOCK/item-list.json"; exit 0 ;;
  "project item-edit")
    if [[ -n "${MOCK_EDIT_FAIL:-}" ]]; then echo "boom" >&2; exit 1; fi
    exit 0 ;;
esac
echo "unexpected gh call: $*" >&2
exit 9
EOF
chmod +x "$ROOT/bin/gh"

cat > "$MOCK/project-view.json" <<'EOF'
{"id": "PVT_proj7", "number": 7, "title": "Pelway agent team"}
EOF

python3 -I - > "$MOCK/field-list.json" <<'EOF'
import json
names = ["Backlog", "Todo", "Working", "In review", "Changes requested",
         "Ready to merge", "Needs human", "Done"]
opts = [{"id": f"opt-{i}", "name": n} for i, n in enumerate(names, 1)]
print(json.dumps({"fields": [{"id": "PVTSSF_status", "name": "Status",
                  "type": "ProjectV2SingleSelectField", "options": opts}]}))
EOF

cat > "$MOCK/item-list.json" <<'EOF'
{"items": [
  {"id": "PVTI_i352", "content": {"type": "Issue", "number": 352, "repository": "hatai/Pelway"}},
  {"id": "PVTI_i353", "content": {"type": "Issue", "number": 353, "repository": "hatai/Pelway"}},
  {"id": "PVTI_p100", "content": {"type": "PullRequest", "number": 100, "repository": "hatai/Pelway"}}
]}
EOF

start_watch() {
  MOCK="$MOCK" SLEEP_MAX="${SLEEP_MAX:-}" MOCK_EDIT_FAIL="${MOCK_EDIT_FAIL:-}" \
  PELWAY_BOARD_QUEUE_DIR="$QUEUE" XDG_STATE_HOME="$ROOT/state" PATH="$ROOT/bin:$PATH" \
  PELWAY_PATROL_STATE="$ROOT/state/pelway/patrol-state.json" \
    bash "$ROOT/repo/scripts/conductor-watch.sh" >>"$ROOT/out.log" 2>&1 &
  WPID=$!
}

# 巡回 n 回（n 回目のキュー処理まで完了）を待って終了する
run_cycles() {
  echo 0 > "$MOCK/sleeps"
  SLEEP_MAX="$1" start_watch
  wait "$WPID" 2>/dev/null; WPID=""
}

# 要求ファイルを一時ファイル経由で置く（書き込み途中を拾わせない置き方）
req() { printf '%s\n' "$2" > "$QUEUE/$1.tmp" && mv "$QUEUE/$1.tmp" "$QUEUE/$1"; }

check() {
  local name="$1"; shift
  if "$@"; then printf 'ok   %s\n' "$name"; else printf 'FAIL %s\n' "$name"; failures+=("$name"); fi
}

has() { grep -qF "$2" "$1"; }
nhas() { ! grep -qF "$2" "$1"; }
eq() { [[ "$1" == "$2" ]]; }

# 1. Issue 番号で指定 → item が解決されて item-edit が呼ばれ .done が残る
: > "$MOCK/gh-calls.log"
req 352-in-review.json '{"issue": 352, "status": "In review"}'
run_cycles 1
check "issue request produces .done" test -f "$QUEUE/352-in-review.done"
check "issue request file consumed" test ! -e "$QUEUE/352-in-review.json"
check "done file carries result json" has "$QUEUE/352-in-review.done" '"request": "352-in-review.json"'
check "item-edit called with resolved ids" \
  has "$MOCK/gh-calls.log" "item-edit --project-id PVT_proj7 --id PVTI_i352 --field-id PVTSSF_status --single-select-option-id opt-4"
check "QUEUE line on stdout" has "$ROOT/out.log" "QUEUE 352-in-review.json done"

# 2. item id 指定 → item-list を引かずにそのまま item-edit
: > "$MOCK/gh-calls.log"
req direct-item.json '{"item_id": "PVTI_direct", "status": "Done"}'
run_cycles 1
check "item_id request produces .done" test -f "$QUEUE/direct-item.done"
check "item-edit uses given item id" \
  has "$MOCK/gh-calls.log" "item-edit --project-id PVT_proj7 --id PVTI_direct --field-id PVTSSF_status --single-select-option-id opt-8"
check "item-list not fetched for item_id target" nhas "$MOCK/gh-calls.log" "item-list"

# 3. 構造・選択肢・対象の検証で落ちる要求は gh をまったく呼ばず .error。
# シェルに実行させようとする文字列も実行されない
: > "$MOCK/gh-calls.log"
req bad-status.json '{"issue": 352, "status": "Hacked"}'
req inject-status.json "{\"issue\": 352, \"status\": \"Working; touch $ROOT/pwned\"}"
req inject-id.json "{\"item_id\": \"x;touch $ROOT/pwned\", \"status\": \"Working\"}"
req both-targets.json '{"issue": 352, "item_id": "PVTI_i352", "status": "Working"}'
req extra-key.json '{"issue": 352, "status": "Working", "run": "touch /tmp/x"}'
req null-issue.json '{"issue": null, "status": "Working"}'
req null-itemid.json '{"item_id": null, "status": "Working"}'
req null-both.json '{"issue": null, "item_id": null, "status": "Working"}'
req empty-itemid.json '{"item_id": "", "status": "Working"}'
printf 'not json at all' > "$QUEUE/malformed.json"
run_cycles 1
for f in bad-status inject-status inject-id both-targets extra-key null-issue null-itemid null-both empty-itemid malformed; do
  check "$f produces .error" test -f "$QUEUE/$f.error"
  check "$f request file consumed" test ! -e "$QUEUE/$f.json"
done
check "invalid requests never call gh" test ! -s "$MOCK/gh-calls.log"
check "injection string not executed" test ! -e "$ROOT/pwned"

# 4. 検証を通るが解決・適用できない要求は item-edit なしで .error。
# （Issue がボードに無い / PR の番号 / プロジェクトの選択肢に無い Status）
: > "$MOCK/gh-calls.log"
req no-item.json '{"issue": 999, "status": "Working"}'
req pr-number.json '{"issue": 100, "status": "Working"}'
req no-option.json '{"issue": 352, "status": "Done"}'
# 「Done」だけ選択肢から消した field-list に差し替える
python3 -I - > "$MOCK/field-list.json" <<'EOF'
import json
names = ["Backlog", "Todo", "Working", "In review", "Changes requested",
         "Ready to merge", "Needs human"]
opts = [{"id": f"opt-{i}", "name": n} for i, n in enumerate(names, 1)]
print(json.dumps({"fields": [{"id": "PVTSSF_status", "name": "Status",
                  "type": "ProjectV2SingleSelectField", "options": opts}]}))
EOF
run_cycles 1
for f in no-item pr-number no-option; do
  check "$f produces .error" test -f "$QUEUE/$f.error"
done
check "unresolvable requests never call item-edit" nhas "$MOCK/gh-calls.log" "item-edit"
check "gh reads still happened" has "$MOCK/gh-calls.log" "project item-list"

# 5. item-edit 自体の失敗は .error（呼び出しは記録に残る）
python3 -I - > "$MOCK/field-list.json" <<'EOF'
import json
names = ["Backlog", "Todo", "Working", "In review", "Changes requested",
         "Ready to merge", "Needs human", "Done"]
opts = [{"id": f"opt-{i}", "name": n} for i, n in enumerate(names, 1)]
print(json.dumps({"fields": [{"id": "PVTSSF_status", "name": "Status",
                  "type": "ProjectV2SingleSelectField", "options": opts}]}))
EOF
req edit-fail.json '{"item_id": "PVTI_i352", "status": "Todo"}'
: > "$MOCK/gh-calls.log"
MOCK_EDIT_FAIL=1 run_cycles 1
check "item-edit failure produces .error" test -f "$QUEUE/edit-fail.error"
check "item-edit was attempted" has "$MOCK/gh-calls.log" "item-edit"

# 6. 書き込み途中の .tmp は拾わない。空巡回は gh を呼ばない
req after-tmp.json '{"issue": 353, "status": "Todo"}'
printf '{"issue": 353' > "$QUEUE/half-written.json.tmp"
: > "$MOCK/gh-calls.log"
run_cycles 1
check "half-written tmp file untouched" test -f "$QUEUE/half-written.json.tmp"
check "no .error for tmp file" test ! -e "$QUEUE/half-written.error"
check "after-tmp processed" test -f "$QUEUE/after-tmp.done"

# 7. <名前>.error.tmp / .done.tmp への symlink で外のファイルを上書きさせない。
# 一時ファイルは予測不能な名前で O_EXCL|O_NOFOLLOW に作られる
printf 'SENTINEL-keep-me\n' > "$ROOT/victim"
req esc-done.json '{"item_id": "PVTI_i352", "status": "Todo"}'
req esc-err.json '{"issue": 352, "status": "Hacked"}'
ln -s "$ROOT/victim" "$QUEUE/esc-done.done.tmp"
ln -s "$ROOT/victim" "$QUEUE/esc-err.error.tmp"
run_cycles 1
check "victim file not overwritten via done.tmp" eq "$(cat "$ROOT/victim")" "SENTINEL-keep-me"
check "done.tmp symlink not consumed" test -L "$QUEUE/esc-done.done.tmp"
check "err.tmp symlink not consumed" test -L "$QUEUE/esc-err.error.tmp"
check "esc-done still processed" test -f "$QUEUE/esc-done.done"
check "esc-err still processed" test -f "$QUEUE/esc-err.error"

# 8. キューディレクトリが symlink に差し替えられたら、その巡回では処理しない
# （差し替え先の *.json を読み・消さない）
rm -rf "$QUEUE.out" && mkdir -p "$QUEUE.out"
printf '%s\n' '{"issue": 352, "status": "Hacked"}' > "$QUEUE.out/keep-safe.json"
mv "$QUEUE" "$QUEUE.real" && ln -s "$QUEUE.out" "$QUEUE"
run_cycles 1
check "swapped-out request file still there" test -f "$QUEUE.out/keep-safe.json"
check "no .error written into swapped dir" test ! -e "$QUEUE.out/keep-safe.error"
check "queue-dir invalid line" has "$ROOT/out.log" "QUEUE queue-dir invalid"
rm "$QUEUE" && mv "$QUEUE.real" "$QUEUE"

# 9. 通常ファイルでない要求（FIFO）は巡回を止めず .error に回す
mkfifo "$QUEUE/fifo.json"
req fifo-guard.json '{"issue": 352, "status": "Hacked"}'
run_cycles 1
check "fifo request produces .error" test -f "$QUEUE/fifo.error"
check "fifo file unlinked" test ! -e "$QUEUE/fifo.json"
check "request after fifo also processed" test -f "$QUEUE/fifo-guard.error"

# 10. 大きすぎる要求ファイル（上限超過）は gh を呼ばず .error
python3 -I - > "$QUEUE/oversize.json.tmp" <<'EOF'
print('{"issue": 352, "status": "Working", "pad": "' + "x" * 20000 + '"}')
EOF
mv "$QUEUE/oversize.json.tmp" "$QUEUE/oversize.json"
: > "$MOCK/gh-calls.log"
run_cycles 1
check "oversize request produces .error" test -f "$QUEUE/oversize.error"
check "oversize request never calls gh" test ! -s "$MOCK/gh-calls.log"

if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}"
  exit 1
fi
printf '\nall passed\n'
