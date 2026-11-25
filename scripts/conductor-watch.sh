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
# GitHub への書き込みはほかに 1 つだけ: needs-decision が 15 分決まらず指揮役が動いていない Issue に、scripts/agent-gh dev で
# コメントを書き、needs-decision を needs-human に替える（Issue #602。runbook の「指揮役の定期巡回（スケジュール起床）」）。
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
# 旧置き場（$XDG_STATE_HOME/pelway/patrol-state.json）が残っていて新置き場に何も無い（symlink・FIFO も無い）
# ときは、中身をそのまま引き継ぐ（1 回きりの移行。sandbox 内の指揮役は旧置き場に書けない）。新置き場は sandbox の
# 中から置き換えられるため、親ディレクトリ（既定は共有 git dir の pelway/）は巡回の open_ps_dir と同じく作って、
# 信頼できる基点（その 1 つ上。既定は git が解決した共有 git dir）から symlink をたどらずに開いて確かめ、
# 移行先はそのファイル記述子を基準に O_EXCL|O_NOFOLLOW・0600 で作る。旧置き場は種類を確かめ、通常ファイルだけを
# 巡回の read_ps と同じ検査（symlink をたどらず・FIFO で待たず・4 MiB 以下）で読む（ソケットなど開くと失敗する
# 種類で見張りを止めない）。親ディレクトリを確かめられないときは移行しない（巡回がその回ごとに確かめ直す。Issue #699）。
# 親ディレクトリは巡回の open_ps_dir が作るため、置き場が旧置き場と同じときはここでは何もしない。
P_OLD="${XDG_STATE_HOME:-$HOME/.local/state}/pelway/patrol-state.json"
if [[ "$P" != "$P_OLD" ]]; then
  python3 -I - "$P_OLD" "$P" <<'PY' || { echo "conductor-watch.sh: patrol-state の移行に失敗しました" >&2; exit 1; }
import errno, os, stat, sys
old, new = sys.argv[1], sys.argv[2]
PS_MAX = 4194304  # 巡回の read_ps と同じ読み取り上限（4 MiB）
def open_ps_dir(path):  # 巡回の open_ps_dir と同じ（変えるときは両方を直す）
    base, name = os.path.split(os.path.dirname(os.path.abspath(path)))
    os.makedirs(base, exist_ok=True)
    bfd = os.open(base, os.O_RDONLY | os.O_DIRECTORY)
    try:
        try: os.mkdir(name, dir_fd=bfd)
        except FileExistsError: pass
        dfd = os.open(name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=bfd)
    finally:
        os.close(bfd)
    st = os.fstat(dfd)
    if not stat.S_ISDIR(st.st_mode) or st.st_uid != os.geteuid():
        os.close(dfd); raise OSError("patrol-state dir not usable")
    return dfd
def skip(reason):
    print(f"PATROL-STATE not-migrated {old} reason={reason}", file=sys.stderr); sys.exit(0)
try: dfd = open_ps_dir(new)
except OSError:
    print(f"PATROL-STATE dir-invalid {os.path.dirname(new)}", file=sys.stderr); sys.exit(0)
name = os.path.basename(new)
try:
    os.lstat(name, dir_fd=dfd); sys.exit(0)  # 新置き場に何かある（symlink・FIFO を含む）: 引き継がない
