#!/usr/bin/env bash
# scripts/agent-gh のテスト。偽の fnox・gh・git を PATH の先頭に置き、渡されたトークンと引数を確かめる。
set -uo pipefail

SCRIPT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/agent-gh"
ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT
failures=()

BIN="$ROOT/bin"
mkdir -p "$BIN"

# 偽の fnox: `fnox -c <config> get NAME` に FAKE_<NAME> の値を返す。値が空なら失敗する。
cat >"$BIN/fnox" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$*" >>"$CALL_LOG.fnox-args"
[[ "$1" == "-c" && "$3" == "get" ]] || exit 9
var="FAKE_$4"
[[ -n "${!var:-}" ]] || exit 1
printf '%s\n' "${!var}"
EOF

# 偽の gh: `gh token generate` は JSON を返す。それ以外は GH_TOKEN と引数を記録する。
# GH_TOKEN_GENERATE_FAIL があれば generate は stderr 2 行を出して失敗する（#520 の失敗経路）。
# GH_EXTENSION_LIST / GH_EXTENSION_LIST_FAIL で `gh extension list` の応答を切り替える。
cat >"$BIN/gh" <<'EOF'
#!/usr/bin/env bash
if [[ "$1 $2" == "token generate" ]]; then
  printf '%s\n' "$*" >"$CALL_LOG.token-args"
  # --key <file> の中身と権限を記録する（秘密鍵が argv に載らないことの確認用）
  prev=""
  for a in "$@"; do
    if [[ "$prev" == "--key" ]]; then
      cat "$a" >"$CALL_LOG.key-content"
      stat -c %a "$a" >"$CALL_LOG.key-mode"
      printf '%s\n' "$a" >"$CALL_LOG.key-path"
    fi
    prev="$a"
  done
  # 失敗の演出: GH_TOKEN_GENERATE_FAIL があれば stderr 2 行（1 行目だけ出るべき）を出して終了 1
  if [[ -n "${GH_TOKEN_GENERATE_FAIL:-}" ]]; then
    printf '%s\n' "${GH_TOKEN_GENERATE_STDERR_1:-}" >&2
    printf '%s\n' "${GH_TOKEN_GENERATE_STDERR_2:-second line must not appear}" >&2
    exit 1
  fi
  # App ID ごとに別のトークンを返す（333 = 実装担当 pelway-dev、111 = レビュー担当 pelway-review、555 = QA 担当 pelway-qa）
  case "$*" in
    *"--app-id 333 "*) printf '{"token":"dev-token","expires_at":"x"}\n' ;;
    *"--app-id 111 "*) printf '{"token":"review-token","expires_at":"x"}\n' ;;
    *"--app-id 555 "*) printf '{"token":"qa-token","expires_at":"x"}\n' ;;
    *) exit 1 ;;
  esac
  exit 0
fi
if [[ "$1 $2" == "extension list" ]]; then
  if [[ -n "${GH_EXTENSION_LIST_FAIL:-}" ]]; then exit 1; fi
  printf '%s\n' "${GH_EXTENSION_LIST:-gh-token  v2.0.10  Link-/gh-token}"
  exit 0
fi
{ printf 'GH_TOKEN=%s\n' "${GH_TOKEN:-}"; printf 'ARGS=%s\n' "$*"; } >"$CALL_LOG"
EOF

# 偽の git: GH_TOKEN と引数を追記する（push 後の agent-gh-resign からの呼び出しも残すため追記）。
cat >"$BIN/git" <<'EOF'
#!/usr/bin/env bash
{ printf 'GH_TOKEN=%s\n' "${GH_TOKEN:-}"; printf 'ARGS=%s\n' "$*"; } >>"$CALL_LOG"
EOF
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
export FAKE_PELWAY_REVIEW_APP_KEY=pem-body
export FAKE_PELWAY_QA_APP_ID=555
export FAKE_PELWAY_QA_INSTALLATION_ID=666
export FAKE_PELWAY_QA_PRIVATE_KEY=qa-pem-body

