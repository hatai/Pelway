#!/usr/bin/env bash
# scripts/conductor-watch.sh のラベル操作裏付け検査（Issue #405、2026-11-17 の要件変更）のテスト。
# actor だけでは判定しない（エージェントが hatai のトークンで gh を動かすと actor が hatai に
# なることがある）。needs-human の unlabeled と automerge の labeled/unlabeled について、
# イベント時刻の前 24 時間以内に同じ Issue・PR への hatai 本人のコメントがあるかを突き合わせ、
# 無ければ LABEL unbacked（出どころ不明）行を出す。needs-human の解除では Grok Bot 自身の決定の
# コメント（<!-- grokbot-decision -->）を裏付けに数えない（Issue #602）。モック agent-gh の api が
# issues/events と issues/{n}/comments の固定応答を返す。runbook の記述も grep で確かめる。
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUNBOOK="$REPO/docs/agents/grokbot-runbook.md"
ROOT="$(mktemp -d)"
WPID=""
trap '[[ -n "${WPID:-}" ]] && kill "$WPID" 2>/dev/null; rm -rf "$ROOT"' EXIT
failures=()

MOCK="$ROOT/mock"
mkdir -p "$ROOT/repo/scripts" "$ROOT/bin" "$MOCK" "$ROOT/state/pelway"

