#!/usr/bin/env bash
# 指揮役のマージの順番待ち（skill command-team の「マージ」）。引数は "PR番号 ブランチ レビュー済みsha" の組を並べる。
# 1 本ずつ、Ready for review を待ち、main を含まなければ取り込んで CI を待ち、レビュー後に main 以外のコミットが無ければ --admin でマージする。
# 使い方: scripts/conductor-merge-queue.sh 128 agent/102-checkpoint-store fff6284... 130 agent/96-rmcp-connection 67aaa70...
cd "$(dirname "$0")/.." || exit 1
merge_one() {
  pr=$1; br=$2; rsha=$3
  until [ "$(scripts/agent-gh dev pr view "$pr" --json isDraft -q .isDraft 2>/dev/null)" = false ]; do sleep 20; done
  for attempt in 1 2 3; do
    scripts/agent-gh dev git fetch -q origin 2>/dev/null
    H=$(git rev-parse "origin/$br")
    n=$(git rev-list "$rsha..$H" --no-merges ^origin/main | wc -l)
    if [ "$n" != 0 ]; then echo "#$pr new non-main commits ($n) since review; re-review needed"; return 1; fi
    if ! git merge-base --is-ancestor origin/main "$H"; then
      scripts/agent-gh dev pr update-branch "$pr" >/dev/null 2>&1 || { echo "#$pr update-branch failed (conflict?)"; return 1; }
      sleep 20; scripts/agent-gh dev git fetch -q origin 2>/dev/null; H=$(git rev-parse "origin/$br")
    fi
    until [ "$(scripts/agent-gh dev run list --branch "$br" --workflow verify --json headSha,status -q ".[]|select(.headSha==\"$H\" and .status==\"completed\")|.status" 2>/dev/null | head -1)" = completed ]; do sleep 30; done
    c=$(scripts/agent-gh dev run list --branch "$br" --workflow verify --json headSha,conclusion -q ".[]|select(.headSha==\"$H\")|.conclusion" 2>/dev/null | head -1)
    [ "$c" = success ] || { echo "#$pr verify=$c"; return 1; }
    scripts/agent-gh dev git fetch -q origin 2>/dev/null
    if git merge-base --is-ancestor origin/main "$H"; then
      gh pr merge "$pr" --merge --admin --delete-branch --match-head-commit "$H" >/dev/null 2>&1 && { echo "#$pr merged"; return; }
      echo "#$pr head moved since verify; rechecking"
    fi
  done
  echo "#$pr gave up after retries"
  return 1
}
while [ $# -ge 3 ]; do
  merge_one "$1" "$2" "$3" || { echo "stopped at #$1; later PRs were not merged"; exit 1; }
  shift 3
done
