#!/usr/bin/env bash
# .agents/skills/pr-watch/pr-digest のテスト。
# agent-gh をモックに置き換え、固定 JSON への応答でダイジェストの出力と終了コードを確かめる。
# 一時ディレクトリ（git リポジトリの外）で動かすので、手元 checkout との同期検査は常にスキップされる。
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DIGEST="$HERE/pr-digest"
failures=()
ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT

# ---- agent-gh のモック -----------------------------------------------------
# `dev <gh の引数...>` を受け、$PR_DIGEST_FIXTURE のファイルから応答する。
# --jq が付いた呼び出しは、その式をモック内で jq に通してから返す（gh と同じ扱い）。
# `pr view` と `check-runs` は呼び出し回数を数え、<名前>-<N>.json があれば N 回目の
# 応答として使う（ポーリングのシーケンスと head の移動の再現用）。
mkdir -p "$ROOT/bin"
cat >"$ROOT/bin/agent-gh" <<'MOCK'
#!/usr/bin/env bash
set -uo pipefail
[ "${1:-}" = dev ] || { echo "mock: role must be dev" >&2; exit 2; }
shift
fix="${PR_DIGEST_FIXTURE:?}"
all="$*"

jq_expr=""
prev=""
for a in "$@"; do
  [ "$prev" = "--jq" ] && jq_expr="$a"
  prev="$a"
done

emit() {
  if [ -n "$jq_expr" ]; then jq -r "$jq_expr" <"$1" || exit 1
  else cat "$1" || exit 1; fi
}
nth() { # nth <ベース名>: N 回目のファイル。無ければ番号の近いもの、最後はベース名を繰り返す
  local base="$1" n f i
  touch "$fix/.$base"
  echo x >>"$fix/.$base"
  n=$(wc -l <"$fix/.$base")
  f="$fix/$base.json"
  for ((i = n; i >= 1; i--)); do
    if [ -f "$fix/$base-$i.json" ]; then f="$fix/$base-$i.json"; break; fi
  done
  printf '%s' "$f"
}

case "$all" in
  "repo view"*) printf 'hatai/Pelway\n'; exit 0 ;;
  "api graphql"*)
    printf '{"data":{"viewer":{"login":"pelway-dev[bot]"}}}\n' >"$fix/.viewer.json"
    emit "$fix/.viewer.json"; exit 0 ;;
esac

if [[ "$all" == "pr view"* ]]; then
  if [[ "$all" == *"--json number"* ]]; then printf '217\n'; exit 0; fi
  emit "$(nth pr-view)"; exit 0
fi
if [[ "$all" == "run view"* ]]; then
  cat "$fix/log.txt" 2>/dev/null || exit 1
  exit 0
fi
if [[ "$all" == api* ]]; then
  if [[ "$all" == *"/replies"* ]]; then
    printf '%s\n' "$all" >>"${REPLY_CALLS:?}"
    printf '{"id":999}\n' >"$fix/.reply.json"
    emit "$fix/.reply.json"; exit 0
  fi
  if [[ "$all" == *"pulls/comments/"* ]]; then
    printf '{"pull_request_url":"https://api.github.com/repos/hatai/Pelway/pulls/217"}\n' >"$fix/.prurl.json"
    emit "$fix/.prurl.json"; exit 0
  fi
  if [[ "$all" == *"check-runs"* ]]; then
    emit "$(nth check-runs)"; exit 0
  fi
  case "$all" in
    *"/status"*)             emit "$fix/status.json"; exit 0 ;;
    *"pulls/"*"/comments"*)  emit "$fix/inline.json"; exit 0 ;;
    *"issues/"*"/comments"*) emit "$fix/conversation.json"; exit 0 ;;
    *"issues/"*"/timeline"*) emit "$fix/timeline.json"; exit 0 ;;
    *"pulls/"*"/reviews"*)   emit "$fix/reviews.json"; exit 0 ;;
    *"actions/jobs/"*)       emit "$fix/jobs.json"; exit 0 ;;
  esac
  echo "mock: unhandled api call: $all" >&2; exit 2
fi
echo "mock: unhandled: $all" >&2; exit 2
MOCK
chmod +x "$ROOT/bin/agent-gh"

# ---- テストの部品 -----------------------------------------------------------

