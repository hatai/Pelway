#!/usr/bin/env bash
# scripts/conductor-rebuild のテスト。偽の t3（T3_BIN）が受けた呼び出しを記録し、
# --to で渡した新しい指揮役の権限の確かめ → 旧指揮役の停止 → 引き継ぎの指示 → 宛先の登録、
# の順と引数・本文を確かめる。本文は決まった文と完全一致で比べる。
# 新しい指揮役はスクリプトでは立てない（--to は必須。t3 thread start には権限のフラグが無い）。
# --to なし・--provider/--model/--effort 付きは、偽の t3 を一度も呼ばずに止まることを確かめる。
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT
failures=()
LOG="$ROOT/t3-calls"
FILE="$ROOT/home/.config/pelway-agent-notify/conductor-thread-id"
OLD="mcp:launch:old-conductor"
NEW="mcp:launch:new-conductor"
NEW_WT_PATH="/home/u/.t3/worktrees/Pelway/conductor"

# 期待する本文（スクリプトの決まった文と完全一致で比べる）
STOP_WANT="恒久交代を始める（docs/agents/conductor-context.md の「恒久交代」）。案件を進める操作（スレッドの起動・指示、マージ、ラベル、コメント、ステータス変更）を止め、引き継ぎの記録を指揮役の作業ログと patrol-state.json に書き出して、このターンを終える。以後は操作しない。scheduled task や起こしで起きても、巡回も patrol-state.json の書き込みもせずにターンを終える。"
START_WANT="あなたは新しい指揮役。skill command-team と docs/agents/conductor-context.md の「恒久交代」に従い、旧指揮役（スレッド $OLD）から引き継ぐ。引き継ぎ確認（GitHub の実状態、patrol-state、巡回記録、実行中スレッドの照合）を済ませて操作権限を受け取ったら、旧指揮役のスレッドを t3_thread_organize で settle し、巡回の scheduled task を作り直し、patrol-state.json の conductor_thread_id を書き込み手順の --claim-conductor で自分のスレッド ID にする（runbook の「指揮役の定期巡回（スケジュール起床）」）。確認が完了しない場合は権限を受け取らず、hatai に知らせる。"

# 偽の t3。呼び出しを 1 行ずつ記録し、標準入力の本文は宛先ごとのファイルに残す。
# thread start は受け付けない（スクリプトは新しい指揮役を立てない。呼ぶと exit 9 で失敗し、記録に残る）。
# 旧指揮役の runtimeMode は auto-accept-edits、新しい指揮役は $NEW_MODE（無ければ auto-accept-edits）。
# NEW_NOMODE=1 のとき、新しい指揮役の thread show は runtimeMode を返さない。
cat > "$ROOT/t3" <<'T3'
#!/usr/bin/env bash
echo "$*" >> "$LOG"
case "$1 $2" in
  "thread show")
    mode=auto-accept-edits
    [[ "$3" == *new* ]] && mode="${NEW_MODE:-auto-accept-edits}"
    wt=""; [[ "$3" == *new* ]] && wt="${NEW_WT-/home/u/.t3/worktrees/Pelway/conductor}"
    mode_field=',"runtimeMode":"'"$mode"'"'
    [[ "$3" == *new* && -n "${NEW_NOMODE:-}" ]] && mode_field=""
    echo '{"thread":{"threadId":"'"$3"'","providerInstanceId":"'"${OLD_PROVIDER:-acpRegistry_devin}"'","model":"'"${OLD_MODEL:-swe-2-high}"'"'"$mode_field"',"worktreePath":"'"$wt"'"}}' ;;
  "thread send")
    if [[ "$3" == --wait ]]; then id="$4"; else id="$3"; fi
    cat > "$LOG.send-stdin.${id##*:}"; [[ -e "$LOG.send-fail" ]] && exit 1; echo "ok" ;;
  *) exit 9 ;;