# run <name> <args...>: CALL_LOG を新しくして agent-gh を実行し、出力を OUT、終了コードを RC に入れる。
run() {
  local name="$1"
  shift
  export CALL_LOG="$ROOT/$name.log"
  rm -f "$CALL_LOG" "$CALL_LOG.fnox-args"
  OUT="$(bash "$SCRIPT" "$@" 2>&1)"
  RC=$?
}

expect_log() {
  local name="$1" pattern="$2"
  grep -qF -- "$pattern" "$ROOT/$name.log" 2>/dev/null || failures+=("$name: log lacks '$pattern'")
}

run dev_mints_token dev pr list
expect_log dev_mints_token "GH_TOKEN=dev-token"
expect_log dev_mints_token "ARGS=pr list"
grep -qF -- "--app-id 333 --installation-id 444" "$ROOT/dev_mints_token.log.token-args" 2>/dev/null ||
  failures+=("dev_mints_token: token generate args wrong")
[[ "$(cat "$ROOT/dev_mints_token.log.key-content" 2>/dev/null)" == "dev-pem-body" ]] || failures+=("dev_mints_token: --key file lacks dev key")

grep -qF -- "-c $ROOT/agent-fnox.toml get PELWAY_DEV_APP_ID" "$ROOT/dev_mints_token.log.fnox-args" 2>/dev/null ||
  failures+=("uses_machine_config: fnox not called with PELWAY_AGENT_FNOX_CONFIG")

run token_not_leaked dev pr list
[[ "$OUT" != *dev-token* ]] || failures+=("token_not_leaked: token printed")

run review_mints_token review pr review 1 --approve
expect_log review_mints_token "GH_TOKEN=review-token"
expect_log review_mints_token "ARGS=pr review 1 --approve"
grep -qF -- "--app-id 111 --installation-id 222" "$ROOT/review_mints_token.log.token-args" 2>/dev/null ||
  failures+=("review_mints_token: token generate args wrong")
[[ "$OUT" != *review-token* && "$OUT" != *pem-body* ]] || failures+=("review_mints_token: secret printed")
L="$ROOT/review_mints_token.log"
! grep -qF -- "pem-body" "$L.token-args" 2>/dev/null || failures+=("review_key_not_in_argv: key on command line")
! grep -qF -- "$(printf pem-body | base64 -w 0)" "$L.token-args" 2>/dev/null || failures+=("review_key_not_in_argv: base64 key on command line")
[[ "$(cat "$L.key-content" 2>/dev/null)" == "pem-body" ]] || failures+=("review_key_file: --key file lacks key")
[[ "$(cat "$L.key-mode" 2>/dev/null)" == "600" ]] || failures+=("review_key_file: mode not 600")
[[ -n "$(cat "$L.key-path" 2>/dev/null)" && ! -e "$(cat "$L.key-path" 2>/dev/null)" ]] || failures+=("review_key_file: key file not removed")

# review ロールは review 専用の fnox 設定（PELWAY_AGENT_FNOX_CONFIG_REVIEW）だけを読む（#449 の鍵分割）。
# dev 用の設定からは 1 項目も読まない（PELWAY_DEV_* が review 側から復号できない構成にするため）。
grep -qF -- "-c $ROOT/agent-fnox-review.toml get PELWAY_REVIEW_APP_ID" "$ROOT/review_mints_token.log.fnox-args" 2>/dev/null ||
  failures+=("review_uses_review_config: fnox not called with PELWAY_AGENT_FNOX_CONFIG_REVIEW")
! grep -qF -- "-c $ROOT/agent-fnox.toml get" "$ROOT/review_mints_token.log.fnox-args" 2>/dev/null ||
  failures+=("review_uses_review_config: review role read the dev config")

