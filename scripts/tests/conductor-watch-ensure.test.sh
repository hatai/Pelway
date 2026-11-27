#!/usr/bin/env bash
# scripts/conductor-watch-ensure.sh のテスト。
# 構造検査（存在・実行権・構文・正本が ensure を指すか）は常に行う。
# 起動・冪等・kill 後の再起動・セッション切り離しの動作は、ユーザー+PID+マウント
# 名前空間の中で行う。名前空間内の /proc には名前空間内のプロセスだけが見えるため、
# 実環境で動いている conductor-watch.sh と混ざらない（pgrep の数え間違いが起きない）。
# systemd の経路はモックの systemctl（start が unit の ExecStart を読んで setsid で起きる）
# で試す。本物の Restart= による自動復旧は環境依存のため、このテストの対象外。
# 動作検査は unshare で名前空間が作れない環境では飛ばす。
# Issue #808: ensure はコマンドラインに scripts/conductor-watch.sh を持つプロセスをすべて「見張り」
# に数えていたため、shellcheck の検査（スクリプトのパスを引数に並べる）や、conductor-watch の
# 試験が一時ディレクトリに置いた写しの見張りまで kill していた。名前空間の中で、ensure がそれらを
# 選ばないこと（見張りを置き換えるのと同じ 1 回の実行で確かめる）、相対パス（`./scripts/…`）で
# 起動した見張りや空白を含む checkout の見張りを見失わないこと、逆に、実運用の conductor-watch
# の試験（自分の見張りを止める片付けを持つもの）を管理対象の見張りと並行して通しても、
# その見張りが残ることを確かめる。
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ENSURE="$REPO/scripts/conductor-watch-ensure.sh"
ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT
failures=()

check() {
  local name="$1"; shift
  if "$@"; then printf 'ok   %s\n' "$name"; else printf 'FAIL %s\n' "$name"; failures+=("$name"); fi
}
eq() { [[ "$1" == "$2" ]]; }
has() { grep -q "$2" "$1"; }

# --- 構造（常に確かめる） ---

check "ensure script exists"        test -f "$ENSURE"
check "ensure script is executable" test -x "$ENSURE"
check "ensure script syntax"        bash -n "$ENSURE"
check "command-team points to ensure" \
  has "$REPO/.agents/skills/command-team/SKILL.md" "conductor-watch-ensure.sh"
check "runbook points to ensure" \
  has "$REPO/docs/agents/grokbot-runbook.md" "conductor-watch-ensure.sh"
check "watch header points to ensure" \
  has "$REPO/scripts/conductor-watch.sh" "conductor-watch-ensure.sh"
check "skill trees identical" \
  diff -qr "$REPO/.agents/skills" "$REPO/.claude/skills"

# --- 動作（名前空間の中。ensure が無い、または unshare が使えないときは飛ばす） ---

if [[ ! -f "$ENSURE" ]]; then
  echo "skip behavior: $ENSURE is missing"
elif ! unshare -U -r -m -p -f --mount-proc true 2>/dev/null; then
  echo "skip behavior: unshare (user+pid+mount namespace) unavailable"
else
  mkdir -p "$ROOT/repo/scripts" "$ROOT/bin" "$ROOT/sysbin" "$ROOT/systemd-bin" \
           "$ROOT/state" "$ROOT/config" "$ROOT/home" "$ROOT/mock" "$ROOT/queue"
  cp "$REPO/scripts/conductor-watch.sh" "$ENSURE" "$ROOT/repo/scripts/"
  cp "$REPO/scripts/patrol-state-update" "$ROOT/repo/scripts/"
  # 実運用の見張りを止める 2 つの試験（slots: 単体 PID の kill。stall: プロセスグループの kill）を
  # 名前空間の中で実際に通すため、試験ごとの偽リポジトリに置く。各試験は REPO/scripts から
  # 自分で写すため、写す側の scripts には試験が読むスクリプトを本物のまま置く。
  mkdir -p "$ROOT/trepo-slots/scripts/tests" "$ROOT/trepo-stall/scripts/tests"
  cp "$REPO/scripts/"{conductor-watch.sh,agent-team-config,patrol-state-update} "$ROOT/trepo-slots/scripts/"
  cp "$REPO/scripts/"{conductor-watch.sh,patrol-state-update} "$ROOT/trepo-stall/scripts/"
  cp "$REPO/scripts/tests/conductor-watch-slots.test.sh" "$ROOT/trepo-slots/scripts/tests/"
  cp "$REPO/scripts/tests/conductor-watch-stall.test.sh" "$ROOT/trepo-stall/scripts/tests/"
  # グループ停止の検査が回す見張り用: trepo-stall を git init して agent-gh のモックを置く
  # （見張りは自分のリポジトリの scripts/agent-gh を呼ぶ）。
  git init -q "$ROOT/trepo-stall"
  cat > "$ROOT/trepo-stall/scripts/agent-gh" <<'EOF'
