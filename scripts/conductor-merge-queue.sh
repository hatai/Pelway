#!/usr/bin/env bash
# 指揮役のマージの順番待ち（skill command-team の「マージ」）。引数は "PR番号 ブランチ レビュー済みsha" の組を並べる。
# 1 本ずつ、Ready for review を待ち、main を含まなければ取り込んで CI を待ち、レビュー後に main 以外のコミットが無ければ --admin でマージする。
# 途中のどの手順が失敗しても、その PR で止まり（終了コード 1）、後の PR には進まない。
# 使い方: scripts/conductor-merge-queue.sh 128 agent/102-checkpoint-store fff6284... 130 agent/96-rmcp-connection 67aaa70...
set -u -o pipefail
cd "$(dirname "$0")/.." || exit 1

# GitHub との通信は scripts/agent-gh dev 経由（AGENTS.md）。例外は委任された gh pr merge --admin だけ。
fetch() { scripts/agent-gh dev git fetch -q origin 2>/dev/null; }
remote_head() { git rev-parse --verify -q "origin/$1"; }
verify_status() { scripts/agent-gh dev run list --branch "$1" --workflow verify --json headSha,status,conclusion -q ".[]|select(.headSha==\"$2\")|\"\(.status) \(.conclusion)\"" 2>/dev/null | head -1; }

merge_one() {
  local pr=$1 br=$2 rsha=$3 H n s draft
  while :; do
    draft=$(scripts/agent-gh dev pr view "$pr" --json isDraft -q .isDraft 2>/dev/null) || { echo "#$pr cannot read PR"; return 1; }
    [ "$draft" = false ] && break
    sleep 20
  done
  for attempt in 1 2 3; do
    fetch || { echo "#$pr fetch failed"; return 1; }
    H=$(remote_head "$br") || { echo "#$pr branch $br not found"; return 1; }
    n=$(git rev-list "$rsha..$H" --no-merges ^origin/main | wc -l) || { echo "#$pr cannot compare with reviewed sha"; return 1; }
    [ "$n" = 0 ] || { echo "#$pr new non-main commits ($n) since review; re-review needed"; return 1; }
    if ! git merge-base --is-ancestor origin/main "$H"; then
      scripts/agent-gh dev pr update-branch "$pr" >/dev/null 2>&1 || { echo "#$pr update-branch failed (conflict?)"; return 1; }
      sleep 20
      fetch || { echo "#$pr fetch failed"; return 1; }
      H=$(remote_head "$br") || { echo "#$pr branch $br not found"; return 1; }
    fi
    s=""
    for _ in $(seq 1 120); do
      s=$(verify_status "$br" "$H")
      case "$s" in completed*) break ;; esac
      sleep 30
    done
    case "$s" in completed*) ;; *) echo "#$pr verify did not finish within 60 minutes (last: ${s:-none})"; return 1 ;; esac
    [ "$s" = "completed success" ] || { echo "#$pr verify: $s"; return 1; }
    fetch || { echo "#$pr fetch failed"; return 1; }
    git merge-base --is-ancestor origin/main "$H" || { echo "#$pr main moved during CI; retrying"; continue; }
    if gh pr merge "$pr" --merge --admin --delete-branch --match-head-commit "$H" >/dev/null 2>&1; then
      echo "#$pr merged"; return 0
    fi
    echo "#$pr head moved since verify; rechecking"
  done
  echo "#$pr gave up after retries"
  return 1
}

while [ $# -ge 3 ]; do
  merge_one "$1" "$2" "$3" || { echo "stopped at #$1; later PRs were not merged"; exit 1; }
  shift 3
done
