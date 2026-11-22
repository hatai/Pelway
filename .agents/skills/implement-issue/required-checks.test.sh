#!/usr/bin/env bash
# .agents/skills/implement-issue/required-checks のテスト。
# agent-gh をモックに置き換え、固定 JSON への応答で出力と終了コードを確かめる。
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HELPER="$HERE/required-checks"
failures=()
ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT

# ---- agent-gh のモック -----------------------------------------------------
# `dev <gh の引数...>` を受け、$RC_FIXTURE のファイルから応答する。
#   pr checks  : checks.json（無ければ失敗）。checks.rc があればその終了コードで、
#                checks.err があればその内容を標準エラーに出す（gh の終了コード 8 などの再現）
#   pr view    : pr-view.json（無ければ失敗）
#   issue view : issue-<番号>.json（無ければ失敗）
# --jq が付いた呼び出しは、その式をモック内で jq に通してから返す（gh と同じ扱い）。
mkdir -p "$ROOT/bin"
cat >"$ROOT/bin/agent-gh" <<'MOCK'
#!/usr/bin/env bash
set -uo pipefail
[ "${1:-}" = dev ] || { echo "mock: role must be dev" >&2; exit 2; }
shift
fix="${RC_FIXTURE:?}"
all="$*"
printf '%s\n' "$all" >>"$fix/.calls"

jq_expr=""
prev=""
for a in "$@"; do
  [ "$prev" = "--jq" ] && jq_expr="$a"
  prev="$a"
done
emit() {
  [ -f "$1" ] || { echo "mock: HTTP 502" >&2; exit 1; }
  if [ -n "$jq_expr" ]; then jq -r "$jq_expr" <"$1" || exit 1
  else cat "$1" || exit 1; fi
}

case "$all" in
  "pr checks 42 "*)
    [ -f "$fix/checks.err" ] && cat "$fix/checks.err" >&2
    [ -f "$fix/checks.json" ] && cat "$fix/checks.json"
    if [ -f "$fix/checks.rc" ]; then exit "$(cat "$fix/checks.rc")"; fi
    [ -f "$fix/checks.json" ] || { echo "mock: HTTP 502" >&2; exit 1; }
    exit 0 ;;
  "pr view 42 "*) emit "$fix/pr-view.json"; exit 0 ;;
  "issue view "*)
    # -R <owner>/<name> が無い呼び出しは失敗させる（番号だけで照会しないことの確認）。
    n=$(awk '{print $3}' <<<"$all")
    repo=$(sed -nE 's/.* -R ([^ ]+).*/\1/p' <<<"$all")
    [ -n "$repo" ] || { echo "mock: issue view without -R" >&2; exit 2; }
    emit "$fix/issue-${repo//\//-}-$n.json"; exit 0 ;;
esac
echo "mock: unhandled: $all" >&2; exit 2
MOCK
chmod +x "$ROOT/bin/agent-gh"

# ---- テストの部品 -----------------------------------------------------------

n_fix=0
new_fixture() { # new_fixture -> 新しいフィクスチャのディレクトリを $fix に置く
  n_fix=$((n_fix + 1))
  fix="$ROOT/fx/$n_fix"
  mkdir -p "$fix"
}

checks() { # checks <fix> <name>=<bucket>... -> checks.json を書く
  local fix="$1"; shift
  local pair
  for pair in "$@"; do
    jq -n --arg n "${pair%=*}" --arg b "${pair##*=}" '{name: $n, bucket: $b}'
  done | jq -s . >"$fix/checks.json"
}

# pr-size-exception の目印に関わるフィクスチャ。
pr_view() { # pr_view <fix> <labels（カンマ区切り、空可）> [閉じる Issue のリポジトリ（既定 hatai/Pelway）]
  local repo=${3:-hatai/Pelway}
  jq -n --arg l "$2" --arg o "${repo%%/*}" --arg r "${repo#*/}" \
    '{labels: ($l | split(",") | map(select(length > 0) | {name: .})),
      closingIssuesReferences: [{number: 645, repository: {name: $r, owner: {login: $o}}}]}' \
    >"$1/pr-view.json"
}
issue_comment() { # issue_comment <fix> <author> <body> [Issue のリポジトリ（既定 hatai/Pelway）]
  local repo=${4:-hatai/Pelway}
  jq -n --arg a "$2" --arg b "$3" \
    '{comments: [{author: {login: "pelway-dev"}, body: "着手"}, {author: {login: $a}, body: $b}]}' \
    >"$1/issue-${repo//\//-}-645.json"
}
MARK='<!-- pr-size-exception -->
pr-size の超過を認める。'

