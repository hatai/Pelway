#!/usr/bin/env bash
# scripts/agent-verify-record と scripts/agent-gh の verify ゲート（#807）のテスト。
# 本物の git とローカルの bare remote で、印の記録・push の停止・docs/worklogs の例外・skip を確かめる。
set -uo pipefail
AGENT_GH="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/agent-gh"
RECORD="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/agent-verify-record"
VERIFY_RUN="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/agent-verify-run"
ROOT="$(mktemp -d)" || { echo "agent-gh-verify-gate.test: 一時ディレクトリを作れません" >&2; exit 1; }
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
var="FAKE_$4"; [[ -n "${!var:-}" ]] || exit 1; printf '%s\n' "${!var}"
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
[[ "$1 $2" == "token --help" ]] && { printf 'gh-token help\n'; exit 0; }
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
  local who="$1"; shift
  GIT_AUTHOR_NAME="$who" GIT_AUTHOR_EMAIL="$who@example.com" GIT_COMMITTER_NAME="$who" GIT_COMMITTER_EMAIL="$who@example.com" "$REAL_GIT" -c commit.gpgsign=false "$@"
}
# setup_repo <name>: hatai の main を持つ bare remote と、agent/<name> を切った work を作る。
setup_repo() {
  REPO_DIR="$ROOT/repo-$1"
  mkdir -p "$REPO_DIR"
  "$REAL_GIT" init -q --bare -b main "$REPO_DIR/remote.git"
  "$REAL_GIT" init -q -b main "$REPO_DIR/seed"
  (cd "$REPO_DIR/seed" && as hatai commit -q --allow-empty -m init && "$REAL_GIT" push -q "$REPO_DIR/remote.git" main)
  "$REAL_GIT" clone -q "$REPO_DIR/remote.git" "$REPO_DIR/work"
  "$REAL_GIT" -C "$REPO_DIR/work" switch -q -c "agent/$1"
}
work_git() { "$REAL_GIT" -C "$REPO_DIR/work" "$@"; }
remote_ref() { "$REAL_GIT" --git-dir="$REPO_DIR/remote.git" rev-parse -q --verify "refs/heads/$1"; }
marker_file() { printf '%s\n' "$REPO_DIR/work/.git/pelway-verify-tree"; }
marker_tree_file() { sed -n '1p' "$1"; }
started_count() {
  shopt -s nullglob
  local entries=("$1"/*)
  printf '%s\n' "${#entries[@]}"
}
# push_role <name> <role> <git push の引数...>: 標準出力を OUT、標準エラーを ERR、終了コードを RC に入れる。
push_role() {
  local name="$1" role="$2"; shift 2
  (cd "$REPO_DIR/work" && PELWAY_AGENT_SKIP_RESIGN=1 bash "$AGENT_GH" "$role" git push "$@" >"$ROOT/$name.out" 2>"$ROOT/$name.err")
  RC=$?; ERR="$(cat "$ROOT/$name.err")"
}
# --- 印の記録: 成功時は検証した作業ツリー（commit 後に HEAD になる木）の hash を残す ---
setup_repo record
(cd "$REPO_DIR/work" && bash "$RECORD" >/dev/null 2>&1); rc=$?
[[ "$rc" == 0 && -f "$(marker_file)" ]] || failures+=("record_clean: exit $rc or marker missing")
[[ "$(marker_tree_file "$(marker_file)" 2>/dev/null)" == "$(work_git rev-parse 'HEAD^{tree}')" && "$(sed -n '2p' "$(marker_file)")" =~ ^verify-id:[0-9a-f]{32}$ ]] ||
  failures+=("record_clean: marker is not the HEAD tree with a verify id")
printf 'implementation\n' >"$REPO_DIR/work/code.txt"
mkdir -p "$REPO_DIR/work/docs/worklogs"
printf 'memo\n' >"$REPO_DIR/work/docs/worklogs/issue-807.md"
(cd "$REPO_DIR/work" && bash "$RECORD" >/dev/null 2>&1); rc=$?
work_git add code.txt docs/worklogs/issue-807.md
(cd "$REPO_DIR/work" && as 'pelway-dev[bot]' commit -q -m implementation)
[[ "$rc" == 0 && "$(marker_tree_file "$(marker_file)" 2>/dev/null)" == "$(work_git rev-parse 'HEAD^{tree}')" ]] ||
  failures+=("record_worktree: marker does not match the tree committed after verify")
# --- 実際の index の追跡情報を引き継ぐ: core.fileMode=false でも実行権限が残り、
# 無視のパターンに当たる追跡済みファイルも印の木に入る ---
setup_repo index_details
printf '#!/usr/bin/env bash\nexit 0\n' >"$REPO_DIR/work/tool.sh"
chmod +x "$REPO_DIR/work/tool.sh"
printf 'ignored-but-tracked\n' >"$REPO_DIR/work/ignored.txt"
printf 'ignored.txt\n' >"$REPO_DIR/work/.gitignore"
work_git add tool.sh .gitignore; work_git add -f ignored.txt
(cd "$REPO_DIR/work" && as 'pelway-dev[bot]' commit -q -m implementation)
work_git config core.fileMode false
(cd "$REPO_DIR/work" && bash "$RECORD" >/dev/null 2>&1); rc=$?
marker_tree="$(marker_tree_file "$(marker_file)" 2>/dev/null)"
[[ "$rc" == 0 && "$(work_git ls-tree "$marker_tree" tool.sh | awk '{print $1}')" == 100755 ]] ||
  failures+=("record_index_mode: executable mode is not preserved")
[[ -n "$(work_git ls-tree "$marker_tree" ignored.txt)" ]] ||
  failures+=("record_index_ignored: tracked ignored file is missing")
# --- assume-unchanged / skip-worktree が付いていても、印は index の古い内容ではなく実ファイルを映す ---
setup_repo index_flags
printf 'index content\n' >"$REPO_DIR/work/assumed.txt"; printf 'skip content\n' >"$REPO_DIR/work/skipped.txt"
work_git add assumed.txt skipped.txt
(cd "$REPO_DIR/work" && as 'pelway-dev[bot]' commit -q -m implementation)
work_git update-index --assume-unchanged assumed.txt; work_git update-index --skip-worktree skipped.txt
printf 'worktree content\n' >"$REPO_DIR/work/assumed.txt"; printf 'skip worktree content\n' >"$REPO_DIR/work/skipped.txt"
(cd "$REPO_DIR/work" && bash "$RECORD" >/dev/null 2>&1); rc=$?
marker_tree="$(marker_tree_file "$(marker_file)" 2>/dev/null)"
[[ "$rc" == 0 && "$(work_git ls-tree "$marker_tree" assumed.txt | awk '{print $3}')" == "$(work_git hash-object assumed.txt)" ]] ||
  failures+=("record_assume_unchanged: marker does not use the worktree content")
[[ "$(work_git ls-tree "$marker_tree" skipped.txt | awk '{print $3}')" == "$(work_git hash-object skipped.txt)" ]] ||
  failures+=("record_skip_worktree: marker does not use the worktree content")
# mise の verify タスクは、検査の前に前回の印を消し、3 グループがすべて成功したときだけ印を残す。
# 同じ木で再実行が失敗したとき、古い印で push が通らないことを合成の依存タスクで確かめる。
verify_section="$(sed -n '/^\[tasks\.verify\]/,/^\[tasks\.verify-lint\]/p' mise.toml)"
grep -qF 'run = "python3 -I scripts/agent-verify-run"' <<<"$verify_section" ||
  failures+=("mise_verify_hookup: verify task does not call agent-verify-run")
MISE_TOY="$ROOT/mise-verify-result"
mkdir -p "$MISE_TOY/scripts"; cp "$RECORD" "$MISE_TOY/scripts/agent-verify-record"; cp "$VERIFY_RUN" "$MISE_TOY/scripts/agent-verify-run"; cp "$(dirname "$RECORD")/agent-verify-paths.sh" "$MISE_TOY/scripts/agent-verify-paths.sh"
printf '/.pelway-verify/\n' >"$MISE_TOY/.gitignore"
cat >"$MISE_TOY/scripts/verify-lint-toy" <<'EOS'
#!/usr/bin/env bash
[[ -z "${VERIFY_TOUCH_BEFORE_EXIT:-}" ]] || touch "$VERIFY_TOUCH_BEFORE_EXIT"
sleep "${VERIFY_SLEEP:-0}"
exit "${VERIFY_DEPS_RC:-0}"
EOS
cat >"$MISE_TOY/mise.toml" <<'EOS'
[tasks.verify-lint]
run = "bash scripts/verify-lint-toy"
[tasks.verify-test]
run = "true"
[tasks.verify-static]
run = "true"
EOS
sed -n '/^\[tasks\.verify\]/,/^\[tasks\.verify-lint\]/p' mise.toml | sed '$d' >>"$MISE_TOY/mise.toml"
"$REAL_GIT" init -q -b main "$MISE_TOY"; "$REAL_GIT" init -q --bare -b main "$ROOT/mise-toy-remote.git"
(cd "$MISE_TOY" && as 'pelway-dev[bot]' commit -q --allow-empty -m init)
"$REAL_GIT" -C "$MISE_TOY" remote add origin "$ROOT/mise-toy-remote.git"
(cd "$MISE_TOY" && as 'pelway-dev[bot]' add -A && as 'pelway-dev[bot]' commit -q -m verify-test-files)
"$REAL_GIT" -C "$MISE_TOY" update-ref refs/remotes/origin/main HEAD
(cd "$MISE_TOY" && MISE_TRUSTED_CONFIG_PATHS="$MISE_TOY/mise.toml" mise run verify >"$ROOT/mise-verify-success.out" 2>&1); rc=$?
[[ "$rc" == 0 && -f "$MISE_TOY/.git/pelway-verify-tree" ]] ||
  failures+=("mise_verify_success: exit $rc or marker missing")
(cd "$MISE_TOY" && VERIFY_DEPS_RC=1 MISE_TRUSTED_CONFIG_PATHS="$MISE_TOY/mise.toml" mise run verify >"$ROOT/mise-verify-failure.out" 2>&1); rc=$?
[[ "$rc" != 0 && ! -e "$MISE_TOY/.git/pelway-verify-tree" ]] ||
  failures+=("mise_verify_failure: exit $rc or stale marker remains")
(cd "$MISE_TOY" && PELWAY_AGENT_SKIP_RESIGN=1 bash "$AGENT_GH" dev git push origin HEAD:refs/heads/agent/flaky-verify >"$ROOT/flaky-push.out" 2>"$ROOT/flaky-push.err"); RC=$?; ERR="$(cat "$ROOT/flaky-push.err")"
[[ "$RC" != 0 && "$ERR" == *'mise run verify'* ]] ||
  failures+=("push_after_same_tree_verify_failure: not stopped: rc=$RC $ERR")
[[ -z "$("$REAL_GIT" --git-dir="$ROOT/mise-toy-remote.git" rev-parse -q --verify refs/heads/agent/flaky-verify 2>/dev/null)" ]] ||
  failures+=("push_after_same_tree_verify_failure: remote ref changed")
# 同じ worktree で成功と失敗の verify が重なっても直列になる。直列なら、後から始めた失敗が
# 先の成功の記録を待ってから印を消すため、最後に印は残らない。試験用タスクの到達を合図にする。
overlap_started="$ROOT/mise-overlap-started"; rm -f "$overlap_started"
(cd "$MISE_TOY" && VERIFY_TOUCH_BEFORE_EXIT="$overlap_started" VERIFY_SLEEP=2 MISE_TRUSTED_CONFIG_PATHS="$MISE_TOY/mise.toml" mise run verify >"$ROOT/mise-overlap-success.out" 2>&1) &
overlap_first=$!
for ((i = 0; i < 100; i++)); do [[ -e "$overlap_started" ]] && break; sleep 0.1; done
(cd "$MISE_TOY" && VERIFY_DEPS_RC=1 MISE_TRUSTED_CONFIG_PATHS="$MISE_TOY/mise.toml" mise run verify >"$ROOT/mise-overlap-failure.out" 2>&1); overlap_failure_rc=$?
wait "$overlap_first"; overlap_success_rc=$?
[[ "$overlap_success_rc" == 0 && "$overlap_failure_rc" != 0 && ! -e "$MISE_TOY/.git/pelway-verify-tree" ]] ||
  failures+=("mise_verify_overlap: success=$overlap_success_rc failure=$overlap_failure_rc marker=$(test -e "$MISE_TOY/.git/pelway-verify-tree" && echo present || echo missing)")
# ロックファイルが symlink なら、リンク先を切り詰めない。前回の印は残っても、
# verify の開始の記録があるため push では再利用できない。読めない印は inode の指紋でも止まる。
lock_path="$MISE_TOY/.git/pelway-verify.lock"; marker_path="$MISE_TOY/.git/pelway-verify-tree"; lock_target="$ROOT/verify-lock-target"
printf 'important\n' >"$lock_target"; rm -f "$lock_path"; ln -s "$lock_target" "$lock_path"
printf '%s\nverify-id:0123456789abcdef0123456789abcdef\n' "$("$REAL_GIT" -C "$MISE_TOY" rev-parse 'HEAD^{tree}')" >"$marker_path"
chmod 000 "$marker_path"
(cd "$MISE_TOY" && MISE_TRUSTED_CONFIG_PATHS="$MISE_TOY/mise.toml" mise run verify >"$ROOT/mise-lock-symlink.out" 2>&1); rc=$?
chmod 600 "$marker_path"
[[ "$rc" != 0 && "$(cat "$lock_target")" == important && -e "$marker_path" && "$(started_count "$MISE_TOY/.pelway-verify/started")" -ge 1 ]] ||
  failures+=("verify_lock_symlink: rc=$rc target=$(cat "$lock_target" 2>/dev/null) marker=$(test -e "$marker_path" && echo present || echo missing) started=$(started_count "$MISE_TOY/.pelway-verify/started")")
(cd "$MISE_TOY" && PELWAY_AGENT_SKIP_RESIGN=1 bash "$AGENT_GH" dev git push origin HEAD:refs/heads/agent/lock-poison >"$ROOT/lock-poison-push.out" 2>"$ROOT/lock-poison-push.err"); RC=$?; ERR="$(cat "$ROOT/lock-poison-push.err")"
[[ "$RC" != 0 && "$ERR" == *'前回の verify はこの印を無効化しました'* ]] ||
  failures+=("push_lock_poison: not stopped: rc=$RC $ERR")
rm -f "$lock_path" "$marker_path"; rm -rf "$MISE_TOY/.pelway-verify"
# 開始の記録を残せないときは検査を始めない。予備に残せるときは印を残し、予備にも残せないときは印を消す。
mkdir "$MISE_TOY/.pelway-verify"; chmod a-w "$MISE_TOY/.pelway-verify"
printf '%s\nverify-id:0123456789abcdef0123456789abcdef\n' "$("$REAL_GIT" -C "$MISE_TOY" rev-parse 'HEAD^{tree}')" >"$marker_path"
start_touch="$ROOT/verify-start-touch"; rm -f "$start_touch"
(cd "$MISE_TOY" && VERIFY_TOUCH_BEFORE_EXIT="$start_touch" MISE_TRUSTED_CONFIG_PATHS="$MISE_TOY/mise.toml" mise run verify >"$ROOT/mise-start-unwritable.out" 2>&1); rc=$?
[[ "$rc" != 0 && ! -e "$start_touch" && -f "$marker_path" ]] ||
  failures+=("verify_start_unwritable: rc=$rc touch=$(test -e "$start_touch" && echo present || echo missing) marker=$(test -e "$marker_path" && echo present || echo missing)")
chmod u+w "$MISE_TOY/.pelway-verify"
(cd "$MISE_TOY" && PELWAY_AGENT_SKIP_RESIGN=1 bash "$AGENT_GH" dev git push origin HEAD:refs/heads/agent/start-unwritable-before-verify >"$ROOT/start-unwritable-push.out" 2>"$ROOT/start-unwritable-push.err"); RC=$?; ERR="$(cat "$ROOT/start-unwritable-push.err")"
[[ "$RC" != 0 && "$ERR" == *'前回の verify はこの印を無効化しました'* ]] ||
  failures+=("verify_start_unwritable_push: failure was not persisted: rc=$RC $ERR")
rm -rf "$MISE_TOY/.pelway-verify"
(cd "$MISE_TOY" && MISE_TRUSTED_CONFIG_PATHS="$MISE_TOY/mise.toml" mise run verify >"$ROOT/mise-start-recovered.out" 2>&1); recovered_rc=$?
[[ "$recovered_rc" == 0 && -f "$marker_path" ]] ||
  failures+=("verify_start_recovered: rc=$recovered_rc marker=$(test -e "$marker_path" && echo present || echo missing)")
rm -rf "$MISE_TOY/.pelway-verify"; mkdir "$MISE_TOY/.pelway-verify"; chmod a-w "$MISE_TOY/.pelway-verify"
printf '%s\nverify-id:0123456789abcdef0123456789abcdef\n' "$("$REAL_GIT" -C "$MISE_TOY" rev-parse 'HEAD^{tree}')" >"$marker_path"
rm -rf "$MISE_TOY/.git/pelway-verify-started"; : >"$MISE_TOY/.git/pelway-verify-started"
(cd "$MISE_TOY" && VERIFY_TOUCH_BEFORE_EXIT="$start_touch" MISE_TRUSTED_CONFIG_PATHS="$MISE_TOY/mise.toml" mise run verify >"$ROOT/mise-all-state-unwritable.out" 2>&1); rc=$?
[[ "$rc" != 0 && ! -e "$start_touch" && ! -e "$marker_path" ]] ||
  failures+=("verify_all_state_unwritable: rc=$rc touch=$(test -e "$start_touch" && echo present || echo missing) marker=$(test -e "$marker_path" && echo present || echo missing)")
rm -rf "$MISE_TOY/.git/pelway-verify-started"; chmod u+w "$MISE_TOY/.pelway-verify"; rm -rf "$MISE_TOY/.pelway-verify" "$marker_path"
mkdir "$MISE_TOY/.pelway-verify"; chmod a-w "$MISE_TOY/.pelway-verify"
printf '%s
verify-id:0123456789abcdef0123456789abcdef
' "$("$REAL_GIT" -C "$MISE_TOY" rev-parse 'HEAD^{tree}')" >"$marker_path"
chmod 000 "$marker_path"
rm -rf "$MISE_TOY/.git/pelway-verify-started"; : >"$MISE_TOY/.git/pelway-verify-started"
: >"$lock_path"; chmod 600 "$lock_path"
readability_lock_held="$ROOT/readability-lock-held"; readability_lock_release="$ROOT/readability-lock-release"; rm -f "$readability_lock_held" "$readability_lock_release"
LOCK_HELD="$readability_lock_held" LOCK_RELEASE="$readability_lock_release" flock -x "$lock_path" -c 'touch "$LOCK_HELD"; while [[ ! -e "$LOCK_RELEASE" ]]; do sleep 0.05; done' &
readability_manual_lock=$!
for ((i = 0; i < 100; i++)); do [[ -e "$readability_lock_held" ]] && break; sleep 0.05; done
(cd "$MISE_TOY" && MISE_TRUSTED_CONFIG_PATHS="$MISE_TOY/mise.toml" mise run verify >"$ROOT/mise-readability-restore.out" 2>&1) &
readability_verify=$!
readability_waited=0
for ((i = 0; i < 100; i++)); do
  if grep -q '印を無効化するため verify のロックを待っています' "$ROOT/mise-readability-restore.out" 2>/dev/null; then readability_waited=1; break; fi
  sleep 0.05
done
chmod 600 "$marker_path"
touch "$readability_lock_release"
wait "$readability_manual_lock" 2>/dev/null; readability_lock_rc=$?
wait "$readability_verify"; readability_verify_rc=$?
[[ "$readability_waited" == 1 && "$readability_lock_rc" == 0 && "$readability_verify_rc" != 0 && ! -e "$marker_path" ]] ||
  failures+=("verify_readability_restored: waited=$readability_waited lock=$readability_lock_rc verify=$readability_verify_rc marker=$(test -e "$marker_path" && echo present || echo missing)")
rm -rf "$MISE_TOY/.git/pelway-verify-started"; chmod u+w "$MISE_TOY/.pelway-verify"; rm -rf "$MISE_TOY/.pelway-verify" "$marker_path"
mkdir "$MISE_TOY/.git/pelway-verify-started"; chmod 000 "$MISE_TOY/.git/pelway-verify-started"
(cd "$MISE_TOY" && MISE_TRUSTED_CONFIG_PATHS="$MISE_TOY/mise.toml" mise run verify >"$ROOT/mise-cleanup-fails.out" 2>&1); cleanup_verify_rc=$?
chmod 700 "$MISE_TOY/.git/pelway-verify-started"
(cd "$MISE_TOY" && PELWAY_AGENT_SKIP_RESIGN=1 bash "$AGENT_GH" dev git push origin HEAD:refs/heads/agent/cleanup-fails >"$ROOT/cleanup-fails-push.out" 2>"$ROOT/cleanup-fails-push.err"); RC=$?; ERR="$(cat "$ROOT/cleanup-fails-push.err")"
[[ "$cleanup_verify_rc" != 0 && -f "$marker_path" && "$RC" != 0 && "$ERR" == *'前回の verify はこの印を無効化しました'* ]] ||
  failures+=("verify_cleanup_fails: verify=$cleanup_verify_rc push=$RC $ERR")
rm -rf "$MISE_TOY/.git/pelway-verify-started" "$MISE_TOY/.pelway-verify"; rm -f "$marker_path"
: >"$lock_path"; chmod 600 "$lock_path"
printf '%s\nverify-id:0123456789abcdef0123456789abcdef\n' "$("$REAL_GIT" -C "$MISE_TOY" rev-parse 'HEAD^{tree}')" >"$marker_path"
lock_held="$ROOT/manual-lock-held"; lock_release="$ROOT/manual-lock-release"; rm -f "$lock_held" "$lock_release"
LOCK_HELD="$lock_held" LOCK_RELEASE="$lock_release" flock -x "$lock_path" -c 'touch "$LOCK_HELD"; while [[ ! -e "$LOCK_RELEASE" ]]; do sleep 0.05; done' &
manual_lock=$!
for ((i = 0; i < 100; i++)); do [[ -e "$lock_held" ]] && break; sleep 0.05; done
(cd "$MISE_TOY" && MISE_TRUSTED_CONFIG_PATHS="$MISE_TOY/mise.toml" mise run verify >"$ROOT/mise-lock-generation.out" 2>&1) &
lock_generation_verify=$!
start_base=""
for ((i = 0; i < 100; i++)); do
  for entry in "$MISE_TOY/.pelway-verify/started"/*; do [[ -e "$entry" ]] && start_base="${entry##*/}"; done
  [[ -n "$start_base" ]] && break
  sleep 0.05
done
chmod a-w "$MISE_TOY/.pelway-verify/started"
: >"$MISE_TOY/.git/pelway-verify-started"
printf '%s\nverify-id:fedcba9876543210fedcba9876543210\n' "$("$REAL_GIT" -C "$MISE_TOY" rev-parse 'HEAD^{tree}')" >"$marker_path"
touch "$lock_release"
wait "$manual_lock" 2>/dev/null; manual_lock_rc=$?
wait "$lock_generation_verify"; lock_generation_rc=$?
[[ "$manual_lock_rc" == 0 && -n "$start_base" && "$lock_generation_rc" != 0 && ! -e "$marker_path" && "$(grep -c 'Traceback' "$ROOT/mise-lock-generation.out" 2>/dev/null)" == 0 ]] ||
  failures+=("verify_lock_generation_unwritable: lock=$manual_lock_rc verify=$lock_generation_rc start=${start_base:-none} marker=$(test -e "$marker_path" && echo present || echo missing) traceback=$(grep -c 'Traceback' "$ROOT/mise-lock-generation.out" 2>/dev/null)")
chmod u+w "$MISE_TOY/.pelway-verify/started" 2>/dev/null
rm -rf "$MISE_TOY/.git/pelway-verify-started" "$MISE_TOY/.pelway-verify"; rm -f "$marker_path"
# gitdir のパスの末尾に空白や CR があっても、3 経路が Git が返した同じパスを使う。
special_n=0
for suffix in ' ' $'\r' $'\n'; do
  special_n=$((special_n + 1)); weird_gitdir="$ROOT/gitdir-with-special$suffix"
  cp -a "$MISE_TOY/.git" "$weird_gitdir"; rm -f "$weird_gitdir/pelway-verify-tree"
  (cd "$MISE_TOY" && GIT_DIR="$weird_gitdir" MISE_TRUSTED_CONFIG_PATHS="$MISE_TOY/mise.toml" mise run verify >"$ROOT/mise-special-gitdir.out" 2>&1)
  rc=$?
  (cd "$MISE_TOY" && GIT_DIR="$weird_gitdir" PELWAY_AGENT_SKIP_RESIGN=1 bash "$AGENT_GH" dev git push origin HEAD:refs/heads/agent/special-gitdir-$special_n >"$ROOT/special-push.out" 2>"$ROOT/special-push.err")
  push_rc=$?
  [[ "$rc" == 0 && "$push_rc" == 0 && -f "$weird_gitdir/pelway-verify-tree" ]] ||
    failures+=("verify_special_gitdir: verify=$rc push=$push_rc suffix=$(printf %q "$suffix") marker=$(test -f "$weird_gitdir/pelway-verify-tree" && echo present || echo missing) err=$(cat "$ROOT/special-push.err")")
done
# hardlink や FIFO の印は通常ファイルとして扱わず、中身を切り詰めたり FIFO を開いたりしない。
printf '%s\nverify-id:0123456789abcdef0123456789abcdef\n' "$("$REAL_GIT" -C "$MISE_TOY" rev-parse 'HEAD^{tree}')" >"$marker_path"
hardlink_copy="$ROOT/marker-hardlink"; ln "$marker_path" "$hardlink_copy"; before="$(cat "$hardlink_copy")"
(cd "$MISE_TOY" && VERIFY_DEPS_RC=1 MISE_TRUSTED_CONFIG_PATHS="$MISE_TOY/mise.toml" mise run verify >"$ROOT/mise-marker-hardlink.out" 2>&1); rc=$?
[[ "$rc" != 0 && "$(cat "$hardlink_copy")" == "$before" && "$(stat -c %h "$marker_path")" == 2 ]] ||
  failures+=("verify_marker_hardlink: rc=$rc links=$(stat -c %h "$marker_path" 2>/dev/null) content=$(cat "$hardlink_copy" 2>/dev/null)")
rm -f "$marker_path" "$hardlink_copy"; mkfifo "$marker_path"
(cd "$MISE_TOY" && VERIFY_DEPS_RC=1 timeout 10 mise run verify >"$ROOT/mise-marker-fifo.out" 2>&1); rc=$?
[[ "$rc" != 0 && "$rc" != 124 ]] || failures+=("verify_marker_fifo: rc=$rc")
rm -f "$marker_path"; rm -rf "$MISE_TOY/.pelway-verify"
# 記録先がディレクトリなら、成功とせず一時印をその中へ移さない。
mkdir "$marker_path"
(cd "$MISE_TOY" && bash "$RECORD" >"$ROOT/record-directory.out" 2>"$ROOT/record-directory.err"); rc=$?
[[ "$rc" != 0 && -d "$marker_path" && "$(cat "$ROOT/record-directory.err")" == *'置けません'* ]] ||
  failures+=("record_marker_directory: rc=$rc err=$(cat "$ROOT/record-directory.err")")
rmdir "$marker_path"
# 親だけが中断されても record がロックを持ち続ける。record の確定を遅らせ、
# 途中で再実行した失敗側が record の終了を待ってから印を消すことを確かめる。
FAKE_MV_BIN="$ROOT/bin-slow-mv"; FAKE_PYTHON_BIN="$ROOT/bin-python"; mkdir -p "$FAKE_MV_BIN" "$FAKE_PYTHON_BIN"; REAL_MV="$(command -v mv)"; REAL_PYTHON3="$(command -v python3)"
cat >"$FAKE_MV_BIN/mv" <<'EOS'
#!/usr/bin/env bash
is_verify_mv=0
[[ "$*" == *pelway-verify-tree* ]] && is_verify_mv=1
if ((is_verify_mv)); then
  touch "$FAKE_MV_STARTED"
  for ((i = 0; i < 100; i++)); do [[ -e "$FAKE_MV_ALLOW" ]] && break; sleep 0.1; done
fi
"$FAKE_MV_REAL" "$@"; rc=$?
if ((is_verify_mv)); then
  touch "$FAKE_MV_DONE"
  if [[ -n "${FAKE_MV_RELEASE:-}" ]]; then
    for ((i = 0; i < 100; i++)); do [[ -e "$FAKE_MV_RELEASE" ]] && break; sleep 0.1; done
  fi
fi
exit "$rc"
EOS
cat >"$FAKE_PYTHON_BIN/python3" <<'EOS'
#!/usr/bin/env bash
[[ "${1:-}" != -I || "${2:-}" != scripts/agent-verify-run || -z "${VERIFY_RUNNER_PID_FILE:-}" ]] || printf '%s\n' "$$" >"$VERIFY_RUNNER_PID_FILE"
exec "$FAKE_PYTHON3_REAL" "$@"
EOS
chmod +x "$FAKE_MV_BIN/mv" "$FAKE_PYTHON_BIN/python3"
# ロック待ちを中断した実行の開始の記録は、開始前にあった印だけを指す。
# あとで確定した別の成功の印では push を止めない。
interrupt_started="$ROOT/mise-interrupt-started" interrupt_first_pid="$ROOT/interrupt-first.pid" interrupt_second_pid="$ROOT/interrupt-second.pid"
rm -f "$interrupt_started" "$interrupt_first_pid" "$interrupt_second_pid" "$marker_path"; rm -rf "$MISE_TOY/.pelway-verify"
(cd "$MISE_TOY" && bash "$RECORD" >/dev/null 2>&1)
(cd "$MISE_TOY" && PATH="$FAKE_PYTHON_BIN:$PATH" FAKE_PYTHON3_REAL="$REAL_PYTHON3" VERIFY_RUNNER_PID_FILE="$interrupt_first_pid" VERIFY_TOUCH_BEFORE_EXIT="$interrupt_started" VERIFY_SLEEP=3 MISE_TRUSTED_CONFIG_PATHS="$MISE_TOY/mise.toml" mise run verify >"$ROOT/mise-interrupt-success.out" 2>&1) &
interrupt_first=$!
for ((i = 0; i < 100; i++)); do [[ -e "$interrupt_started" ]] && break; sleep 0.1; done
(cd "$MISE_TOY" && PATH="$FAKE_PYTHON_BIN:$PATH" FAKE_PYTHON3_REAL="$REAL_PYTHON3" VERIFY_RUNNER_PID_FILE="$interrupt_second_pid" MISE_TRUSTED_CONFIG_PATHS="$MISE_TOY/mise.toml" mise run verify >"$ROOT/mise-interrupt-waiting.out" 2>&1) &
interrupt_second=$!
for ((i = 0; i < 100; i++)); do grep -q '別の verify' "$ROOT/mise-interrupt-waiting.out" 2>/dev/null && break; sleep 0.1; done
interrupt_runner="$(cat "$interrupt_second_pid" 2>/dev/null)"
[[ -n "$interrupt_runner" ]] && kill -INT "$interrupt_runner" 2>/dev/null
wait "$interrupt_second"; interrupted_rc=$?
wait "$interrupt_first"; interrupt_success_rc=$?
(cd "$MISE_TOY" && as 'pelway-dev[bot]' add -A && as 'pelway-dev[bot]' commit -q --allow-empty -m interrupted-verified-files)
(cd "$MISE_TOY" && PELWAY_AGENT_SKIP_RESIGN=1 bash "$AGENT_GH" dev git push origin HEAD:refs/heads/agent/interrupted-success >"$ROOT/interrupted-push.out" 2>"$ROOT/interrupted-push.err"); RC=$?; ERR="$(cat "$ROOT/interrupted-push.err")"
[[ "$interrupt_success_rc" == 0 && "$interrupted_rc" != 0 && "$(started_count "$MISE_TOY/.pelway-verify/started")" -ge 1 && -f "$marker_path" && "$RC" == 0 ]] ||
  failures+=("verify_interrupted_waiting: success=$interrupt_success_rc interrupted=$interrupted_rc started=$(started_count "$MISE_TOY/.pelway-verify/started") push=$RC $ERR")
mv_started="$ROOT/verify-record-mv-started" mv_allow="$ROOT/verify-record-mv-allow" mv_done="$ROOT/verify-record-mv-done" mv_release="$ROOT/verify-record-mv-release"; second_started="$ROOT/verify-second-started" first_pid="$ROOT/verify-first.pid" second_pid="$ROOT/verify-second.pid"
rm -f "$mv_started" "$mv_allow" "$mv_done" "$mv_release" "$second_started" "$first_pid" "$second_pid" "$marker_path"
(cd "$MISE_TOY" && PATH="$FAKE_PYTHON_BIN:$FAKE_MV_BIN:$PATH" FAKE_PYTHON3_REAL="$REAL_PYTHON3" VERIFY_RUNNER_PID_FILE="$first_pid" FAKE_MV_STARTED="$mv_started" FAKE_MV_ALLOW="$mv_allow" FAKE_MV_DONE="$mv_done" FAKE_MV_RELEASE="$mv_release" FAKE_MV_REAL="$REAL_MV" MISE_TRUSTED_CONFIG_PATHS="$MISE_TOY/mise.toml" mise run verify >"$ROOT/mise-interrupted-first.out" 2>&1) &
first_verify=$!
for ((i = 0; i < 100; i++)); do [[ -e "$mv_started" ]] && break; sleep 0.1; done
first_runner="$(cat "$first_pid" 2>/dev/null)"
kill_rc=1
[[ -n "$first_runner" ]] && kill -KILL "$first_runner" 2>/dev/null && kill_rc=0
wait "$first_verify"; first_verify_rc=$?
(cd "$MISE_TOY" && PATH="$FAKE_PYTHON_BIN:$PATH" FAKE_PYTHON3_REAL="$REAL_PYTHON3" VERIFY_RUNNER_PID_FILE="$second_pid" VERIFY_DEPS_RC=1 VERIFY_TOUCH_BEFORE_EXIT="$second_started" MISE_TRUSTED_CONFIG_PATHS="$MISE_TOY/mise.toml" mise run verify >"$ROOT/mise-interrupted-second.out" 2>&1) &
second_verify=$!
for ((i = 0; i < 100; i++)); do [[ -e "$second_pid" ]] && break; sleep 0.1; done
second_runner="$(cat "$second_pid" 2>/dev/null)"
sleep 2
second_started_early=0
[[ -e "$second_started" ]] && second_started_early=1
touch "$mv_allow"
for ((i = 0; i < 100; i++)); do [[ -e "$mv_done" ]] && break; sleep 0.1; done
(cd "$MISE_TOY" && PELWAY_AGENT_SKIP_RESIGN=1 bash "$AGENT_GH" dev git push origin HEAD:refs/heads/agent/interrupted-child-marker >"$ROOT/interrupted-child-push.out" 2>"$ROOT/interrupted-child-push.err"); child_push_rc=$?; child_push_err="$(cat "$ROOT/interrupted-child-push.err")"
touch "$mv_release"
wait "$second_verify"; second_verify_rc=$?
[[ "$kill_rc" == 0 && "$first_verify_rc" != 0 && -n "$second_runner" && "$second_started_early" == 0 && -e "$mv_done" && "$child_push_rc" != 0 && "$child_push_err" == *'前回の verify はこの印を無効化しました'* && -e "$second_started" && "$second_verify_rc" != 0 && ! -e "$marker_path" ]] ||
  failures+=("verify_record_inherits_lock: kill=$kill_rc first=$first_verify_rc second=$second_verify_rc early=$second_started_early runner=${second_runner:-none} mv_done=$(test -e "$mv_done" && echo yes || echo no) child_push=$child_push_rc $child_push_err second=$(test -e "$second_started" && echo yes || echo no) marker=$(test -e "$marker_path" && echo present || echo missing)")
# --- 印が無い push は止まり、理由と mise run verify の案内が出る ---
setup_repo missing
printf 'new implementation\n' >"$REPO_DIR/work/code.txt"
work_git add code.txt; (cd "$REPO_DIR/work" && as 'pelway-dev[bot]' commit -q -m implementation)
push_role missing dev -u origin HEAD
[[ "$RC" != 0 && "$ERR" == *'verify'* && "$ERR" == *'mise run verify'* ]] ||
  failures+=("push_missing_marker: not stopped with guidance: rc=$RC $ERR")
[[ -z "$(remote_ref agent/missing)" ]] || failures+=("push_missing_marker: remote ref changed")
# --- 検証した木と同じ HEAD は通る ---
setup_repo exact
printf 'implementation\n' >"$REPO_DIR/work/code.txt"
(cd "$REPO_DIR/work" && bash "$RECORD" >/dev/null 2>&1)
work_git add code.txt; (cd "$REPO_DIR/work" && as 'pelway-dev[bot]' commit -q -m implementation)
push_role exact dev -u origin HEAD
[[ "$RC" == 0 && "$(remote_ref agent/exact)" == "$(work_git rev-parse HEAD)" ]] ||
  failures+=("push_exact_marker: not pushed: rc=$RC $ERR")
# 開始の記録は印の世代ごとに効く。別世代の記録は残っていても、今の印を指す記録だけが止める。
state_dir="$REPO_DIR/work/.pelway-verify/started"; attempt=0123456789abcdef0123456789abcdef
printf 'old generation\n' >"$state_dir/0000000000000000000000000000000000000000000000000000000000000000.$attempt"
push_role generation_old dev origin HEAD:refs/heads/agent/generation-old
[[ "$RC" == 0 ]] || failures+=("push_generation_old: stopped by another generation: rc=$RC $ERR")
marker_fp="$(sha256sum <"$(marker_file)" | cut -d' ' -f1)"
printf 'failed generation\n' >"$state_dir/$marker_fp.$attempt"
push_role generation_match dev origin HEAD:refs/heads/agent/generation-match
[[ "$RC" != 0 && "$ERR" == *'前回の verify はこの印を無効化しました'* ]] ||
  failures+=("push_generation_match: not stopped: rc=$RC $ERR")
rm -f "$state_dir/$marker_fp.$attempt"
printf 'unknown generation
' >"$state_dir/any.$attempt"
push_role generation_any dev origin HEAD:refs/heads/agent/generation-any
[[ "$RC" != 0 && "$ERR" == *'前回の verify はこの印を無効化しました'* ]] ||
  failures+=("push_generation_any: not stopped: rc=$RC $ERR")
rm -f "$state_dir/any.$attempt"
# gitdir に書けず印を無効化できない状態では、正しい印が残っていても止める。
chmod a-w "$REPO_DIR/work/.git"
push_role readonly_gate dev origin HEAD:refs/heads/agent/readonly-gate
chmod u+w "$REPO_DIR/work/.git"
[[ "$RC" != 0 && "$ERR" == *'印を無効化できない状態'* ]] ||
  failures+=("push_readonly_gitdir: not stopped: rc=$RC $ERR")
[[ -z "$(remote_ref agent/readonly-gate)" ]] || failures+=("push_readonly_gitdir: remote ref changed")
# 読み取り専用の間に verify が印を無効化できなかったときは、書き込みの回復後も古い印で通さない。
chmod -R a-w "$REPO_DIR/work/.git"
(cd "$REPO_DIR/work" && "$VERIFY_RUN" >"$ROOT/readonly-verify.out" 2>&1); readonly_verify_rc=$?
chmod -R u+w "$REPO_DIR/work/.git"
push_role readonly_after dev origin HEAD:refs/heads/agent/readonly-after
[[ "$readonly_verify_rc" != 0 && "$RC" != 0 && "$ERR" == *'前回の verify はこの印を無効化しました'* ]] ||
  failures+=("verify_readonly_gitdir: verify=$readonly_verify_rc push=$RC $ERR")
[[ -z "$(remote_ref agent/readonly-after)" ]] || failures+=("verify_readonly_gitdir: remote ref changed")
# ロックを取った後に印の unlink が失敗したときは、ロック取得時点の世代を指す記録で止める。
# gitdir そのものを書けなくすると lock file を作れないため、lock file を先に作り、dir だけ書けなくする。
lock_file="$REPO_DIR/work/.git/pelway-verify.lock"
rm -f "$lock_file"; : >"$lock_file"; chmod 600 "$lock_file"
chmod a-w "$REPO_DIR/work/.git"
(cd "$REPO_DIR/work" && "$VERIFY_RUN" >"$ROOT/invalidate-fails-verify.out" 2>&1); invalidate_verify_rc=$?
chmod u+w "$REPO_DIR/work/.git"
push_role invalidate_fails dev origin HEAD:refs/heads/agent/invalidate-fails
[[ "$invalidate_verify_rc" != 0 && "$RC" != 0 && "$ERR" == *'前回の verify はこの印を無効化しました'* ]] ||
  failures+=("verify_invalidate_fails: verify=$invalidate_verify_rc push=$RC $ERR")
[[ -z "$(remote_ref agent/invalidate-fails)" ]] || failures+=("verify_invalidate_fails: remote ref changed")
# .pelway-verify そのものが symlink なら、リンク先の started が正しい形でも止める。
setup_repo control_symlink
printf 'implementation\n' >"$REPO_DIR/work/code.txt"
(cd "$REPO_DIR/work" && bash "$RECORD" >/dev/null 2>&1)
work_git add code.txt; (cd "$REPO_DIR/work" && as 'pelway-dev[bot]' commit -q -m implementation)
mkdir -p "$ROOT/control-target/started"; chmod 700 "$ROOT/control-target" "$ROOT/control-target/started"
rm -rf "$REPO_DIR/work/.pelway-verify"; ln -s "$ROOT/control-target" "$REPO_DIR/work/.pelway-verify"
rm -rf "$ROOT/control-target/started"
(cd "$REPO_DIR/work" && bash "$RECORD" >"$ROOT/record-control-symlink.out" 2>"$ROOT/record-control-symlink.err"); record_symlink_rc=$?
[[ "$record_symlink_rc" != 0 && ! -e "$ROOT/control-target/started" ]] ||
  failures+=("record_control_symlink: rc=$record_symlink_rc target_started=$(test -e "$ROOT/control-target/started" && echo present || echo missing)")
mkdir -p "$ROOT/control-target/started"; chmod 700 "$ROOT/control-target/started"
push_role control_symlink dev -u origin HEAD
[[ "$RC" != 0 && "$ERR" == *'信頼できるディレクトリではありません'* ]] ||
  failures+=("push_control_symlink: not stopped: rc=$RC $ERR")
[[ -z "$(remote_ref agent/control_symlink)" ]] || failures+=("push_control_symlink: remote ref changed")
# --- HEAD と印の差が docs/worklogs/ の下だけなら通す（実装コミットとメモコミットの両方） ---
setup_repo worklogs
printf 'implementation\n' >"$REPO_DIR/work/code.txt"
mkdir -p "$REPO_DIR/work/docs/worklogs"
printf 'memo\n' >"$REPO_DIR/work/docs/worklogs/issue-807.md"
(cd "$REPO_DIR/work" && bash "$RECORD" >/dev/null 2>&1)
work_git add code.txt; (cd "$REPO_DIR/work" && as 'pelway-dev[bot]' commit -q -m implementation)
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
(cd "$REPO_DIR/work" && bash "$RECORD" >/dev/null 2>&1)
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
(cd "$REPO_DIR/work" && bash "$RECORD" >/dev/null 2>&1)
work_git add code.txt; (cd "$REPO_DIR/work" && as 'pelway-dev[bot]' commit -q -m implementation)
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
# --- ブランチの削除とタグだけの push は印が無くても通し、ブランチ更新を含む push は止める ---
setup_repo delete_only
printf 'implementation\n' >"$REPO_DIR/work/code.txt"; work_git add code.txt; (cd "$REPO_DIR/work" && as 'pelway-dev[bot]' commit -q -m implementation)
(cd "$REPO_DIR/work" && PELWAY_AGENT_SKIP_RESIGN=1 PELWAY_AGENT_SKIP_VERIFY_GATE=1 bash "$AGENT_GH" dev git push -q -u origin HEAD >/dev/null 2>"$ROOT/delete-only-seed.err")
[[ "$?" == 0 && -n "$(remote_ref agent/delete_only)" ]] ||
  failures+=("push_delete_only_seed: remote branch was not created: $(cat "$ROOT/delete-only-seed.err")")
rm -f "$(marker_file)"
(cd "$REPO_DIR/work" && PELWAY_AGENT_SKIP_RESIGN=1 bash "$AGENT_GH" dev git push origin ":refs/heads/agent/delete_only" >"$ROOT/delete-only.out" 2>"$ROOT/delete-only.err"); RC=$?; ERR="$(cat "$ROOT/delete-only.err")"
[[ "$RC" == 0 && -z "$(remote_ref agent/delete_only)" ]] ||
  failures+=("push_delete_only: branch deletion not passed: rc=$RC $ERR")
setup_repo tag_only
printf 'implementation\n' >"$REPO_DIR/work/code.txt"; work_git add code.txt; (cd "$REPO_DIR/work" && as 'pelway-dev[bot]' commit -q -m implementation)
work_git tag verify-gate-tag
push_role tag_only dev origin refs/tags/verify-gate-tag
[[ "$RC" == 0 && -n "$("$REAL_GIT" --git-dir="$REPO_DIR/remote.git" rev-parse -q --verify refs/tags/verify-gate-tag 2>/dev/null)" ]] ||
  failures+=("push_tag_only: tag-only push not passed: rc=$RC $ERR")
setup_repo mixed_tag_branch
printf 'implementation\n' >"$REPO_DIR/work/code.txt"; work_git add code.txt; (cd "$REPO_DIR/work" && as 'pelway-dev[bot]' commit -q -m implementation)
work_git tag mixed-tag
push_role mixed_tag_branch dev origin refs/tags/mixed-tag HEAD:refs/heads/agent/mixed_tag_branch
[[ "$RC" != 0 && "$ERR" == *'mise run verify'* ]] ||
  failures+=("push_mixed_tag_branch: not stopped: rc=$RC $ERR")
[[ -z "$("$REAL_GIT" --git-dir="$REPO_DIR/remote.git" rev-parse -q --verify refs/tags/mixed-tag 2>/dev/null)" && -z "$(remote_ref agent/mixed_tag_branch)" ]] ||
  failures+=("push_mixed_tag_branch: remote refs changed")
# --- PELWAY_AGENT_SKIP_VERIFY_GATE は 1 のときだけ外れる。0 では外れない ---
setup_repo skip_zero
printf 'implementation\n' >"$REPO_DIR/work/code.txt"; work_git add code.txt; (cd "$REPO_DIR/work" && as 'pelway-dev[bot]' commit -q -m implementation)
(cd "$REPO_DIR/work" && PELWAY_AGENT_SKIP_RESIGN=1 PELWAY_AGENT_SKIP_VERIFY_GATE=0 bash "$AGENT_GH" dev git push -u origin HEAD >"$ROOT/skip_zero.out" 2>"$ROOT/skip_zero.err"); RC=$?; ERR="$(cat "$ROOT/skip_zero.err")"
[[ "$RC" != 0 && "$ERR" == *'mise run verify'* ]] ||
  failures+=("push_skip_zero: skipped with PELWAY_AGENT_SKIP_VERIFY_GATE=0: rc=$RC $ERR")
[[ -z "$(remote_ref agent/skip_zero)" ]] || failures+=("push_skip_zero: remote ref changed")
# --- PELWAY_AGENT_SKIP_VERIFY_GATE=1 は確かめずに通し、通したことを標準エラーに出す ---
setup_repo skip
printf 'implementation\n' >"$REPO_DIR/work/code.txt"; work_git add code.txt; (cd "$REPO_DIR/work" && as 'pelway-dev[bot]' commit -q -m implementation)
(cd "$REPO_DIR/work" && PELWAY_AGENT_SKIP_RESIGN=1 PELWAY_AGENT_SKIP_VERIFY_GATE=1 bash "$AGENT_GH" dev git push -u origin HEAD >"$ROOT/skip.out" 2>"$ROOT/skip.err"); RC=$?; ERR="$(cat "$ROOT/skip.err")"
[[ "$RC" == 0 && "$(remote_ref agent/skip)" == "$(work_git rev-parse HEAD)" ]] ||
  failures+=("push_skip_gate: not pushed: rc=$RC $ERR")
[[ "$ERR" == *'PELWAY_AGENT_SKIP_VERIFY_GATE=1'* && "$ERR" == *'確かめず'* ]] ||
  failures+=("push_skip_gate: skip notice missing: $ERR")
if ((${#failures[@]})); then
  printf 'FAIL %s\n' "${failures[@]}"
  exit 1
fi
echo ok
