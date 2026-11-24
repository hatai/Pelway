#!/usr/bin/env bash
# 指揮役の見張り: T3 スレッドの状態の変化と、agent/ ブランチの PR の変化を 1 行ずつ標準出力に出す。
# 巡回のたびに、巡回状態の補助記録（対象ごとの誰待ちか・待ち始めた時刻・次の担当・次回確認時刻・最終更新時刻）を
# 共有 git dir（--git-common-dir）内の pelway/patrol-state.json に更新する。正本は GitHub Issues／Projects で、
# 記録と GitHub の実状態が食い違うときは GitHub を正として記録を直す（runbook の「巡回状態の補助記録」）。
# 見張り自身の状態は $XDG_STATE_HOME/pelway/conductor-watch/ に置く（セッションの一時フォルダに置かない）。
# 巡回のたびに、ボード Status 更新の書き込みキュー（$PELWAY_BOARD_QUEUE_DIR、既定
# /tmp/pelway-board-queue）の *.json を処理する。キューの形式と結果の見方は
# runbook の「ボード Status 更新キュー」。実行するのは gh project の読み取り
# （view・field-list・item-list）と item-edit だけで、人間名義の gh が使われる
# （エージェント環境から人間名義 gh は使えないため、ホスト側のこの常駐で処理する）。
# 使い方: scripts/conductor-watch-ensure.sh で起動する（冪等の起動入口。止めるまで 30 秒ごとに見る）
D="${XDG_STATE_HOME:-$HOME/.local/state}/pelway/conductor-watch"; mkdir -p "$D"
Q="${PELWAY_BOARD_QUEUE_DIR:-/tmp/pelway-board-queue}"; mkdir -p "$Q"
# 一覧の 1 回の取得上限。件数が上限に達した巡回は打ち切りの可能性があるため、
# python 側で巡回状態を更新しない（窓外の対象を消したり誤判定したりしない）。
IL=1000; PL=1000
cd "$(dirname "$0")/.." || exit 1
# 巡回状態の補助記録（patrol-state.json）の置き場は、リポジトリの共有 git dir
# （git rev-parse --git-common-dir）内の pelway/。worktree の sandbox 内の指揮役は
# 共有 git dir には書き込めるが ~/.local/state には書けないため（Issue #477）、
# git の管理対象外のこの場所を主 checkout・worktree・指揮役で共有する。
# PELWAY_PATROL_STATE で差し替えられる（テスト用）。
P="${PELWAY_PATROL_STATE:-}"
if [[ -z "$P" ]]; then
  common="$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null)" \
    || { echo "conductor-watch.sh: git common dir を解決できません" >&2; exit 1; }
  P="$common/pelway/patrol-state.json"
fi
mkdir -p "$(dirname "$P")"
# 旧置き場（$XDG_STATE_HOME/pelway/patrol-state.json）が残っていて新置き場が無いときは、
# 中身をそのまま引き継ぐ（1 回きりの移行。sandbox 内の指揮役は旧置き場に書けない）。
P_OLD="${XDG_STATE_HOME:-$HOME/.local/state}/pelway/patrol-state.json"
if [[ "$P" != "$P_OLD" && ! -f "$P" && -f "$P_OLD" ]]; then
  cp "$P_OLD" "$P" || { echo "conductor-watch.sh: patrol-state の移行に失敗しました" >&2; exit 1; }
  echo "PATROL-STATE migrated $P_OLD -> $P" >&2