#!/usr/bin/env bash
echo '[]'
EOF
  chmod +x "$ROOT/trepo-stall/scripts/agent-gh"

  # 見張りが呼ぶ外部コマンドのモック。巡回の成否は問わない（動作検査はプロセスの
  # 生死だけを見る）ため、どちらも空の一覧を返すだけにする。
  cat > "$ROOT/repo/scripts/agent-gh" <<'EOF'
#!/usr/bin/env bash
echo '[]'
EOF
  cat > "$ROOT/bin/t3" <<'EOF'
#!/usr/bin/env bash
echo '[]'
EOF
  chmod +x "$ROOT/repo/scripts/agent-gh" "$ROOT/bin/t3"

  # systemctl を除いた最小 PATH。ここに無いコマンドは名前空間内で「無い」扱いになる。
  for t in bash sh env setsid pgrep ps mkdir cp chmod dirname python3 cat grep sed tr cut wc sleep flock kill ls head tail awk date git mktemp rm ln touch chown sha256sum rmdir mkfifo; do
    p="$(command -v "$t" 2>/dev/null)" && ln -s "$p" "$ROOT/sysbin/$t"
  done

  # モックの systemctl。start は unit ファイルの ExecStart を読んで setsid で起動し、
  # MainPID を記録する（本物の unit ファイルが ensure に書かせた内容で動くことを
  # 確かめられる）。show -p MainPID --value・is-active・起動済みの start は本物に倣う。
  cat > "$ROOT/systemd-bin/systemctl" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$*" >> "$MOCK/systemctl.log"
[[ "${1:-}" == "--user" ]] && shift
verb="${1:-}"; shift || true
alive() { [[ -s "$MOCK/mainpid" ]] && kill -0 "$(cat "$MOCK/mainpid")" 2>/dev/null; }
case "$verb" in
  show)
    if alive; then cat "$MOCK/mainpid"; else echo 0; fi ;;
  is-active)
    alive ;;
  start)
    alive && exit 0
    unit="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user/$1"
    exe="$(sed -n 's/^ExecStart=//p' "$unit" | head -1)"
    [[ -n "$exe" ]] || exit 9
    mkdir -p "${XDG_STATE_HOME:-$HOME/.local/state}/pelway/conductor-watch"
    setsid "$exe" 9>&- </dev/null >>"${XDG_STATE_HOME:-$HOME/.local/state}/pelway/conductor-watch/watch.log" 2>&1 &
    echo $! > "$MOCK/mainpid" ;;
esac
exit 0
EOF
  chmod +x "$ROOT/systemd-bin/systemctl"

  # 名前空間の中で動く検査本体。$1 = fallback（systemctl 無し）| systemd（モックあり）。
  cat > "$ROOT/ns-run.sh" <<'EOF'
#!/usr/bin/env bash
set -u
MODE="$1"
export HOME="$ROOT/home" XDG_STATE_HOME="$ROOT/state" XDG_CONFIG_HOME="$ROOT/config" MOCK="$ROOT/mock"
# fake repo は git 管理でないため、patrol-state の置き場は
# 環境変数で明示する（他の conductor-watch テストと同じやり方）。
export PELWAY_PATROL_STATE="$ROOT/state/pelway/patrol-state.json"
# 見張りは既定で実運用のボード更新キュー（/tmp/pelway-board-queue）を開く。キューはマウント
# 名前空間で分かれないため、名前空間内のすべての起動が試験用のキューを使うように固定する
# （gh が PATH に無いため、実運用の未処理の依頼を読んだ見張りはそれを失敗扱いで消す）。
export PELWAY_BOARD_QUEUE_DIR="$ROOT/queue"
if [[ "$MODE" == systemd ]]; then
  export PATH="$ROOT/bin:$ROOT/systemd-bin:$ROOT/sysbin"
