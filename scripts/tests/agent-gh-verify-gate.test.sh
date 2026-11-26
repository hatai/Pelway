#!/usr/bin/env bash
# scripts/agent-verify-record と scripts/agent-gh の verify ゲート（#807）のテスト。
# 本物の git とローカルの bare remote で、印の記録・push の停止・docs/worklogs の例外・skip を確かめる。
set -uo pipefail

AGENT_GH="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/agent-gh"
RECORD="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/agent-verify-record"
ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT
failures=()

# 利用者の git 設定に左右されないよう、global / system の設定を読ませない。
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1

BIN="$ROOT/bin"
mkdir -p "$BIN"
REAL_GIT="$(command -v git)"

# 偽の fnox: `fnox -c <config> get NAME` に FAKE_<NAME> の値を返す。
cat >"$BIN/fnox" <<'EOS'
#!/usr/bin/env bash
[[ "$1" == "-c" && "$3" == "get" ]] || exit 9
var="FAKE_$4"
[[ -n "${!var:-}" ]] || exit 1
printf '%s\n' "${!var}"
EOS

# 偽の gh: トークン発行だけを受け、push（ローカルの bare remote）では credential を求められない。
cat >"$BIN/gh" <<'EOS'
#!/usr/bin/env bash
if [[ "$1 $2" == "token generate" ]]; then
  case "$*" in
    *"--app-id 333 "*) printf '{"token":"dev-token","expires_at":"x"}\n' ;;
    *"--app-id 111 "*) printf '{"token":"review-token","expires_at":"x"}\n' ;;
    *) exit 1 ;;
  esac
  exit 0
fi
if [[ "$1 $2" == "token --help" ]]; then
  printf 'gh-token help\n'
  exit 0