# dev ロールは dev 用の設定のまま（review 用設定を読まない）
run dev_uses_dev_config dev pr list
grep -qF -- "-c $ROOT/agent-fnox.toml get PELWAY_DEV_APP_ID" "$ROOT/dev_uses_dev_config.log.fnox-args" 2>/dev/null ||
  failures+=("dev_uses_dev_config: fnox not called with PELWAY_AGENT_FNOX_CONFIG")
! grep -qF -- "-c $ROOT/agent-fnox-review.toml get" "$ROOT/dev_uses_dev_config.log.fnox-args" 2>/dev/null ||
  failures+=("dev_uses_dev_config: dev role read the review config")

# review 用の設定に秘密が無ければ fail-closed（exit 3）。gh も呼ばない
FAKE_PELWAY_REVIEW_APP_KEY="" run review_missing_secret_fails_closed review pr list
[[ "$RC" == 3 ]] || failures+=("review_missing_secret_fails_closed: exit $RC, want 3")
[[ ! -e "$ROOT/review_missing_secret_fails_closed.log" ]] || failures+=("review_missing_secret_fails_closed: gh was called")

# --- gh token generate の失敗（#520）: 本当の原因が分かるようにする ---
# 使う値はすべて合成のダミー（FAKE_* の 333 / 444 / dev-pem-body もダミー）。外のネットワークにも本物の秘密にも触れない。

# stderr の 1 行目を、トークンの形を伏せて出す。2 行目は出さない。終了コードは 3 のまま。
GH_TOKEN_GENERATE_FAIL=1 \
GH_TOKEN_GENERATE_STDERR_1='gh: read /home/x/.config/gh/config.yml: permission denied (token ghp_abcdefghijklmnopqrstuvwxyz0123456789)' \
GH_TOKEN_GENERATE_STDERR_2='second line must not appear' \
  run token_fail_shows_first_stderr_line dev pr list
[[ "$RC" == 3 ]] || failures+=("token_fail_shows_first_stderr_line: exit $RC, want 3")
grep -qF 'permission denied' <<<"$OUT" || failures+=("token_fail_shows_first_stderr_line: first stderr line not shown")
grep -qF 'gh?_' <<<"$OUT" || failures+=("token_fail_shows_first_stderr_line: token not masked")
[[ "$OUT" != *ghp_abcdefghijklmnopqrstuvwxyz0123456789* ]] || failures+=("token_fail_shows_first_stderr_line: token value leaked")
[[ "$OUT" != *'second line must not appear'* ]] || failures+=("token_fail_shows_first_stderr_line: second stderr line shown")

# 出す行に鍵・トークン・APP ID・INSTALLATION ID の値が入らない（ダミー値で確かめる）
GH_TOKEN_GENERATE_FAIL=1 \
GH_TOKEN_GENERATE_STDERR_1='sign failed app=333 installation=444 key=dev-pem-body value=ghs_0123456789abcdefghijklmn' \
  run token_fail_masks_secret_values dev pr list
[[ "$RC" == 3 ]] || failures+=("token_fail_masks_secret_values: exit $RC, want 3")
[[ "$OUT" != *333* ]] || failures+=("token_fail_masks_secret_values: app id value leaked")
[[ "$OUT" != *444* ]] || failures+=("token_fail_masks_secret_values: installation id value leaked")
[[ "$OUT" != *dev-pem-body* ]] || failures+=("token_fail_masks_secret_values: key value leaked")
[[ "$OUT" != *ghs_0123456789abcdefghijklmn* ]] || failures+=("token_fail_masks_secret_values: token value leaked")

# stderr が空のときは 1 行目を出せないので、終了コードで原因の種別だけ示す
GH_TOKEN_GENERATE_FAIL=1 \
GH_TOKEN_GENERATE_STDERR_1='' \
  run token_fail_empty_stderr dev pr list
