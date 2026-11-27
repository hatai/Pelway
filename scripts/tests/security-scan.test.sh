#!/usr/bin/env bash
# scripts/security-scan.sh のテスト（Issue #571）。gitleaks・semgrep・osv-scanner を
# PATH 上の偽物に差し替え、終了コードの扱いとサマリーの中身を確かめる:
#   - 検出なし: 終了コード 0、警告なし
#   - 失敗させる検出あり（gitleaks の全件・semgrep の INFO 以外・osv-scanner の unmaintained
#     以外の勧告）: 終了コード 4、::error:: に件数、サマリーに場所と種類
#   - 報告のみの検出だけ（semgrep の INFO・osv-scanner の unmaintained）: 終了コード 0、::warning::
#   - ツールの失敗: 終了コード 0 以外（検出と区別してジョブを失敗させる）
#   - gitleaks の検出の値（Secret・Match）はサマリーにもログにも出さない
# 偽物は、渡された出力先（--report-path / --output の次の引数）へ STUB_JSON の内容を書き、
# STUB_RC の終了コードで終わる。本物のツールの JSON の形は各ツールの公式の出力に合わせた。
# 期待する文字列にはコードスパンの ` をそのまま書くため、単一引用符の中の ` を許す。
# shellcheck disable=SC2016
set -uo pipefail

