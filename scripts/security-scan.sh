#!/usr/bin/env bash
# security-scan workflow（.github/workflows/security-scan.yml）の 1 ツール分の本体（Issue #571）。
# 呼び出しは `bash scripts/security-scan.sh <gitleaks|semgrep|osv-scanner>`。ツールは PATH 上のものを使う。
#
# 当面は報告のみ: 検出があっても終了コード 0 で終わり、::warning:: とジョブサマリー
# （$GITHUB_STEP_SUMMARY。無ければ標準出力）に件数と一覧を出す。ツール自体の失敗は、
# ツールの終了コードのまま失敗させる（検出と取り違えない）。各ツールの終了コードの意味:
#   gitleaks    0 = 検出なし、2 = 検出あり（--exit-code 2 で指定）、それ以外 = 失敗
#   semgrep     0 = 検出なし、1 = 検出あり（--error）、それ以外 = 失敗
#   osv-scanner 0 = 検出なし、1 = 検出あり、128 = ロックファイルが無い、それ以外 = 失敗。
#               ただし 2.7.1 はロックファイルの抽出の失敗を標準エラーの
#               「Error during extraction:」の行に出すだけで、検出があれば 1 を返すため、
#               その行があれば検出があっても失敗にする
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
    # 標準エラーは公開のジョブログに出さず、ファイルにだけ書く（取得先の URL などを含みうるため）。
    # 抽出の失敗は、osv-scanner が終わってからこのファイルで見分ける。
    # --verbosity error: 既定（info）の進捗ログは読めたファイルのパスをそのまま書き、パスに改行が
    # あると「Error during extraction:」で始まる行を作れるため、エラーの行だけにする。
    # --no-call-analysis=all: 呼び出し解析（Go は既定で有効）は失敗を標準エラーに出すだけで、
    # 検出があれば 1 を返して見分けられないため、使わない（依存のバージョンの照合だけにする）。
    osv-scanner scan source --verbosity error --no-call-analysis=all --recursive --format json --output-file "$report" . \
      2>"$out_dir/osv-scanner.stderr" || rc=$?
    found_rc=1
    ;;
esac

if [[ "$rc" != 0 && "$rc" != "$found_rc" ]]; then
  echo "::error::$tool が失敗しました（終了コード $rc）"
  if [[ "$tool" == osv-scanner ]]; then
    echo "osv-scanner の標準エラーはログに出さず、$out_dir/osv-scanner.stderr に書いた。手元で同じコマンドを実行して確かめる。"
  fi
  exit "$rc"
fi

if [[ "$tool" == osv-scanner ]]; then
  extraction_errors="$(grep -c '^Error during extraction:' "$out_dir/osv-scanner.stderr" || true)"
  if [[ "$extraction_errors" != 0 ]]; then
    echo "::error::osv-scanner がロックファイルの抽出に失敗しました（抽出エラーの診断 ${extraction_errors} 件。中身はログに出さず、$out_dir/osv-scanner.stderr に書いた）"
    exit 127
  fi
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

# 検出の一覧（Markdown の表の行）と件数を作る。件数は JSON の検出の数から数える
# （ファイル名などに改行があっても件数が増えないように）。表のセルでは改行を空白にし、
# ASCII の記号をすべて \ でエスケープする。パスなどは PR の作者が決められるため、リンク・
# 装飾・HTML として解釈させず、元の文字のまま表示する（| のエスケープで列も崩れない）。
cell='def cell: tostring | gsub("[\\r\\n]"; " ") | gsub("(?<c>[!-/:-@\\[-`{-~])"; "\\\(.c)");'
# ジョブサマリーは 1 ステップ 1 MiB を超えると載らないため、一覧は先頭の max_rows 行までにする。
max_rows=500
case "$tool" in
  gitleaks)
    header='| 規則 | 場所 | コミット |'
    count="$(jq 'length' "$report")"
    rows="$(jq -r --argjson n "$max_rows" "$cell"' .[:$n][] | "| \(.RuleID | cell) | \("\(.File):\(.StartLine)" | cell) | \(.Commit[0:12] | cell) |"' "$report")"
    places="$(jq '[.[] | "\(.RuleID) \(.File):\(.StartLine)"] | unique | length' "$report")"
    ;;
  semgrep)
    header='| 規則 | 場所 | 重大度 |'
    count="$(jq '.results | length' "$report")"
    rows="$(jq -r --argjson n "$max_rows" "$cell"' .results[:$n][] | "| \(.check_id | cell) | \("\(.path):\(.start.line)" | cell) | \(.extra.severity | cell) |"' "$report")"
    errors="$(jq '.errors | length' "$report")"
    ;;
  osv-scanner)
    header='| パッケージ | 勧告 | ロックファイル |'
    count="$(jq '[.results[]?.packages[]?] | length' "$report")"
    rows="$(jq -r --argjson n "$max_rows" "$cell"' [.results[]? | .source.path as $p | .packages[]
      | "| \("\(.package.name) \(.package.version)" | cell) | \([.vulnerabilities[].id] | join(", ") | cell) | \($p | cell) |"][:$n][]' "$report")"
    ;;
esac

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
    if ((count > max_rows)); then
      echo
      echo "一覧は先頭 ${max_rows} 件だけ載せた（残り $((count - max_rows)) 件は省略。全件は $report）。"
    fi
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