[[ "$RC" == 3 ]] || failures+=("token_fail_empty_stderr: exit $RC, want 3")
grep -qF '終了コード' <<<"$OUT" || failures+=("token_fail_empty_stderr: exit code not shown")

# 「拡張が必要」の文は gh extension list に gh-token があるときは出さない
GH_TOKEN_GENERATE_FAIL=1 \
GH_EXTENSION_LIST=$'gh-token  v2.0.10  Link-/gh-token\ngh-stack  v0.2.0  vi/weird' \
  run token_fail_no_install_hint_when_present dev pr list
[[ "$RC" == 3 ]] || failures+=("token_fail_no_install_hint_when_present: exit $RC, want 3")
[[ "$OUT" != *'gh extension install'* ]] || failures+=("token_fail_no_install_hint_when_present: install hint shown despite extension present")

# gh extension list に gh-token が無いときだけ「拡張が必要」を出す
GH_TOKEN_GENERATE_FAIL=1 \
GH_EXTENSION_LIST=$'gh-stack  v0.2.0  vi/weird' \
  run token_fail_install_hint_when_absent dev pr list
[[ "$RC" == 3 ]] || failures+=("token_fail_install_hint_when_absent: exit $RC, want 3")
grep -qF 'gh extension install Link-/gh-token' <<<"$OUT" || failures+=("token_fail_install_hint_when_absent: install hint missing")

# gh extension list 自体が失敗したら有無は分からないので「拡張が必要」は出さない
GH_TOKEN_GENERATE_FAIL=1 \
GH_EXTENSION_LIST_FAIL=1 \
  run token_fail_no_install_hint_when_list_fails dev pr list
[[ "$RC" == 3 ]] || failures+=("token_fail_no_install_hint_when_list_fails: exit $RC, want 3")
[[ "$OUT" != *'gh extension install'* ]] || failures+=("token_fail_no_install_hint_when_list_fails: install hint shown despite unknown extension state")

run dev_git_identity dev git commit -m x
expect_log dev_git_identity "GH_TOKEN=dev-token"
expect_log dev_git_identity "-c user.name=pelway-dev[bot]"
expect_log dev_git_identity "-c user.email=9+pelway-dev[bot]@users.noreply.github.com"
# 環境の署名設定が App 名義のコミットに混ざらないよう、ローカル署名は常に切る
expect_log dev_git_identity "-c commit.gpgsign=false"
expect_log dev_git_identity "-c credential.helper= -c credential.helper=!gh auth git-credential"
expect_log dev_git_identity "commit -m x"
# origin が SSH でも、push / fetch を HTTPS（= GH_TOKEN の名義）に向ける。SSH のままだと hatai の鍵で push される。
expect_log dev_git_identity "-c url.https://github.com/.insteadOf=git@github.com:"
expect_log dev_git_identity "-c url.https://github.com/.insteadOf=ssh://git@github.com/"
expect_log dev_git_identity "-c url.https://github.com/.pushInsteadOf=git@github.com:"
expect_log dev_git_identity "-c url.https://github.com/.pushInsteadOf=ssh://git@github.com/"

# git push の成功後は agent-gh-resign が upstream の確かめ（rev-parse @{u}）まで走る。
# リポジトリ外で動かす（resign は upstream が無ければ何もしない）。
mkdir -p "$ROOT/norepo"
export CALL_LOG="$ROOT/dev_git_push.log"
rm -f "$CALL_LOG"
(cd "$ROOT/norepo" && bash "$SCRIPT" dev git push -u origin HEAD >/dev/null 2>&1)
push_rc=$?
[[ "$push_rc" == 0 ]] || failures+=("dev_git_push: exit $push_rc, want 0")
expect_log dev_git_push "push -u origin HEAD"
# resign が upstream を確かめる rev-parse を呼んだ形跡（= push 後に走った証拠）
expect_log dev_git_push "rev-parse --abbrev-ref --symbolic-full-name @{u}"

