#!/usr/bin/env bash
# cargo deny check advisories の出力（人間可読形式）を stdin で受け、unmaintained /
# unsound の advisory を警告行として標準出力に出す。終了コードは常に 0（この抽出は
# 警告の表示であって検査の成否では無いため）。mise.toml の deny-check タスク（第2段）
# が使う。見つからないときは何も出さない。
#
# 出力（見つかったとき）:
#   warning: unmaintained / unsound の advisory があります（検査は失敗しません）:
#   warning: <RUSTSEC-ID> <cargo-deny の診断の1行目（error[unmaintained]: … など）>
#
# advisory の ID はフィールド番号（awk の $n）で取り出さない。cargo-deny 0.20.2 の
# 人間可読出力では ID 行が「    ├ ID: RUSTSEC-…」のように樹形図の接頭辞付きで現れ、
# 何番目のフィールドに ID が来るかは接頭辞の付け方で変わる（接頭辞が付いた実際の
# 出力では $3、接頭辞を数えない読みでは $2 になる）。正規表現で ID そのものを抜く
# ことで出力形式の細部に依存しない（match は POSIX awk。gawk・mawk 両方で動く）。
# なお診断内の Advisory URL 行（https://rustsec.org/advisories/RUSTSEC-…）も同じ ID
# を含むため、ID 行より先に現れても結果は同じ。
#
# テスト: scripts/tests/deny-warn-extract.test.sh（cargo-deny の実際の出力を写した
# 固定サンプルで確かめる。mise の script-test タスク経由で verify に含まれる）。
set -uo pipefail

warnings="$(awk '
  /^error\[(unmaintained|unsound)\]:/ {
    title = $0
    keep = 1
    next
  }
  keep && match($0, /RUSTSEC-[0-9][0-9][0-9][0-9]-[0-9]+/) {
    print "warning: " substr($0, RSTART, RLENGTH) " " title
    keep = 0
  }
')"

if [ -n "$warnings" ]; then
  echo "warning: unmaintained / unsound の advisory があります（検査は失敗しません）:"
  printf '%s\n' "$warnings"
fi
exit 0