SCRIPT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/security-scan.sh"
ROOT="$(cd "$(dirname "$SCRIPT")/.." && pwd)"
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
if [[ -n "$out" && -n "${STUB_JSON_FILE:-}" ]]; then cp "$STUB_JSON_FILE" "$out"; fi
if [[ -n "${STUB_ARGS:-}" ]]; then printf '%s\n' "$@" >"$STUB_ARGS"; fi
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
  local summary="$work/summary-$name.md" rc scan_base_set=0 scan_base_value=""
  # 呼び出し側で一時指定した SECURITY_SCAN_BASE_SHA だけを使い、外の環境の値は外す。
  if [[ -v SECURITY_SCAN_BASE_SHA ]]; then
    scan_base_set=1
    scan_base_value="$SECURITY_SCAN_BASE_SHA"
  fi
  local -a scan_base_env=()
  if [[ "$scan_base_set" == 1 ]]; then
    scan_base_env=("SECURITY_SCAN_BASE_SHA=$scan_base_value")
  fi
  : >"$summary"
  REPLY_LOG="$(cd "$work" && env -u SECURITY_SCAN_BASE_SHA PATH="$work/bin:$PATH" \
    STUB_RC="$stub_rc" STUB_JSON="$stub_json" GITHUB_STEP_SUMMARY="$summary" \
    SECURITY_SCAN_OUT="$work/out-$name" STUB_ARGS="$work/args-$name" \
    "${scan_base_env[@]}" bash "$SCRIPT" "$tool" 2>&1)"
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
run_case gitleaks-hit gitleaks 2 "$gitleaks_hit" 4
want_contains gitleaks-hit "$REPLY_LOG" "::error::gitleaks: 検出 1 件のうち 1 件"
want_absent gitleaks-hit "$REPLY_LOG" "::warning"
want_contains gitleaks-hit "$REPLY_LOG" "1 件"
want_contains gitleaks-hit "$REPLY_SUMMARY" '` github-pat `'
want_contains gitleaks-hit "$REPLY_SUMMARY" '` docs/a.md:12 `'
want_contains gitleaks-hit "$REPLY_SUMMARY" "0123456789ab"
want_absent gitleaks-hit "$REPLY_SUMMARY" "$secret_value"
want_absent gitleaks-hit "$REPLY_LOG" "$secret_value"
run_case gitleaks-error gitleaks 1 "" 1
# gitleaks 8.30.1 は git log の失敗を ERR のログに出すだけで、終了コードは 0（検出ありなら 2）に
# なる。--log-level error で、標準エラーに 1 行でもあれば失敗にする。診断の中身（.gitleaks.toml の
# 誤った行など、秘密を含みうる）はログに出さない。
gitleaks_diag="ERR [git] fatal: ambiguous argument 'FAKEGITLEAKSSECRET': unknown revision"
STUB_STDERR="$gitleaks_diag" run_case gitleaks-clean-stderr gitleaks 0 "[]" 3
want_contains gitleaks-clean-stderr "$REPLY_LOG" "::error::"
want_absent gitleaks-clean-stderr "$REPLY_LOG" "FAKEGITLEAKSSECRET"
STUB_STDERR="$gitleaks_diag" run_case gitleaks-hit-stderr gitleaks 2 "$gitleaks_hit" 3
want_absent gitleaks-hit-stderr "$REPLY_LOG" "FAKEGITLEAKSSECRET"
want_contains gitleaks-log-level "$(cat "$work/args-gitleaks-clean")" $'--log-level\nerror'
# 件数は JSON の検出の数から数える。ファイル名に改行や | があっても件数は増えず、表の列も崩れない。
gitleaks_odd='[{"RuleID":"r","File":"a\nb|c.txt","StartLine":1,"Commit":"3333333333333333"}]'
run_case gitleaks-odd-path gitleaks 2 "$gitleaks_odd" 4
want_contains gitleaks-odd-path "$REPLY_LOG" "検出 1 件"
want_contains gitleaks-odd-path "$REPLY_SUMMARY" '` a b\|c.txt:1 `'
# 表のセルはコードスパンにし、パスに Markdown の記号（リンク・強調・HTML・文字参照・
# メールアドレス・`）があっても書式や自動リンクとして解釈させず、元の文字のまま表示する。
# 囲む ` は値の中の最長の並び（``）より 1 つ多い ``` にする。
gitleaks_md='[{"RuleID":"r","File":"[x](https://e.invalid)*b*<i>&amp;`c``d`|a@b.invalid","StartLine":1,"Commit":"4444444444444444"}]'
run_case gitleaks-md-path gitleaks 2 "$gitleaks_md" 4
want_contains gitleaks-md-path "$REPLY_SUMMARY" '| ``` [x](https://e.invalid)*b*<i>&amp;`c``d`\|a@b.invalid:1 ``` |'
# ジョブサマリーの大きさの上限（1 ステップ 1 MiB）に収めるため、一覧の行は UTF-8 のバイト数の
# 合計で切る。長いパスの行が並んでも上限を超えず、省略した件数を書く。
long="$(printf 'p%.0s' {1..2000})"
# 環境変数には入りきらない大きさのため、偽物にはファイルで渡す（STUB_JSON_FILE）。
jq -cn --arg l "$long" '[range(600) | {RuleID:"r",File:"\($l)\(.)",StartLine:1,Commit:"5555555555555555"}]' >"$work/gitleaks-many.json"
STUB_JSON_FILE="$work/gitleaks-many.json" run_case gitleaks-many gitleaks 2 "" 4
want_contains gitleaks-many "$REPLY_SUMMARY" "検出 600 件"
want_contains gitleaks-many "$REPLY_SUMMARY" "件は、ジョブサマリーの大きさの上限のため省略"
summary_bytes="$(wc -c <"$work/summary-gitleaks-many.md")"
((summary_bytes <= 1048576)) || failures+=("gitleaks-many: summary is $summary_bytes bytes, want <= 1048576")
((summary_bytes >= 800000)) || failures+=("gitleaks-many: summary is $summary_bytes bytes, want rows up to the budget")
# マージコミットの差分（競合解消で足した行）も走査するため、git log に -m を渡す。
# gitleaks の値を伏せる --redact と、semgrep の利用状況の送信を止める --metrics=off を外さない。
want_contains gitleaks-redact "$(cat "$work/args-gitleaks-clean")" "--redact"
want_contains gitleaks-merge-diff "$(cat "$work/args-gitleaks-clean")" "--log-opts=-m HEAD"
# PR では base（SECURITY_SCAN_BASE_SHA）から HEAD までのコミットだけを走査する。
base="0123456789abcdef0123456789abcdef01234567"
SECURITY_SCAN_BASE_SHA="$base" run_case gitleaks-pr-range gitleaks 0 "[]" 0
want_contains gitleaks-pr-range "$(cat "$work/args-gitleaks-pr-range")" "--log-opts=-m $base..HEAD"
# base が SHA の形でなければ、ツールを呼ばずに引数の誤り（2）で止める。空文字も未設定ではなく不正値。
SECURITY_SCAN_BASE_SHA="main; rm -rf /" run_case gitleaks-bad-base gitleaks 0 "[]" 2
want_contains gitleaks-bad-base "$REPLY_LOG" "::error::SECURITY_SCAN_BASE_SHA"
[[ ! -e "$work/args-gitleaks-bad-base" ]] || failures+=("gitleaks-bad-base: gitleaks was called")
SECURITY_SCAN_BASE_SHA="0123456" run_case gitleaks-short-base gitleaks 0 "[]" 2
[[ ! -e "$work/args-gitleaks-short-base" ]] || failures+=("gitleaks-short-base: gitleaks was called")
SECURITY_SCAN_BASE_SHA="" run_case gitleaks-empty-base gitleaks 0 "[]" 2
[[ ! -e "$work/args-gitleaks-empty-base" ]] || failures+=("gitleaks-empty-base: gitleaks was called")
# -m では同じ場所の値が複数のコミットで重ねて出るため、場所の重複を除いた数もサマリーに出す。
gitleaks_dup='[{"RuleID":"r","File":"a.txt","StartLine":3,"Commit":"1111111111111111"},{"RuleID":"r","File":"a.txt","StartLine":3,"Commit":"2222222222222222"}]'
run_case gitleaks-dup gitleaks 2 "$gitleaks_dup" 4
want_contains gitleaks-dup "$REPLY_LOG" "2 件"
want_contains gitleaks-dup "$REPLY_SUMMARY" "1 か所"

