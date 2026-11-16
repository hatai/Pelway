#!/usr/bin/env bash
# scripts/agent-sandbox/ のテスト（Issue #151）。sandbox の書込拒否と
# プラグインの実体パス検査、devin フックのネスト分離を確かめる。
set -uo pipefail

TOP="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRIPT="$TOP/agent-sandbox/sandbox-shell"
PLUGIN="$TOP/agent-sandbox/opencode-plugin/index.js"
failures=()

check() { # check <name> <actual> <expected>
  if [[ "$2" != "$3" ]]; then
    failures+=("$1: got '$2', want '$3'")
  fi
}

if ! command -v bwrap >/dev/null 2>&1; then
  echo "SKIP: bwrap not installed"
  exit 0
fi

# agent-sandbox の内側では実行しない（ホスト専用）。フィクスチャを
# ~/.t3/worktrees/ 直下と $HOME 直下に作る必要があり、sandbox 内では
# どちらも read-only なため。検出は sandbox が渡す PELWAY_AGENT_SANDBOX で行う。
if [[ -n "${PELWAY_AGENT_SANDBOX:-}" ]]; then
  echo "SKIP: agent-sandbox の内側（ホスト専用テスト: ~/.t3/worktrees と \$HOME 直下へのフィクスチャ作成が必要）"
  exit 0
fi

# テスト用の本体リポジトリ + linked worktree を作る
base="$(mktemp -d)"
trap 'chmod -R u+w "$base" 2>/dev/null; rm -rf "$base"' EXIT
main="$base/main"
wt="$base/wt-1"
mkdir -p "$main"
git -C "$main" init -q
git -C "$main" -c user.email=t@t -c user.name=t commit -qm init --allow-empty
git -C "$main" worktree add -q "$wt" -b wt-branch

# リポジトリ同梱のマスク対象と同じレイアウトを作る（存在するパスは
# ro-bind される。bwrap は無いパスのマウントポイントをホスト側に作る）。
printf '{}\n' > "$wt/opencode.json"
mkdir -p "$wt/.opencode" "$wt/.devin"
printf '{}\n' > "$wt/.devin/config.json"

cd "$wt"
export PELWAY_SBX_ROOT="$wt"
export T3_ACP_MCP_AUTHORIZATION="secret-token"

run() { PELWAY_SBX_ROOT="$wt" "$SCRIPT" -c "$1" 2>&1; }

# 1. worktree 内は書き込み可
out="$(run 'echo x > in-wt.txt && echo WTOK')"
check "worktree write" "$out" "WTOK"
[[ -f "$wt/in-wt.txt" ]] || failures+=("worktree write: file missing")

# 2. $HOME 直下は read-only
out="$(run 'echo x > "$HOME/sbx-test-home" && echo BAD || echo DENIED' | tail -1)"
check "home write" "$out" "DENIED"
[[ -f "$HOME/sbx-test-home" ]] && failures+=("home write: file created")

# 3. /etc も read-only
out="$(run 'echo x > /etc/sbx-test-etc && echo BAD || echo DENIED' | tail -1)"
check "etc write" "$out" "DENIED"

# 4. 秘匿パスが隠れる（tmpfs マスクまたは /mnt 自体の不在）
out="$(run 'ls -A "$HOME/.ssh" | head -1; echo ssh-done')"
check "ssh masked" "$out" "ssh-done"
# /etc/resolv.conf が symlink のとき（WSL では /mnt/wsl/resolv.conf）、実体が
# 正本パスに ro-bind されるため /mnt/wsl が見える。見えてよいのはリンク先だけで、
# /mnt/c 等の Windows 側は見えない。
out="$(run 'ls /mnt/c 2>&1 | head -1')"
check "mnt windows absent" "$out" "ls: cannot access '/mnt/c': No such file or directory"
out="$(run 'ls -A "$HOME/.config/gh" 2>/dev/null | head -1; echo gh-done')"
check "gh config masked" "$out" "gh-done"

# 5. sudo は使えない（bwrap の no-new-privileges）
out="$(run 'sudo -n true 2>/dev/null; echo rc=$?')"
check "sudo denied" "$out" "rc=1"

