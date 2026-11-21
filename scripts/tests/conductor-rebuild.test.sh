#!/usr/bin/env bash
# scripts/conductor-rebuild のテスト。偽の t3（T3_BIN）が受けた呼び出しを記録し、
# 新しい指揮役の起動（本体 checkout、worktree なし、待たせる文）→ 権限の確かめ → 旧指揮役の停止
# → 引き継ぎの指示 → 宛先の登録、の順と引数・本文を確かめる。本文は決まった文と完全一致で比べる。
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT
failures=()
LOG="$ROOT/t3-calls"
FILE="$ROOT/home/.config/pelway-agent-notify/conductor-thread-id"
OLD="mcp:launch:old-conductor"
NEW="mcp:launch:new-conductor"

# 期待する本文（スクリプトの決まった文と完全一致で比べる）
HOLD_WANT="恒久交代の新しい指揮役として立てた。権限の確かめと引き継ぎの指示が届くまで、何もせずにこのターンを終える。"
STOP_WANT="恒久交代を始める（docs/agents/conductor-context.md の「恒久交代」）。案件を進める操作（スレッドの起動・指示、マージ、ラベル、コメント、ステータス変更）を止め、引き継ぎの記録を指揮役の作業ログと patrol-state.json に書き出して、このターンを終える。以後は操作しない。scheduled task や起こしで起きても、巡回も patrol-state.json の書き込みもせずにターンを終える。"
START_WANT="あなたは新しい指揮役。skill command-team と docs/agents/conductor-context.md の「恒久交代」に従い、旧指揮役（スレッド $OLD）から引き継ぐ。引き継ぎ確認（GitHub の実状態、patrol-state、巡回記録、実行中スレッドの照合）を済ませて操作権限を受け取ったら、旧指揮役のスレッドを t3_thread_organize で settle し、巡回の scheduled task を作り直す（runbook の「指揮役の定期巡回（スケジュール起床）」）。確認が完了しない場合は権限を受け取らず、hatai に知らせる。"

# 偽の t3。呼び出しを 1 行ずつ記録し、標準入力の本文は宛先ごとのファイルに残す。
# 旧指揮役の runtimeMode は auto-accept-edits、新しい指揮役は $NEW_MODE（無ければ auto-accept-edits）。
cat > "$ROOT/t3" <<'T3'
#!/usr/bin/env bash
echo "$*" >> "$LOG"
case "$1 $2" in
  "thread show")
    mode=auto-accept-edits
    [[ "$3" == *new* ]] && mode="${NEW_MODE:-auto-accept-edits}"
    wt=""; [[ "$3" == *new* ]] && wt="${NEW_WT-/home/u/.t3/worktrees/Pelway/conductor}"
    echo '{"thread":{"threadId":"'"$3"'","providerInstanceId":"acpRegistry_devin","model":"swe-2-high","runtimeMode":"'"$mode"'","worktreePath":"'"$wt"'"}}' ;;
  "thread send")
    if [[ "$3" == --wait ]]; then id="$4"; else id="$3"; fi
    cat > "$LOG.send-stdin.${id##*:}"; [[ -e "$LOG.send-fail" ]] && exit 1; echo "ok" ;;
  "thread start") cat > "$LOG.start-stdin"; [[ -e "$LOG.start-fail" ]] && exit 1; echo "mcp:launch:new-conductor" ;;
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
    bash "$REPO/scripts/conductor-rebuild" "$@" >"$ROOT/out" 2>&1
}
check() {
  local name="$1"; shift
  if "$@"; then printf 'ok   %s\n' "$name"; else printf 'FAIL %s\n' "$name"; failures+=("$name"); fi
}
line() { sed -n "${1}p" "$LOG"; }
same() { [[ "$(cat "$1" 2>/dev/null)" == "$2" ]]; }
no_line() { ! grep -q -- "$1" "$LOG" 2>/dev/null; }

# 既定: 旧指揮役の provider・model を引き継ぎ、立てる → 確かめる → 止める → 指示する → 登録する
run "" --from "$OLD"; rc=$?
check "succeeds" [ "$rc" -eq 0 ]
check "1st call reads the old conductor" [ "$(line 1)" = "thread show $OLD --json" ]
check "2nd call starts the new conductor" grep -q "^thread start " <(line 2)
no_arg() { ! grep -q -- "$1" <(line 2); }
check "new conductor runs in its own worktree from main" grep -q -- "--worktree main --branch conductor/rebuild-" <(line 2)
check "new conductor inherits provider and model" grep -q -- "--provider acpRegistry_devin --model swe-2-high" <(line 2)
check "no effort option unless given" no_arg "--option"
check "missing --effort is reported" grep -q -- "--effort not given" "$ROOT/out"
check "first message is exactly the hold message" same "$LOG.start-stdin" "$HOLD_WANT"
check "3rd call reads the new conductor's mode" [ "$(line 3)" = "thread show $NEW --json" ]
check "4th call stops the old conductor with --wait" [ "$(line 4)" = "thread send --wait $OLD -" ]
check "stop message is exactly the fixed text" same "$LOG.send-stdin.old-conductor" "$STOP_WANT"
check "5th call sends the handover to the new conductor" [ "$(line 5)" = "thread send $NEW -" ]
check "handover message is exactly the fixed text" same "$LOG.send-stdin.new-conductor" "$START_WANT"
check "registers the new conductor" [ "$(cat "$FILE" 2>/dev/null)" = "$NEW" ]

