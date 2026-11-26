#!/usr/bin/env bash
# scripts/patrol-state-update のテスト（Issue #671）。
# patrol-state.json の共通の書き込み手順が、conductor_thread_id の持ち主でないスレッドの
# 書き込みを拒否して anomalies に記録すること、watch_log_offset・inbox_offset の
# 巻き戻し（現行値より小さい値への書き戻し）を拒否すること、merge patch の適用と
# 書き込み経路の一元化（conductor-watch.sh も同じ手順を通ること）を確かめる。あわせて親ディレクトリの検査（確かめた後の差し替えを含む）・書き戻す結果の上限・読み取りの入出力エラーの扱いを確かめる。
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUNBOOK="$REPO/docs/agents/grokbot-runbook.md"
ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT
failures=()

UPD="$REPO/scripts/patrol-state-update"
STATE="$ROOT/state/pelway/patrol-state.json"
mkdir -p "$(dirname "$STATE")"

put_state() { printf '%s' "$1" > "$STATE"; }

pget() { # pget <top-level key>
  timeout 30 python3 -I -c 'import json,sys
print(json.dumps(json.load(open(sys.argv[1])).get(sys.argv[2], ""), ensure_ascii=False))' "$STATE" "$1"
}

anoms() { # anomalies の kind 一覧（改行区切り）
  timeout 30 python3 -I -c 'import json,sys
d = json.load(open(sys.argv[1]))
for a in d.get("anomalies", []):
    if isinstance(a, dict): print(a.get("kind", ""))' "$STATE"
}

upd() { # upd <expected-rc> <patch> [args...] — 標準エラーは $LAST_ERR に残る
  local want="$1" patch="$2" got; shift 2
  printf '%s' "$patch" | timeout 30 python3 -I "$UPD" --state "$STATE" "$@" >"$ROOT/out" 2>"$ROOT/err"
  got=$?
  LAST_ERR="$(cat "$ROOT/err")"
  [[ "$got" == "$want" ]]
}

upd_at() { # upd_at <expected-rc> <state> <patch> [args...] — 置き場を指定する upd。標準エラーは $LAST_ERR に残る
  local want="$1" st="$2" patch="$3" got; shift 3
  printf '%s' "$patch" | timeout 30 python3 -I "$UPD" --state "$st" "$@" >"$ROOT/out" 2>"$ROOT/err"
  got=$?
  LAST_ERR="$(cat "$ROOT/err")"
  [[ "$got" == "$want" ]]
}

check() {
  local name="$1"; shift
  if "$@"; then printf 'ok   %s\n' "$name"; else printf 'FAIL %s\n' "$name"; failures+=("$name"); fi
}

eq() { [[ "$1" == "$2" ]]; }
has() { grep -q "$2" "$1"; }
has_all() { local f="$1"; shift; local w; for w in "$@"; do grep -q "$w" "$f" || return 1; done; }
anoms_has() { anoms | grep -qx "$1"; }

# --- merge patch の適用と鍵の保持 ---
put_state '{"conductor_thread_id":"mcp:cur","watch_log_offset":500,"inbox_offset":40,
 "subjects":{"11":{"waiting_on":"review"}},"items":{"11":{"main_pr":101}}}'

check "matching writer-thread applies patch" upd 0 '{"watch_log_offset":600}' --writer-thread mcp:cur
check "patch applied" eq "$(pget watch_log_offset)" '600'
check "untouched top-level keys preserved" has <(pget items) '"main_pr"'
check "conductor_thread_id preserved" eq "$(pget conductor_thread_id)" '"mcp:cur"'
check "deep merge keeps sibling keys" upd 0 '{"items":{"11":{"work_state":"再開待ち"}}}' --writer-thread mcp:cur
check "merged items keep main_pr" has <(pget items) '"main_pr"'
check "merged items add work_state" has <(pget items) '再開待ち'
check "null deletes key" upd 0 '{"items":{"11":{"work_state":null}}}' --writer-thread mcp:cur
check "work_state deleted" eq "$(python3 -I -c 'import json,sys
print("work_state" in json.load(open(sys.argv[1]))["items"]["11"])' "$STATE")" "False"