# PELWAY_AGENT_SKIP_RESIGN=1 のときは resign を呼ばない
export CALL_LOG="$ROOT/dev_git_push_skip.log"
rm -f "$CALL_LOG"
(cd "$ROOT/norepo" && PELWAY_AGENT_SKIP_RESIGN=1 bash "$SCRIPT" dev git push >/dev/null 2>&1)
push_rc=$?
[[ "$push_rc" == 0 ]] || failures+=("dev_git_push_skip: exit $push_rc, want 0")
expect_log dev_git_push_skip "push"
! grep -qF 'rev-parse' "$ROOT/dev_git_push_skip.log" || failures+=("dev_git_push_skip: resign ran despite skip")

run bad_role admin pr list
[[ "$RC" == 2 ]] || failures+=("bad_role: exit $RC, want 2")

# run_help <name> <args...>: run と同じだが stderr を <name>.stderr に分け、OUT には標準出力だけを入れる。
run_help() {
  local name="$1"
  shift
  export CALL_LOG="$ROOT/$name.log"
  rm -f "$CALL_LOG" "$CALL_LOG.fnox-args" "$CALL_LOG.token-args" "$ROOT/$name.stderr"
  OUT="$(bash "$SCRIPT" "$@" 2>"$ROOT/$name.stderr")"
  RC=$?
}

# expect_no_calls <name>: fnox・gh・git のどれも呼ばれていない（呼ぶと CALL_LOG 系ができる）
expect_no_calls() {
  local name="$1"
  local log="$ROOT/$name.log"
  [[ ! -e "$log" && ! -e "$log.fnox-args" && ! -e "$log.token-args" ]] ||
    failures+=("$name: fnox or gh was called")
}

# --help: usage: で始まる行を標準出力に出し、終了コード 0。fnox も gh も呼ばない。
run_help help_long --help
[[ "$RC" == 0 ]] || failures+=("help_long: exit $RC, want 0")
grep -q '^usage:' <<<"$OUT" || failures+=("help_long: stdout lacks 'usage:' line")
[[ ! -s "$ROOT/help_long.stderr" ]] || failures+=("help_long: wrote to stderr")
expect_no_calls help_long

# -h: --help と同じ動き
run_help help_short -h
[[ "$RC" == 0 ]] || failures+=("help_short: exit $RC, want 0")
grep -q '^usage:' <<<"$OUT" || failures+=("help_short: stdout lacks 'usage:' line")
[[ ! -s "$ROOT/help_short.stderr" ]] || failures+=("help_short: wrote to stderr")
expect_no_calls help_short

# 認証情報が無い環境（fnox が何も返さない）でも --help は終了コード 0
export CALL_LOG="$ROOT/help_no_credentials.log"
rm -f "$CALL_LOG" "$CALL_LOG.fnox-args" "$CALL_LOG.token-args"
OUT="$(env -u FAKE_PELWAY_DEV_APP_ID -u FAKE_PELWAY_DEV_APP_INSTALLATION_ID -u FAKE_PELWAY_DEV_APP_KEY \
  -u FAKE_PELWAY_DEV_GIT_EMAIL -u FAKE_PELWAY_REVIEW_APP_ID -u FAKE_PELWAY_REVIEW_APP_INSTALLATION_ID \
  -u FAKE_PELWAY_REVIEW_APP_KEY bash "$SCRIPT" --help 2>"$ROOT/help_no_credentials.stderr")"
RC=$?
[[ "$RC" == 0 ]] || failures+=("help_no_credentials: exit $RC, want 0")
grep -q '^usage:' <<<"$OUT" || failures+=("help_no_credentials: stdout lacks 'usage:' line")
expect_no_calls help_no_credentials

FAKE_PELWAY_DEV_APP_KEY="" run missing_token_fails_closed dev pr list
[[ "$RC" == 3 ]] || failures+=("missing_token_fails_closed: exit $RC, want 3")
[[ ! -e "$ROOT/missing_token_fails_closed.log" ]] || failures+=("missing_token_fails_closed: gh was called")

