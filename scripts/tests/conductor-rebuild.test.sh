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
START_WANT="あなたは新しい指揮役。skill command-team と docs/agents/conductor-context.md の「恒久交代」に従い、旧指揮役（スレッド $OLD）から引き継ぐ。引き継ぎ確認（GitHub の実状態、patrol-state、巡回記録、実行中スレッドの照合）を済ませて操作権限を受け取ったら、旧指揮役のスレッドを t3_thread_organize で settle し、巡回の scheduled task を作り直す（runbook の「指揮役の定期巡回（スケジュール起床）」）。確認が完了しない場合は権限を受け取らず、hatai に知らせる。"

# 偽の t3。呼び出しを 1 行ずつ記録し、標準入力の本文は宛先ごとのファイルに残す。
# thread start は受け付けない（スクリプトは新しい指揮役を立てない。呼ぶと exit 9 で失敗し、記録に残る）。
# 旧指揮役の runtimeMode は auto-accept-edits、新しい指揮役は $NEW_MODE（無ければ auto-accept-edits）。
# NEW_NOMODE=1 のとき、新しい指揮役の thread show は runtimeMode を返さない。
# NEW_WT_AFTER=<n> で新しい指揮役の worktreePath を n 回目の thread show から返す（立てた直後の準備中の演出）。
# NEW_SHOW_DELAY=<秒> で新しい指揮役の thread show の応答を遅らせる（期限までに終わらない読み取りの演出）。
# NEW_CORRUPT=1 で新しい指揮役の thread show が終了コード 0 かつ JSON でない出力（"database is locked" を含む）を返す。
# T3_LOCK_TIMES=<n>（全部の呼び出し）・NEW_LOCK_TIMES=<n>（新しい指揮役の呼び出しだけ）で
# 先頭 n 回の thread show を "database is locked"（標準エラー・終了コード 1）にする。
# T3_SHOW_FAIL=<n> で先頭 n 回の thread show を lock 以外の失敗にする。
cat > "$ROOT/t3" <<'T3'
#!/usr/bin/env bash
echo "$*" >> "$LOG"
case "$1 $2" in
  "thread show")
    n=$(( $(cat "$LOG.show-count" 2>/dev/null || echo 0) + 1 )); echo "$n" > "$LOG.show-count"
    kn=0; if [[ "$3" == *new* ]]; then
      kn=$(( $(cat "$LOG.show-new" 2>/dev/null || echo 0) + 1 )); echo "$kn" > "$LOG.show-new"
    fi
    [[ "$3" == *new* && -n "${NEW_SHOW_DELAY:-}" ]] && sleep "$NEW_SHOW_DELAY"
    if [[ "$3" == *new* && -n "${NEW_CORRUPT:-}" ]]; then
      echo 'not-json database is locked'; exit 0
    fi
    if { [[ -n "${T3_LOCK_TIMES:-}" ]] && [[ "$n" -le "$T3_LOCK_TIMES" ]]; } \
      || { [[ -n "${NEW_LOCK_TIMES:-}" ]] && [[ "$kn" -gt 0 && "$kn" -le "$NEW_LOCK_TIMES" ]]; }; then
      echo "error: database is locked" >&2; exit 1
    fi
    if [[ -n "${T3_SHOW_FAIL:-}" && "$n" -le "$T3_SHOW_FAIL" ]]; then
      echo "error: t3 server unreachable" >&2; exit 1
    fi
    mode=auto-accept-edits
    [[ "$3" == *new* ]] && mode="${NEW_MODE:-auto-accept-edits}"
    wt=""; if [[ "$3" == *new* ]]; then
      wt="${NEW_WT-/home/u/.t3/worktrees/Pelway/conductor}"
      [[ -n "${NEW_WT_AFTER:-}" && "$kn" -lt "$NEW_WT_AFTER" ]] && wt=""
    fi
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
count() { local n; n="$(grep -c -- "$1" "$LOG" 2>/dev/null)"; echo "${n:-0}"; }
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