except FileNotFoundError: pass
try: st = os.lstat(old)
except FileNotFoundError: sys.exit(0)
if stat.S_ISLNK(st.st_mode): skip("symlink")
if not stat.S_ISREG(st.st_mode): skip("not-regular")  # ソケット・FIFO・デバイスなど
try:
    fd = os.open(old, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
except OSError as e:
    if e.errno in (errno.ELOOP, errno.ENXIO): skip("not-regular")  # 確かめた後に差し替えられた
    if e.errno == errno.ENOENT: sys.exit(0)
    raise
with os.fdopen(fd, "rb") as f:
    if not stat.S_ISREG(os.fstat(f.fileno()).st_mode): skip("not-regular")
    data = f.read(PS_MAX + 1)
if len(data) > PS_MAX: skip("too-large")
try:
    out = os.open(name, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600, dir_fd=dfd)
except FileExistsError:
    sys.exit(0)  # 確かめた後に何かが置かれた: 引き継がない
with os.fdopen(out, "wb") as f: f.write(data)
print(f"PATROL-STATE migrated {old} -> {new}", file=sys.stderr)
PY
fi
# 指揮役スレッドの登録ファイル（pelway-agent-watch --register が書く。docs/dev-environment.md の「pitchfork について」）。
# needs-decision の格上げで指揮役が動いているかの判定に使う。PELWAY_CONDUCTOR_FILE で差し替えられる（テスト用）。
C="${PELWAY_CONDUCTOR_FILE:-$HOME/.config/pelway-agent-notify/conductor-thread-id}"
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
  # threads.json は python 側で取得の成功を確かめてから差し替える（失敗で前回の一覧を消さない）。
  # T3 の失敗はスレッドに基づく判定をせず（格上げは判定しない）、Issue・PR の失敗は patrol-state.json を書き換えない（Issue なら DECISION の検知もしない）
  # T3 一覧の取得開始時刻も記録する（SLOTS の記録の時刻は、使用数の根拠のうち最も古い
  # 取得開始時刻にする。一覧の取得が遅れた回に、古い停止の状態に新しい時刻が付かないようにする）。
  if [[ -n "${PELWAY_WATCH_NOW:-}" && -r "$PELWAY_WATCH_NOW" ]]; then cat "$PELWAY_WATCH_NOW"; else date +%s; fi > "$D/threads-at"
  "$T3_BIN" thread list --json > "$D/threads.json.new" 2> "$D/threads.err"; trc=$?
  # GitHub からの取得に失敗したらファイルを消す（空の実状態と区別する。python 側で取得失敗と扱い、
  # 巡回状態の記録を書き換えさせない）。
  # SLOTS の記録の時刻の一部に使う Issue 一覧の取得開始時刻（PR 一覧・ラベルイベントの取得が
  # 遅れても、古い使用数に新しい時刻が付かないようにする。テストは PELWAY_WATCH_NOW の値を使う）。
  if [[ -n "${PELWAY_WATCH_NOW:-}" && -r "$PELWAY_WATCH_NOW" ]]; then cat "$PELWAY_WATCH_NOW"; else date +%s; fi > "$D/issues-at"
  scripts/agent-gh dev issue list --state all --limit "$IL" --json number,state,labels > "$D/issues.json" 2>/dev/null || rm -f "$D/issues.json"
  scripts/agent-gh dev pr list --state all --limit "$PL" --json number,state,isDraft,headRefName,reviewDecision,statusCheckRollup > "$D/prs.json" 2>/dev/null || rm -f "$D/prs.json"
  # ラベルイベント（issues/events は PR のラベルイベントも含む）。付け外しの裏付け検査
  # （runbook の「1. 役割と触ってよい範囲」）に使う。取得に失敗したらファイルを消す
  # （python 側で取得失敗と扱い、前回の処理済み ID を保つ）。
  scripts/agent-gh dev api "/repos/hatai/Pelway/issues/events?per_page=100" > "$D/label-events.json" 2>/dev/null || rm -f "$D/label-events.json"
  python3 -I - "$D" "$P" "$IL" "$PL" "$C" "$trc" "$T3_BIN" <<'PY'
import json, sys, time, os, re, secrets, stat, subprocess
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
       "decision_warned": {} if prev is None else prev.get("decision_warned", {}),
       "shared_notified": {} if prev is None else prev.get("shared_notified", {}),
       "heartbeat_at": 0 if prev is None else prev.get("heartbeat_at", 0),
       "thread_list": None if prev is None else prev.get("thread_list"),
       "pending_sends": None if prev is None else prev.get("pending_sends"),
       "ps_write_skipped": None if prev is None else prev.get("ps_write_skipped"),
       "slots": None if prev is None else prev.get("slots")}
# ラベル操作の裏付け検査（Issue #405、2026-11-17 の要件変更）。actor だけでは判定しない
# （エージェントが hatai のトークンで gh を動かすと、タイムラインの actor が hatai に
# なることがあるため）。needs-human の解除と automerge の付け外しを、イベント時刻の前
# LABEL_BACKUP_HOURS 時間以内の同じ Issue・PR への hatai 本人のコメント（指示の記録）と
# 突き合わせ、裏付けが無ければ「出どころ不明」として LABEL unbacked 行を出す。
# 指揮役と T3 スレッドの操作記録との突き合わせは巡回が行う
# （runbook の「指揮役の定期巡回（スケジュール起床）」）。
LABEL_BACKUP_HOURS = 24
LABEL_BACKING_AUTHOR = "hatai"
# Grok Bot も hatai の名義で書くので、needs-human の解除では Grok Bot 自身の決定のコメントを裏付けに数えない（Issue #602）
GROKBOT_DECISION_MARK = "<!-- grokbot-decision -->"
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
threads, thread_err = fetch_threads(int(sys.argv[6]))
if thread_err is None: os.replace(os.path.join(d, "threads.json.new"), os.path.join(d, "threads.json"))
try: issues = json.load(open(os.path.join(d, "issues.json")))
except Exception: issues = None
try: prs = json.load(open(os.path.join(d, "prs.json"))); prs_ok = True
except Exception: prs = []; prs_ok = False
# 巡回の時刻。PELWAY_WATCH_NOW が指すファイルに epoch 秒が書かれていればそれを使う（テスト用）。
try: now = float(open(os.environ["PELWAY_WATCH_NOW"]).read().strip())
except Exception: now = time.time()
# Issue 一覧の取得開始時刻（巡回の冒頭で巡回ループが書く。SLOTS の記録の updated_at に使う。
# 後続の取得が遅れた回に、古い使用数へ巡回処理の時刻を付けないため）。
try: issues_at = float(open(os.path.join(d, "issues-at")).read().strip())
except Exception: issues_at = now
# T3 のスレッド一覧の取得開始時刻（同じく巡回ループが書く。停止の判定の根拠にもなるため、
# 記録の updated_at はこちらと Issue 一覧の、より古い方の時刻にする）。
try: threads_at = float(open(os.path.join(d, "threads-at")).read().strip())
except Exception: threads_at = now
# 巡回の印。state.json と slots.json の両方に書き、一致すれば記録が最新の巡回のものと
# 読み手が分かる（公開に失敗した回は state.json の印だけが進む）。
cur["patrol_at"] = now
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
def safe_to(v):  # watch.log の 1 行に収めるため、宛先はスレッド ID の形に限る（conductor-rebuild の valid_id が受け付ける文字種と長さと同じ。先頭ハイフンの拒否は対象外）
    s = str(v or "")
    return s if re.fullmatch(r"[A-Za-z0-9:_.%-]{1,256}", s) else "invalid"