# 引数で指定した値が渡る
run "" --from "$OLD" --provider codex --model gpt-6.2-sol --effort high
check "explicit provider/model/effort are passed" grep -q -- "--provider codex --model gpt-6.2-sol --option effort=high" <(line 2)

# 権限が合わなければ、旧指揮役に何も送らず、登録も変えずに止まる
for mode in full-access auto; do
  printf 'mcp:launch:kept\n' > "$FILE"
  NEW_MODE="$mode" run "" --from "$OLD"; rc=$?
  check "mode $mode: exits 3" [ "$rc" -eq 3 ]
  check "mode $mode: nothing sent to the old conductor" no_line "thread send --wait $OLD"
  check "mode $mode: no handover sent" no_line "thread send $NEW"
  check "mode $mode: registration unchanged" [ "$(cat "$FILE")" = "mcp:launch:kept" ]
  check "mode $mode: tells how to continue with --to" grep -q -- "--to '$NEW'" "$ROOT/out"
done

# --to で渡したスレッドを使う（立てない）
printf 'mcp:launch:kept\n' > "$FILE"
run "" --from "$OLD" --to "$NEW"; rc=$?
check "--to succeeds" [ "$rc" -eq 0 ]
check "--to does not start a thread" no_line "^thread start"
check "--to sends the handover" same "$LOG.send-stdin.new-conductor" "$START_WANT"
check "--to registers the given thread" [ "$(cat "$FILE")" = "$NEW" ]

# provider と model は組で指定する
run "" --from "$OLD" --provider codex; rc=$?
check "provider without model is rejected" [ "$rc" -eq 2 ]
check "provider without model makes no t3 calls" [ ! -s "$LOG" ]

# --to が --from と同じなら止まる
run "" --from "$OLD" --to "$OLD"; rc=$?
check "--to equal to --from is rejected" [ "$rc" -ne 0 ]
check "--to equal to --from makes no t3 calls" [ ! -s "$LOG" ]

# 新しい指揮役が本体 checkout で動いていたら（worktree でなければ）、旧指揮役に何も送らずに止まる
printf 'mcp:launch:kept\n' > "$FILE"
NEW_WT="" run "" --from "$OLD" --to "$NEW"; rc=$?
check "main-checkout new conductor is rejected" [ "$rc" -ne 0 ]
check "main-checkout new conductor: nothing sent to the old conductor" no_line "thread send --wait $OLD"
check "main-checkout new conductor: registration unchanged" [ "$(cat "$FILE")" = "mcp:launch:kept" ]

# 失敗したら登録を変えずに止まる
printf 'mcp:launch:kept\n' > "$FILE"
run send-fail --from "$OLD"; rc=$?
check "stop failure exits non-zero" [ "$rc" -ne 0 ]
check "stop failure keeps the registration" [ "$(cat "$FILE")" = "mcp:launch:kept" ]
run start-fail --from "$OLD"; rc=$?
check "start failure exits non-zero" [ "$rc" -ne 0 ]
check "start failure sends nothing to the old conductor" no_line "thread send --wait $OLD"
check "start failure says the conductor may exist" grep -q "may have been created" "$ROOT/out"
check "start failure keeps the registration" [ "$(cat "$FILE")" = "mcp:launch:kept" ]

# スレッド ID の検査
run "" --from "--help"; rc=$?
check "rejects an id starting with -" [ "$rc" -ne 0 ]
run "" --from 'a;rm'; rc=$?
check "rejects odd characters" [ "$rc" -ne 0 ]
check "rejected ids make no t3 calls" [ ! -s "$LOG" ]

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
run "" --from "$OLD"; rc=$?
check "missing pelway-agent-watch exits non-zero" [ "$rc" -ne 0 ]
check "missing pelway-agent-watch makes no t3 calls" [ ! -s "$LOG" ]
mv "$ROOT/watch.bak" "$ROOT/home/.local/bin/pelway-agent-watch"

# worktree からの起動は止まる（PELWAY_REBUILD_ALLOW_WORKTREE を付けない）。
# 試すのはコミット済みの版（git worktree add は HEAD を取り出す）
wt="$ROOT/wt"
if git -C "$REPO" worktree add -q --detach "$wt" HEAD 2>/dev/null; then
  rm -f "$LOG"
  HOME="$ROOT/home" T3_BIN="$ROOT/t3" LOG="$LOG" bash "$wt/scripts/conductor-rebuild" --from "$OLD" >"$ROOT/out" 2>&1; rc=$?
  git -C "$REPO" worktree remove --force "$wt"
  check "refuses to run from a worktree" [ "$rc" -eq 1 ]
  check "worktree refusal says why" grep -q "not from a worktree" "$ROOT/out"
  check "worktree refusal makes no t3 calls" [ ! -s "$LOG" ]
else
  echo "skip worktree refusal check: git worktree add failed here"
fi

if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}"
  exit 1
fi
printf '\nall passed\n'
