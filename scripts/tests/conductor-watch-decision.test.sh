#!/usr/bin/env bash
# scripts/conductor-watch.sh の needs-decision の 15 分の検知（DECISION overdue）と、指揮役が動いていないときの
# needs-human への格上げ（Issue #602）のテスト。モックの agent-gh が GitHub の状態（ラベル・Issue ごとのイベント・コメント）を
# $MOCK/gh.json に持ち、書き込み（issue comment・issue edit）の引数を $MOCK/calls に、Issue ごとの読み取りの path を
# $MOCK/reads に順に記録する。読み取りの間のほかの誰かの変更は on_read で差し込む。モックの t3 がスレッドの一覧を返す。
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

# モックの agent-gh。$MOCK/fail-<events|reread|comment|edit-add|edit-remove> があると、その呼び出しは失敗する。-lost の付く
# ものはラベルを変えてから失敗を返し（応答が失われた）、fail-edit-add-crash は付けた直後に見張りを止める。ラベルが実際に変わった
# ときだけ GitHub と同じく pelway-dev[bot] の名義のイベントを残し、イベントには古い順に増える ID を付けて返す。
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
def hook(what, n, i):  # この呼び出しの間に、ほかの誰かが Issue を変えた（$MOCK/on-<what>-<番号>: 1 行目の回数だけ見送り、残りの python の文を実行）
    hp = os.path.join(mock, f"on-{what}-{n}")
    if not os.path.exists(hp): return
    skip, code = open(hp).read().split("\n", 1)
    if int(skip): open(hp, "w").write(f"{int(skip) - 1}\n{code}"); return
    exec(code, {"i": i, "ev": lambda k, l, at: {"event": k, "label": {"name": l}, "created_at": at}})
    save(); os.remove(hp)
if a[:2] == ["issue", "list"]:
    out([{"number": int(n), "state": i["state"].upper(), "labels": labels(i.get("list_labels", i["labels"]))}
         for n, i in st.items()])