# 6. 共有 git dir にはコミットできるが config/hooks は read-only
out="$(run 'git commit --allow-empty -qm t && git rev-parse HEAD >/dev/null && echo CTOK')"
check "git commit" "$out" "CTOK"
common="$(git -C "$wt" rev-parse --git-common-dir)"
out="$(run "echo probe >> '$common/config' && echo BAD || echo DENIED" | tail -1)"
check "git config ro" "$out" "DENIED"
out="$(run "echo probe > '$common/hooks/sbx-hook' && echo BAD || echo DENIED" | tail -1)"
check "git hooks ro" "$out" "DENIED"

# 7. sandbox 無効化に使える worktree 内パスは ro-bind で変更不可
out="$(run 'echo x > opencode.json && echo BAD || echo DENIED' | tail -1)"
check "opencode.json ro" "$out" "DENIED"
out="$(run 'echo x > .opencode/opencode.json && echo BAD || echo DENIED' | tail -1)"
check ".opencode ro" "$out" "DENIED"
out="$(run 'echo x > .devin/config.json && echo BAD || echo DENIED' | tail -1)"
check ".devin ro" "$out" "DENIED"

# 8. 環境変数: T3 接続変数を落とし、mbx 無効化フラグ・sandbox 検出用の目印・
# sandbox 専用キャッシュ（XDG_CACHE_HOME）を渡す
out="$(run 'echo "auth=[${T3_ACP_MCP_AUTHORIZATION:-}] mbx=${MISE_RUST_MR_BOXINGTON:-} sbx=${PELWAY_AGENT_SANDBOX:-} xdg=${XDG_CACHE_HOME:-}"')"
check "env scrub" "$out" "auth=[] mbx=0 sbx=1 xdg=$HOME/.cache/agent-sandbox"

# sandbox 専用キャッシュは書き込み可、ホストの ~/.cache/mbx は read-only のまま
# （Issue #151: ハードリンクキャッシュ経由の書き換え経路は開けない）
out="$(run 'touch "$XDG_CACHE_HOME/probe" && rm -f "$XDG_CACHE_HOME/probe" && echo XDG-WOK')"
check "xdg cache writable" "$out" "XDG-WOK"
out="$(run 'touch "$HOME/.cache/mbx/probe" 2>/dev/null && echo BAD || echo DENIED')"
check "host mbx cache ro" "$out" "DENIED"
out="$(run 'touch "$HOME/.cache/hk/probe" 2>/dev/null && echo BAD || echo DENIED')"
check "host hk cache ro" "$out" "DENIED"

# MISE_RUST_MR_BOXINGTON=0 を渡してもコマンドラッパー生成済みの cargo は mbx を
# 通る。その mbx が XDG_CACHE_HOME 配下の書き込み可キャッシュで lock を取れる
# ことを、依存を持たない新規プロジェクトの cargo check で確かめる（Issue #325）。
# 固定名は使わず mktemp -d で専用ディレクトリを作り、自分の作成物だけを消す。
out="$(run 'd="$(mktemp -d /tmp/sbx-mbx-probe.XXXXXX)" && trap "rm -rf \"$d\"" EXIT && cd "$d" && cargo new probe -q 2>/dev/null && cd probe && cargo check -q 2>&1; rc=$?; echo "cargorc=$rc"')"
check "cargo via mbx" "$(tail -1 <<<"$out")" "cargorc=0"
[[ "$out" == *"Read-only file system"* ]] && failures+=("cargo via mbx: mbx cache is read-only")

# DNS: /etc/resolv.conf が symlink のとき実体が正本パスに bind され、sandbox 内で
# 名前解決できる（Issue #325。実ファイルなら /etc の ro-bind で届く）
out="$(run 'getent hosts api.github.com >/dev/null 2>&1 && echo DNS-OK || echo DNS-FAIL')"
check "dns resolves" "$out" "DNS-OK"
if [[ -L /etc/resolv.conf ]]; then
  rtarget="$(readlink -f /etc/resolv.conf)"
  out="$(run "test -r '$rtarget' && echo RESOLV-OK")"
  check "resolv.conf symlink target reachable" "$out" "RESOLV-OK"
fi

# 9. /tmp は書き込み可
out="$(run 'echo x > /tmp/sbx-test-tmp && cat /tmp/sbx-test-tmp && rm -f /tmp/sbx-test-tmp')"
check "tmp write" "$out" "x"