run() { # run <fix> [引数...] -> 出力は $out、終了コードは $rc
  local fix="$1"; shift
  out="$(RC_FIXTURE="$fix" REQUIRED_CHECKS_GH="bash $ROOT/bin/agent-gh dev" bash "$HELPER" "$@" 2>/dev/null)"
  rc=$?
}

expect_rc() { # expect_rc <name> <want-rc>
  [ "$rc" = "$2" ] || failures+=("$1: exit $rc, want $2 (out: ${out//$'\n'/ | })")
}
expect_has() { # expect_has <name> <行>
  grep -qxF -- "$2" <<<"$out" || failures+=("$1: output missing line '$2' (out: ${out//$'\n'/ | })")
}

# ---- 両方 pass → 0、1 チェック 1 行の出力 ------------------------------------
new_fixture
checks "$fix" "mise run verify=pass" "pr-size=pass"
run "$fix" 42
expect_rc both_pass 0
expect_has both_pass "mise run verify: pass"
expect_has both_pass "pr-size: pass"
[ "$(wc -l <<<"$out")" = 2 ] || failures+=("both_pass: want 2 output lines, got: $out")

# ---- どちらかが fail → 1 ---------------------------------------------------
new_fixture
checks "$fix" "mise run verify=fail" "pr-size=pass"
run "$fix" 42
expect_rc verify_fail 1
expect_has verify_fail "mise run verify: fail"

new_fixture
checks "$fix" "mise run verify=pass" "pr-size=fail"
pr_view "$fix" ""
run "$fix" 42
expect_rc size_fail 1
expect_has size_fail "pr-size: fail"

# fail と pending が混ざれば fail を優先する。
new_fixture
checks "$fix" "mise run verify=pending" "pr-size=fail"
pr_view "$fix" ""
run "$fix" 42
expect_rc fail_over_pending 1

# ---- fail が無く、pending か一覧に無い → 2 ----------------------------------
new_fixture
checks "$fix" "mise run verify=pending" "pr-size=pass"
run "$fix" 42
expect_rc verify_pending 2
expect_has verify_pending "mise run verify: pending"

new_fixture
checks "$fix" "mise run verify=pass"
run "$fix" 42
expect_rc size_missing 2
expect_has size_missing "pr-size: pending"

# gh pr checks は pending があると終了コード 8 で一覧を返す。
new_fixture
checks "$fix" "mise run verify=pending" "pr-size=pass"
echo 8 >"$fix/checks.rc"
run "$fix" 42
expect_rc gh_exit8_pending 2

# チェックが 1 つも報告されていない head では gh が一覧を返さずに失敗する。
new_fixture
echo "no checks reported on the 'agent/645-x' branch" >"$fix/checks.err"
echo 1 >"$fix/checks.rc"
run "$fix" 42
expect_rc no_checks_reported 2
expect_has no_checks_reported "mise run verify: pending"
expect_has no_checks_reported "pr-size: pending"

# ---- 同じ名前の行が複数 ------------------------------------------------------
new_fixture
checks "$fix" "mise run verify=pass" "mise run verify=pass" "pr-size=pass" "pr-size=pass"
run "$fix" 42
expect_rc dup_all_pass 0

new_fixture
checks "$fix" "mise run verify=pass" "mise run verify=fail" "mise run verify=pending" "pr-size=pass"
run "$fix" 42
expect_rc dup_one_fail 1
expect_has dup_one_fail "mise run verify: fail"

new_fixture
checks "$fix" "mise run verify=pass" "mise run verify=pending" "pr-size=pass"
run "$fix" 42
expect_rc dup_one_pending 2
expect_has dup_one_pending "mise run verify: pending"

# ---- skipping は pass、cancel は fail -----------------------------------------
new_fixture
checks "$fix" "mise run verify=skipping" "pr-size=pass"
run "$fix" 42
expect_rc skipping_is_pass 0
expect_has skipping_is_pass "mise run verify: pass"

new_fixture
checks "$fix" "mise run verify=cancel" "pr-size=pass"
run "$fix" 42
expect_rc cancel_is_fail 1
expect_has cancel_is_fail "mise run verify: fail"

# ---- ほかの名前の行は影響しない ---------------------------------------------
new_fixture
checks "$fix" "mise run verify=pass" "CodeQL=fail" "pr-size=pass" "notify-grokbot=pending" "lint=cancel"
run "$fix" 42
expect_rc others_ignored 0

new_fixture
checks "$fix" "mise run verify (extra)=pass" "pr-size=pass"
run "$fix" 42
expect_rc name_exact_match 2

# ---- pr-size-exception -----------------------------------------------------
new_fixture
checks "$fix" "mise run verify=pass" "pr-size=fail"
pr_view "$fix" "agent:working,pr-size-exception"
issue_comment "$fix" hatai "$MARK"
run "$fix" 42
expect_rc exception_both 0
expect_has exception_both "pr-size: fail（pr-size-exception）"