FAKE_PELWAY_DEV_GIT_EMAIL="" run missing_email_fails_closed dev git push
[[ "$RC" == 3 ]] || failures+=("missing_email_fails_closed: exit $RC, want 3")
[[ ! -e "$ROOT/missing_email_fails_closed.log" ]] || failures+=("missing_email_fails_closed: git was called")

# \uXXXX のエスケープが残った引数は、fnox・gh・git のどれも呼ばず終了コード 5 で止める。
# ハーネス層で非 ASCII が潰れた状態で届く引数をそのまま流すと壊れた文字列が確定するため。
run unicode_guard_dev dev issue create --title 'x\u30b9y'
[[ "$RC" == 5 ]] || failures+=("unicode_guard_dev: exit $RC, want 5")
[[ "$OUT" == *'\uXXXX'* ]] || failures+=("unicode_guard_dev: no guidance")
# 拒否経路は引数の値を複製しない（秘密が混入した引数を stderr へ露出させない）
[[ "$OUT" != *'x\u30b9y'* ]] || failures+=("unicode_guard_dev: arg value echoed")
[[ ! -e "$ROOT/unicode_guard_dev.log" && ! -e "$ROOT/unicode_guard_dev.log.fnox-args" ]] ||
  failures+=("unicode_guard_dev: fnox or gh was called")

run unicode_guard_review review pr review 1 --approve --body 'ok\u1234'
[[ "$RC" == 5 ]] || failures+=("unicode_guard_review: exit $RC, want 5")
[[ ! -e "$ROOT/unicode_guard_review.log" && ! -e "$ROOT/unicode_guard_review.log.fnox-args" ]] ||
  failures+=("unicode_guard_review: fnox or gh was called")

run unicode_guard_git dev git commit -m 'msg \u3042'
[[ "$RC" == 5 ]] || failures+=("unicode_guard_git: exit $RC, want 5")
[[ ! -e "$ROOT/unicode_guard_git.log" ]] || failures+=("unicode_guard_git: git was called")

# 16進が 4 桁未満・大文字 U は検出対象外（誤検知しない）
run unicode_guard_short_hex dev pr list --search 'a\u12b'
[[ "$RC" == 0 ]] || failures+=("unicode_guard_short_hex: exit $RC, want 0")
expect_log unicode_guard_short_hex 'ARGS=pr list --search a\u12b'

run unicode_guard_capital_u dev pr list --search 'a\U3042'
[[ "$RC" == 0 ]] || failures+=("unicode_guard_capital_u: exit $RC, want 0")
expect_log unicode_guard_capital_u 'ARGS=pr list --search a\U3042'

# qa ロール: `qa comment <番号> --body-file <path|->` の 1 形だけ許可する（許可リスト方式）。
# それ以外の呼び出しは、fnox（秘密の読み出し）と gh / git（ネットワーク）のどちらも呼ばずに拒否する。
printf 'qa body\n' >"$ROOT/qa-body.md"

run qa_comment_posts qa comment 42 --body-file "$ROOT/qa-body.md"
[[ "$RC" == 0 ]] || failures+=("qa_comment_posts: exit $RC, want 0 ($OUT)")
expect_log qa_comment_posts "GH_TOKEN=qa-token"
expect_log qa_comment_posts "issue comment 42"
expect_log qa_comment_posts "--repo hatai/Pelway"
expect_log qa_comment_posts "--body-file $ROOT/qa-body.md"
grep -qF -- "--app-id 555 --installation-id 666" "$ROOT/qa_comment_posts.log.token-args" 2>/dev/null ||
  failures+=("qa_comment_posts: token generate args wrong")