check "patch list applies in order" upd 0 '[{"subjects":null},{"subjects":{"12":{"waiting_on":"human"}}}]' --writer-thread mcp:cur
check "list patch replaced subjects" eq "$(pget subjects)" '{"12": {"waiting_on": "human"}}'
# patch のオブジェクトは、既存値がオブジェクトでなくても深く併合される
# （この手順の併合仕様。中の null は常に鍵を消し、値として残らない）
put_state '{"conductor_thread_id":"mcp:cur","watch_log_offset":600,"items":5}'
check "object patch deep-merges over non-object value" upd 0 '{"items":{"a":null,"b":1}}' --writer-thread mcp:cur
check "inner null is a delete, not a stored value" eq "$(pget items)" '{"b": 1}'
put_state '{"conductor_thread_id":"mcp:cur","watch_log_offset":600,"inbox_offset":40}'
# --- conductor_thread_id の持ち主でない書き手（Issue #671 受け入れ条件 1） ---
put_state '{"conductor_thread_id":"mcp:cur","watch_log_offset":600,"inbox_offset":40}'
check "foreign thread write refused" upd 3 '{"watch_log_offset":700,"last_patrol_note":"old"}' --writer-thread mcp:old
check "refused patch not applied" eq "$(pget watch_log_offset)" '600'
check "refusal records mismatch anomaly" anoms_has "conductor-mismatch"
check "stderr explains refusal" has <(printf '%s' "$LAST_ERR") "mcp:old"
check "patch setting another thread id refused" upd 3 '{"conductor_thread_id":"mcp:old"}' --writer-thread mcp:cur
check "thread id still current" eq "$(pget conductor_thread_id)" '"mcp:cur"'
# 引き継ぎ・交代: --claim-conductor で新指揮役が自分の ID にする
check "claim succeeds" upd 0 '{"scheduled_task_id":"sched-new"}' --writer-thread mcp:new --claim-conductor
check "claim sets conductor_thread_id" eq "$(pget conductor_thread_id)" '"mcp:new"'
check "claim applies the rest of the patch" eq "$(pget scheduled_task_id)" '"sched-new"'
check "claim forces writer id over patch value" upd 0 '{"conductor_thread_id":"mcp:evil"}' --writer-thread mcp:new --claim-conductor
check "writer id wins" eq "$(pget conductor_thread_id)" '"mcp:new"'
check "old conductor now refused" upd 3 '{"watch_log_offset":700}' --writer-thread mcp:cur
put_state '{"conductor_thread_id":"mcp:cur","watch_log_offset":600,"inbox_offset":40}'
# 登録済みの conductor_thread_id は --claim-conductor でだけ変更できる。
# 現指揮役自身の通常 patch による削除・無効化・別の値への変更も拒否する
# （この項目が失われると後続の持ち主照合が素通りになるため）。
check "patch deleting thread id refused" upd 3 '{"conductor_thread_id":null,"watch_log_offset":601}' --writer-thread mcp:cur
check "delete refused keeps thread id" eq "$(pget conductor_thread_id)" '"mcp:cur"'
check "delete refused keeps offset" eq "$(pget watch_log_offset)" '600'
check "delete refusal recorded" anoms_has "conductor-id-change-refused"
check "patch emptying thread id refused" upd 3 '{"conductor_thread_id":""}' --writer-thread mcp:cur
check "patch non-string thread id refused" upd 3 '{"conductor_thread_id":123}' --writer-thread mcp:cur
check "thread id still registered" eq "$(pget conductor_thread_id)" '"mcp:cur"'
# 同じ値の書き戻しは変更にならないので許す
check "patch rewriting same thread id applies" upd 0 '{"conductor_thread_id":"mcp:cur","watch_log_offset":601}' --writer-thread mcp:cur
check "same-id write applied" eq "$(pget watch_log_offset)" '601'
put_state '{"conductor_thread_id":"mcp:cur","watch_log_offset":600,"inbox_offset":40}'
# スレッド ID を名乗らない書き手も conductor_thread_id を変えられない
check "non-thread writer cannot change thread id" upd 3 '{"conductor_thread_id":"mcp:evil","subjects":{"9":{"waiting_on":"human"}}}' --writer conductor-watch
check "non-thread change keeps thread id" eq "$(pget conductor_thread_id)" '"mcp:cur"'
check "non-thread change refused wholesale" eq "$(pget subjects)" '""'
check "non-thread refusal recorded" anoms_has "conductor-id-change-refused"
check "non-thread cannot delete thread id" upd 3 '{"conductor_thread_id":null}' --writer conductor-watch
check "non-thread delete keeps thread id" eq "$(pget conductor_thread_id)" '"mcp:cur"'

