#!/usr/bin/env bash
# 指揮役の見張り: T3 スレッドの状態の変化と、agent/ ブランチの PR の変化を 1 行ずつ標準出力に出す。
# 巡回のたびに、巡回状態の補助記録（対象ごとの誰待ちか・待ち始めた時刻・次の担当・次回確認時刻・最終更新時刻）を
# $XDG_STATE_HOME/pelway/patrol-state.json に更新する。正本は GitHub Issues／Projects で、
# 記録と GitHub の実状態が食い違うときは GitHub を正として記録を直す（runbook の「巡回状態の補助記録」）。
# 状態は $XDG_STATE_HOME/pelway/conductor-watch/ に置く（セッションの一時フォルダに置かない）。
# 巡回のたびに、ボード Status 更新の書き込みキュー（$PELWAY_BOARD_QUEUE_DIR、既定
# /tmp/pelway-board-queue）の *.json を処理する。キューの形式と結果の見方は
# runbook の「ボード Status 更新キュー」。実行するのは gh project の読み取り
# （view・field-list・item-list）と item-edit だけで、人間名義の gh が使われる
# （エージェント環境から人間名義 gh は使えないため、ホスト側のこの常駐で処理する）。
# 使い方: scripts/conductor-watch-ensure.sh で起動する（冪等の起動入口。止めるまで 30 秒ごとに見る）
D="${XDG_STATE_HOME:-$HOME/.local/state}/pelway/conductor-watch"; mkdir -p "$D"
P="${XDG_STATE_HOME:-$HOME/.local/state}/pelway/patrol-state.json"
Q="${PELWAY_BOARD_QUEUE_DIR:-/tmp/pelway-board-queue}"; mkdir -p "$Q"
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
       "badtitle": [] if prev is None else prev.get("badtitle", []),
       "due_warned": {} if prev is None else prev.get("due_warned", {})}
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
# Projects で、ここには GitHub に載らない情報（誰待ちか・待ち始めた時刻・次の担当・次回確認時刻）だけを置く。
# 巡回のたびに必須項目（waiting_on・waiting_since・next_owner・next_check_at・updated_at）を GitHub の実状態
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
    # 次回確認時刻（フォローアップタイマー）の待ちの種類ごとの目安（時間）。runbook の
    # 「次回確認時刻（フォローアップタイマー）」の表と同じ値。変えるときは両方を直す。
    check_hours = {"human": 24, "implement": 6, "review": 24, "merge": 1, "conductor": 1}
    def deadline_ts(s):  # ISO 8601 を epoch に読む。読めない値（無い・壊れている）は None
        try:
            return datetime.fromisoformat(str(s).replace("Z", "+00:00")).timestamp()
        except Exception:
            return None
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
        # 次回確認時刻: waiting_on が同じ間は前回値（指揮役が延長・短縮した値を含む）を引き継ぐ。
        # 待ちが変わった、または値が無い・読めないときは、待ちの種類の目安で設定し直す
        # （自己修復として扱い、特別な報告はしない）。
        nca = old.get("next_check_at") if old.get("waiting_on") == won else None
        if deadline_ts(nca) is None:
            nca = datetime.fromtimestamp(now + check_hours[won] * 3600).isoformat(timespec="seconds")
        new_subj[n] = {"waiting_on": won, "waiting_since": since, "next_owner": owner,
                       "next_check_at": nca, "updated_at": now_iso}
        if old.get("waiting_on") != won:
            print(f"PATROL #{n} waiting_on={won} since={since} next={owner}", flush=True)
        # 期限到来: 次回確認時刻を過ぎた待ちを due 行で知らせる（同じ期限の値につき 1 回）。
        # 記録の値は更新されないまま残るため、巡回は記録側でも期限を過ぎた待ちを必ず確認対象に拾う。
        if deadline_ts(nca) <= now and cur["due_warned"].get(n) != nca:
            print(f"PATROL #{n} due next_check_at={nca} waiting_on={won} next={owner}", flush=True)
            cur["due_warned"][n] = nca
    ps["subjects"] = new_subj
    # 記録から消えた対象の期限通知の状態も消す
    cur["due_warned"] = {k: v for k, v in cur["due_warned"].items() if k in new_subj}
    ps["updated_at"] = now_iso
    os.makedirs(os.path.dirname(ps_path) or ".", exist_ok=True)
    json.dump(ps, open(ps_path, "w"), ensure_ascii=False, indent=2, sort_keys=True)
    json.dump(cur, open(sp, "w"))
PY
  # ボード Status 更新キュー: $Q の *.json を 1 件ずつ処理する。要求は
  # {"issue": <番号> | "item_id": "<id>", "status": "<Status 名>"} だけを持つ JSON で、
  # 値は gh の引数としてだけ渡す（シェルの文字列には組み込まない）。結果は同名の
  # .done / .error（結果 JSON）を残して要求ファイルを消す。失敗は再試行しない。
  # キューの利用者（エージェント）がディレクトリに書けるため、symlink 差し替え・
  # FIFO による巡回停止を防ぐ O_NOFOLLOW・dir_fd 指定の操作で処理する。
  python3 -I - "$Q" <<'PY' || true
import json, os, re, secrets, stat, subprocess, sys
from datetime import datetime
qdir = sys.argv[1]
# ボードの Status 選択肢（runbook「ボードと巡回」の先頭と同じ値。変えるときは両方を直す）。
STATUS_NAMES = {"Backlog", "Todo", "Working", "In review", "Changes requested",
                "Ready to merge", "Needs human", "Done"}
