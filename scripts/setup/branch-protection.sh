#!/usr/bin/env bash
# main のブランチ保護を設定する（何度実行しても同じ結果）。
# - 人間（CODEOWNERS の @hatai）の Approve が必須
# - CI の "mise run verify" と "pr-size" の成功が必須
# - enforce_admins は false: hatai が緊急時に保護を迂回できる
set -euo pipefail

repo="${1:-hatai/Pelway}"

gh api --method PUT "repos/$repo/branches/main/protection" --input - >/dev/null <<'JSON'
{
  "required_status_checks": { "strict": false, "contexts": ["mise run verify", "pr-size"] },
  "enforce_admins": false,
  "required_pull_request_reviews": {
    "required_approving_review_count": 1,
    "require_code_owner_reviews": true,
    "dismiss_stale_reviews": true
  },
  "restrictions": null
}
JSON

gh api "repos/$repo/branches/main/protection" \
  --jq '{checks: .required_status_checks.contexts, codeowners: .required_pull_request_reviews.require_code_owner_reviews}'