# 登録が残るので、削除を試みた後も旧指揮役の書き込みは拒否され続ける
check "old conductor still refused after tamper tries" upd 3 '{"watch_log_offset":700}' --writer-thread mcp:old

# conductor_thread_id が無い（未記録の）状態では書ける
put_state '{"watch_log_offset":10}'
check "unclaimed state accepts thread write" upd 0 '{"watch_log_offset":20}' --writer-thread mcp:any
check "unclaimed write applied" eq "$(pget watch_log_offset)" '20'

# 登録前の既存ファイル: 登録も --claim-conductor で行い、通常の patch では登録できない
check "unclaimed: patch cannot register thread id" upd 3 '{"conductor_thread_id":"mcp:a","watch_log_offset":30}' --writer-thread mcp:a
check "unclaimed: refused write not applied" eq "$(pget watch_log_offset)" '20'
check "unclaimed: non-thread cannot register thread id" upd 3 '{"conductor_thread_id":"mcp:a"}' --writer conductor-watch
check "unclaimed: still no thread id" eq "$(pget conductor_thread_id)" '""'
check "unclaimed: claim registers id" upd 0 '{"watch_log_offset":30}' --writer-thread mcp:a --claim-conductor
check "unclaimed: claim sets id" eq "$(pget conductor_thread_id)" '"mcp:a"'
put_state '{"conductor_thread_id":"mcp:cur","watch_log_offset":600,"inbox_offset":40}'

# --- オフセットの巻き戻り保護（Issue #671 受け入れ条件 2） ---
check "watch_log_offset regression refused" upd 3 '{"watch_log_offset":100}' --writer-thread mcp:cur
check "regression refused keeps value" eq "$(pget watch_log_offset)" '600'
check "regression refusal recorded" anoms_has "offset-regression-refused"
check "inbox_offset regression refused" upd 3 '{"inbox_offset":10}' --writer-thread mcp:cur
check "inbox regression refused keeps value" eq "$(pget inbox_offset)" '40'

check "same offset is not a regression" upd 0 '{"watch_log_offset":600}' --writer-thread mcp:cur
check "larger offset applies" upd 0 '{"watch_log_offset":601,"inbox_offset":41}' --writer-thread mcp:cur
check "advanced offsets stored" eq "$(pget inbox_offset)" '41'

check "allow flag applies regression" upd 0 '{"inbox_offset":5}' --writer-thread mcp:cur --allow-offset-regression
check "allowed regression applied" eq "$(pget inbox_offset)" '5'
check "allowed regression recorded" anoms_has "offset-regression"
put_state '{"conductor_thread_id":"mcp:cur","watch_log_offset":600,"inbox_offset":40}'
check "non-integer offset refused" upd 3 '{"watch_log_offset":"abc"}' --writer-thread mcp:cur
check "bool offset refused" upd 3 '{"watch_log_offset":true}' --writer-thread mcp:cur
check "negative offset refused" upd 3 '{"watch_log_offset":-1}' --writer-thread mcp:cur
check "offset still 600" eq "$(pget watch_log_offset)" '600'

# --- 書き手でない名乗り（conductor-watch など）はスレッド照合の対象外 ---
check "non-thread writer skips thread check" upd 0 '{"subjects":{"9":{"waiting_on":"conductor"}},"updated_at":"t0"}' --writer conductor-watch
check "non-thread write applied" has <(pget subjects) '"9"'
check "non-thread write keeps offsets" eq "$(pget watch_log_offset)" '600'

# --- ファイルの無い・壊れた状態 ---
rm -f "$STATE"
check "missing state file is created" upd 0 '{"a":1}' --writer conductor-watch
check "created file has patch" eq "$(pget a)" '1'

