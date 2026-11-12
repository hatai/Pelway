#!/usr/bin/env bash
# 指揮役の見張り: T3 スレッドの状態の変化と、agent/ ブランチの PR の変化を 1 行ずつ標準出力に出す。
# 状態は $XDG_STATE_HOME/pelway/conductor-watch/ に置く（セッションの一時フォルダに置かない）。
# 使い方: scripts/conductor-watch.sh（止めるまで 30 秒ごとに見る）
D="${XDG_STATE_HOME:-$HOME/.local/state}/pelway/conductor-watch"; mkdir -p "$D"
cd "$(dirname "$0")/.." || exit 1
while true; do
  t3 thread list --json > "$D/threads.json" 2>/dev/null || true
  scripts/agent-gh dev pr list --state all --limit 30 --json number,state,isDraft,headRefName,reviewDecision,statusCheckRollup > "$D/prs.json" 2>/dev/null || true
  python3 -I - "$D" <<'PY'
import json, sys, time, os, re
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
try: prs = json.load(open(os.path.join(d, "prs.json")))
except Exception: prs = []
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
PY
  sleep 30
done
