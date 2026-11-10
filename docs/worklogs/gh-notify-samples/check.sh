#!/usr/bin/env bash
# notify-grokbot.yml の payload 組立をローカルで確認する（curl は実行しない）。
#
# このディレクトリの見本 payload（github.event 相当・架空値）ごとに、
# workflow の "Send notification" step と同じ env 名・同じ jq の組み立てを
# 実行し、次を確かめる:
#   1. 出力 JSON のキーが指示の 13 個だけであること
#   2. タイトル・本文等のテキスト（*-MUST-NOT-APPEAR マーカー）が混入していないこと
#   3. イベントごとの期待値（null の扱い、run_url の切り替え等）が合っていること
#
# 使い方: bash docs/worklogs/gh-notify-samples/check.sh
set -euo pipefail

samples_dir="$(cd "$(dirname "$0")" && pwd)"

# ランナーの既定 env 相当（架空値。見本 payload と同じリポジトリ）。
export GITHUB_SERVER_URL="https://github.com"
export GITHUB_REPOSITORY="example-org/pelway"
export GITHUB_RUN_ID="123456789"

# webhook payload に許されるキーはこの 13 個だけ。
expected_keys="action,actor,conclusion,event,head_branch,head_sha,merged,occurred_at,pr_number,repo,run_url,schema,workflow_name"

# --- workflow "Send notification" step の jq 部分の逐語コピー（ここから） ---
build_payload() {
  jq -n \
    --arg schema "pelway.github-notify/v1" \
    --arg repo "$REPO" \
    --arg event "$EVENT" \
    --arg action "${ACTION:-}" \
    --argjson pr_number "${PR_NUMBER:-null}" \
    --arg head_branch "${HEAD_BRANCH:-}" \
    --arg head_sha "${HEAD_SHA:-}" \
    --argjson merged "${MERGED:-null}" \
    --arg conclusion "${CONCLUSION:-}" \
    --arg workflow_name "${WORKFLOW_NAME:-}" \
    --arg run_url "${RUN_URL:-$GITHUB_SERVER_URL/$GITHUB_REPOSITORY/actions/runs/$GITHUB_RUN_ID}" \
    --arg actor "${ACTOR:-}" \
    --arg occurred_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    '{
      schema: $schema,
      repo: $repo,
      event: $event,
      action: (if $action == "" then null else $action end),
      pr_number: $pr_number,
      head_branch: (if $head_branch == "" then null else $head_branch end),
      head_sha: (if $head_sha == "" then null else $head_sha end),
      merged: $merged,
      conclusion: (if $conclusion == "" then null else $conclusion end),
      workflow_name: (if $workflow_name == "" then null else $workflow_name end),
      run_url: $run_url,
      actor: (if $actor == "" then null else $actor end),
      occurred_at: $occurred_at
    }'
}
# --- 逐語コピー（ここまで） ---

# workflow の env: 式と同じ対応で github.event payload から env 値を用意する。
#   REPO         <- github.repository                          (= GITHUB_REPOSITORY)
#   EVENT        <- event_name=='workflow_dispatch' ? 'test' : event_name
#   ACTION       <- github.event.action
#   PR_NUMBER    <- pull_request.number || issue.number || workflow_run.pull_requests[0].number
#   HEAD_BRANCH  <- pull_request.head.ref  || workflow_run.head_branch
#   HEAD_SHA     <- pull_request.head.sha  || workflow_run.head_sha
#   MERGED       <- pull_request.merged（無ければ ""、あれば "true"/"false" 文字列）
#   CONCLUSION   <- workflow_run.conclusion
#   WORKFLOW_NAME<- workflow_run.name
#   RUN_URL      <- workflow_run.html_url
#   ACTOR        <- github.actor（= sender.login）
load_env_from_event() {
  local ev="$1" event_name="$2"
  REPO="$GITHUB_REPOSITORY"
  if [ "$event_name" = "workflow_dispatch" ]; then
    EVENT="test"
  else
    EVENT="$event_name"
  fi
  ACTION="$(jq -r '.action // empty' <<<"$ev")"
  PR_NUMBER="$(jq -r '.pull_request.number // .issue.number // .workflow_run.pull_requests[0].number // empty' <<<"$ev")"
  HEAD_BRANCH="$(jq -r '.pull_request.head.ref // .workflow_run.head_branch // empty' <<<"$ev")"
  HEAD_SHA="$(jq -r '.pull_request.head.sha // .workflow_run.head_sha // empty' <<<"$ev")"
  MERGED="$(jq -r 'if .pull_request.merged == null then "" else (.pull_request.merged | tostring) end' <<<"$ev")"
  CONCLUSION="$(jq -r '.workflow_run.conclusion // empty' <<<"$ev")"
  WORKFLOW_NAME="$(jq -r '.workflow_run.name // empty' <<<"$ev")"
  RUN_URL="$(jq -r '.workflow_run.html_url // empty' <<<"$ev")"
  ACTOR="$(jq -r '.sender.login // empty' <<<"$ev")"
  export REPO EVENT ACTION PR_NUMBER HEAD_BRANCH HEAD_SHA MERGED CONCLUSION WORKFLOW_NAME RUN_URL ACTOR
}

