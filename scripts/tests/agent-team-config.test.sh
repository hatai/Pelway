#!/usr/bin/env bash
# agent-team.toml と scripts/agent-team-config のテスト（Issue #713）。
# リポジトリの agent-team.toml が validate を通ること、hatai の枠が `slots` で
# 読めること、登録情報のインスタンス名・モデルが運用中の名前と同じことを確かめる。
# 合成データで、上限が 1 未満・整数でない、優先順位に使える担当に無い担当、
# 実装・レビューの使える担当が 0 種、TOML として読めない、共有部分に無い担当 id の
# 各場合に検証が非 0 で止まることを確かめる。
# このテストが scripts/script-test.sh に収集される（= mise run verify で走る）ことと、
# AGENTS.md の共有層の列挙に agent-team.toml があることも確かめる。
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT
failures=()

CFG="$REPO/scripts/agent-team-config"

check() {
  local name="$1"; shift
  if "$@"; then printf 'ok   %s\n' "$name"; else printf 'FAIL %s\n' "$name"; failures+=("$name"); fi
}
eq() { [[ "$1" == "$2" ]]; }
has() { grep -q -- "$2" "$1"; }

# 4 担当の登録情報（有効な合成データの共有部分）
base_toml() {
  cat <<'EOF'
[agents.opencode]
instance = { implement = "opencode", investigate = "opencode" }
models = ["zai-coding-plan/glm-5.4"]
effort = { option = "variant", raise = ["low", "default", "high", "max"] }
assign_label = ""

[agents.devin]
instance = { implement = "acpRegistry_devin", review = "devin_review" }
models = ["swe-2-1-high"]
effort = { option = "thought_level", raise = ["medium", "high", "max"] }
assign_label = "agent:devin"

[agents.codex]
instance = { review = "codex_review" }
models = ["gpt-6.2-sol", "gpt-6.1-terra", "gpt-6-luna"]
effort = { option = "reasoningEffort", raise = ["low", "medium", "high", "xhigh", "max"] }
assign_label = ""

[agents.claude]
instance = { review = "claude_review" }
models = ["claude-opus-5-5". "claude-sonnet-5-5", "claude-haiku-5-5"]
effort = { option = "effort", raise = ["low", "medium", "high", "xhigh", "max"] }
assign_label = ""
EOF
}

# write_case <file> — 共有部分 + stdin の開発者の表
write_case() { base_toml > "$1"; cat >> "$1"; }

# v_run <file> — validate を走らせ、標準エラーを $ROOT/err に残す
v_run() { bash "$CFG" validate "$1" 2> "$ROOT/err" > /dev/null; }

# 試験 1: リポジトリの agent-team.toml が validate を通る。
out=""
out="$(bash "$CFG" validate 2>&1)"; rc=$?
check "1: repo agent-team.toml validates (rc=0)" eq "$rc" "0"

# 試験 2: slots hatai が opencode 3 / devin 6 の 2 行を出す。
out="$(bash "$CFG" slots hatai 2>&1)"; rc=$?
check "2: slots hatai exits 0" eq "$rc" "0"
check "2: slots hatai prints opencode 3" eq "$(printf '%s\n' "$out" | grep -cx 'opencode 3')" "1"
check "2: slots hatai prints devin 6" eq "$(printf '%s\n' "$out" | grep -cx 'devin 6')" "1"
check "2: slots hatai prints exactly 2 lines" eq "$(printf '%s\n' "$out" | grep -c .)" "2"

# 試験 3: 登録情報のインスタンス名とモデルが運用中の名前と同じ。役割ごとの
# インスタンス名の表全体と、モデルの一覧全体を各担当で比較する。
pins="$(python3 -I - "$REPO/agent-team.toml" <<'PY' 2>&1
import sys, tomllib
d = tomllib.load(open(sys.argv[1], "rb"))
a = d["agents"]
want = {
    "opencode": (
        {"implement": "opencode", "investigate": "opencode"},
        ["zai-coding-plan/glm-5.4"],
    ),
    "devin": (
        {"implement": "acpRegistry_devin", "review": "devin_review"},
        ["swe-2-1-high"],
    ),
    "codex": (
        {"review": "codex_review"},
        ["gpt-6.2-sol", "gpt-6.1-terra", "gpt-6-luna"],
    ),
    "claude": (
        {"review": "claude_review"},
        ["claude-opus-5-5". "claude-sonnet-5-5", "claude-haiku-5-5"],
    ),
}
bad = [
    f"{n}: instance={a[n]['instance']} models={a[n]['models']}"
    for n, (inst, models) in want.items()
    if a[n]["instance"] != inst or a[n]["models"] != models
]
print("\n".join(bad) if bad else "ok")
PY
)"
check "3: instance names and models match current ops" eq "$pins" "ok"