PS_MAX = 4194304  # patrol-state.json の読み取りと書き戻しの上限（4 MiB）
# 補助記録は sandbox の中からも書ける共有 git dir の中にある。末尾の名前だけを O_NOFOLLOW で開いても、
# 親ディレクトリ（pelway/）が symlink に差し替えられると保存先の外を読み書きするため、ボード Status 更新キューと
# 同じく、親ディレクトリを信頼できる基点（その 1 つ上。既定は git が解決した共有 git dir）から symlink を
# たどらずに開いて、自分の所有のディレクトリであることを確かめ、記録の読み書きはそのファイル記述子を基準に行う。
# 確かめられない回は記録を読まず（無いものとして扱い）、書き戻さない（Issue #699）。
def open_ps_dir(path):  # 移行の open_ps_dir と同じ（変えるときは両方を直す）
    base, name = os.path.split(os.path.dirname(os.path.abspath(path)))
    os.makedirs(base, exist_ok=True)
    bfd = os.open(base, os.O_RDONLY | os.O_DIRECTORY)
    try:
        try: os.mkdir(name, dir_fd=bfd)
        except FileExistsError: pass
        dfd = os.open(name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=bfd)
    finally:
        os.close(bfd)
    st = os.fstat(dfd)
    if not stat.S_ISDIR(st.st_mode) or st.st_uid != os.geteuid():
        os.close(dfd); raise OSError("patrol-state dir not usable")
    return dfd
ps_name = os.path.basename(sys.argv[2])
try: ps_dfd = open_ps_dir(sys.argv[2])
except OSError: ps_dfd = None
def read_ps():  # read_request と同じく通常ファイル・待ち・量に限る
    if ps_dfd is None: raise OSError("patrol-state dir not usable")
    fd = os.open(ps_name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=ps_dfd)
    with os.fdopen(fd, "rb") as f:
        st = os.fstat(f.fileno())
        if not stat.S_ISREG(st.st_mode) or st.st_size > PS_MAX:
            raise ValueError("not a regular patrol-state file")
        data = f.read(PS_MAX + 1)  # 確かめの後にファイルが増えても、読み取りは上限で切る
        if len(data) > PS_MAX:
            raise ValueError("not a regular patrol-state file")
        return data
def write_ps(obj):  # 書き戻せたら None、書き戻さなかったらその理由（dir-invalid・too-large・unencodable）を返す
    if ps_dfd is None: return "dir-invalid"
    # 次の巡回で読めない大きさ（read_ps の上限超え）なら置き換えず、読めていた記録を保つ。字下げは入れ子が
    # 深いほど膨らみ、上限内の記録でも全体を一度に整形するとメモリを使い切るため、少しずつ整形して UTF-8 の
    # 累計バイト数を数え、上限を超えた時点で打ち切る（json.dumps と同じ出力）。
    # 整形が例外で止まる記録（json.loads は C の再帰の上限まで読めるが、整形は Python の再帰の上限で
    # RecursionError になる深い入れ子など）も置き換えず、読めていた記録を保って理由を返す。
    parts, size = [], 0
    try:
        for chunk in json.JSONEncoder(ensure_ascii=False, indent=2, sort_keys=True).iterencode(obj):
            b = chunk.encode("utf-8"); size += len(b)
            if size > PS_MAX: return "too-large"
            parts.append(b)
    except Exception:
        return "unencodable"
    data = b"".join(parts)
    # symlink の先・FIFO に書かず、同じディレクトリの一時ファイルからの置き換えで行う。一時ファイルは
    # 予測できない名前を O_EXCL|O_NOFOLLOW・0600 で作る（ボード Status 更新キューの結果ファイルと同じ）。
    # 置き換えた記録はこの一時ファイルそのものなので、umask によらず許可ビットは 0600 を超えない。
    tmp = f".patrol-state.{secrets.token_hex(8)}.tmp"
    fd = os.open(tmp, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600, dir_fd=ps_dfd)
    try:
        with os.fdopen(fd, "wb") as f:
            f.write(data)
        # 置き場が symlink・FIFO なら、指す先ではなくそれ自体を置き換える
        os.replace(tmp, ps_name, src_dir_fd=ps_dfd, dst_dir_fd=ps_dfd)
    except BaseException:
        try: os.unlink(tmp, dir_fd=ps_dfd)
        except OSError: pass
        raise
    return None
def iso(s):
    try: return datetime.fromisoformat(str(s).replace("Z", "+00:00"))
    except Exception: return None
