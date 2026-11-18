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
  # App ID ごとに別のトークンを返す（333 = 実装担当 pelway-dev、111 = レビュー担当 pelway-review）
  case "$*" in
    *"--app-id 333 "*) printf '{"token":"dev-token","expires_at":"x"}\n' ;;
    *"--app-id 111 "*) printf '{"token":"review-token","expires_at":"x"}\n' ;;
    *) exit 1 ;;
  esac
  exit 0
fi
{ printf 'GH_TOKEN=%s\n' "${GH_TOKEN:-}"; printf 'ARGS=%s\n' "$*"; } >"$CALL_LOG"
env | sort >"$CALL_LOG.env"
# review 名義は GH_CONFIG_DIR/hosts.yml でトークンを受け取る（環境には載せない）
[ -z "${GH_CONFIG_DIR:-}" ] || cat "$GH_CONFIG_DIR/hosts.yml" >"$CALL_LOG.hosts" 2>/dev/null
EOF

# 偽の git: GH_TOKEN と引数を追記し、環境変数を <log>.env に記録する
# （push 後の agent-gh-resign からの呼び出しも残すため追記）。
# `agent-gh review git` は CALL_LOG を持たない最小環境で実行されるので、
# CALL_LOG 未設定時は $ROOT/git-last.log に記録する。
cat >"$BIN/git" <<EOF
#!/usr/bin/env bash
log="\${CALL_LOG:-$ROOT/git-last.log}"
{ printf 'GH_TOKEN=%s\n' "\${GH_TOKEN:-}"; printf 'ARGS=%s\n' "\$*"; } >>"\$log"
env | sort >"\$log.env"
EOF
chmod +x "$BIN"/*

export PATH="$BIN:$PATH"
export PELWAY_AGENT_FNOX_CONFIG="$ROOT/agent-fnox.toml"
export FAKE_PELWAY_DEV_APP_ID=333
export FAKE_PELWAY_DEV_APP_INSTALLATION_ID=444
export FAKE_PELWAY_DEV_APP_KEY=dev-pem-body
export FAKE_PELWAY_DEV_GIT_EMAIL="9+pelway-dev[bot]@users.noreply.github.com"
export FAKE_PELWAY_REVIEW_APP_ID=111
export FAKE_PELWAY_REVIEW_APP_INSTALLATION_ID=222
export FAKE_PELWAY_REVIEW_APP_KEY=pem-body

# run <name> <args...>: CALL_LOG を新しくして agent-gh を実行し、出力を OUT、終了コードを RC に入れる。
run() {
  local name="$1"
  shift
  export CALL_LOG="$ROOT/$name.log"
  rm -f "$CALL_LOG" "$CALL_LOG.fnox-args" "$CALL_LOG.env" "$ROOT/git-last.log" "$ROOT/git-last.log.env"
  OUT="$(bash "$SCRIPT" "$@" 2>&1)"
  RC=$?
}

expect_log() {
  local name="$1" pattern="$2"
  grep -qF -- "$pattern" "$ROOT/$name.log" 2>/dev/null || failures+=("$name: log lacks '$pattern'")
}

# expect_review_auth <name>: review 名義の gh が、トークンを GH_TOKEN ではなく 0600 の
# 一時 GH_CONFIG_DIR/hosts.yml で受け取り、終了後に消したことを確かめる。
expect_review_auth() {
  local name="$1" log="$ROOT/$1.log" cfgdir
  [[ "$(head -n1 "$log" 2>/dev/null)" == "GH_TOKEN=" ]] || failures+=("$name: GH_TOKEN in gh env")
  grep -qF 'oauth_token: review-token' "$log.hosts" 2>/dev/null || failures+=("$name: hosts.yml lacks review token")
  cfgdir="$(sed -n 's/^GH_CONFIG_DIR=//p' "$log.env" 2>/dev/null | head -1)"
  [[ -n "$cfgdir" && ! -e "$cfgdir" ]] || failures+=("$name: GH_CONFIG_DIR left behind")
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
expect_review_auth review_mints_token
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

# expect_no_calls <name>: fnox・gh・git のどれも呼ばれていない（呼ぶと CALL_LOG 系か git-last.log ができる）
expect_no_calls() {
  local name="$1"
  local log="$ROOT/$name.log"
  [[ ! -e "$log" && ! -e "$log.fnox-args" && ! -e "$log.token-args" && ! -e "$ROOT/git-last.log" ]] ||
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

# Issue #295: review 名義で実行できるのは gh の読み取り系（＋レビュー投稿の pr review）
# だけで、git は実行できない。許可される形はトークンを発行し、GH_CONFIG_DIR の
# hosts.yml で gh に渡す。許可されない形は終了コード 6 で、fnox（トークン発行）・
# gh・git のどれも呼ばれない。
run review_gh_api review api /rate_limit
expect_review_auth review_gh_api
expect_log review_gh_api "ARGS=api /rate_limit"

# api は GET のみ許可（PR #463 round 1 の must-fix）。明示の GET・読み取り系フラグは通す
run review_gh_api_get review api -X GET /rate_limit
expect_log review_gh_api_get "ARGS=api -X GET /rate_limit"

run review_gh_api_method_get review api --method get /rate_limit
expect_log review_gh_api_method_get "ARGS=api --method get /rate_limit"

# 束ね書きのクラスタ内で -X が明示する GET も許す
run review_gh_api_cluster_get review api -iX GET /rate_limit
expect_log review_gh_api_cluster_get "ARGS=api -iX GET /rate_limit"

run review_gh_api_xget review api -XGET /rate_limit
expect_log review_gh_api_xget "ARGS=api -XGET /rate_limit"

run review_gh_api_readflags review api --paginate --slurp -H 'Accept: x' -q '.a' /rate_limit
expect_log review_gh_api_readflags "ARGS=api --paginate --slurp -H Accept: x -q .a /rate_limit"

run review_gh_api_bool_cluster review api -i /rate_limit
expect_log review_gh_api_bool_cluster "ARGS=api -i /rate_limit"

# --jq の値が環境変数を読んでも漏れない（GH_TOKEN は子プロセスの環境に載せない。
# round 4 の must-fix: `--jq '$ENV.GH_TOKEN'` で review トークンが出力に出た）
# shellcheck disable=SC2016
run review_gh_api_jq_env review api /rate_limit --jq '$ENV.GH_TOKEN'
expect_review_auth review_gh_api_jq_env
# shellcheck disable=SC2016
expect_log review_gh_api_jq_env 'ARGS=api /rate_limit --jq $ENV.GH_TOKEN'

run review_gh_issue_view review issue view 1 --json body
expect_review_auth review_gh_issue_view
expect_log review_gh_issue_view "ARGS=issue view 1 --json body"

run review_gh_pr_view review pr view 1 --json title
expect_review_auth review_gh_pr_view
expect_log review_gh_pr_view "ARGS=pr view 1 --json title"

# pr view の --jq でも環境にトークンが無い
# shellcheck disable=SC2016
run review_gh_pr_view_jq_env review pr view 1 --json title --jq '$ENV'
expect_review_auth review_gh_pr_view_jq_env
# shellcheck disable=SC2016
expect_log review_gh_pr_view_jq_env 'ARGS=pr view 1 --json title --jq $ENV'

run review_gh_pr_diff review pr diff 1
expect_review_auth review_gh_pr_diff
expect_log review_gh_pr_diff "ARGS=pr diff 1"

run review_gh_pr_checks review pr checks 1 --required
expect_review_auth review_gh_pr_checks
expect_log review_gh_pr_checks "ARGS=pr checks 1 --required"

run review_gh_pr_review review pr review 1 --approve --body-file -
expect_review_auth review_gh_pr_review
expect_log review_gh_pr_review "ARGS=pr review 1 --approve --body-file -"

# review_deny <name> <args...>: `agent-gh review <args...>` が終了コード 6 で拒否され、
# トークン発行（fnox）も gh/git も呼ばれないことを確かめる。
review_deny() {
  local name="$1"
  shift
  run "$name" review "$@"
  [[ "$RC" == 6 ]] || failures+=("$name: exit $RC, want 6")
  [[ "$OUT" == *'review 名義'* ]] || failures+=("$name: no guidance on stderr")
  expect_no_calls "$name"
}

# git は review 名義ではサブコマンドを問わずすべて拒否（round 4。レビュースレッドは
# main の git を直接読む。読み取り形の引数でも --output・-O・--no-index のような
# 書き込み・実行・リポジトリ外の読み取りに届く形を引数検査で網羅できないため、
# サブコマンドごと止める）
review_deny review_git_show git show HEAD
review_deny review_git_log git log --oneline
review_deny review_git_diff git diff HEAD~1
review_deny review_git_grep git grep -e foo
review_deny review_git_blame git blame file
review_deny review_git_grep_noindex git grep --no-index . /path/to/secret
review_deny review_git_grep_open_abbrev git grep --op=evil foo
# deny 済みの git 操作や任意実行がラッパー経由で通らない（#256 の実測指摘の形）
review_deny review_git_switch git switch -c probe
review_deny review_git_c_alias git -c 'alias.x=!id' x
review_deny review_git_push git push -u origin HEAD
review_deny review_git_fetch git fetch origin
review_deny review_git_lsremote git ls-remote https://example.com/
review_deny review_git_no_sub git
# 列挙外の gh サブコマンドはすべて拒否
review_deny review_pr_checkout pr checkout 1
review_deny review_pr_ready pr ready 1
review_deny review_pr_list pr list
review_deny review_issue_list issue list
review_deny review_repo_clone repo clone hatai/Pelway dst
review_deny review_alias alias list
review_deny review_auth auth status
review_deny review_gist gist list
review_deny review_release release list
review_deny review_no_args
# 読み取り形に見える引数（書き込み・実行の経路を持つ形）も、git 全拒否で止まる
review_deny review_git_show_output git show --output=/tmp/x HEAD
review_deny review_git_log_output git log --output /tmp/x
review_deny review_git_grep_pager git grep -Oevil foo
review_deny review_git_grep_open git grep --open-files-in-pager=evil foo
review_deny review_git_log_P git log -P
review_deny review_git_diff_extdiff git diff --ext-diff
review_deny review_git_show_textconv git show --textconv

# api の変更系メソッド・POST を意味する引数形は拒否（GH_TOKEN を持つ review トークンでの書き込みを止める）
review_deny review_api_patch api -X PATCH /repos/hatai/Pelway/pulls/1
review_deny review_api_delete api -X DELETE /repos/hatai/Pelway/pulls/1
review_deny review_api_post api -X POST /repos/hatai/Pelway/issues
review_deny review_api_method_put api --method PUT /x
review_deny review_api_method_eq api --method=POST /x
review_deny review_api_short_inline api -XDELETE /x
review_deny review_api_method_missing api -X
review_deny review_api_raw_field api -f query='{ viewer }' graphql
review_deny review_api_field api -F 'key=@/etc/passwd' /x
review_deny review_api_raw_field_long api --raw-field 'x=1' /x
review_deny review_api_field_long api --field 'x=1' /x
review_deny review_api_input api --input /tmp/body.json /x
# 値を取らない短いオプションを先頭に束ねたクラスタ内の X/f/F も拒否する
# （pflag はクラスタを 1 文字ずつ読み、-i は値を取らないので続く文字が次のフラグになる）
review_deny review_api_cluster_x api -iX PATCH /x
review_deny review_api_cluster_xval api -iXDELETE /x
review_deny review_api_cluster_xeq api -iX=POST /x
review_deny review_api_cluster_f api -iftitle=x /x
review_deny review_api_cluster_F api -iF 'key=@/etc/passwd' /x
# 許可リストに無い長いオプション・短いオプションは拒否
review_deny review_api_unknown_long api --nonexistent /x
review_deny review_api_unknown_short api -z /x
# メソッド上書きを試みるヘッダは拒否
review_deny review_api_method_override api -H 'X-HTTP-Method-Override: PATCH' /x
review_deny review_api_method_override_long api --header 'x-http-method-override: DELETE' /x

# dev 名義はこれまでどおり制限されない
run dev_git_unrestricted dev git switch -c probe
expect_log dev_git_unrestricted "GH_TOKEN=dev-token"
expect_log dev_git_unrestricted "switch -c probe"

run dev_gh_unrestricted dev repo list
expect_log dev_gh_unrestricted "ARGS=repo list"

if ((${#failures[@]})); then
  printf 'FAIL %s\n' "${failures[@]}"
  exit 1
fi
echo ok
