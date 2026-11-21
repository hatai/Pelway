#!/usr/bin/env bash
# scripts/conductor-watch.sh の needs-decision の 15 分の検知（DECISION overdue）と、指揮役が
# 動いていないときの needs-human への格上げ（Issue #602）のテスト。モックの agent-gh が GitHub の
# 状態（Issue のラベル・Issue ごとのイベント・コメント）を $MOCK/gh.json に持ち、書き込み
# （issue comment・issue edit）の引数を $MOCK/calls に順に記録する。モックの t3 がスレッドの一覧を返す。
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ROOT="$(mktemp -d)"
WPID=""
trap '[[ -n "${WPID:-}" ]] && kill "$WPID" 2>/dev/null; rm -rf "$ROOT"' EXIT
failures=()

MOCK="$ROOT/mock"
mkdir -p "$ROOT/repo/scripts" "$ROOT/bin" "$MOCK" "$ROOT/state"
cp "$REPO/scripts/conductor-watch.sh" "$ROOT/repo/scripts/"
git init -q "$ROOT/repo"
PELWAY="$ROOT/repo/.git/pelway"
STATE="$PELWAY/patrol-state.json"
REG="$ROOT/conductor-thread-id"
CONDUCTOR="conductor-thread-7f3a"

# モックの agent-gh。$MOCK/fail-<events|reread|comment|edit-add|edit-remove> があると、その呼び出しは失敗する。
cat > "$ROOT/repo/scripts/agent-gh" <<'EOF'
#!/usr/bin/env python3
import json, os, re, sys, time
mock = os.environ["MOCK"]; gp = os.path.join(mock, "gh.json")
st = json.load(open(gp)) if os.path.exists(gp) else {}
a = sys.argv[2:]
def out(v): print(json.dumps(v)); sys.exit(0)
def failing(k): return os.path.exists(os.path.join(mock, "fail-" + k))
def labels(names): return [{"name": l} for l in names]
def save(): json.dump(st, open(gp, "w"))
if a[:2] == ["issue", "list"]:
    out([{"number": int(n), "state": i["state"].upper(), "labels": labels(i.get("list_labels", i["labels"]))}
         for n, i in st.items()])
if a[:2] == ["pr", "list"]: out([])
if a[:2] == ["api", "/repos/hatai/Pelway/issues/events?per_page=100"]: out([])
if a[:1] == ["api"]:
    m = re.fullmatch(r"/repos/hatai/Pelway/issues/(\d+)(/events|/comments)?(\?per_page=100)?", a[1])
    if m and m.group(1) in st:
        i = st[m.group(1)]
        if m.group(2) is None and m.group(3) is None and len(a) == 2:
            if failing("reread"): sys.exit(9)
            out({"number": int(m.group(1)), "state": i["state"], "labels": labels(i["labels"])})
        if m.group(3) and a[2:] == ["--paginate", "--slurp"]:
            if m.group(2) == "/events":
                if failing("events"): sys.exit(9)
                out(i["events"])
            cs = i.get("comments", [])
            out([cs[k:k + 2] for k in range(0, len(cs), 2)] or [[]])
open(os.path.join(mock, "calls"), "a").write(" ".join(a) + "\n")
if a[:2] == ["issue", "comment"] and a[3:] == ["--body-file", "-"] and a[2] in st:
    body = sys.stdin.read()
    k = sum(1 for f in os.listdir(mock) if f.startswith(f"body-{a[2]}-")) + 1
    open(os.path.join(mock, f"body-{a[2]}-{k}"), "w").write(body)
    if failing("comment"): sys.exit(9)
    st[a[2]].setdefault("comments", []).append({"user": {"login": "pelway-dev[bot]"}, "body": body})
    save(); sys.exit(0)
if a[:2] == ["issue", "edit"] and len(a) == 5 and a[2] in st:
    i = st[a[2]]; i.pop("list_labels", None)
    if a[3] == "--add-label" and not failing("edit-add"):
        i["labels"] = sorted(set(i["labels"]) | {a[4]}); save(); sys.exit(0)
    if a[3] == "--remove-label" and not failing("edit-remove"):
        i["labels"] = [l for l in i["labels"] if l != a[4]]; save(); sys.exit(0)
