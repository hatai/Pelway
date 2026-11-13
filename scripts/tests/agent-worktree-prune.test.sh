#!/usr/bin/env bash
# scripts/agent-worktree-prune のテスト。一時ディレクトリに origin・本体・worktree を作り、偽の gh で PR の状態を返す。
set -uo pipefail

SCRIPT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/agent-worktree-prune"
ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT
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

g() { git -c user.name=t -c user.email=t@example.com "$@"; }

# 毎回まっさらな origin・本体・worktree を作る
setup() {
  rm -rf "$ROOT/origin.git" "$ROOT/main" "$ROOT/wt" "$ROOT/other"
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

run() {
  (cd "$ROOT/main" && PATH="$BIN:$PATH" PELWAY_T3_WORKTREES="$ROOT/wt" bash "$SCRIPT" "$@")
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
run --yes >/dev/null 2>&1
check "merged worktree removed" gone a-merged
check "merged branch deleted" branch_gone a-merged

# 2. 取り込まれていず PR も無いものは残す
setup
add_unmerged b-open
run --yes >/dev/null 2>&1
check "unmerged worktree kept" kept b-open

# 3. 変更が残っているものは、取り込み済みでも残す
setup
add_merged c-dirty
echo x >"$ROOT/wt/c-dirty/new.txt"
run --yes >/dev/null 2>&1
check "dirty worktree kept" kept c-dirty

# 4. 旧プラグインのファイルだけなら変更と数えない
setup
add_merged d-plugin
mkdir -p "$ROOT/wt/d-plugin/.opencode/plugins"
echo x >"$ROOT/wt/d-plugin/.opencode/plugins/pelway-agent-notify.ts"
run --yes >/dev/null 2>&1
check "old plugin file is not a change" gone d-plugin

# 5. squash などで取り込まれ、PR がマージ済みなら消す
setup
add_unmerged e-squashed
FAKE_PRS='[{"headRefName":"e-squashed","state":"MERGED"}]' run --yes >/dev/null 2>&1
check "merged PR worktree removed" gone e-squashed
check "merged PR branch deleted" branch_gone e-squashed

# 6. PR が閉じただけなら worktree は消すがブランチは残す
setup
add_unmerged f-closed
FAKE_PRS='[{"headRefName":"f-closed","state":"CLOSED"}]' run --yes >/dev/null 2>&1
check "closed PR worktree removed" gone f-closed
check "closed PR branch kept" branch_kept f-closed

# 7. 同じブランチで開いている PR があれば残す
setup
add_unmerged g-reopened
FAKE_PRS='[{"headRefName":"g-reopened","state":"CLOSED"},{"headRefName":"g-reopened","state":"OPEN"}]' run --yes >/dev/null 2>&1
check "open PR worktree kept" kept g-reopened

# 8. 確認に n と答えたら何も消さない
setup
add_merged h-ask
echo n | run >/dev/null 2>&1
check "answer n removes nothing" kept h-ask

# 9. T3 の worktree の置き場所の外にあるものは触らない
setup
mkdir -p "$ROOT/other"
g -C "$ROOT/main" worktree add -q -b i-outside "$ROOT/other/i-outside" main
run --yes >/dev/null 2>&1
check "worktree outside root untouched" test -d "$ROOT/other/i-outside"

# 10. --dry-run は一覧を出すだけ
setup
add_merged j-dry
out="$(run --dry-run 2>&1)"
check "dry-run removes nothing" kept j-dry
check "dry-run lists candidate" grep -q "j-dry" <<<"$out"

if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}"
  exit 1
fi
printf '\nall passed\n'
