#!/usr/bin/env bash
# evals/agent-team/ のデータセット構造の検査（Issue #220）。
# 各ケースが形式（README・case.toml・instruction.md・environment/・tests/verify.sh）を
# 満たすこと、case.toml が由来インシデントへの参照を持つこと、検証スクリプトが
# 同梱の pass/fail フィクスチャで実行できることを確かめる。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DATASET="$ROOT/evals/agent-team"
failures=()

fail() { failures+=("$1"); }

# --- データセット本体 ---
[[ -d "$DATASET" ]] || fail "evals/agent-team/ が無い"
[[ -f "$DATASET/README.md" ]] || fail "evals/agent-team/README.md が無い"
if [[ -f "$DATASET/README.md" ]]; then
  grep -q "ケース化" "$DATASET/README.md" \
    || fail "README.md にインシデントをケース化する手順の節が無い（「ケース化」を含むこと）"
fi

# --- ケースごとの構造 ---
shopt -s nullglob
cases=("$DATASET"/*/)
if [[ ${#cases[@]} -eq 0 ]]; then
  fail "evals/agent-team/ にケースが 1 件も無い"
fi

for dir in "${cases[@]}"; do
  name="$(basename "$dir")"

  for f in README.md case.toml instruction.md; do
    [[ -f "$dir$f" ]] || fail "$name: $f が無い"
  done
  [[ -d "$dir/environment" ]] || fail "$name: environment/ が無い"
  if [[ -d "$dir/environment" ]]; then
    [[ -n "$(ls -A "$dir/environment")" ]] || fail "$name: environment/ が空"
  fi
  [[ -x "$dir/tests/verify.sh" ]] || fail "$name: tests/verify.sh が無いか実行可能でない"

  # case.toml の必須キー
  if [[ -f "$dir/case.toml" ]]; then
    grep -qE '^name *= *"' "$dir/case.toml" || fail "$name: case.toml に name が無い"
    grep -qE '^incident *= *"' "$dir/case.toml" || fail "$name: case.toml に由来インシデント（incident）が無い"
    grep -qE '^judge *= *"(deterministic|llm)"' "$dir/case.toml" \
      || fail "$name: case.toml の judge は deterministic か llm"
  fi

  # 検証スクリプトをフィクスチャで実行する
  if [[ -x "$dir/tests/verify.sh" ]]; then
    pass_fx=("$dir"/tests/fixtures/pass*)
    fail_fx=("$dir"/tests/fixtures/fail*)
    if [[ ${#pass_fx[@]} -eq 0 || ${#fail_fx[@]} -eq 0 ]]; then
      fail "$name: tests/fixtures/ に pass* と fail* のフィクスチャが要る"
    else
      for fx in "${pass_fx[@]}"; do
        if ! bash "$dir/tests/verify.sh" "$fx" >/dev/null 2>&1; then
          fail "$name: verify.sh が pass フィクスチャ $(basename "$fx") を不合格にした"
        fi
      done
      for fx in "${fail_fx[@]}"; do
        if bash "$dir/tests/verify.sh" "$fx" >/dev/null 2>&1; then
          fail "$name: verify.sh が fail フィクスチャ $(basename "$fx") を合格にした"
        fi
      done
    fi
  fi
done

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "agent-team-evals: ok（ケース ${#cases[@]} 件）"