fi
# t3 CLI の解決: 実行中の T3 サーバー（~/.t3/wsl-runtime 配下のバイナリ）と同じ実行ファイルを
# 優先する。PATH の t3（~/.local/bin/t3 など）は版番号が同じでも、サーバーが所有する
# ~/.t3/userdata へのアクセスが ServerOwnsDatabaseError で拒否されることがある（ビルド差）。
# T3_BIN 環境変数で明示指定したときは解決しない（テストのモック差し替えに使う）。
# サーバーが見つからないときは PATH の t3 を使う。
T3_BIN="${T3_BIN:-}"
if [[ -z "$T3_BIN" ]]; then
  T3_BIN="$(command -v t3 || true)"
  for exe in /proc/[0-9]*/exe; do
    link="$(readlink "$exe" 2>/dev/null || true)"
    case "$link" in
      */.t3/wsl-runtime/*/t3) T3_BIN="$link"; break ;;
    esac
  done
fi
while true; do
  # threads.json は python 側で取得の成功を確かめてから差し替える（失敗で前回の一覧を消さない）
  "$T3_BIN" thread list --json > "$D/threads.json.new" 2> "$D/threads.err"; trc=$?
  # GitHub からの取得に失敗したらファイルを消す（空の実状態と区別する。python 側で取得失敗と扱い、
  # 巡回状態の記録を書き換えさせない）。
  scripts/agent-gh dev issue list --state all --limit "$IL" --json number,state,labels > "$D/issues.json" 2>/dev/null || rm -f "$D/issues.json"
  scripts/agent-gh dev pr list --state all --limit "$PL" --json number,state,isDraft,headRefName,reviewDecision,statusCheckRollup > "$D/prs.json" 2>/dev/null || rm -f "$D/prs.json"
  # ラベルイベント（issues/events は PR のラベルイベントも含む）。付け外しの裏付け検査
  # （runbook の「1. 役割と触ってよい範囲」）に使う。取得に失敗したらファイルを消す
  # （python 側で取得失敗と扱い、前回の処理済み ID を保つ）。
  scripts/agent-gh dev api "/repos/hatai/Pelway/issues/events?per_page=100" > "$D/label-events.json" 2>/dev/null || rm -f "$D/label-events.json"
  python3 -I - "$D" "$P" "$IL" "$PL" "$trc" "$T3_BIN" <<'PY'
import json, sys, time, os, re, subprocess
from datetime import datetime, timedelta
d = sys.argv[1]; sp = os.path.join(d, "state.json")
# スレッド名の形式（skill command-team の「スレッド名」）。指揮役が作ったスレッドだけを見る。
TITLE = [re.compile(r"^\[実装\] #\d+ \S"), re.compile(r"^\[レビュー\] PR #\d+ \S"), re.compile(r"^\[(?!実装\]|レビュー\])[^\]\s]+\] \S")]
def title_ok(t): return any(r.search(t) for r in TITLE)
try: prev = json.load(open(sp))
except Exception: prev = None
cur = {"threads": {}, "prs": {}, "since": {} if prev is None else prev.get("since", {}),
       "badtitle": [] if prev is None else prev.get("badtitle", []),
       "label_seen": [] if prev is None else prev.get("label_seen", []),
       "due_warned": {} if prev is None else prev.get("due_warned", {}),
       "stall_notified": {} if prev is None else prev.get("stall_notified", {}),
       "shared_notified": {} if prev is None else prev.get("shared_notified", {}),
       "heartbeat_at": 0 if prev is None else prev.get("heartbeat_at", 0),
       "thread_list": None if prev is None else prev.get("thread_list"),
       "pending_sends": None if prev is None else prev.get("pending_sends")}
# ラベル操作の裏付け検査（Issue #405、2026-11-17 の要件変更）。actor だけでは判定しない
# （エージェントが hatai のトークンで gh を動かすと、タイムラインの actor が hatai に
# なることがあるため）。needs-human の解除と automerge の付け外しを、イベント時刻の前
# LABEL_BACKUP_HOURS 時間以内の同じ Issue・PR への hatai 本人のコメント（指示の記録）と
# 突き合わせ、裏付けが無ければ「出どころ不明」として LABEL unbacked 行を出す。
# 指揮役と T3 スレッドの操作記録との突き合わせは巡回が行う
# （runbook の「指揮役の定期巡回（スケジュール起床）」）。
LABEL_BACKUP_HOURS = 24
LABEL_BACKING_AUTHOR = "hatai"
# t3 thread list の取得の判定（Issue #651）。終了コードが 0 で、標準出力が JSON の配列で、
# 各要素がスレッドの形（後の処理が読むフィールドの型を持つ dict）のときだけ成功とし、
# threads.json を差し替える。失敗した巡回は空の一覧として扱い、スレッドに基づく判定
# （THREAD 行・停滞・止まり）をしない。失敗の理由は stderr の空白だけの行を除いた最初の行、無ければ失敗の種類。
def fetch_threads(rc):
    try: out = open(os.path.join(d, "threads.json.new"), "rb").read()
    except Exception: out = b""
    try: err = next((l.strip() for l in open(os.path.join(d, "threads.err"), errors="replace") if l.strip()), "")
    except Exception: err = ""
    if rc != 0: kind = f"exit={rc}"
    elif not out.strip(): kind = "empty"
    else:
        # Python の json は JSON 外のリテラル NaN・Infinity・-Infinity も読む。保存する
        # 一覧の形式としては受け付けないため、読み取りで例外にして失敗側に回す。
        def reject_const(s): raise ValueError(s)
        try: v = json.loads(out, parse_constant=reject_const)
        except Exception: v = None
        # 一覧でも要素がスレッドの形でないものは失敗と同じ扱い。後の処理は "threadId" への
        # 直接アクセス・"title" のスライスと正規表現・"updatedAt" の比較をするため、型が
        # 壊れた一覧を成功として保存すると、スナップショットを差し替えたあと python が
        # 落ちて WATCHER 行も出ずに巡回が止まる。欠落や null のフィールドは後の処理が
        # 空文字・None として扱うので許容する（表示側は "title" を or "" で正規化する）。
        def ok_thread(t):
            return isinstance(t, dict) and isinstance(t.get("threadId"), str) \
                and all(t.get(k) is None or isinstance(t.get(k), str) for k in ("title", "updatedAt", "status"))
        if isinstance(v, list) and all(ok_thread(t) for t in v): return v, None
        kind = "invalid-json"
    return [], err[:120] or kind
threads, thread_err = fetch_threads(int(sys.argv[5]))
if thread_err is None: os.replace(os.path.join(d, "threads.json.new"), os.path.join(d, "threads.json"))
try: issues = json.load(open(os.path.join(d, "issues.json")))
except Exception: issues = None
try: prs = json.load(open(os.path.join(d, "prs.json"))); prs_ok = True
except Exception: prs = []; prs_ok = False
# 巡回の時刻。PELWAY_WATCH_NOW が指すファイルに epoch 秒が書かれていればそれを使う（テスト用）。
try: now = float(open(os.environ["PELWAY_WATCH_NOW"]).read().strip())
except Exception: now = time.time()
def stamp(ts): return datetime.fromtimestamp(ts).isoformat(timespec="seconds")
def stampable(ts):  # 地方時の年の範囲を外れる時刻は、読めない since と同じく数に入れないための確かめ
    try:
        stamp(ts); return True
    except Exception:
        return False
def deadline_ts(s):  # ISO 8601 を epoch に読む。読めない値（無い・壊れている）は None
    try:
        return datetime.fromisoformat(str(s).replace("Z", "+00:00")).timestamp()
    except Exception:
        return None
def env_secs(name, default):  # 間隔の環境変数。数値でなければ設定名を警告して既定値に倒す（見張りを止めない）
    v = os.environ.get(name)
    if not v:
        return float(default)
    try:
        return float(v)
    except ValueError:
        print(f"WATCHER bad-config {name}={v} default={int(default)}s", flush=True)
        return float(default)
def safe_to(v):  # watch.log の 1 行に収めるため、宛先はスレッド ID の形（英数字と :._- で 128 文字以内）に限る
    s = str(v or "")
    return s if re.fullmatch(r"[A-Za-z0-9:._-]{1,128}", s) else "invalid"
# 見張りが動いていることを示す行（Issue #504）。HEARTBEAT_SECS ごとに 1 行。
# PELWAY_WATCH_HEARTBEAT_SECS で差し替えられる（テスト用）。
HEARTBEAT_SECS = env_secs("PELWAY_WATCH_HEARTBEAT_SECS", 600)
if now - (cur["heartbeat_at"] or 0) >= HEARTBEAT_SECS:
    print(f"HEARTBEAT at={stamp(now)}", flush=True)
    cur["heartbeat_at"] = now
# 未配送のスレッド指示（patrol-state.json の pending_thread_sends）の滞留（Issue #669）。
# MCP の send 系障害などで配信されないままの指示を、最古の since からの経過が
# PENDING_SENDS_SECS（既定 30 分。PELWAY_WATCH_PENDING_SENDS_SECS で上書きできる）を
# 超えたら知らせる。配列が空・キーが無い・patrol-state.json が読めないときは滞留なしとして
# 扱う（since を読めない要素は数に入れず、誤検知で止まらない）。滞留の間は HEARTBEAT_SECS
# ごとに知らせ直し、解消（配列が空になるか最古の since が新しい値に変わる）で回復の行を
# 1 つ出す。巡回状態（GitHub の一覧）の成否にかかわらず毎巡回動かす。
PENDING_SENDS_SECS = env_secs("PELWAY_WATCH_PENDING_SENDS_SECS", 1800)
try:  # 検知の失敗で巡回の後段を止めない（誤検知で止まらない方針と同じ）
    oldest, pend_cnt, pend_to = None, 0, ""
    try:
        psw = json.load(open(sys.argv[2]))
        pend_arr = psw.get("pending_thread_sends")
    except Exception:
        pend_arr = None
    if isinstance(pend_arr, list):
        for e in pend_arr:
            if not isinstance(e, dict): continue
            ts = deadline_ts(e.get("since"))
            if ts is None or not stampable(ts): continue
            pend_cnt += 1
            if oldest is None or ts < oldest:
                oldest, pend_to = ts, safe_to(e.get("to"))
    pend = cur["pending_sends"] if isinstance(cur["pending_sends"], dict) else None
    if oldest is not None and now - oldest > PENDING_SENDS_SECS:
        if pend is None or pend.get("since") != oldest:
            if pend is not None:
                print(f"WATCHER pending-sends-recovered since={stamp(pend['since'])} after={int(now - pend['since'])}s", flush=True)
            pend = {"since": oldest, "notified_at": now}
            print(f"WATCHER pending-sends-stalled since={stamp(oldest)} for={int(now - oldest)}s count={pend_cnt} oldest_to={pend_to}", flush=True)
        elif now - pend["notified_at"] >= HEARTBEAT_SECS:
            pend["notified_at"] = now
            print(f"WATCHER pending-sends-still-stalled since={stamp(oldest)} for={int(now - oldest)}s count={pend_cnt} oldest_to={pend_to}", flush=True)
    elif pend is not None:
        print(f"WATCHER pending-sends-recovered since={stamp(pend['since'])} after={int(now - pend['since'])}s", flush=True)
        pend = None
    cur["pending_sends"] = pend
except Exception as ex:
    print(f"WATCHER pending-sends-error {type(ex).__name__}", flush=True)
# スレッド一覧の取得の状態は ok（thread_list が None）か failing の 2 つ。failing の間は
# 失敗の始まりから HEARTBEAT_SECS ごとに知らせ直す（HEARTBEAT の行とは別に数える）。
tl = cur["thread_list"]
if thread_err is None:
    if tl: print(f"WATCHER thread-list-recovered since={stamp(tl['fail_since'])} after={int(now - tl['fail_since'])}s", flush=True)
    tl = None
elif tl is None:
    tl = {"fail_since": now, "notified_at": now, "reason": thread_err}
    print(f"WATCHER thread-list-failed since={stamp(now)} reason={thread_err}", flush=True)
else:
    tl["reason"] = thread_err
    if now - tl["notified_at"] >= HEARTBEAT_SECS:
        tl["notified_at"] = now
        print(f"WATCHER thread-list-still-failing since={stamp(tl['fail_since'])} for={int(now - tl['fail_since'])}s reason={thread_err}", flush=True)
cur["thread_list"] = tl
for t in threads:
    tid, s = t["threadId"], t.get("status")
    cur["threads"][tid] = s
    if prev is not None and prev["threads"].get(tid) != s:
        print(f"THREAD {s} {tid} prev={prev['threads'].get(tid)} {(t.get('title') or '')[:40]}", flush=True)
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
            print(f"THREAD stalled-45m {tid} {(t.get('title') or '')[:40]}", flush=True)
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
# ラベルイベントの裏付け検査。初回観測と label_seen を持たない旧状態からの更新は
# 基準化だけ（過去のイベントを遡って報告しない）。events・comments の取得に失敗した
# イベントは処理済みにせず、次の巡回で再試行する（誤報しない）。
try: levs = json.load(open(os.path.join(d, "label-events.json")))
except Exception: levs = None
if not isinstance(levs, list): levs = None
if levs is not None:
    comments_cache = {}
    def iso(s):
        try: return datetime.fromisoformat(str(s).replace("Z", "+00:00"))
        except Exception: return None
    def human_backed(num, t):
        # num の Issue・PR のコメントを取得し、t の前 LABEL_BACKUP_HOURS 時間〜t の間の
        # hatai 本人のコメント（指示の記録）を探す。取得失敗は None（判定の延期）
        if num not in comments_cache:
            try:
                r = subprocess.run(["scripts/agent-gh", "dev", "api",
                    f"/repos/hatai/Pelway/issues/{num}/comments?per_page=100&sort=created&direction=desc"],
                    stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, timeout=60)
                cs = json.loads(r.stdout) if r.returncode == 0 else None
            except Exception:
                cs = None
            comments_cache[num] = cs if isinstance(cs, list) else None
        cs = comments_cache[num]
        if cs is None: return None
        for c in cs:
            if (c.get("user") or {}).get("login") != LABEL_BACKING_AUTHOR: continue
            ct = iso(c.get("created_at"))
            if ct is not None and t - timedelta(hours=LABEL_BACKUP_HOURS) <= ct <= t:
                return True
        return False
    done = set(cur["label_seen"])
    if prev is not None and "label_seen" in prev:
        for e in levs:
            if not isinstance(e, dict): continue
            evid = e.get("id")
            if evid is None or evid in done: continue
            kind, lab = e.get("event"), ((e.get("label") or {}).get("name") or "")
            # 検査対象: needs-human は解除のみ（付与はエスカレーションで日常的）、automerge は付け外し両方
            target = (kind == "unlabeled" and lab == "needs-human") or \
                     (kind in ("labeled", "unlabeled") and lab == "automerge")
            if not target:
                done.add(evid); continue
            iss = e.get("issue") if isinstance(e.get("issue"), dict) else {}
            num, t = iss.get("number"), iso(e.get("created_at"))
            if num is None or t is None:
                done.add(evid); continue
            backed = human_backed(num, t)
            if backed is None: continue  # comments 取得失敗: 次の巡回で再試行
            if not backed:
                actor = ((e.get("actor") or {}).get("login") or "")
                what = f"PR #{num}" if iss.get("pull_request") else f"issue #{num}"
                print(f"LABEL unbacked {kind} {lab} {what} at={e.get('created_at')} by={actor}", flush=True)
            done.add(evid)
    else:
        done.update(e.get("id") for e in levs if isinstance(e, dict))
    cur["label_seen"] = [e.get("id") for e in levs if isinstance(e, dict) and e.get("id") in done]
# 作業ツリーの共有（Issue #643）: settle されていないトップレベルのスレッドの worktreePath を
# t3 thread show で 1 本 1 回ずつ集め、2 本以上が同じパスなら THREAD shared-worktree の行を出す。
# 同じ組み合わせ（パスとスレッドの集合）は続く間 1 回だけ出す。通知済みの記録は stall_notified と
# 同じ JSON オブジェクトで、パス → 組み合わせ（昇順の threadId をカンマでつないだ文字列）ごとの記録。
# 失敗するスレッドが巡回ごとに入れ替わると同じパスに複数の組み合わせが並存しうるため、組み合わせごとに
# 記録する。show が失敗したスレッドはこの巡回で数えない。既出の組み合わせに失敗したスレッドが含まれ、
# 残りのスレッドが同じパスにいると確かめられたときだけ記録を保つ（失敗の巡回で再通知しない）。
# 残りのどれかが一覧から消えた・別のパスに移ったと確かめられたら、共有の解消として記録を消す。
# 一覧の取得に失敗した巡回は記録を保ち、正常に空の一覧は共有の解消として記録を消す。
if thread_err is None:
    by_path, failed = {}, set()
    for t in threads:
        if t.get("settled") or t.get("parentThreadId"): continue
        tid = t.get("threadId")
        try:
            r = subprocess.run([sys.argv[6], "thread", "show", tid, "--json"],
                               stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, timeout=60)
            wp = json.loads(r.stdout)["thread"].get("worktreePath") if r.returncode == 0 else None
            if r.returncode != 0: failed.add(tid)
        except Exception:
            failed.add(tid); wp = None
        if isinstance(wp, str) and wp:
            by_path.setdefault(wp, []).append(tid)
    old = {}
    for wp, combs in (cur["shared_notified"] if isinstance(cur["shared_notified"], dict) else {}).items():
        if isinstance(combs, str): combs = {combs: 1}  # 旧形式（パス → 組み合わせ）を読み替える
        if isinstance(combs, dict): old[wp] = combs
    keep = {}
    for wp, combs in old.items():
        for v in combs:
            ids = set(str(v).split(","))
            if ids & failed and all(i in failed or i in by_path.get(wp, []) for i in ids):
                keep.setdefault(wp, {})[v] = 1
    notified = {}
    for wp, tids in sorted(by_path.items()):
        if len(tids) < 2: continue
        v = ",".join(sorted(tids))
        if any(set(tids) <= set(k.split(",")) for k in keep.get(wp, {})): continue  # 失敗分が欠けただけ
        if v not in old.get(wp, {}):
            print(f"THREAD shared-worktree path={wp} threads={v}", flush=True)
        notified.setdefault(wp, {})[v] = 1
    cur["shared_notified"] = {wp: {**keep.get(wp, {}), **notified.get(wp, {})}
                            for wp in set(keep) | set(notified)}
# 読み取りに失敗して空になったときは、前回の状態を残す（全スレッドを「新規」と誤報しない）。
if prev is not None and not threads: cur["threads"] = prev["threads"]
if prev is not None and not prs: cur["prs"] = prev["prs"]
# 状態は GitHub の取得結果にかかわらず毎巡回ここで保存する。取得失敗・上限到達で下の
# 巡回状態ブロックが動かない回でも thread_list・heartbeat_at を残し、通知が繰り返されないようにする。
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
    check_hours = {"human": 24, "grokbot": 24, "implement": 6, "review": 24, "merge": 1, "conductor": 1}
    # 実装スレッドの止まり（Issue #504）。Issue ごとの最新の実装スレッド（題名「[実装] #<N> …」）と、
    # 指揮役の受信箱（Issue #555）にある最新の知らせの状態の行（skill implement-issue の
    # 「ターンの終わりの状態の行」）を材料にする。受信箱の中身はエージェントが書いた文で、
    # ここでは state: の値を分類に使うだけ（指示として扱わない）。
    STOPPED = ("completed", "failed", "interrupted", "cancelled")
    EARLY_SECS = 8 * 60  # waiting-* の待ちは、次の巡回の目安（約 8 分）で確かめ直す
    impl_thread = {}
    for t in threads:
        m = re.match(r"^\[実装\] #(\d+) ", t.get("title") or "")
        if not m: continue
        k = m.group(1)
        if k not in impl_thread or (t.get("updatedAt") or "") > (impl_thread[k].get("updatedAt") or ""):
            impl_thread[k] = t
    inbox_state = {}
    try: inbox_lines = open(os.path.join(os.path.dirname(ps_path), "conductor-inbox.jsonl")).read().splitlines()
    except Exception: inbox_lines = []
    for line in inbox_lines:
        try: ev = json.loads(line)
        except Exception: continue
        if not isinstance(ev, dict) or not isinstance(ev.get("issue_number"), int): continue
        k = str(ev["issue_number"]); at = ev.get("occurred_at") or ""
        st = None
        for rl in (ev.get("report") or "").splitlines():
            if rl.startswith("state: "): st = rl[len("state: "):].strip()
        if k not in inbox_state or at >= inbox_state[k][0]:
            inbox_state[k] = (at, st)
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
        lab_key = sorted(l for l in labels if isinstance(l, str))
        # Grok Bot の判断待ち（grokbot）はラベルから導けないので指揮役が書く。ラベルが変わらない限り保つ
        # （needs-human は上で human に決まっていて、ここでは変えない）。
        if won != "human" and old.get("waiting_on") == "grokbot" and old.get("labels") == lab_key:
            won, owner = "grokbot", "grokbot"
        # スレッド一覧が取れない間は止まりが続いているかを確かめられないため、conductor を
        # implement・review に戻さない（止まりで conductor にした対象を見失わない）。
        if cur["thread_list"] and won in ("implement", "review") and old.get("waiting_on") == "conductor":
            won, owner = "conductor", "conductor"
        # 実装スレッドが止まっていて、状態の行が working か無ければ、指揮役の判断待ちにする。
        # 書き換えてよいのは implement / review から conductor への向きだけ。
        early = False
        th = impl_thread.get(n)
        if won in ("implement", "review") and th and th.get("status") in STOPPED:
            st = inbox_state.get(n, ("", None))[1]
            if st in (None, "working"):
                if old.get("waiting_on") != "conductor":
                    print(f"PATROL #{n} waiting_on=conductor from={won} reason=stopped-thread state={st or 'none'} thread={th.get('threadId')} at={now_iso}", flush=True)
                key = th.get("updatedAt") or ""
                if cur["stall_notified"].get(n) != key:
                    print(f"STALL #{n} thread={th.get('threadId')} status={th.get('status')} state={st or 'none'} since={key}", flush=True)
                    cur["stall_notified"][n] = key
                won, owner = "conductor", "conductor"
            elif st == "waiting-conductor":
                # scripts/agent-gh を変える PR で、指揮役のレビュースレッドを待っている（Issue #622）。
                # 止まりではなく指揮役が動く合図なので、conductor にして STALL は出さない。
                won, owner = "conductor", "conductor"
            elif st in ("waiting-review", "waiting-checks"):
                early = True
        since = old.get("waiting_since") if old.get("waiting_on") == won and isinstance(old.get("waiting_since"), str) else ""
        if not since: since = now_iso
        # 次回確認時刻: waiting_on が同じ間は前回値（指揮役が延長・短縮した値を含む）を引き継ぐ。
        # 待ちが変わった、または値が無い・読めないときは、待ちの種類の目安で設定し直す
        # （自己修復として扱い、特別な報告はしない）。
        nca = old.get("next_check_at") if old.get("waiting_on") == won else None
        if deadline_ts(nca) is None:
            nca = datetime.fromtimestamp(now + check_hours[won] * 3600).isoformat(timespec="seconds")
        # waiting-* の待ちは、待っている先を次の巡回で確かめるため、次回確認時刻を早める向きにだけ変える
        if early and deadline_ts(nca) > now + EARLY_SECS:
            nca = datetime.fromtimestamp(now + EARLY_SECS).isoformat(timespec="seconds")
        new_subj[n] = {"waiting_on": won, "waiting_since": since, "next_owner": owner,
                       "next_check_at": nca, "updated_at": now_iso, "labels": lab_key}
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
    cur["stall_notified"] = {k: v for k, v in cur["stall_notified"].items() if k in new_subj}
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
