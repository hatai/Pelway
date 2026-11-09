# shellcheck shell=bash
# demo-check 共通定義: 走査範囲・除外・出力の決まり。各段階スクリプトから source する。
#
# 走査範囲（存在するものだけ）:
#   製品ソース・資産 : crates/ assets/ ui/ web/ static/ locales/ i18n/ dist/
#   ビルド成果物     : target/demo/ 配下すべて、
#                      target/wasm32-unknown-unknown/{debug,release}/ 直下の *.wasm *.js *.html *.css
#                      （deps/ build/ incremental/ などの中間物は対象外）
# 除外（理由）:
#   docs/                 … 仕様・計画文書。禁止語を「禁止語として」記載するため
#   scripts/demo-check/   … 検査そのもの（禁止語・許可ホストのリストを含む）
#   README.md AGENTS.md mise.toml mise.lock hk.pkl fnox.toml など直下の開発ツール設定
#                         … 製品に同梱されない（mise.toml の PELWAY_BIND_ADDR だけは loopback 段階が個別に見る）
#   */tests/fixtures/* */testdata/* … 違反を意図的に含むテスト用データ
#   .git/
# 出力: 違反は「ファイル:行: 規則」の形で出す。行の中身は出さない（秘密値を表示しないため）。
#       *.wasm はバイナリのため「ファイル:(binary): 規則 (N件)」とする。
# 終了コード: 0 = 合格、1 = 違反あり、2 = 検査自体の異常（リスト欠落など）
set -u
export LC_ALL=C.UTF-8

DC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DC_ROOT="${DEMO_CHECK_ROOT:-$(cd "${DC_DIR}/../.." && pwd)}"
DC_LIST_DIR="${DEMO_CHECK_LIST_DIR:-${DC_DIR}}"

DC_SOURCE_DIRS=(crates assets ui web static locales i18n dist)

dc_is_excluded() {
  case "$1" in
    docs/* | scripts/demo-check/* | .git/* | */tests/fixtures/* | tests/fixtures/* | */testdata/* | testdata/*) return 0 ;;
  esac
  return 1
}

# 走査対象のファイル（ルートからの相対パス、1行1件）。テキストもバイナリも含む。
dc_all_files() {
  (
    cd "${DC_ROOT}" || exit 2
    local d
    for d in "${DC_SOURCE_DIRS[@]}" target/demo; do
      [ -d "${d}" ] && find -L "${d}" -type f
    done
    for d in target/wasm32-unknown-unknown/debug target/wasm32-unknown-unknown/release; do
      [ -d "${d}" ] && find -L "${d}" -maxdepth 1 -type f \( -name "*.wasm" -o -name "*.js" -o -name "*.html" -o -name "*.css" \)
    done
  ) | while IFS= read -r f; do
    dc_is_excluded "${f}" || printf "%s\n" "${f}"
  done | sort -u
}

# UI 文言を含みうるファイル（行全体を見る）: 資産・UI 系ディレクトリと UI 系拡張子
dc_is_copy_file() {
  case "$1" in
    assets/* | ui/* | web/* | static/* | locales/* | i18n/* | dist/* | target/*) return 0 ;;
    *.html | *.htm | *.css | *.js | *.mjs | *.ts | *.svg | *.ftl | *.po | *.json | *.yaml | *.yml | *.md | *.txt) return 0 ;;
  esac
  return 1
}

dc_is_wasm() { case "$1" in *.wasm) return 0 ;; esac; return 1; }

# バイナリ判定（grep -I でテキスト扱いにならないもの）
dc_is_text() { [ ! -s "${DC_ROOT}/$1" ] || grep -qI . "${DC_ROOT}/$1"; }

# リストファイルの有効行（コメント・空行を除く）
dc_list_lines() {
  local f="$1"
  [ -f "${f}" ] || { echo "demo-check: list file missing: ${f}" >&2; return 2; }
  grep -vE '^[[:space:]]*(#|$)' "${f}"
}