# --- 読めない状態（壊れた JSON・非原子的な直書きの途中）---
# 元のファイルは <state>.unreadable-<時刻> に退避され、照合できない書き込みは拒否する。
put_state '{"broken'
check "corrupt: foreign thread write refused" upd 3 '{"watch_log_offset":3796,"last_patrol_note":"old"}' --writer-thread mcp:old
check "corrupt: refusal recorded" anoms_has "owner-unverifiable"
check "corrupt: refused write not applied" eq "$(pget watch_log_offset)" '""'
check "corrupt: broken file backed up" bash -c 'compgen -G "$1.unreadable-*" >/dev/null' _ "$STATE"
check "corrupt: backup holds original bytes" bash -c 'grep -lq broken "$1".unreadable-*' _ "$STATE"
check "corrupt: unreadable recorded" anoms_has "state-unreadable"

# 退避と回復状態の保存の間で中断しても、正規パスには読めない元ファイルか
# 回復を待つ新しい状態のどちらかが残る（残らないと次回は初回作成扱いになり、
# 持ち主照合とオフセット照合が素通りになる）。回復状態の保存に失敗した後も
# 次の書き込みは拒否され続ける。os.replace の 2 回目（退避コピーの設置の次、
# 回復状態の書き戻し）を失敗させて中断を再現する。
put_state '{"broken'
check "crash after backup leaves unreadable file at canonical path" python3 -I - "$UPD" "$STATE" <<'PY'
import importlib.util, io, json, os, sys
from importlib.machinery import SourceFileLoader

upd, state = sys.argv[1], sys.argv[2]
sys.dont_write_bytecode = True
loader = SourceFileLoader("patrol_state_update", upd)
spec = importlib.util.spec_from_loader(loader.name, loader)
m = importlib.util.module_from_spec(spec)
loader.exec_module(m)

real_replace = os.replace
calls = []
def flaky_replace(src, dst, **kw):
    calls.append(dst)
    if len(calls) > 1:
        raise OSError("simulated crash")
    return real_replace(src, dst, **kw)
os.replace = flaky_replace

sys.argv = ["patrol-state-update", "--state", state, "--writer-thread", "mcp:old"]
sys.stdin = io.StringIO('{"watch_log_offset": 5}')
try:
    m.main()
except (OSError, SystemExit):
    pass

# 中断の後: 正規パスにファイルが残り、それが読めない元ファイルであること
exists = os.path.exists(state)
unreadable = False
if exists:
    try:
        with open(state) as f:
            json.load(f)
    except Exception:
        unreadable = True
sys.exit(0 if len(calls) >= 2 and exists and unreadable else 1)
PY
check "crash sim: backup preserves original" bash -c 'grep -lq broken "$1".unreadable-*' _ "$STATE"
check "crash sim: next foreign write still refused" upd 3 '{"watch_log_offset":700}' --writer-thread mcp:old
check "crash sim: refusal recorded" anoms_has "owner-unverifiable"
check "crash sim: recovery flag set" eq "$(pget needs_conductor_claim)" 'true'
rm -f "$STATE".unreadable-*

# 拒否で書き戻した回復途中の記録からは持ち主もオフセットも確かめられないので、
# --claim-conductor の回復書き込みが来るまで拒否が続く。
check "corrupt: flag marks recovery" eq "$(pget needs_conductor_claim)" 'true'
check "corrupt: second foreign thread write still refused" upd 3 '{"watch_log_offset":3796}' --writer-thread mcp:old
check "corrupt: second write not applied" eq "$(pget watch_log_offset)" '""'
check "corrupt: non-thread offset write still refused" upd 3 '{"watch_log_offset":100}' --writer conductor-watch
check "corrupt: second offset refusal recorded" anoms_has "offset-unverifiable"
check "corrupt: patch cannot clear flag" upd 0 '{"needs_conductor_claim":false,"a":1}' --writer conductor-watch
check "corrupt: flag survives patch" eq "$(pget needs_conductor_claim)" 'true'
put_state '{"broken'
check "corrupt: offset write by non-thread refused" upd 3 '{"watch_log_offset":100}' --writer conductor-watch
check "corrupt: offset refusal recorded" anoms_has "offset-unverifiable"
put_state '{"broken'
check "corrupt: claim recovers" upd 0 '{"scheduled_task_id":"s1"}' --writer-thread mcp:new --claim-conductor
check "corrupt: claim sets owner" eq "$(pget conductor_thread_id)" '"mcp:new"'
check "corrupt: claim clears flag" eq "$(pget needs_conductor_claim)" '""'
check "corrupt: recovery recorded" anoms_has "state-unreadable"
check "corrupt: claim applies patch" eq "$(pget scheduled_task_id)" '"s1"'
put_state '{"broken'
check "corrupt: non-offset write by non-thread applies" upd 0 '{"a":2}' --writer conductor-watch
check "corrupt: non-offset write applied" eq "$(pget a)" '2'
put_state '{"conductor_thread_id":"mcp:cur","watch_log_offset":600,"inbox_offset":40}'