esac
T3
chmod +x "$ROOT/t3"

# 偽の見張り役（インストールした pelway-agent-watch の代わり）。--register <id> で登録ファイルに書く
mkdir -p "$ROOT/home/.local/bin"
cat > "$ROOT/home/.local/bin/pelway-agent-watch" <<'W'
#!/usr/bin/env bash
[[ "$1" == --register ]] || exit 9
mkdir -p "$HOME/.config/pelway-agent-notify"
printf '%s\n' "$2" > "$HOME/.config/pelway-agent-notify/conductor-thread-id"
W
chmod +x "$ROOT/home/.local/bin/pelway-agent-watch"

# run <状態ファイル名|""> <引数...>。新しい指揮役の権限は環境変数 NEW_MODE で変える
run() {
  rm -f "$LOG" "$LOG".*; [[ -n "${1:-}" ]] && touch "$LOG.$1"
  shift || true
  HOME="$ROOT/home" T3_BIN="$ROOT/t3" LOG="$LOG" PELWAY_REBUILD_ALLOW_WORKTREE=1 \
    bash "$REPO/scripts/conductor-rebuild" "$@" >"$ROOT/out" 2>"$ROOT/err"
}
check() {
  local name="$1"; shift
  if "$@"; then printf 'ok   %s\n' "$name"; else printf 'FAIL %s\n' "$name"; failures+=("$name"); fi
}
line() { sed -n "${1}p" "$LOG"; }
same() { [[ "$(cat "$1" 2>/dev/null)" == "$2" ]]; }
no_line() { ! grep -q -- "$1" "$LOG" 2>/dev/null; }
all_err() { local f="$1"; shift; local p; for p in "$@"; do grep -qF -- "$p" "$f" || return 1; done; }

# --to 付き: 権限の確かめ → 旧指揮役の停止 → 引き継ぎ → 宛先の登録（新しい指揮役は立てない）
mkdir -p "$(dirname "$FILE")"
printf 'mcp:launch:kept\n' > "$FILE"
run "" --from "$OLD" --to "$NEW"; rc=$?
check "succeeds" [ "$rc" -eq 0 ]
check "1st call reads the old conductor" [ "$(line 1)" = "thread show $OLD --json" ]
check "2nd call reads the new conductor" [ "$(line 2)" = "thread show $NEW --json" ]
check "no thread start" no_line "thread start"
check "3rd call stops the old conductor with --wait" [ "$(line 3)" = "thread send --wait $OLD -" ]
check "stop message is exactly the fixed text" same "$LOG.send-stdin.old-conductor" "$STOP_WANT"
check "4th call sends the handover to the new conductor" [ "$(line 4)" = "thread send $NEW -" ]
check "handover message is exactly the fixed text" same "$LOG.send-stdin.new-conductor" "$START_WANT"
check "registers the new conductor" [ "$(cat "$FILE")" = "$NEW" ]
check "logs the new conductor's runtimeMode and worktree" \
  grep -qF "conductor-rebuild: new conductor $NEW runtimeMode=auto-accept-edits worktree=$NEW_WT_PATH" "$ROOT/err"

# --to なし: 何も作らず、t3_thread_launch で立てて --to で実行し直す手順を出して止まる
printf 'mcp:launch:kept\n' > "$FILE"
run "" --from "$OLD"; rc=$?
check "no --to: exits non-zero" [ "$rc" -ne 0 ]
check "no --to: makes no t3 calls" [ ! -s "$LOG" ]
check "no --to: shows how to launch and rerun" \
  all_err "$ROOT/err" "t3_thread_launch" "runtimeMode" "full-access" "worktree" "baseRef" "main" "--to" "'<新指揮役のスレッド ID>'"
check "no --to: registration unchanged" [ "$(cat "$FILE")" = "mcp:launch:kept" ]