new_fixture() { # new_fixture <name> -> 名前を出力し、基本ファイル（空コメント・PR メタ）を置く
  local name="$1" fix="$ROOT/fx/$1"
  mkdir -p "$fix"
  cat >"$fix/pr-view.json" <<'EOF'
{"headRefOid":"abc123def","state":"OPEN","mergeable":"MERGEABLE","headRefName":"agent/217-example","reviewDecision":"APPROVED","createdAt":"2026-01-02T00:00:00Z","isDraft":false}
EOF
  echo '[]' >"$fix/inline.json"
  echo '[]' >"$fix/conversation.json"
  echo '[]' >"$fix/reviews.json"
  echo '[]' >"$fix/timeline.json"
  echo "$fix"
}

check_run() { # check_run <name> <status> <conclusion> [url-suffix]
  printf '{"name":"%s","status":"%s","conclusion":"%s","html_url":"https://github.com/hatai/Pelway/actions/runs/1/job/%s"}\n' \
    "$1" "$2" "$3" "${4:-10}"
}

run_digest() { # run_digest <fixture> [pr-digest の引数...] -> 出力は $out、終了コードは $rc
  local fix="$1"; shift
  out="$(cd "$ROOT" && \
    PR_DIGEST_GH="bash $ROOT/bin/agent-gh dev" \
    PR_DIGEST_FIXTURE="$fix" \
    PR_DIGEST_SEEN_DIR="$fix/seen" \
    PR_DIGEST_POLL=1 PR_DIGEST_DEADLINE=10 \
    bash "$DIGEST" "$@" 2>&1)"
  rc=$?
}

expect_rc() { # expect_rc <name> <want-rc>
  [ "$rc" = "$2" ] || failures+=("$1: exit $rc, want $2")
}
expect_has() { # expect_has <name> <substring>
  grep -qF -- "$2" <<<"$out" || failures+=("$1: output missing '$2'")
}
expect_lacks() { # expect_lacks <name> <substring>
  ! grep -qF -- "$2" <<<"$out" || failures+=("$1: output should not contain '$2'")
}

# ---- 1. green: 全チェック完走・成功、未回答の指摘なし ------------------------

fix="$(new_fixture green)"
{ check_run "mise run verify" completed success 10
  check_run "static (hk / fnox / pitchfork)" completed success 11
} | jq -s '{check_runs: .}' >"$fix/check-runs.json"
echo '{"statuses":[{"context":"pr-size","state":"success","description":"+12 / -0"}]}' >"$fix/status.json"
run_digest "$fix"
expect_rc green 0
expect_has green "VERDICT green"
expect_has green "checks 2 success"
expect_lacks green "STATUS"

# ---- 2. failures: check run の失敗に失敗ステップを添える ----------------------

fix="$(new_fixture failures)"
{ check_run "mise run verify" completed failure 10; } | jq -s '{check_runs: .}' >"$fix/check-runs.json"
echo '{"statuses":[]}' >"$fix/status.json"
cat >"$fix/jobs.json" <<'EOF'
{"steps":[{"number":1,"name":"Setup","conclusion":"success"},{"number":2,"name":"Verify (mise run verify)","conclusion":"failure"},{"number":3,"name":"Report","conclusion":"skipped"}]}
EOF
run_digest "$fix"
expect_rc failures 10
expect_has failures "FAIL mise run verify [failure]"
expect_has failures 'step "Verify (mise run verify)" failure'
expect_has failures "(later steps skipped)"
expect_has failures "VERDICT failures"

# ---- 3. status_failure: 旧来の commit status の失敗も verdict に届ぶ ----------

fix="$(new_fixture status_failure)"
{ check_run "static (hk / fnox / pitchfork)" completed success 11; } | jq -s '{check_runs: .}' >"$fix/check-runs.json"
echo '{"statuses":[{"context":"pr-size","state":"failure","description":"+900 / -0"}]}' >"$fix/status.json"
run_digest "$fix"
expect_rc status_failure 10
expect_has status_failure "STATUS pr-size failure"
expect_has status_failure "VERDICT failures"

# ---- 4. pending: まだ走っているチェック --------------------------------------

fix="$(new_fixture pending)"
cat >"$fix/check-runs.json" <<'EOF'
{"check_runs":[{"name":"mise run verify","status":"in_progress","conclusion":null,
  "html_url":"https://github.com/hatai/Pelway/actions/runs/1/job/10"}]}
EOF
echo '{"statuses":[]}' >"$fix/status.json"
run_digest "$fix"
expect_rc pending 12
expect_has pending "checks 1 pending"
expect_has pending "VERDICT pending"

# ---- 5. open_inline: 未返信のインラインレビューコメント ------------------------