# --- 正規パス・ロックパスの symlink・特殊ファイルは中身を読まず写さず拒否する ---
# （Issue #671 レビュー指摘: 差し替えでホストファイルの中身が漏れないこと）
PDIR="$(dirname "$STATE")"
rm -f "$STATE".unreadable-*
SECRET_JSON="$ROOT/host-secret.json"
printf '{"conductor_thread_id":"mcp:host","token":"TOPSECRET"}' > "$SECRET_JSON"
printf 'TOPSECRET host bytes' > "$ROOT/host-secret.txt"
rm -f "$STATE"
ln -s "$ROOT/host-secret.txt" "$STATE"
check "symlink to non-json host file refused" upd 3 '{"a":1}' --writer-thread mcp:cur
check "symlink: refusal recorded" anoms_has "state-untrusted"
check "symlink: no quarantine copy of target" bash -c '! compgen -G "$1.unreadable-*" >/dev/null' _ "$STATE"
check "symlink: host bytes not copied into pelway dir" bash -c '! grep -rlq TOPSECRET "$1"' _ "$PDIR"
check "symlink: target file unchanged" eq "$(cat "$ROOT/host-secret.txt")" 'TOPSECRET host bytes'
check "symlink: canonical replaced by recovery state" eq "$(pget needs_conductor_claim)" 'true'
rm -f "$STATE"
ln -s "$SECRET_JSON" "$STATE"
check "symlink to json object refused" upd 3 '{"a":1}' --writer-thread mcp:cur
check "symlink-json: target content not adopted as base" bash -c '! grep -q TOPSECRET "$1"' _ "$STATE"
check "symlink-json: registered id not adopted" eq "$(pget conductor_thread_id)" '""'
check "symlink-json: link target owner still refused" upd 3 '{"a":2}' --writer-thread mcp:host
rm -f "$STATE" "$STATE".unreadable-*
mkfifo "$STATE"
check "fifo state refused without blocking" bash -c 'printf "%s" "{\"a\":1}" | timeout 15 python3 -I "$1" --state "$2" --writer-thread mcp:cur 2>/dev/null; [[ $? == 3 ]]' _ "$UPD" "$STATE"
check "fifo: no quarantine copy made" bash -c '! compgen -G "$1.unreadable-*" >/dev/null' _ "$STATE"
check "fifo: procedure continues with claim" upd 0 '{"scheduled_task_id":"s3"}' --writer-thread mcp:new --claim-conductor
check "fifo: claim sets owner" eq "$(pget conductor_thread_id)" '"mcp:new"'
rm -f "$STATE"
put_state '{"conductor_thread_id":"mcp:cur","watch_log_offset":600,"inbox_offset":40}'

# ロックファイルの symlink もたどらない（flock をホストファイルに掛けない）
rm -f "$STATE.lock"
ln -s "$ROOT/host-secret.txt" "$STATE.lock"
check "symlinked lock file fails closed" upd 1 '{"a":1}' --writer conductor-watch
check "lock symlink: state file untouched" eq "$(pget watch_log_offset)" '600'
rm -f "$STATE.lock"

# --- anomalies は手順だけが記録する項目。patch からの注入・消去はできない ---
check "patch cannot inject anomalies" upd 0 '{"anomalies":[{"kind":"fake","note":"planted"}],"a":7}' --writer-thread mcp:cur
check "injected anomaly not stored" bash -c '! grep -q fake "$1"' _ "$STATE"
put_state '{"conductor_thread_id":"mcp:cur","watch_log_offset":600,"anomalies":[{"kind":"owner-unverifiable","writer":"w","at":"t"}]}'
check "patch cannot wipe anomalies" upd 0 '{"anomalies":[],"a":8}' --writer-thread mcp:cur
check "recorded anomaly survives empty patch" anoms_has "owner-unverifiable"
check "patch cannot null anomalies" upd 0 '{"anomalies":null,"a":9}' --writer-thread mcp:cur
check "recorded anomaly survives null patch" anoms_has "owner-unverifiable"
put_state '{"conductor_thread_id":"mcp:cur","watch_log_offset":600,"inbox_offset":40}'

