#!/usr/bin/env bash
# scripts/agent-worktree-prune のテスト。一時ディレクトリに origin・本体・worktree を作り、偽の gh で PR の状態を、
# 偽の t3 でスレッドの状態を返す。偽の mbx は呼ばれたことを記録する。
set -uo pipefail

SCRIPT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/agent-worktree-prune"
ROOT="$(mktemp -d)"
sleeper=""
trap '[[ -n "$sleeper" ]] && kill "$sleeper" 2>/dev/null; rm -rf "$ROOT"' EXIT
failures=()

BIN="$ROOT/bin"
mkdir -p "$BIN"
# 偽の gh: `gh pr list --state all ...` に FAKE_PRS（JSON）を返す
cat >"$BIN/gh" <<'EOF'
#!/usr/bin/env bash
[[ "$1 $2" == "pr list" ]] || exit 9
printf '%s\n' "${FAKE_PRS:-[]}"
EOF
chmod +x "$BIN/gh"
# 偽の t3: FAKE_THREADS（threadId・settled・worktreePath を持つ JSON 配列）から
# `t3 thread list --json` と `t3 thread show <id> --json` に答える。FAKE_T3_FAIL=1 なら失敗する。
cat >"$BIN/t3" <<'EOF'
#!/usr/bin/env bash
[[ "${FAKE_T3_FAIL:-}" == 1 ]] && exit 1
t="${FAKE_THREADS:-[]}"
case "$1 $2" in
  "thread list") jq '[.[] | {threadId, settled}]' <<<"$t" ;;
  "thread show") jq --arg id "$3" '{thread: (.[] | select(.threadId == $id))}' <<<"$t" ;;
  *) exit 9 ;;
esac
EOF
chmod +x "$BIN/t3"
# 偽の mbx: 呼ばれたら $ROOT/mbx-called に引数を書く
cat >"$BIN/mbx" <<EOF
#!/usr/bin/env bash
echo "\$*" >"$ROOT/mbx-called"
EOF
chmod +x "$BIN/mbx"

g() { git -c user.name=t -c user.email=t@example.com "$@"; }

# 毎回まっさらな origin・本体・worktree を作る
setup() {
  rm -rf "$ROOT/origin.git" "$ROOT/main" "$ROOT/wt" "$ROOT/other" "$ROOT/mbx-called"
  git init -q --bare -b main "$ROOT/origin.git"
  g clone -q "$ROOT/origin.git" "$ROOT/main" 2>/dev/null
  g -C "$ROOT/main" commit -q --allow-empty -m init
  g -C "$ROOT/main" push -q origin main
  mkdir -p "$ROOT/wt"
}

# 本体の main に取り込まれた worktree（merge コミットで取り込む）
add_merged() {
  g -C "$ROOT/main" worktree add -q -b "$1" "$ROOT/wt/$1" main
  g -C "$ROOT/wt/$1" commit -q --allow-empty -m "work $1"
  g -C "$ROOT/main" merge -q --no-ff -m "merge $1" "$1"
  g -C "$ROOT/main" push -q origin main
}

# main に取り込まれていない worktree
add_unmerged() {
  g -C "$ROOT/main" worktree add -q -b "$1" "$ROOT/wt/$1" main
  g -C "$ROOT/wt/$1" commit -q --allow-empty -m "work $1"
}

# worktree の作業ツリーと git dir の HEAD・index を 7 時間前にする（6 時間以内に更新された worktree は残すため）
age() {
  find "$ROOT/wt" "$ROOT/other" "$ROOT/main/.git/worktrees" -exec touch -h -d '7 hours ago' {} + 2>/dev/null
}

run() {
  (cd "$ROOT/main" && PATH="$BIN:$PATH" T3_BIN="$BIN/t3" XDG_STATE_HOME="$ROOT/state" PELWAY_T3_WORKTREES="$ROOT/wt" bash "$SCRIPT" "$@")
}

check() {
  local name="$1"; shift
  if "$@"; then printf 'ok   %s\n' "$name"; else printf 'FAIL %s\n' "$name"; failures+=("$name"); fi
}

gone() { [[ ! -e "$ROOT/wt/$1" ]]; }
kept() { [[ -d "$ROOT/wt/$1" ]]; }
branch_gone() { ! git -C "$ROOT/main" rev-parse -q --verify "refs/heads/$1" >/dev/null; }
branch_kept() { git -C "$ROOT/main" rev-parse -q --verify "refs/heads/$1" >/dev/null; }

