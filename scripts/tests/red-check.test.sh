#!/usr/bin/env bash
# scripts/red-check のテスト。一時 git リポジトリに「前の実装」(ref old) と「今の作業ツリー」を作り、
# 重ねたファイル・重ねないファイル・終了コード・一時 worktree の後始末を確かめる。
set -uo pipefail

SCRIPT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/red-check"
failures=()
ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT

# 前の実装を ref old に、今の実装と新しい試験を作業ツリー（未コミット・未追跡を含む）に置く。
REPO="$ROOT/repo"
mkdir -p "$REPO/src" "$REPO/tests"
git -C "$REPO" init -q -b main
git -C "$REPO" config user.name test
git -C "$REPO" config user.email test@example.invalid
git -C "$REPO" config commit.gpgsign false
printf 'old-impl\n' >"$REPO/src/impl.txt"
printf 'old-test\n' >"$REPO/tests/check.txt"
git -C "$REPO" add -A
git -C "$REPO" commit -q -m old
git -C "$REPO" tag old
printf 'new-impl\n' >"$REPO/src/impl.txt"     # 重ねない: ref の版のままであるべき
printf 'new-test\n' >"$REPO/tests/check.txt"  # 重ねる: 追跡済みで変更あり
printf 'added-test\n' >"$REPO/tests/added.txt" # 重ねる: ref には無い未追跡のファイル

# run <name> <args...>: REPO で red-check を実行し、stdout を OUT、stdout+stderr を ALL、終了コードを RC に入れる。
run() {
  local name="$1"
  shift
  CASE="$name"
  OUT="$ROOT/$name.out"
  RC=0
  (cd "$REPO" && bash "$SCRIPT" "$@") >"$OUT" 2>"$ROOT/$name.err" || RC=$?
}

# 一時 worktree が残っていないこと（git の登録と実ディレクトリの両方）
expect_no_worktree() {
  local n
  n="$(git -C "$REPO" worktree list --porcelain | grep -c '^worktree ')"
  [[ "$n" == 1 ]] || failures+=("$CASE: $n worktrees registered after run, want 1")
  if [[ -s "$ROOT/$CASE.wt" ]]; then
    [[ ! -e "$(cat "$ROOT/$CASE.wt")" ]] || failures+=("$CASE: temporary worktree directory left behind")
  else
    failures+=("$CASE: command did not record its directory")
  fi
}

# 重ねたファイルが使われ、--keep に無いファイルは ref の版のまま。終了コード 0 を返す。
run keep_overlays_files old --keep tests/check.txt tests/added.txt -- \
  sh -c 'pwd >"$1"; cat src/impl.txt tests/check.txt tests/added.txt' _ "$ROOT/keep_overlays_files.wt"
[[ "$RC" == 0 ]] || failures+=("$CASE: exit $RC, want 0 ($(cat "$ROOT/$CASE.err"))")
grep -qxF new-test "$OUT" || failures+=("$CASE: kept tracked file not used")
grep -qxF added-test "$OUT" || failures+=("$CASE: kept file absent in ref not used")
grep -qxF old-impl "$OUT" || failures+=("$CASE: file not in --keep is not the ref version")
! grep -qxF new-impl "$OUT" || failures+=("$CASE: file not in --keep came from the working tree")
[[ "$(cat "$ROOT/$CASE.wt" 2>/dev/null)" != "$REPO" ]] || failures+=("$CASE: command ran in the working tree, not a temporary worktree")
expect_no_worktree

# コマンドの終了コードで終わる。失敗しても一時 worktree を消す。
run exit_code_passes_through old --keep tests/check.txt -- \
  sh -c 'pwd >"$1"; exit 7' _ "$ROOT/exit_code_passes_through.wt"
[[ "$RC" == 7 ]] || failures+=("$CASE: exit $RC, want 7")
expect_no_worktree

# --keep を渡さなければ何も重ねない（重ねるファイルを自動で選ばない）。
run no_keep_overlays_nothing old -- \
  sh -c 'pwd >"$1"; cat src/impl.txt tests/check.txt; test ! -e tests/added.txt && echo no-added' _ "$ROOT/no_keep_overlays_nothing.wt"
[[ "$RC" == 0 ]] || failures+=("$CASE: exit $RC, want 0 ($(cat "$ROOT/$CASE.err"))")
grep -qxF old-test "$OUT" || failures+=("$CASE: changed test file was overlaid without --keep")
grep -qxF no-added "$OUT" || failures+=("$CASE: untracked file was overlaid without --keep")
expect_no_worktree

# 引数の誤り（-- が無い）は終了コード 2 で、worktree を作らない。
run missing_separator old --keep tests/check.txt
[[ "$RC" == 2 ]] || failures+=("$CASE: exit $RC, want 2")
n="$(git -C "$REPO" worktree list --porcelain | grep -c '^worktree ')"
[[ "$n" == 1 ]] || failures+=("$CASE: $n worktrees registered, want 1")

# 作業ツリーに無いパスを --keep に渡すと終了コード 2 で、コマンドを走らせない。
run keep_missing_path old --keep tests/nope.txt -- sh -c 'echo ran'
[[ "$RC" == 2 ]] || failures+=("$CASE: exit $RC, want 2")
! grep -qxF ran "$OUT" || failures+=("$CASE: command ran despite a missing --keep path")
n="$(git -C "$REPO" worktree list --porcelain | grep -c '^worktree ')"
[[ "$n" == 1 ]] || failures+=("$CASE: $n worktrees registered, want 1")

if ((${#failures[@]})); then
  printf 'FAIL %s\n' "${failures[@]}"
  exit 1
fi
echo ok
