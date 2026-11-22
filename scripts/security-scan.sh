#!/usr/bin/env bash
# security-scan workflow（.github/workflows/security-scan.yml）の 1 ツール分の本体（Issue #571）。
# 呼び出しは `bash scripts/security-scan.sh <gitleaks|semgrep|osv-scanner>`。ツールは PATH 上のものを使う。
#
# 当面は報告のみ: 検出があっても終了コード 0 で終わり、::warning:: とジョブサマリー
# （$GITHUB_STEP_SUMMARY。無ければ標準出力）に件数と一覧を出す。ツール自体の失敗は
# ジョブを失敗させる（検出と取り違えない）。各ツールの終了コードの意味:
#   gitleaks    0 = 検出なし、2 = 検出あり（--exit-code 2 で指定）、それ以外 = 失敗
#   semgrep     0 = 検出なし、1 = 検出あり（--error）、それ以外 = 失敗
#   osv-scanner 0 = 検出なし、1 = 検出あり、128 = ロックファイルが無い、それ以外 = 失敗
#
# ツールの診断（標準エラー）の決まり。3 つのツールとも、診断には PR の作者が決められる値
# （設定の行・パス・取得先の URL など）が入りうり、失敗が終了コードに出ないことがある:
#   - 標準エラーは公開のジョブログに出さず、$out_dir/<tool>.stderr にだけ書く。ログには
#     失敗の種類と行数だけを出す（中身は手元で同じコマンドを実行して確かめる）
#   - gitleaks と osv-scanner はログの水準を error に絞り、標準エラーに 1 行でもあれば、
#     終了コードが検出なし・検出ありでも失敗にする（gitleaks 8.30.1 は git log の失敗を、
#     osv-scanner 2.7.1 は抽出の失敗を、ログに出すだけで終了コードに出さない）
#   - semgrep は解析エラーを JSON の errors に書くため、level が error のものがあれば失敗にする
#     （semgrep は検出の終了コード 1 をエラーより優先して返す）
#
# 秘密の値は出さない: gitleaks は --redact で値を伏せ、サマリーにも規則・場所・コミットだけを書く
# （レポートの Secret / Match は読まない）。
# 結果の JSON は $SECURITY_SCAN_OUT（既定: リポジトリ外の一時ディレクトリ）に置く。
# 終了コード: 0 = 検出なし・検出あり（報告のみ）、それ以外 = 失敗。2 = 引数の誤り、3 = 終了コードに
# 出ないツールの失敗（標準エラーの診断・semgrep の解析エラー）、それ以外はツール自身の終了コードを
# そのまま返す（ツールの終了コードが 2・3 と重なることがあるため、どの失敗かはログの ::error:: で見る）
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
errfile="$out_dir/$tool.stderr"
summary="${GITHUB_STEP_SUMMARY:-/dev/stdout}"
local_hint="中身はログに出さない。手元で SECURITY_SCAN_OUT=<dir> bash scripts/security-scan.sh $tool を実行し、<dir>/$tool.stderr を読む"

# Semgrep のルールセット（Semgrep Registry から取得。ログイン・トークンは使わない）。
# p/rust: Rust、p/github-actions: CI 設定、r/bash: シェル（p/bash は Registry に無い）。
semgrep_configs=(--config p/rust --config p/github-actions --config r/bash)

rc=0
case "$tool" in
  gitleaks)
    # HEAD から辿れる履歴だけを走査する（checkout の他ブランチやローカルの ref は対象外）。
    # -m: git log は既定でマージコミットの差分を出さないため、競合解消で足した行が漏れる。
    # 各親との差分を出させて走査する（同じ値が複数のコミットで重ねて出る）。
    gitleaks git --redact --no-banner --log-level error --exit-code 2 --log-opts="-m HEAD" \
      --report-format json --report-path "$report" . 2>"$errfile" || rc=$?
    found_rc=2
    ;;
  semgrep)
    semgrep scan --metrics=off --error "${semgrep_configs[@]}" \
      --json --output "$report" . 2>"$errfile" || rc=$?
    found_rc=1
    ;;
  osv-scanner)
    # --verbosity error: 既定（info）の進捗ログは読めたファイルのパスを書くため、エラーだけにする。
    # --no-call-analysis=all: 呼び出し解析（Go は既定で有効）は使わない。
    # --no-resolve: マニフェスト（pom.xml など）の推移的な依存を、PR の作者が指定できる
    # レジストリへ取りに行かない。
    # --x-no-default-plugins --x-plugins rust/cargolock: 抽出器は Cargo.lock 用だけにする。既定の
    # 抽出器（同梱の C/C++ ソースのパスとハッシュを OSV.dev に送る directory など）も、lockfile
    # プリセット（pom.xml・go.mod などのマニフェストの抽出器を含む）も使わない。PR の作者が足した
    # マニフェストの依存を OSV.dev に照会しないため。別のエコシステムのロックファイルを入れるときは、
    # その抽出器の名前をここに足す。
    osv-scanner scan source --verbosity error --no-call-analysis=all --no-resolve \
      --x-no-default-plugins --x-plugins rust/cargolock \
      --recursive --format json --output-file "$report" . 2>"$errfile" || rc=$?
    found_rc=1
    ;;