sys.exit(9)
EOF
chmod +x "$ROOT/repo/scripts/agent-gh"
# モックの t3。$MOCK/fail-t3 があると、[] を出してから失敗する（空の一覧と区別できるかを見る）。
cat > "$ROOT/bin/t3" <<'EOF'
#!/usr/bin/env bash
[[ "$1 $2" == "thread list" ]] || exit 9
[[ -e "$MOCK/fail-t3" ]] && { echo '[]'; exit 9; }
cat "$MOCK/threads.json"
EOF
chmod +x "$ROOT/bin/t3"
cat > "$ROOT/bin/sleep" <<'EOF'
#!/usr/bin/env bash
n=$(( $(cat "$MOCK/sleeps" 2>/dev/null || echo 0) + 1 ))
echo "$n" > "$MOCK/sleeps"
if [[ -n "${SLEEP_MAX:-}" ]] && (( n >= SLEEP_MAX )); then kill -TERM "$PPID"; exit 0; fi
exec /bin/sleep 0.03
EOF
chmod +x "$ROOT/bin/sleep"

ago() { # ago <分> — その分だけ前の時刻（UTC、GitHub の形）
  python3 -I -c 'import datetime,sys
t = datetime.datetime.now(datetime.timezone.utc) - datetime.timedelta(minutes=int(sys.argv[1]))
print(t.strftime("%Y-%m-%dT%H:%M:%SZ"))' "$1"
}
T5="$(ago 5)"; T16="$(ago 16)"; T20="$(ago 20)"; T40="$(ago 40)"

gh_edit() { # gh_edit <python の文> [引数...] — gh.json を i（Issue の辞書）と st（全体）として書き換える
  local code="$1"; shift
  python3 -I - "$MOCK/gh.json" "$code" "$@" <<'EOF'
import json, os, sys
p, code, args = sys.argv[1], sys.argv[2], sys.argv[3:]
st = json.load(open(p)) if os.path.exists(p) else {}
def ev(kind, label, at): return {"event": kind, "label": {"name": label}, "created_at": at}
exec(code)
json.dump(st, open(p, "w"))
EOF
}
# issue_with <番号> <labeled_at> <ラベル...> — 開いた Issue を置く。イベントは 2 ページで、古い付け外しと
# labeled_at の後の別のラベルを混ぜる（最新の needs-decision の labeled だけが起点になる）。
issue_with() {
  gh_edit '
from datetime import datetime, timedelta
n, at = args[0], args[1]
t = lambda m: (datetime.fromisoformat(at.replace("Z", "+00:00")) + timedelta(minutes=m)).strftime("%Y-%m-%dT%H:%M:%SZ")
st[n] = {"state": "open", "labels": args[2:], "comments": [],
         "events": [[ev("labeled", "needs-decision", t(-40)), ev("unlabeled", "needs-decision", t(-30))],
                    [ev("labeled", "needs-decision", at), ev("labeled", "agent:working", t(1))]]}' "$@"
}
relabel() { gh_edit 'st[args[0]]["events"][-1].append(ev("labeled", "needs-decision", args[1]))' "$1" "$2"; }
set_labels() { gh_edit 'st[args[0]]["labels"] = args[1:]; st[args[0]].pop("list_labels", None)' "$@"; }
# GitHub の実際のラベルだけを変え、一覧（issue list）は前のまま見せる（読み直しの確認用）
set_actual_labels() { gh_edit 'i = st[args[0]]; i.setdefault("list_labels", i["labels"]); i["labels"] = args[1:]' "$@"; }
add_comment() { gh_edit 'st[args[0]]["comments"].append({"user": {"login": args[1]}, "body": args[2]})' "$@"; }

threads() { printf '%s\n' "$1" > "$MOCK/threads.json"; }
live_thread() { threads "[{\"threadId\":\"$CONDUCTOR\",\"title\":\"指揮役\",\"status\":\"idle\",\"archived\":${1:-false},\"createdBy\":\"user\"}]"; }
reg() { printf '%s\n' "$1" > "$REG"; }