def agent_gh(*args, stdin=None):  # scripts/agent-gh dev の標準出力。失敗（終了コード・例外）は None
    io = {"input": stdin} if stdin is not None else {"stdin": subprocess.DEVNULL}
    try:
        r = subprocess.run(["scripts/agent-gh", "dev", *args], stdout=subprocess.PIPE,
                           stderr=subprocess.DEVNULL, text=True, timeout=60, **io)
    except Exception: return None
    return r.stdout if r.returncode == 0 else None
def gh_json(*args):
    try: return json.loads(agent_gh(*args))
    except Exception: return None
# 見張りが動いていることを示す行（Issue #504）。HEARTBEAT_SECS ごとに 1 行。
# PELWAY_WATCH_HEARTBEAT_SECS で差し替えられる（テスト用）。
HEARTBEAT_SECS = env_secs("PELWAY_WATCH_HEARTBEAT_SECS", 600)
hb = now - (cur["heartbeat_at"] or 0) >= HEARTBEAT_SECS
if hb:
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
        psw = json.loads(read_ps())
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
    def human_backed(num, t, skip_mark=None):
        # num の Issue・PR のコメントから t の前 LABEL_BACKUP_HOURS 時間〜t の hatai のコメント（指示の記録。skip_mark 以外）を探す。取得失敗は None（判定の延期）
        if num not in comments_cache:
            cs = gh_json("api", f"/repos/hatai/Pelway/issues/{num}/comments?per_page=100&sort=created&direction=desc")
            comments_cache[num] = cs if isinstance(cs, list) else None
        cs = comments_cache[num]
        if cs is None: return None
        for c in cs:
            if (c.get("user") or {}).get("login") != LABEL_BACKING_AUTHOR: continue
            if skip_mark and skip_mark in (c.get("body") or ""): continue
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
            backed = human_backed(num, t, GROKBOT_DECISION_MARK if lab == "needs-human" else None)
            if backed is None: continue  # comments 取得失敗: 次の巡回で再試行
            if not backed:
                actor = ((e.get("actor") or {}).get("login") or "")
                what = f"PR #{num}" if iss.get("pull_request") else f"issue #{num}"
                print(f"LABEL unbacked {kind} {lab} {what} at={e.get('created_at')} by={actor}", flush=True)
            done.add(evid)
    else:
        done.update(e.get("id") for e in levs if isinstance(e, dict))
    cur["label_seen"] = [e.get("id") for e in levs if isinstance(e, dict) and e.get("id") in done]
# needs-decision（実装担当の停止の印。#603）が最新の labeled から DECISION_SECS たっても外れない Issue を DECISION overdue で知らせ（同じ labeled_at に 1 回）、指揮役が動いていなければ needs-human に格上げする（#602）
DECISION_SECS = 15 * 60
ESCALATION_MARK = "<!-- conductor-watch-escalation -->"
ESCALATION_AUTHOR = "pelway-dev[bot]"
# 格上げの段の記録（Issue 番号 → labeled_at・stage・seen・reason）。どこで止まっても次の周が照合できるよう、段が変わるたびにその場で置き換える
EP = os.path.join(d, "escalation.json")
try: esc = {k: v for k, v in json.load(open(EP)).items() if isinstance(v, dict)}
except Exception: esc = {}
def store():
    with open(EP + ".tmp", "w") as f: json.dump(esc, f)
    os.replace(EP + ".tmp", EP)
def remember(n, lt, stage, **more): esc[n] = {"labeled_at": lt, "stage": stage, **more}; store()
def all_pages(path):  # --paginate --slurp の応答（ページの配列）を 1 つの一覧にする。取得失敗は None
    data = gh_json("api", path, "--paginate", "--slurp")
    if not isinstance(data, list) or not all(isinstance(p, list) for p in data): return None
    return [x for p in data for x in p if isinstance(x, dict)]
