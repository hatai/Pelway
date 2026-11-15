#!/usr/bin/env bash
# scripts/script-test.sh（script-test タスクの収集と実行）のテスト。
# 一時ディレクトリの合成ツリーで、収集対象（scripts/tests・スキル同梱・demo-check の自己テスト）、
# 収集対象外の除外、実行と失敗の集計、置いただけでの新規テストの収集を確かめる。
set -uo pipefail

SCRIPT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/script-test.sh"
failures=()
ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT

TREE="$ROOT/tree"
mkdir -p "$TREE/scripts/tests" "$TREE/scripts/demo-check" "$TREE/.agents/skills/foo"

# fake_test <ルートからの相対パス> <終了コード> <stdout へ出すマーカー>: 合成のテストを 1 つ足す。
fake_test() {
  local path="$TREE/$1" rc="$2" marker="$3"
  mkdir -p "$(dirname "$path")"
  printf '#!/usr/bin/env bash\necho %q\nexit %s\n' "$marker" "$rc" >"$path"
}

# run_case <name> <want-rc> <script args...>: 終了コードだけを確かめる。
run_case() {
  local name="$1" want_rc="$2"
  shift 2
  bash "$SCRIPT" "$@" >/dev/null 2>&1
  local rc=$?
  if [[ "$rc" != "$want_rc" ]]; then
    failures+=("$name: exit $rc, want $want_rc")
  fi
}

# 出力にマーカー（1 行）が含まれるか。
has_line() { grep -Fxq "$2" <<<"$1"; }

fake_test scripts/tests/aaa.test.sh 0 aaa-ok
fake_test scripts/tests/zzz.test.sh 0 zzz-ok
fake_test .agents/skills/foo/helper.test.sh 0 skill-ok
fake_test scripts/demo-check/selftest.sh 0 selftest-ok
# 収集対象外: .test.sh の付かないスクリプト、ヘルパー本体、スキル直下より深いテスト。
printf '#!/usr/bin/env bash\ntrue\n' >"$TREE/scripts/tests/helper.sh"
printf '#!/usr/bin/env bash\ntrue\n' >"$TREE/.agents/skills/foo/helper"
fake_test .agents/skills/foo/nested/inner.test.sh 0 nested-ok

# --list: 対象を過不足なく、バイト順（LC_ALL=C。'.'(0x2E) は 's'(0x73) より前）で 1 行 1 パスで出す。
expected_list=".agents/skills/foo/helper.test.sh
scripts/demo-check/selftest.sh
scripts/tests/aaa.test.sh
scripts/tests/zzz.test.sh"
out="$(bash "$SCRIPT" --list "$TREE" 2>/dev/null)"
if [[ "$out" != "$expected_list" ]]; then
  failures+=("list_exact: got '$(printf '%s' "$out" | tr '\n' '|')', want '$(printf '%s' "$expected_list" | tr '\n' '|')'")
fi

# 実行: 全テストが走り、全部通れば 0。
out="$(bash "$SCRIPT" "$TREE" 2>/dev/null)"; rc=$?
if [[ "$rc" != 0 ]]; then
  failures+=("run_all_pass: exit $rc, want 0")
fi
for marker in aaa-ok zzz-ok skill-ok selftest-ok; do
  has_line "$out" "$marker" || failures+=("run_all_pass: marker $marker missing")
done
has_line "$out" "nested-ok" && failures+=("run_all_pass: nested test must not run")
if ! grep -q 'script-test: 4 tests ok' <<<"$out"; then
  failures+=("run_all_pass: summary line missing")
fi

# 実行: 失敗があっても残りを走らせ、失敗を 1 つにまとめて報告して 1 で終わる。
fake_test scripts/tests/mmm.test.sh 1 ''
out="$(bash "$SCRIPT" "$TREE" 2>&1)"; rc=$?
if [[ "$rc" != 1 ]]; then
  failures+=("run_with_failure: exit $rc, want 1")
fi
for marker in aaa-ok skill-ok selftest-ok; do
  has_line "$out" "$marker" || failures+=("run_with_failure: marker $marker missing (残りのテストが走っていない)")
done
grep -Fxq 'FAIL scripts/tests/mmm.test.sh' <<<"$out" || failures+=("run_with_failure: 失敗テストのパスが報告に無い")
if ! grep -q 'script-test: 1/5 failed' <<<"$out"; then
  failures+=("run_with_failure: summary line missing")
fi

# テストを置くだけで収集に乗る（mise.toml の編集を要さないことの収集側の保証）。
fake_test scripts/tests/new.test.sh 0 new-ok
out="$(bash "$SCRIPT" --list "$TREE" 2>/dev/null)"
has_line "$out" "scripts/tests/new.test.sh" || failures+=("new_test_collected: scripts/tests/new.test.sh not listed")
out="$(bash "$SCRIPT" "$TREE" 2>/dev/null)"
has_line "$out" "new-ok" || failures+=("new_test_collected: new-ok marker missing")

# テストが 1 つも無いルートは収集不能として 2。
EMPTY="$ROOT/empty"
mkdir -p "$EMPTY"
run_case no_tests_found 2 --list "$EMPTY"

# check_help <name> <arg>: 使い方（usage: で始まる行）を標準出力に出し、終了コード 0 で終わる。
check_help() {
  local name="$1" arg="$2"
  local out rc
  out="$(bash "$SCRIPT" "$arg" 2>/dev/null)"
  rc=$?
  if [[ "$rc" != 0 ]]; then
    failures+=("$name: exit $rc, want 0")
    return
  fi
  if ! grep -q '^usage:' <<<"$out"; then
    failures+=("$name: stdout does not contain a 'usage:' line")
  fi
}

check_help help_long --help
check_help help_short -h

if ((${#failures[@]})); then
  printf 'FAIL %s\n' "${failures[@]}"
  exit 1
fi
echo ok