# --provider・--model・--effort は受け付けない（立てる処理が無く、使い道が無いため）
for arg in "--provider codex --model gpt-6.2-sol" "--provider codex" "--model gpt-6.2-sol" "--effort high"; do
  run "" --from "$OLD" --to "$NEW" $arg; rc=$?
  check "rejects: $arg" [ "$rc" -ne 0 ]
  check "rejects $arg: shows usage" grep -qF "conductor-rebuild --from" "$ROOT/err"
  check "rejects $arg: makes no t3 calls" [ ! -s "$LOG" ]
done

# 権限が合わなければ、旧指揮役に何も送らず、登録も変えずに止まる
for mode in full-access auto; do
  printf 'mcp:launch:kept\n' > "$FILE"
  NEW_MODE="$mode" run "" --from "$OLD" --to "$NEW"; rc=$?
  check "mode $mode: exits 3" [ "$rc" -eq 3 ]
  check "mode $mode: nothing sent to the old conductor" no_line "thread send --wait $OLD"
  check "mode $mode: no handover sent" no_line "thread send $NEW"
  check "mode $mode: no thread start" no_line "thread start"
  check "mode $mode: registration unchanged" [ "$(cat "$FILE")" = "mcp:launch:kept" ]
  check "mode $mode: tells how to continue" all_err "$ROOT/err" "t3_thread_launch" "--to"
done

# 新しい指揮役の runtimeMode が読めない（show が値を返さない）ときも、何も送らず exit 3 で止まる
printf 'mcp:launch:kept\n' > "$FILE"
NEW_NOMODE=1 run "" --from "$OLD" --to "$NEW"; rc=$?
check "unreadable mode: exits 3" [ "$rc" -eq 3 ]
check "unreadable mode: nothing sent to the old conductor" no_line "thread send --wait $OLD"
check "unreadable mode: no handover sent" no_line "thread send $NEW"
check "unreadable mode: no thread start" no_line "thread start"
check "unreadable mode: registration unchanged" [ "$(cat "$FILE")" = "mcp:launch:kept" ]
check "unreadable mode: logs the read values" \
  grep -qF "conductor-rebuild: new conductor $NEW runtimeMode=- worktree=$NEW_WT_PATH" "$ROOT/err"

# --to が --from と同じなら止まる
run "" --from "$OLD" --to "$OLD"; rc=$?
check "--to equal to --from is rejected" [ "$rc" -ne 0 ]
check "--to equal to --from makes no t3 calls" [ ! -s "$LOG" ]

# 新しい指揮役が本体 checkout で動いていたら（worktree でなければ）、旧指揮役に何も送らずに止まる
printf 'mcp:launch:kept\n' > "$FILE"
NEW_WT="" run "" --from "$OLD" --to "$NEW"; rc=$?
check "main-checkout new conductor is rejected" [ "$rc" -ne 0 ]
check "main-checkout new conductor: nothing sent to the old conductor" no_line "thread send --wait $OLD"
check "main-checkout new conductor: no thread start" no_line "thread start"
check "main-checkout new conductor: registration unchanged" [ "$(cat "$FILE")" = "mcp:launch:kept" ]
check "main-checkout new conductor: logs worktree=-" \
  grep -qF "conductor-rebuild: new conductor $NEW runtimeMode=auto-accept-edits worktree=-" "$ROOT/err"
check "main-checkout new conductor: tells how to continue" \
  all_err "$ROOT/err" "t3_thread_launch" "--to"

# worktree が無く、かつ権限も合わないときは、権限の失敗（exit 3）で止まる
printf 'mcp:launch:kept\n' > "$FILE"
NEW_MODE=full-access NEW_WT="" run "" --from "$OLD" --to "$NEW"; rc=$?
check "no worktree + bad mode: exits 3" [ "$rc" -eq 3 ]
check "no worktree + bad mode: nothing sent to the old conductor" no_line "thread send --wait $OLD"
check "no worktree + bad mode: no handover sent" no_line "thread send $NEW"
check "no worktree + bad mode: no thread start" no_line "thread start"
check "no worktree + bad mode: registration unchanged" [ "$(cat "$FILE")" = "mcp:launch:kept" ]
check "no worktree + bad mode: logs both read values" \
  grep -qF "conductor-rebuild: new conductor $NEW runtimeMode=full-access worktree=-" "$ROOT/err"