else
  export PATH="$ROOT/bin:$ROOT/sysbin"
fi
ENS="$ROOT/repo/scripts/conductor-watch-ensure.sh"
LOGD="$ROOT/state/pelway/conductor-watch"
UNIT="$ROOT/config/systemd/user/pelway-conductor-watch.service"

count() { pgrep -fc "${1:-conductor-watch\.sh}"; }   # 見張りは巡回の中で自分と同じコマンドラインの subshell を瞬間的に作るため、数は一瞬多く数えられる
pids()  { pgrep -f 'conductor-watch\.sh' | head -1; }
wait0() { local i; for i in 1 2 3 4 5 6 7 8 9 10; do [[ "$(count)" == 0 ]] && return 0; sleep 0.2; done; return 1; }
waitn() { local want="$1" pat="${2:-conductor-watch\.sh}" i; for i in {1..15}; do [[ "$(count "$pat")" == "$want" ]] && return 0; sleep 0.2; done; [[ "$(count "$pat")" == "$want" ]]; }
t() { local n="$1"; shift; if "$@"; then echo "ok   $n"; else echo "FAIL $n"; fi }
eq() { [[ "$1" == "$2" ]]; }
neq() { [[ "$1" != "$2" ]]; }
has() { grep -q "$2" "$1"; }
# 見張りではないのに scripts/conductor-watch.sh をコマンドラインに持つ 2 つの形（Issue #808）。
# ensure がどちらも選ばないことを、unit 外・切り離し前の見張りを置き換えるのと同じ 1 回の実行で確かめる。
# count / pids は広いパターンのままなので、decoy は数える処理のあとに立てて、数える前に片付ける。
argpid=""; copypid=""
start_decoys() {  # 1) shellcheck の検査と同じ形（スクリプトのパスを引数に並べるだけ） 2) リポジトリの外の写しの見張り（conductor-watch の試験と同じ形）
  local d="$ROOT/elsewhere/tmp.a1B2c3/repo/scripts"
  bash -c 'exec -a "shellcheck --severity=warning scripts/agent-gh scripts/conductor-watch.sh scripts/agent-team-config" sleep 300' >/dev/null 2>&1 &
  argpid=$!
  mkdir -p "$d"
  printf '#!/usr/bin/env bash\nwhile :; do sleep 30; done\n' >"$d/conductor-watch.sh"
  bash "$d/conductor-watch.sh" >/dev/null 2>&1 &
  copypid=$!
}
t_decoys() {  # <mode>: どちらも生きていること
  t "$1: argument-only process kept"   eq "$(kill -0 "$argpid" 2>/dev/null; echo $?)" "0"
  t "$1: out-of-repo watch copy kept"  eq "$(kill -0 "$copypid" 2>/dev/null; echo $?)" "0"
}
stop_decoys() { kill "$argpid" "$copypid" 2>/dev/null; wait "$argpid" "$copypid" 2>/dev/null; argpid=""; copypid=""; }
wait_gone() {  # <pid>: 終わるのを上限付きで待つ（生き残った場合に wait で止まらないようにする）
  local i; for i in {1..25}; do kill -0 "$1" 2>/dev/null || return 0; sleep 0.2; done; return 1
}
t_test_kills() {  # <mode>: conductor-watch の実際の試験（slots: 単体 PID の kill）と実際の停止処理（stall の試験が使うプロセスグループへの kill -KILL -- -PGID）を、ensure の管理対象の見張り（p1）と並行して通す。試験が通ること、試験が立てた見張りが残らないこと、p1 が残ることを確かめる
  # 試験側の偽リポジトリは git init されるため、patrol-state の置き場は試験に既定の判定を任せる
  env -u PELWAY_PATROL_STATE bash "$ROOT/trepo-slots/scripts/tests/conductor-watch-slots.test.sh" >"$MOCK/slots.out" 2>&1
  t "$1: slots test passes beside managed watch"  eq "$?" "0"
  env -u PELWAY_PATROL_STATE bash "$ROOT/trepo-stall/scripts/tests/conductor-watch-stall.test.sh" >"$MOCK/stall.out" 2>&1
  t "$1: stall test passes beside managed watch"  eq "$?" "0"
  t "$1: test watchers all stopped"               waitn 1
  # 正常終了する巡回では stall の試験のグループ停止を通らないため、子を持つ試験用の見張りに
  # その停止処理（kill -KILL -- -PGID。stall の試験の 15・93 行）をそのまま通す。子の 1 つは
  # 見張りと別名（inbox-sleeper）にする: 名前で数える検査は別名の子を見逃すため、残りは
  # プロセスグループで確かめる。
  env -u PELWAY_PATROL_STATE setsid bash -c \
    'bash "$1" >>"$MOCK/groupkill.out" 2>&1 & (exec -a inbox-sleeper sleep 300) & wait' \
    _ "$ROOT/trepo-stall/scripts/conductor-watch.sh" &
  gpid=$!
  for i in {1..25}; do pgrep -g "$gpid" -f inbox-sleeper >/dev/null 2>&1 && break; sleep 0.2; done
  t "$1: group-kill target has a differently-named child" test -n "$(pgrep -g "$gpid" -f inbox-sleeper)"
  kill -KILL -- -"$gpid" 2>/dev/null
  wait "$gpid" 2>/dev/null  # 起動元は自分の子なので回収しておく（zombie は pgrep -g に数えられる）
  for i in {1..15}; do [[ -z "$(pgrep -g "$gpid" 2>/dev/null)" ]] && break; sleep 0.2; done
  t "$1: group kill stops test watcher and all children" eq "$(pgrep -g "$gpid" 2>/dev/null || true)" ""
  t "$1: ensure-managed watch kept beside the tests' kills"  eq "$(kill -0 "$p1" 2>/dev/null; echo $?)" "0"
}

