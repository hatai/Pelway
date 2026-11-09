# shellcheck shell=bash
# demo-check 共通定義: 走査範囲・除外・出力の決まり。各段階スクリプトから source する。
#
# 走査範囲（存在するものだけ）:
#   製品ソース・資産 : crates/ assets/ ui/ web/ static/ locales/ i18n/ dist/
#   外向け文書       : docs/public/（docs/ のうち、ここだけは利用者・社外に出す文書として走査する）
#   ビルド成果物     : target/demo/ 配下すべて、
#                      target/wasm32-unknown-unknown/{debug,release}/ 直下の *.wasm *.js *.html *.css
#                      （deps/ build/ incremental/ などの中間物は対象外）
# 除外（理由）:
#   docs/（docs/public/ を除く） … 内部の仕様・計画文書。禁止語を「禁止語として」記載するため
#   scripts/demo-check/   … 検査そのもの（禁止語・許可ホスト・カナリアのリストを含む）
#   README.md AGENTS.md mise.toml mise.lock hk.pkl fnox.toml など直下の開発ツール設定
#                         … 製品に同梱されない（mise.toml の PELWAY_BIND_ADDR だけは loopback 段階が個別に見る）
#   */tests/fixtures/* */testdata/* … 違反を意図的に含むテスト用データ
#   .git/
# シンボリックリンク: find に -L を付けず、走査の途中で出会ったリンクはたどらない
#   （リンク先が走査ルートの外でも読まない。ルート自体がリンクのときは各段階が realpath で実体を走査する）
# ファイル名の区切り: 改行を含むファイル名でも壊れないよう、ファイル一覧は NUL 区切りで渡す
# 出力: 違反は「ファイル:行: 規則」の形で出す。行の中身は出さない（秘密値を表示しないため）。
#       *.wasm はバイナリのため「ファイル:(binary): 規則 (N件)」とする。
# 終了コード: 0 = 合格、1 = 違反あり、2 = 検査自体の異常（リスト欠落・走査失敗など）
set -uo pipefail
export LC_ALL=C.UTF-8

DC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DC_ROOT="${DEMO_CHECK_ROOT:-$(cd "${DC_DIR}/../.." && pwd)}"
DC_LIST_DIR="${DEMO_CHECK_LIST_DIR:-${DC_DIR}}"

DC_SOURCE_DIRS=(crates assets ui web static locales i18n dist docs/public)

dc_is_excluded() {
  case "$1" in
    docs/public/*/tests/fixtures/* | docs/public/*/testdata/*) return 0 ;;
    docs/public/*) return 1 ;;
  esac
  case "$1" in
    docs/* | scripts/demo-check/* | .git/* | */tests/fixtures/* | tests/fixtures/* | */testdata/* | testdata/*) return 0 ;;
  esac
  return 1
}

# 走査対象のファイル（ルートからの相対パス、NUL 区切りで 1 件 1 エントリ）。テキストもバイナリも含む。
# find などの走査に失敗したら戻り値を非 0 にする（呼び出し側は exit 2 にする）。
dc_all_files() {
  local d st=0
  (
    cd "${DC_ROOT}" || exit 2
    for d in "${DC_SOURCE_DIRS[@]}" target/demo; do
      if [ -d "${d}" ]; then
        find "${d}" -type f -print0 || st=2
      fi
    done
    for d in target/wasm32-unknown-unknown/debug target/wasm32-unknown-unknown/release; do
      if [ -d "${d}" ]; then
        find "${d}" -maxdepth 1 -type f \( -name "*.wasm" -o -name "*.js" -o -name "*.html" -o -name "*.css" \) -print0 || st=2
      fi
    done
    exit "${st}"
  ) | while IFS= read -r -d '' f; do
    dc_is_excluded "${f}" || printf "%s\0" "${f}"
  done | sort -z -u
}

# dc_all_files の結果を配列に読み込む。走査に失敗したら exit 2 する（DC-05）。
dc_load_files() { # dc_load_files <配列名>
  local __dst="$1" __tmp
  __tmp="$(mktemp)" || exit 2
  if ! dc_all_files > "${__tmp}"; then
    rm -f "${__tmp}"
    echo "demo-check: file scan failed under ${DC_ROOT}" >&2
    exit 2
  fi
  mapfile -d '' -t "${__dst}" < "${__tmp}"
  rm -f "${__tmp}"
}

# UI 文言を含みうるファイル（行全体を見る）: 資産・UI 系ディレクトリと UI 系拡張子
dc_is_copy_file() {
  case "$1" in
    assets/* | ui/* | web/* | static/* | locales/* | i18n/* | dist/* | docs/public/* | target/*) return 0 ;;
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