new_fixture
checks "$fix" "mise run verify=pass" "pr-size=fail"
pr_view "$fix" "pr-size-exception"
issue_comment "$fix" hatai "pr-size の超過を認める。"
run "$fix" 42
expect_rc exception_label_only 1
expect_has exception_label_only "pr-size: fail"

new_fixture
checks "$fix" "mise run verify=pass" "pr-size=fail"
pr_view "$fix" ""
issue_comment "$fix" hatai "$MARK"
run "$fix" 42
expect_rc exception_comment_only 1
expect_has exception_comment_only "pr-size: fail"

new_fixture
checks "$fix" "mise run verify=pass" "pr-size=fail"
pr_view "$fix" "pr-size-exception"
issue_comment "$fix" "pelway-dev[bot]" "$MARK"
run "$fix" 42
expect_rc exception_not_hatai 1
expect_has exception_not_hatai "pr-size: fail"

# 目印が行頭に無い（引用の中など）コメントは目印として扱わない。
new_fixture
checks "$fix" "mise run verify=pass" "pr-size=fail"
pr_view "$fix" "pr-size-exception"
issue_comment "$fix" hatai "> <!-- pr-size-exception -->"
run "$fix" 42
expect_rc exception_not_line_start 1

# 閉じる Issue が別リポジトリにあるとき、同じ番号の hatai/Pelway の Issue の目印は使わない。
new_fixture
checks "$fix" "mise run verify=pass" "pr-size=fail"
pr_view "$fix" "pr-size-exception" "someone/other"
issue_comment "$fix" hatai "$MARK" hatai/Pelway
issue_comment "$fix" someone "pr-size の超過を認める。" someone/other
run "$fix" 42
expect_rc exception_other_repo 1
expect_has exception_other_repo "pr-size: fail"

# ---- 取得の失敗・引数の誤り → 3 -----------------------------------------------

# 標準出力に JSON の値が複数続くとき（一覧が分かれて届いたとき）は判定できない。
new_fixture
checks "$fix" "mise run verify=pass" "pr-size=pass"
printf '[{"name":"mise run verify","bucket":"pass"},{"name":"pr-size","bucket":"pass"}]\n[{"name":"mise run verify","bucket":"fail"},{"name":"pr-size","bucket":"fail"}]\n' >"$fix/checks.json"
run "$fix" 42
expect_rc checks_multiple_json 3

# pr view の応答に JSON の値が複数続くとき、別々の値のラベルと閉じる Issue を組み合わせない。
new_fixture
checks "$fix" "mise run verify=pass" "pr-size=fail"
printf '%s\n%s\n' \
  '{"labels":[],"closingIssuesReferences":[{"number":645,"repository":{"name":"Pelway","owner":{"login":"hatai"}}}]}' \
  '{"labels":[{"name":"pr-size-exception"}],"closingIssuesReferences":[]}' >"$fix/pr-view.json"
issue_comment "$fix" hatai "$MARK"
run "$fix" 42
expect_rc pr_view_multiple_json 3

# issue view の応答に JSON の値が複数続くときも判定できない。
new_fixture
checks "$fix" "mise run verify=pass" "pr-size=fail"
pr_view "$fix" "pr-size-exception"
issue_comment "$fix" hatai "$MARK"
cp "$fix/issue-hatai-Pelway-645.json" "$fix/one.json"
cat "$fix/one.json" "$fix/one.json" >"$fix/issue-hatai-Pelway-645.json"
run "$fix" 42
expect_rc issue_view_multiple_json 3
new_fixture
run "$fix" 42
expect_rc checks_fetch_error 3

new_fixture
echo '<html>bad gateway</html>' >"$fix/checks.json"
run "$fix" 42
expect_rc checks_not_json 3

new_fixture
checks "$fix" "mise run verify=weird" "pr-size=pass"
run "$fix" 42
expect_rc unknown_bucket 3

new_fixture
checks "$fix" "mise run verify=pass" "pr-size=fail"
run "$fix" 42
expect_rc pr_view_error 3

new_fixture
checks "$fix" "mise run verify=pass" "pr-size=fail"
pr_view "$fix" "pr-size-exception"
run "$fix" 42
expect_rc issue_view_error 3

new_fixture
checks "$fix" "mise run verify=pass" "pr-size=pass"
run "$fix"
expect_rc no_args 3
run "$fix" abc
expect_rc non_numeric 3
run "$fix" 42 43
expect_rc too_many_args 3

# ---- 結果 -------------------------------------------------------------------

if [ "${#failures[@]}" -gt 0 ]; then
  printf 'required-checks.test: %d failure(s)\n' "${#failures[@]}"
  printf '  - %s\n' "${failures[@]}"
  exit 1
fi
echo "required-checks.test: ok"
