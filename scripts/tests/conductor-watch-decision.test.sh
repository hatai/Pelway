#!/usr/bin/env bash
# scripts/conductor-watch.sh の needs-decision の 15 分の検知（DECISION overdue）と、指揮役が動いていないときの
# needs-human への格上げ（Issue #602）のテスト。モックの agent-gh が GitHub の状態を $MOCK/gh.json に持ち、書き込みの引数を
# $MOCK/calls に、読み取りの path を $MOCK/reads に記録する。読み取りの間の変更は on_read で差し込む。モックの t3 が一覧を返す。
set -uo pipefail

# --only <番号>[,<番号>...] で選んだ場面だけを実行する。引数なしは全場面。
ONLY="" have_only=0
usage() { echo "usage: $0 [--only <番号>[,<番号>...]]" >&2; exit 2; }
while (($#)); do
  case "$1" in
    --only) [[ $# -ge 2 ]] || usage; have_only=1; ONLY="$2"; shift ;;
    --only=*) have_only=1; ONLY="${1#--only=}" ;;
    *) usage ;;
  esac
  shift
done
((have_only)) && [[ ! "$ONLY" =~ ^[0-9]+(,[0-9]+)*$ ]] && usage

matched="" # --only で実行した場面の番号（`,6,13` の形。照合は `,$matched,` と両端にカンマを足してから行う）
want() { # want <番号> — その場面を実行するか。実行する番号は matched に残す
  ((have_only)) || return 0
  [[ ",$ONLY," == *",$1,"* ]] || return 1
  matched+=",$1"
}

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ROOT="$(mktemp -d)"
WPID=""
trap '[[ -n "${WPID:-}" ]] && kill "$WPID" 2>/dev/null; rm -rf "$ROOT"' EXIT
failures=()

MOCK="$ROOT/mock"
mkdir -p "$ROOT/repo/scripts" "$ROOT/bin" "$MOCK" "$ROOT/state"
cp "$REPO/scripts/conductor-watch.sh" "$ROOT/repo/scripts/"
cp "$REPO/scripts/patrol-state-update" "$ROOT/repo/scripts/"
git init -q "$ROOT/repo"
PELWAY="$ROOT/repo/.git/pelway"
STATE="$PELWAY/patrol-state.json"
REG="$ROOT/conductor-thread-id"
CONDUCTOR="conductor-thread-7f3a"

# モックの agent-gh。$MOCK/fail-<events|reread|comment|edit-add|edit-remove> があると、その呼び出しは失敗する。-lost の付く
# ものはラベルを変えてから失敗を返し（応答が失われた）、fail-edit-add-crash は付けた直後に見張りを止める。ラベルが変わったときだけ pelway-dev[bot] 名義のイベント（増える ID 付き）を残す。
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
# issue_with <番号> <labeled_at> <ラベル...> — 開いた Issue を置く。イベントは 2 ページで、古い付け外し（labeled_at より前の needs-human の解除を含む）と後の別ラベルを混ぜる。
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

reset() { rm -rf "$PELWAY" "$ROOT/state/pelway" "${MOCK:?}"/* "$REG"; mkdir -p "$PELWAY"; : > "$ROOT/out.log"; live_thread; }
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
retry() { heal; run_cycles 1; }
again() { : > "$MOCK/calls"; run_cycles 1; }  # 書き込みの記録を空にして 1 周回す
# once <番号> [準備のコマンド...] — 新しい Issue（labeled_at=$T20）で準備のコマンドを実行してから ${CYCLES:-1} 周回す。準備の失敗は FAIL
once() { local n="$1"; shift; reset; issue_with "$n" "$T20" needs-decision
  if (($#)) && ! "$@"; then printf 'FAIL setup #%s\n' "$n"; failures+=("setup #$n"); fi; run_cycles "${CYCLES:-1}"; }
# flip_human <番号> — labeled_at の後に needs-human が付いて（$T16）外れた（$T5）イベントを足す
flip_human() { gh_edit 'st[args[0]]["events"][-1] += [ev("labeled", "needs-human", args[1]), ev("unlabeled", "needs-human", args[2])]' "$1" "$T16" "$T5"; }
# other_adds_human <番号> — ほかの誰かが needs-human を付けた（$T5）
other_adds_human() { gh_edit 'i = st[args[0]]; i["labels"].append("needs-human"); i["events"][-1].append(ev("labeled", "needs-human", args[1]))' "$1" "$T5"; }

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

check() { local name="$1"; shift; if "$@"; then printf 'ok   %s\n' "$name"; else printf 'FAIL %s\n' "$name"; failures+=("$name"); fi; }
eq() { [[ "$1" == "$2" ]]; }
lacks() { ! grep -qF -- "$2" "$1"; }

# --- 検知と補助記録（指揮役が動いている） ---

# 1. 15 分を過ぎたら DECISION overdue を 1 回。指揮役がいるので書き込みはしない。補助記録は grokbot。
if want 1; then
reset; reg "$CONDUCTOR"; issue_with 701 "$T20" needs-decision
run_cycles 2
check "1: overdue line once for the same labeled_at (exact), conductor running -> no comment or label change" eq "$(decision_lines)|$(calls)" "$(overdue 701)|"
check "1: record waiting_on and next_owner are grokbot, next_check_at is labeled_at + 15 min" eq "$(sget 701 waiting_on) $(sget 701 next_owner) $(nca_after 701 "$T20")" "grokbot grokbot 900"
check "1: record holds no thread id" lacks "$STATE" "$CONDUCTOR"
fi

# 2. 15 分より前は出さない（指揮役がいなくても書き込まない）
if want 2; then
reset; issue_with 702 "$T5" needs-decision; run_cycles 2
check "2: no line and no writes before 15 minutes, next_check_at is labeled_at + 15 min" eq "$(decision_lines)|$(calls)|$(nca_after 702 "$T5")" "||900"
fi

# 3. 外れてから付け直されたら、新しい labeled_at で改めて数える。外れずに付け直しのイベントだけ増えたときも同じ
if want 3; then
reset; reg "$CONDUCTOR"; issue_with 703 "$T40" needs-decision; run_cycles 1
set_labels 703; run_cycles 1
relabel 703 "$T20"; set_labels 703 needs-decision; run_cycles 1
relabel 703 "$T16"; run_cycles 1
check "3: re-labeling counts again from the new labeled_at, next_check_at follows it" eq "$(decision_lines)|$(nca_after 703 "$T16")" "$(lines "$(overdue 703 "$T40")" "$(overdue 703)" "$(overdue 703 "$T16")")|900"
fi

# 4. Issue ごとのイベントの取得に失敗した周は、行も書き込みも無く、次の周で取り直す
if want 4; then
fail_first events 704
check "4: no line and no writes when the per-issue events fetch fails" eq "$(decision_lines)|$(calls)" "|"
rm "$MOCK/fail-events"; run_cycles 1
check "4: next cycle fetches again and reports" eq "$(decision_lines)" "$(lines "$(overdue 704)" "$(escalated 704)")"
fi

# 5. needs-decision と needs-human の両方が付いていれば human のまま、DECISION は出さない
if want 5; then
reset; issue_with 705 "$T20" needs-decision needs-human; run_cycles 1
check "5: both labels -> waiting_on human, no DECISION line, no writes, no per-issue reads without the watcher's record" eq "$(sget 705 waiting_on)|$(decision_lines)|$(calls)|$(reads_of 705)" "human|||"
check "5: both labels -> next_check_at is a readable datetime, not labeled_at + 15 min" eval 'v="$(nca_after 705 "$T20")" && [[ "$v" =~ ^-?[0-9]+$ && "$v" != 900 ]]'
fi

# --- 指揮役が動いていないときの格上げ ---

# 6. 登録ファイルが無い: コメントを書いてから needs-human を付け、needs-decision を外す。1 回だけ。
if want 6; then
CYCLES=2 once 706
check "6: comment, then add needs-human, then remove needs-decision; lines (exact)" eq "$(calls)|$(decision_lines)" "$(escalation_calls 706)|$(lines "$(overdue 706)" "$(escalated 706)")"
check "6: comment body is the fixed text only" cmp -s "$MOCK/body-706-1" <(body_for "$T20")
check "6: escalated issue waits on human, the record is dropped once needs-decision is gone" eq "$(sget 706 waiting_on)|$(record_of 706)" "human|"
fi

# 7〜9. 空・symlink・FIFO・ディレクトリの登録ファイルは、登録が無いものとして扱う（symlink はたどらない）
if want 7; then
once 707 cp /dev/null "$REG"
check "7: empty registration -> escalated (exact)" eq "$(last_line)" "$(escalated 707)"
fi
if want 8; then
printf '%s\n' "$CONDUCTOR" > "$ROOT/real-id"; once 708 ln -s "$ROOT/real-id" "$REG"
check "8: symlinked registration is not followed -> escalated" eq "$(last_line)" "$(escalated 708)"
fi
if want 9; then
once 709 mkfifo "$REG"
check "9: FIFO registration -> escalated without blocking" eq "$(last_line)" "$(escalated 709)"
once 733 mkdir "$REG"
check "9: directory registration -> escalated" eq "$(last_line)" "$(escalated 733)"
fi

# 10〜11. 登録された ID が一覧に無い・archived のときも格上げする
if want 10; then
reset; issue_with 710 "$T20" needs-decision; reg "$CONDUCTOR"; threads '[]'; run_cycles 1
check "10: registered thread missing -> escalated" eq "$(last_line)" "$(escalated 710 thread-missing)"
check "10: lines and record hold no thread id" eval '! grep -qF "$CONDUCTOR" "$ROOT/out.log" && ! grep -qF "$CONDUCTOR" "$STATE"'
fi
if want 11; then
reset; issue_with 711 "$T20" needs-decision; reg "$CONDUCTOR"; live_thread true; run_cycles 1
check "11: registered thread archived -> escalated, escalation writes in order" eq "$(last_line)|$(calls)" "$(escalated 711 thread-archived)|$(escalation_calls 711)"
fi

# 12. t3 thread list の取得に失敗した周は格上げしない（登録が無くても判定しない）。次の周で判定する
if want 12; then
reset; issue_with 712 "$T20" needs-decision; reg "$CONDUCTOR"; threads '[]'; touch "$MOCK/fail-t3"; run_cycles 1
check "12: failed thread list -> overdue only, no writes" eq "$(decision_lines)|$(calls)" "$(overdue 712)|"
rm "$REG"; run_cycles 1
check "12: failed thread list without registration -> still no writes" eq "$(calls)" ""
rm "$MOCK/fail-t3"; run_cycles 1
check "12: next cycle with a working list escalates" eq "$(last_line)" "$(escalated 712)"
fi

# 13. 読み直したときに needs-decision が外れている・needs-human が付いているなら、何も書かない
if want 13; then
once 713 set_actual_labels 713
check "13: label already removed on re-read -> overdue only, no writes" eq "$(decision_lines)|$(calls)" "$(overdue 713)|"
once 714 set_actual_labels 714 needs-decision needs-human
check "13: needs-human already added on re-read -> no writes" eq "$(calls)" ""
fi

# 14. コメントに失敗: step=comment を出し、ラベルは変えない。次の周で読み直しからやり直す
if want 14; then
fail_first comment 715
check "14: failed comment -> step=comment line (exact), no label change" eq "$(calls)|$(last_line)" "issue comment 715 --body-file -|DECISION escalate-failed #715 step=comment"
retry
check "14: retried from the re-read on the next cycle, escalated after retry" eq "$(calls)|$(last_line)" "$(escalation_calls 715)|$(escalated 715)"
fi

# 15. コメントは書けてラベルの変更に失敗: step=label。次の周で同じコメントを 2 回目は書かない
if want 15; then
fail_first edit-add 716
check "15: failed label change line (exact)" eq "$(last_line)" "DECISION escalate-failed #716 step=label"
retry
check "15: no second comment for the same labeled_at, escalated after retry" eq "$(calls)|$(last_line)" "$(label_calls 716)|$(escalated 716)"
fi

# 16. needs-human を付けた後に外すのが失敗したら両方残る。次の周で、同じ labeled_at の見張りの記録で自分の格上げと確かめて外し直す
if want 16; then
fail_first edit-remove 717
check "16: failed removal -> step=label, both labels left, the watcher's record is stage added for the labeled_at" \
  eq "$(last_line)|$(labels_of 717)|$(record_of 717)" "DECISION escalate-failed #717 step=label|needs-decision needs-human|$T20 added"
again
check "16: retried removal fails again -> only the removal is retried, step=label (exact)" eq "$(calls)|$(decision_lines)" "issue edit 717 --remove-label needs-decision|$(lines "$(overdue 717)" "DECISION escalate-failed #717 step=label" "DECISION escalate-failed #717 step=label")"
retry
check "16: next cycle removes needs-decision only, escalated after retry" eq "$(calls)|$(last_line)" "issue edit 717 --remove-label needs-decision|$(escalated 717)"
check "16: the notice is written once, labels end as needs-human, issue waits on human" eq "$(find "$MOCK" -name 'body-717-*' | wc -l)|$(labels_of 717)|$(sget 717 waiting_on)" "1|needs-human|human"
# 外すのに失敗した後に指揮役が動き始めたら、見張りは外し直さない（ラベルを替えるのは指揮役がいない周だけ）
fail_first edit-remove 717; heal; : > "$MOCK/reads"; reg "$CONDUCTOR"; run_cycles 1
check "16: conductor running -> the leftover removal is not retried" eq "$(calls)" ""
fi

# 17. 同じ目印・同じ本文でも、pelway-dev[bot] 以外のコメントは書いた印にしない
if want 17; then
once 718 add_comment 718 stranger "$(body_for "$T20")"
check "17: a stranger's marker comment does not suppress the notice" eq "$(calls)" "$(escalation_calls 718)"
fi
# 18. 前の labeled_at の格上げのコメントは、新しい labeled_at の書いた印にしない
if want 18; then
once 719 add_comment 719 'pelway-dev[bot]' "$(body_for "$T40")"
check "18: a comment for an older labeled_at does not suppress the notice" eq "$(calls)" "$(escalation_calls 719)"
fi
# 19. 登録ファイルは 1 行目だけを読む（pelway-agent-watch の parse_conductor_file と同じ）
if want 19; then
reset; issue_with 720 "$T20" needs-decision; printf '%s\nsecond line\n' "$CONDUCTOR" > "$REG"; run_cycles 1
check "19: only the first line of the registration is the thread id" eq "$(calls)" ""
fi

# 20. 期限切れと判定した後、コメントを読んでから書くまでの間に外して付け直されたら（最新の labeled_at が変わったら）、何も書かない
if want 20; then
CYCLES=2 once 721 on_read comments 721 "i['events'][-1].append(ev('labeled', 'needs-decision', '$T5'))"
check "20: re-labeled between reads -> overdue for the old labeled_at only, no writes" eq "$(decision_lines)|$(calls)" "$(overdue 721)|"
fi
# 21. 外し直しの前にも同じ確認をする。根拠は見張りの記録だけで、同じ App の名義の定型コメントがあっても記録が無ければ外さず、読みもしない
if want 21; then
fail_first edit-remove 722; heal; on_read events 722 "i['events'][-1].append(ev('labeled', 'needs-decision', '$T5'))" 1; run_cycles 1
check "21: re-labeled before the retried removal -> no writes" eq "$(calls)" ""
fail_first edit-remove 745; heal; on_read issue 745 "i['events'][-1].append(ev('labeled', 'needs-decision', '$T5'))"; run_cycles 1
check "21: re-labeled during the issue re-read -> no writes" eq "$(calls)" ""
reset; issue_with 723 "$T20" needs-decision needs-human; add_comment 723 'pelway-dev[bot]' "$(body_for "$T20")"; run_cycles 1
check "21: the template comment without the watcher's record -> needs-decision is not removed, no per-issue reads" eq "$(calls)|$(reads_of 723)" "|"
fi
# 22. 同じ pelway-dev[bot] の名義でも、定型の本文と完全には一致しないコメントは書いた印にしない
if want 22; then
once 724 add_comment 724 'pelway-dev[bot]' "$(printf '%s\n%s' '<!-- conductor-watch-escalation -->' "needs-decision の時刻 $T20 から 15 分たちました。")"
check "22: a same-App comment that is not the template does not suppress the notice" eq "$(calls)" "$(escalation_calls 724)"
fi

# 23. 登録ファイルがあってほかの理由で読めない（権限が無い）ときは、指揮役がいないと確かめられないので書かない
if want 23; then
reset; issue_with 725 "$T20" needs-decision; reg "$CONDUCTOR"; chmod 000 "$REG"; run_cycles 1
check "23: unreadable registration -> overdue only (exact), no writes" eq "$(decision_lines)|$(calls)" "$(overdue 725)|"
fi
# 24. コメントを読んでから書くまでの間に needs-decision が外された（付け直されない）・needs-human が付いたら、書かない
if want 24; then
once 726 on_read comments 726 "i['labels'].remove('needs-decision')"
check "24: needs-decision removed after the first read -> no writes" eq "$(calls)" ""
once 727 on_read comments 727 "i['labels'].append('needs-human')"
check "24: needs-human added after the first read -> no writes" eq "$(calls)" ""
once 731 on_read comments 731 "i['state'] = 'closed'"
check "24: issue closed after the first read -> no writes" eq "$(calls)" ""
# 付与・開閉は Issue の読み取りと最後の events の取得の間にも確かめ直す（events の方が新しい。ラベル一覧はまだ古い）。通知の前なので何も書かない
once 755 on_read issue 755 "i['events'][-1].append(ev('labeled', 'needs-human', '$T5'))"
check "24: needs-human event between the issue read and the last events fetch -> no writes" eq "$(calls)" ""
once 756 on_read issue 756 "i['events'][-1].append(ev('closed', '', '$T5'))"
check "24: closed event between the issue read and the last events fetch -> no writes" eq "$(calls)" ""
reset; issue_with 762 "$T20" needs-decision; gh_edit 'st[args[0]]["events"][0][:0] = [ev("closed", "", args[1]), ev("reopened", "", args[1])]' 762 "$(ago 60)"
on_read issue 762 "i['state'] = 'closed'"; run_cycles 1
check "24: issue read as closed while events still end with an old reopened -> no writes" eq "$(calls)" ""
fi
# 25. 見張りのコメントの後に needs-human を付けるのに失敗し、ほかの誰かが needs-human を付けたら、needs-decision を外さない
if want 25; then
fail_first edit-add 728; heal; set_labels 728 needs-decision needs-human; run_cycles 1
check "25: someone else's needs-human -> needs-decision is not removed, both labels stay" eq "$(calls)|$(labels_of 728)" "|needs-decision needs-human"
fi
# 26. 見張りが付けた needs-human を、needs-decision を外し損ねている間に人間が外したら、付け直さずに needs-decision だけを外す
if want 26; then
fail_first edit-remove 729; heal
gh_edit 'i = st[args[0]]; i["labels"] = ["needs-decision"]; i["events"][-1].append(ev("unlabeled", "needs-human", args[1]))' 729 "$T5"; run_cycles 1
check "26: human removed needs-human -> only needs-decision is removed, neither label stays" eq "$(calls)|$(labels_of 729)" "issue edit 729 --remove-label needs-decision|"
fi
# 27. 付与の試行（`adding` の記録）のあとにほかの名義の needs-human の付け外しがあれば、出どころを確かめられないので `held` にして付け直さない。通知前なら、試行前の付け外しは試行の照合に入らないので通常どおり進める
if want 27; then
fail_first edit-add 730; heal; flip_human 730; run_cycles 1
check "27: resumed escalation, needs-human removed after labeled_at -> not added again" eq "$(calls)" ""
once 734 flip_human 734
check "27: fresh escalation, needs-human removed after labeled_at -> escalated as usual" eq "$(calls)" "$(escalation_calls 734)"
fi
# 28. 記録は同じ labeled_at のものだけを使う。外し損ねの記録の後に外して付け直されたら、新しい labeled_at の格上げを初めからする
if want 28; then
fail_first edit-remove 732 "$T40"; heal
gh_edit 'i = st[args[0]]; i["labels"] = ["needs-decision"]
i["events"][-1] += [ev("unlabeled", "needs-human", args[1]), ev("unlabeled", "needs-decision", args[1]), ev("labeled", "needs-decision", args[2])]' 732 "$(ago 30)" "$T20"; run_cycles 1
check "28: a record for an older labeled_at is not reused" eq "$(calls)" "$(escalation_calls 732)"
fi

# --- 試行の記録と GitHub のラベル履歴での照合（記録は段ごとにその場で保存する） ---

# 29. 試行の前に needs-human が付け外しされていても、一時的に付けられなかったら次の周に付け直す
if want 29; then
reset; issue_with 735 "$T20" needs-decision; touch "$MOCK/fail-edit-add"; flip_human 735
run_cycles 1; retry
check "29: removed before the attempt, transient failure -> added again, then needs-decision removed" eq "$(calls)" "$(label_calls 735)"
fi
# 30. 付いたのに応答が失われたら、記録の試行の後の自分の名義の labeled で付けたと確かめ、needs-decision だけを外す。
# その間に人間が needs-human を外していても付け直さない。付けた直後に保存の前で止まっても同じ
if want 30; then
fail_first edit-add-lost 736
check "30: add applied but the response lost -> step=label (exact), the attempt is recorded before the call" eq "$(last_line)|$(record_of 736)" "DECISION escalate-failed #736 step=label|$T20 adding"
retry
check "30: lost response -> next cycle removes needs-decision only, escalated (exact), labels end as needs-human" eq "$(calls)|$(last_line)|$(labels_of 736)" "issue edit 736 --remove-label needs-decision|$(escalated 736)|needs-human"
fail_first edit-add-lost 737; heal
gh_edit 'i = st[args[0]]; i["labels"].remove("needs-human"); i["events"][-1].append(ev("unlabeled", "needs-human", args[1]))' 737 "$T5"; run_cycles 1
check "30: removed by a human after the attempt -> not added again, needs-decision removed" eq "$(calls)" "issue edit 737 --remove-label needs-decision"
fail_first edit-add-crash 738; retry
check "30: stopped before saving -> next cycle removes needs-decision only, labels end as needs-human" eq "$(calls)|$(labels_of 738)" "issue edit 738 --remove-label needs-decision|needs-human"
fi
# 31. 試行の後にほかの誰かが needs-human を付けたら、held を残して DECISION held を出し、needs-decision は外さない。以後は読まない
if want 31; then
fail_first edit-add 739; heal; other_adds_human 739; run_cycles 1
check "31: someone else's needs-human after the attempt -> no writes, held (exact)" eq "$(calls)|$(last_line)" "|DECISION held #739 labeled_at=$T20"
: > "$MOCK/reads"; run_cycles 1
check "31: held -> the issue is not read again" eq "$(reads_of 739)" ""
fi
# 32. 書く直前の events の取得の間に needs-decision が外されたら書かない。返った events に unlabeled があれば、その後の
# Issue の読み取りで付き直っていても（新しい付与）書かず、unlabeled がまだ無ければ events の後に読む Issue で分かる
if want 32; then
reset; issue_with 740 "$T20" needs-decision
on_read events 740 "i['labels'].remove('needs-decision'); i['events'][-1].append(ev('unlabeled', 'needs-decision', '$T5'))" 1
on_read issue 740 "i['labels'] += [] if 'needs-decision' in i['labels'] else ['needs-decision']"; run_cycles 1
check "32: removed during the events fetch (unlabeled returned), back on the issue read -> no writes" eq "$(calls)" ""
once 741 on_read events 741 "i['labels'].remove('needs-decision')" 1
check "32: removed during the events fetch (not yet in events) -> no writes" eq "$(calls)" ""
fi
# 33. 付いたのに応答が失われ、その labeled が events にまだ出ていないうちは、held にせず次の周に回す
if want 33; then
fail_first edit-add-lost 742; heal; gh_edit 'json.dump(st[args[0]]["events"][-1].pop(), open(args[1], "w"))' 742 "$MOCK/lagged"; run_cycles 1
check "33: added but not yet in events -> no writes, the attempt kept" eq "$(calls)|$(record_of 742)" "|$T20 adding"
gh_edit 'st[args[0]]["events"][-1].append(json.load(open(args[1])))' 742 "$MOCK/lagged"; run_cycles 1
check "33: once in events -> needs-decision removed" eq "$(calls)" "issue edit 742 --remove-label needs-decision"
fi
# 34. 付与が Issue の読み取りと最後の events の取得の間に起きたら、通知の前なら何も書かない。通知の後なら通知だけ書いて held になる
if want 34; then
once 743 on_read events 743 "i['labels'].append('needs-human'); i['events'][-1].append(ev('labeled', 'needs-human', '$T5'))" 2
check "34: someone else's needs-human before the notice -> no writes, no record" eq "$(calls)|$(record_of 743)" "|"
# 定型コメントだけで見張りの記録が無い（ほかの担当が書いた）ときは、別の担当の needs-human で held にせず、外れたら格上げを続ける
reset; issue_with 744 "$T20" needs-decision; add_comment 744 'pelway-dev[bot]' "$(body_for "$T20")"
on_read issue 744 "i['labels'].append('needs-human')"; run_cycles 1
check "34: template comment without a record, another's needs-human -> no writes, no held" eq "$(calls)|$(record_of 744)" "|"
set_labels 744 needs-decision; again
check "34: after the other's needs-human is removed -> the notice is not reposted, escalated" eq "$(calls)|$(last_line)" "$(label_calls 744)|$(escalated 744)"
# 通知の後、Issue の読み取りと最後の events の取得の間に付与が出るとき（ラベル一覧はまだ古い）は、通知だけ書いて held になる
once 758 on_read issue 758 "i['events'][-1].append(ev('labeled', 'needs-human', '$T5'))" 1
check "34: needs-human after the notice, before the add step -> notice once, no label edit, held (exact), needs-decision kept" \
  eq "$(calls)|$(last_line)|$(labels_of 758)" "issue comment 758 --body-file -|DECISION held #758 labeled_at=$T20|needs-decision"
fi
# 35. needs-human の付与が変更なしで成功しても（直前にほかの担当が付けた）added にせず、試行と付与イベントの照合で held にする
if want 35; then
once 746 on_read edit 746 "i['labels'].append('needs-human'); i['events'][-1].append(ev('labeled', 'needs-human', '$T5'))"
check "35: no-op add success -> needs-decision is not removed, both labels stay" eq "$(calls)|$(labels_of 746)" "$(lines "issue comment 746 --body-file -" "issue edit 746 --add-label needs-human")|needs-decision needs-human"
check "35: no-op add success -> held, not added, held line (exact)" eq "$(record_of 746)|$(last_line)" "$T20 held|DECISION held #746 labeled_at=$T20"
fi
# 36. needs-decision を外す応答が失われても、外れていれば次の周で完了を報告してから記録を整理する
if want 36; then
fail_first edit-remove-lost 747
check "36: removal response lost -> step=label, needs-decision is actually gone, record kept as added" eq "$(last_line)|$(labels_of 747)|$(record_of 747)" "DECISION escalate-failed #747 step=label|needs-human|$T20 added"
again
check "36: removal confirmed -> escalated reported (exact), record cleaned, no writes" eq "$(last_line)|$(record_of 747)|$(calls)" "$(escalated 747)||"
# 外した応答が失われたあと次の周までに新しい needs-decision が付いたら、前の付与の完了を 1 回報告してから新しい付与を別の期限で進める
fail_first edit-remove-lost 759 "$T40"; heal
relabel 759 "$T5"; set_labels 759 needs-decision needs-human; run_cycles 1
check "36: relabeled after a lost removal -> old escalation completed once, no writes for the new labeling, old record replaced" \
  eq "$(decision_lines | grep -c "escalated #759 labeled_at=$T40")|$(calls)|$(record_of 759)" "1||"
fail_first edit-remove-lost 763 "$T40"; heal; relabel 763 "$T5"; set_labels 763 needs-decision needs-human; reg "$CONDUCTOR"; run_cycles 1
check "36: relabeled after a lost removal, conductor running -> old escalation completed once, no writes" eq "$(decision_lines | grep -c "escalated #763 labeled_at=$T40")|$(calls)" "1|"
fi
# 37. 途中で Issue が閉じたら記録を残し、再オープン後に解除の再試行を再開する
if want 37; then
fail_first edit-remove 748
check "37: mid-flight record is added" eq "$(record_of 748)" "$T20 added"
gh_edit 'i = st[args[0]]; i["state"] = "closed"' 748; again
check "37: closed mid-flight -> no writes, record kept" eq "$(calls)|$(record_of 748)" "|$T20 added"
gh_edit 'i = st[args[0]]; i["state"] = "open"' 748; retry
check "37: reopened -> removal retried, escalated, labels end as needs-human" eq "$(calls)|$(last_line)|$(labels_of 748)" "issue edit 748 --remove-label needs-decision|$(escalated 748)|needs-human"
fi
# 38. 同じ付与が続いている間は通知済みの記録を残す: needs-decision の付いたまま閉じて再オープンしても通知は繰り返さない
if want 38; then
reset; issue_with 760 "$T40" needs-decision; reg "$CONDUCTOR"; run_cycles 1  # 指揮役がいるので知らせるだけ（overdue 1 回）
gh_edit 'st[args[0]]["state"] = "closed"' 760; run_cycles 1
gh_edit 'st[args[0]]["state"] = "open"' 760; run_cycles 1
check "38: same labeling close+reopen -> overdue is not repeated, no writes" eq "$(decision_lines | grep -c 'overdue #760')|$(calls)" "1|"
# `held` の記録も同じ付与が続く間は残す: 再オープン後に格上げを再開しない
fail_first edit-add 761; heal; other_adds_human 761; run_cycles 1  # 試行の後のほかの誰かの付与 -> held
gh_edit 'i = st[args[0]]; i["state"] = "closed"; i["labels"].remove("needs-human")' 761; run_cycles 1
gh_edit 'st[args[0]]["state"] = "open"' 761; run_cycles 1
check "38: held record survives close+reopen -> still held, no re-escalation, overdue once (exact)" eq "$(record_of 761)|$(calls)|$(decision_lines | grep -c 'overdue #761')" "$T20 held||1"
fi
# 39. 一覧の後に needs-human が付き、最初の Issue ごとの events に出たら DECISION overdue を出さず、通知済みにもしない（外れたら知らせる）
if want 39; then
once 764 on_read events 764 "i['labels'].append('needs-human'); i['events'][-1].append(ev('labeled', 'needs-human', '$T5'))"
check "39: needs-human in the first events fetch -> no overdue, no writes, waiting_on human, next_owner hatai" eq "$(decision_lines)|$(calls)|$(sget 764 waiting_on) $(sget 764 next_owner)" "||human hatai"
check "39: needs-human in the first events fetch -> next_check_at is a readable datetime, not labeled_at + 15 min" eval 'v="$(nca_after 764 "$T20")" && [[ "$v" =~ ^-?[0-9]+$ && "$v" != 900 ]]'
gh_edit 'i = st[args[0]]; i["labels"].remove("needs-human"); i["events"][-1].append(ev("unlabeled", "needs-human", args[1]))' 764 "$T5"; run_cycles 1
check "39: after needs-human is removed -> overdue, then escalated" eq "$(decision_lines)" "$(lines "$(overdue 764)" "$(escalated 764)")"
fi

if ((have_only)); then
  missed=""
  for n in ${ONLY//,/ }; do
    [[ ",$matched," == *",$n,"* ]] || missed="${missed:+$missed,}$n"
  done
  if [[ -n "$missed" ]]; then
    printf 'no scenario matched --only %s\n' "$missed" >&2
    exit 2
  fi
fi
if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}"
  exit 1
fi
printf '\nall passed\n'