fix="$(new_fixture open_inline)"
{ check_run "mise run verify" completed success 10; } | jq -s '{check_runs: .}' >"$fix/check-runs.json"
echo '{"statuses":[]}' >"$fix/status.json"
cat >"$fix/inline.json" <<'EOF'
[{"id":501,"in_reply_to_id":null,"user":{"login":"pelway-review[bot]","type":"Bot"},
  "path":"src/lib.rs","line":42,"created_at":"2026-01-02T00:00:00Z","updated_at":"2026-01-02T00:00:00Z",
  "body":"must-fix: テストが失敗している"}]
EOF
run_digest "$fix"
expect_rc open_inline 11
expect_has open_inline "[501] pelway-review[bot] src/lib.rs:42"
expect_has open_inline "must-fix: テストが失敗している"
expect_has open_inline "VERDICT open-comments"

# ---- 6. replied_not_open: 自分の返信済みなら open に数えない -------------------

fix="$(new_fixture replied_not_open)"
{ check_run "mise run verify" completed success 10; } | jq -s '{check_runs: .}' >"$fix/check-runs.json"
echo '{"statuses":[]}' >"$fix/status.json"
cat >"$fix/inline.json" <<'EOF'
[{"id":501,"in_reply_to_id":null,"user":{"login":"pelway-review[bot]","type":"Bot"},
  "path":"src/lib.rs","line":42,"created_at":"2026-01-02T00:00:00Z","updated_at":"2026-01-02T00:00:00Z",
  "body":"must-fix: テストが失敗している"},
 {"id":502,"in_reply_to_id":501,"user":{"login":"pelway-dev[bot]","type":"Bot"},
  "path":"src/lib.rs","line":42,"created_at":"2026-01-03T00:00:00Z","updated_at":"2026-01-03T00:00:00Z",
  "body":"修正して push しました"}]
EOF
run_digest "$fix"
expect_rc replied_not_open 0
expect_has replied_not_open "VERDICT green"

# ---- 7. open_conversation: 会話コメントは permalink を引用した返信で処理済みになる

fix="$(new_fixture open_conversation)"
{ check_run "mise run verify" completed success 10; } | jq -s '{check_runs: .}' >"$fix/check-runs.json"
echo '{"statuses":[]}' >"$fix/status.json"
cat >"$fix/conversation.json" <<'EOF'
[{"id":601,"user":{"login":"hatai","type":"User"},
  "created_at":"2026-01-02T00:00:00Z","updated_at":"2026-01-02T00:00:00Z",
  "html_url":"https://github.com/hatai/Pelway/pull/217#issuecomment-601",
  "body":"<!-- pr-size-exception --> 900 行超過を認める"}]
EOF
run_digest "$fix"
expect_rc open_conversation 11
expect_has open_conversation "(conversation)"
expect_has open_conversation "VERDICT open-comments"

# conversation_permalink: 自分が後から permalink を引用していれば open に数えない
fix="$(new_fixture conversation_permalink)"
{ check_run "mise run verify" completed success 10; } | jq -s '{check_runs: .}' >"$fix/check-runs.json"
echo '{"statuses":[]}' >"$fix/status.json"
cat >"$fix/conversation.json" <<'EOF'
[{"id":601,"user":{"login":"hatai","type":"User"},
  "created_at":"2026-01-02T00:00:00Z","updated_at":"2026-01-02T00:00:00Z",
  "html_url":"https://github.com/hatai/Pelway/pull/217#issuecomment-601",
  "body":"確認をよろしく"},
 {"id":602,"user":{"login":"pelway-dev[bot]","type":"Bot"},
  "created_at":"2026-01-03T00:00:00Z","updated_at":"2026-01-03T00:00:00Z",
  "html_url":"https://github.com/hatai/Pelway/pull/217#issuecomment-602",
  "body":"対応しました https://github.com/hatai/Pelway/pull/217#issuecomment-601"}]
EOF
run_digest "$fix"
expect_rc conversation_permalink 0
expect_has conversation_permalink "VERDICT green"

# ---- 8. open_review_summary: 行コメントなしの Request changes も読む ----------

fix="$(new_fixture open_review_summary)"
{ check_run "mise run verify" completed success 10; } | jq -s '{check_runs: .}' >"$fix/check-runs.json"
echo '{"statuses":[]}' >"$fix/status.json"
cat >"$fix/pr-view.json" <<'EOF'
{"headRefOid":"abc123def","state":"OPEN","mergeable":"MERGEABLE","headRefName":"agent/217-example","reviewDecision":"CHANGES_REQUESTED","createdAt":"2026-01-02T00:00:00Z","isDraft":false}
EOF
cat >"$fix/reviews.json" <<'EOF'
[{"id":701,"user":{"login":"pelway-review[bot]","type":"Bot"},"state":"CHANGES_REQUESTED",
  "submitted_at":"2026-01-02T00:00:00Z","body":"must-fix が 2 件ある"}]
