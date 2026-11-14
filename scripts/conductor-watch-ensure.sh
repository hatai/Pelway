#!/usr/bin/env bash
# 指揮役の見張り conductor-watch.sh の起動入口（冪等）。起動・復旧は常にこのスクリプトに任せる。
# systemd --user が使える環境では unit（pelway-conductor-watch.service、Restart=always）経由で
# 起動し、プロセスが死んでもユーザーマネージャが自動で再起動する。使えない環境では
# setsid で呼び出し元のセッションから切り離して直接起動する（nohup & のように起動元の
# シェルの終了に道連れにされることを防ぐ）。どちらの経路でも出力は
# $XDG_STATE_HOME/pelway/conductor-watch/watch.log に追記される。
# 「正しい形」でない見張り（unit 外・セッション切り離し無し）は、kill しても復旧しないまま
# 残るため、その形のプロセスに置き換える。正しい形で動いているものはそのまま採用し、
# 二重起動しない。起動対象は worktree から呼ばれても worktree の消去で死なないよう、
# 主 checkout（git の共通トップ）の conductor-watch.sh を優先する。
# 使い方: scripts/conductor-watch-ensure.sh
set -u -o pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
UNIT="pelway-conductor-watch.service"
STATE="${XDG_STATE_HOME:-$HOME/.local/state}/pelway/conductor-watch"
mkdir -p "$STATE"

WATCH="$HERE/conductor-watch.sh"
if command -v git >/dev/null 2>&1; then
  common="$(git -C "$HERE" rev-parse --path-format=absolute --git-common-dir 2>/dev/null || true)"
  main_watch="${common%/.git}/scripts/conductor-watch.sh"
  [[ -n "$common" && -f "$main_watch" ]] && WATCH="$main_watch"
fi

command -v pgrep >/dev/null 2>&1 || { echo "conductor-watch-ensure: pgrep がありません" >&2; exit 1; }

# ensure 自身（conductor-watch-ensure.sh）はパターンに一致しないが、呼び出し方によっては
# 自分や直近の親が紛れ込む可能性があるため除いて数える。
watch_pids() { pgrep -f "conductor-watch\.sh" | grep -vx -e "$$" -e "${PPID:-0}" || true; }

ppid_of() { ps -o ppid= -p "$1" 2>/dev/null | tr -d ' '; }
is_leader() { [[ -n "$1" ]] && [[ "$(ps -o sid= -p "$1" 2>/dev/null | tr -d ' ')" == "$1" ]]; }

# unit（$1 = MainPID）に属さない見張りの pid。unit の子（起動直後の一時的な subshell）は残す。
foreign_pids() {
  local p
  for p in $(watch_pids); do
    [[ "$p" == "$1" || "$(ppid_of "$p")" == "$1" ]] || echo "$p"
  done
}

# セッションリーダーの見張りの pid（setsid で切り離して起きた「正しい形」）。
leader_pids() {
  local p
  for p in $(watch_pids); do is_leader "$p" && echo "$p"; done
}

# セッションを切り離していない見張りの pid。リーダーの子（一時的な subshell）は残す。
fragile_pids() {
  local p ppid
  for p in $(watch_pids); do
    is_leader "$p" && continue
    ppid="$(ppid_of "$p")"
    { is_leader "$ppid" && leader_pids | grep -qx "$ppid"; } && continue
    echo "$p"
  done
}

kill_pids() {
  local p
  for p in $1; do kill "$p" 2>/dev/null || true; done
  sleep 0.5
  for p in $1; do kill -0 "$p" 2>/dev/null && kill -9 "$p" 2>/dev/null; done
  return 0
}

have_systemd() {
  command -v systemctl >/dev/null 2>&1 && systemctl --user list-units >/dev/null 2>&1
}

# 確認から起動までを排他にし、同時実行による二重起動を防ぐ。
if command -v flock >/dev/null 2>&1; then
  exec 9>"$STATE/ensure.lock"
  flock 9
