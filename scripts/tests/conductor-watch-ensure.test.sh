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
# 起動した見張りや空白を含む checkout の見張りを見失わないこと、逆に conductor-watch の試験が
# 止めるのに使う kill の形では ensure の管理対象の見張りが残ることを確かめる。
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
           "$ROOT/state" "$ROOT/config" "$ROOT/home" "$ROOT/mock"
  cp "$REPO/scripts/conductor-watch.sh" "$ENSURE" "$ROOT/repo/scripts/"
  cp "$REPO/scripts/patrol-state-update" "$ROOT/repo/scripts/"
  # conductor-watch の試験が見張りを止めるときの kill の形（モックの sleep）を、試験のファイルから取り出す
  cp "$REPO/scripts/tests/conductor-watch-slots.test.sh" "$ROOT/slots.test.sh"

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
  for t in bash sh env setsid pgrep ps mkdir cp chmod dirname python3 cat grep sed tr cut wc sleep flock kill ls head tail awk date; do
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
# fake repo は git 管理でなく最小 PATH に git も無いため、patrol-state の置き場は
# 環境変数で明示する（他の conductor-watch テストと同じやり方）。
export PELWAY_PATROL_STATE="$ROOT/state/pelway/patrol-state.json"
if [[ "$MODE" == systemd ]]; then
  export PATH="$ROOT/bin:$ROOT/systemd-bin:$ROOT/sysbin"
else
  export PATH="$ROOT/bin:$ROOT/sysbin"
fi
ENS="$ROOT/repo/scripts/conductor-watch-ensure.sh"
SLT="$ROOT/slots.test.sh"
LOGD="$ROOT/state/pelway/conductor-watch"
UNIT="$ROOT/config/systemd/user/pelway-conductor-watch.service"

count() { pgrep -f 'conductor-watch\.sh' | wc -l | tr -d ' '; }
pids()  { pgrep -f 'conductor-watch\.sh' | head -1; }
wait0() { local i; for i in 1 2 3 4 5 6 7 8 9 10; do [[ "$(count)" == 0 ]] && return 0; sleep 0.2; done; return 1; }
waitn() { local i; for i in {1..15}; do [[ "$(count)" == "$1" ]] && return 0; sleep 0.2; done; [[ "$(count)" == "$1" ]]; }
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
t_test_kills() {  # <mode>: conductor-watch の試験が見張りを止めるときの kill の形を、ensure の管理対象の見張り（p1）の横で実行して、p1 が残ること
  local d="$ROOT/elsewhere/tmp.d4E5f6/repo/scripts" tw i
  mkdir -p "$d" "$MOCK/tbin"
  # 試験のファイルから、モックの sleep（SLEEP_MAX 回目に親へ TERM を送る）をそのまま取り出す
  awk '/^cat > "\$ROOT\/bin\/sleep" <<.EOF.$/{f=1; next} f && /^EOF$/{exit} f' "$SLT" >"$MOCK/tbin/sleep"
  chmod +x "$MOCK/tbin/sleep"
  t "$1: mock sleep taken from the conductor-watch test"  test -x "$MOCK/tbin/sleep"
  t "$1: mock sleep sends TERM to its parent (as in the test file)"  has "$MOCK/tbin/sleep" 'kill -TERM "\$PPID"'
  printf '#!/usr/bin/env bash\nwhile :; do sleep 30; done\n' >"$d/conductor-watch.sh"
  MOCK="$MOCK" SLEEP_MAX=1 PATH="$MOCK/tbin:$PATH" bash "$d/conductor-watch.sh" >/dev/null 2>&1 &
  tw=$!
  wait_gone "$tw" && wait "$tw" 2>/dev/null
  t "$1: test-shaped watch stopped by its mock sleep"   eq "$?" "143"
  # 試験の EXIT trap と片付けの形: kill "$WPID" と kill -KILL -- -"$WPID"
  bash "$d/conductor-watch.sh" >/dev/null 2>&1 & tw=$!
  kill "$tw" 2>/dev/null; wait "$tw" 2>/dev/null
  bash "$d/conductor-watch.sh" >/dev/null 2>&1 & tw=$!
  kill -KILL -- -"$tw" 2>/dev/null; kill -KILL "$tw" 2>/dev/null; wait "$tw" 2>/dev/null
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
  t "fallback: spaces in path: one watch"               eq "$(pgrep -fc 'with space/repo/.*conductor-watch\.sh')" "1"
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