EOF
run_digest "$fix"
expect_rc open_review_summary 11
expect_has open_review_summary "REVIEW changes requested"
expect_has open_review_summary "(review CHANGES_REQUESTED)"
expect_has open_review_summary "must-fix が 2 件ある"
expect_has open_review_summary "VERDICT open-comments"

# ---- 9. closed: 閉じた PR はそれ以上読まない ---------------------------------

fix="$(new_fixture closed)"
cat >"$fix/pr-view.json" <<'EOF'
{"headRefOid":"abc123def","state":"MERGED","mergeable":"MERGEABLE","headRefName":"agent/217-example","reviewDecision":"APPROVED"}
EOF
run_digest "$fix"
expect_rc closed 0
expect_has closed "VERDICT MERGED"

# ---- 10. head_moved: 観察中に head が動いたら out-of-sync ---------------------

fix="$(new_fixture head_moved)"
{ check_run "mise run verify" completed success 10; } | jq -s '{check_runs: .}' >"$fix/check-runs.json"
echo '{"statuses":[]}' >"$fix/status.json"
echo '{"headRefOid":"fff999"}' >"$fix/pr-view-2.json"
run_digest "$fix"
expect_rc head_moved 20
expect_has head_moved "VERDICT out-of-sync (head moved during this observation)"

# ---- 11. api_error: API の失敗は決して green に見えない ----------------------

fix="$(new_fixture api_error)"
rm "$fix/pr-view.json"  # どの応答も取れない
run_digest "$fix"
expect_rc api_error 1

# ---- 12. reply: 返信は replies エンドポイントに投稿される --------------------

fix="$(new_fixture reply)"
export REPLY_CALLS="$fix/reply-calls"
: >"$REPLY_CALLS"
run_digest "$fix" --reply 501 "修正して push しました"
expect_rc reply 0
expect_has reply "replied 999"
grep -qF 'pulls/217/comments/501/replies' "$REPLY_CALLS" \
  || failures+=("reply: POST went to the wrong endpoint: $(cat "$REPLY_CALLS")")
grep -qF 'body=修正して push しました' "$REPLY_CALLS" \
  || failures+=("reply: body missing in POST: $(cat "$REPLY_CALLS")")
unset REPLY_CALLS

# ---- 13. watch_settles: --watch は全チェック terminal なら 1 ポーリングで抜ける

fix="$(new_fixture watch_settles)"
{ check_run "mise run verify" completed success 10; } | jq -s '{check_runs: .}' >"$fix/check-runs.json"
echo '{"statuses":[]}' >"$fix/status.json"
run_digest "$fix" --watch
expect_rc watch_settles 0
expect_has watch_settles "VERDICT green"

# ---- 14. watch_wakes: --watch は走行中→新失敗の変化で起こす -------------------

fix="$(new_fixture watch_wakes)"
cat >"$fix/check-runs-1.json" <<'EOF'
{"check_runs":[{"name":"mise run verify","status":"in_progress","conclusion":null,
  "html_url":"https://github.com/hatai/Pelway/actions/runs/1/job/10"}]}
EOF
{ check_run "mise run verify" completed failure 10; } | jq -s '{check_runs: .}' >"$fix/check-runs-2.json"
echo '{"statuses":[]}' >"$fix/status.json"
run_digest "$fix" --watch
expect_rc watch_wakes 10
expect_has watch_wakes "VERDICT failures"

# ---- 15. review_wait_created: Draft でなかった PR は作成時刻が起点 --------------
# review-wait の経過は PR_DIGEST_NOW で現在時刻を固定して確かめる。

fix="$(new_fixture review_wait_created)"
{ check_run "mise run verify" completed success 10; } | jq -s '{check_runs: .}' >"$fix/check-runs.json"
echo '{"statuses":[]}' >"$fix/status.json"
cat >"$fix/pr-view.json" <<'EOF'
{"headRefOid":"abc123def","state":"OPEN","mergeable":"MERGEABLE","headRefName":"agent/217-example","reviewDecision":"","createdAt":"2026-01-01T00:00:00Z","isDraft":false}
EOF
echo '[]' >"$fix/timeline.json"
PR_DIGEST_NOW="2026-01-02T06:00:00Z"; export PR_DIGEST_NOW
run_digest "$fix"
unset PR_DIGEST_NOW
expect_rc review_wait_created 0
expect_has review_wait_created "review-wait 30h (since 2026-01-01T00:00:00Z)"