# 1. main に取り込まれたものは worktree もブランチも消す
setup
add_merged a-merged
age
run --yes >/dev/null 2>&1
check "merged worktree removed" gone a-merged
check "merged branch deleted" branch_gone a-merged

# 2. 取り込まれていず PR も無いものは残す
setup
add_unmerged b-open
age
run --yes >/dev/null 2>&1
check "unmerged worktree kept" kept b-open

# 3. 変更が残っているものは、取り込み済みでも残す
setup
add_merged c-dirty
echo x >"$ROOT/wt/c-dirty/new.txt"
age
run --yes >/dev/null 2>&1
check "dirty worktree kept" kept c-dirty

# 4. 旧プラグインのファイルだけなら変更と数えない
setup
add_merged d-plugin
mkdir -p "$ROOT/wt/d-plugin/.opencode/plugins"
echo x >"$ROOT/wt/d-plugin/.opencode/plugins/pelway-agent-notify.ts"
age
run --yes >/dev/null 2>&1
check "old plugin file is not a change" gone d-plugin

# 5. squash などで取り込まれ、PR がマージ済みなら消す
setup
add_unmerged e-squashed
age
FAKE_PRS='[{"headRefName":"e-squashed","state":"MERGED"}]' run --yes >/dev/null 2>&1
check "merged PR worktree removed" gone e-squashed
check "merged PR branch deleted" branch_gone e-squashed

# 6. PR が閉じただけなら worktree は消すがブランチは残す
setup
add_unmerged f-closed
age
FAKE_PRS='[{"headRefName":"f-closed","state":"CLOSED"}]' run --yes >/dev/null 2>&1
check "closed PR worktree removed" gone f-closed
check "closed PR branch kept" branch_kept f-closed

# 7. 同じブランチで開いている PR があれば残す
setup
add_unmerged g-reopened
age
FAKE_PRS='[{"headRefName":"g-reopened","state":"CLOSED"},{"headRefName":"g-reopened","state":"OPEN"}]' run --yes >/dev/null 2>&1
check "open PR worktree kept" kept g-reopened

# 8. 確認に n と答えたら何も消さない
setup
add_merged h-ask
age
echo n | run >/dev/null 2>&1
check "answer n removes nothing" kept h-ask

# 9. T3 の worktree の置き場所の外にあるものは触らない
setup
mkdir -p "$ROOT/other"
g -C "$ROOT/main" worktree add -q -b i-outside "$ROOT/other/i-outside" main
age
run --yes >/dev/null 2>&1
check "worktree outside root untouched" test -d "$ROOT/other/i-outside"

# 10. --dry-run は一覧を出すだけ
setup
add_merged j-dry
out="$(run --dry-run 2>&1)"
check "dry-run removes nothing" kept j-dry
check "dry-run lists candidate" grep -q "j-dry" <<<"$out"

# 11. settle されていないスレッドの worktree は、取り込み済みでも残す（作りたてで main と同じものを含む）
setup
g -C "$ROOT/main" worktree add -q -b k-busy "$ROOT/wt/k-busy" main
age
FAKE_THREADS='[{"threadId":"t1","settled":false,"worktreePath":"'"$ROOT/wt/k-busy"'"}]' run --auto >/dev/null 2>&1
check "unsettled thread worktree kept" kept k-busy
check "unsettled thread branch kept" branch_kept k-busy
age
FAKE_THREADS='[{"threadId":"t1","settled":false,"worktreePath":"'"$ROOT/wt/k-busy"'"}]' run --yes >/dev/null 2>&1
check "unsettled thread worktree kept with --yes" kept k-busy

# 12. settle 済みのスレッドの worktree は消す
setup
add_merged l-settled
age
FAKE_THREADS='[{"threadId":"t1","settled":true,"worktreePath":"'"$ROOT/wt/l-settled"'"}]' run --auto >/dev/null 2>&1
check "settled thread worktree removed" gone l-settled

# 13. T3 に問い合わせられなければ何も消さず、失敗で終わる
setup
add_merged m-t3down
age
FAKE_T3_FAIL=1 run --auto >/dev/null 2>&1; rc=$?
check "t3 failure removes nothing" kept m-t3down
check "t3 failure exits non-zero" test "$rc" -ne 0
check "t3 failure skips mbx gc" test ! -e "$ROOT/mbx-called"