if [[ "$MODE" == systemd ]]; then
  bash "$ENS" >"$MOCK/out.log" 2>&1
  t "systemd: ensure exits 0"            eq "$?" "0"
  t "systemd: unit file written"         test -f "$UNIT"
  t "systemd: unit has Restart=always"   has "$UNIT" "^Restart=always"
  t "systemd: unit ExecStart is watch"   has "$UNIT" "^ExecStart=$ROOT/repo/scripts/conductor-watch.sh\$"
  t "systemd: systemctl start called"    has "$MOCK/systemctl.log" "start pelway-conductor-watch.service"
  t "systemd: watch is running"          waitn 1
  p1="$(pids)"
  bash "$ENS" >>"$MOCK/out.log" 2>&1
  t "systemd: second ensure keeps 1"     waitn 1
  t_test_kills systemd
  t "systemd: count back to 1 after test kills"  waitn 1
  # unit 外で動く見張りは、kill しても復旧しない形のため unit に置き換える
  setsid -f bash "$ROOT/repo/scripts/conductor-watch.sh" >/dev/null 2>&1
  waitn 2
  foreign="$(pgrep -f 'conductor-watch\.sh' | grep -vx "$p1" | head -1)"
  t "systemd: foreign watch present"     waitn 2
  start_decoys
  bash "$ENS" >>"$MOCK/out.log" 2>&1
  t "systemd: foreign watch killed"      eq "$(kill -0 "$foreign" 2>/dev/null; echo $?)" "1"
  t "systemd: unit watch kept"           eq "$(kill -0 "$p1" 2>/dev/null; echo $?)" "0"
  t_decoys systemd
  stop_decoys
  t "systemd: count back to 1"           waitn 1
  kill "$p1" 2>/dev/null; wait0
  bash "$ENS" >>"$MOCK/out.log" 2>&1
  t "systemd: ensure restarts watch"     waitn 1
  p2="$(pids)"
  t "systemd: restarted pid differs"     neq "$p2" "$p1"