OWNER, NUM = "hatai", "7"
ITEM_RE = re.compile(r"[A-Za-z0-9_=-]{1,128}")
# 要求ファイルの大きさの上限（数 KB の JSON を想定。これを超えるものは要求とみなさない）
REQ_MAX = 16384

# キューディレクトリ自体を検証する。symlink に差し替えられた先のファイルを
# 読み・消してしまわないよう、O_NOFOLLOW で開いて所有者と種類を確かめ、
# 以降の操作は開いた dir_fd 経由で行う。不成立ならこの巡回では処理しない。
try:
    dfd = os.open(qdir, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    st = os.fstat(dfd)
    if not stat.S_ISDIR(st.st_mode) or st.st_uid != os.geteuid():
        raise OSError("queue dir not usable")
except OSError:
    print("QUEUE queue-dir invalid", flush=True)
    sys.exit(0)
reqs = sorted(n for n in os.listdir(dfd) if n.endswith(".json"))
if not reqs:
    sys.exit(0)
now_iso = datetime.now().isoformat(timespec="seconds")

def record(name, ok, **detail):
    out = name[:-len(".json")] + (".done" if ok else ".error")
    body = {"request": name, "at": now_iso, **detail}
    # 一時ファイルは予測不能な名前を O_EXCL|O_NOFOLLOW で作る
    # （<名前>.done.tmp への symlink で外のファイルを上書きさせない）
    tmp = f".q-{os.getpid()}-{secrets.token_hex(8)}.tmp"
    fd = os.open(tmp, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600, dir_fd=dfd)
    with os.fdopen(fd, "w") as f:
        json.dump(body, f, ensure_ascii=False)
    os.replace(tmp, out, src_dir_fd=dfd, dst_dir_fd=dfd)
    os.unlink(name, dir_fd=dfd)
    print(f"QUEUE {name} {'done' if ok else 'error: ' + str(detail.get('reason'))}", flush=True)

def fail(name, reason, **kw):
    record(name, False, reason=reason, **kw)

def read_request(name):
    fd = os.open(name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=dfd)
    with os.fdopen(fd, "rb") as f:
        st = os.fstat(f.fileno())
        if not stat.S_ISREG(st.st_mode) or st.st_size > REQ_MAX:
            raise ValueError("not a regular request file")
        return f.read()

# 第 1 段: ファイルとフィールドの検証（gh は呼ばない）
pending = []
for name in reqs:
    try:
        req = json.loads(read_request(name))
    except Exception:
        fail(name, "bad-json"); continue
    if not isinstance(req, dict) or not set(req) <= {"issue", "item_id", "status"}:
        fail(name, "bad-shape"); continue
    st = req.get("status")
    if not isinstance(st, str) or st not in STATUS_NAMES:
        fail(name, "bad-status"); continue
    iss, iid = req.get("issue"), req.get("item_id")
    iss_ok = isinstance(iss, int) and not isinstance(iss, bool) and iss > 0
    iid_ok = isinstance(iid, str) and ITEM_RE.fullmatch(iid) is not None
    if ("issue" in req) == ("item_id" in req) or ("issue" in req and not iss_ok) \
       or ("item_id" in req and not iid_ok):
        fail(name, "bad-target"); continue
    pending.append((name, req))
if not pending:
    sys.exit(0)

# 第 2 段: プロジェクトの実態（選択肢・カード）を読み、item-edit で適用する
def gh(*args):
    try:
        return subprocess.run(["gh", "project", *args], stdout=subprocess.PIPE,
                              stderr=subprocess.PIPE, text=True, timeout=60)
    except Exception:
        return None

def gh_json(*args):
    r = gh(*args)
    if r is None or r.returncode != 0:
        return None
    try:
        return json.loads(r.stdout)
    except Exception:
        return None

view = gh_json("view", NUM, "--owner", OWNER, "--format", "json")
fields = gh_json("field-list", NUM, "--owner", OWNER, "--format", "json")
sf = next((f for f in (fields or {}).get("fields", []) if f.get("name") == "Status"), None)
pid = (view or {}).get("id")
if not pid or not sf:
    for name, _ in pending:
        fail(name, "gh-read")
    sys.exit(0)
opts = {o.get("name"): o.get("id") for o in sf.get("options", [])}

item_of = {}
if any("issue" in r for _, r in pending):
    items = gh_json("item-list", NUM, "--owner", OWNER, "--format", "json", "--limit", "1000")
    if isinstance(items, dict):
        for it in items.get("items", []):
            c = it.get("content") or {}
            if c.get("type") == "Issue" and isinstance(c.get("number"), int):
                item_of[c["number"]] = it.get("id")
    else:
        item_of = None

for name, req in pending:
    st = req["status"]
    oid = opts.get(st)
    if not oid:
        fail(name, "no-option", status=st); continue
    if "issue" in req:
        if item_of is None:
            fail(name, "item-list"); continue
        iid = item_of.get(req["issue"])
        if not iid:
            fail(name, "item-not-found", issue=req["issue"]); continue
    else:
        iid = req["item_id"]
    r = gh("item-edit", "--project-id", pid, "--id", iid,
           "--field-id", sf["id"], "--single-select-option-id", oid)
    if r is not None and r.returncode == 0:
        record(name, True, status=st, item=iid)
    else:
        fail(name, "item-edit", rc=(r.returncode if r else -1))
PY
  sleep 30
done
