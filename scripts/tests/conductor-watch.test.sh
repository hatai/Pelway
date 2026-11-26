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

# 本物の conductor-watch.sh と patrol-state-update（書き戻しの共通手順）をコピーした偽リポジトリ（scripts/agent-gh をモックに差し替える）。
# patrol-state.json の既定の置き場は共有 git dir（--git-common-dir）内の pelway/ のため、
# 偽リポジトリも git init して実際の解決を通す（Issue #477）。
cp "$REPO/scripts/conductor-watch.sh" "$ROOT/repo/scripts/"
cp "$REPO/scripts/patrol-state-update" "$ROOT/repo/scripts/"
git init -q "$ROOT/repo"
STATE="$ROOT/repo/.git/pelway/patrol-state.json"
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
# スレッド一覧の取得は成功（0 本）にしておく。失敗した巡回では conductor が implement・review に
# 戻らないため（Issue #651）、ここでのラベルと PR からの導出の試験が成り立たない。
mock_threads

# iss 11 OPEN agent:working needs-human … の形で Issue 1 件の JSON を作る
iss() {
  python3 -I - "$@" <<'EOF'
import json, sys
print(json.dumps({"number": int(sys.argv[1]), "state": sys.argv[2],
                  "labels": [{"name": l} for l in sys.argv[3:]]}))
EOF
}

jarr() { local out="" x; for x in "$@"; do out+="${out:+,}$x"; done; printf '[%s]' "$out"; }

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
  T3_BIN="${T3_BIN:-$ROOT/bin/t3}" \
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

set_updated() { # patrol-state.json の updated_at を書き換える（巡回が記録を書き換えたかの判定に使う）
  python3 -I -c 'import json,sys
p, n, v = sys.argv[1:4]
d = json.load(open(p)); d["subjects"][n]["updated_at"] = v
json.dump(d, open(p, "w"), ensure_ascii=False, indent=2, sort_keys=True)' "$STATE" "$1" "$2"
}

set_nca() { # patrol-state.json の next_check_at を書き換える（指揮役の延長・短縮の再現）
  python3 -I -c 'import json,sys
p, n, v = sys.argv[1:4]
d = json.load(open(p)); d["subjects"][n]["next_check_at"] = v
json.dump(d, open(p, "w"), ensure_ascii=False, indent=2, sort_keys=True)' "$STATE" "$1" "$2"
}

nca_off() { # next_check_at の現在時刻からのずれ（時間単位で四捨五入）。読めなければ空
  python3 -I -c 'import json,sys,datetime,time
d = json.load(open(sys.argv[1]))
s = d["subjects"].get(sys.argv[2], {}).get("next_check_at", "")
try: print(round((datetime.datetime.fromisoformat(s.replace("Z", "+00:00")).timestamp() - time.time()) / 3600))
except Exception: print("")' "$STATE" "$1"
}

