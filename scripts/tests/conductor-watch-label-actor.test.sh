#!/usr/bin/env bash
# scripts/conductor-watch.sh のラベル操作主体検査（Issue #405）のテスト。
# needs-human の unlabeled は hatai だけ、automerge の labeled/unlabeled は
# hatai と pelway-dev[bot] だけが許される。モック agent-gh の api が
# issues/events の固定応答を返し、違反への LABEL bad-actor 行・許可された主体への
# 沈黙・重複報告の無さ・取得失敗の無害さを確かめる。runbook の記述も grep で確かめる。
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUNBOOK="$REPO/docs/agents/grokbot-runbook.md"
ROOT="$(mktemp -d)"
WPID=""
trap '[[ -n "${WPID:-}" ]] && kill "$WPID" 2>/dev/null; rm -rf "$ROOT"' EXIT
failures=()

MOCK="$ROOT/mock"
mkdir -p "$ROOT/repo/scripts" "$ROOT/bin" "$MOCK" "$ROOT/state/pelway"

# 本物の conductor-watch.sh をコピーした偽リポジトリ（scripts/agent-gh をモックに差し替える）
cp "$REPO/scripts/conductor-watch.sh" "$ROOT/repo/scripts/"
cat > "$ROOT/repo/scripts/agent-gh" <<'EOF'
#!/usr/bin/env bash
# モック: issue list / pr list は空を返す。api の issues/events は $MOCK/events.json を返し、
# MOCK_EVENTS_FAIL があるとき失敗する。
if [[ "$2 $3" == "issue list" || "$2 $3" == "pr list" ]]; then echo '[]'; exit 0; fi
if [[ "$2" == "api" && "$3" == "/repos/hatai/Pelway/issues/events?per_page=100" ]]; then
  [[ -n "${MOCK_EVENTS_FAIL:-}" ]] && exit 9
  cat "$MOCK/events.json"; exit 0
fi
exit 9
EOF
chmod +x "$ROOT/repo/scripts/agent-gh"

cat > "$ROOT/bin/t3" <<'EOF'
#!/usr/bin/env bash
[[ "$1 $2" == "thread list" ]] && echo '[]' && exit 0
exit 9
EOF
chmod +x "$ROOT/bin/t3"

cat > "$ROOT/bin/sleep" <<'EOF'
#!/usr/bin/env bash
# 巡回の周期を縮める。SLEEP_MAX 回目の呼び出しで親（conductor-watch.sh）を終了させ、
# 巡回を規定回数で確定させる。
n=$(( $(cat "$MOCK/sleeps" 2>/dev/null || echo 0) + 1 ))
echo "$n" > "$MOCK/sleeps"
if [[ -n "${SLEEP_MAX:-}" ]] && (( n >= SLEEP_MAX )); then
  kill -TERM "$PPID"; exit 0
fi
exec /bin/sleep 0.03
EOF
chmod +x "$ROOT/bin/sleep"

mock_events() { printf '%s\n' "$1" > "$MOCK/events.json"; }

# lev <id> <labeled|unlabeled> <label> <actor> <number> [pr] — issue イベント 1 件の JSON。
# 6 番目に pr を渡すと issue が pull_request を持つ（PR のラベルイベント）。
lev() {
  python3 -I - "$@" <<'EOF'
import json, sys
issue = {"number": int(sys.argv[5])}
if len(sys.argv) > 6:
    issue["pull_request"] = {"url": f"https://api.github.com/repos/hatai/Pelway/pulls/{sys.argv[5]}"}
print(json.dumps({"id": int(sys.argv[1]), "event": sys.argv[2],
                  "actor": {"login": sys.argv[4]}, "label": {"name": sys.argv[3]},
                  "issue": issue, "created_at": "2026-11-17T01:00:00Z"}))
EOF
}

jarr() { local IFS=,; printf '[%s]' "$*"; }

start_watch() {
  MOCK="$MOCK" SLEEP_MAX="${SLEEP_MAX:-}" MOCK_EVENTS_FAIL="${MOCK_EVENTS_FAIL:-}" \
  XDG_STATE_HOME="$ROOT/state" PATH="$ROOT/bin:$PATH" \
    bash "$ROOT/repo/scripts/conductor-watch.sh" >>"$ROOT/out.log" 2>&1 &
  WPID=$!
}

# 巡回 n 回（n 回目の処理まで完了）を待って終了する
run_cycles() {
  echo 0 > "$MOCK/sleeps"
  SLEEP_MAX="$1" start_watch
  wait "$WPID" 2>/dev/null; WPID=""
}

check() {
  local name="$1"; shift
  if "$@"; then printf 'ok   %s\n' "$name"; else printf 'FAIL %s\n' "$name"; failures+=("$name"); fi
}

has() { grep -qF -- "$2" "$1"; }
nhas() { ! grep -qF -- "$2" "$1"; }
eq() { [[ "$1" == "$2" ]]; }
has_all() { local f="$1"; shift; local w; for w in "$@"; do grep -qF -- "$w" "$f" || return 1; done; }
sec_has_all() { local s="$1"; shift; local w; for w in "$@"; do printf '%s\n' "$s" | grep -qF -- "$w" || return 1; done; }

# 1. 初回観測は基準化だけ（過去のイベントを遡って報告しない。#188 の遡及処理は対象外）
mock_events "$(jarr "$(lev 1 unlabeled needs-human pelway-dev[bot] 188)")"
: > "$ROOT/out.log"
run_cycles 2
check "first observation is baseline (no retroactive report)" nhas "$ROOT/out.log" "LABEL bad-actor"