# dev / review と秘密の名前が違う（PELWAY_QA_INSTALLATION_ID・PELWAY_QA_PRIVATE_KEY）ことを確かめる
grep -qF -- "-c $ROOT/agent-fnox.toml get PELWAY_QA_PRIVATE_KEY" "$ROOT/qa_comment_posts.log.fnox-args" 2>/dev/null ||
  failures+=("qa_comment_posts: fnox not called with PELWAY_QA_PRIVATE_KEY")
[[ "$OUT" != *qa-pem-body* && "$OUT" != *qa-token* ]] || failures+=("qa_comment_posts: secret printed")

# --body-file - は標準入力をそのまま gh に渡す
export CALL_LOG="$ROOT/qa_comment_stdin.log"
rm -f "$CALL_LOG" "$CALL_LOG.fnox-args" "$CALL_LOG.token-args"
OUT="$(printf 'from stdin\n' | bash "$SCRIPT" qa comment 7 --body-file - 2>&1)"
RC=$?
[[ "$RC" == 0 ]] || failures+=("qa_comment_stdin: exit $RC, want 0 ($OUT)")
expect_log qa_comment_stdin "GH_TOKEN=qa-token"
expect_log qa_comment_stdin "issue comment 7"
expect_log qa_comment_stdin "--body-file -"

# qa_reject <name> <args...>: `qa` の後に続く引数が許可の形でなければ、終了コード 2 で何も呼ばずに拒否する
qa_reject() {
  local name="$1"
  shift
  run "$name" qa "$@"
  [[ "$RC" == 2 ]] || failures+=("$name: exit $RC, want 2")
  expect_no_calls "$name"
}
qa_reject qa_rejects_no_args
qa_reject qa_rejects_help --help
qa_reject qa_rejects_pr_list pr list
qa_reject qa_rejects_api api "repos/hatai/Pelway/issues/1/comments" -X POST
qa_reject qa_rejects_git git push
qa_reject qa_rejects_label pr edit 1 --add-label automerge
qa_reject qa_rejects_approve pr review 1 --approve
qa_reject qa_rejects_issue_edit issue edit 1 --title x
qa_reject qa_rejects_pr_merge pr merge 1
qa_reject qa_rejects_issue_comment issue comment 1 --body-file "$ROOT/qa-body.md"
qa_reject qa_rejects_bad_number comment abc --body-file "$ROOT/qa-body.md"
qa_reject qa_rejects_bad_flag comment 1 --body "$ROOT/qa-body.md"
qa_reject qa_rejects_no_body_file comment 1 --body-file
qa_reject qa_rejects_extra_arg comment 1 --body-file "$ROOT/qa-body.md" --silent
qa_reject qa_rejects_body_file_missing comment 1 --body-file "$ROOT/nonexistent.md"

# PELWAY_AGENT_FNOX_CONFIG 未設定の qa は、既定値（dev/review 用の設定）に落ちず終了コード 3 で拒否する
export CALL_LOG="$ROOT/qa_missing_config.log"
rm -f "$CALL_LOG" "$CALL_LOG.fnox-args" "$CALL_LOG.token-args"
OUT="$(env -u PELWAY_AGENT_FNOX_CONFIG bash "$SCRIPT" qa comment 1 --body-file "$ROOT/qa-body.md" 2>&1)"
RC=$?
[[ "$RC" == 3 ]] || failures+=("qa_missing_config: exit $RC, want 3")
expect_no_calls qa_missing_config
[[ "$OUT" == *PELWAY_AGENT_FNOX_CONFIG* ]] || failures+=("qa_missing_config: no guidance")

# qa の引数にも \uXXXX ガードが効く（終了コード 5。ガードは役割の判定より前に走る）
run unicode_guard_qa qa comment 1 --body-file 'x\u3042'
[[ "$RC" == 5 ]] || failures+=("unicode_guard_qa: exit $RC, want 5")
expect_no_calls unicode_guard_qa

if ((${#failures[@]})); then
  printf 'FAIL %s\n' "${failures[@]}"
  exit 1
fi
echo ok
