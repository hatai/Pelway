#!/usr/bin/env bash
# 指揮役の見張り: T3 スレッドの状態の変化と、agent/ ブランチの PR の変化を 1 行ずつ標準出力に出す。
# 状態は $XDG_STATE_HOME/pelway/conductor-watch/ に置く（セッションの一時フォルダに置かない）。
# 使い方: scripts/conductor-watch.sh（止めるまで 30 秒ごとに見る）
D="${XDG_STATE_HOME:-$HOME/.local/state}/pelway/conductor-watch"; mkdir -p "$D"
cd "$(dirname "$0")/.." || exit 1
while true; do
  t3 thread list --json > "$D/threads.json" 2>/dev/null || true
  gh pr list --state all --limit 30 --json number,state,isDraft,headRefName,reviewDecision,statusCheckRollup > "$D/prs.json" 2>/dev/null || true
  python3 -I - "$D" <<'PY'
import json, sys, time, os
d = sys.argv[1]; sp = os.path.join(d, "state.json")
try: prev = json.load(open(sp))
except Exception: prev = None
cur = {"threads": {}, "prs": {}, "since": {} if prev is None else prev.get("since", {})}
try: threads = json.load(open(os.path.join(d, "threads.json")))
except Exception: threads = []
try: prs = json.load(open(os.path.join(d, "prs.json")))
except Exception: prs = []
now = time.time()
for t in threads:
    if t.get("settled"): continue
    tid, s = t["threadId"], t.get("status")
    cur["threads"][tid] = s
    if prev is not None and prev["threads"].get(tid) != s:
        print(f"THREAD {s} {tid} prev={prev['threads'].get(tid)} {t.get('title','')[:40]}", flush=True)
    if s == "running":
        st = cur["since"].setdefault(tid, now)
        if now - st > 2700 and not cur["since"].get(tid + "!"):
            cur["since"][tid + "!"] = 1
            print(f"THREAD stalled-45m {tid} {t.get('title','')[:40]}", flush=True)
    else:
        cur["since"].pop(tid, None); cur["since"].pop(tid + "!", None)
for pr in prs:
    if not pr["headRefName"].startswith("agent/"): continue
    ch = {(c.get("name") or c.get("context")): (c.get("conclusion") or c.get("state") or c.get("status") or "") for c in (pr.get("statusCheckRollup") or [])}
    v = ",".join(f"{k}={ch[k]}" for k in sorted(ch) if k and ("verify" in k or "pr-size" in k))
    key = f"{pr['state']}|draft={pr['isDraft']}|review={pr.get('reviewDecision')}|{v}"
    n = str(pr["number"]); cur["prs"][n] = key
    if prev is not None and prev["prs"].get(n) != key and (n in prev["prs"] or pr["state"] == "OPEN"):
        print(f"PR #{n} {pr['headRefName']} {key}", flush=True)
json.dump(cur, open(sp, "w"))
PY
  sleep 30
done