# worktree のパスが上限まで空のままのときは、旧指揮役にも新指揮役にも何も送らず、
# 宛先の登録も変えずに止まる（試験では上限・間隔を小さく上書きする）
printf 'mcp:launch:kept\n' > "$FILE"
NEW_WT="" PELWAY_REBUILD_WT_WAIT=1 PELWAY_REBUILD_WT_INTERVAL=0.1 run "" --from "$OLD" --to "$NEW"; rc=$?
check "no worktree until the limit: exits non-zero" [ "$rc" -ne 0 ]
check "no worktree until the limit: rereads the new conductor" [ "$(count "thread show $NEW --json")" -gt 1 ]
check "no worktree until the limit: nothing sent to either conductor" no_line "thread send"
check "no worktree until the limit: no thread start" no_line "thread start"
check "no worktree until the limit: registration unchanged" [ "$(cat "$FILE")" = "mcp:launch:kept" ]
check "no worktree until the limit: stderr has the waited seconds" \
  grep -qE "still has no worktree after waiting [0-9]+s" "$ROOT/err"
check "no worktree until the limit: stderr gives the undecided cause and the T3 window check" \
  all_err "$ROOT/err" "準備中" "worktree なしで立てられ" "T3"
check "no worktree until the limit: logs the last-read values" \
  grep -qF "conductor-rebuild: new conductor $NEW runtimeMode=auto-accept-edits worktree=-" "$ROOT/err"
check "no worktree until the limit: tells how to continue" \
  all_err "$ROOT/err" "t3_thread_launch" "--to"

# worktree のパスが読み直しの途中で入ったときは、権限の確かめ以降に進む
printf 'mcp:launch:kept\n' > "$FILE"
NEW_WT="/home/u/.t3/worktrees/Pelway/late" NEW_WT_AFTER=3 \
  PELWAY_REBUILD_WT_WAIT=30 PELWAY_REBUILD_WT_INTERVAL=0.1 run "" --from "$OLD" --to "$NEW"; rc=$?
check "worktree arrives while waiting: succeeds" [ "$rc" -eq 0 ]
check "worktree arrives while waiting: reread the new conductor 3 times" \
  [ "$(count "thread show $NEW --json")" -eq 3 ]
check "worktree arrives while waiting: stops the old conductor once" \
  [ "$(count "thread send --wait $OLD -")" -eq 1 ]
check "worktree arrives while waiting: sends the handover" \
  [ "$(count "thread send $NEW -")" -eq 1 ]
check "worktree arrives while waiting: registers the new conductor" [ "$(cat "$FILE")" = "$NEW" ]
check "worktree arrives while waiting: logs the last-read worktree" \
  grep -qF "conductor-rebuild: new conductor $NEW runtimeMode=auto-accept-edits worktree=/home/u/.t3/worktrees/Pelway/late" "$ROOT/err"

# 読み直しの間隔が残り時間を超えるときは、期限を過ぎる読み取りを始めずに止まる
printf 'mcp:launch:kept\n' > "$FILE"
NEW_WT="/home/u/.t3/worktrees/Pelway/too-late" NEW_WT_AFTER=2 \
  PELWAY_REBUILD_WT_WAIT=1 PELWAY_REBUILD_WT_INTERVAL=1.5 run "" --from "$OLD" --to "$NEW"; rc=$?
check "interval past the limit: exits non-zero" [ "$rc" -ne 0 ]
check "interval past the limit: does not read past the deadline" \
  [ "$(count "thread show $NEW --json")" -eq 1 ]
check "interval past the limit: nothing sent to either conductor" no_line "thread send"
check "interval past the limit: no thread start" no_line "thread start"
check "interval past the limit: registration unchanged" [ "$(cat "$FILE")" = "mcp:launch:kept" ]
check "interval past the limit: stderr reports the timeout" \
  grep -qE "still has no worktree after waiting [0-9]+s" "$ROOT/err"

# 読み取りが期限までに終わらない（応答が期限を過ぎて返る）ときは、その結果を受け入れずに
# 止まる（整数秒では期限と同じに見える境界。偽の t3 が応答を 1.1 秒遅らせる）
printf 'mcp:launch:kept\n' > "$FILE"
NEW_WT="/home/u/.t3/worktrees/Pelway/just-past" NEW_WT_AFTER=1 NEW_SHOW_DELAY=1.1 \
  PELWAY_REBUILD_WT_WAIT=1 PELWAY_REBUILD_WT_INTERVAL=0.1 run "" --from "$OLD" --to "$NEW"; rc=$?
