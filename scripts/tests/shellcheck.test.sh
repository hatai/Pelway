#!/usr/bin/env bash
# Issue #265（shellcheck 導入）の受け入れ条件を確かめるテスト:
#   - mise のツール定義に shellcheck が載り、mise exec -- shellcheck --version が通る
#   - mise.toml の shellcheck の版が latest ではなく版番号で固定され、
#     実行される shellcheck の --version 出力と一致する
#   - mise.lock に shellcheck の取得先 URL と checksum が記録されている（取得物の固定）
#   - shellcheck scripts/demo-check/*.sh が severity 既定で指摘 0 件・exit 0
#   - 専用の mise タスク（shellcheck）が通り、verify-static の依存に含まれる
#     （= mise run verify と CI の static ジョブから実行される）
# Issue #621（検査の範囲を scripts/ の bash・sh 全部に広げる）の受け入れ条件:
#   - mise run shellcheck が検査したファイルの数を出し、scripts/ 以下で 1 行目が
#     #! で始まり bash か sh を含むファイルの数と一致する
#   - scripts/ の写しに warning 級の指摘を 1 件含むスクリプトを足すと、検査が
#     0 以外で終わる（info 級だけなら通る）。リポジトリのファイルは変えない
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT" || exit 1
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
task_out="$(mise run shellcheck 2>/dev/null)"
task_rc=$?
if [[ "$task_rc" -ne 0 ]]; then
  failures+=("mise run shellcheck failed (exit ${task_rc})")
fi
if ! sed -n '/^\[tasks\.verify-static\]/,/^\[/p' mise.toml | grep -q '"shellcheck"'; then
  failures+=("verify-static does not depend on the shellcheck task")
fi

# 6. 検査したファイルの数が、scripts/ 以下の bash・sh のスクリプトの数と一致する
want_count="$(find scripts -type f -exec sh -c 'head -n 1 "$1" | grep -qE "^#!.*\b(bash|sh)\b"' _ {} \; -print | wc -l)"
got_count="$(sed -nE 's/^shellcheck: ([0-9]+) files checked.*/\1/p' <<<"$task_out")"
if [[ "$got_count" != "$want_count" ]]; then
  failures+=("mise run shellcheck reported '${got_count:-<none>}' files checked, want ${want_count}")
fi

# 7. scripts/ の写しで確かめる（mise タスクの本体 scripts/shellcheck.sh を写しのルートに当てる。
#    リポジトリのファイルは変えない）:
#    - info 級（SC2016）だけのスクリプトは、demo-check の外なら通り、
#      scripts/demo-check/*.sh なら既定の severity のため失敗する
#    - warning 級（SC2164）を 1 件含むスクリプトで失敗し、その指摘を出す
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
cp -a scripts "$tmp/scripts"
sc() { mise exec -- bash "$tmp/scripts/shellcheck.sh" "$tmp" 2>&1; }
if [[ ! -f "$tmp/scripts/shellcheck.sh" ]]; then
  failures+=("scripts/shellcheck.sh is missing")
else
  info_fixture="$(printf '#!/usr/bin/env bash\necho %s\n' "'\$HOME'")"
  printf '%s\n' "$info_fixture" > "$tmp/scripts/info-only.sh"
  if ! out="$(sc)"; then
    failures+=("shellcheck.sh failed on an info-level-only script outside demo-check (want pass): $(head -n3 <<<"$out")")
  fi
  printf '%s\n' "$info_fixture" > "$tmp/scripts/demo-check/info-only.sh"
  if out="$(sc)"; then
    failures+=("shellcheck.sh passed an info-level finding in scripts/demo-check/info-only.sh (want non-zero at default severity)")
  elif ! grep -q 'SC2016' <<<"$out"; then
    failures+=("shellcheck.sh failed on demo-check/info-only.sh without reporting SC2016: $(head -n3 <<<"$out")")
  fi
  rm "$tmp/scripts/demo-check/info-only.sh"
  mkdir -p "$tmp/scripts/sub"
  printf '#!/bin/sh\ncd /nonexistent\n' > "$tmp/scripts/sub/warn.sh"
  if out="$(sc)"; then
    failures+=("shellcheck.sh passed with a warning-level finding in scripts/sub/warn.sh (want non-zero)")
  elif ! grep -q 'sub/warn.sh' <<<"$out" || ! grep -q 'SC2164' <<<"$out"; then
    failures+=("shellcheck.sh failed without reporting SC2164 in scripts/sub/warn.sh: $(head -n3 <<<"$out")")
  fi
fi

if ((${#failures[@]})); then
  printf 'FAIL %s\n' "${failures[@]}"
  exit 1
fi
echo ok
