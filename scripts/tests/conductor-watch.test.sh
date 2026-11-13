#!/usr/bin/env bash
# scripts/conductor-watch.sh のテスト。モックの t3・agent-gh で GitHub と T3 の状態を返し、
# 巡回状態の補助記録（patrol-state.json）への反映を確かめる。runbook の記述も grep で確かめる。
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUNBOOK="$REPO/docs/agents/grokbot-runbook.md"
ROOT="$(mktemp -d)"
WPID=""
trap '[[ -n "${WPID:-}" ]] && kill "$WPID" 2>/dev/null; rm -rf "$ROOT"' EXIT
failures=()

MOCK="$ROOT/mock"
mkdir -p "$ROOT/repo/scripts" "$ROOT/bin" "$MOCK" "$ROOT/state/pelway"
STATE="$ROOT/state/pelway/patrol-state.json"

# 本物の conductor-watch.sh をコピーした偽リポジトリ（scripts/agent-gh をモックに差し替える）
cp "$REPO/scripts/conductor-watch.sh" "$ROOT/repo/scripts/"
cat > "$ROOT/repo/scripts/agent-gh" <<'EOF'
#!/usr/bin/env bash
# モック: issue list / pr list に $MOCK の JSON を返す。pr list の呼び出し回数を $MOCK/cycle に数える。
# MOCK_ISSUE_FAIL / MOCK_PR_FAIL があれば、失敗（exit 9）の代わりに [] を標準出力に出してから失敗する
# （一時的な API 失敗で空配列相当が stdout に残るケースの再現）。
if [[ "$2 $3" == "issue list" ]]; then
  [[ -n "${MOCK_ISSUE_FAIL:-}" ]] && { echo '[]'; exit 9; }
  cat "$MOCK/issues.json"; exit 0
fi
if [[ "$2 $3" == "pr list" ]]; then
  [[ -n "${MOCK_PR_FAIL:-}" ]] && { echo '[]'; exit 9; }
  echo $(( $(cat "$MOCK/cycle" 2>/dev/null || echo 0) + 1 )) > "$MOCK/cycle"
  cat "$MOCK/prs.json"; exit 0
fi
exit 9
EOF
chmod +x "$ROOT/repo/scripts/agent-gh"

cat > "$ROOT/bin/t3" <<'EOF'
#!/usr/bin/env bash
[[ "$1 $2" == "thread list" ]] && cat "$MOCK/threads.json" && exit 0
exit 9
EOF
chmod +x "$ROOT/bin/t3"

cat > "$ROOT/bin/sleep" <<'EOF'
#!/usr/bin/env bash
# 巡回の周期を縮める。SLEEP_MAX 回目の呼び出しで親（conductor-watch.sh）を終了させ、
# 巡回を規定回数で確定させる（python の書き込みの途中を kill しない）。
n=$(( $(cat "$MOCK/sleeps" 2>/dev/null || echo 0) + 1 ))
echo "$n" > "$MOCK/sleeps"
if [[ -n "${SLEEP_MAX:-}" ]] && (( n >= SLEEP_MAX )); then
  kill -TERM "$PPID"; exit 0
fi
exec /bin/sleep 0.03
EOF
chmod +x "$ROOT/bin/sleep"

mock_issues() { printf '%s\n' "$1" > "$MOCK/issues.json"; }
mock_prs() { printf '%s\n' "$1" > "$MOCK/prs.json"; }
mock_threads() { printf '%s\n' "${1:-[]}" > "$MOCK/threads.json"; }

# iss 11 OPEN agent:working needs-human … の形で Issue 1 件の JSON を作る
iss() {
  python3 -I - "$@" <<'EOF'
import json, sys
print(json.dumps({"number": int(sys.argv[1]), "state": sys.argv[2],
                  "labels": [{"name": l} for l in sys.argv[3:]]}))
EOF
}

jarr() { local IFS=,; printf '[%s]' "$*"; }