# 失敗したら登録を変えずに止まる
printf 'mcp:launch:kept\n' > "$FILE"
run send-fail --from "$OLD" --to "$NEW"; rc=$?
check "stop failure exits non-zero" [ "$rc" -ne 0 ]
check "stop failure keeps the registration" [ "$(cat "$FILE")" = "mcp:launch:kept" ]
check "stop failure: no thread start" no_line "thread start"

# スレッド ID の検査
run "" --from "--help" --to "$NEW"; rc=$?
check "rejects an id starting with -" [ "$rc" -ne 0 ]
check "id starting with - makes no t3 calls" [ ! -s "$LOG" ]
run "" --from 'a;rm' --to "$NEW"; rc=$?
check "rejects odd characters" [ "$rc" -ne 0 ]
check "rejected ids make no t3 calls" [ ! -s "$LOG" ]
run "" --from "$OLD" --to "--bad"; rc=$?
check "rejects a bad --to id" [ "$rc" -ne 0 ]
check "bad --to id makes no t3 calls" [ ! -s "$LOG" ]

# mise のタスクから同じスクリプトを呼べる（引数は -- の後ろがそのまま渡る）
task_runs() { /usr/bin/python3 -I -c 'import sys, tomllib
t = tomllib.load(open(sys.argv[1], "rb"))["tasks"][sys.argv[2]]
sys.exit(0 if t.get("run") == sys.argv[3] else 1)' "$REPO/mise.toml" "$1" "$2"; }
check "mise task conductor-rebuild runs the script" task_runs conductor-rebuild "bash scripts/conductor-rebuild"
no_task() { /usr/bin/python3 -I -c 'import sys, tomllib
sys.exit(1 if sys.argv[2] in tomllib.load(open(sys.argv[1], "rb"))["tasks"] else 0)' "$REPO/mise.toml" "$1"; }
check "no mise task for the removed conductor-register" no_task conductor-register

# 見張り役が入っていなければ、何も送らずに止まる
mv "$ROOT/home/.local/bin/pelway-agent-watch" "$ROOT/watch.bak"
run "" --from "$OLD" --to "$NEW"; rc=$?
check "missing pelway-agent-watch exits non-zero" [ "$rc" -ne 0 ]
check "missing pelway-agent-watch makes no t3 calls" [ ! -s "$LOG" ]
mv "$ROOT/watch.bak" "$ROOT/home/.local/bin/pelway-agent-watch"

# worktree からの起動は止まる（PELWAY_REBUILD_ALLOW_WORKTREE を付けない）。
# 試すのはコミット済みの版（git worktree add は HEAD を取り出す）
wt="$ROOT/wt"
if git -C "$REPO" worktree add -q --detach "$wt" HEAD 2>/dev/null; then
  rm -f "$LOG"
  HOME="$ROOT/home" T3_BIN="$ROOT/t3" LOG="$LOG" bash "$wt/scripts/conductor-rebuild" --from "$OLD" --to "$NEW" >"$ROOT/out" 2>"$ROOT/err"; rc=$?
  git -C "$REPO" worktree remove --force "$wt"
  check "refuses to run from a worktree" [ "$rc" -eq 1 ]
  check "worktree refusal says why" grep -q "not from a worktree" "$ROOT/err"
  check "worktree refusal makes no t3 calls" [ ! -s "$LOG" ]
else
  echo "skip worktree refusal check: git worktree add failed here"
fi

if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}"
  exit 1
fi
printf '\nall passed\n'
