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
  if [[ "$prev" == "--report-path" || "$prev" == "--output" || "$prev" == "--output-file" ]]; then out="$a"; fi
  prev="$a"
done
if [[ -n "$out" && -n "${STUB_JSON:-}" ]]; then printf '%s' "$STUB_JSON" >"$out"; fi
if [[ -n "${STUB_ARGS:-}" ]]; then printf '%s\n' "$@" >"$STUB_ARGS"; fi
echo "stub $(basename "$0") $*" >&2
if [[ -n "${STUB_STDERR:-}" ]]; then printf '%s\n' "$STUB_STDERR" >&2; fi
# STUB_INFO は osv-scanner の進捗ログ（info）の代わり。--verbosity error のときは出さない。
if [[ -n "${STUB_INFO:-}" && " $* " != *" --verbosity error "* ]]; then printf '%s\n' "$STUB_INFO" >&2; fi
exit "${STUB_RC:-0}"
EOF
  chmod +x "$work/bin/$tool"
done

# run_case <name> <tool> <stub-rc> <stub-json> <want-rc>
# STUB_STDERR が設定されていれば、偽物はその行を標準エラーに出す。
# 結果は REPLY_LOG（標準出力と標準エラー）と REPLY_SUMMARY（ジョブサマリー）に入る。
run_case() {
  local name="$1" tool="$2" stub_rc="$3" stub_json="$4" want_rc="$5"
  local summary="$work/summary-$name.md" rc
  : >"$summary"
  REPLY_LOG="$(cd "$work" && PATH="$work/bin:$PATH" STUB_RC="$stub_rc" STUB_JSON="$stub_json" \
    GITHUB_STEP_SUMMARY="$summary" SECURITY_SCAN_OUT="$work/out-$name" STUB_ARGS="$work/args-$name" \
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
# 件数は JSON の検出の数から数える。ファイル名に改行や | があっても件数は増えず、表の列も崩れない。
gitleaks_odd='[{"RuleID":"r","File":"a\nb|c.txt","StartLine":1,"Commit":"3333333333333333"}]'
run_case gitleaks-odd-path gitleaks 2 "$gitleaks_odd" 0
want_contains gitleaks-odd-path "$REPLY_LOG" "検出 1 件"
want_contains gitleaks-odd-path "$REPLY_SUMMARY" 'a b\|c.txt:1'
# マージコミットの差分（競合解消で足した行）も走査するため、git log に -m を渡す。
want_contains gitleaks-merge-diff "$(cat "$work/args-gitleaks-clean")" "--log-opts=-m HEAD"
# -m では同じ場所の値が複数のコミットで重ねて出るため、場所の重複を除いた数もサマリーに出す。
gitleaks_dup='[{"RuleID":"r","File":"a.txt","StartLine":3,"Commit":"1111111111111111"},{"RuleID":"r","File":"a.txt","StartLine":3,"Commit":"2222222222222222"}]'
run_case gitleaks-dup gitleaks 2 "$gitleaks_dup" 0
want_contains gitleaks-dup "$REPLY_LOG" "2 件"
want_contains gitleaks-dup "$REPLY_SUMMARY" "1 か所"

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
# 検出（終了コード 1）があっても、level が error の解析エラーはツールの失敗として扱う
# （semgrep は検出の終了コードをエラーより優先するため、終了コードだけでは見分けられない）。
semgrep_fatal='{"results":[{"check_id":"x","path":"a.rs","start":{"line":1},"extra":{"severity":"INFO"}}],"errors":[{"level":"error","type":"Fatal error","code":2}]}'
run_case semgrep-hit-fatal semgrep 1 "$semgrep_fatal" 2
want_contains semgrep-hit-fatal "$REPLY_LOG" "::error::"
# level が warn の解析エラー（構文を解析できなかったファイル）は報告だけにする。
semgrep_warn='{"results":[{"check_id":"x","path":"a.rs","start":{"line":1},"extra":{"severity":"INFO"}}],"errors":[{"level":"warn","type":"Syntax error","code":3}]}'
run_case semgrep-hit-warn semgrep 1 "$semgrep_warn" 0
want_contains semgrep-hit-warn "$REPLY_SUMMARY" "解析エラー 1 件"

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
# 検出（終了コード 1）があっても、ロックファイルの抽出に失敗していれば失敗にする
# （osv-scanner 2.7.1 は抽出の失敗を標準エラーに出すだけで、検出があれば 1 を返す）。
osv_diag="Error during extraction: (extracting as javascript/packagelockjson) bad/package-lock.json: fetch https://user:FAKEPASSWORD@registry.invalid/x failed"
STUB_STDERR="$osv_diag" run_case osv-hit-partial osv-scanner 1 "$osv_hit" 127
want_contains osv-hit-partial "$REPLY_LOG" "::error::"
want_contains osv-hit-partial "$REPLY_LOG" "抽出エラーの診断 1 件"
# osv-scanner の標準エラー（取得先の URL などを含みうる）は公開のジョブログに出さない。
want_absent osv-hit-partial "$REPLY_LOG" "FAKEPASSWORD"
want_absent osv-hit-partial "$REPLY_LOG" "stub osv-scanner"
# 進捗ログは読めたファイルのパスをそのまま書く。パスに改行があると「Error during extraction:」で
# 始まる行を作れるため、進捗ログを出させず（--verbosity error）、正常な抽出を失敗と取り違えない。
STUB_INFO=$'Scanned /w/a\nError during extraction: x/Cargo.lock file and found 1 packages' \
  run_case osv-hit-odd-path osv-scanner 1 "$osv_hit" 0
want_contains osv-hit-odd-path "$REPLY_LOG" "::warning::"
STUB_STDERR="$osv_diag" run_case osv-error-quiet osv-scanner 127 "" 127
want_absent osv-error-quiet "$REPLY_LOG" "FAKEPASSWORD"
# ロックファイルが 1 つも見つからない（128）は設定の誤りとして失敗させる。
run_case osv-nopkg osv-scanner 128 "" 128

# ---- 引数の誤り
run_case bad-tool nope 0 "" 2

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}"
  exit 1
fi
echo "security-scan.test.sh: all cases passed"
