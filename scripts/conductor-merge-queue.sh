#!/usr/bin/env bash
# 指揮役のマージの順番待ち（skill command-team の「マージ」）。引数は "PR番号 ブランチ レビュー済みsha" の組を並べる。
# 1 本ずつ、Ready for review を待ち、main を含まなければ取り込み、マージする head に対して次をすべて確かめてから
# --admin でマージする: レビュー済み sha を含む / レビュー後に main 以外のコミットが無い / 今の main を含む /
# 必須チェック（mise run verify・pr-size）が成功（pr-size の失敗は pr-size-exception ラベルと hatai の目印コメントがあるときだけ許す）。
# 途中のどの手順が失敗しても、その PR で止まり（終了コード 1）、後の PR には進まない。
# 使い方: scripts/conductor-merge-queue.sh 128 agent/102-checkpoint-store fff6284... 130 agent/96-rmcp-connection 67aaa70...
set -u -o pipefail
cd "$(dirname "$0")/.." || exit 1

# GitHub との通信は scripts/agent-gh dev 経由（AGENTS.md）。例外は委任された gh pr merge --admin だけ。
gh_dev() { scripts/agent-gh dev "$@" 2>/dev/null; }
fetch() { gh_dev git fetch -q origin; }
remote_head() { git rev-parse --verify -q "origin/$1"; }
run_id() { gh_dev run list --branch "$1" --workflow verify --json headSha,status,databaseId -q ".[]|select(.headSha==\"$2\" and .status==\"completed\")|.databaseId" | head -1; }
job() { gh_dev run view "$1" --json jobs -q ".jobs[]|select(.name==\"$2\")|.conclusion"; }

pr_size_exception() {
  local pr=$1 issue
  gh_dev pr view "$pr" --json labels -q '.labels[].name' | grep -qx pr-size-exception || return 1
  issue=$(gh_dev pr view "$pr" --json closingIssuesReferences -q '.closingIssuesReferences[0].number')
  [ -n "$issue" ] || return 1
  gh_dev issue view "$issue" --json comments -q '.comments[]|select(.author.login=="hatai")|.body' | grep -q '^<!-- pr-size-exception -->'
}

# マージする head に対する条件をまとめて確かめる。0 = マージしてよい、1 = 止める、2 = 待つ（CI 未完了）。
check_head() {
  local pr=$1 br=$2 rsha=$3 H=$4 n rid v s
  git merge-base --is-ancestor "$rsha" "$H" || { echo "#$pr head does not contain the reviewed sha; re-review needed"; return 1; }
  n=$(git rev-list "$rsha..$H" --no-merges ^origin/main | wc -l) || { echo "#$pr cannot compare with reviewed sha"; return 1; }
  [ "$n" = 0 ] || { echo "#$pr new non-main commits ($n) since review; re-review needed"; return 1; }
  git merge-base --is-ancestor origin/main "$H" || { echo "#$pr does not contain current main"; return 1; }
  rid=$(run_id "$br" "$H") || return 2
  [ -n "$rid" ] || return 2
  v=$(job "$rid" "mise run verify"); s=$(job "$rid" "pr-size")
  [ "$v" = success ] || { echo "#$pr mise run verify: ${v:-missing}"; return 1; }
  if [ "$s" != success ]; then
    pr_size_exception "$pr" || { echo "#$pr pr-size: ${s:-missing} (no approved exception)"; return 1; }
    echo "#$pr pr-size failed but has an approved exception"
  fi
  return 0
}

merge_one() {
  local pr=$1 br=$2 rsha=$3 H draft rc
  while :; do
    draft=$(gh_dev pr view "$pr" --json isDraft -q .isDraft) || { echo "#$pr cannot read PR"; return 1; }
    [ "$draft" = false ] && break
    sleep 20
  done
  for attempt in 1 2 3; do
    fetch || { echo "#$pr fetch failed"; return 1; }
    H=$(remote_head "$br") || { echo "#$pr branch $br not found"; return 1; }
    if ! git merge-base --is-ancestor origin/main "$H"; then
      gh_dev pr update-branch "$pr" >/dev/null || { echo "#$pr update-branch failed (conflict?)"; return 1; }
      sleep 20
      continue
    fi
    for _ in $(seq 1 120); do
      check_head "$pr" "$br" "$rsha" "$H"; rc=$?
      [ "$rc" = 2 ] || break
      sleep 30
      fetch || { echo "#$pr fetch failed"; return 1; }
      [ "$(remote_head "$br")" = "$H" ] || { rc=3; break; }
    done
    case "$rc" in
      0) ;;
      1) return 1 ;;
      2) echo "#$pr CI did not finish within 60 minutes"; return 1 ;;
      3) echo "#$pr head moved while waiting; rechecking"; continue ;;
    esac
    if gh pr merge "$pr" --merge --admin --delete-branch --match-head-commit "$H" >/dev/null 2>&1; then
      echo "#$pr merged"; return 0
    fi
    echo "#$pr merge refused (head moved or main changed); rechecking"
  done
  echo "#$pr gave up after retries"
  return 1
}

if [ $# -eq 0 ] || [ $(( $# % 3 )) -ne 0 ]; then
  echo "usage: $0 <PR> <branch> <reviewed-sha> [<PR> <branch> <reviewed-sha> ...]" >&2
  exit 2
fi
while [ $# -ge 3 ]; do
  merge_one "$1" "$2" "$3" || { echo "stopped at #$1; later PRs were not merged"; exit 1; }
  shift 3
done