reset() {
  rm -rf "$PELWAY" "$ROOT/state/pelway" "$MOCK"/* "$REG"
  mkdir -p "$PELWAY"; : > "$ROOT/out.log"; live_thread
}
run_cycles() {
  echo 0 > "$MOCK/sleeps"
  MOCK="$MOCK" SLEEP_MAX="$1" XDG_STATE_HOME="$ROOT/state" PATH="$ROOT/bin:$PATH" \
    T3_BIN="$ROOT/bin/t3" PELWAY_BOARD_QUEUE_DIR="$ROOT/queue" PELWAY_CONDUCTOR_FILE="$REG" \
    bash "$ROOT/repo/scripts/conductor-watch.sh" >>"$ROOT/out.log" 2>&1 &
  WPID=$!
  wait "$WPID" 2>/dev/null; WPID=""
}

sget() {
  python3 -I -c 'import json,sys
print(json.load(open(sys.argv[1]))["subjects"].get(sys.argv[2], {}).get(sys.argv[3], ""))' "$STATE" "$1" "$2"
}
nca_after() { # nca_after <番号> <時刻> — 補助記録の next_check_at が時刻の何秒後か
  python3 -I -c 'import json,sys,datetime
s = json.load(open(sys.argv[1]))["subjects"][sys.argv[2]]["next_check_at"]
base = datetime.datetime.fromisoformat(sys.argv[3].replace("Z", "+00:00"))
print(round(datetime.datetime.fromisoformat(s).timestamp() - base.timestamp()))' "$STATE" "$1" "$2"
}
decision_lines() { grep '^DECISION ' "$ROOT/out.log"; }
calls() { cat "$MOCK/calls" 2>/dev/null; }
lines() { printf '%s\n' "$@"; }
body_for() { # body_for <labeled_at> — 格上げのコメントの本文（定型）
  printf '%s\n%s' '<!-- conductor-watch-escalation -->' \
    "@hatai \`needs-decision\` が付いた時刻 $1 から 15 分決まらず、指揮役が動いていないため \`needs-human\` に替えました。"
}
escalation_calls() { lines "issue comment $1 --body-file -" "issue edit $1 --add-label needs-human" "issue edit $1 --remove-label needs-decision"; }

check() {
  local name="$1"; shift
  if "$@"; then printf 'ok   %s\n' "$name"; else printf 'FAIL %s\n' "$name"; failures+=("$name"); fi
}
eq() { [[ "$1" == "$2" ]]; }
same_file() { cmp -s "$1" "$2"; }
lacks() { ! grep -qF -- "$2" "$1"; }

# --- 検知と補助記録（指揮役が動いている） ---

# 1. 15 分を過ぎたら DECISION overdue を 1 回。指揮役がいるので書き込みはしない。補助記録は grokbot。
reset; reg "$CONDUCTOR"; issue_with 701 "$T20" needs-decision
run_cycles 2
check "1: overdue line once for the same labeled_at (exact)" eq "$(decision_lines)" "DECISION overdue #701 labeled_at=$T20"
check "1: conductor running -> no comment or label change" eq "$(calls)" ""
check "1: record waiting_on is grokbot" eq "$(sget 701 waiting_on)" "grokbot"
check "1: record next_owner is grokbot" eq "$(sget 701 next_owner)" "grokbot"
check "1: record next_check_at is labeled_at + 15 min" eq "$(nca_after 701 "$T20")" "900"
check "1: record holds no thread id" lacks "$STATE" "$CONDUCTOR"

# 2. 15 分より前は出さない（指揮役がいなくても書き込まない）
reset; issue_with 702 "$T5" needs-decision
run_cycles 2
check "2: no line before 15 minutes" eq "$(decision_lines)" ""
check "2: no writes before 15 minutes" eq "$(calls)" ""
check "2: next_check_at is labeled_at + 15 min" eq "$(nca_after 702 "$T5")" "900"

# 3. 外れてから付け直されたら、新しい labeled_at で改めて数える。外れずに付け直しのイベントだけ
# 増えたときも同じ。
reset; reg "$CONDUCTOR"; issue_with 703 "$T40" needs-decision
run_cycles 1
set_labels 703
run_cycles 1
relabel 703 "$T20"; set_labels 703 needs-decision
run_cycles 1
relabel 703 "$T16"
run_cycles 1
check "3: re-labeling counts again from the new labeled_at" eq "$(decision_lines)" \
  "$(lines "DECISION overdue #703 labeled_at=$T40" "DECISION overdue #703 labeled_at=$T20" "DECISION overdue #703 labeled_at=$T16")"
check "3: next_check_at follows the new labeled_at" eq "$(nca_after 703 "$T16")" "900"

# 4. Issue ごとのイベントの取得に失敗した周は、行も書き込みも無く、次の周で取り直す
reset; issue_with 704 "$T20" needs-decision; touch "$MOCK/fail-events"
run_cycles 1
check "4: no line when the per-issue events fetch fails" eq "$(decision_lines)" ""
check "4: no writes when the per-issue events fetch fails" eq "$(calls)" ""
rm "$MOCK/fail-events"
run_cycles 1
check "4: next cycle fetches again and reports" eq "$(decision_lines)" \
  "$(lines "DECISION overdue #704 labeled_at=$T20" "DECISION escalated #704 labeled_at=$T20 reason=no-registration")"

# 5. needs-decision と needs-human の両方が付いていれば human のまま、DECISION は出さない
reset; issue_with 705 "$T20" needs-decision needs-human
run_cycles 1
check "5: both labels -> waiting_on human" eq "$(sget 705 waiting_on)" "human"
check "5: both labels -> no DECISION line" eq "$(decision_lines)" ""
check "5: both labels -> no writes" eq "$(calls)" ""

# --- 指揮役が動いていないときの格上げ ---

# 6. 登録ファイルが無い: コメントを書いてから needs-human を付け、needs-decision を外す。1 回だけ。
reset; issue_with 706 "$T20" needs-decision
run_cycles 2
check "6: comment, then add needs-human, then remove needs-decision" eq "$(calls)" "$(escalation_calls 706)"
check "6: comment body is the fixed text only" same_file "$MOCK/body-706-1" <(body_for "$T20")
check "6: lines (exact)" eq "$(decision_lines)" \
  "$(lines "DECISION overdue #706 labeled_at=$T20" "DECISION escalated #706 labeled_at=$T20 reason=no-registration")"
check "6: escalated issue waits on human" eq "$(sget 706 waiting_on)" "human"

# 7〜9. 空・symlink・FIFO の登録ファイルは、登録が無いものとして扱う（symlink はたどらない）
reset; issue_with 707 "$T20" needs-decision; : > "$REG"
run_cycles 1
check "7: empty registration -> escalated (exact)" eq "$(decision_lines | tail -n 1)" "DECISION escalated #707 labeled_at=$T20 reason=no-registration"
reset; issue_with 708 "$T20" needs-decision; printf '%s\n' "$CONDUCTOR" > "$ROOT/real-id"; ln -s "$ROOT/real-id" "$REG"
run_cycles 1
check "8: symlinked registration is not followed -> escalated" eq "$(decision_lines | tail -n 1)" "DECISION escalated #708 labeled_at=$T20 reason=no-registration"
reset; issue_with 709 "$T20" needs-decision; mkfifo "$REG"
run_cycles 1
check "9: FIFO registration -> escalated without blocking" eq "$(decision_lines | tail -n 1)" "DECISION escalated #709 labeled_at=$T20 reason=no-registration"

# 10〜11. 登録された ID が一覧に無い・archived のときも格上げする
reset; issue_with 710 "$T20" needs-decision; reg "$CONDUCTOR"; threads '[]'
run_cycles 1
check "10: registered thread missing -> escalated" eq "$(decision_lines | tail -n 1)" "DECISION escalated #710 labeled_at=$T20 reason=thread-missing"
check "10: lines and record hold no thread id" eval '! grep -qF "$CONDUCTOR" "$ROOT/out.log" && ! grep -qF "$CONDUCTOR" "$STATE"'
reset; issue_with 711 "$T20" needs-decision; reg "$CONDUCTOR"; live_thread true
run_cycles 1
check "11: registered thread archived -> escalated" eq "$(decision_lines | tail -n 1)" "DECISION escalated #711 labeled_at=$T20 reason=thread-archived"
check "11: escalation writes in order" eq "$(calls)" "$(escalation_calls 711)"

# 12. t3 thread list の取得に失敗した周は格上げしない（登録が無くても判定しない）。次の周で判定する
reset; issue_with 712 "$T20" needs-decision; reg "$CONDUCTOR"; threads '[]'; touch "$MOCK/fail-t3"
run_cycles 1
check "12: failed thread list -> overdue only" eq "$(decision_lines)" "DECISION overdue #712 labeled_at=$T20"
check "12: failed thread list -> no writes" eq "$(calls)" ""
rm "$REG"
run_cycles 1
check "12: failed thread list without registration -> still no writes" eq "$(calls)" ""
rm "$MOCK/fail-t3"
run_cycles 1
check "12: next cycle with a working list escalates" eq "$(decision_lines | tail -n 1)" "DECISION escalated #712 labeled_at=$T20 reason=no-registration"

# 13. 読み直したときに needs-decision が外れている・needs-human が付いているなら、何も書かない
reset; issue_with 713 "$T20" needs-decision; set_actual_labels 713
run_cycles 1
check "13: label already removed on re-read -> no writes" eq "$(calls)" ""
check "13: label already removed on re-read -> overdue only" eq "$(decision_lines)" "DECISION overdue #713 labeled_at=$T20"
reset; issue_with 714 "$T20" needs-decision; set_actual_labels 714 needs-decision needs-human
run_cycles 1
check "13: needs-human already added on re-read -> no writes" eq "$(calls)" ""

# 14. コメントに失敗: step=comment を出し、ラベルは変えない。次の周で読み直しからやり直す
reset; issue_with 715 "$T20" needs-decision; touch "$MOCK/fail-comment"
run_cycles 1
check "14: failed comment -> step=comment, no label change" eq "$(calls)" "issue comment 715 --body-file -"
check "14: failed comment line (exact)" eq "$(decision_lines | tail -n 1)" "DECISION escalate-failed #715 step=comment"
rm "$MOCK/fail-comment"; : > "$MOCK/calls"
run_cycles 1
check "14: retried from the re-read on the next cycle" eq "$(calls)" "$(escalation_calls 715)"
check "14: escalated after retry" eq "$(decision_lines | tail -n 1)" "DECISION escalated #715 labeled_at=$T20 reason=no-registration"

# 15. コメントは書けてラベルの変更に失敗: step=label。次の周で同じコメントを 2 回目は書かない
reset; issue_with 716 "$T20" needs-decision; touch "$MOCK/fail-edit-add"
run_cycles 1
check "15: failed label change line (exact)" eq "$(decision_lines | tail -n 1)" "DECISION escalate-failed #716 step=label"
rm "$MOCK/fail-edit-add"; : > "$MOCK/calls"
run_cycles 1
check "15: no second comment for the same labeled_at" eq "$(calls)" \
  "$(lines "issue edit 716 --add-label needs-human" "issue edit 716 --remove-label needs-decision")"
check "15: escalated after retry" eq "$(decision_lines | tail -n 1)" "DECISION escalated #716 labeled_at=$T20 reason=no-registration"

# 16. needs-human を付けた後に外すのが失敗しても、どちらのラベルも無い状態にはならない
reset; issue_with 717 "$T20" needs-decision; touch "$MOCK/fail-edit-remove"
run_cycles 1
check "16: failed removal -> step=label" eq "$(decision_lines | tail -n 1)" "DECISION escalate-failed #717 step=label"
rm "$MOCK/fail-edit-remove"; : > "$MOCK/calls"
run_cycles 1
check "16: needs-human is kept, nothing more is written" eq "$(calls)" ""
check "16: issue waits on human" eq "$(sget 717 waiting_on)" "human"

# 17. 同じ目印・同じ本文でも、pelway-dev[bot] 以外のコメントは書いた印にしない
reset; issue_with 718 "$T20" needs-decision; add_comment 718 stranger "$(body_for "$T20")"
run_cycles 1
check "17: a stranger's marker comment does not suppress the notice" eq "$(calls)" "$(escalation_calls 718)"
# 18. 前の labeled_at の格上げのコメントは、新しい labeled_at の書いた印にしない
reset; issue_with 719 "$T20" needs-decision; add_comment 719 'pelway-dev[bot]' "$(body_for "$T40")"
run_cycles 1
check "18: a comment for an older labeled_at does not suppress the notice" eq "$(calls)" "$(escalation_calls 719)"

if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}"
  exit 1
fi
printf '\nall passed\n'