fi

# systemd が使える環境では unit を常に最新化する。呼ばれるたびに書き直すことで、
# checkout の移動や定義の変更に古い unit が残ったままにならない。
mode=direct
if have_systemd; then
  mode=systemd
  udir="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
  mkdir -p "$udir"
  cat > "$udir/$UNIT" <<EOF
[Unit]
Description=Pelway conductor-watch（指揮役の見張り）

[Service]
ExecStart=$WATCH
Restart=always
RestartSec=5
StandardOutput=append:$STATE/watch.log
StandardError=append:$STATE/watch.log

[Install]
WantedBy=default.target
EOF
  systemctl --user daemon-reload
fi

if [[ "$mode" == systemd ]]; then
  unit_pid="$(systemctl --user show "$UNIT" -p MainPID --value 2>/dev/null || true)"
  unit_pid="${unit_pid:-0}"
  foreign="$(foreign_pids "$unit_pid")"
  if [[ -n "$foreign" ]]; then
    # unit 外の見張り（直接起動・別 unit など）を置き換える。このまま採用すると、
    # それが止まったときに Restart=always が働かず監視が空白になる。
    kill_pids "$foreign"
    echo "unit 外の見張り（pid: $(echo "$foreign" | tr '\n' ' ')）を unit $UNIT に置き換えます"
  fi
  if systemctl --user is-active --quiet "$UNIT" && [[ "$unit_pid" != "0" ]]; then
    echo "conductor-watch.sh はすでに unit $UNIT で動いています（pid: $unit_pid）"
    exit 0
  fi
  # 見張りが t3・fnox・scripts/agent-gh を PATH から見つけられるよう、現在の環境変数を
  # ユーザーマネージャに取り込む（unit ファイルには環境を書き込まない）。
  vars=(PATH)
  [[ -n "${XDG_STATE_HOME:-}" ]] && vars+=(XDG_STATE_HOME)
  systemctl --user import-environment "${vars[@]}" 2>/dev/null || true
  systemctl --user start "$UNIT"
else
  # セッションを切り離していない見張り（nohup & など、起動元に道連れにされる形）は
  # 切り離した形に置き換える。正しく切り離された見張り（セッションリーダー）は採用する。
  frag="$(fragile_pids)"
  if [[ -n "$frag" ]]; then
    kill_pids "$frag"
    echo "起動元と同じセッションの見張り（pid: $(echo "$frag" | tr '\n' ' ')）を切り離した形に置き換えます"
  fi
  leaders="$(leader_pids)"
  if [[ -n "$leaders" ]]; then
    keep="$(echo "$leaders" | head -1)"
    extra="$(echo "$leaders" | tail -n +2)"
    [[ -n "$extra" ]] && kill_pids "$extra"
    echo "conductor-watch.sh はすでに動いています（pid: $keep）"
    exit 0
  fi
  # fd 9（ロック）を見張りに引き継がせない。引き継ぐと見張りがロックを持ち続け、
  # 次の ensure が flock で永久に待つ。
  setsid -f "$WATCH" 9>&- </dev/null >>"$STATE/watch.log" 2>&1
fi

pids=""
for _ in 1 2 3 4 5 6 7 8 9 10; do
  pids="$(watch_pids)"
  [[ -n "$pids" ]] && break
  sleep 0.5
done
if [[ -n "$pids" ]]; then
  if [[ "$mode" == systemd ]]; then
    echo "conductor-watch.sh を unit $UNIT で起動しました（pid: $(echo "$pids" | tr '\n' ' ')）"
  else
    echo "conductor-watch.sh をセッション切り離しで起動しました（pid: $(echo "$pids" | tr '\n' ' ')、ログ: $STATE/watch.log）"
  fi
else
  echo "conductor-watch-ensure: conductor-watch.sh の起動を確かめられませんでした（mode: $mode、対象: $WATCH）" >&2
  exit 1
fi
