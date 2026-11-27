#!/usr/bin/env bash
# notify-grokbot の見本テストを script-test から実行する。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

bash -n docs/worklogs/gh-notify-samples/check.sh
shellcheck --severity=warning docs/worklogs/gh-notify-samples/check.sh
python3 -c 'compile(open("docs/worklogs/gh-notify-samples/notify_harness.py", encoding="utf-8").read(), "docs/worklogs/gh-notify-samples/notify_harness.py", "exec")'
extracted="$(mktemp -d)"
trap 'rm -rf "$extracted"' EXIT
python3 docs/worklogs/gh-notify-samples/notify_harness.py extract .github/workflows/notify-grokbot.yml "$extracted"
shellcheck --shell=bash --severity=warning "$extracted/dedup.sh" "$extracted/send.sh" "$extracted/record.sh"
bash docs/worklogs/gh-notify-samples/check.sh