run_sample() {
  local file="$1" event_name="$2"
  shift 2
  local ev payload keys kv key want got
  ev="$(cat "$samples_dir/$file")"
  load_env_from_event "$ev" "$event_name"
  payload="$(build_payload)"

  # 1) キーが 13 個だけ
  keys="$(jq -r 'keys | join(",")' <<<"$payload")"
  if [ "$keys" != "$expected_keys" ]; then
    echo "FAIL $file: keys are [$keys]"
    return 1
  fi
  # 2) ペイロード本文（タイトル・本文等）の混入なし
  if grep -q 'MUST-NOT-APPEAR' <<<"$payload"; then
    echo "FAIL $file: sensitive payload text leaked into the notification"
    return 1
  fi
  # 3-1) 共通の固定値
  if ! jq -e '.schema == "pelway.github-notify/v1"' <<<"$payload" >/dev/null; then
    echo "FAIL $file: wrong schema"
    return 1
  fi
  if ! jq -e '.occurred_at | test("^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}Z$")' <<<"$payload" >/dev/null; then
    echo "FAIL $file: occurred_at is not an ISO-8601 UTC timestamp"
    return 1
  fi
  # 3-2) イベントごとの期待値（null も "null" 文字列で比較）
  for kv in "$@"; do
    key="${kv%%=*}"
    want="${kv#*=}"
    got="$(jq -r --arg k "$key" '.[$k] | if . == null then "null" else tostring end' <<<"$payload")"
    if [ "$got" != "$want" ]; then
      echo "FAIL $file: $key is ${got}, want ${want}"
      return 1
    fi
  done
  echo "PASS $file"
}

main() {
  local failures=0
  local own_run_url="https://github.com/example-org/pelway/actions/runs/123456789"

  run_sample pull_request.opened.json pull_request \
    'action=opened' 'pr_number=42' 'head_branch=feature/demo' \
    'head_sha=1111111111111111111111111111111111111111' \
    'merged=false' 'conclusion=null' 'workflow_name=null' \
    "run_url=$own_run_url" 'actor=example-user' || failures=$((failures + 1))
  run_sample pull_request.closed-merged.json pull_request \
    'action=closed' 'pr_number=42' 'head_branch=feature/demo' \
    'head_sha=3333333333333333333333333333333333333333' \
    'merged=true' 'conclusion=null' 'workflow_name=null' \
    "run_url=$own_run_url" 'actor=example-user' || failures=$((failures + 1))
  run_sample pull_request_review.submitted.json pull_request_review \
    'action=submitted' 'pr_number=42' 'head_branch=feature/demo' \
    'head_sha=1111111111111111111111111111111111111111' \
    'merged=false' 'conclusion=null' 'workflow_name=null' \
    "run_url=$own_run_url" 'actor=example-reviewer' || failures=$((failures + 1))
  run_sample pull_request_review_comment.created.json pull_request_review_comment \
    'action=created' 'pr_number=42' 'head_branch=feature/demo' \
    'head_sha=1111111111111111111111111111111111111111' \
    'merged=false' 'conclusion=null' 'workflow_name=null' \
    "run_url=$own_run_url" 'actor=example-commenter' || failures=$((failures + 1))
  run_sample issue_comment.created-pr.json issue_comment \
    'action=created' 'pr_number=42' 'head_branch=null' 'head_sha=null' \
    'merged=null' 'conclusion=null' 'workflow_name=null' \
    "run_url=$own_run_url" 'actor=example-commenter' || failures=$((failures + 1))
  run_sample workflow_run.completed.json workflow_run \
    'action=completed' 'pr_number=42' 'head_branch=feature/demo' \
    'head_sha=4444444444444444444444444444444444444444' \
    'merged=null' 'conclusion=success' 'workflow_name=verify' \
    'run_url=https://github.com/example-org/pelway/actions/runs/9999999999' \
    'actor=example-user' || failures=$((failures + 1))
  run_sample workflow_dispatch.json workflow_dispatch \
    'event=test' 'action=null' 'pr_number=null' 'head_branch=null' 'head_sha=null' \
    'merged=null' 'conclusion=null' 'workflow_name=null' \
    "run_url=$own_run_url" 'actor=example-admin' || failures=$((failures + 1))

  if [ "$failures" -ne 0 ]; then
    echo "$failures sample(s) FAILED"
    exit 1
  fi
  echo "all 7 samples passed"
}

main "$@"