check "read finishes past the limit: exits non-zero" [ "$rc" -ne 0 ]
check "read finishes past the limit: the late read is rejected" \
  [ "$(count "thread show $NEW --json")" -eq 1 ]
check "read finishes past the limit: nothing sent to either conductor" no_line "thread send"
check "read finishes past the limit: no thread start" no_line "thread start"
check "read finishes past the limit: registration unchanged" [ "$(cat "$FILE")" = "mcp:launch:kept" ]
check "read finishes past the limit: stderr reports the timeout" \
  grep -qE "still has no worktree after waiting [0-9]+s" "$ROOT/err"

# 読み取りの中の lock のやり直しも残り時間に収める。やり直しの待ちを残りで切り詰め、
# 期限を超える読み直しは始めない
printf 'mcp:launch:kept\n' > "$FILE"
NEW_LOCK_TIMES=1 PELWAY_REBUILD_SHOW_RETRY_SLEEP=1.2 \
  PELWAY_REBUILD_WT_WAIT=1 PELWAY_REBUILD_WT_INTERVAL=0.1 run "" --from "$OLD" --to "$NEW"; rc=$?
check "lock retry past the limit: exits non-zero" [ "$rc" -ne 0 ]
check "lock retry past the limit: does not retry past the deadline" \
  [ "$(count "thread show $NEW --json")" -eq 1 ]
check "lock retry past the limit: nothing sent to either conductor" no_line "thread send"
check "lock retry past the limit: registration unchanged" [ "$(cat "$FILE")" = "mcp:launch:kept" ]
check "lock retry past the limit: stderr reports the timeout" \
  grep -qE "still has no worktree after waiting [0-9]+s" "$ROOT/err"

# 小数点がコンマのロケールでも期限を検出する（期限の時計はロケールに依存しない）
loc="$(locale -a 2>/dev/null | grep -iEx 'fr_FR|de_DE|nl_NL|it_IT|fi_FI|da_DK|sv_SE|pl_PL|pt_BR|tr_TR|es_ES|ru_RU' | head -1)"
if [[ -n "$loc" ]]; then
  printf 'mcp:launch:kept\n' > "$FILE"
  LC_ALL="$loc" NEW_WT="" PELWAY_REBUILD_WT_WAIT=1 PELWAY_REBUILD_WT_INTERVAL=0.1 \
    run "" --from "$OLD" --to "$NEW"; rc=$?
  check "comma-decimal locale ($loc): exits non-zero" [ "$rc" -ne 0 ]
  check "comma-decimal locale ($loc): nothing sent to either conductor" no_line "thread send"
  check "comma-decimal locale ($loc): registration unchanged" [ "$(cat "$FILE")" = "mcp:launch:kept" ]
  check "comma-decimal locale ($loc): stderr reports the timeout" \
    grep -qE "still has no worktree after waiting [0-9]+s" "$ROOT/err"
else
  echo "skip comma-decimal locale check: no comma-decimal locale installed"
fi

# thread show が "database is locked" を返し続けるときは、やり直しの上限（初回 + 3 回）のあと
# 旧指揮役に何も送らずに止まる
printf 'mcp:launch:kept\n' > "$FILE"
T3_LOCK_TIMES=99 PELWAY_REBUILD_SHOW_RETRY_SLEEP=0.1 run "" --from "$OLD" --to "$NEW"; rc=$?
check "lock keeps failing: exits non-zero" [ "$rc" -ne 0 ]
check "lock keeps failing: retried 3 times (4 reads)" [ "$(count "thread show $OLD --json")" -eq 4 ]
check "lock keeps failing: nothing sent to either conductor" no_line "thread send"
check "lock keeps failing: registration unchanged" [ "$(cat "$FILE")" = "mcp:launch:kept" ]

