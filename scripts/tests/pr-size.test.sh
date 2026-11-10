#!/usr/bin/env bash
# scripts/pr-size.sh のテスト。一時 git リポジトリで base と HEAD を作り、数えた行数と終了コードを確かめる。
set -uo pipefail

SCRIPT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/pr-size.sh"
failures=()
ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT

# 新しい一時リポジトリを作り、空の base コミットを置く。リポジトリのパスを出力する。
new_repo() {
  local dir
  dir="$(mktemp -d -p "$ROOT")"
  git -C "$dir" init -q -b main
  git -C "$dir" config user.name test
  git -C "$dir" config user.email test@example.invalid
  git -C "$dir" config commit.gpgsign false
  git -C "$dir" commit -q --allow-empty -m base
  git -C "$dir" tag base
  printf '%s\n' "$dir"
}

lines() { seq 1 "$1"; }

commit_all() { git -C "$1" add -A && git -C "$1" commit -q -m change; }

# run_case <name> <dir> <expected-exit> <expected-first-line or empty> [script args...]
run_case() {
  local name="$1" dir="$2" want_rc="$3" want_line="$4"
  shift 4
  local out rc
  out="$(cd "$dir" && bash "$SCRIPT" "$@" 2>/dev/null)"
  rc=$?
  if [[ "$rc" != "$want_rc" ]]; then
    failures+=("$name: exit $rc, want $want_rc")
    return
  fi
  if [[ -n "$want_line" && "$(head -n1 <<<"$out")" != "$want_line" ]]; then
    failures+=("$name: got '$(head -n1 <<<"$out")', want '$want_line'")
  fi
}

d="$(new_repo)"; lines 10 >"$d/a.rs"; commit_all "$d"
run_case under_limit "$d" 0 "pr-size: 10 / 800 lines" --base base --limit 800

d="$(new_repo)"; lines 801 >"$d/a.rs"; commit_all "$d"
run_case over_limit "$d" 1 "pr-size: 801 / 800 lines" --base base --limit 800

d="$(new_repo)"; lines 900 >"$d/tokens.json"; printf 'tokens.json linguist-generated=true\n' >"$d/.gitattributes"; lines 5 >"$d/a.rs"; commit_all "$d"
run_case generated_excluded "$d" 0 "pr-size: 6 / 800 lines" --base base --limit 800

d="$(new_repo)"; printf '\x00\x01\x02binary' >"$d/blob.bin"; commit_all "$d"
run_case binary_ignored "$d" 0 "pr-size: 0 / 800 lines" --base base --limit 800

d="$(new_repo)"; lines 3 >"$d/my file.rs"; commit_all "$d"
run_case space_in_name "$d" 0 "pr-size: 3 / 800 lines" --base base --limit 800

d="$(new_repo)"; lines 100 >"$d/old.rs"; commit_all "$d"; git -C "$d" tag -f base >/dev/null
git -C "$d" mv old.rs new.rs; sed -i '50s/.*/changed/' "$d/new.rs"; commit_all "$d"
run_case rename_counts_changes_only "$d" 0 "pr-size: 2 / 800 lines" --base base --limit 800

d="$(new_repo)"
run_case bad_args "$d" 2 "" --base base --limit abc

if ((${#failures[@]})); then
  printf 'FAIL %s\n' "${failures[@]}"
  exit 1
fi
echo ok
