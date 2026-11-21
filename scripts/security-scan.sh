#!/usr/bin/env bash
# security-scan workflow（.github/workflows/security-scan.yml）の 1 ツール分の本体（Issue #571）。
# 呼び出しは `bash scripts/security-scan.sh <gitleaks|semgrep|osv-scanner>`。ツールは PATH 上のものを使う。
#
# 当面は報告のみ: 検出があっても終了コード 0 で終わり、::warning:: とジョブサマリー
# （$GITHUB_STEP_SUMMARY。無ければ標準出力）に件数と一覧を出す。ツール自体の失敗は、
# ツールの終了コードのまま失敗させる（検出と取り違えない）。各ツールの終了コードの意味:
#   gitleaks    0 = 検出なし、2 = 検出あり（--exit-code 2 で指定）、それ以外 = 失敗
#   semgrep     0 = 検出なし、1 = 検出あり（--error）、それ以外 = 失敗
#   osv-scanner 0 = 検出なし、1 = 検出あり、128 = ロックファイルが無い、それ以外 = 失敗
#
# 秘密の値は出さない: gitleaks は --redact で値を伏せ、サマリーにも規則・場所・コミットだけを書く
# （レポートの Secret / Match は読まない）。
# 結果の JSON は $SECURITY_SCAN_OUT（既定: リポジトリ外の一時ディレクトリ）に置く。
# 終了コード: 0 = 検出なし・検出あり（報告のみ）、2 = 引数の誤り、それ以外 = ツールの失敗
set -euo pipefail

tool="${1:-}"
case "$tool" in
  gitleaks | semgrep | osv-scanner) ;;
  *)
    echo "usage: $0 <gitleaks|semgrep|osv-scanner>" >&2
    exit 2
    ;;
esac

out_dir="${SECURITY_SCAN_OUT:-$(mktemp -d)}"
mkdir -p "$out_dir"
report="$out_dir/$tool.json"
summary="${GITHUB_STEP_SUMMARY:-/dev/stdout}"

# Semgrep のルールセット（Semgrep Registry から取得。ログイン・トークンは使わない）。
# p/rust: Rust、p/github-actions: CI 設定、r/bash: シェル（p/bash は Registry に無い）。
semgrep_configs=(--config p/rust --config p/github-actions --config r/bash)

rc=0
case "$tool" in
  gitleaks)
    # HEAD から辿れる履歴だけを走査する（checkout の他ブランチやローカルの ref は対象外）。
    # -m: git log は既定でマージコミットの差分を出さないため、競合解消で足した行が漏れる。
    # 各親との差分を出させて走査する（同じ値が複数のコミットで重ねて出る）。
    gitleaks git --redact --no-banner --exit-code 2 --log-opts="-m HEAD" \
      --report-format json --report-path "$report" . || rc=$?
    found_rc=2
    ;;
  semgrep)
    semgrep scan --metrics=off --error "${semgrep_configs[@]}" \
      --json --output "$report" . || rc=$?
    found_rc=1
    ;;
  osv-scanner)
    osv-scanner scan source --recursive --format json --output "$report" . || rc=$?
    found_rc=1
    ;;
esac

if [[ "$rc" != 0 && "$rc" != "$found_rc" ]]; then
  echo "::error::$tool が失敗しました（終了コード $rc）"
  exit "$rc"
fi

# semgrep は検出の終了コード（1）を解析エンジンのエラーより優先して返すため、
# level が error のエラーが JSON にあれば、検出があってもツールの失敗として扱う。
if [[ "$tool" == semgrep ]]; then
  fatal="$(jq '[.errors[]? | select(.level == "error")] | length' "$report")"
  if [[ "$fatal" != 0 ]]; then
    echo "::error::semgrep が失敗しました（level が error の解析エラー ${fatal} 件。詳細は $report）"
    exit 2
  fi
fi

# 検出の一覧（1 行 1 件、タブ区切りではなく Markdown の表の行）を作る。
case "$tool" in
  gitleaks)
    header='| 規則 | 場所 | コミット |'
    rows="$(jq -r '.[] | "| \(.RuleID) | \(.File):\(.StartLine) | \(.Commit[0:12]) |"' "$report")"
    places="$(jq '[.[] | "\(.RuleID) \(.File):\(.StartLine)"] | unique | length' "$report")"
    ;;
  semgrep)
    header='| 規則 | 場所 | 重大度 |'
    rows="$(jq -r '.results[] | "| \(.check_id) | \(.path):\(.start.line) | \(.extra.severity) |"' "$report")"
    errors="$(jq '.errors | length' "$report")"
    ;;
  osv-scanner)
    header='| パッケージ | 勧告 | ロックファイル |'
    rows="$(jq -r '.results[]? | .source.path as $p | .packages[]
      | "| \(.package.name) \(.package.version) | \([.vulnerabilities[].id] | join(", ")) | \($p) |"' "$report")"
    ;;
esac
count=0
[[ -z "$rows" ]] || count="$(printf '%s\n' "$rows" | wc -l | tr -d ' ')"

{
  echo "## $tool"
  echo
  if [[ "$count" == 0 ]]; then
    echo "検出なし。"
  else
    echo "検出 $count 件（報告のみ。ジョブは失敗させない）。"
    if [[ -n "${places:-}" ]]; then
      echo
      echo "規則と場所の重複を除くと ${places} か所（マージコミットの差分も走査するため、同じ値が複数のコミットで出る）。"
    fi
    echo
    echo "$header"
    echo "|---|---|---|"
    printf '%s\n' "$rows"
  fi
  if [[ "${errors:-0}" != 0 ]]; then
    echo
    echo "semgrep の解析エラー ${errors} 件（構文を解析できなかったファイルなど。詳細は $report）。"
  fi
} >>"$summary"

if [[ "$count" != 0 ]]; then
  echo "::warning::$tool: 検出 $count 件（報告のみ）。一覧はジョブサマリーにあります"
fi
exit 0
