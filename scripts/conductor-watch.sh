#!/usr/bin/env bash
# 指揮役の見張り: T3 スレッドの状態の変化と、agent/ ブランチの PR の変化を 1 行ずつ標準出力に出す。
# 巡回のたびに、巡回状態の補助記録（対象ごとの誰待ちか・待ち始めた時刻・次の担当・最終更新時刻）を
# $XDG_STATE_HOME/pelway/patrol-state.json に更新する。正本は GitHub Issues／Projects で、
# 記録と GitHub の実状態が食い違うときは GitHub を正として記録を直す（runbook の「巡回状態の補助記録」）。
# 状態は $XDG_STATE_HOME/pelway/conductor-watch/ に置く（セッションの一時フォルダに置かない）。
# 使い方: scripts/conductor-watch-ensure.sh で起動する（冪等の起動入口。止めるまで 30 秒ごとに見る）
D="${XDG_STATE_HOME:-$HOME/.local/state}/pelway/conductor-watch"; mkdir -p "$D"
P="${XDG_STATE_HOME:-$HOME/.local/state}/pelway/patrol-state.json"
# 一覧の 1 回の取得上限。件数が上限に達した巡回は打ち切りの可能性があるため、
# python 側で巡回状態を更新しない（窓外の対象を消したり誤判定したりしない）。
IL=1000; PL=1000
cd "$(dirname "$0")/.." || exit 1
while true; do
  t3 thread list --json > "$D/threads.json" 2>/dev/null || true
  # GitHub からの取得に失敗したらファイルを消す（空の実状態と区別する。python 側で取得失敗と扱い、
  # 巡回状態の記録を書き換えさせない）。
  scripts/agent-gh dev issue list --state all --limit "$IL" --json number,state,labels > "$D/issues.json" 2>/dev/null || rm -f "$D/issues.json"
  scripts/agent-gh dev pr list --state all --limit "$PL" --json number,state,isDraft,headRefName,reviewDecision,statusCheckRollup > "$D/prs.json" 2>/dev/null || rm -f "$D/prs.json"
  python3 -I - "$D" "$P" "$IL" "$PL" <<'PY'
import json, sys, time, os, re
from datetime import datetime
d = sys.argv[1]; sp = os.path.join(d, "state.json")
# スレッド名の形式（skill command-team の「スレッド名」）。指揮役が作ったスレッドだけを見る。
TITLE = [re.compile(r"^\[実装\] #\d+ \S"), re.compile(r"^\[レビュー\] PR #\d+ \S"), re.compile(r"^\[(?!実装\]|レビュー\])[^\]\s]+\] \S")]
def title_ok(t): return any(r.search(t) for r in TITLE)
try: prev = json.load(open(sp))
except Exception: prev = None
cur = {"threads": {}, "prs": {}, "since": {} if prev is None else prev.get("since", {}),
       "badtitle": [] if prev is None else prev.get("badtitle", [])}
try: threads = json.load(open(os.path.join(d, "threads.json")))
except Exception: threads = []
try: issues = json.load(open(os.path.join(d, "issues.json")))
except Exception: issues = None
try: prs = json.load(open(os.path.join(d, "prs.json"))); prs_ok = True
except Exception: prs = []; prs_ok = False
now = time.time()
for t in threads:
    tid, s = t["threadId"], t.get("status")
    cur["threads"][tid] = s
    if prev is not None and prev["threads"].get(tid) != s:
        print(f"THREAD {s} {tid} prev={prev['threads'].get(tid)} {t.get('title','')[:40]}", flush=True)
    title = t.get("title") or ""
    # 古い終わったスレッドは数えない（次に操作するときに直す）。動いているものと新しく現れたものだけを見る。
    active = s in ("running", "starting", "preparing") or (prev is not None and tid not in prev["threads"])
    if t.get("createdBy") == "agent" and not t.get("archived") and active and not title_ok(title):
        if tid not in cur["badtitle"]:
            cur["badtitle"].append(tid)
            print(f"THREAD bad-title {tid} {title[:40]}", flush=True)
    elif tid in cur["badtitle"]:
        cur["badtitle"].remove(tid)
    # 停滞: 実行中のまま、スレッドの手順の数（itemCount）が 45 分変わらない。
    ic = t.get("itemCount")
    last = cur["since"].get(tid)
    if not isinstance(last, dict): last = None
    if s == "running":
        if not last or last.get("items") != ic:
            cur["since"][tid] = {"items": ic, "at": now, "warned": False}
        elif now - last["at"] > 2700 and not last.get("warned"):
            last["warned"] = True
            print(f"THREAD stalled-45m {tid} {t.get('title','')[:40]}", flush=True)
    else:
        cur["since"].pop(tid, None)