esac

if [[ "$rc" != 0 && "$rc" != "$found_rc" ]]; then
  echo "::error::$tool が失敗しました（終了コード $rc。標準エラーの${local_hint}）"
  exit "$rc"
fi

if [[ "$tool" != semgrep && -s "$errfile" ]]; then
  echo "::error::$tool が標準エラーにエラーを出しました（$(wc -l <"$errfile") 行。終了コード $rc でも失敗として扱う。${local_hint}）"
  exit 3
fi

if [[ "$tool" == semgrep ]]; then
  fatal="$(jq '[.errors[]? | select(.level == "error")] | length' "$report")"
  if [[ "$fatal" != 0 ]]; then
    echo "::error::semgrep が失敗しました（level が error の解析エラー ${fatal} 件。${local_hint}）"
    exit 3
  fi
fi

# 検出の一覧（Markdown の表の行）と件数を作る。件数は JSON の検出の数から数える
# （ファイル名などに改行があっても件数が増えないように）。
# パスなどは PR の作者が決められるため、表のセルはコードスパン（`...`）にして、リンク・装飾・
# HTML・メールアドレスの自動リンクとして解釈させない。囲む ` の数は値の中の ` の最長の並びより
# 1 つ多くし、改行は空白に、| は \| にする（GFM の表ではコードスパンの中でも \| が | になる）。
# shellcheck disable=SC2016 # jq の式（$n など）をそのまま渡す
cell='def cell: tostring | gsub("[\\r\\n]"; " ") | gsub("\\|"; "\\|")
  | ([match("`+"; "g").length] | max // 0) as $n | ("`" * ($n + 1)) as $f | "\($f) \(.) \($f)";'
case "$tool" in
  gitleaks)
    header='| 規則 | 場所 | コミット |'
    count="$(jq 'length' "$report")"
    rows_json="$(jq "$cell"' [.[] | "| \(.RuleID | cell) | \("\(.File):\(.StartLine)" | cell) | \(.Commit[0:12] | cell) |"]' "$report")"
    places="$(jq '[.[] | "\(.RuleID) \(.File):\(.StartLine)"] | unique | length' "$report")"
    ;;
  semgrep)
    header='| 規則 | 場所 | 重大度 |'
    count="$(jq '.results | length' "$report")"
    rows_json="$(jq "$cell"' [.results[] | "| \(.check_id | cell) | \("\(.path):\(.start.line)" | cell) | \(.extra.severity | cell) |"]' "$report")"
    errors="$(jq '.errors | length' "$report")"
    ;;
  osv-scanner)
    header='| パッケージ | 勧告 | ロックファイル |'
    count="$(jq '[.results[]?.packages[]?] | length' "$report")"
    rows_json="$(jq "$cell"' [.results[]? | .source.path as $p | .packages[]
      | "| \("\(.package.name) \(.package.version)" | cell) | \([.vulnerabilities[].id] | join(", ") | cell) | \($p | cell) |"]' "$report")"
    ;;
esac

# ジョブサマリーは 1 ステップ 1 MiB を超えると丸ごと載らない。一覧の行は UTF-8 のバイト数の
# 合計が row_budget に収まる先頭の行までにする（見出しや注記の分の余裕を残す）。
row_budget="${SECURITY_SCAN_ROW_BUDGET:-900000}"
rows="$(jq -r --argjson b "$row_budget" '
  reduce .[] as $r ({n: 0, out: [], full: false};
    if .full then . elif .n + ($r | utf8bytelength) + 1 > $b then .full = true
    else .n += ($r | utf8bytelength) + 1 | .out += [$r] end)
  | .out[]' <<<"${rows_json:-[]}")"
shown=0
if [[ -n "$rows" ]]; then shown="$(printf '%s\n' "$rows" | wc -l)"; fi

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
    if ((shown > 0)); then printf '%s\n' "$rows"; fi
    if ((count > shown)); then
      echo
      echo "一覧は先頭 ${shown} 件だけ載せた（残り $((count - shown)) 件は、ジョブサマリーの大きさの上限のため省略。全件は手元で同じコマンドを実行して確かめる）。"
    fi
  fi
  if [[ "${errors:-0}" != 0 ]]; then
    echo
    echo "semgrep の解析エラー ${errors} 件（構文を解析できなかったファイルなど。詳細は手元で同じコマンドを実行して確かめる）。"
  fi
} >>"$summary"

if [[ "$count" != 0 ]]; then
  echo "::warning::$tool: 検出 $count 件（報告のみ）。一覧はジョブサマリーにあります"
fi
exit 0