# ---- semgrep
semgrep_hit='{"results":[{"check_id":"rust.lang.security.unsafe-usage","path":"crates/x/src/lib.rs","start":{"line":7},"extra":{"severity":"WARNING"}},{"check_id":"yaml.github-actions.security.run-shell-injection","path":".github/workflows/a.yml","start":{"line":3},"extra":{"severity":"ERROR"}}],"errors":[]}'
run_case semgrep-clean semgrep 0 '{"results":[],"errors":[]}' 0
want_absent semgrep-clean "$REPLY_LOG" "::warning"
want_contains semgrep-metrics-off "$(cat "$work/args-semgrep-clean")" "--metrics=off"
run_case semgrep-hit semgrep 1 "$semgrep_hit" 4
want_contains semgrep-hit "$REPLY_LOG" "::error::semgrep: 検出 2 件のうち 2 件"
want_contains semgrep-hit "$REPLY_SUMMARY" '` rust.lang.security.unsafe-usage `'
want_contains semgrep-hit "$REPLY_SUMMARY" '` crates/x/src/lib.rs:7 `'
want_contains semgrep-hit "$REPLY_SUMMARY" '` .github/workflows/a.yml:3 `'
# 重大度 INFO の検出（unsafe の使用など監査向けの規則）は報告だけにし、ジョブを失敗させない。
semgrep_info='{"results":[{"check_id":"rust.lang.security.unsafe-usage","path":"a.rs","start":{"line":1},"extra":{"severity":"INFO"}},{"check_id":"y","path":"b.yml","start":{"line":2},"extra":{"severity":"WARNING"}}],"errors":[]}'
run_case semgrep-mixed semgrep 1 "$semgrep_info" 4
want_contains semgrep-mixed "$REPLY_LOG" "検出 2 件のうち 1 件"
want_contains semgrep-mixed "$REPLY_SUMMARY" "うちジョブを失敗させるもの 1 件"
semgrep_info_only='{"results":[{"check_id":"rust.lang.security.unsafe-usage","path":"a.rs","start":{"line":1},"extra":{"severity":"INFO"}}],"errors":[]}'
run_case semgrep-info semgrep 1 "$semgrep_info_only" 0
want_contains semgrep-info "$REPLY_LOG" "::warning::semgrep: 検出 1 件（報告のみ）"
want_absent semgrep-info "$REPLY_LOG" "::error"
# 知らない重大度は失敗させる側に倒す（Registry の規則が新しい重大度を使っても見逃さない）。
semgrep_unknown='{"results":[{"check_id":"z","path":"c.rs","start":{"line":3},"extra":{"severity":"CRITICAL"}}],"errors":[]}'
run_case semgrep-unknown-severity semgrep 1 "$semgrep_unknown" 4
# SECURITY_SCAN_BASE_SHA は gitleaks だけが使う。semgrep では、形が違っても検証で止まらずツールを呼ぶ。
SECURITY_SCAN_BASE_SHA="main; rm -rf /" run_case semgrep-bad-base semgrep 0 '{"results":[],"errors":[]}' 0
[[ -e "$work/args-semgrep-bad-base" ]] || failures+=("semgrep-bad-base: semgrep was not called")
# uses: のタグ参照は actions.lock が SHA を固定するため、mutable-action-tag の規則は使わない。
want_contains semgrep-exclude-rule "$(cat "$work/args-semgrep-clean")" $'--exclude-rule\nyaml.github-actions.security.github-actions-mutable-action-tag.github-actions-mutable-action-tag'
run_case semgrep-error semgrep 2 "" 2
# ツール自身が終了コード 4 を返したときも、ログの ::error:: で「失敗させる検出」と見分けられる。
run_case semgrep-error-rc4 semgrep 4 "" 4
want_contains semgrep-error-rc4 "$REPLY_LOG" "::error::semgrep が失敗しました（終了コード 4"
want_absent semgrep-error-rc4 "$REPLY_LOG" "ジョブを失敗させる検出"
# semgrep の標準エラー（進捗など）は失敗の判定に使わず（解析エラーは JSON で見る）、ログにも出さない。
STUB_STDERR="semgrep progress FAKESEMGREPDIAG" run_case semgrep-clean-stderr semgrep 0 '{"results":[],"errors":[]}' 0
want_absent semgrep-clean-stderr "$REPLY_LOG" "FAKESEMGREPDIAG"
# 検出（終了コード 1）があっても、level が error の解析エラーはツールの失敗として扱う
# （semgrep は検出の終了コードをエラーより優先するため、終了コードだけでは見分けられない）。
semgrep_fatal='{"results":[{"check_id":"x","path":"a.rs","start":{"line":1},"extra":{"severity":"INFO"}}],"errors":[{"level":"error","type":"Fatal error","code":2}]}'
run_case semgrep-hit-fatal semgrep 1 "$semgrep_fatal" 3
want_contains semgrep-hit-fatal "$REPLY_LOG" "::error::"
# level が warn の解析エラー（構文を解析できなかったファイル）は報告だけにする。
semgrep_warn='{"results":[{"check_id":"x","path":"a.rs","start":{"line":1},"extra":{"severity":"INFO"}}],"errors":[{"level":"warn","type":"Syntax error","code":3}]}'
run_case semgrep-hit-warn semgrep 1 "$semgrep_warn" 0
want_contains semgrep-hit-warn "$REPLY_SUMMARY" "解析エラー 1 件"