# pr 101 OPEN true agent/11-abc "" … の形で PR 1 件の JSON を作る
# （番号 状態 isDraft ブランチ reviewDecision verify結論: ""|SUCCESS|FAILURE）
pr() {
  python3 -I - "$@" <<'EOF'
import json, sys
n, st, draft, br, rd, vf = sys.argv[1:7]
roll = [] if vf in ("", "NONE") else [{"name": "verify", "conclusion": vf}]
print(json.dumps({"number": int(n), "state": st, "isDraft": draft == "true",
                  "headRefName": br, "reviewDecision": None if rd in ("", "NONE") else rd,
                  "statusCheckRollup": roll}))
EOF
}

start_watch() {
  MOCK="$MOCK" SLEEP_MAX="${SLEEP_MAX:-}" MOCK_ISSUE_FAIL="${MOCK_ISSUE_FAIL:-}" \
  MOCK_PR_FAIL="${MOCK_PR_FAIL:-}" XDG_STATE_HOME="$ROOT/state" PATH="$ROOT/bin:$PATH" \
    bash "$ROOT/repo/scripts/conductor-watch.sh" >>"$ROOT/out.log" 2>&1 &
  WPID=$!
}

# 巡回 n 回（n 回目の python まで完了）を待って終了する
run_cycles() {
  local want="$1"
  echo 0 > "$MOCK/cycle"; echo 0 > "$MOCK/sleeps"
  SLEEP_MAX="$want" start_watch
  wait "$WPID" 2>/dev/null; WPID=""
}

sget() { # sget <issue> <field>
  python3 -I -c 'import json,sys
d = json.load(open(sys.argv[1]))
print(d["subjects"].get(sys.argv[2], {}).get(sys.argv[3], ""))' "$STATE" "$1" "$2"
}

set_since() { # patrol-state.json の waiting_since を書き換える
  python3 -I -c 'import json,sys
p, n, v = sys.argv[1:4]
d = json.load(open(p)); d["subjects"][n]["waiting_since"] = v
json.dump(d, open(p, "w"), ensure_ascii=False, indent=2, sort_keys=True)' "$STATE" "$1" "$2"
}

check() {
  local name="$1"; shift
  if "$@"; then printf 'ok   %s\n' "$name"; else printf 'FAIL %s\n' "$name"; failures+=("$name"); fi
}

eq() { [[ "$1" == "$2" ]]; }
neq() { [[ "$1" != "$2" ]]; }
has() { grep -q "$2" "$1"; }
nhas() { ! grep -q "$2" "$1"; }
has_all() { local f="$1"; shift; local w; for w in "$@"; do grep -q "$w" "$f" || return 1; done; }

OLD_SINCE="2020-01-01T00:00:00+09:00"

# --- ここから巡回の動作（Issue 11 と PR 101 を追う） ---

# 0. agent:working で PR がまだ無い → 実装担当の作業待ち
mock_issues "$(jarr "$(iss 11 OPEN agent:working)")"
mock_prs "[]"
: > "$ROOT/out.log"
run_cycles 2
check "no-pr issue is waiting_on=implement" eq "$(sget 11 waiting_on)" "implement"
check "no-pr issue has next_owner" has <(sget 11 next_owner) "implement thread (#11)"

# 1. 必須項目がすべて埋まっている（巡回のたびに更新される）
check "waiting_since filled" has <(sget 11 waiting_since) "20"
check "updated_at filled" has <(sget 11 updated_at) "20"

# 2. waiting_on が同じ対象は待ち始めた時刻を引き継ぐ
set_since 11 "$OLD_SINCE"
run_cycles 2
check "same waiting_on keeps waiting_since" eq "$(sget 11 waiting_since)" "$OLD_SINCE"

# 3. Draft PR ができても実装待ちのまま（PATROL 行は出ない。初回の新規記録行は対象外）
mock_prs "$(jarr "$(pr 101 OPEN true agent/11-abc "" NONE)")"
set_since 11 "$OLD_SINCE"
: > "$ROOT/out.log"
run_cycles 2
check "draft PR keeps waiting_on=implement" eq "$(sget 11 waiting_on)" "implement"
check "draft PR keeps waiting_since" eq "$(sget 11 waiting_since)" "$OLD_SINCE"
check "no PATROL line while unchanged" nhas "$ROOT/out.log" "PATROL #11"

