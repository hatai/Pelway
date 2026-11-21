#!/usr/bin/env bash
# scripts/security-scan.sh のテスト（Issue #571）。gitleaks・semgrep・osv-scanner を
# PATH 上の偽物に差し替え、終了コードの扱いとサマリーの中身を確かめる:
#   - 検出なし: 終了コード 0、警告なし
#   - 検出あり: 終了コード 0（報告のみ）、::warning:: に件数、サマリーに場所と種類
#   - ツールの失敗: 終了コード 0 以外（検出と区別してジョブを失敗させる）
#   - gitleaks の検出の値（Secret・Match）はサマリーにもログにも出さない
# 偽物は、渡された出力先（--report-path / --output の次の引数）へ STUB_JSON の内容を書き、
# STUB_RC の終了コードで終わる。本物のツールの JSON の形は各ツールの公式の出力に合わせた。
set -uo pipefail

SCRIPT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/security-scan.sh"
failures=()

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/bin"
for tool in gitleaks semgrep osv-scanner; do
  cat >"$work/bin/$tool" <<'EOF'
#!/usr/bin/env bash
out=""
prev=""
for a in "$@"; do
  if [[ "$prev" == "--report-path" || "$prev" == "--output" ]]; then out="$a"; fi
  prev="$a"
done
if [[ -n "$out" && -n "${STUB_JSON:-}" ]]; then printf '%s' "$STUB_JSON" >"$out"; fi
echo "stub $(basename "$0") $*" >&2
exit "${STUB_RC:-0}"
EOF
  chmod +x "$work/bin/$tool"
done

# run_case <name> <tool> <stub-rc> <stub-json> <want-rc>
# 結果は REPLY_LOG（標準出力と標準エラー）と REPLY_SUMMARY（ジョブサマリー）に入る。
run_case() {
  local name="$1" tool="$2" stub_rc="$3" stub_json="$4" want_rc="$5"
  local summary="$work/summary-$name.md" rc
  : >"$summary"
  REPLY_LOG="$(cd "$work" && PATH="$work/bin:$PATH" STUB_RC="$stub_rc" STUB_JSON="$stub_json" \
    GITHUB_STEP_SUMMARY="$summary" SECURITY_SCAN_OUT="$work/out-$name" \
    bash "$SCRIPT" "$tool" 2>&1)"
  rc=$?
  REPLY_SUMMARY="$(cat "$summary")"
  if [[ "$rc" != "$want_rc" ]]; then
    failures+=("$name: exit $rc, want $want_rc")
  fi
}

# want_contains <name> <haystack> <needle>
want_contains() {
  [[ "$2" == *"$3"* ]] || failures+=("$1: missing \"$3\"")
}
want_absent() {
  [[ "$2" != *"$3"* ]] || failures+=("$1: unexpected \"$3\"")
}

# ---- gitleaks
secret_value="ghp_THISISAFAKESECRETVALUE0123456789"
gitleaks_hit="$(cat <<EOF
[{"RuleID":"github-pat","Description":"GitHub Personal Access Token","File":"docs/a.md","StartLine":12,"Commit":"0123456789abcdef0123456789abcdef01234567","Secret":"$secret_value","Match":"token = $secret_value"}]
EOF
)"
run_case gitleaks-clean gitleaks 0 "[]" 0
want_absent gitleaks-clean "$REPLY_LOG" "::warning"
run_case gitleaks-hit gitleaks 2 "$gitleaks_hit" 0
want_contains gitleaks-hit "$REPLY_LOG" "::warning"
want_contains gitleaks-hit "$REPLY_LOG" "1 件"
want_contains gitleaks-hit "$REPLY_SUMMARY" "github-pat"
want_contains gitleaks-hit "$REPLY_SUMMARY" "docs/a.md:12"
want_contains gitleaks-hit "$REPLY_SUMMARY" "0123456789ab"
want_absent gitleaks-hit "$REPLY_SUMMARY" "$secret_value"
want_absent gitleaks-hit "$REPLY_LOG" "$secret_value"
run_case gitleaks-error gitleaks 1 "" 1

# ---- semgrep
semgrep_hit='{"results":[{"check_id":"rust.lang.security.unsafe-usage","path":"crates/x/src/lib.rs","start":{"line":7},"extra":{"severity":"WARNING"}},{"check_id":"yaml.github-actions.security.run-shell-injection","path":".github/workflows/a.yml","start":{"line":3},"extra":{"severity":"ERROR"}}],"errors":[]}'
run_case semgrep-clean semgrep 0 '{"results":[],"errors":[]}' 0
want_absent semgrep-clean "$REPLY_LOG" "::warning"
run_case semgrep-hit semgrep 1 "$semgrep_hit" 0
want_contains semgrep-hit "$REPLY_LOG" "2 件"
want_contains semgrep-hit "$REPLY_SUMMARY" "rust.lang.security.unsafe-usage"
want_contains semgrep-hit "$REPLY_SUMMARY" "crates/x/src/lib.rs:7"
want_contains semgrep-hit "$REPLY_SUMMARY" ".github/workflows/a.yml:3"
run_case semgrep-error semgrep 2 "" 2

# ---- osv-scanner
osv_hit='{"results":[{"source":{"path":"/w/Cargo.lock","type":"lockfile"},"packages":[{"package":{"name":"rsa","version":"0.9.8","ecosystem":"crates.io"},"vulnerabilities":[{"id":"RUSTSEC-2023-0071"},{"id":"GHSA-c38w-74pg-36hr"}]}]}]}'
run_case osv-clean osv-scanner 0 '{"results":[]}' 0
want_absent osv-clean "$REPLY_LOG" "::warning"
run_case osv-hit osv-scanner 1 "$osv_hit" 0
want_contains osv-hit "$REPLY_LOG" "1 件"
want_contains osv-hit "$REPLY_SUMMARY" "rsa 0.9.8"
want_contains osv-hit "$REPLY_SUMMARY" "RUSTSEC-2023-0071"
want_contains osv-hit "$REPLY_SUMMARY" "Cargo.lock"
run_case osv-error osv-scanner 127 "" 127
# ロックファイルが 1 つも見つからない（128）は設定の誤りとして失敗させる。
run_case osv-nopkg osv-scanner 128 "" 128

# ---- 引数の誤り
run_case bad-tool nope 0 "" 2

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}"
  exit 1
fi
echo "security-scan.test.sh: all cases passed"
