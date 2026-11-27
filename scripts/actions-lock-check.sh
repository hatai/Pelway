#!/usr/bin/env bash
# workflow の uses: と .github/workflows/actions.lock の照合（Issue #838）。
# jactionlint（GitHub Actions 用 linter）の actions-lock / unpinned-uses / yaml-syntax を使う。
# --no-online で upstream への問い合わせをせず、--no-ignores で inline ignore や設定による
# bypass を効かなくする。実行はカレントディレクトリの git プロジェクトが対象。
set -euo pipefail

if ! command -v jactionlint >/dev/null 2>&1; then
  echo "actions-lock-check: jactionlint が PATH にありません（mise run actions-lock-check から実行してください）" >&2
  exit 2
fi

exec jactionlint \
  --select actions-lock,unpinned-uses,yaml-syntax \
  --no-online \
  --no-ignores \
  --oneline