# 試験 4: 合成データの正常系は通る。
write_case "$ROOT/ok.toml" <<'EOF'
[developers.dev]
use = ["opencode", "devin", "codex", "claude"]
[developers.dev.slots]
opencode = 1
devin = 2
[developers.dev.review_limits]
codex = 1
devin = 1
claude = 1
[developers.dev.priority.implement]
normal = ["devin", "opencode"]
[developers.dev.priority.review]
normal = ["codex", "devin"]
EOF
v_run "$ROOT/ok.toml"; check "4: minimal valid file passes" eq "$?" "0"

# 試験 5: 上限が 0 のとき非 0 で止まり、違反したキーを伝える。
write_case "$ROOT/zero.toml" <<'EOF'
[developers.dev]
use = ["opencode", "devin", "codex", "claude"]
[developers.dev.slots]
opencode = 0
devin = 2
[developers.dev.review_limits]
codex = 1
devin = 1
claude = 1
[developers.dev.priority.implement]
normal = ["devin", "opencode"]
[developers.dev.priority.review]
normal = ["codex", "devin"]
EOF
v_run "$ROOT/zero.toml"; rc=$?
check "5: limit 0 fails" eq "$rc" "1"
check "5: names the violating key" has "$ROOT/err" "developers.dev.slots.opencode"

# 試験 6: 上限が整数でない（float・文字列・真偽値）とき非 0 で止まり、キーを伝える。
write_case "$ROOT/float.toml" <<'EOF'
[developers.dev]
use = ["opencode", "devin", "codex", "claude"]
[developers.dev.slots]
opencode = 1.5
devin = 2
[developers.dev.review_limits]
codex = 1
devin = 1
claude = 1
[developers.dev.priority.implement]
normal = ["devin", "opencode"]
[developers.dev.priority.review]
normal = ["codex", "devin"]
EOF
v_run "$ROOT/float.toml"; rc=$?
check "6: float limit fails" eq "$rc" "1"
check "6: names the violating key" has "$ROOT/err" "developers.dev.slots.opencode"
write_case "$ROOT/str.toml" <<'EOF'
[developers.dev]
use = ["opencode", "devin", "codex", "claude"]
[developers.dev.slots]
opencode = "3"
devin = 2
[developers.dev.review_limits]
codex = 1
devin = 1
claude = 1
[developers.dev.priority.implement]
normal = ["devin", "opencode"]
[developers.dev.priority.review]
normal = ["codex", "devin"]
EOF
v_run "$ROOT/str.toml"; check "6: string limit fails" eq "$?" "1"
write_case "$ROOT/bool.toml" <<'EOF'
[developers.dev]
use = ["opencode", "devin", "codex", "claude"]
[developers.dev.slots]
opencode = true
devin = 2
[developers.dev.review_limits]
codex = 1
devin = 1
claude = 1
[developers.dev.priority.implement]
normal = ["devin", "opencode"]
[developers.dev.priority.review]
normal = ["codex", "devin"]
EOF
v_run "$ROOT/bool.toml"; check "6: bool limit fails" eq "$?" "1"
write_case "$ROOT/rfloat.toml" <<'EOF'
[developers.dev]
use = ["opencode", "devin", "codex", "claude"]
[developers.dev.slots]
opencode = 1
devin = 2
[developers.dev.review_limits]
codex = 1.5
devin = 1
claude = 1
[developers.dev.priority.implement]
normal = ["devin", "opencode"]
[developers.dev.priority.review]
normal = ["codex", "devin"]
EOF
v_run "$ROOT/rfloat.toml"; rc=$?
check "6: float review limit fails" eq "$rc" "1"
check "6: names the review limit key" has "$ROOT/err" "developers.dev.review_limits.codex"

# 試験 7: 優先順位に使える担当に無い担当を書いたとき非 0。
write_case "$ROOT/badprio.toml" <<'EOF'
[developers.dev]
use = ["opencode", "codex", "claude"]
[developers.dev.slots]
opencode = 1
[developers.dev.review_limits]
codex = 1
claude = 1
[developers.dev.priority.implement]
normal = ["opencode"]
[developers.dev.priority.review]
normal = ["codex", "devin"]
EOF
v_run "$ROOT/badprio.toml"; rc=$?
check "7: priority outside use fails" eq "$rc" "1"
check "7: names the priority key" has "$ROOT/err" "devin"