# 2. 旧状態（label_seen 無し）からの更新も基準化だけ（導入時の旧イベントの波を防ぐ）
python3 -I -c 'import json,sys
p = sys.argv[1]; d = json.load(open(p)); d.pop("label_seen", None)
json.dump(d, open(p, "w"))' "$ROOT/state/pelway/conductor-watch/state.json"
: > "$ROOT/out.log"
run_cycles 2
check "upgrade from old state is baseline too" nhas "$ROOT/out.log" "LABEL bad-actor"

# 3. needs-human の解除: hatai 以外なら警告行、hatai なら沈黙
: > "$ROOT/out.log"
mock_events "$(jarr \
  "$(lev 10 unlabeled needs-human pelway-dev[bot] 188)" \
  "$(lev 11 unlabeled needs-human hatai 190)")"
run_cycles 2
check "non-hatai needs-human removal warns" \
  has "$ROOT/out.log" "LABEL bad-actor unlabeled needs-human issue #188 by=pelway-dev[bot]"
check "hatai needs-human removal is silent" nhas "$ROOT/out.log" "#190"

# 4. automerge の付け外し: 許可リスト内は沈黙、外は警告行
: > "$ROOT/out.log"
mock_events "$(jarr \
  "$(lev 20 labeled automerge hatai 388 pr)" \
  "$(lev 21 labeled automerge pelway-dev[bot] 388 pr)" \
  "$(lev 22 unlabeled automerge hatai 390 pr)" \
  "$(lev 23 labeled automerge kodiak-fiend 391 pr)" \
  "$(lev 24 unlabeled automerge stranger 392 pr)")"
run_cycles 2
check "hatai automerge ops are silent" nhas "$ROOT/out.log" "by=hatai"
check "pelway-dev bot automerge op is silent" nhas "$ROOT/out.log" "by=pelway-dev[bot]"
check "unauthorized automerge add warns" \
  has "$ROOT/out.log" "LABEL bad-actor labeled automerge PR #391 by=kodiak-fiend"
check "unauthorized automerge removal warns" \
  has "$ROOT/out.log" "LABEL bad-actor unlabeled automerge PR #392 by=stranger"

# 5. needs-human の付与（labeled）は検査しない
: > "$ROOT/out.log"
mock_events "$(jarr "$(lev 30 labeled needs-human pelway-dev[bot] 191)")"
run_cycles 2
check "labeled needs-human is not checked" nhas "$ROOT/out.log" "LABEL bad-actor"

# 6. 同じイベントは 2 度報告しない
mock_events "$(jarr \
  "$(lev 30 labeled needs-human pelway-dev[bot] 191)" \
  "$(lev 31 unlabeled needs-human rogue 192)")"
: > "$ROOT/out.log"
run_cycles 2
check "new violation in window reported" \
  has "$ROOT/out.log" "LABEL bad-actor unlabeled needs-human issue #192 by=rogue"
check "warning emitted exactly once" eq "$(grep -c 'LABEL bad-actor' "$ROOT/out.log")" "1"
run_cycles 2
check "no duplicate after refetch" eq "$(grep -c 'LABEL bad-actor' "$ROOT/out.log")" "1"

# 7. events の取得失敗は警告無し・クラッシュ無し。回復後は 1 度だけ報告
mock_events "$(jarr "$(lev 40 unlabeled needs-human rogue 193)")"
: > "$ROOT/out.log"
MOCK_EVENTS_FAIL=1 run_cycles 2
check "failed events fetch emits no warning" nhas "$ROOT/out.log" "#193"
check "no traceback on fetch failure" nhas "$ROOT/out.log" "Traceback"
# MOCK_EVENTS_FAIL=x の前置代入は関数実行中だけ有効なので、素の run_cycles で復帰する
run_cycles 2
check "violation during outage reported after recovery" \
  has "$ROOT/out.log" "LABEL bad-actor unlabeled needs-human issue #193 by=rogue"
check "recovered violation reported once" eq "$(grep -c 'LABEL bad-actor' "$ROOT/out.log")" "1"

# --- runbook の記述（Issue #405 の受け入れ条件） ---

check "runbook: permission table limits needs-human removal to hatai" \
  has_all "$RUNBOOK" '`needs-human` を外すのは hatai のみ' '指揮役は Grok Bot 経由で解除を依頼する'
check "runbook: resume procedure waits for hatai's removal" \
  has "$RUNBOOK" 'hatai が `needs-human` を外すのを待つ'
check "runbook: resume procedure no longer removes the label itself" \
  nhas "$RUNBOOK" '`needs-human` を外す（戻り先は使わない）'
outage="$(awk '/^### Claude 不能時の例外/{f=1;next} /^## /{f=0} f' "$RUNBOOK")"
[[ -n "$outage" ]] || failures+=("runbook: 'Claude 不能時の例外' 節が無い")
check "runbook: outage exception allows Codex-only approval for docs-only PRs" \
  sec_has_all "$outage" '文書だけの PR' 'Codex の承認だけ'
check "runbook: outage exception stops code-touching PRs with needs-human" \
  sec_has_all "$outage" 'コード・秘密・外部接続・権限' '`needs-human` を付けたまま'
check "runbook: outage exception requires outage log evidence" \
  sec_has_all "$outage" '障害が始まった時刻' '停滞の証拠'
check "runbook: outage exception records sampling review as not done" \
  sec_has_all "$outage" '「未実施」' '復旧後'
check "runbook: Claude/Devin-mandatory PRs wait for recovery" \
  sec_has_all "$outage" 'Claude か Devin のレビューが必須' '待つ'
check "runbook: LABEL bad-actor detection is documented" \
  has "$RUNBOOK" 'LABEL bad-actor'

if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}"
  exit 1
fi
printf '\nall passed\n'