for pr in prs:
    if not pr["headRefName"].startswith("agent/"): continue
    ch = {(c.get("name") or c.get("context")): (c.get("conclusion") or c.get("state") or c.get("status") or "") for c in (pr.get("statusCheckRollup") or [])}
    v = ",".join(f"{k}={ch[k]}" for k in sorted(ch) if k and ("verify" in k or "pr-size" in k))
    key = f"{pr['state']}|draft={pr['isDraft']}|review={pr.get('reviewDecision')}|{v}"
    n = str(pr["number"]); cur["prs"][n] = key
    if prev is not None and prev["prs"].get(n) != key and (n in prev["prs"] or pr["state"] == "OPEN"):
        print(f"PR #{n} {pr['headRefName']} {key}", flush=True)
# 読み取りに失敗して空になったときは、前回の状態を残す（全スレッドを「新規」と誤報しない）。
if prev is not None and not threads: cur["threads"] = prev["threads"]
if prev is not None and not prs: cur["prs"] = prev["prs"]
json.dump(cur, open(sp, "w"))
# 巡回状態の補助記録（patrol-state.json）。対象は Issue 番号ごと。正本は GitHub Issues と
# Projects で、ここには GitHub に載らない情報（誰待ちか・待ち始めた時刻・次の担当）だけを置く。
# 巡回のたびに必須項目（waiting_on・waiting_since・next_owner・updated_at）を GitHub の実状態
# （Issue のラベルと状態、PR の Draft・レビュー・CI の状態）で上書きし、食い違いは GitHub を
# 正として直す。waiting_on が同じ対象だけ待ち始めた時刻を引き継ぐ。
ps_path = sys.argv[2]
issue_limit = int(sys.argv[3]); pr_limit = int(sys.argv[4])
try: ps = json.load(open(ps_path))
except Exception: ps = {}
if not isinstance(ps.get("subjects"), dict): ps["subjects"] = {}
# 取得に成功し、かつどちらの一覧も打ち切られていない（件数が上限に達していない）ときだけ更新する。
if issues is not None and prs_ok and len(issues) < issue_limit and len(prs) < pr_limit:
    pr_of = {}
    for p in prs:
        m = re.match(r"agent/(\d+)", p.get("headRefName") or "")
        if not m: continue
        k, c = m.group(1), pr_of.get(m.group(1))
        # 同じ Issue の PR が複数あるときは open を優先し、同じ状態なら番号が大きい方を残す
        if c is None or ((p.get("state") == "OPEN") != (c.get("state") == "OPEN")
                         and p.get("state") == "OPEN") \
                      or (p.get("state") == c.get("state") and p["number"] > c["number"]):
            pr_of[k] = p
    now_iso = datetime.fromtimestamp(now).isoformat(timespec="seconds")
    new_subj = {}
    for iss in issues:
        n = str(iss.get("number"))
        # 閉じた Issue と待ち状態でない Issue は GitHub が正: 記録に残さない（残っていれば消える）
        if iss.get("state") != "OPEN": continue
        labels = {l.get("name") for l in (iss.get("labels") or []) if isinstance(l, dict)}
        pr = pr_of.get(n)
        if "needs-human" in labels:
            won, owner = "human", "hatai"
        elif "agent:working" in labels:
            if pr is None:
                won, owner = "implement", f"implement thread (#{n})"
            elif pr.get("state") != "OPEN":
                won, owner = "conductor", "conductor"  # PR が閉じても Issue が開いている
            else:
                ch = {(c.get("name") or c.get("context")): (c.get("conclusion") or c.get("state") or c.get("status") or "") for c in (pr.get("statusCheckRollup") or [])}
                if pr.get("isDraft") or any("verify" in k and v == "FAILURE" for k, v in ch.items()):
                    won, owner = "implement", f"implement thread (#{n})"
                elif pr.get("reviewDecision") == "APPROVED":
                    won, owner = "merge", f"conductor (PR #{pr['number']})"
                elif pr.get("reviewDecision") == "CHANGES_REQUESTED":
                    won, owner = "implement", f"implement thread (#{n})"
                else:
                    won, owner = "review", f"reviewer (PR #{pr['number']})"
        elif "ready-for-agent" in labels:
            won, owner = "conductor", "conductor"
        else:
            continue
        old = ps["subjects"].get(n) if isinstance(ps["subjects"].get(n), dict) else {}
        since = old.get("waiting_since") if old.get("waiting_on") == won and isinstance(old.get("waiting_since"), str) else ""
        if not since: since = now_iso
        new_subj[n] = {"waiting_on": won, "waiting_since": since, "next_owner": owner, "updated_at": now_iso}
        if old.get("waiting_on") != won:
            print(f"PATROL #{n} waiting_on={won} since={since} next={owner}", flush=True)
    ps["subjects"] = new_subj
    ps["updated_at"] = now_iso
    os.makedirs(os.path.dirname(ps_path) or ".", exist_ok=True)
    json.dump(ps, open(ps_path, "w"), ensure_ascii=False, indent=2, sort_keys=True)
PY
  sleep 30
done
