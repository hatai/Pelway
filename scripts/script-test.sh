#!/usr/bin/env bash
# mise run script-test の本体。テストの列挙を glob 収集で行い、テスト追加のたびに
# mise.toml の同一行を編集して衝突するのを防ぐ（Issue #342）。
# 収集対象（いずれもルートからの相対パス）:
#   - scripts/tests/*.test.sh
#   - .agents/skills/<name>/*.test.sh（スキル同梱テスト。.claude/skills/<name> は
#     .agents/skills/<name> への symlink なため、.agents 側だけを収集して二重実行を避ける）
#   - scripts/demo-check/selftest.sh（glob で拾えないため明示）
# 使い方: scripts/script-test.sh [--list] [<root>]
# 終了コード: 0 = 全テスト合格、1 = 失敗したテストあり、2 = 引数の誤りまたは収集不能
set -uo pipefail

usage_text="usage: $0 [--list] [<root>]"

usage() { echo "$usage_text" >&2; exit 2; }

help() {
  cat <<EOF
$usage_text

script-test タスクの本体。テストを収集して順に実行する。失敗しても残りのテストを
走らせ、最後に失敗をまとめて報告する。

収集対象:
  scripts/tests/*.test.sh          scripts/ のシェルスクリプトのテスト
  .agents/skills/<name>/*.test.sh  スキル同梱テスト
  scripts/demo-check/selftest.sh   demo-check の自己テスト（明示）

テストを追加するときは上記の置き場に *.test.sh を置くだけでよく、mise.toml の編集は不要。

options:
  --list     収集したテストの相対パスを 1 行 1 つで出力して終了する（実行しない）
  <root>     収集と実行のルート（既定: カレントディレクトリ）
  -h, --help この使い方を標準出力に出して終了する

終了コード: 0 = 全テスト合格、1 = 失敗したテストあり、2 = 引数の誤りまたは収集不能
EOF
  exit 0
}

root=""
list_only=0
while (($#)); do
  case "$1" in
    -h|--help) help ;;
    --list) list_only=1; shift ;;
    -*) usage ;;
    *) [[ -z "$root" ]] || usage; root="$1"; shift ;;
  esac
done

cd "${root:-.}" || { echo "script-test: ルート $root に移動できない" >&2; exit 2; }

# 収集したテストの相対パスを 1 行 1 つ、バイト順（LC_ALL=C）で出す。
# 実行順序に依存するテストは無い（テストはそれぞれ一時ディレクトリの中で閉じて動く。
# docs/agents/skill-helpers.md「テストの置き場と実行方法」）。並びは収集結果の
# 再現性のためにだけ決める。
collect() {
  local t
  local -a found=()
  shopt -s nullglob
  for t in scripts/tests/*.test.sh .agents/skills/*/*.test.sh; do
    found+=("$t")
  done
  shopt -u nullglob
  if [[ ! -f scripts/demo-check/selftest.sh ]]; then
    echo "script-test: scripts/demo-check/selftest.sh が無い（収集対象の置き場を確かめる）" >&2
    return 1
  fi
  found+=("scripts/demo-check/selftest.sh")
  ((${#found[@]})) || { echo "script-test: テストが 1 つも見つからない" >&2; return 1; }
  printf '%s\n' "${found[@]}" | LC_ALL=C sort
}

collected="$(collect)" || exit 2
mapfile -t tests <<<"$collected"

if ((list_only)); then
  printf '%s\n' "${tests[@]}"
  exit 0
fi

failures=()
for t in "${tests[@]}"; do
  echo "=== $t ==="
  if ! bash "$t"; then
    echo "FAIL $t" >&2
    failures+=("$t")
  fi
done

if ((${#failures[@]})); then
  printf 'script-test: %s/%s failed\n' "${#failures[@]}" "${#tests[@]}" >&2
  printf '  %s\n' "${failures[@]}" >&2
  exit 1
fi
printf 'script-test: %s tests ok\n' "${#tests[@]}"