else
  t "fallback: no systemctl on PATH"     eq "$(command -v systemctl || echo none)" "none"
  bash "$ENS" >"$MOCK/out.log" 2>&1
  t "fallback: ensure exits 0"           eq "$?" "0"
  t "fallback: watch is running"         waitn 1
  p1="$(pids)"
  bash "$ENS" >>"$MOCK/out.log" 2>&1
  t "fallback: second ensure keeps 1"    waitn 1
  bash "$ENS" >>"$MOCK/out.log" 2>&1
  t "fallback: third ensure keeps 1"     waitn 1
  t_test_kills fallback
  t "fallback: count back to 1 after test kills"  waitn 1
  # セッションを切り離していない見張り（nohup & 相当）は切り離した形に置き換える。
  # `./scripts/…` の相対パスで起動した形で確かめる（見張りは起動してすぐリポジトリのルートへ cd する。
  # ensure は見張りのスクリプトを、その cd のあとの cwd から求める）。
  (cd "$ROOT/repo" && exec bash ./scripts/conductor-watch.sh) >/dev/null 2>&1 &
  fragile=$!
  waitn 2
  t "fallback: fragile watch present"    waitn 2
  start_decoys
  bash "$ENS" >>"$MOCK/out.log" 2>&1
  wait_gone "$fragile" && wait "$fragile" 2>/dev/null   # 置き換わらなかった場合に試験が止まらないようにする
  t "fallback: fragile watch replaced"   eq "$(kill -0 "$fragile" 2>/dev/null; echo $?)" "1"
  t "fallback: leader watch kept"        eq "$(kill -0 "$p1" 2>/dev/null; echo $?)" "0"
  t_decoys fallback
  stop_decoys
  t "fallback: count back to 1"          waitn 1
  # 切り離して起きた見張りが 2 つあるときは後からのものを落として 1 つにする
  setsid -f bash "$ROOT/repo/scripts/conductor-watch.sh" >/dev/null 2>&1
  waitn 2
  t "fallback: extra leader present"     waitn 2
  bash "$ENS" >>"$MOCK/out.log" 2>&1
  t "fallback: extra leader killed"      waitn 1
  t "fallback: original leader kept"     eq "$(kill -0 "$p1" 2>/dev/null; echo $?)" "0"
  kill "$p1" 2>/dev/null; wait0
  bash "$ENS" >>"$MOCK/out.log" 2>&1
  t "fallback: ensure restarts watch"    waitn 1
  p2="$(pids)"
  t "fallback: restarted pid differs"    neq "$p2" "$p1"
  t "fallback: watch has own session"    eq "$(ps -o sid= -p "$p2" | tr -d ' ')" "$p2"
  t "fallback: log file exists"          test -f "$LOGD/watch.log"
  # 空白を含む checkout: 連続して実行しても、ensure は自分が起動した見張りを見失わず 1 本のまま
  # （Issue #808。引数は /proc の NUL 区切りで読み、パスは物理パスで比べる）
  S="$ROOT/with space/repo/scripts"; mkdir -p "$S"
  cp "$ROOT/repo/scripts/"{conductor-watch.sh,agent-gh,patrol-state-update} "$S/"; cp "$ENS" "$S/conductor-watch-ensure.sh"
  bash "$S/conductor-watch-ensure.sh" >"$MOCK/space1.log" 2>&1
  t "fallback: spaces in path: first ensure exits 0"    eq "$?" "0"
  bash "$S/conductor-watch-ensure.sh" >"$MOCK/space2.log" 2>&1
  t "fallback: spaces in path: second ensure exits 0"   eq "$?" "0"
  t "fallback: spaces in path: one watch"               waitn 1 'with space/repo/.*conductor-watch\.sh'
  t "fallback: spaces in path: second run adopts it"    has "$MOCK/space2.log" "すでに動いています"
  sp="$(pgrep -f 'with space/repo/.*conductor-watch\.sh' | head -1)"; kill "$sp" 2>/dev/null
  for i in 1 2 3 4 5 6 7 8 9 10; do kill -0 "$sp" 2>/dev/null || break; sleep 0.2; done
fi
EOF

  export ROOT
  for mode in fallback systemd; do
    out="$(unshare -U -r -m -p -f --mount-proc bash "$ROOT/ns-run.sh" "$mode" 2>&1)"
    printf '%s\n' "$out" | sed "s/^/[$mode] /"
    if grep -q '^FAIL' <<<"$out" || [[ -z "$out" ]]; then
      failures+=("behavior-$mode")
      [[ -z "$out" ]] && echo "[$mode] no output from namespace run"
    fi
  done
fi

if ((${#failures[@]})); then
  printf '\n%d failure(s): %s\n' "${#failures[@]}" "${failures[*]}"
  exit 1
fi
printf '\nall checks passed\n'
