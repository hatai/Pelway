#!/usr/bin/env bash
# Issue #265（shellcheck 導入）の受け入れ条件を確かめるテスト:
#   - mise のツール定義に shellcheck が載り、mise exec -- shellcheck --version が通る
#   - mise.toml の shellcheck の版が latest ではなく版番号で固定され、
#     実行される shellcheck の --version 出力と一致する
#   - mise.lock に shellcheck の取得先 URL と checksum が記録されている（取得物の固定）
#   - shellcheck scripts/demo-check/*.sh が severity 既定で指摘 0 件・exit 0
#   - 専用の mise タスク（shellcheck）が通り、verify-static の依存に含まれる
#     （= mise run verify と CI の static ジョブから実行される）
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
failures=()

# 1. mise exec -- shellcheck --version（mise のツール定義に載っていること）
if ! out="$(mise exec -- shellcheck --version 2>&1)"; then
  failures+=("mise exec -- shellcheck --version failed: $(head -n1 <<<"$out")")
fi

# 2. mise.toml の版固定（latest 不可）と、実行される版の一致
pin="$(grep -E '^\s*"aqua:koalaman/shellcheck"\s*=' mise.toml | head -n1 | sed -E 's/.*=[[:space:]]*"([^"]+)".*/\1/')"
if [[ ! "$pin" =~ ^[0-9]+(\.[0-9]+)+$ ]]; then
  failures+=("mise.toml pins aqua:koalaman/shellcheck to '${pin:-<absent>}': expected a version number, not latest")
elif ! mise exec -- shellcheck --version 2>/dev/null | grep -q "version: ${pin}"; then
  failures+=("shellcheck --version does not report the pinned version ${pin}")
fi

# 3. mise.lock の shellcheck 区間（次の [[tools. まで）に URL と checksum があること
lock_body="$(awk '
  /^\[\[tools\./ { if (insec) exit; if ($0 ~ /aqua:koalaman\/shellcheck/) insec=1 }
  insec { print }
' mise.lock)"
if [[ -z "$lock_body" ]]; then
  failures+=("mise.lock has no [[tools.\"aqua:koalaman/shellcheck\"]] section")
else
  grep -q '^url = "https://' <<<"$lock_body" || failures+=("mise.lock: shellcheck entry has no url")
  grep -q '^checksum = "sha256:' <<<"$lock_body" || failures+=("mise.lock: shellcheck entry has no checksum")
fi

# 4. 本体: severity 既定で指摘 0 件・exit 0。shellcheck の指摘は stdout に出るため
#    stdout のみを判定する（mise exec 自身のツール同期メッセージは stderr で除外）。
sc_out="$(mise exec -- shellcheck scripts/demo-check/*.sh 2>/dev/null)"
sc_rc=$?
if [[ "$sc_rc" -ne 0 ]]; then
  failures+=("shellcheck scripts/demo-check/*.sh exited ${sc_rc} (want 0): $(head -n3 <<<"$sc_out")")
elif [[ -n "$sc_out" ]]; then
  failures+=("shellcheck scripts/demo-check/*.sh printed findings (want no output)")
fi

# 5. 専用タスクの実行と verify-static への組込み
if ! mise run shellcheck >/dev/null 2>&1; then
  failures+=("mise run shellcheck failed")
fi
if ! sed -n '/^\[tasks\.verify-static\]/,/^\[/p' mise.toml | grep -q '"shellcheck"'; then
  failures+=("verify-static does not depend on the shellcheck task")
fi

if ((${#failures[@]})); then
  printf 'FAIL %s\n' "${failures[@]}"
  exit 1
fi
echo ok