pget() { # pget <top-level key> — patrol-state.json のトップレベル項目を読む
  python3 -I -c 'import json,sys
print(json.dumps(json.load(open(sys.argv[1])).get(sys.argv[2], ""), ensure_ascii=False))' "$STATE" "$1"
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

# --- 旧置き場（$XDG_STATE_HOME/pelway/）からの移行（Issue #477）---
# 新置き場（共有 git dir 内の pelway/）が無い初回の起動で中身を引き継ぐ。
# 巡回スクリプトが上書きする subjects・updated_at 以外の項目が残ることを確かめる。
cat > "$ROOT/state/pelway/patrol-state.json" <<'EOF'
{"subjects": {}, "items": {"99": {"main_pr": 123}},
 "scheduled_task_id": "sched-migration", "watch_log_offset": 7,
 "conductor_handover": {"note": "migration-check"}}
EOF

# --- ここから巡回の動作（Issue 11 と PR 101 を追う） ---

# 0. agent:working で PR がまだ無い → 実装担当の作業待ち
mock_issues "$(jarr "$(iss 11 OPEN agent:working)")"
mock_prs "[]"
: > "$ROOT/out.log"
run_cycles 2
check "no-pr issue is waiting_on=implement" eq "$(sget 11 waiting_on)" "implement"
check "no-pr issue has next_owner" has <(sget 11 next_owner) "implement thread (#11)"
check "patrol-state written inside git common dir" eq "$STATE" "$ROOT/repo/.git/pelway/patrol-state.json"
check "migration: scheduled_task_id carried over" eq "$(pget scheduled_task_id)" '"sched-migration"'
check "migration: items carried over" has <(pget items) '"99"'
check "migration: watch_log_offset carried over" eq "$(pget watch_log_offset)" '7'
check "migration: conductor_handover carried over" has <(pget conductor_handover) 'migration-check'
check "migration: old path left untouched" has "$ROOT/state/pelway/patrol-state.json" 'sched-migration'

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

# 11. 一覧の取得上限に達した（打ち切りの可能性がある）巡回は記録を書き換えない
OLD_UPDATED="2020-01-01T00:00:00+09:00"
python3 -I - > "$MOCK/issues.json" <<'EOF'
import json
# 上限（1000 件）ちょうど。対象の Issue #11 を含む
issues = [{"number": 11, "state": "OPEN", "labels": [{"name": "agent:working"}]}]
issues += [{"number": 2000 - i, "state": "CLOSED", "labels": []} for i in range(999)]
print(json.dumps(issues))
EOF
set_updated 11 "$OLD_UPDATED"
run_cycles 2
check "issue list at limit keeps record" eq "$(sget 11 updated_at)" "$OLD_UPDATED"
mock_issues "$(jarr "$(iss 11 OPEN agent:working)")"
python3 -I - > "$MOCK/prs.json" <<'EOF'
import json
# 上限（1000 件）ちょうど。対象の PR #101 を含む
prs = [{"number": 101, "state": "OPEN", "isDraft": False, "headRefName": "agent/11-abc",
        "reviewDecision": None, "statusCheckRollup": []}]
prs += [{"number": 5000 - i, "state": "MERGED", "isDraft": False, "headRefName": f"agent/{4000 + i}-x",
         "reviewDecision": None, "statusCheckRollup": []} for i in range(999)]
print(json.dumps(prs))
EOF
set_updated 11 "$OLD_UPDATED"
run_cycles 2
check "pr list at limit keeps record" eq "$(sget 11 updated_at)" "$OLD_UPDATED"

# 12. 次回確認時刻（next_check_at）: 待ちが発生したら目安で埋まる（review は 24 時間）。
# waiting_on が同じ間は、指揮役が書き換えた値も含めて前回値を引き継ぐ
mock_issues "$(jarr "$(iss 11 OPEN agent:working)")"
mock_prs "$(jarr "$(pr 101 OPEN false agent/11-abc "" NONE)")"
run_cycles 2
check "next_check_at filled with default (review=24h)" eq "$(nca_off 11)" "24"
set_nca 11 "2021-05-05T00:00:00+09:00"
run_cycles 2
check "same waiting_on keeps conductor-set next_check_at" eq "$(sget 11 next_check_at)" "2021-05-05T00:00:00+09:00"

# 13. 期限到来: 過ぎた次回確認時刻に due 行を出す（同じ期限の値につき 1 回）。
# 指揮役が期限を先へ延ばしたら due 行は止まる
set_nca 11 "2020-01-01T00:00:00+09:00"
: > "$ROOT/out.log"
run_cycles 2
check "due line emitted once for past deadline" \
  eq "$(grep -c 'PATROL #11 due next_check_at=2020-01-01T00:00:00+09:00' "$ROOT/out.log")" "1"
set_nca 11 "2999-01-01T00:00:00+09:00"
: > "$ROOT/out.log"
run_cycles 2
check "pushed deadline stops due lines" nhas "$ROOT/out.log" "PATROL #11 due"

# 14. waiting_on が変わったら目安で設定し直す（implement は 6 時間）。
# 読めない値も目安で設定し直す（自己修復。特別な報告はしない）
mock_prs "[]"
run_cycles 2
check "waiting_on change resets next_check_at (implement=6h)" eq "$(nca_off 11)" "6"
set_nca 11 "not-a-date"
run_cycles 2
check "invalid next_check_at refilled with default" eq "$(nca_off 11)" "6"

# --- T3_BIN の明示指定（PATH の t3 が ServerOwnsDatabaseError 相当で失敗するときの差し替え） ---

cat > "$ROOT/bin/t3-server" <<'EOF'
#!/usr/bin/env bash
# モック: 実行中サーバーと同じ実行ファイルの代わり。thread list に server 側の JSON を返す
[[ "$1 $2" == "thread list" ]] && cat "$MOCK/threads-server.json" && exit 0
exit 9
EOF
chmod +x "$ROOT/bin/t3-server"
cat > "$MOCK/threads-server.json" <<'EOF'
[{"threadId":"srv-1","status":"completed","title":"[実装] #1 server-side","createdBy":"agent","archived":false,"itemCount":3}]
EOF
# PATH の t3 を常に失敗する版に替える（/usr/local/bin/t3 が DB 所有権で拒否される状況の再現）
mv "$ROOT/bin/t3" "$ROOT/bin/t3-ok"
printf '#!/usr/bin/env bash\nexit 1\n' > "$ROOT/bin/t3"; chmod +x "$ROOT/bin/t3"
mock_issues '[]'; mock_prs '[]'
: > "$ROOT/out.log"
T3_BIN="$ROOT/bin/t3-server" run_cycles 1
mv "$ROOT/bin/t3-ok" "$ROOT/bin/t3"
check "T3_BIN override: server binary output lands in threads.json" \
  has "$ROOT/state/pelway/conductor-watch/threads.json" "srv-1"

# --- runbook の記述 ---

check "runbook: patrol-state.json format and required fields" \
  has_all "$RUNBOOK" "patrol-state.json" "誰待ちか" "待ち始めた時刻" "次の担当" "最終更新時刻" "waiting_on" "waiting_since" "next_owner" "next_check_at" "updated_at" "次回確認時刻"
check "runbook: auxiliary record, GitHub is the source of truth" \
  has_all "$RUNBOOK" "補助記録" "正本は GitHub" "GitHub を正として"
check "runbook: handover trigger, order, completion" \
  has_all "$RUNBOOK" "引き継ぎ" "一定時間" "blocked" "必須項目が" "矛盾が無い"
check "runbook: agents display is not evidence" \
  has_all "$RUNBOOK" "busy" "根拠にせず" "突き合わせ"
check "runbook: follow-up timer field, defaults table, both-places rule" \
  has_all "$RUNBOOK" "フォローアップタイマー" "目安" "24 時間" "6 時間" "1 時間" "この表とスクリプトの両方を直す"
check "runbook: due waits are always picked up in patrol order" \
  has_all "$RUNBOOK" "期限到来待ちの確認" "その巡回で必ず確認" "次回確認時刻を更新" "自己修復として扱い、特別な報告はしない"
check "runbook: setting a wait sets its follow-up timer (escalation)" \
  has_all "$RUNBOOK" "を付けた時点で、巡回スクリプトが補助記録の次回確認時刻を目安" "人間の判断に期限を付けたいときは"

if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}"
  exit 1
fi
printf '\nall passed\n'