# 14. --auto は未追跡のファイルだけなら消し、後片付けで mbx gc を呼ぶ
setup
add_merged n-untracked
mkdir -p "$ROOT/wt/n-untracked/.pelway-review"
echo x >"$ROOT/wt/n-untracked/.pelway-review/notes.md"
age
run --auto >/dev/null 2>&1
check "auto removes untracked-only worktree" gone n-untracked
check "auto runs mbx gc" grep -qx gc "$ROOT/mbx-called"

# 15. --auto でも追跡ファイルに変更があれば残す
setup
add_merged o-modified
echo x >"$ROOT/wt/o-modified/tracked.txt"
g -C "$ROOT/wt/o-modified" add tracked.txt
age
run --auto >/dev/null 2>&1
check "auto keeps worktree with tracked changes" kept o-modified

# 16. --yes は未追跡のファイルが残っていれば残し、mbx gc を呼ばない
setup
add_merged p-yes-untracked
echo x >"$ROOT/wt/p-yes-untracked/new.txt"
age
run --yes >/dev/null 2>&1
check "yes keeps untracked worktree" kept p-yes-untracked
check "yes skips mbx gc" test ! -e "$ROOT/mbx-called"

# 17. --auto は linked worktree から走らせると何もしない（pitchfork は各 worktree の cron も動かすため）
setup
add_merged q-from-wt
g -C "$ROOT/main" worktree add -q -b q-runner "$ROOT/other/q-runner" main
age
(cd "$ROOT/other/q-runner" && PATH="$BIN:$PATH" T3_BIN="$BIN/t3" XDG_STATE_HOME="$ROOT/state" PELWAY_T3_WORKTREES="$ROOT/wt" bash "$SCRIPT" --auto >/dev/null 2>&1)
check "auto from linked worktree removes nothing" kept q-from-wt
check "auto from linked worktree skips mbx gc" test ! -e "$ROOT/mbx-called"

# 18. --auto は別の --auto が実行中なら何もしない
setup
add_merged r-locked
mkdir -p "$ROOT/state/pelway"
exec 8>"$ROOT/state/pelway/agent-worktree-prune.lock"; flock 8
age
run --auto >/dev/null 2>&1
exec 8>&-
check "auto under lock removes nothing" kept r-locked

# 19. 生きているプロセスのカレントディレクトリが中にある worktree は、取り込み済みでも残し、止めた後は消す
setup
add_merged s-proc
mkdir -p "$ROOT/wt/s-proc/sub"
(cd "$ROOT/wt/s-proc/sub" && exec sleep 300) &
sleeper=$!
for _ in $(seq 50); do [[ "$(readlink "/proc/$sleeper/cwd" 2>/dev/null)" == "$ROOT/wt/s-proc/sub" ]] && break; sleep 0.1; done
age
out="$(run --auto 2>&1)"
check "worktree with live process kept" kept s-proc
check "live process reason listed" grep -q "s-proc (プロセスが動いている: pid $sleeper sleep)" <<<"$out"
out="$(run --dry-run 2>&1)"
check "dry-run lists live process reason" grep -q "s-proc (プロセスが動いている: pid $sleeper sleep)" <<<"$out"
kill "$sleeper"; wait "$sleeper" 2>/dev/null; sleeper=""
age
run --auto >/dev/null 2>&1
check "worktree removed after process stops" gone s-proc

# 20. 6 時間以内に更新された worktree は、スレッドが無くても残し、古くなれば消す
setup
add_merged t-recent
echo x >"$ROOT/wt/t-recent/.pelway-review-notes"
out="$(run --auto 2>&1)"
check "recently updated worktree kept" kept t-recent
check "recent update reason listed" grep -q "t-recent (6 時間以内に更新された)" <<<"$out"
out="$(run --dry-run 2>&1)"
check "dry-run lists recent update reason" grep -q "t-recent (6 時間以内に更新された)" <<<"$out"
age
run --auto >/dev/null 2>&1
check "worktree older than 6 hours removed" gone t-recent

# 21. 作業ツリーが古くても、git dir の HEAD か index が 6 時間以内なら残す
setup
add_merged u-index
age
touch "$ROOT/main/.git/worktrees/u-index/index"
run --auto >/dev/null 2>&1
check "worktree with recent index kept" kept u-index
setup
add_merged v-head
age
touch "$ROOT/main/.git/worktrees/v-head/HEAD"
run --auto >/dev/null 2>&1
check "worktree with recent HEAD kept" kept v-head

if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}"
  exit 1
fi
printf '\nall passed\n'
