#!/usr/bin/env bash
# scripts/patrol-state-update のテスト（Issue #671）。
# patrol-state.json の共通の書き込み手順が、conductor_thread_id の持ち主でないスレッドの
# 書き込みを拒否して anomalies に記録すること、watch_log_offset・inbox_offset の
# 巻き戻し（現行値より小さい値への書き戻し）を拒否すること、merge patch の適用と
# 書き込み経路の一元化（conductor-watch.sh も同じ手順を通ること）を確かめる。
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
  python3 -I -c 'import json,sys
print(json.dumps(json.load(open(sys.argv[1])).get(sys.argv[2], ""), ensure_ascii=False))' "$STATE" "$1"
}

anoms() { # anomalies の kind 一覧（改行区切り）
  python3 -I -c 'import json,sys
d = json.load(open(sys.argv[1]))
for a in d.get("anomalies", []):
    if isinstance(a, dict): print(a.get("kind", ""))' "$STATE"
}

upd() { # upd <expected-rc> <patch> [args...] — 標準エラーは $LAST_ERR に残る
  local want="$1" patch="$2" got; shift 2
  printf '%s' "$patch" | python3 -I "$UPD" --state "$STATE" "$@" >"$ROOT/out" 2>"$ROOT/err"
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

# --- conductor_thread_id の持ち主でない書き手（Issue #671 受け入れ条件 1） ---
put_state '{"conductor_thread_id":"mcp:cur","watch_log_offset":600,"inbox_offset":40}'

check "foreign thread write refused" upd 3 '{"watch_log_offset":700,"last_patrol_note":"old"}' --writer-thread mcp:old
check "refused patch not applied" eq "$(pget watch_log_offset)" '600'
check "refusal records mismatch anomaly" anoms_has "conductor-mismatch"
check "stderr explains refusal" has <(printf '%s' "$LAST_ERR") "mcp:old"

check "patch setting another thread id refused" \
  upd 3 '{"conductor_thread_id":"mcp:old"}' --writer-thread mcp:cur
check "thread id still current" eq "$(pget conductor_thread_id)" '"mcp:cur"'

# 引き継ぎ・交代: --claim-conductor で新指揮役が自分の ID にする
check "claim succeeds" upd 0 '{"scheduled_task_id":"sched-new"}' --writer-thread mcp:new --claim-conductor
check "claim sets conductor_thread_id" eq "$(pget conductor_thread_id)" '"mcp:new"'
check "claim applies the rest of the patch" eq "$(pget scheduled_task_id)" '"sched-new"'
check "claim forces writer id over patch value" \
  upd 0 '{"conductor_thread_id":"mcp:evil"}' --writer-thread mcp:new --claim-conductor
check "writer id wins" eq "$(pget conductor_thread_id)" '"mcp:new"'
check "old conductor now refused" upd 3 '{"watch_log_offset":700}' --writer-thread mcp:cur
put_state '{"conductor_thread_id":"mcp:cur","watch_log_offset":600,"inbox_offset":40}'

# conductor_thread_id が無い（未記録の）状態では書ける
put_state '{"watch_log_offset":10}'
check "unclaimed state accepts thread write" upd 0 '{"watch_log_offset":20}' --writer-thread mcp:any
check "unclaimed write applied" eq "$(pget watch_log_offset)" '20'
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

check "allow flag applies regression" \
  upd 0 '{"inbox_offset":5}' --writer-thread mcp:cur --allow-offset-regression
check "allowed regression applied" eq "$(pget inbox_offset)" '5'
check "allowed regression recorded" anoms_has "offset-regression"
put_state '{"conductor_thread_id":"mcp:cur","watch_log_offset":600,"inbox_offset":40}'

check "non-integer offset refused" upd 3 '{"watch_log_offset":"abc"}' --writer-thread mcp:cur
check "bool offset refused" upd 3 '{"watch_log_offset":true}' --writer-thread mcp:cur
check "negative offset refused" upd 3 '{"watch_log_offset":-1}' --writer-thread mcp:cur
check "offset still 600" eq "$(pget watch_log_offset)" '600'

# --- 書き手でない名乗り（conductor-watch など）はスレッド照合の対象外 ---
check "non-thread writer skips thread check" \
  upd 0 '{"subjects":{"9":{"waiting_on":"conductor"}},"updated_at":"t0"}' --writer conductor-watch
check "non-thread write applied" has <(pget subjects) '"9"'
check "non-thread write keeps offsets" eq "$(pget watch_log_offset)" '600'

# --- ファイルの無い・壊れた状態 ---
rm -f "$STATE"
check "missing state file is created" upd 0 '{"a":1}' --writer conductor-watch
check "created file has patch" eq "$(pget a)" '1'

# --- 読めない状態（壊れた JSON・非原子的な直書きの途中）---
# 元のファイルは <state>.unreadable-<時刻> に退避され、照合できない書き込みは拒否する。
put_state '{"broken'
check "corrupt: foreign thread write refused" \
  upd 3 '{"watch_log_offset":3796,"last_patrol_note":"old"}' --writer-thread mcp:old
check "corrupt: refusal recorded" anoms_has "owner-unverifiable"
check "corrupt: refused write not applied" eq "$(pget watch_log_offset)" '""'
check "corrupt: broken file backed up" \
  bash -c 'compgen -G "$1.unreadable-*" >/dev/null' _ "$STATE"
check "corrupt: unreadable recorded" anoms_has "state-unreadable"

# 拒否で書き戻した回復途中の記録からは持ち主もオフセットも確かめられないので、
# --claim-conductor の回復書き込みが来るまで拒否が続く。
check "corrupt: flag marks recovery" eq "$(pget needs_conductor_claim)" 'true'
check "corrupt: second foreign thread write still refused" \
  upd 3 '{"watch_log_offset":3796}' --writer-thread mcp:old
check "corrupt: second write not applied" eq "$(pget watch_log_offset)" '""'
check "corrupt: non-thread offset write still refused" \
  upd 3 '{"watch_log_offset":100}' --writer conductor-watch
check "corrupt: second offset refusal recorded" anoms_has "offset-unverifiable"
check "corrupt: patch cannot clear flag" \
  upd 0 '{"needs_conductor_claim":false,"a":1}' --writer conductor-watch
check "corrupt: flag survives patch" eq "$(pget needs_conductor_claim)" 'true'

put_state '{"broken'
check "corrupt: offset write by non-thread refused" \
  upd 3 '{"watch_log_offset":100}' --writer conductor-watch
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

# --- 入力の誤り ---
check "empty stdin refused" upd 2 '' --writer-thread mcp:cur
check "non-json patch refused" upd 2 'not json' --writer-thread mcp:cur
check "non-object patch refused" upd 2 '5' --writer-thread mcp:cur
check "list of non-objects refused" upd 2 '[1,2]' --writer-thread mcp:cur
check "missing writer identity refused" upd 2 '{"a":1}'
check "both writer flags refused" upd 2 '{"a":1}' --writer x --writer-thread t
check "claim without writer-thread refused" upd 2 '{"a":1}' --writer conductor-watch --claim-conductor
check "input errors leave file alone" eq "$(pget watch_log_offset)" '600'

# --- 書き込み経路の一元化（Issue #671 受け入れ条件 4） ---
check "conductor-watch writes via shared procedure" \
  has "$REPO/scripts/conductor-watch.sh" "patrol-state-update"
check "conductor-watch has no direct patrol-state dump" \
  bash -c '! grep -q "json.dump(ps, open(ps_path" "$1"' _ "$REPO/scripts/conductor-watch.sh"
check "runbook: shared write procedure and protections documented" \
  has_all "$RUNBOOK" "patrol-state-update" "conductor_thread_id" "anomalies" "巻き戻し" "claim-conductor"

if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}"
  exit 1
fi
printf '\nall passed\n'