# ---- 16. review_wait_ready: Draft だった PR は最初の ready_for_review が起点 ---

fix="$(new_fixture review_wait_ready)"
{ check_run "mise run verify" completed success 10; } | jq -s '{check_runs: .}' >"$fix/check-runs.json"
echo '{"statuses":[]}' >"$fix/status.json"
cat >"$fix/pr-view.json" <<'EOF'
{"headRefOid":"abc123def","state":"OPEN","mergeable":"MERGEABLE","headRefName":"agent/217-example","reviewDecision":"","createdAt":"2026-01-01T00:00:00Z","isDraft":false}
EOF
cat >"$fix/timeline.json" <<'EOF'
[{"event":"review_requested","created_at":"2026-01-01T00:00:00Z"},
 {"event":"ready_for_review","created_at":"2026-01-02T02:00:00Z"}]
EOF
PR_DIGEST_NOW="2026-01-03T00:00:00Z"; export PR_DIGEST_NOW
run_digest "$fix"
unset PR_DIGEST_NOW
expect_rc review_wait_ready 0
expect_has review_wait_ready "review-wait 22h (since 2026-01-02T02:00:00Z)"

# ---- 17. review_wait_sticks: 再割当・再 Draft でも起点は最初の ready ----------

fix="$(new_fixture review_wait_sticks)"
{ check_run "mise run verify" completed success 10; } | jq -s '{check_runs: .}' >"$fix/check-runs.json"
echo '{"statuses":[]}' >"$fix/status.json"
cat >"$fix/pr-view.json" <<'EOF'
{"headRefOid":"abc123def","state":"OPEN","mergeable":"MERGEABLE","headRefName":"agent/217-example","reviewDecision":"","createdAt":"2026-01-01T00:00:00Z","isDraft":false}
EOF
cat >"$fix/timeline.json" <<'EOF'
[{"event":"ready_for_review","created_at":"2026-01-02T02:00:00Z"},
 {"event":"ready_for_review","created_at":"2026-01-02T10:00:00Z"},
 {"event":"review_requested","created_at":"2026-01-02T22:00:00Z"}]
EOF
PR_DIGEST_NOW="2026-01-03T00:00:00Z"; export PR_DIGEST_NOW
run_digest "$fix"
unset PR_DIGEST_NOW
expect_rc review_wait_sticks 0
expect_has review_wait_sticks "review-wait 22h (since 2026-01-02T02:00:00Z)"

# ---- 18. review_wait_draft: 今も Draft の PR は経過を出さない ------------------

fix="$(new_fixture review_wait_draft)"
{ check_run "mise run verify" completed success 10; } | jq -s '{check_runs: .}' >"$fix/check-runs.json"
echo '{"statuses":[]}' >"$fix/status.json"
cat >"$fix/pr-view.json" <<'EOF'
{"headRefOid":"abc123def","state":"OPEN","mergeable":"MERGEABLE","headRefName":"agent/217-example","reviewDecision":"","createdAt":"2026-01-01T00:00:00Z","isDraft":true}
EOF
echo '[]' >"$fix/timeline.json"
run_digest "$fix"
expect_rc review_wait_draft 0
expect_lacks review_wait_draft "review-wait"

# ---- 19. review_wait_minutes: 1 時間未満は分で出す -----------------------------

fix="$(new_fixture review_wait_minutes)"
{ check_run "mise run verify" completed success 10; } | jq -s '{check_runs: .}' >"$fix/check-runs.json"
echo '{"statuses":[]}' >"$fix/status.json"
cat >"$fix/pr-view.json" <<'EOF'
{"headRefOid":"abc123def","state":"OPEN","mergeable":"MERGEABLE","headRefName":"agent/217-example","reviewDecision":"","createdAt":"2026-01-02T23:30:00Z","isDraft":false}
EOF
echo '[]' >"$fix/timeline.json"
PR_DIGEST_NOW="2026-01-03T00:00:00Z"; export PR_DIGEST_NOW
run_digest "$fix"
unset PR_DIGEST_NOW
expect_rc review_wait_minutes 0
expect_has review_wait_minutes "review-wait 30m (since 2026-01-02T23:30:00Z)"

# ---- 結果 -------------------------------------------------------------------

if [ "${#failures[@]}" -gt 0 ]; then
  printf 'pr-digest.test: %d failure(s)\n' "${#failures[@]}"
  printf '  - %s\n' "${failures[@]}"
  exit 1
fi
echo "pr-digest.test: ok"