# 10. mbx の target シンボリックリンクは実ディレクトリに置き換わる
ln -s "$base/mbx-cache" "$wt/target"
run ':' >/dev/null
[[ -L "$wt/target" ]] && failures+=("mbx target: still symlink")
[[ -d "$wt/target" ]] || failures+=("mbx target: dir missing")

# 11. --wrap モードでもコマンドが動く
out="$(cd "$wt" && PELWAY_SBX_ROOT= "$SCRIPT" --wrap -- bash -c 'echo WRAPOK; echo x > "$HOME/sbx-wrap-home" 2>&1 || echo DENIED')"
check "wrap run" "$(echo "$out" | head -1)" "WRAPOK"
check "wrap home write" "$(echo "$out" | tail -1)" "DENIED"
[[ -f "$HOME/sbx-wrap-home" ]] && failures+=("wrap: home file created")

# 12. opencode プラグイン: 実体パス検査（symlink 越しの脱走を拒否）
if command -v node >/dev/null 2>&1; then
  pwt="$HOME/.t3/worktrees/sbx-plug-test"
  rm -rf "$pwt" && mkdir -p "$pwt" "$HOME/sbx-plug-out"
  ln -s /mnt "$pwt/mnt-link"
  ln -s "$HOME/sbx-plug-out" "$pwt/out-link"
  out="$(node - "$PLUGIN" "$pwt" <<'NODE'
const hooks = {}
const plugin = (await import(process.argv[2])).default
await plugin.setup({
  location: { directory: process.argv[3] },
  shell: { hook: async (n, f) => { hooks[n] = f } },
  tool: { hook: async (n, f) => { hooks["t:" + n] = f } },
})
const t = hooks["t:execute.before"]
const allows = async (tool, fp) => {
  try { await t({ tool, input: { filePath: fp } }); return true }
  catch { return false }
}
const wt = process.argv[3]
const bad = []
if (await allows("read", `${wt}/mnt-link/c`)) bad.push("read via deny symlink")
if (await allows("write", `${wt}/out-link/x`)) bad.push("write via dir symlink")
if (!(await allows("write", `${wt}/x`))) bad.push("write inside")
if (!(await allows("read", `${wt}/x`))) bad.push("read inside")
console.log(bad.length ? "FAIL " + bad.join(",") : "OK")
NODE
)"
  check "plugin file-tool" "$out" "OK"
  rm -rf "$pwt" "$HOME/sbx-plug-out"
fi

# 13. --hook: exec を single-quote 付き -c 経由に書き換える（解釈不能なら block）
out="$(echo '{"tool_input":{"command":"echo x"}}' | "$SCRIPT" --hook)"
[[ "$out" == *"$SCRIPT -c 'echo x'"* ]] || failures+=("hook rewrite: got '$out'")
[[ "$(echo bad | "$SCRIPT" --hook)" == *'"decision": "block"'* ]] || failures+=("hook block")

# 14. hook 出力の $(...) が outer シェルで評価されない（二重引用符だと脱走する）
cmd="$(echo '{"tool_input":{"command":"echo $(mkdir -p ~/.local/share/devin/evil)"}}' \
  | "$SCRIPT" --hook \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["hookSpecificOutput"]["updatedInput"]["command"])')"
(cd "$wt" && PELWAY_SBX_ROOT="$wt" "$SCRIPT" --wrap -- sh -c "$cmd" >/dev/null)
[[ ! -d "$HOME/.local/share/devin/evil" ]] || failures+=("hook outer-eval: $(...) が外側で評価された")

# 15. wrap 内でネストした shell は devin 資格情報を隠す（outer では見える）
mkdir -p "$HOME/.local/share/devin" && touch "$HOME/.local/share/devin/sbx-probe"
trap 'rm -rf "$HOME/.local/share/devin/evil"; rm -f "$HOME/.local/share/devin/sbx-probe"; chmod -R u+w "$base" 2>/dev/null; rm -rf "$base"' EXIT
out="$(cd "$wt" && "$SCRIPT" --wrap -- bash -c "test -f ~/.local/share/devin/sbx-probe && echo OUTER; '$SCRIPT' -c 'test -f ~/.local/share/devin/sbx-probe && echo INNER-SEE || echo INNER-HIDDEN'")"
check "wrap outer sees devin" "$(head -1 <<<"$out")" "OUTER"
check "nested hides devin" "$(tail -1 <<<"$out")" "INNER-HIDDEN"

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "PASS: agent-sandbox"