if a[:2] == ["pr", "list"]: out([])
if a[:2] == ["api", "/repos/hatai/Pelway/issues/events?per_page=100"]: out([])
if a[:1] == ["api"]:
    m = re.fullmatch(r"/repos/hatai/Pelway/issues/(\d+)(/events|/comments)?(\?per_page=100)?", a[1])
    if m and m.group(1) in st:
        i = st[m.group(1)]
        open(os.path.join(mock, "reads"), "a").write(a[1] + "\n")
        if m.group(2) is None and m.group(3) is None and len(a) == 2:
            if failing("reread"): sys.exit(9)
            hook("issue", m.group(1), i)
            out({"number": int(m.group(1)), "state": i["state"], "labels": labels(i["labels"])})
        if m.group(3) and a[2:] == ["--paginate", "--slurp"]:
            if m.group(2) == "/events":
                if failing("events"): sys.exit(9)
                hook("events", m.group(1), i); k = iter(range(1, 1 << 30))
                out([[dict(e, id=next(k)) for e in p] for p in i["events"]])
            hook("comments", m.group(1), i)
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
if a[:2] == ["issue", "edit"] and len(a) == 5 and a[2] in st and a[3] in ("--add-label", "--remove-label"):
    i, add = st[a[2]], a[3] == "--add-label"; i.pop("list_labels", None); kind = "edit-add" if add else "edit-remove"
    hook("edit", a[2], i)
    if failing(kind): sys.exit(9)
    changed = a[4] not in i["labels"] if add else a[4] in i["labels"]
    i["labels"] = sorted(set(i["labels"]) | {a[4]}) if add else [l for l in i["labels"] if l != a[4]]
    if changed:
        i["events"][-1].append({"event": "labeled" if add else "unlabeled", "label": {"name": a[4]},
                                "actor": {"login": "pelway-dev[bot]"}, "created_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())})
    save()
    if failing(kind + "-lost"): sys.exit(9)
    if add and failing("edit-add-crash"): os.kill(os.getppid(), 9)
    sys.exit(0)
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
  python3 -I -c 'import datetime as d,sys; print((d.datetime.now(d.timezone.utc) - d.timedelta(minutes=int(sys.argv[1]))).strftime("%Y-%m-%dT%H:%M:%SZ"))' "$1"; }
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
# issue_with <番号> <labeled_at> <ラベル...> — 開いた Issue を置く。イベントは 2 ページで、古い付け外し（labeled_at
# より前の needs-human の解除を含む）と labeled_at の後の別のラベルを混ぜる（最新の needs-decision の labeled だけが起点になる）。
issue_with() {
  gh_edit '
from datetime import datetime, timedelta
n, at = args[0], args[1]
t = lambda m: (datetime.fromisoformat(at.replace("Z", "+00:00")) + timedelta(minutes=m)).strftime("%Y-%m-%dT%H:%M:%SZ")
st[n] = {"state": "open", "labels": args[2:], "comments": [],
         "events": [[ev("labeled", "needs-decision", t(-40)), ev("unlabeled", "needs-human", t(-35)),
                     ev("unlabeled", "needs-decision", t(-30))],
                    [ev("labeled", "needs-decision", at), ev("labeled", "agent:working", t(1))]]}' "$@"
}
relabel() { gh_edit 'st[args[0]]["events"][-1].append(ev("labeled", "needs-decision", args[1]))' "$1" "$2"; }
set_labels() { gh_edit 'st[args[0]]["labels"] = args[1:]; st[args[0]].pop("list_labels", None)' "$@"; }
# GitHub の実際のラベルだけを変え、一覧（issue list）は前のまま見せる（読み直しの確認用）
set_actual_labels() { gh_edit 'i = st[args[0]]; i.setdefault("list_labels", i["labels"]); i["labels"] = args[1:]' "$@"; }
on_read() { printf '%s\n%s\n' "${4:-0}" "$3" > "$MOCK/on-$1-$2"; }  # on_read <issue|comments|events> <番号> <python の文> [見送る回数] — その読み取りの間に Issue（i）を変える
labels_of() { python3 -I -c 'import json,sys; print(" ".join(json.load(open(sys.argv[1]))[sys.argv[2]]["labels"]))' "$MOCK/gh.json" "$1"; }
add_comment() { gh_edit 'st[args[0]]["comments"].append({"user": {"login": args[1]}, "body": args[2]})' "$@"; }

threads() { printf '%s\n' "$1" > "$MOCK/threads.json"; }
live_thread() { threads "[{\"threadId\":\"$CONDUCTOR\",\"title\":\"指揮役\",\"status\":\"idle\",\"archived\":${1:-false},\"createdBy\":\"user\"}]"; }
reg() { printf '%s\n' "$1" > "$REG"; }

reset() { rm -rf "$PELWAY" "$ROOT/state/pelway" "$MOCK"/* "$REG"; mkdir -p "$PELWAY"; : > "$ROOT/out.log"; live_thread; }
run_cycles() {
  echo 0 > "$MOCK/sleeps"
  MOCK="$MOCK" SLEEP_MAX="$1" XDG_STATE_HOME="$ROOT/state" PATH="$ROOT/bin:$PATH" \
    T3_BIN="$ROOT/bin/t3" PELWAY_BOARD_QUEUE_DIR="$ROOT/queue" PELWAY_CONDUCTOR_FILE="$REG" \
    bash "$ROOT/repo/scripts/conductor-watch.sh" >>"$ROOT/out.log" 2>&1 &
  WPID=$!
  wait "$WPID" 2>/dev/null; WPID=""
}
# fail_first <失敗の種類> <番号> [labeled_at] — 新しい Issue で、$MOCK/fail-<種類> の呼び出しが失敗する 1 周を回す
fail_first() { reset; issue_with "$2" "${3:-$T20}" needs-decision; touch "$MOCK/fail-$1"; run_cycles 1; }
heal() { rm "$MOCK"/fail-*; : > "$MOCK/calls"; }  # 失敗を外し、書き込みの記録を空にする

sget() { python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["subjects"].get(sys.argv[2], {}).get(sys.argv[3], ""))' "$STATE" "$1" "$2"; }
nca_after() { # nca_after <番号> <時刻> — 補助記録の next_check_at が時刻の何秒後か
  python3 -I -c 'import json,sys,datetime
s = json.load(open(sys.argv[1]))["subjects"][sys.argv[2]]["next_check_at"]
base = datetime.datetime.fromisoformat(sys.argv[3].replace("Z", "+00:00"))
print(round(datetime.datetime.fromisoformat(s).timestamp() - base.timestamp()))' "$STATE" "$1" "$2"
}
record_of() { # record_of <番号> — 見張りの格上げの記録（escalation.json）の labeled_at と段階
  python3 -I -c 'import json,sys; r = json.load(open(sys.argv[1])).get(sys.argv[2], {}); print((r.get("labeled_at", "") + " " + r.get("stage", "")).strip())' \
    "$ROOT/state/pelway/conductor-watch/escalation.json" "$1"; }
decision_lines() { grep '^DECISION ' "$ROOT/out.log"; }
last_line() { decision_lines | tail -n 1; }
calls() { cat "$MOCK/calls" 2>/dev/null; }
reads_of() { grep -F "/issues/$1" "$MOCK/reads" 2>/dev/null; }
lines() { printf '%s\n' "$@"; }
# 期待値: body_for <labeled_at> は格上げのコメントの本文（定型）。labeled_at の既定は $T20、理由の既定は no-registration
body_for() { printf '%s\n%s' '<!-- conductor-watch-escalation -->' "@hatai \`needs-decision\` が付いた時刻 $1 から 15 分決まらず、指揮役が動いていないため \`needs-human\` に替えました。"; }
overdue() { echo "DECISION overdue #$1 labeled_at=${2:-$T20}"; }
escalated() { echo "DECISION escalated #$1 labeled_at=$T20 reason=${2:-no-registration}"; }
label_calls() { lines "issue edit $1 --add-label needs-human" "issue edit $1 --remove-label needs-decision"; }
escalation_calls() { lines "issue comment $1 --body-file -" "$(label_calls "$1")"; }

check() {
  local name="$1"; shift
  if "$@"; then printf 'ok   %s\n' "$name"; else printf 'FAIL %s\n' "$name"; failures+=("$name"); fi
}
eq() { [[ "$1" == "$2" ]]; }
lacks() { ! grep -qF -- "$2" "$1"; }

# --- 検知と補助記録（指揮役が動いている） ---

# 1. 15 分を過ぎたら DECISION overdue を 1 回。指揮役がいるので書き込みはしない。補助記録は grokbot。
reset; reg "$CONDUCTOR"; issue_with 701 "$T20" needs-decision
run_cycles 2
check "1: overdue line once for the same labeled_at (exact)" eq "$(decision_lines)" "$(overdue 701)"
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

# 3. 外れてから付け直されたら、新しい labeled_at で改めて数える。外れずに付け直しのイベントだけ増えたときも同じ
reset; reg "$CONDUCTOR"; issue_with 703 "$T40" needs-decision
run_cycles 1
set_labels 703
run_cycles 1
relabel 703 "$T20"; set_labels 703 needs-decision
run_cycles 1
relabel 703 "$T16"
run_cycles 1
check "3: re-labeling counts again from the new labeled_at" eq "$(decision_lines)" "$(lines "$(overdue 703 "$T40")" "$(overdue 703)" "$(overdue 703 "$T16")")"
check "3: next_check_at follows the new labeled_at" eq "$(nca_after 703 "$T16")" "900"

# 4. Issue ごとのイベントの取得に失敗した周は、行も書き込みも無く、次の周で取り直す
reset; issue_with 704 "$T20" needs-decision; touch "$MOCK/fail-events"
run_cycles 1
check "4: no line when the per-issue events fetch fails" eq "$(decision_lines)" ""
check "4: no writes when the per-issue events fetch fails" eq "$(calls)" ""
rm "$MOCK/fail-events"
run_cycles 1
check "4: next cycle fetches again and reports" eq "$(decision_lines)" "$(lines "$(overdue 704)" "$(escalated 704)")"

# 5. needs-decision と needs-human の両方が付いていれば human のまま、DECISION は出さない
reset; issue_with 705 "$T20" needs-decision needs-human
run_cycles 1
check "5: both labels -> waiting_on human" eq "$(sget 705 waiting_on)" "human"
check "5: both labels -> no DECISION line" eq "$(decision_lines)" ""
check "5: both labels -> no writes" eq "$(calls)" ""
check "5: both labels without the watcher's record -> no per-issue reads" eq "$(reads_of 705)" ""
check "5: both labels -> next_check_at is not labeled_at + 15 min" eval '[[ "$(nca_after 705 "$T20")" != 900 ]]'

# --- 指揮役が動いていないときの格上げ ---

# 6. 登録ファイルが無い: コメントを書いてから needs-human を付け、needs-decision を外す。1 回だけ。
reset; issue_with 706 "$T20" needs-decision
run_cycles 2
check "6: comment, then add needs-human, then remove needs-decision" eq "$(calls)" "$(escalation_calls 706)"
check "6: comment body is the fixed text only" cmp -s "$MOCK/body-706-1" <(body_for "$T20")
check "6: lines (exact)" eq "$(decision_lines)" "$(lines "$(overdue 706)" "$(escalated 706)")"
check "6: escalated issue waits on human" eq "$(sget 706 waiting_on)" "human"
check "6: the record is dropped once needs-decision is gone" eq "$(record_of 706)" ""

# 7〜9. 空・symlink・FIFO・ディレクトリの登録ファイルは、登録が無いものとして扱う（symlink はたどらない）
reset; issue_with 707 "$T20" needs-decision; : > "$REG"
run_cycles 1
check "7: empty registration -> escalated (exact)" eq "$(last_line)" "$(escalated 707)"
reset; issue_with 708 "$T20" needs-decision; printf '%s\n' "$CONDUCTOR" > "$ROOT/real-id"; ln -s "$ROOT/real-id" "$REG"
run_cycles 1
check "8: symlinked registration is not followed -> escalated" eq "$(last_line)" "$(escalated 708)"
reset; issue_with 709 "$T20" needs-decision; mkfifo "$REG"
run_cycles 1
check "9: FIFO registration -> escalated without blocking" eq "$(last_line)" "$(escalated 709)"
reset; issue_with 733 "$T20" needs-decision; mkdir "$REG"
run_cycles 1
check "9: directory registration -> escalated" eq "$(last_line)" "$(escalated 733)"

# 10〜11. 登録された ID が一覧に無い・archived のときも格上げする
reset; issue_with 710 "$T20" needs-decision; reg "$CONDUCTOR"; threads '[]'
run_cycles 1
check "10: registered thread missing -> escalated" eq "$(last_line)" "$(escalated 710 thread-missing)"
check "10: lines and record hold no thread id" eval '! grep -qF "$CONDUCTOR" "$ROOT/out.log" && ! grep -qF "$CONDUCTOR" "$STATE"'
reset; issue_with 711 "$T20" needs-decision; reg "$CONDUCTOR"; live_thread true
run_cycles 1
check "11: registered thread archived -> escalated" eq "$(last_line)" "$(escalated 711 thread-archived)"
check "11: escalation writes in order" eq "$(calls)" "$(escalation_calls 711)"

# 12. t3 thread list の取得に失敗した周は格上げしない（登録が無くても判定しない）。次の周で判定する
reset; issue_with 712 "$T20" needs-decision; reg "$CONDUCTOR"; threads '[]'; touch "$MOCK/fail-t3"
run_cycles 1
check "12: failed thread list -> overdue only" eq "$(decision_lines)" "$(overdue 712)"
check "12: failed thread list -> no writes" eq "$(calls)" ""
rm "$REG"
run_cycles 1
check "12: failed thread list without registration -> still no writes" eq "$(calls)" ""
rm "$MOCK/fail-t3"
run_cycles 1
check "12: next cycle with a working list escalates" eq "$(last_line)" "$(escalated 712)"

# 13. 読み直したときに needs-decision が外れている・needs-human が付いているなら、何も書かない
reset; issue_with 713 "$T20" needs-decision; set_actual_labels 713
run_cycles 1
check "13: label already removed on re-read -> no writes" eq "$(calls)" ""
check "13: label already removed on re-read -> overdue only" eq "$(decision_lines)" "$(overdue 713)"
reset; issue_with 714 "$T20" needs-decision; set_actual_labels 714 needs-decision needs-human
run_cycles 1
check "13: needs-human already added on re-read -> no writes" eq "$(calls)" ""

# 14. コメントに失敗: step=comment を出し、ラベルは変えない。次の周で読み直しからやり直す
fail_first comment 715
check "14: failed comment -> step=comment, no label change" eq "$(calls)" "issue comment 715 --body-file -"
check "14: failed comment line (exact)" eq "$(last_line)" "DECISION escalate-failed #715 step=comment"
heal
run_cycles 1
check "14: retried from the re-read on the next cycle" eq "$(calls)" "$(escalation_calls 715)"
check "14: escalated after retry" eq "$(last_line)" "$(escalated 715)"

# 15. コメントは書けてラベルの変更に失敗: step=label。次の周で同じコメントを 2 回目は書かない
fail_first edit-add 716
check "15: failed label change line (exact)" eq "$(last_line)" "DECISION escalate-failed #716 step=label"
heal
run_cycles 1
check "15: no second comment for the same labeled_at" eq "$(calls)" "$(label_calls 716)"
check "15: escalated after retry" eq "$(last_line)" "$(escalated 716)"

# 16. needs-human を付けた後に外すのが失敗したら両方残る。次の周で、同じ labeled_at の見張りの記録で自分の格上げと確かめて外し直す
fail_first edit-remove 717
check "16: failed removal -> step=label" eq "$(last_line)" "DECISION escalate-failed #717 step=label"
check "16: failed removal leaves both labels" eq "$(labels_of 717)" "needs-decision needs-human"
check "16: the watcher's record is stage added for the labeled_at" eq "$(record_of 717)" "$T20 added"
: > "$MOCK/calls"
run_cycles 1
check "16: retried removal fails again -> only the removal is retried" eq "$(calls)" "issue edit 717 --remove-label needs-decision"
check "16: retried removal fails again -> step=label (exact)" eq "$(decision_lines)" \
  "$(lines "$(overdue 717)" "DECISION escalate-failed #717 step=label" "DECISION escalate-failed #717 step=label")"
heal
run_cycles 1
check "16: next cycle removes needs-decision only" eq "$(calls)" "issue edit 717 --remove-label needs-decision"
check "16: escalated after retry" eq "$(last_line)" "$(escalated 717)"
check "16: the notice is written once" eq "$(find "$MOCK" -name 'body-717-*' | wc -l)" "1"
check "16: labels end as needs-human" eq "$(labels_of 717)" "needs-human"
check "16: issue waits on human" eq "$(sget 717 waiting_on)" "human"
# 外すのに失敗した後に指揮役が動き始めたら、見張りは外し直さない（ラベルを替えるのは指揮役がいない周だけ）
fail_first edit-remove 717; heal; : > "$MOCK/reads"; reg "$CONDUCTOR"
run_cycles 1
check "16: conductor running -> the leftover removal is not retried" eq "$(calls)" ""
check "16: conductor running -> the leftover issue is not read" eq "$(reads_of 717)" ""

# 17. 同じ目印・同じ本文でも、pelway-dev[bot] 以外のコメントは書いた印にしない
reset; issue_with 718 "$T20" needs-decision; add_comment 718 stranger "$(body_for "$T20")"
run_cycles 1
check "17: a stranger's marker comment does not suppress the notice" eq "$(calls)" "$(escalation_calls 718)"
# 18. 前の labeled_at の格上げのコメントは、新しい labeled_at の書いた印にしない
reset; issue_with 719 "$T20" needs-decision; add_comment 719 'pelway-dev[bot]' "$(body_for "$T40")"
run_cycles 1
check "18: a comment for an older labeled_at does not suppress the notice" eq "$(calls)" "$(escalation_calls 719)"
# 19. 登録ファイルは 1 行目だけを読む（pelway-agent-watch の parse_conductor_file と同じ）
reset; issue_with 720 "$T20" needs-decision; printf '%s\nsecond line\n' "$CONDUCTOR" > "$REG"
run_cycles 1
check "19: only the first line of the registration is the thread id" eq "$(calls)" ""

# 20. 期限切れと判定した後、コメントを読んでから書くまでの間に外して付け直されたら（最新の labeled_at が変わったら）、何も書かない
reset; issue_with 721 "$T20" needs-decision; on_read comments 721 "i['events'][-1].append(ev('labeled', 'needs-decision', '$T5'))"
run_cycles 2
check "20: re-labeled between reads -> no writes" eq "$(calls)" ""
check "20: re-labeled between reads -> overdue for the old labeled_at only" eq "$(decision_lines)" "$(overdue 721)"
# 21. 外し直しの前にも同じ確認をする。根拠は見張りの記録だけで、同じ App の名義の定型コメントがあっても記録が無ければ外さず、読みもしない
fail_first edit-remove 722; heal; on_read events 722 "i['events'][-1].append(ev('labeled', 'needs-decision', '$T5'))" 1
run_cycles 1
check "21: re-labeled before the retried removal -> no writes" eq "$(calls)" ""
fail_first edit-remove 745; heal; on_read issue 745 "i['events'][-1].append(ev('labeled', 'needs-decision', '$T5'))"
run_cycles 1
check "21: re-labeled during the issue re-read -> no writes" eq "$(calls)" ""
reset; issue_with 723 "$T20" needs-decision needs-human; add_comment 723 'pelway-dev[bot]' "$(body_for "$T20")"
run_cycles 1
check "21: the template comment without the watcher's record -> needs-decision is not removed" eq "$(calls)" ""
check "21: the template comment without the watcher's record -> no per-issue reads" eq "$(reads_of 723)" ""
# 22. 同じ pelway-dev[bot] の名義でも、定型の本文と完全には一致しないコメントは書いた印にしない
reset; issue_with 724 "$T20" needs-decision
add_comment 724 'pelway-dev[bot]' "$(printf '%s\n%s' '<!-- conductor-watch-escalation -->' "needs-decision の時刻 $T20 から 15 分たちました。")"
run_cycles 1
check "22: a same-App comment that is not the template does not suppress the notice" eq "$(calls)" "$(escalation_calls 724)"

# 23. 登録ファイルがあってほかの理由で読めない（権限が無い）ときは、指揮役がいないと確かめられないので書かない
reset; issue_with 725 "$T20" needs-decision; reg "$CONDUCTOR"; chmod 000 "$REG"
run_cycles 1
check "23: unreadable registration -> overdue only (exact)" eq "$(decision_lines)" "$(overdue 725)"
check "23: unreadable registration -> no writes" eq "$(calls)" ""
# 24. コメントを読んでから書くまでの間に needs-decision が外された（付け直されない）・needs-human が付いたら、書かない
reset; issue_with 726 "$T20" needs-decision; on_read comments 726 "i['labels'].remove('needs-decision')"
run_cycles 1
check "24: needs-decision removed after the first read -> no writes" eq "$(calls)" ""
reset; issue_with 727 "$T20" needs-decision; on_read comments 727 "i['labels'].append('needs-human')"
run_cycles 1
check "24: needs-human added after the first read -> no writes" eq "$(calls)" ""
reset; issue_with 731 "$T20" needs-decision; on_read comments 731 "i['state'] = 'closed'"
run_cycles 1
check "24: issue closed after the first read -> no writes" eq "$(calls)" ""
# 25. 見張りのコメントの後に needs-human を付けるのに失敗し、ほかの誰かが needs-human を付けたら、needs-decision を外さない
fail_first edit-add 728; heal; set_labels 728 needs-decision needs-human
run_cycles 1
check "25: someone else's needs-human -> needs-decision is not removed" eq "$(calls)" ""
check "25: someone else's needs-human -> both labels stay" eq "$(labels_of 728)" "needs-decision needs-human"
# 26. 見張りが付けた needs-human を、needs-decision を外し損ねている間に人間が外したら、付け直さずに needs-decision だけを外す
fail_first edit-remove 729; heal
gh_edit 'i = st[args[0]]; i["labels"] = ["needs-decision"]; i["events"][-1].append(ev("unlabeled", "needs-human", args[1]))' 729 "$T5"
run_cycles 1
check "26: human removed needs-human -> only needs-decision is removed" eq "$(calls)" "issue edit 729 --remove-label needs-decision"
check "26: human removed needs-human -> neither label stays" eq "$(labels_of 729)" ""
# 27. 付与の試行（`adding` の記録）のあとにほかの名義の needs-human の付け外しがあれば、出どころを確かめられないので `held` にして付け直さない。通知前なら、試行前の付け外しは試行の照合に入らないので通常どおり進める
fail_first edit-add 730; heal
gh_edit 'st[args[0]]["events"][-1] += [ev("labeled", "needs-human", args[1]), ev("unlabeled", "needs-human", args[2])]' 730 "$T16" "$T5"
run_cycles 1
check "27: resumed escalation, needs-human removed after labeled_at -> not added again" eq "$(calls)" ""
reset; issue_with 734 "$T20" needs-decision
gh_edit 'st[args[0]]["events"][-1] += [ev("labeled", "needs-human", args[1]), ev("unlabeled", "needs-human", args[2])]' 734 "$T16" "$T5"
run_cycles 1
check "27: fresh escalation, needs-human removed after labeled_at -> escalated as usual" eq "$(calls)" "$(escalation_calls 734)"
# 28. 記録は同じ labeled_at のものだけを使う。外し損ねの記録の後に外して付け直されたら、新しい labeled_at の格上げを初めからする
fail_first edit-remove 732 "$T40"; heal
gh_edit 'i = st[args[0]]; i["labels"] = ["needs-decision"]
i["events"][-1] += [ev("unlabeled", "needs-human", args[1]), ev("unlabeled", "needs-decision", args[1]), ev("labeled", "needs-decision", args[2])]' 732 "$(ago 30)" "$T20"
run_cycles 1
check "28: a record for an older labeled_at is not reused" eq "$(calls)" "$(escalation_calls 732)"

# --- 試行の記録と GitHub のラベル履歴での照合（記録は段ごとにその場で保存する） ---

# 29. 試行の前に needs-human が付け外しされていても、一時的に付けられなかったら次の周に付け直す
reset; issue_with 735 "$T20" needs-decision; touch "$MOCK/fail-edit-add"
gh_edit 'st[args[0]]["events"][-1] += [ev("labeled", "needs-human", args[1]), ev("unlabeled", "needs-human", args[2])]' 735 "$T16" "$T5"
run_cycles 1
heal
run_cycles 1
check "29: removed before the attempt, transient failure -> added again, then needs-decision removed" eq "$(calls)" "$(label_calls 735)"
# 30. 付いたのに応答が失われたら、記録の試行の後の自分の名義の labeled で付けたと確かめ、needs-decision だけを外す。
# その間に人間が needs-human を外していても付け直さない。付けた直後に保存の前で止まっても同じ
fail_first edit-add-lost 736
check "30: add applied but the response lost -> step=label (exact)" eq "$(last_line)" "DECISION escalate-failed #736 step=label"
check "30: the attempt is recorded before the call" eq "$(record_of 736)" "$T20 adding"
heal
run_cycles 1
check "30: lost response -> next cycle removes needs-decision only" eq "$(calls)" "issue edit 736 --remove-label needs-decision"
check "30: lost response -> escalated (exact)" eq "$(last_line)" "$(escalated 736)"
check "30: lost response -> labels end as needs-human" eq "$(labels_of 736)" "needs-human"
fail_first edit-add-lost 737; heal
gh_edit 'i = st[args[0]]; i["labels"].remove("needs-human"); i["events"][-1].append(ev("unlabeled", "needs-human", args[1]))' 737 "$T5"
run_cycles 1
check "30: removed by a human after the attempt -> not added again, needs-decision removed" eq "$(calls)" "issue edit 737 --remove-label needs-decision"
fail_first edit-add-crash 738; heal
run_cycles 1
check "30: stopped before saving -> next cycle removes needs-decision only" eq "$(calls)" "issue edit 738 --remove-label needs-decision"
check "30: stopped before saving -> labels end as needs-human" eq "$(labels_of 738)" "needs-human"
# 31. 試行の後にほかの誰かが needs-human を付けたら、held を残して DECISION held を出し、needs-decision は外さない。以後は読まない
fail_first edit-add 739; heal
gh_edit 'i = st[args[0]]; i["labels"].append("needs-human"); i["events"][-1].append(ev("labeled", "needs-human", args[1]))' 739 "$T5"
run_cycles 1
check "31: someone else's needs-human after the attempt -> no writes" eq "$(calls)" ""
check "31: someone else's needs-human after the attempt -> held (exact)" eq "$(last_line)" "DECISION held #739 labeled_at=$T20"
: > "$MOCK/reads"
run_cycles 1
check "31: held -> the issue is not read again" eq "$(reads_of 739)" ""
# 32. 書く直前の events の取得の間に needs-decision が外されたら書かない。返った events に unlabeled があれば、その後の
# Issue の読み取りで付き直っていても（新しい付与）書かず、unlabeled がまだ無ければ events の後に読む Issue で分かる
reset; issue_with 740 "$T20" needs-decision
on_read events 740 "i['labels'].remove('needs-decision'); i['events'][-1].append(ev('unlabeled', 'needs-decision', '$T5'))" 1
on_read issue 740 "i['labels'] += [] if 'needs-decision' in i['labels'] else ['needs-decision']"
run_cycles 1
check "32: removed during the events fetch (unlabeled returned), back on the issue read -> no writes" eq "$(calls)" ""
reset; issue_with 741 "$T20" needs-decision; on_read events 741 "i['labels'].remove('needs-decision')" 1
run_cycles 1
check "32: removed during the events fetch (not yet in events) -> no writes" eq "$(calls)" ""
# 33. 付いたのに応答が失われ、その labeled が events にまだ出ていないうちは、held にせず次の周に回す
fail_first edit-add-lost 742; heal; gh_edit 'json.dump(st[args[0]]["events"][-1].pop(), open(args[1], "w"))' 742 "$MOCK/lagged"
run_cycles 1
check "33: added but not yet in events -> no writes, the attempt kept" eq "$(calls)|$(record_of 742)" "|$T20 adding"
gh_edit 'st[args[0]]["events"][-1].append(json.load(open(args[1])))' 742 "$MOCK/lagged"
run_cycles 1
check "33: once in events -> needs-decision removed" eq "$(calls)" "issue edit 742 --remove-label needs-decision"
# 34. 通知を書いた後、付ける前にほかの誰かの needs-human が付いたら（試行の記録が無い）、held を残して止まる
reset; issue_with 743 "$T20" needs-decision
on_read events 743 "i['labels'].append('needs-human'); i['events'][-1].append(ev('labeled', 'needs-human', '$T5'))" 2
run_cycles 1
check "34: someone else's needs-human after the notice -> held, needs-decision kept" eq "$(calls)|$(last_line)" \
  "issue comment 743 --body-file -|DECISION held #743 labeled_at=$T20"
# 定型コメントだけで見張りの記録が無い（ほかの担当が書いた）ときは、別の担当の needs-human で held にせず、外れたら格上げを続ける
reset; issue_with 744 "$T20" needs-decision; add_comment 744 'pelway-dev[bot]' "$(body_for "$T20")"
on_read issue 744 "i['labels'].append('needs-human')"
run_cycles 1
check "34: template comment without a record, another's needs-human -> no writes" eq "$(calls)" ""
check "34: template comment without a record, another's needs-human -> no held" eq "$(record_of 744)" ""
set_labels 744 needs-decision; : > "$MOCK/calls"
run_cycles 1
check "34: after the other's needs-human is removed -> the notice is not reposted" eq "$(calls)" "$(label_calls 744)"
check "34: after the other's needs-human is removed -> escalated" eq "$(last_line)" "$(escalated 744)"
# 35. needs-human の付与が変更なしで成功しても（直前にほかの担当が付けた）added にせず、試行と付与イベントの照合で held にする
reset; issue_with 746 "$T20" needs-decision
on_read edit 746 "i['labels'].append('needs-human'); i['events'][-1].append(ev('labeled', 'needs-human', '$T5'))"
run_cycles 1
check "35: no-op add success -> needs-decision is not removed" eq "$(calls)" "$(lines "issue comment 746 --body-file -" "issue edit 746 --add-label needs-human")"
check "35: no-op add success -> held, not added" eq "$(record_of 746)" "$T20 held"
check "35: no-op add success -> held line (exact)" eq "$(last_line)" "DECISION held #746 labeled_at=$T20"
check "35: no-op add success -> both labels stay" eq "$(labels_of 746)" "needs-decision needs-human"
# 36. needs-decision を外す応答が失われても、外れていれば次の周で完了を報告してから記録を整理する
fail_first edit-remove-lost 747
check "36: removal response lost -> step=label" eq "$(last_line)" "DECISION escalate-failed #747 step=label"
check "36: removal response lost -> needs-decision is actually gone" eq "$(labels_of 747)" "needs-human"
check "36: removal response lost -> record kept as added" eq "$(record_of 747)" "$T20 added"
: > "$MOCK/calls"
run_cycles 1
check "36: removal confirmed -> escalated reported (exact)" eq "$(last_line)" "$(escalated 747)"
check "36: removal confirmed -> record cleaned" eq "$(record_of 747)" ""
check "36: removal confirmed -> no writes" eq "$(calls)" ""
# 37. 途中で Issue が閉じたら記録を残し、再オープン後に解除の再試行を再開する
fail_first edit-remove 748
check "37: mid-flight record is added" eq "$(record_of 748)" "$T20 added"
gh_edit 'i = st[args[0]]; i["state"] = "closed"' 748; : > "$MOCK/calls"
run_cycles 1
check "37: closed mid-flight -> no writes" eq "$(calls)" ""
check "37: closed mid-flight -> record kept" eq "$(record_of 748)" "$T20 added"
gh_edit 'i = st[args[0]]; i["state"] = "open"' 748; heal
run_cycles 1
check "37: reopened -> removal retried" eq "$(calls)" "issue edit 748 --remove-label needs-decision"
check "37: reopened -> escalated" eq "$(last_line)" "$(escalated 748)"
check "37: reopened -> labels end as needs-human" eq "$(labels_of 748)" "needs-human"

if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}"
  exit 1
fi
printf '\nall passed\n'