# 本物の conductor-watch.sh と patrol-state-update（書き戻しの共通手順）をコピーした偽リポジトリ（scripts/agent-gh をモックに差し替える）
cp "$REPO/scripts/conductor-watch.sh" "$ROOT/repo/scripts/"
cp "$REPO/scripts/patrol-state-update" "$ROOT/repo/scripts/"
cat > "$ROOT/repo/scripts/agent-gh" <<'EOF'
#!/usr/bin/env bash
# モック: issue list / pr list は空を返す。api は issues/events と issues/{n}/comments に
# $MOCK の JSON を返す。MOCK_EVENTS_FAIL / MOCK_COMMENTS_FAIL があるとき失敗する。
if [[ "$2 $3" == "issue list" || "$2 $3" == "pr list" ]]; then echo '[]'; exit 0; fi
if [[ "$2" == "api" ]]; then
  case "$3" in
    "/repos/hatai/Pelway/issues/events?per_page=100")
      [[ -n "${MOCK_EVENTS_FAIL:-}" ]] && exit 9
      cat "$MOCK/events.json"; exit 0 ;;
    /repos/hatai/Pelway/issues/*/comments*)
      n="${3#/repos/hatai/Pelway/issues/}"; n="${n%%/*}"
      [[ -n "${MOCK_COMMENTS_FAIL:-}" ]] && exit 9
      cat "$MOCK/comments-$n.json"; exit 0 ;;
  esac
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
mock_comments() { printf '%s\n' "$2" > "$MOCK/comments-$1.json"; }

# lev <id> <labeled|unlabeled> <label> <actor> <number> <iso時刻> [pr] — issue イベント 1 件の JSON。
# 7 番目に pr を渡すと issue が pull_request を持つ（PR のラベルイベント）。
lev() {
  python3 -I - "$@" <<'EOF'
import json, sys
issue = {"number": int(sys.argv[5])}
if len(sys.argv) > 7 and sys.argv[7] == "pr":
    issue["pull_request"] = {"url": f"https://api.github.com/repos/hatai/Pelway/pulls/{sys.argv[5]}"}
print(json.dumps({"id": int(sys.argv[1]), "event": sys.argv[2],
                  "actor": {"login": sys.argv[4]}, "label": {"name": sys.argv[3]},
                  "issue": issue, "created_at": sys.argv[6]}))
EOF
}

# lc <author> <iso時刻> [本文] — issue コメント 1 件の JSON
lc() {
  python3 -I - "$@" <<'EOF'
import json, sys
print(json.dumps({"user": {"login": sys.argv[1]}, "created_at": sys.argv[2], **({"body": sys.argv[3]} if len(sys.argv) > 3 else {})}))
EOF
}

jarr() { local out="" x; for x in "$@"; do out+="${out:+,}$x"; done; printf '[%s]' "$out"; }

start_watch() {
  MOCK="$MOCK" SLEEP_MAX="${SLEEP_MAX:-}" MOCK_EVENTS_FAIL="${MOCK_EVENTS_FAIL:-}" \
  MOCK_COMMENTS_FAIL="${MOCK_COMMENTS_FAIL:-}" \
  XDG_STATE_HOME="$ROOT/state" PATH="$ROOT/bin:$PATH" \
  PELWAY_PATROL_STATE="$ROOT/state/pelway/patrol-state.json" \
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

T0="2026-11-17T01:00:00Z"       # イベントの時刻
T1H="2026-11-17T00:00:00Z"      # 1 時間前（裏付けになる窓の中）
T48H="2026-11-15T01:00:00Z"     # 48 時間前（窓の外）
TAFT="2026-11-17T02:00:00Z"     # イベントの後（裏付けにならない）

# 1. 初回観測は基準化だけ（過去のイベントを遡って報告しない。#188 の遡及処理は対象外）
mock_events "$(jarr "$(lev 1 unlabeled needs-human pelway-dev[bot] 188 "$T0")")"
: > "$ROOT/out.log"
run_cycles 2
check "first observation is baseline (no retroactive report)" nhas "$ROOT/out.log" "LABEL unbacked"

# 2. 旧状態（label_seen 無し）からの更新も基準化だけ（導入時の旧イベントの波を防ぐ）
python3 -I -c 'import json,sys
p = sys.argv[1]; d = json.load(open(p)); d.pop("label_seen", None)
json.dump(d, open(p, "w"))' "$ROOT/state/pelway/conductor-watch/state.json"
: > "$ROOT/out.log"
run_cycles 2
check "upgrade from old state is baseline too" nhas "$ROOT/out.log" "LABEL unbacked"

# 3. needs-human の解除: 前後 24 時間以内に hatai 本人のコメントがあれば沈黙、無ければ警告行
: > "$ROOT/out.log"
mock_comments 188 "[]"
mock_comments 190 "$(jarr "$(lc hatai "$T1H")")"
mock_events "$(jarr \
  "$(lev 10 unlabeled needs-human pelway-dev[bot] 188 "$T0")" \
  "$(lev 11 unlabeled needs-human hatai 190 "$T0")")"
run_cycles 2
check "needs-human removal without hatai comment warns as unbacked" \
  has "$ROOT/out.log" "LABEL unbacked unlabeled needs-human issue #188 at=$T0 by=pelway-dev[bot]"
check "needs-human removal with hatai comment is silent" nhas "$ROOT/out.log" "#190"

# 4. 窓の外・イベントの後の hatai コメントは裏付けにならない
: > "$ROOT/out.log"
mock_comments 194 "$(jarr "$(lc hatai "$T48H")")"
mock_comments 195 "$(jarr "$(lc hatai "$TAFT")")"
mock_events "$(jarr \
  "$(lev 20 unlabeled needs-human pelway-dev[bot] 194 "$T0")" \
  "$(lev 21 unlabeled needs-human pelway-dev[bot] 195 "$T0")")"
run_cycles 2
check "comment older than window does not back" has "$ROOT/out.log" "issue #194 at=$T0"
check "comment after the event does not back" has "$ROOT/out.log" "issue #195 at=$T0"

# 5. automerge の付け外しも同じ突き合わせ。裏付けがあれば actor が誰でも沈黙
: > "$ROOT/out.log"
mock_comments 391 "[]"
mock_comments 392 "$(jarr "$(lc hatai "$T1H")")"
mock_comments 393 "$(jarr "$(lc hatai "$T1H")")"
mock_events "$(jarr \
  "$(lev 30 labeled automerge kodiak-fiend 391 "$T0" pr)" \
  "$(lev 31 labeled automerge stranger 392 "$T0" pr)" \
  "$(lev 32 unlabeled automerge stranger 393 "$T0" pr)")"
run_cycles 2
check "automerge add without backing warns" \
  has "$ROOT/out.log" "LABEL unbacked labeled automerge PR #391 at=$T0 by=kodiak-fiend"
check "automerge add with hatai comment is silent" nhas "$ROOT/out.log" "#392"
check "automerge removal with hatai comment is silent" nhas "$ROOT/out.log" "#393"

# 6. needs-human の付与（labeled）は検査しない（エスカレーションでの付与は日常的）
: > "$ROOT/out.log"
mock_events "$(jarr "$(lev 40 labeled needs-human pelway-dev[bot] 191 "$T0")")"
run_cycles 2
check "labeled needs-human is not checked" nhas "$ROOT/out.log" "LABEL unbacked"

# 7. 同じイベントは 2 度報告しない
: > "$ROOT/out.log"
mock_comments 192 "[]"
mock_events "$(jarr "$(lev 41 unlabeled needs-human rogue 192 "$T0")")"
run_cycles 2
check "new violation in window reported" \
  has "$ROOT/out.log" "LABEL unbacked unlabeled needs-human issue #192 at=$T0 by=rogue"
check "warning emitted exactly once" eq "$(grep -c 'LABEL unbacked' "$ROOT/out.log")" "1"
run_cycles 2
check "no duplicate after refetch" eq "$(grep -c 'LABEL unbacked' "$ROOT/out.log")" "1"

# 8. comments の取得失敗は判定を翌巡回に延期（誤報しない）。回復後は 1 度だけ報告
mock_comments 193 "[]"
mock_events "$(jarr "$(lev 50 unlabeled needs-human rogue 193 "$T0")")"
: > "$ROOT/out.log"
MOCK_COMMENTS_FAIL=1 run_cycles 2
check "failed comments fetch defers judgment" nhas "$ROOT/out.log" "#193"
check "no traceback on comments fetch failure" nhas "$ROOT/out.log" "Traceback"
# MOCK_COMMENTS_FAIL=x の前置代入は関数実行中だけ有効なので、素の run_cycles で復帰する
run_cycles 2
check "deferred violation reported after recovery" \
  has "$ROOT/out.log" "LABEL unbacked unlabeled needs-human issue #193 at=$T0 by=rogue"
check "recovered violation reported once" eq "$(grep -c 'LABEL unbacked' "$ROOT/out.log")" "1"

# 9. events の取得失敗は警告無し・クラッシュ無し
mock_events "$(jarr "$(lev 60 unlabeled needs-human rogue 196 "$T0")")"
: > "$ROOT/out.log"
MOCK_EVENTS_FAIL=1 run_cycles 2
check "failed events fetch emits no warning" nhas "$ROOT/out.log" "#196"
check "no traceback on events fetch failure" nhas "$ROOT/out.log" "Traceback"

# 10. needs-human の解除では、hatai の名義でも Grok Bot 自身の決定のコメント（<!-- grokbot-decision -->）を裏付けに数えない。
# 人間の判断の記録（<!-- grokbot-human-decision -->）と目印の無い hatai のコメントは数える。automerge は目印を見ない。
DEC=$'<!-- grokbot-decision -->\n決定: 範囲を縮めて続ける'
HUM=$'<!-- grokbot-human-decision -->\nhatai「続けてよい」（09:50 JST）'
mock_comments 501 "$(jarr "$(lc hatai "$T1H" "$DEC")" "$(lc hatai "$T1H" "$DEC")")"
mock_comments 502 "$(jarr "$(lc hatai "$T1H" "$HUM")")"
mock_comments 503 "$(jarr "$(lc hatai "$T1H" "$DEC")" "$(lc hatai "$T1H" "外してよい")")"
mock_comments 504 "$(jarr "$(lc hatai "$T1H" "$DEC")")"
mock_events "$(jarr \
  "$(lev 70 unlabeled needs-human hatai 501 "$T0")" \
  "$(lev 71 unlabeled needs-human hatai 502 "$T0")" \
  "$(lev 72 unlabeled needs-human hatai 503 "$T0")" \
  "$(lev 73 labeled automerge hatai 504 "$T0" pr)")"
: > "$ROOT/out.log"
run_cycles 2
check "needs-human removal backed only by grokbot-decision comments is unbacked (exact line)" \
  eq "$(grep '^LABEL ' "$ROOT/out.log")" "LABEL unbacked unlabeled needs-human issue #501 at=$T0 by=hatai"
check "grokbot-human-decision comment backs needs-human removal" nhas "$ROOT/out.log" "#502"
check "unmarked hatai comment still backs needs-human removal" nhas "$ROOT/out.log" "#503"
check "automerge backing still counts grokbot-decision comments" nhas "$ROOT/out.log" "#504"

# --- runbook の記述（Issue #405 の受け入れ条件と 2026-11-17 の要件変更） ---

check "runbook: permission table limits needs-human removal to hatai" \
  has_all "$RUNBOOK" '`needs-human` を外すのは hatai のみ' '指揮役は Grok Bot 経由で解除を依頼する'
check "runbook: resume procedure waits for hatai's removal" \
  has "$RUNBOOK" 'hatai が `needs-human` を外すのを待つ'
check "runbook: resume procedure no longer removes the label itself" \
  nhas "$RUNBOOK" '`needs-human` を外す（戻り先は使わない）'
check "runbook: cross-check does not trust actor alone" \
  has "$RUNBOOK" 'actor だけでは判定しない'
check "runbook: unbacked label ops are reported" \
  has_all "$RUNBOOK" 'LABEL unbacked' '出どころ不明'
check "runbook: patrol cross-checks conductor and T3 records" \
  has "$RUNBOOK" '指揮役と T3 スレッドの操作記録'
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

if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}"
  exit 1
fi
printf '\nall passed\n'