# --- 入力の誤り ---
check "empty stdin refused" upd 2 '' --writer-thread mcp:cur
check "non-json patch refused" upd 2 'not json' --writer-thread mcp:cur
check "non-object patch refused" upd 2 '5' --writer-thread mcp:cur
check "list of non-objects refused" upd 2 '[1,2]' --writer-thread mcp:cur
check "missing writer identity refused" upd 2 '{"a":1}'
check "both writer flags refused" upd 2 '{"a":1}' --writer x --writer-thread t
check "claim without writer-thread refused" upd 2 '{"a":1}' --writer conductor-watch --claim-conductor
check "input errors leave file alone" eq "$(pget watch_log_offset)" '600'

# --- 親ディレクトリの検査（Issue #699 の write_ps と同じ守りを、手順自身がロックの前に行う） ---
# (a) 親が symlink なら rc 1 で何も書かない。リンク先には何も作らず、リンク先の既存の記録は byte 単位で変わらない
LINKROOT="$ROOT/linked"; OUT="$ROOT/outside"
mkdir -p "$OUT" "$LINKROOT"
printf '%s' '{"conductor_thread_id":"mcp:host","token":"TOPSECRET"}' > "$OUT/patrol-state.json"; cp "$OUT/patrol-state.json" "$ROOT/outside.orig"
ln -s "$OUT" "$LINKROOT/pelway"
check "symlinked parent: refused with rc 1" upd_at 1 "$LINKROOT/pelway/patrol-state.json" '{"a":1}' --writer conductor-watch
check "symlinked parent: link target byte-identical" cmp -s "$OUT/patrol-state.json" "$ROOT/outside.orig"
check "symlinked parent: nothing created at the link target" eq "$(find "$OUT" -mindepth 1 | wc -l)" 1
check "symlinked parent: stderr names the parent directory" has <(printf '%s' "$LAST_ERR") "親ディレクトリ"
# (b) ロック待ちの間に親が symlink に差し替えられても、確かめたディレクトリ（記述子）にだけ書き、リンク先には書かない
REAL="$ROOT/real"; MOVED="$ROOT/real-moved"; OUT2="$ROOT/outside2"
mkdir -p "$REAL/pelway" "$OUT2"; printf '%s' '{"a":0}' > "$REAL/pelway/patrol-state.json"
printf '%s' '{"conductor_thread_id":"mcp:host","token":"TOPSECRET"}' > "$OUT2/patrol-state.json"; cp "$OUT2/patrol-state.json" "$ROOT/outside2.orig"
python3 -I -c 'import fcntl,sys,time
f = open(sys.argv[1], "a"); fcntl.flock(f, fcntl.LOCK_EX); open(sys.argv[2], "w").write("1"); time.sleep(20)' "$REAL/pelway/patrol-state.json.lock" "$ROOT/locked" &
HOLDER=$!
until [ -e "$ROOT/locked" ]; do /bin/sleep 0.05; done  # ロックは握られた
printf '%s' '{"a":1}' | python3 -I "$UPD" --state "$REAL/pelway/patrol-state.json" --writer conductor-watch >"$ROOT/out" 2>"$ROOT/err" &
PROC=$!
# 手順が親を確かめてロックファイルを開き、待ち始めたこと（手順の fd にロックファイルが現れる）を確かめてから差し替える
for ((i = 0; i < 100; i++)); do [[ -n "$(find "/proc/$PROC/fd" -lname '*/patrol-state.json.lock' 2>/dev/null)" ]] && break; /bin/sleep 0.05; done
mv "$REAL/pelway" "$MOVED"; ln -s "$OUT2" "$REAL/pelway"
kill "$HOLDER" 2>/dev/null; wait "$HOLDER" 2>/dev/null
wait "$PROC"; B_RC=$?
check "parent swapped while waiting for the lock: write applied to the checked directory" eq "$B_RC" 0
check "parent swapped while waiting for the lock: moved directory holds the patch" has "$MOVED/patrol-state.json" '"a": 1'
check "parent swapped while waiting for the lock: link target byte-identical" cmp -s "$OUT2/patrol-state.json" "$ROOT/outside2.orig"
check "parent swapped while waiting for the lock: nothing created at the link target" eq "$(find "$OUT2" -mindepth 1 | wc -l)" 1