# ---- osv-scanner
osv_hit='{"results":[{"source":{"path":"/w/Cargo.lock","type":"lockfile"},"packages":[{"package":{"name":"rsa","version":"0.9.8","ecosystem":"crates.io"},"vulnerabilities":[{"id":"RUSTSEC-2023-0071","affected":[{"database_specific":{"informational":null}}]},{"id":"GHSA-c38w-74pg-36hr"}]}]}]}'
run_case osv-clean osv-scanner 0 '{"results":[]}' 0
want_absent osv-clean "$REPLY_LOG" "::warning"
# 呼び出し解析（Go は既定で有効）は失敗を検出の終了コードに埋もれさせるため、無効にする。
want_contains osv-no-call-analysis "$(cat "$work/args-osv-clean")" "--no-call-analysis=all"
# マニフェストの推移的な依存の解決は、PR の作者が指定できるレジストリへ取りに行くため、使わない。
want_contains osv-no-resolve "$(cat "$work/args-osv-clean")" "--no-resolve"
# 既定の抽出器（同梱のソースのパスとハッシュを OSV.dev に送る directory など）も、マニフェストの
# 抽出器を含む lockfile プリセットも使わず、Cargo.lock の抽出器だけにする。
want_contains osv-plugins "$(cat "$work/args-osv-clean")" $'--x-no-default-plugins\n--x-plugins\nrust/cargolock'
run_case osv-hit osv-scanner 1 "$osv_hit" 4
want_contains osv-hit "$REPLY_LOG" "::error::osv-scanner: 検出 1 件のうち 1 件"
want_contains osv-hit "$REPLY_SUMMARY" '` rsa 0.9.8 `'
want_contains osv-hit "$REPLY_SUMMARY" '` RUSTSEC-2023-0071, GHSA-c38w-74pg-36hr `'
want_contains osv-hit "$REPLY_SUMMARY" '` /w/Cargo.lock `'
# unmaintained の告知（RUSTSEC の informational）だけのパッケージは報告だけにし、サマリーに種類を書く。
osv_unmaintained='{"results":[{"source":{"path":"/w/Cargo.lock","type":"lockfile"},"packages":[{"package":{"name":"pin-utils","version":"0.1.1","ecosystem":"crates.io"},"vulnerabilities":[{"id":"RUSTSEC-2026-0441","affected":[{"database_specific":{"informational":"unmaintained"}}]}]}]}]}'
run_case osv-unmaintained osv-scanner 1 "$osv_unmaintained" 0
want_contains osv-unmaintained "$REPLY_LOG" "::warning::osv-scanner: 検出 1 件（報告のみ）"
want_contains osv-unmaintained "$REPLY_SUMMARY" '` RUSTSEC-2026-0441 (unmaintained) `'
want_absent osv-unmaintained "$REPLY_LOG" "::error"
# unmaintained 以外の告知（unsound など）は失敗させる。
osv_unsound='{"results":[{"source":{"path":"/w/Cargo.lock","type":"lockfile"},"packages":[{"package":{"name":"x","version":"1.0.0","ecosystem":"crates.io"},"vulnerabilities":[{"id":"RUSTSEC-2099-0001","affected":[{"database_specific":{"informational":"unsound"}}]}]}]}]}'
run_case osv-unsound osv-scanner 1 "$osv_unsound" 4
want_contains osv-unsound "$REPLY_SUMMARY" '` RUSTSEC-2099-0001 (unsound) `'
# 同じパッケージに unmaintained と通常の勧告があれば、通常の勧告の分で失敗させる。
osv_mixed='{"results":[{"source":{"path":"/w/Cargo.lock","type":"lockfile"},"packages":[{"package":{"name":"y","version":"1.0.0","ecosystem":"crates.io"},"vulnerabilities":[{"id":"RUSTSEC-2099-0002","affected":[{"database_specific":{"informational":"unmaintained"}}]},{"id":"RUSTSEC-2099-0003","affected":[{"database_specific":{"informational":null}}]}]}]}]}'
run_case osv-mixed osv-scanner 1 "$osv_mixed" 4
want_contains osv-mixed "$REPLY_SUMMARY" '` RUSTSEC-2099-0002 (unmaintained), RUSTSEC-2099-0003 `'
run_case osv-error osv-scanner 127 "" 127
# 検出（終了コード 1）があっても、ロックファイルの抽出に失敗していれば失敗にする
# （osv-scanner 2.7.1 は抽出の失敗を標準エラーに出すだけで、検出があれば 1 を返す）。
osv_diag="Error during extraction: (extracting as javascript/packagelockjson) bad/package-lock.json: fetch https://user:FAKEPASSWORD@registry.invalid/x failed"
STUB_STDERR="$osv_diag" run_case osv-hit-partial osv-scanner 1 "$osv_hit" 3
want_contains osv-hit-partial "$REPLY_LOG" "::error::"
want_contains osv-hit-partial "$REPLY_LOG" "1 行"
# osv-scanner の標準エラー（取得先の URL などを含みうる）は公開のジョブログに出さない。
want_absent osv-hit-partial "$REPLY_LOG" "FAKEPASSWORD"
# 進捗ログは読めたファイルのパスをそのまま書く。パスに改行があると「Error during extraction:」で
# 始まる行を作れるため、進捗ログを出させず（--verbosity error）、正常な抽出を失敗と取り違えない。
STUB_INFO=$'Scanned /w/a\nError during extraction: x/Cargo.lock file and found 1 packages' \
  run_case osv-hit-odd-path osv-scanner 1 "$osv_hit" 4