# 4. PR が Ready になりレビューが付いていない → レビュー待ちに変わる（GitHub を正とする）
mock_prs "$(jarr "$(pr 101 OPEN false agent/11-abc "" NONE)")"
run_cycles 2
check "ready PR is waiting_on=review" eq "$(sget 11 waiting_on)" "review"
check "review next_owner names PR" has <(sget 11 next_owner) "reviewer (PR #101)"
check "changed waiting_on resets waiting_since" neq "$(sget 11 waiting_since)" "$OLD_SINCE"
check "PATROL line on change" has "$ROOT/out.log" "PATROL #11 waiting_on=review"

# 5. Approve 済み → マージ待ち
mock_prs "$(jarr "$(pr 101 OPEN false agent/11-abc APPROVED SUCCESS)")"
run_cycles 2
check "approved PR is waiting_on=merge" eq "$(sget 11 waiting_on)" "merge"

# 6. レビュー待ちのまま verify が失敗 → 実装待ちに戻す（GitHub を正とする）
mock_prs "$(jarr "$(pr 101 OPEN false agent/11-abc REVIEW_REQUIRED FAILURE)")"
run_cycles 2
check "failed verify is waiting_on=implement" eq "$(sget 11 waiting_on)" "implement"

# 7. Request changes → 修正待ち（実装担当）
mock_prs "$(jarr "$(pr 101 OPEN false agent/11-abc CHANGES_REQUESTED SUCCESS)")"
run_cycles 2
check "changes requested is waiting_on=implement" eq "$(sget 11 waiting_on)" "implement"

# 8. needs-human → 人間の判断待ち
mock_issues "$(jarr "$(iss 11 OPEN needs-human)")"
run_cycles 2
check "needs-human is waiting_on=human" eq "$(sget 11 waiting_on)" "human"
check "needs-human next_owner is hatai" eq "$(sget 11 next_owner)" "hatai"

# 9. ready-for-agent → 指揮役の判断待ち（順番待ち）。閉じた Issue #12 は記録から消える
mock_issues "$(jarr "$(iss 11 OPEN ready-for-agent)" "$(iss 12 CLOSED agent:working)")"
mock_prs "$(jarr "$(pr 102 CLOSED false agent/12-old "" NONE)")"
run_cycles 2
check "queued issue is waiting_on=conductor" eq "$(sget 11 waiting_on)" "conductor"
check "closed issue removed from record" eq "$(sget 12 waiting_on)" ""

# 10. GitHub の取得に失敗した巡回（失敗時に [] が標準出力に残る）は記録を書き換えない
MOCK_ISSUE_FAIL=1 run_cycles 2
check "failed issue fetch keeps record" eq "$(sget 11 waiting_on)" "conductor"
MOCK_ISSUE_FAIL=0 mock_issues "$(jarr "$(iss 11 OPEN agent:working)")"
mock_prs "$(jarr "$(pr 101 OPEN false agent/11-abc "" NONE)")"
run_cycles 2
check "ready for review again" eq "$(sget 11 waiting_on)" "review"
MOCK_PR_FAIL=1 run_cycles 2
check "failed pr fetch keeps record" eq "$(sget 11 waiting_on)" "review"

# --- runbook の記述 ---

check "runbook: patrol-state.json format and required fields" \
  has_all "$RUNBOOK" "patrol-state.json" "誰待ちか" "待ち始めた時刻" "次の担当" "最終更新時刻" "waiting_on" "waiting_since" "next_owner" "updated_at"
check "runbook: auxiliary record, GitHub is the source of truth" \
  has_all "$RUNBOOK" "補助記録" "正本は GitHub" "GitHub を正として"
check "runbook: handover trigger, order, completion" \
  has_all "$RUNBOOK" "引き継ぎ" "一定時間" "blocked" "必須項目が" "矛盾が無い"
check "runbook: agents display is not evidence" \
  has_all "$RUNBOOK" "busy" "根拠にせず" "突き合わせ"

if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}"
  exit 1
fi
printf '\nall passed\n'