# --- 書き戻す結果の上限（読み取り上限と同じ 4 MiB。末尾の改行を含む） ---
# seed_near <埋め草のバイト数> — 詰めた形で読み取り上限の近くまで埋めた、読める記録を置く
seed_near() { python3 -I -c 'import json,sys
json.dump({"conductor_thread_id": "mcp:cur", "watch_log_offset": 600, "pad": "x" * int(sys.argv[2])}, open(sys.argv[1], "w"), separators=(",", ":"))' "$STATE" "$1"; }
# 埋め草 4194150: 適用結果（字下げ + subjects）は上限の 4 バイト下。4194180: 適用結果は上限の 26 バイト上、拒否の記録（異常 2 件の追記）は 351 バイト上
seed_near 4194150
check "result just under the limit is written" upd 0 '{"subjects":{"61":{"waiting_on":"implement"}}}' --writer conductor-watch
check "result just under the limit: subject stored" has <(pget subjects) '"61"'
seed_near 4194180; cp "$STATE" "$ROOT/near.orig"
check "result over the limit is not written" upd 4 '{"subjects":{"61":{"waiting_on":"implement"}}}' --writer conductor-watch
check "result over the limit: state byte-identical" cmp -s "$STATE" "$ROOT/near.orig"
check "result over the limit: stderr says too-large" has <(printf '%s' "$LAST_ERR") "too-large"
check "refusal record over the limit is not written" upd 3 '{"watch_log_offset":1}' --writer-thread mcp:old
check "refusal record over the limit: state byte-identical" cmp -s "$STATE" "$ROOT/near.orig"
check "refusal record over the limit: stderr says the attempt is not recorded" has <(printf '%s' "$LAST_ERR") "記録されていません"

# --- 読み取りの入出力エラーは、種類の不適合と分けて扱う（正常なファイルを読めなかったときは正規パスを保つ） ---
put_state '{"conductor_thread_id":"mcp:cur","watch_log_offset":600,"inbox_offset":40}'; cp "$STATE" "$ROOT/io.orig"
check "read failure exits 1 and keeps the state" python3 -I - "$UPD" "$STATE" <<'PY'
import errno, importlib.util, io, os, sys
from importlib.machinery import SourceFileLoader
upd, state = sys.argv[1], sys.argv[2]
sys.dont_write_bytecode = True
loader = SourceFileLoader("patrol_state_update", upd)
spec = importlib.util.spec_from_loader(loader.name, loader)
m = importlib.util.module_from_spec(spec); loader.exec_module(m)
before = open(state, "rb").read()
def flaky_read(fd, n): raise OSError(errno.EIO, "simulated read failure")
os.read = flaky_read
sys.argv = ["patrol-state-update", "--state", state, "--writer", "conductor-watch"]
sys.stdin = io.StringIO('{"subjects":{"9":{"waiting_on":"conductor"}}}')
try:
    m.main(); code = 0
except SystemExit as e:
    code = e.code
sys.exit(0 if code == 1 and open(state, "rb").read() == before else 1)
PY
check "read failure: the next update resumes from the original record" upd 0 '{"watch_log_offset":601}' --writer-thread mcp:cur
check "read failure: watch_log_offset continues from the recorded value" eq "$(pget watch_log_offset)" '601'
check "read failure: inbox_offset carried over" eq "$(pget inbox_offset)" '40'

# --- 書き込み経路の一元化（Issue #671 受け入れ条件 4） ---
check "conductor-watch writes via shared procedure" has "$REPO/scripts/conductor-watch.sh" "patrol-state-update"
check "conductor-watch has no direct patrol-state dump" bash -c '! grep -q "json.dump(ps, open(ps_path" "$1"' _ "$REPO/scripts/conductor-watch.sh"
check "presence check: runbook mentions write procedure and protection terms" has_all "$RUNBOOK" "patrol-state-update" "conductor_thread_id" "anomalies" "巻き戻し" "claim-conductor" "needs_conductor_claim" "state-untrusted" "O_NOFOLLOW"

if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}"
  exit 1
fi
printf '\nall passed\n'