want_contains osv-hit-odd-path "$REPLY_LOG" "::error::osv-scanner: 検出 1 件"
STUB_STDERR="$osv_diag" run_case osv-error-quiet osv-scanner 127 "" 127
want_absent osv-error-quiet "$REPLY_LOG" "FAKEPASSWORD"
# osv-scanner でも SECURITY_SCAN_BASE_SHA の形は検証しない。
SECURITY_SCAN_BASE_SHA="main; rm -rf /" run_case osv-bad-base osv-scanner 0 '{"results":[]}' 0
[[ -e "$work/args-osv-bad-base" ]] || failures+=("osv-bad-base: osv-scanner was not called")
# ロックファイルが 1 つも見つからない（128）は設定の誤りとして失敗させる。
run_case osv-nopkg osv-scanner 128 "" 128

# ---- 引数の誤り
run_case bad-tool nope 0 "" 2

# ---- workflow の配線の presence check（意味の検査は上の引数テスト）
workflow_text="$(cat "$ROOT/.github/workflows/security-scan.yml")"
want_contains workflow-pr-step "$workflow_text" '- name: Scan (gitleaks git, pull_request)'
want_contains workflow-pr-if "$workflow_text" "if: github.event_name == 'pull_request'"
want_contains workflow-base-env "$workflow_text" 'SECURITY_SCAN_BASE_SHA: ${{ github.event.pull_request.base.sha }}'
want_contains workflow-full-step "$workflow_text" '- name: Scan (gitleaks git, full history)'
want_contains workflow-full-if "$workflow_text" "if: github.event_name != 'pull_request'"
want_contains workflow-fetch-depth "$workflow_text" 'fetch-depth: 0'
base_env_count="$(grep -cF 'SECURITY_SCAN_BASE_SHA: ${{ github.event.pull_request.base.sha }}' \
  "$ROOT/.github/workflows/security-scan.yml" || true)"
[[ "$base_env_count" == 1 ]] || failures+=("workflow-base-env: count $base_env_count, want 1")

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}"
  exit 1
fi
echo "security-scan.test.sh: all cases passed"