fi
exit 1
EOS
chmod +x "$BIN"/*
export PATH="$BIN:$PATH"
export PELWAY_AGENT_FNOX_CONFIG="$ROOT/agent-fnox.toml"
export PELWAY_AGENT_FNOX_CONFIG_REVIEW="$ROOT/agent-fnox-review.toml"
export FAKE_PELWAY_DEV_APP_ID=333
export FAKE_PELWAY_DEV_APP_INSTALLATION_ID=444
export FAKE_PELWAY_DEV_APP_KEY=dev-pem-body
export FAKE_PELWAY_DEV_GIT_EMAIL="9+pelway-dev[bot]@users.noreply.github.com"
export FAKE_PELWAY_REVIEW_APP_ID=111
export FAKE_PELWAY_REVIEW_APP_INSTALLATION_ID=222
export FAKE_PELWAY_REVIEW_APP_KEY=review-pem-body

# as <name> <git の引数...>: author・committer を <name> にして本物の git を実行する。
as() {
  local who="$1"
  shift
  GIT_AUTHOR_NAME="$who" GIT_AUTHOR_EMAIL="$who@example.com" GIT_COMMITTER_NAME="$who" GIT_COMMITTER_EMAIL="$who@example.com" \
    "$REAL_GIT" -c commit.gpgsign=false "$@"
}

# setup_repo <name>: hatai の main を持つ bare remote と、agent/<name> を切った work を作る。
setup_repo() {
  REPO_DIR="$ROOT/repo-$1"
  mkdir -p "$REPO_DIR"
  "$REAL_GIT" init -q --bare -b main "$REPO_DIR/remote.git"
  "$REAL_GIT" init -q -b main "$REPO_DIR/seed"
  (
    cd "$REPO_DIR/seed" || exit 1
    as hatai commit -q --allow-empty -m init
    "$REAL_GIT" push -q "$REPO_DIR/remote.git" main
  )
  "$REAL_GIT" clone -q "$REPO_DIR/remote.git" "$REPO_DIR/work"
  "$REAL_GIT" -C "$REPO_DIR/work" switch -q -c "agent/$1"
}

work_git() { "$REAL_GIT" -C "$REPO_DIR/work" "$@"; }
remote_ref() { "$REAL_GIT" --git-dir="$REPO_DIR/remote.git" rev-parse -q --verify "refs/heads/$1"; }
marker_file() { printf '%s\n' "$REPO_DIR/work/.git/pelway-verify-tree"; }

# push_role <name> <role> <git push の引数...>: 標準出力を OUT、標準エラーを ERR、終了コードを RC に入れる。
push_role() {
  local name="$1" role="$2"
  shift 2
  (cd "$REPO_DIR/work" && PELWAY_AGENT_SKIP_RESIGN=1 bash "$AGENT_GH" "$role" git push "$@" >"$ROOT/$name.out" 2>"$ROOT/$name.err")
  RC=$?
  ERR="$(cat "$ROOT/$name.err")"
}

# --- 印の記録: 成功時は検証した作業ツリー（commit 後に HEAD になる木）の hash を残す ---
setup_repo record
(
  cd "$REPO_DIR/work" || exit 1
  bash "$RECORD" >/dev/null 2>&1
)
rc=$?
[[ "$rc" == 0 && -f "$(marker_file)" ]] || failures+=("record_clean: exit $rc or marker missing")
[[ "$(cat "$(marker_file)" 2>/dev/null)" == "$(work_git rev-parse 'HEAD^{tree}')" ]] ||
  failures+=("record_clean: marker is not the HEAD tree")
printf 'implementation\n' >"$REPO_DIR/work/code.txt"
mkdir -p "$REPO_DIR/work/docs/worklogs"
printf 'memo\n' >"$REPO_DIR/work/docs/worklogs/issue-807.md"
(
  cd "$REPO_DIR/work" || exit 1
  bash "$RECORD" >/dev/null 2>&1
)
rc=$?
work_git add code.txt docs/worklogs/issue-807.md
(cd "$REPO_DIR/work" && as 'pelway-dev[bot]' commit -q -m implementation)
[[ "$rc" == 0 && "$(cat "$(marker_file)" 2>/dev/null)" == "$(work_git rev-parse 'HEAD^{tree}')" ]] ||
  failures+=("record_worktree: marker does not match the tree committed after verify")

# mise の verify タスクは、依存タスクの後で印の記録を呼ぶ。依存タスクが失敗したときは run の段に進まず、
# 成功したときだけ印ができる。合成の依存タスクに、リポジトリの verify タスクの節をそのままつないで確かめる。
verify_section="$(sed -n '/^\[tasks\.verify\]/,/^\[tasks\.verify-lint\]/p' mise.toml)"
grep -qF 'run = "bash scripts/agent-verify-record"' <<<"$verify_section" ||
  failures+=("mise_verify_hookup: verify task does not call agent-verify-record")
MISE_TOY="$ROOT/mise-verify-result"
mkdir -p "$MISE_TOY/scripts"
cp "$RECORD" "$MISE_TOY/scripts/agent-verify-record"
cat >"$MISE_TOY/mise.toml" <<'EOS'
[tasks.verify-lint]
run = "exit ${VERIFY_DEPS_RC:-0}"

[tasks.verify-test]
run = "true"

[tasks.verify-static]
run = "true"

EOS
sed -n '/^\[tasks\.verify\]/,/^\[tasks\.verify-lint\]/p' mise.toml | sed '$d' >>"$MISE_TOY/mise.toml"
"$REAL_GIT" init -q -b main "$MISE_TOY"
(cd "$MISE_TOY" && "$REAL_GIT" -c user.name=test -c user.email=test@example.com commit -q --allow-empty -m init)
(cd "$MISE_TOY" && MISE_TRUSTED_CONFIG_PATHS="$MISE_TOY/mise.toml" mise run verify >"$ROOT/mise-verify-success.out" 2>&1)
rc=$?
[[ "$rc" == 0 && -f "$MISE_TOY/.git/pelway-verify-tree" ]] ||
  failures+=("mise_verify_success: exit $rc or marker missing")
first_marker="$(cat "$MISE_TOY/.git/pelway-verify-tree")"
printf 'unverified change\n' >"$MISE_TOY/change.txt"
(cd "$MISE_TOY" && VERIFY_DEPS_RC=1 MISE_TRUSTED_CONFIG_PATHS="$MISE_TOY/mise.toml" mise run verify >"$ROOT/mise-verify-failure.out" 2>&1)
rc=$?
[[ "$rc" != 0 && -f "$MISE_TOY/.git/pelway-verify-tree" && "$(cat "$MISE_TOY/.git/pelway-verify-tree")" == "$first_marker" ]] ||
  failures+=("mise_verify_failure: exit $rc or marker changed")

# --- 印が無い push は止まり、理由と mise run verify の案内が出る ---
setup_repo missing
printf 'new implementation\n' >"$REPO_DIR/work/code.txt"
work_git add code.txt
(cd "$REPO_DIR/work" && as 'pelway-dev[bot]' commit -q -m implementation)
push_role missing dev -u origin HEAD
[[ "$RC" != 0 && "$ERR" == *'verify'* && "$ERR" == *'mise run verify'* ]] ||
  failures+=("push_missing_marker: not stopped with guidance: rc=$RC $ERR")
[[ -z "$(remote_ref agent/missing)" ]] || failures+=("push_missing_marker: remote ref changed")

# --- 検証した木と同じ HEAD は通る ---
setup_repo exact
printf 'implementation\n' >"$REPO_DIR/work/code.txt"
(
  cd "$REPO_DIR/work" || exit 1
  bash "$RECORD" >/dev/null 2>&1
)
work_git add code.txt
(cd "$REPO_DIR/work" && as 'pelway-dev[bot]' commit -q -m implementation)
push_role exact dev -u origin HEAD
[[ "$RC" == 0 && "$(remote_ref agent/exact)" == "$(work_git rev-parse HEAD)" ]] ||
  failures+=("push_exact_marker: not pushed: rc=$RC $ERR")

# --- HEAD と印の差が docs/worklogs/ の下だけなら通す（実装コミットとメモコミットの両方） ---
setup_repo worklogs
printf 'implementation\n' >"$REPO_DIR/work/code.txt"
mkdir -p "$REPO_DIR/work/docs/worklogs"
printf 'memo\n' >"$REPO_DIR/work/docs/worklogs/issue-807.md"
(
  cd "$REPO_DIR/work" || exit 1
  bash "$RECORD" >/dev/null 2>&1
)
work_git add code.txt
(cd "$REPO_DIR/work" && as 'pelway-dev[bot]' commit -q -m implementation)
push_role worklogs_impl dev -u origin HEAD
[[ "$RC" == 0 && "$(remote_ref agent/worklogs)" == "$(work_git rev-parse HEAD)" ]] ||
  failures+=("push_worklogs_only_impl: implementation commit not pushed: rc=$RC $ERR")
work_git add docs/worklogs/issue-807.md
(cd "$REPO_DIR/work" && as 'pelway-dev[bot]' commit -q -m memo)
push_role worklogs_memo dev origin HEAD
[[ "$RC" == 0 && "$(remote_ref agent/worklogs)" == "$(work_git rev-parse HEAD)" ]] ||
  failures+=("push_worklogs_only_memo: memo commit not pushed: rc=$RC $ERR")

# --- docs/worklogs/ の外のファイルを docs/worklogs/ の下へ移しただけの木は通さない ---
# rename 検出が効くと diff の出力が移動先だけになり、外の削除を見落とすため、門は --no-renames で比べる。
setup_repo rename_escape
mkdir -p "$REPO_DIR/work/crates/x/tests"
printf 'same contents for rename detection\n' >"$REPO_DIR/work/crates/x/tests/foo.rs"
work_git add crates/x/tests/foo.rs
(cd "$REPO_DIR/work" && as 'pelway-dev[bot]' commit -q -m implementation)
(
  cd "$REPO_DIR/work" || exit 1
  bash "$RECORD" >/dev/null 2>&1
)
mkdir -p "$REPO_DIR/work/docs/worklogs"
work_git mv crates/x/tests/foo.rs docs/worklogs/foo.rs
(cd "$REPO_DIR/work" && as 'pelway-dev[bot]' commit -q -m 'move into worklogs')
push_role rename_escape dev -u origin HEAD
[[ "$RC" != 0 && "$ERR" == *'mise run verify'* ]] ||
  failures+=("push_rename_escape: not stopped: rc=$RC $ERR")
[[ -z "$(remote_ref agent/rename_escape)" ]] || failures+=("push_rename_escape: remote ref changed")

# --- docs/worklogs/ の外が変わった HEAD には前の印を使わない ---
setup_repo stale
printf 'implementation\n' >"$REPO_DIR/work/code.txt"
(
  cd "$REPO_DIR/work" || exit 1
  bash "$RECORD" >/dev/null 2>&1
)
work_git add code.txt
(cd "$REPO_DIR/work" && as 'pelway-dev[bot]' commit -q -m implementation)
printf 'unverified change\n' >>"$REPO_DIR/work/code.txt"
work_git add code.txt
(cd "$REPO_DIR/work" && as 'pelway-dev[bot]' commit -q -m unverified)
push_role stale dev -u origin HEAD
[[ "$RC" != 0 && "$ERR" == *'mise run verify'* ]] ||
  failures+=("push_stale_marker: not stopped: rc=$RC $ERR")
[[ -z "$(remote_ref agent/stale)" ]] || failures+=("push_stale_marker: remote ref changed")

# --- review 役の push にも同じ門が掛かる ---
setup_repo review_role
printf 'review implementation\n' >"$REPO_DIR/work/code.txt"
work_git add code.txt
(cd "$REPO_DIR/work" && as 'pelway-review[bot]' commit -q -m implementation)
push_role review_role review -u origin HEAD
[[ "$RC" != 0 && "$ERR" == *'mise run verify'* ]] ||
  failures+=("push_review_role: not stopped: rc=$RC $ERR")
[[ -z "$(remote_ref agent/review_role)" ]] || failures+=("push_review_role: remote ref changed")

# --- PELWAY_AGENT_SKIP_VERIFY_GATE は 1 のときだけ外れる。0 では外れない ---
setup_repo skip_zero
printf 'implementation\n' >"$REPO_DIR/work/code.txt"
work_git add code.txt
(cd "$REPO_DIR/work" && as 'pelway-dev[bot]' commit -q -m implementation)
(cd "$REPO_DIR/work" && PELWAY_AGENT_SKIP_RESIGN=1 PELWAY_AGENT_SKIP_VERIFY_GATE=0 bash "$AGENT_GH" dev git push -u origin HEAD >"$ROOT/skip_zero.out" 2>"$ROOT/skip_zero.err")
RC=$?
ERR="$(cat "$ROOT/skip_zero.err")"
[[ "$RC" != 0 && "$ERR" == *'mise run verify'* ]] ||
  failures+=("push_skip_zero: skipped with PELWAY_AGENT_SKIP_VERIFY_GATE=0: rc=$RC $ERR")
[[ -z "$(remote_ref agent/skip_zero)" ]] || failures+=("push_skip_zero: remote ref changed")

# --- PELWAY_AGENT_SKIP_VERIFY_GATE=1 は確かめずに通し、通したことを標準エラーに出す ---
setup_repo skip
printf 'implementation\n' >"$REPO_DIR/work/code.txt"
work_git add code.txt
(cd "$REPO_DIR/work" && as 'pelway-dev[bot]' commit -q -m implementation)
(cd "$REPO_DIR/work" && PELWAY_AGENT_SKIP_RESIGN=1 PELWAY_AGENT_SKIP_VERIFY_GATE=1 bash "$AGENT_GH" dev git push -u origin HEAD >"$ROOT/skip.out" 2>"$ROOT/skip.err")
RC=$?
ERR="$(cat "$ROOT/skip.err")"
[[ "$RC" == 0 && "$(remote_ref agent/skip)" == "$(work_git rev-parse HEAD)" ]] ||
  failures+=("push_skip_gate: not pushed: rc=$RC $ERR")
[[ "$ERR" == *'PELWAY_AGENT_SKIP_VERIFY_GATE=1'* && "$ERR" == *'確かめず'* ]] ||
  failures+=("push_skip_gate: skip notice missing: $ERR")

if ((${#failures[@]})); then
  printf 'FAIL %s\n' "${failures[@]}"
  exit 1
fi
echo ok
