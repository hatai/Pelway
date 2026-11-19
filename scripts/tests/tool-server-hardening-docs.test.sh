#!/usr/bin/env bash
# Issue #199 のツール実行系の堅牢化チェックリスト（docs/technical.md §4 の節）が
# 落ちているかの構造検査（節の実在・4領域の見出し・#183 との対応付け・出典・対象外の記録）。
# チェックリストの内容（観点の過不足・対応付けの妥当性）の成否は Issue #199 の受け入れ条件
# どおり目視確認と skill review-pr のレビューが担う。自然言語の意味をキーワード一致や
# 正規表現で採点することは AGENTS.md の「LLM judges」が禁じるため、このテストでは行わない。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
failures=()

# docs/technical.md から「### ツール実行系の堅牢化チェックリスト」節（次の ## 見出しの
# 前まで）を切り出す。4領域の見出しと対応表がこの節の中にあることを確かめるため。
section="$(awk '
  /^### ツール実行系の堅牢化チェックリスト/ {in_section=1; next}
  in_section && /^## / {in_section=0}
  in_section {print}
' "$ROOT/docs/technical.md")"

[[ -n "$section" ]] ||
  failures+=("section: docs/technical.md に「### ツール実行系の堅牢化チェックリスト」節が無い")

# 条件1: チェックリストの4領域（プロセスの終了保証・出力・履歴の上限・タイムアウトと
# キャンセルの扱い・作業ディレクトリの解決規則）が節内に見出しとして文書化されている。
for area in 'プロセスの終了保証' '出力・履歴の上限' 'タイムアウトとキャンセルの扱い' '作業ディレクトリの解決規則'; do
  grep -qF "$area" <<<"$section" ||
    failures+=("cond1_area: チェックリスト節内に見出しが無い: $area")
done

# 条件2: 各項目に対応するテスト観点が #183 の分類と対応付けられている
# （#183 の本文が挙げるリスク単位への参照が節内にあること）。
grep -qF '#183' <<<"$section" ||
  failures+=("cond2_183: チェックリスト節内に #183 の分類への対応付けが無い")

# 出典の明記（採用する方式が Buzz の VISION_AGENT.md / buzz-dev-mcp 由来であること）。
grep -qF 'VISION_AGENT.md' <<<"$section" ||
  failures+=("source_vision_agent: チェックリスト節内に VISION_AGENT.md への出典が無い")
grep -qF 'buzz-dev-mcp' <<<"$section" ||
  failures+=("source_buzz_dev_mcp: チェックリスト節内に buzz-dev-mcp への言及が無い")

# 対象外の記録（サンドボックス基盤そのものは #184。実装も対象外）。
grep -qF '#184' <<<"$section" ||
  failures+=("out_of_scope_184: チェックリスト節内に #184（対象外）の記録が無い")

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "tool-server-hardening-docs: ok"