# 試験 8: 実装に使える担当が 0 種、レビューに使える担当が 0 種の表は非 0。
write_case "$ROOT/noimpl.toml" <<'EOF'
[developers.dev]
use = ["codex", "claude"]
[developers.dev.slots]
[developers.dev.review_limits]
codex = 1
claude = 1
[developers.dev.priority.implement]
normal = ["codex"]
[developers.dev.priority.review]
normal = ["codex", "claude"]
EOF
v_run "$ROOT/noimpl.toml"; rc=$?
check "8: no implement-capable agent fails" eq "$rc" "1"
check "8: names the use check for implement" has "$ROOT/err" "developers.dev.use: 実装の役割を持つ担当が 1 つも無い"
write_case "$ROOT/norev.toml" <<'EOF'
[developers.dev]
use = ["opencode"]
[developers.dev.slots]
opencode = 1
[developers.dev.review_limits]
[developers.dev.priority.implement]
normal = ["opencode"]
[developers.dev.priority.review]
normal = ["opencode"]
EOF
v_run "$ROOT/norev.toml"; rc=$?
check "8: no review-capable agent fails" eq "$rc" "1"
check "8: names the use check for review" has "$ROOT/err" "developers.dev.use: レビューの役割を持つ担当が 1 つも無い"

# 試験 9: TOML として読めないファイルは非 0。
printf 'this is [ not valid toml\n' > "$ROOT/broken.toml"
v_run "$ROOT/broken.toml"; check "9: unparsable TOML fails" eq "$?" "1"

# 試験 10: 共有部分に無い担当 id が書かれたファイルは非 0。
write_case "$ROOT/unknown.toml" <<'EOF'
[developers.dev]
use = ["opencode", "devin", "codex", "zed"]
[developers.dev.slots]
opencode = 1
devin = 2
zed = 1
[developers.dev.review_limits]
codex = 1
devin = 1
zed = 1
[developers.dev.priority.implement]
normal = ["devin", "opencode"]
[developers.dev.priority.review]
normal = ["codex", "devin", "zed"]
EOF
v_run "$ROOT/unknown.toml"; rc=$?
check "10: unregistered agent id fails" eq "$rc" "1"
check "10: names the unknown id" has "$ROOT/err" "zed"

# 試験 11: このテストが scripts/script-test.sh に収集される（mise run verify で走る）。
listed="$(bash "$REPO/scripts/script-test.sh" --list "$REPO" 2>/dev/null)"
check "11: this test is collected by script-test" eq "$(printf '%s\n' "$listed" | grep -cx 'scripts/tests/agent-team-config.test.sh')" "1"

# 試験 12: AGENTS.md の「Agent workflow」の共有層の列挙に agent-team.toml がある。
enum="$(grep -n 'Shared rules and skills live only where every tool reads them' "$REPO/AGENTS.md" | head -1)"
check "12: AGENTS.md shared-layer enumeration exists" eq "$([ -n "$enum" ] && echo yes)" "yes"
check "12: enumeration names agent-team.toml" eq "$(printf '%s\n' "$enum" | grep -c 'agent-team\.toml')" "1"

# 試験 13: slots で表に無い login は非 0。
bash "$CFG" slots nobody "$ROOT/ok.toml" > /dev/null 2>&1
check "13: slots for unknown login fails" eq "$?" "1"

# 試験 14: 使える実装担当に枠の上限が無い（無制限になる）表は非 0。
write_case "$ROOT/nolimit.toml" <<'EOF'
[developers.dev]
use = ["opencode", "devin", "codex", "claude"]
[developers.dev.slots]
opencode = 1
[developers.dev.review_limits]
codex = 1
devin = 1
claude = 1
[developers.dev.priority.implement]
normal = ["devin", "opencode"]
[developers.dev.priority.review]
normal = ["codex", "devin"]
EOF
v_run "$ROOT/nolimit.toml"; rc=$?
check "14: implement-capable agent without a limit fails" eq "$rc" "1"
check "14: names the missing limit" has "$ROOT/err" "devin"

# 試験 15: slots はファイル全体の検証を通してから出す。未登録の担当・枠の欠落・
# 途中の上限値の誤りでは、非 0 で標準出力は空。
s_run() { bash "$CFG" slots dev "$1" 2> "$ROOT/err" > "$ROOT/out"; }
s_run "$ROOT/unknown.toml"; rc=$?
check "15: slots with unregistered agent fails" eq "$rc" "1"
check "15: slots prints nothing on unregistered agent" eq "$(grep -c . "$ROOT/out")" "0"
s_run "$ROOT/nolimit.toml"; rc=$?
check "15: slots with missing limit fails" eq "$rc" "1"
check "15: slots prints nothing on missing limit" eq "$(grep -c . "$ROOT/out")" "0"
write_case "$ROOT/midbad.toml" <<'EOF'
[developers.dev]
use = ["opencode", "devin", "codex", "claude"]
[developers.dev.slots]
opencode = 1
devin = 0
[developers.dev.review_limits]
codex = 1
devin = 1
claude = 1
[developers.dev.priority.implement]
normal = ["devin", "opencode"]
[developers.dev.priority.review]
normal = ["codex", "devin"]
EOF
s_run "$ROOT/midbad.toml"; rc=$?
check "15: slots with mid-list bad limit fails" eq "$rc" "1"
check "15: slots prints nothing on mid-list bad limit" eq "$(grep -c . "$ROOT/out")" "0"

if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}"
  exit 1
fi
printf '\nall passed\n'