def registered_thread():
    # 登録ファイルの 1 行目（スレッド ID）。無い・空・通常のファイルでない（symlink・FIFO）ときは ""、ほかの理由で読めないときは None（判定の保留）。
    # O_NOFOLLOW|O_NONBLOCK と fd での種類・所有者の確認は pelway-agent-watch の read_conductor_file、1 行目の trim は parse_conductor_file と同じ（通常でないファイルを不在・UTF-8 を置き換えて読む点だけ違う）
    p = sys.argv[5]
    try:
        fd = os.open(p, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    except FileNotFoundError: return ""
    except OSError:
        try: return None if stat.S_ISREG(os.lstat(p).st_mode) else ""
        except OSError: return None
    try:
        st = os.fstat(fd)
        if not stat.S_ISREG(st.st_mode): return ""
        if st.st_uid != os.geteuid(): return None
        return os.read(fd, 4096).decode("utf-8", "replace").split("\n", 1)[0].strip()
    except OSError: return None
    finally: os.close(fd)
def conductor_absence():  # 指揮役が T3 のスレッドとして動いていないと確かめられたときだけ理由の語（ほかは None）
    tid = registered_thread()
    if tid is None: return None
    if not tid: return "no-registration"
    th = next((t for t in threads if isinstance(t, dict) and t.get("threadId") == tid), None)
    if th is None: return "thread-missing"
    return "thread-archived" if th.get("archived") else None
def events(n): return all_pages(f"/repos/hatai/Pelway/issues/{n}/events?per_page=100")  # 全ページ。取得失敗は None
def label_events(evs, label):  # evs のうち label の付け外しのイベント（GitHub が返す古い順のまま）
    return [e for e in evs or [] if e.get("event") in ("labeled", "unlabeled") and (e.get("label") or {}).get("name") == label]
def last_event(es): return (es or [{}])[-1].get("event")
def open_labeling(evs):  # needs-decision の今の付与の時刻。最後の付け外しが外したもの・見つからない・時刻が読めないときは None
    last = (label_events(evs, "needs-decision") or [{}])[-1]
    return last.get("created_at") if last.get("event") == "labeled" and iso(last.get("created_at")) else None
def escalate(n, lt, reason):
    # 3 段（通知 → needs-human の付与 → needs-decision の解除）は runbook の格上げの項どおり。各段は記録と書く直前の読み直し（events・Issue・events）だけで決め、読み取りに失敗した周は書かない
    body = (f"{ESCALATION_MARK}\n@hatai `needs-decision` が付いた時刻 {lt} から 15 分決まらず、"
            "指揮役が動いていないため `needs-human` に替えました。")
    steps = {None: ("comment", ["issue", "comment", n, "--body-file", "-"], None, "commented"),
             "commented": ("label", ["issue", "edit", n, "--add-label", "needs-human"], "adding", "added"),
             "added": ("label", ["issue", "edit", n, "--remove-label", "needs-decision"], None, "removed")}
    add_tried = False
    while True:
        rec = esc.get(n) if (esc.get(n) or {}).get("labeled_at") == lt else {}
        recorded = rec.get("stage")
        stage = recorded
        if stage not in (None, "commented", "adding", "added"): return  # held・removed: この labeled_at にはもう書かない
        if stage is None:
            cs = all_pages(f"/repos/hatai/Pelway/issues/{n}/comments?per_page=100")
            if cs is None: return
            # 通知の重複を防ぐだけの確かめ（実装担当も同じ名義で書くので、ラベルの操作や保留の判定の根拠にはしない）
            if any((c.get("user") or {}).get("login") == ESCALATION_AUTHOR and c.get("body") == body for c in cs): stage = "commented"
        evs = events(n)
        iss = gh_json("api", f"/repos/hatai/Pelway/issues/{n}") if evs is not None else None
        if not isinstance(iss, dict): return
        evs = events(n)  # Issue の読み直しの間に外して付け直された付与を、古い付与のまま扱わない
        if evs is None: return
        labs = {l.get("name") for l in (iss.get("labels") or []) if isinstance(l, dict)}
        # Issue の読み取りとの間の付け外し・開閉は最後の events の方が新しい: events の最後のイベントで確かめ直す
        open_ = iss.get("state") == "open" and last_event([e for e in evs if e.get("event") in ("closed", "reopened")]) in (None, "reopened")
        if not open_ or "needs-decision" not in labs or open_labeling(evs) != lt: return
        nh = label_events(evs, "needs-human")
        human = "needs-human" in labs or bool(nh and nh[-1]["event"] == "labeled")
        if stage == "adding":
            new = [e for e in label_events(evs, "needs-human") if isinstance(e.get("id"), int) and e["id"] > rec.get("seen", 0)]
            if any(e["event"] == "labeled" and (e.get("actor") or {}).get("login") == ESCALATION_AUTHOR for e in new):
                remember(n, lt, "added", reason=reason)
                stage = "added"  # 自分の名義で付いた。その後に人間が外していても付け直さず、needs-decision を外す
            elif new: stage = "held"  # 試行の後にほかの誰かが付け外しした: 出どころを確かめられない
            elif human: return  # 付いたが events にまだ出ていない: 次の周で照合する
            elif add_tried: return  # この周に付け直したのに付与もラベルも見えない: 次の周に回す
            else: stage = "commented"  # 付かなかった: 付け直す
        elif human and stage != "added":  # 自分で付けていない needs-human はそのままにする（記録した通知の後なら止める）
            if recorded != "commented": return
            stage = "held"
        if stage == "held": remember(n, lt, "held", reason=reason); print(f"DECISION held #{n} labeled_at={lt}", flush=True); return
        step, args, intent, done = steps[stage]
        if intent: remember(n, lt, intent, seen=max((e["id"] for e in evs if isinstance(e.get("id"), int)), default=0), reason=reason)
        if agent_gh(*args, stdin=body if stage is None else None) is None: print(f"DECISION escalate-failed #{n} step={step}", flush=True); return
        if intent: add_tried = True; continue  # 付与は終了コードでなく試行の後の付与イベントで照合する（変更なしの成功・応答の喪失と取り違えない）
        remember(n, lt, done, reason=reason)
        if done == "removed": print(f"DECISION escalated #{n} labeled_at={lt} reason={reason}", flush=True); return
decision_at, late_human = {}, set()  # 最新の needs-decision の labeled_at（補助記録の next_check_at に使う）・一覧の後に events で needs-human が付いた Issue
if issues is not None:
    labeled = set()
    reason = conductor_absence() if thread_err is None else None  # 一覧の取得に失敗した周は判定しない
    for iss in issues:
        labs = {l.get("name") for l in (iss.get("labels") or []) if isinstance(l, dict)}
        if iss.get("state") != "OPEN" or "needs-decision" not in labs: continue
        n = str(iss.get("number")); labeled.add(n)
        # needs-human もあれば人間待ち（DECISION も補助記録の起点も出さない）。格上げが途中の Issue だけ、前の付与の完了を照合する（書くのは指揮役がいない周だけ）
        human = "needs-human" in labs
        if human and (esc.get(n) or {}).get("stage") not in ("commented", "adding", "added"): continue
        evs = events(n); lt = open_labeling(evs)
        if lt is None: continue  # 取得失敗・イベントが見つからない: 遅れとは扱わず、次の周で取り直す
        old = esc.get(n)
        if old and old.get("labeled_at") != lt:
            # 前の labeled_at の途中の記録: 新しい付与が開いているので前の付与の解除は確定。added なら完了を 1 回報告してから替える
            if old.get("stage") == "added":
                print(f"DECISION escalated #{n} labeled_at={old.get('labeled_at')} reason={old.get('reason') or reason}", flush=True)
            del esc[n]; store()
        if last_event(label_events(evs, "needs-human")) == "labeled": late_human.add(n)  # 補助記録も人間待ちにする
        if not human and n not in late_human: decision_at[n] = lt
        if now - iso(lt).timestamp() < DECISION_SECS: continue
        # 一覧の後の needs-human の付与・クローズが events に出ていれば知らせず、通知済みにもしない
        gone = n in late_human or last_event([e for e in evs if e.get("event") in ("closed", "reopened")]) == "closed"
        if not human and not gone and cur["decision_warned"].get(n) != lt: print(f"DECISION overdue #{n} labeled_at={lt}", flush=True); cur["decision_warned"][n] = lt
        if reason: escalate(n, lt, reason)
    # 一覧から外れた Issue（閉じた・一覧の窓外）の通知済み時刻も残す。同じ labeled_at で再出現しても重複して知らせない
    # 処理対象外の Issue に途中の記録があれば Issue の状態で照合する（読むだけ）。needs-decision が外れていれば消す（完了の行は added のときだけ）、残っていれば残す。held は読まずに残し、removed は消す
    keep = set(labeled)
    for n, r in esc.items():
        if n in keep: continue
        if r.get("stage") == "held": keep.add(n); continue  # 同じ labeled_at の保留は残す
        if r.get("stage") not in ("commented", "adding", "added"): continue
        iss = gh_json("api", f"/repos/hatai/Pelway/issues/{n}")
        if not isinstance(iss, dict): keep.add(n); continue  # 読めなければ次の周に照合する
        labs = {l.get("name") for l in (iss.get("labels") or []) if isinstance(l, dict)}
        if "needs-decision" in labs: keep.add(n); continue
        if r.get("stage") == "added":
            print(f"DECISION escalated #{n} labeled_at={r.get('labeled_at')} reason={r.get('reason') or reason}", flush=True)
    esc = {k: v for k, v in esc.items() if k in keep}; store()
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
            r = subprocess.run([sys.argv[7], "thread", "show", tid, "--json"],
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
# 記録は sandbox の中からも置き換えられるため read_ps で読み（Issue #699）、読めないもの（JSON でない・
# 辞書でない・通常ファイルでない・4 MiB を超える）は無いものとして扱って、下の書き戻しで新しい記録に置き換える
# （親ディレクトリを確かめられない回は、無いものとして扱い、書き戻さない）。
try: ps = json.loads(read_ps())
except Exception: ps = {}
if not isinstance(ps, dict): ps = {}
if not isinstance(ps.get("subjects"), dict): ps["subjects"] = {}
# SLOTS 行（Issue #714）。実装・調査に使っている枠の数を agent-team.toml の上限と
# 突き合わせて出す。指揮役は割り当ての前に使用数を読む（runbook「3. 同時数と順番待ち」。
# 判断に使うのは watch.log の行でなく、見張り自身が巡回ごとに書き直す slots.json）。
# agent-team.toml が作業ツリーに無いときは SLOTS の行を出さない。使用数を確かめられない
# 巡回（issue 一覧の取得失敗・件数の打ち切り・agent-team.toml の検証失敗）は
# SLOTS error <理由> を出し、使用数は出さない（直前の巡回の行が残ったまま使われないようにする）。
# 出すのは、使用数または状態が前回の巡回から変わったときと、HEARTBEAT の行を出した巡回。
# Issue を持たない調査（入口B）は数えない（指揮役が巡回記録から加算する）。数えるのは
# open で agent:working の付いた Issue で、agent:devin が付いたものは devin に、ほかは
# opencode に入れる。作業状態の印が停止中・再開処理中・再開待ちのものは、印の裏付けを
# 取ってから使っていない枠として除く: 補助記録は実装エージェントも書き込め、pelway-dev[bot] は
# 実装担当と指揮役の共通名義のため、GitHub の記録では実際の停止を確かめられない。代わりに
# 実装スレッドの実際の状態（T3 の一覧。題名が「[実装] #<番号> 」のもの）で確かめ、その Issue の
# スレッドがどれも動いていない（完了・失敗・中断・取消、アーカイブを含む）ものだけを除く。
# 動いているスレッドが残るもの・スレッドが見つからないもの（一覧の取得失敗を含む）は
# 使用中として数える。
slots_st = "unavailable"; slots_reason = "agent-team.toml not in worktree"
slots_usage = None; slots_lines = None; slots_sig = "unavailable"
if os.path.isfile("agent-team.toml"):
    if issues is None:
        slots_st, slots_reason = "error", "issue-list-failed"
    elif len(issues) >= issue_limit:
        slots_st, slots_reason = "error", "issue-list-limit"
    else:
        try:
            r = subprocess.run(["scripts/agent-team-config", "slots", "hatai"],
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=60)
            if r.returncode != 0:
                reason = next((ln.strip() for ln in r.stderr.splitlines() if ln.strip()), None) or f"exit={r.returncode}"
                slots_st, slots_reason = "error", reason[:160]
            else:
                limits = {}
                for ln in r.stdout.splitlines():
                    p = ln.split()
                    if len(p) == 2 and p[1].isdigit(): limits[p[0]] = int(p[1])
                marks = set()
                items = ps.get("items")
                if isinstance(items, dict):
                    for n, it in items.items():
                        ws = it.get("work_state") if isinstance(it, dict) else None
                        if isinstance(ws, dict) and ws.get("state") in ("停止中", "再開処理中", "再開待ち"):
                            marks.add(str(n))
                impl = {}
                for t in threads:
                    if not isinstance(t, dict): continue
                    tm = re.match(r"^\[実装\] #(\d+) ", t.get("title") or "")
                    if tm: impl.setdefault(tm.group(1), []).append(t)
                stopped = ("completed", "failed", "interrupted", "cancelled")
                def is_stopped(t): return t.get("status") in stopped or t.get("archived")
                by_id = {t.get("threadId"): t for t in threads if isinstance(t, dict)}
                def bound_ids(it):
                    # implement_thread の組（PR の番号・thread_id・push 先のブランチ。組の形は
                    # dict でも 3 要素の列でも読む）から結び付いたスレッドの ID を全部挙げる。
                    # 読めない組が 1 つでもあれば結び付きを確かめられないとして None。
                    ids = []
                    for pair in it.get("implement_thread") or []:
                        if isinstance(pair, dict): tid = pair.get("thread_id")
                        elif isinstance(pair, (list, tuple)) and len(pair) >= 2: tid = pair[1]
                        else: tid = None
                        if not isinstance(tid, str) or not tid: return None
                        ids.append(tid)
                    return ids or None
                def stop_verified(n):
                    # 枠を空ける印の裏付け。現在の割り当てに結び付いたスレッド（補助記録の
                    # implement_thread の組の thread_id）が T3 の一覧にあってどれも動いていない
                    # （完了・失敗・中断・取消、アーカイブを含む）ことと、一覧にある
                    # [実装] #<番号> のスレッドがどれも動いていないことの両方を確かめる。
                    # 組の無いもの・組のスレッドが一覧に無いもの（起動した引き継ぎ先が一時的に
                    # 一覧から欠ける回を含む）・割り当ての照合が残るもの（rollback の割り当て
                    # ID）・一覧に動いているものが残るものは確かめられず使用中として数える。
                    it = items.get(n) if isinstance(items, dict) else None
                    if not isinstance(it, dict) or it.get("rollback") is not None: return False
                    ids = bound_ids(it)
                    if ids is None: return False
                    if any(i not in by_id or not is_stopped(by_id[i]) for i in ids): return False
                    ts = impl.get(n)
                    if not ts: return False
                    return all(is_stopped(t) for t in ts)
                used = {name: 0 for name in limits}
                for iss in issues:
                    if not isinstance(iss, dict) or iss.get("state") != "OPEN": continue
                    labs = {lb.get("name") for lb in (iss.get("labels") or []) if isinstance(lb, dict)}
                    n = str(iss.get("number"))
                    if "agent:working" not in labs or (n in marks and stop_verified(n)): continue
                    who = "devin" if "agent:devin" in labs else "opencode"
                    if who in used: used[who] += 1
                slots_usage = {n: {"used": used[n], "limit": lim} for n, lim in limits.items()}
                slots_lines = [f"SLOTS{' over' if used[n] > lim else ''} {n} {used[n]}/{lim}" for n, lim in limits.items()]
                slots_st, slots_reason, slots_sig = "ok", None, "|".join(slots_lines)
        except Exception as e:
            slots_st, slots_reason = "error", f"agent-team-config failed ({type(e).__name__})"
if slots_st != "ok": slots_sig = f"{slots_st}|{slots_reason}"
emit = None
if slots_st == "ok": emit = slots_lines
elif slots_st == "error": emit = [f"SLOTS error {slots_reason}"]
if emit is not None and (cur["slots"] != slots_sig or hb):
    for ln in emit: print(ln, flush=True)
cur["slots"] = slots_sig
# 指揮役の照合用の記録（Issue #714）。キューに書けるファイル名がそのまま watch.log の行に
# なるため、行の見た目だけでは出どころを区別できない。巡回ごとに最新の判定を書き直し、
# 直前の巡回の結果が残って使われないようにする。読み手の規約は runbook「3. 同時数と順番待ち」。
rec = {"updated_at": min(issues_at, threads_at), "patrol_at": now, "status": slots_st}
if slots_reason is not None: rec["reason"] = slots_reason
if slots_usage is not None: rec["usage"] = slots_usage
# 同じディレクトリの一時ファイルからの置き換えで公開し、読み手に空・書きかけの記録が
# 見えないようにする（write_ps と同じ形）。先に前回の記録を外してから公開する:
# 公開に失敗した回に前回の記録が残ると、読み手が鮮度の条件の内側で古い status:ok を
# 使い続ける。書けない巡回はそのまま次へ進める: 外れた・古い記録は読み手の規約どおり
# 実測の数え方に戻る（runbook「3. 同時数と順番待ち」）。
try:
    try: os.unlink(os.path.join(d, "slots.json"))
    except FileNotFoundError: pass
    tmp = os.path.join(d, f".slots.{secrets.token_hex(8)}.tmp")
    fd = os.open(tmp, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    try:
        with os.fdopen(fd, "w") as f: json.dump(rec, f, ensure_ascii=False)
        os.replace(tmp, os.path.join(d, "slots.json"))
    except BaseException:
        try: os.unlink(tmp)
        except OSError: pass
        raise
except Exception:
    pass
json.dump(cur, open(sp, "w"))
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
    # 指揮役の受信箱（Issue #555）にある、そのスレッドの最新の知らせの状態の行（skill implement-issue の
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
    # 受信箱は sandbox の中からも書ける場所にあるので、symlink をたどらず・FIFO で止まらずに開き、
    # 通常ファイル・自分の所有のときだけ読む。読むのは末尾の INBOX_TAIL バイト以内で、read 1 回で
    # 読む（ファイルが伸びても量は増えない）。途中から読み始めたときは、最初の LF までの断片を捨てる。
    # 行は LF だけで分け、最後の LF の後ろ（書き込み中の行）は読まない。1 行ずつ UTF-8 と JSON として
    # 読み、読めない行・型の違う行は飛ばして後ろの行を読む。
    INBOX_TAIL = 4 * 1024 * 1024
    inbox_lines = []
    try:
        ifd = os.open(os.path.join(os.path.dirname(ps_path), "conductor-inbox.jsonl"),
                      os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        with os.fdopen(ifd, "rb") as f:
            inbox_st = os.fstat(f.fileno())
            if stat.S_ISREG(inbox_st.st_mode) and inbox_st.st_uid == os.geteuid():
                start = max(0, inbox_st.st_size - INBOX_TAIL)
                f.seek(start)
                data = f.read(INBOX_TAIL)
                if start > 0:
                    data = data[data.find(b"\n") + 1:] if b"\n" in data else b""
                inbox_lines = data.split(b"\n")[:-1]
    except OSError:
        inbox_lines = []
    # 状態の行は、その Issue の最新の実装スレッドの threadId と thread_id が一致する知らせから選ぶ
    # （CI の連続失敗の stalled は thread_id が空文字で、実装担当の状態を上書きしない）。
    inbox_state = {}
    for raw in inbox_lines:
        try: ev = json.loads(raw.decode("utf-8"))
        except Exception: continue
        if not isinstance(ev, dict): continue
        num, tid, at, rep = ev.get("issue_number"), ev.get("thread_id"), ev.get("occurred_at"), ev.get("report")
        if not isinstance(num, int) or isinstance(num, bool) or not isinstance(tid, str) \
           or not isinstance(at, str) or not (rep is None or isinstance(rep, str)):
            continue
        k = str(num)
        if k not in impl_thread or impl_thread[k].get("threadId") != tid: continue
        st = None
        for rl in (rep or "").splitlines():
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
        if "needs-human" in labels or n in late_human:
            won, owner = "human", "hatai"
        elif "needs-decision" in labels:
            won, owner = "grokbot", "grokbot"
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
        # needs-decision の無い Grok Bot の判断待ち（grokbot）は指揮役が書く。ラベルが変わらない限り保つ
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
        # needs-decision の待ちは毎周 labeled_at から導き直す（付け直されたら起点が変わるため）
        if n in decision_at:
            nca = datetime.fromtimestamp(iso(decision_at[n]).timestamp() + DECISION_SECS).isoformat(timespec="seconds")
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
    # 書き戻さなかった回（親ディレクトリを確かめられない・上限を超える・整形できない）は理由の行を出す。続く間は
    # HEARTBEAT_SECS ごと（理由が変わればすぐ）に出し直し、書き戻せた回に回復の行を 1 つ出す。
    why = write_ps(ps)
    wsk = cur["ps_write_skipped"] if isinstance(cur["ps_write_skipped"], dict) else None
    if why is None:
        if wsk: print(f"WATCHER patrol-state-write-recovered since={stamp(wsk['since'])} after={int(now - wsk['since'])}s", flush=True)
        wsk = None
    elif wsk is None or wsk.get("reason") != why or now - wsk["notified_at"] >= HEARTBEAT_SECS:
        wsk = {"since": now if wsk is None else wsk["since"], "notified_at": now, "reason": why}
        print(f"WATCHER patrol-state-write-skipped since={stamp(wsk['since'])} reason={why} max={PS_MAX}", flush=True)
    cur["ps_write_skipped"] = wsk
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