# "database is locked" のあと成功すれば、やり直して進む
printf 'mcp:launch:kept\n' > "$FILE"
T3_LOCK_TIMES=1 PELWAY_REBUILD_SHOW_RETRY_SLEEP=0.1 run "" --from "$OLD" --to "$NEW"; rc=$?
check "lock then ok: succeeds" [ "$rc" -eq 0 ]
check "lock then ok: reread the old conductor" [ "$(count "thread show $OLD --json")" -eq 2 ]
check "lock then ok: stops the old conductor once" [ "$(count "thread send --wait $OLD -")" -eq 1 ]

# 新しい指揮役の読み取りでも "database is locked" はやり直す
printf 'mcp:launch:kept\n' > "$FILE"
NEW_LOCK_TIMES=1 PELWAY_REBUILD_SHOW_RETRY_SLEEP=0.1 run "" --from "$OLD" --to "$NEW"; rc=$?
check "lock on the new conductor then ok: succeeds" [ "$rc" -eq 0 ]
check "lock on the new conductor then ok: reread the new conductor" \
  [ "$(count "thread show $NEW --json")" -eq 2 ]

# "database is locked" 以外の thread show の失敗では、やり直さずに止まる
printf 'mcp:launch:kept\n' > "$FILE"
T3_SHOW_FAIL=1 PELWAY_REBUILD_SHOW_RETRY_SLEEP=0.1 run "" --from "$OLD" --to "$NEW"; rc=$?
check "non-lock show failure: exits non-zero" [ "$rc" -ne 0 ]
check "non-lock show failure: reads once" [ "$(count "thread show")" -eq 1 ]
check "non-lock show failure: nothing sent to either conductor" no_line "thread send"
check "non-lock show failure: registration unchanged" [ "$(cat "$FILE")" = "mcp:launch:kept" ]

# t3 コマンドは成功したが出力が JSON として読めない（"database is locked" の文字を含んでも）
# 場合は、t3 コマンド自体の失敗ではないため、やり直さずに止まる
printf 'mcp:launch:kept\n' > "$FILE"
NEW_CORRUPT=1 PELWAY_REBUILD_SHOW_RETRY_SLEEP=0.1 run "" --from "$OLD" --to "$NEW"; rc=$?
check "corrupt output: exits non-zero" [ "$rc" -ne 0 ]
check "corrupt output: does not retry the new conductor" \
  [ "$(count "thread show $NEW --json")" -eq 1 ]
check "corrupt output: nothing sent to either conductor" no_line "thread send"
check "corrupt output: registration unchanged" [ "$(cat "$FILE")" = "mcp:launch:kept" ]

# worktree が無く、かつ権限も合わないときは、権限の失敗（exit 3）で止まる
printf 'mcp:launch:kept\n' > "$FILE"
NEW_MODE=full-access NEW_WT="" PELWAY_REBUILD_WT_WAIT=0 PELWAY_REBUILD_WT_INTERVAL=0.1 \
  run "" --from "$OLD" --to "$NEW"; rc=$?
check "no worktree + bad mode: exits 3" [ "$rc" -eq 3 ]
check "no worktree + bad mode: nothing sent to the old conductor" no_line "thread send --wait $OLD"
check "no worktree + bad mode: no handover sent" no_line "thread send $NEW"
check "no worktree + bad mode: no thread start" no_line "thread start"
check "no worktree + bad mode: registration unchanged" [ "$(cat "$FILE")" = "mcp:launch:kept" ]
check "no worktree + bad mode: logs both read values" \
  grep -qF "conductor-rebuild: new conductor $NEW runtimeMode=full-access worktree=-" "$ROOT/err"

# 失敗したら登録を変えずに止まる。thread send はやり直さない（二重送信を防ぐため）。
printf 'mcp:launch:kept\n' > "$FILE"
run send-fail --from "$OLD" --to "$NEW"; rc=$?
check "stop failure exits non-zero" [ "$rc" -ne 0 ]
check "stop failure keeps the registration" [ "$(cat "$FILE")" = "mcp:launch:kept" ]
check "stop failure: no thread start" no_line "thread start"
check "stop failure: thread send is tried once" [ "$(count "thread send")" -eq 1 ]
check "stop failure: the one send was the stop to the old conductor" \
  [ "$(count "thread send --wait $OLD -")" -eq 1 ]

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
