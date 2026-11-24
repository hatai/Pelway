#!/usr/bin/env bash
# scripts/agent-gh のテスト。偽の fnox・gh・git を PATH の先頭に置き、渡されたトークンと引数を確かめる。
set -uo pipefail

SCRIPT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/agent-gh"
ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT
failures=()

# 偽の mktemp / chmod は本物に委譲する。この時点（PATH 変更前）に本物のパスを取っておく
REAL_MKTEMP="$(command -v mktemp)"
REAL_CHMOD="$(command -v chmod)"
# push の名義の確認（#613）の試験は本物の git とローカルの bare remote で行う
REAL_GIT="$(command -v git)"

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
# どちらの経路でも GH_CONFIG_DIR / GH_NO_UPDATE_NOTIFIER も記録する（#522 の隔離の確認用）。
# GH_TELEMETRY / DO_NOT_TRACK も記録する（#527 のテレメトリ停止の確認用）。
# GH_TOKEN_GENERATE_FAIL があれば generate は stderr 2 行を出して失敗する（#520 の失敗経路）。
# GH_EXTENSION_LIST / GH_EXTENSION_LIST_FAIL で `gh extension list` の応答を切り替える。
cat >"$BIN/gh" <<'EOF'
#!/usr/bin/env bash
if [[ "$1 $2" == "token generate" ]]; then
  printf '%s\n' "$*" >"$CALL_LOG.token-args"
  { printf 'GH_CONFIG_DIR=%s\n' "${GH_CONFIG_DIR:-}"; printf 'GH_TELEMETRY=%s\n' "${GH_TELEMETRY:-}"; printf 'DO_NOT_TRACK=%s\n' "${DO_NOT_TRACK:-}"; } >"$CALL_LOG.token-env"
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
if [[ "$1 $2" == "token --help" ]]; then
  # 拡張の有無の判定に使う（#520）。GH_TOKEN_HELP_MODE で応答を切り替える:
  # ok（既定・拡張あり exit 0）/ absent（拡張なし・unknown command の exit 1）/ missing（gh 無し・exit 127）
  case "${GH_TOKEN_HELP_MODE:-ok}" in
    absent) printf 'unknown command "token" for "gh"\n' >&2; exit 1 ;;
    missing) exit 127 ;;
    ok) printf 'gh-token 2.0.10 help\n'; exit 0 ;;
  esac
fi
# FAKE_GH_HINT があれば、gh 2.105 の `gh event listen` の案内（実物の形）を警告・エラーの行と混ぜて
# stderr に出し、stdout に本来の出力を出して FAKE_GH_EXIT で終わる（#596 の案内の除去の確認用）。
# FAKE_GH_LOOKALIKE があれば、案内と同じ書式の先頭行で、続きが `gh event listen` でない別の診断を出す。
if [[ -n "${FAKE_GH_LOOKALIKE:-}" ]]; then
  printf '%s\n' 'hint: to see other fields, use this instead of polling:' '  - gh pr view --json' >&2
  printf 'hint: trailing lookalike instead of polling:\n' >&2
  exit 0
fi
if [[ -n "${FAKE_GH_HINT:-}" ]]; then
  printf 'STDOUT-BODY\n'
  printf 'warning: before the hint\n' >&2
  printf '%s\n' \
    'hint: if you need to wait for this pull request to be reviewed, merged, or closed before continuing, use this instead of polling:' \
    '    gh event listen --resource https://github.com/hatai/Pelway/pull/1 --event com.github.pull_request.review.submitted.v1' \
    '  - It prints the first event as a JSON line on stdout and exits; run it again to wait for the next one.' \
    '  - Add `--timeout` to stop waiting after a while, for example `--timeout 30m`; it then exits with status 8.' \
    '  - Events carry no details; read the pull request to see what changed.' \
    '  - It misses changes made before it starts, so read the pull request once it is running.' \
    '  - See `gh event listen --help` for other event types.' \
    '  - Event subscriptions are in preview and may not be enabled for your account.' >&2
  printf 'error: after the hint\n' >&2
  printf '  - indented error detail\n' >&2
  exit "${FAKE_GH_EXIT:-0}"
fi
{ printf 'GH_TOKEN=%s\n' "${GH_TOKEN:-}"; printf 'GH_CONFIG_DIR=%s\n' "${GH_CONFIG_DIR:-}"; printf 'GH_NO_UPDATE_NOTIFIER=%s\n' "${GH_NO_UPDATE_NOTIFIER:-}"; printf 'GH_TELEMETRY=%s\n' "${GH_TELEMETRY:-}"; printf 'DO_NOT_TRACK=%s\n' "${DO_NOT_TRACK:-}"; printf 'ARGS=%s\n' "$*"; } >"$CALL_LOG"
EOF

# 偽の git: GH_TOKEN と引数を追記する（push 後の agent-gh-resign からの呼び出しも残すため追記）。
# GH_CONFIG_DIR / GH_TELEMETRY / DO_NOT_TRACK も記録する（credential helper として呼ばれる gh への伝搬確認用）。
cat >"$BIN/git" <<'EOF'
#!/usr/bin/env bash
{ printf 'GH_TOKEN=%s\n' "${GH_TOKEN:-}"; printf 'GH_CONFIG_DIR=%s\n' "${GH_CONFIG_DIR:-}"; printf 'GH_TELEMETRY=%s\n' "${GH_TELEMETRY:-}"; printf 'DO_NOT_TRACK=%s\n' "${DO_NOT_TRACK:-}"; printf 'ARGS=%s\n' "$*"; } >>"$CALL_LOG"
EOF

# 偽の mktemp: 本物に委譤し、作成したパスを種別（f=ファイル、d=ディレクトリ）とともに記録する
# （一時ファイルの残存確認用。引数をそのまま記録すると、引数なしの呼び出しが read の
#  IFS ホワイトスペースの詰めで読み取れないため種別を付ける）。
cat >"$BIN/mktemp" <<EOF
#!/usr/bin/env bash
p="\$($REAL_MKTEMP "\$@")"
if ((\$# == 0)); then kind=f; else kind="\${1#-}"; fi
printf '%s\t%s\n' "\$kind" "\$p" >>"\${MKTEMP_LOG:-/dev/null}"
printf '%s\n' "\$p"
EOF

# 偽の chmod: FAKE_CHMOD_FAIL=1 のときだけ失敗する（app_token の subshell 途中終了の再現用）。
cat >"$BIN/chmod" <<EOF
#!/usr/bin/env bash
[[ -z "\${FAKE_CHMOD_FAIL:-}" ]] || exit 1
exec '$REAL_CHMOD' "\$@"
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

# 鍵は複数行の PEM。stderr の 1 行目に鍵本体の 1 行（base64 行）が混ざっても伏せる
# （鍵全体が 1 行に現れることは無いため、行ごとの伏せが必要）
FAKE_PELWAY_DEV_APP_KEY=$'-----BEGIN PRIVATE KEY-----\nMIIBfakeKEYbodyLINEone1234\nMIIBfakeKEYbodyLINEtwo5678\n-----END PRIVATE KEY-----' \
GH_TOKEN_GENERATE_FAIL=1 \
GH_TOKEN_GENERATE_STDERR_1='jwt sign error at MIIBfakeKEYbodyLINEone1234 (app 333)' \
  run token_fail_masks_multiline_key_fragment dev pr list
[[ "$RC" == 3 ]] || failures+=("token_fail_masks_multiline_key_fragment: exit $RC, want 3")
[[ "$OUT" != *MIIBfakeKEYbodyLINEone1234* ]] || failures+=("token_fail_masks_multiline_key_fragment: key body line leaked")
[[ "$OUT" != *MIIBfakeKEYbodyLINEtwo5678* ]] || failures+=("token_fail_masks_multiline_key_fragment: other key body line leaked")
[[ "$OUT" != *333* ]] || failures+=("token_fail_masks_multiline_key_fragment: app id value leaked")

# stderr が空のときは 1 行目を出せないので、終了コードで原因の種別だけ示す
GH_TOKEN_GENERATE_FAIL=1 \
GH_TOKEN_GENERATE_STDERR_1='' \
  run token_fail_empty_stderr dev pr list
[[ "$RC" == 3 ]] || failures+=("token_fail_empty_stderr: exit $RC, want 3")
grep -qF '終了コード' <<<"$OUT" || failures+=("token_fail_empty_stderr: exit code not shown")

# 「拡張が必要」の文は gh token --help（隔離された空の GH_CONFIG_DIR の中）が成功する
# （= gh-token 拡張が gh token コマンドを提供している）ときは出さない
GH_TOKEN_GENERATE_FAIL=1 GH_TOKEN_HELP_MODE=ok \
  run token_fail_no_install_hint_when_present dev pr list
[[ "$RC" == 3 ]] || failures+=("token_fail_no_install_hint_when_present: exit $RC, want 3")
[[ "$OUT" != *'gh extension install'* ]] || failures+=("token_fail_no_install_hint_when_present: install hint shown despite extension present")

# gh token --help が unknown command（exit 1）= 拡張が無いときだけ「拡張が必要」を出す
GH_TOKEN_GENERATE_FAIL=1 GH_TOKEN_HELP_MODE=absent \
  run token_fail_install_hint_when_absent dev pr list
[[ "$RC" == 3 ]] || failures+=("token_fail_install_hint_when_absent: exit $RC, want 3")
grep -qF 'gh extension install Link-/gh-token' <<<"$OUT" || failures+=("token_fail_install_hint_when_absent: install hint missing")

# gh が起動できない（exit 127）は「拡張が無い」と区別する（install 案内を出さない）
GH_TOKEN_GENERATE_FAIL=1 GH_TOKEN_HELP_MODE=missing \
  run token_fail_gh_missing_hint dev pr list
[[ "$RC" == 3 ]] || failures+=("token_fail_gh_missing_hint: exit $RC, want 3")
[[ "$OUT" == *'gh が起動できません'* ]] || failures+=("token_fail_gh_missing_hint: gh-missing hint missing")
[[ "$OUT" != *'gh extension install'* ]] || failures+=("token_fail_gh_missing_hint: install hint shown despite gh itself missing")

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

# --- #522: gh は毎回空の一時 GH_CONFIG_DIR で呼ばれ、~/.config/gh や外部の hosts.yml を読ませない ---
# 呼び出し元の GH_CONFIG_DIR に偽の hosts.yml（人間のトークンの代わりのマーカー）を置いても、
# gh に渡るのは毎回新しく作った空の一時ディレクトリで、agent-gh の終了後には残っていない。
fake_gh_dir="$ROOT/fake-gh-config"
mkdir -p "$fake_gh_dir"
printf 'github.com:\n    oauth_token: hosts-yml-token\n' >"$fake_gh_dir/hosts.yml"

# used_gh_config <name>: 偽 gh / 偽 git が記録した GH_CONFIG_DIR の値
used_gh_config() { sed -n 's/^GH_CONFIG_DIR=//p' "$ROOT/$1.log"; }

# dev ロール: 呼び出し元の GH_CONFIG_DIR（偽 hosts.yml 置き場）は上書きされる
GH_CONFIG_DIR="$fake_gh_dir" run gh_config_isolated dev pr list
[[ "$RC" == 0 ]] || failures+=("gh_config_isolated: exit $RC, want 0 ($OUT)")
used="$(used_gh_config gh_config_isolated)"
[[ -n "$used" && "$used" != "$fake_gh_dir" ]] || failures+=("gh_config_isolated: caller GH_CONFIG_DIR not overridden")
[[ ! -e "$used/hosts.yml" ]] || failures+=("gh_config_isolated: gh saw a hosts.yml in its config dir")
[[ ! -e "$used" ]] || failures+=("gh_config_isolated: temp GH_CONFIG_DIR left behind")
grep -qF 'GH_NO_UPDATE_NOTIFIER=1' "$ROOT/gh_config_isolated.log" || failures+=("gh_config_isolated: GH_NO_UPDATE_NOTIFIER not set")
# トークン発行（gh token generate）も隔離された空ディレクトリで呼ばれ、終了後に消える
tok="$(sed -n 's/^GH_CONFIG_DIR=//p' "$ROOT/gh_config_isolated.log.token-env")"
[[ -n "$tok" && "$tok" != "$fake_gh_dir" ]] || failures+=("gh_config_isolated: token generate saw caller GH_CONFIG_DIR")
[[ ! -e "$tok" ]] || failures+=("gh_config_isolated: token generate config dir left behind")
# 読まれなかった偽 hosts.yml はそのまま残る
[[ -e "$fake_gh_dir/hosts.yml" ]] || failures+=("gh_config_isolated: fake hosts.yml disturbed")

# review ロール（~/.config/gh が読めないレビュー sandbox が発端）でも同じ隔離
GH_CONFIG_DIR="$fake_gh_dir" run gh_config_isolated_review review pr list
[[ "$RC" == 0 ]] || failures+=("gh_config_isolated_review: exit $RC, want 0 ($OUT)")
used="$(used_gh_config gh_config_isolated_review)"
[[ -n "$used" && "$used" != "$fake_gh_dir" && ! -e "$used" ]] || failures+=("gh_config_isolated_review: GH_CONFIG_DIR not isolated/removed")

# qa ロールでも同じ隔離
GH_CONFIG_DIR="$fake_gh_dir" run gh_config_isolated_qa qa comment 1 --body-file "$ROOT/qa-body.md"
[[ "$RC" == 0 ]] || failures+=("gh_config_isolated_qa: exit $RC, want 0 ($OUT)")
used="$(used_gh_config gh_config_isolated_qa)"
[[ -n "$used" && "$used" != "$fake_gh_dir" && ! -e "$used" ]] || failures+=("gh_config_isolated_qa: GH_CONFIG_DIR not isolated/removed")

# git サブコマンド（credential helper として gh が呼ばれる経路）にも同じ環境が渡る
GH_CONFIG_DIR="$fake_gh_dir" run gh_config_isolated_git dev git status
[[ "$RC" == 0 ]] || failures+=("gh_config_isolated_git: exit $RC, want 0 ($OUT)")
used="$(used_gh_config gh_config_isolated_git)"
[[ -n "$used" && "$used" != "$fake_gh_dir" && ! -e "$used" ]] || failures+=("gh_config_isolated_git: GH_CONFIG_DIR not isolated/removed")

# --- #527: gh のテレメトリ（cafe.github.com への送信）を止める ---
# どのロール（dev / review / qa、git サブコマンド経由も）で呼ばれた gh にも GH_TELEMETRY=false と
# DO_NOT_TRACK=1 が入る。呼び出し元が別の値を渡しても上書きされる。トークン発行の gh にも入る。
# expect_no_telemetry <name> <file>: 記録ファイルに 2 変数の値が入っていることを確かめる
expect_no_telemetry() {
  grep -qx 'GH_TELEMETRY=false' "$2" || failures+=("$1: GH_TELEMETRY=false not set ($(basename "$2"))")
  grep -qx 'DO_NOT_TRACK=1' "$2" || failures+=("$1: DO_NOT_TRACK=1 not set ($(basename "$2"))")
}
for t in "telemetry_dev:dev pr list" "telemetry_review:review pr list" "telemetry_git:dev git status" \
  "telemetry_qa:qa comment 1 --body-file $ROOT/qa-body.md"; do
  name="${t%%:*}"
  read -r -a targs <<<"${t#*:}"
  GH_TELEMETRY=enabled DO_NOT_TRACK=0 run "$name" "${targs[@]}"
  [[ "$RC" == 0 ]] || failures+=("$name: exit $RC, want 0 ($OUT)")
  expect_no_telemetry "$name" "$ROOT/$name.log"
  expect_no_telemetry "$name" "$ROOT/$name.log.token-env"
done

# トークンが取れない（fnox が秘密を返さない）ときは hosts.yml へフォールバックせず fail-closed。
# gh は一切呼ばれない
GH_CONFIG_DIR="$fake_gh_dir" FAKE_PELWAY_DEV_APP_KEY="" run no_fallback_missing_secret dev pr list
[[ "$RC" == 3 ]] || failures+=("no_fallback_missing_secret: exit $RC, want 3")
[[ ! -e "$ROOT/no_fallback_missing_secret.log" ]] || failures+=("no_fallback_missing_secret: gh was called")

# トークン発行そのものが失敗しても、本体の gh は呼ばず exit 3（hosts.yml のトークンで続けない）。
# 発行の試行も隔離された空ディレクトリでだけ行われる
GH_CONFIG_DIR="$fake_gh_dir" FAKE_PELWAY_DEV_APP_ID=999 run no_fallback_generate_fails dev pr list
[[ "$RC" == 3 ]] || failures+=("no_fallback_generate_fails: exit $RC, want 3")
[[ ! -e "$ROOT/no_fallback_generate_fails.log" ]] || failures+=("no_fallback_generate_fails: gh was called")
tok="$(sed -n 's/^GH_CONFIG_DIR=//p' "$ROOT/no_fallback_generate_fails.log.token-env")"
[[ -n "$tok" && "$tok" != "$fake_gh_dir" ]] || failures+=("no_fallback_generate_fails: token generate saw caller GH_CONFIG_DIR")

# --- app_token の subshell 途中終了でも秘密鍵の一時ファイルが残らない ---
# app_token は token="$(app_token ...)" のコマンド置換 subshell の中で動く。subshell では親の
# EXIT trap は走らないため、subshell 内で trap を設定して mktemp から消し忘れを防いでいる。

# トークン発行の失敗経路: --key の一時ファイルは発行に失敗しても残らない（明示的な rm の経路）
GH_CONFIG_DIR="$fake_gh_dir" FAKE_PELWAY_DEV_APP_ID=999 run key_removed_on_generate_fail dev pr list
[[ "$RC" == 3 ]] || failures+=("key_removed_on_generate_fail: exit $RC, want 3")
kp="$(cat "$ROOT/key_removed_on_generate_fail.log.key-path" 2>/dev/null)"
[[ -n "$kp" ]] || failures+=("key_removed_on_generate_fail: key path not recorded")
[[ ! -e "$kp" ]] || failures+=("key_removed_on_generate_fail: key file left behind")

# subshell の途中終了（chmod 600 の失敗で set -e が即座に subshell を止める）でも、
# 鍵ファイル（引数なし mktemp）と隔離ディレクトリ（mktemp -d）はどちらも残らない
export MKTEMP_LOG="$ROOT/mktemp-chmod-fail.log"
: >"$MKTEMP_LOG"
GH_CONFIG_DIR="$fake_gh_dir" FAKE_CHMOD_FAIL=1 run keyfile_chmod_fail dev pr list
[[ "$RC" != 0 ]] || failures+=("keyfile_chmod_fail: unexpectedly succeeded")
[[ -s "$MKTEMP_LOG" ]] || failures+=("keyfile_chmod_fail: mktemp calls not recorded")
while IFS=$'\t' read -r kind mp; do
  if [[ "$kind" == "f" ]]; then
    [[ ! -e "$mp" ]] || failures+=("keyfile_chmod_fail: key file left behind ($mp)")
  elif [[ "$kind" == "d" ]]; then
    [[ ! -e "$mp" ]] || failures+=("keyfile_chmod_fail: isolated GH_CONFIG_DIR left behind ($mp)")
  fi
done <"$MKTEMP_LOG"

# --- push するコミットの名義の確認（#613）---
# 本物の git とローカルの bare remote で、pre-push フックによる名義の確認を確かめる。
# 利用者の git 設定（署名・フック）に左右されないよう、この節では global / system の設定を読ませない。
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1
RGBIN="$ROOT/realgit-bin"
mkdir -p "$RGBIN"
ln -s "$REAL_GIT" "$RGBIN/git"
DEV_EMAIL="$FAKE_PELWAY_DEV_GIT_EMAIL"
# as <name> <git の引数...>: author・committer を <name> にして本物の git を実行する
as() {
  local who="$1"
  shift
  GIT_AUTHOR_NAME="$who" GIT_AUTHOR_EMAIL="$who@example.com" GIT_COMMITTER_NAME="$who" GIT_COMMITTER_EMAIL="$who@example.com" \
    "$REAL_GIT" -c commit.gpgsign=false "$@"
}
# setup_push_repo <name>: remote.git（bare）と、それを clone した work を作る。
# main には hatai のコミットと、kodiakhq[bot]（committer は GitHub）のマージコミットを置く。
# work では agent/<name> ブランチを main から切る。PUSH_DIR に置き場所を入れる。
setup_push_repo() {
  PUSH_DIR="$ROOT/push-$1"
  mkdir -p "$PUSH_DIR"
  "$REAL_GIT" init -q --bare -b main "$PUSH_DIR/remote.git"
  "$REAL_GIT" init -q -b main "$PUSH_DIR/seed"
  (
    cd "$PUSH_DIR/seed" || exit 1
    as hatai commit -q --allow-empty -m init
    as hatai switch -q -c feature
    as hatai commit -q --allow-empty -m feature
    as hatai switch -q main
    GIT_AUTHOR_NAME='kodiakhq[bot]' GIT_AUTHOR_EMAIL=k@example.com GIT_COMMITTER_NAME=GitHub GIT_COMMITTER_EMAIL=noreply@github.com \
      "$REAL_GIT" -c commit.gpgsign=false merge -q --no-ff -m 'Merge feature' feature
    "$REAL_GIT" push -q "$PUSH_DIR/remote.git" main
  )
  "$REAL_GIT" clone -q "$PUSH_DIR/remote.git" "$PUSH_DIR/work"
  "$REAL_GIT" -C "$PUSH_DIR/work" switch -q -c "agent/$1"
}
# push_run <name> <agent-gh の引数...>: work の中で agent-gh を本物の git で実行する
push_run() {
  local name="$1"
  shift
  export CALL_LOG="$ROOT/$name.log"
  OUT="$(cd "$PUSH_DIR/work" && PATH="$RGBIN:$PATH" bash "$SCRIPT" "$@" 2>&1)"
  RC=$?
}
remote_ref() { "$REAL_GIT" --git-dir="$PUSH_DIR/remote.git" rev-parse -q --verify "refs/heads/$1"; }
work_git() { "$REAL_GIT" -C "$PUSH_DIR/work" "$@"; }

# 1. 範囲は「push する ref から辿れて origin の追跡 ref から辿れないコミット」。main から取り込んだ
#    hatai のコミットと、remote に既にある kodiakhq[bot] のマージコミットがあっても push は通る。
setup_push_repo author_range
(cd "$PUSH_DIR/work" && as 'pelway-dev[bot]' commit -q --allow-empty -m a1)
(cd "$PUSH_DIR/seed" && as hatai commit -q --allow-empty -m main2 && "$REAL_GIT" push -q "$PUSH_DIR/remote.git" main)
work_git fetch -q origin
(cd "$PUSH_DIR/work" && as 'pelway-dev[bot]' merge -q --no-ff -m 'Merge main' origin/main)
PELWAY_AGENT_SKIP_RESIGN=1 push_run author_range dev git push -u origin HEAD
[[ "$RC" == 0 ]] || failures+=("author_range: exit $RC, want 0: $OUT")
[[ "$(remote_ref agent/author_range)" == "$(work_git rev-parse HEAD)" ]] || failures+=("author_range: remote ref not updated")

# 2・3. hatai 名義のコミット（author と committer の両方、または committer だけ）を含む push は止まる。
#       remote の ref は変わらず、標準エラーに短い sha・名義・作り直しの案内が出る。
setup_push_repo author_block
(cd "$PUSH_DIR/work" && as 'pelway-dev[bot]' commit -q --allow-empty -m ok)
PELWAY_AGENT_SKIP_RESIGN=1 push_run author_block_setup dev git push -u origin HEAD
before="$(remote_ref agent/author_block)"
[[ -n "$before" ]] || failures+=("author_block: setup push failed: $OUT")
(cd "$PUSH_DIR/work" && as hatai commit -q --allow-empty -m bad)
bad_sha="$(work_git rev-parse --short HEAD)"
push_run author_block dev git push
[[ "$RC" != 0 ]] || failures+=("author_block: exit 0, want non-zero")
[[ "$(remote_ref agent/author_block)" == "$before" ]] || failures+=("author_block: remote ref changed")
[[ "$OUT" == *"$bad_sha"* ]] || failures+=("author_block: stderr lacks short sha $bad_sha: $OUT")
[[ "$OUT" == *"author=hatai"* && "$OUT" == *"committer=hatai"* ]] || failures+=("author_block: stderr lacks identity: $OUT")
[[ "$OUT" == *"scripts/agent-gh dev git commit"* ]] || failures+=("author_block: stderr lacks rebuild guidance: $OUT")

work_git reset -q --hard "$before"
(cd "$PUSH_DIR/work" && GIT_AUTHOR_NAME='pelway-dev[bot]' GIT_AUTHOR_EMAIL="$DEV_EMAIL" GIT_COMMITTER_NAME=hatai GIT_COMMITTER_EMAIL=h@example.com \
  "$REAL_GIT" -c commit.gpgsign=false commit -q --allow-empty -m committer-only)
bad_sha="$(work_git rev-parse --short HEAD)"
push_run committer_block dev git push
[[ "$RC" != 0 ]] || failures+=("committer_block: exit 0, want non-zero")
[[ "$(remote_ref agent/author_block)" == "$before" ]] || failures+=("committer_block: remote ref changed")
[[ "$OUT" == *"$bad_sha"* && "$OUT" == *"committer=hatai"* ]] || failures+=("committer_block: stderr lacks sha/committer: $OUT")

work_git reset -q --hard "$before"
(cd "$PUSH_DIR/work" && GIT_AUTHOR_NAME=hatai GIT_AUTHOR_EMAIL=h@example.com GIT_COMMITTER_NAME='pelway-dev[bot]' GIT_COMMITTER_EMAIL="$DEV_EMAIL" \
  "$REAL_GIT" -c commit.gpgsign=false commit -q --allow-empty -m author-only)
bad_sha="$(work_git rev-parse --short HEAD)"
push_run author_only_block dev git push
[[ "$RC" != 0 ]] || failures+=("author_only_block: exit 0, want non-zero")
[[ "$(remote_ref agent/author_block)" == "$before" ]] || failures+=("author_only_block: remote ref changed")
[[ "$OUT" == *"$bad_sha"* && "$OUT" == *"author=hatai"* ]] || failures+=("author_only_block: stderr lacks sha/author: $OUT")

# 5. 確認を外す環境変数・引数は無い: PELWAY_AGENT_SKIP_RESIGN=1・--no-verify（その省略形）・
#    core.hooksPath の上書きのどれでも push されない。
PELWAY_AGENT_SKIP_RESIGN=1 push_run skip_resign_still_checks dev git push
[[ "$RC" != 0 ]] || failures+=("skip_resign_still_checks: exit 0, want non-zero")
[[ "$(remote_ref agent/author_block)" == "$before" ]] || failures+=("skip_resign_still_checks: remote ref changed")
[[ "$OUT" == *"$bad_sha"* ]] || failures+=("skip_resign_still_checks: stderr lacks short sha: $OUT")
for bypass in "push --no-verify" "push --no-verif origin HEAD" "-c core.hooksPath=/dev/null push" "-c core.hookspath=/dev/null push" "--config-env=core.hooksPath=HOME push"; do
  read -ra bypass_args <<<"$bypass"
  push_run no_bypass dev git "${bypass_args[@]}"
  [[ "$RC" != 0 ]] || failures+=("no_bypass ($bypass): exit 0, want non-zero")
  [[ "$(remote_ref agent/author_block)" == "$before" ]] || failures+=("no_bypass ($bypass): remote ref changed")
done

# -c と設定値を連結した形（-ccore.hooksPath=...）も、git の版に頼らず agent-gh が終了コード 2 で拒否する
# （git 2.56 は連結形を unknown option の 129 で拒むが、受け付ける版ではフックを外す引数になる）。
for attached in "-ccore.hooksPath=/dev/null" "-cCORE.HOOKSPATH=/dev/null"; do
  push_run attached_c dev git "$attached" push origin HEAD
  [[ "$RC" == 2 ]] || failures+=("attached_c ($attached): exit $RC, want 2: $OUT")
  [[ "$OUT" == *"core.hooksPath"* ]] || failures+=("attached_c ($attached): stderr lacks reason: $OUT")
  [[ "$(remote_ref agent/author_block)" == "$before" ]] || failures+=("attached_c ($attached): remote ref changed")
done

# 名前や設定の読み込み先を変えてフックを外す経路も、agent-gh が終了コード 2 で拒否し、remote は変わらない:
# - グローバルオプションの -c include.path / -c includeIf.<条件>.path（読み込んだ設定の core.hooksPath を効かせる）
# - グローバルオプションの -c alias.<x>=push、-c 'alias.<x>=!git push'（サブコマンドの名前を変える）
# - repo の設定に置いた alias（push と、! で始まるシェルの alias）
# - pre-push フックを通らない配管コマンド（send-pack）
printf '[core]\n\thooksPath = /dev/null\n' >"$PUSH_DIR/nohook.cfg"
expect_rejected() {
  local label="$1"
  shift
  push_run rejected dev git "$@"
  [[ "$RC" == 2 ]] || failures+=("rejected ($label): exit $RC, want 2: $OUT")
  [[ "$(remote_ref agent/author_block)" == "$before" ]] || failures+=("rejected ($label): remote ref changed")
}
expect_rejected include.path -c "include.path=$PUSH_DIR/nohook.cfg" push origin HEAD
expect_rejected includeIf -c "includeIf.gitdir:/.path=$PUSH_DIR/nohook.cfg" push origin HEAD
expect_rejected alias-arg -c alias.p=push p origin HEAD
expect_rejected alias-arg-shell -c 'alias.p=!git push' p origin HEAD
work_git config alias.pp push
expect_rejected alias-repo pp origin HEAD
work_git config alias.sp '!git push'
expect_rejected alias-repo-shell sp origin HEAD
work_git config --unset alias.pp
work_git config --unset alias.sp
expect_rejected send-pack send-pack "$PUSH_DIR/remote.git" HEAD:refs/heads/agent/author_block
# 許可リストに無いサブコマンドは exit 2（コマンドを実行するもの、名前の打ち間違いの自動修正を含む）
expect_rejected rebase-exec rebase -x 'git push origin HEAD' HEAD
expect_rejected unknown-name p origin HEAD
work_git config help.autocorrect immediate
expect_rejected autocorrect pushh origin HEAD
expect_rejected glob-question 'pu?h' origin HEAD
# 許可リストの名前を空白でつないだ 1 つの引数は、部分一致では通ってしまう。空白を含む subsection 形の alias
# （環境変数でも repo の設定でも置ける）で push になるため、完全一致で拒否する。
GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0='alias.pull push.command' GIT_CONFIG_VALUE_0=push \
  expect_rejected joined-env-alias 'pull push' origin HEAD
work_git config 'alias.pull push.command' push
expect_rejected joined-repo-alias 'pull push' origin HEAD
work_git config --unset 'alias.pull push.command'
expect_rejected joined-names 'merge merge-base' origin HEAD
expect_rejected glob-star 'p*' origin HEAD
work_git config --unset help.autocorrect
# 許可したサブコマンドでも、コマンドを実行させる転送のオプション（とその省略形）は exit 2 で、コマンドは実行されない
ran="$PUSH_DIR/transport-ran"
expect_rejected receive-pack push "--receive-pack=touch $ran; false" origin HEAD
expect_rejected receive-pack-split push --receive-pack "touch $ran; false" origin HEAD
expect_rejected exec push "--exec=touch $ran; false" origin HEAD
expect_rejected upload-pack fetch "--upload-pack=touch $ran; false" origin
expect_rejected upload-pack-abbrev fetch "--upload=touch $ran; false" origin
expect_rejected pull-upload-pack pull "--upload-pack=touch $ran; false" origin
expect_rejected ls-remote-upload-pack ls-remote "--upload-pack=touch $ran; false" origin
# ls-remote の -u（--upload-pack の短い形。git 2.56 は受け付けないが、版に頼らず agent-gh が拒否する）
expect_rejected ls-remote-u ls-remote -u "touch $ran; false" origin
expect_rejected ls-remote-u-attached ls-remote "-utouch $ran" origin
expect_rejected ls-remote-u-bundled ls-remote -qu "touch $ran; false" origin
# ls-remote は --exec も --upload-pack の別名（builtin/ls-remote.c の隠れたオプション）なので、位置引数の前では拒否する
expect_rejected ls-remote-exec ls-remote "--exec=touch $ran; false" origin
# fetch・pull・push は位置引数の後ろでも `-` の引数をオプションとして解釈する（parse-options の既定）ため、
# リポジトリ名の後ろの転送オプションも拒否する（実測: git fetch <repo> --upload-pack <path> は <path> を実行する）
expect_rejected fetch-after-positional fetch origin "--upload-pack=touch $ran; false"
expect_rejected push-after-positional push origin "--exec=touch $ran; false" HEAD
# 短いオプションの連結形（-of は -o の値が "f"）のあとに続く引数は、オプションの位置のまま
expect_rejected bundle-attached-then-blocked push -of "--exec=touch $ran; false" origin HEAD
# 別の引数を値として取るオプションが `--` を値として消費したときは、その後ろの引数はオプションの位置のまま
# （`git push -o -- --no-verify` は git 側でも --no-verify が効く）
expect_rejected ddash-value-transport push -o -- "--receive-pack=touch $ran; false" origin HEAD
expect_rejected ddash-value-no-verify push -o -- --no-verify origin HEAD
# -o 以外の値を取るオプション（--jobs・-o の省略形 --push-opt など）の値は、検査の単純さのため
# 意図的に区別しない（git は値として読むが、誤って値と見なしても拒否側に倒れるだけなので一律に走査する）。
# それらの値が `--`・-o・ls-remote の位置引数に見えるときも同じ
# （`push --repo -- --no-verify` では `--` が --repo の値で --no-verify は効く）
expect_rejected value-jobs fetch --jobs --exec origin
expect_rejected value-push-option-abbrev push --push-opt --exec origin HEAD
expect_rejected marker-repo-ddash push --repo -- --no-verify origin HEAD
expect_rejected marker-vo-ddash push -vo -- --no-verify origin HEAD
expect_rejected marker-repo-o push --repo -o --no-verify origin HEAD
expect_rejected marker-vo-upload fetch -vo -- "--upload-pack=touch $ran; false" origin
expect_rejected marker-sort ls-remote --sort refname "--upload-pack=touch $ran; false" origin
# 位置引数が先に来ると `--` を値として取るオプションとの区別が付かないため、`--` の後ろも一律に走査する
expect_rejected ddash-refspec-fetch fetch origin -- "--upload-pack=touch $ran; false"
expect_rejected ddash-refspec-push push origin -- --exec
# pull・push は `--` の後ろを `--` なしで子の git（pull は fetch、push はサブモジュールの push）に渡し、
# 子はオプションとして解釈する（実測: `git pull -- . --upload-pack=<cmd>` は <cmd> を実行する）ため、
# pull・push では `--` を終端にせず末尾まで走査する
expect_rejected ddash-pull pull -- . "--upload-pack=touch $ran; false"
expect_rejected ddash-pull-after-o pull -o x -- "--receive-pack=touch $ran; false"
expect_rejected ddash-repo push -- "--receive-pack=touch $ran; false"
expect_rejected ddash-after-o push -o x -- --exec
expect_rejected ddash-submodule push --recurse-submodules=on-demand -- origin --no-verify
[[ ! -e "$ran" ]] || failures+=("transport options: a command ran")

# 値として消費される引数・参照パターン・確かな `--` の後ろの位置引数は、git がオプションとして解釈しないため
# 通す。agent-gh が終了コード 2 を返さないことと、コマンドが実行されないことを確かめる
expect_not_rejected() {
  local label="$1"
  shift
  push_run "$label" dev git "$@"
  # 終了コード 2 が agent-gh の引数の拒否（pre-push フックの名義確認は 1。git 自身は 2 を返さない）
  [[ "$RC" != 2 ]] || failures+=("not_rejected ($label): exit $RC, want != 2: $OUT")
  [[ "$(remote_ref agent/author_block)" == "$before" ]] || failures+=("not_rejected ($label): remote ref changed")
}
# -o（push の --push-option・fetch・pull・ls-remote の --server-option）の値はオプションとして解釈されない
# （-o の値は remote が受け付けず git が失敗するが、agent-gh の拒否ではない）
expect_not_rejected value-o-exec push -o --exec origin HEAD
expect_not_rejected value-o-no-verify push -o --no-verify origin HEAD
expect_not_rejected value-o-attached push -o--exec origin HEAD
expect_not_rejected value-push-option-eq push "--push-option=--exec" origin HEAD
# ls-remote は最初の位置引数（リポジトリ名）の後ろを参照パターンとして読む（STOP_AT_NON_OPTION）
expect_not_rejected pattern-exec ls-remote origin --exec
expect_not_rejected pattern-u ls-remote origin -u
expect_not_rejected pattern-upload-pack ls-remote origin "--upload-pack=x"
expect_not_rejected pattern-server-opt ls-remote -o --exec origin
# `=` 付きのオプションは値を自身で持つため、後ろの位置引数や `--` の扱いに影響しない
expect_not_rejected pattern-sort-eq ls-remote --sort=refname origin --exec
# fetch・ls-remote では clean な `--` の後ろは refspec・参照パターンで、オプションとしては解釈されない
# （`--upload-pack` が refspec・パターン扱いで git が失敗するのは、agent-gh の拒否ではない）
expect_not_rejected ddash-fetch fetch -- "--upload-pack=touch $ran; false" origin
expect_not_rejected ddash-ls-remote ls-remote -o x -- --exec
[[ ! -e "$ran" ]] || failures+=("transport passthrough: a command ran")

# 検査対象外のサブコマンドでは、`--` の後ろに転送オプションと同じ名前のファイルがあっても通す
# （`git add -- --exec` のようなパスの指定を拒否しない）。
(cd "$PUSH_DIR/work" && touch -- --exec --upload-pack --receive-pack)
push_run pathspec_add dev git add -- --exec --upload-pack --receive-pack
[[ "$RC" == 0 ]] || failures+=("pathspec_add: exit $RC, want 0: $OUT")
[[ "$(work_git diff --cached --name-only | sort)" == $'--exec\n--receive-pack\n--upload-pack' ]] ||
  failures+=("pathspec_add: files not staged: $(work_git diff --cached --name-only)")
for pathspec_sub in status diff restore; do
  push_run "pathspec_$pathspec_sub" dev git "$pathspec_sub" -- --exec
  [[ "$RC" == 0 ]] || failures+=("pathspec_$pathspec_sub: exit $RC, want 0: $OUT")
done
work_git reset -q
(cd "$PUSH_DIR/work" && rm -f -- --exec --upload-pack --receive-pack)

# 許可リストの名前は、どれも git の組み込みのサブコマンドである（git の alias は組み込みを上書きできないため、
# 同じ名前の alias を設定に置いても alias は使われない）。
git_allowed_subcommands=()
eval "$(grep '^git_allowed_subcommands=' "$SCRIPT")"
((${#git_allowed_subcommands[@]})) || failures+=("allowlist: git_allowed_subcommands not found in agent-gh")
builtins="$("$REAL_GIT" --list-cmds=builtins)"
for c in "${git_allowed_subcommands[@]}"; do
  grep -qxF -- "$c" <<<"$builtins" || failures+=("allowlist: $c is not a git builtin")
done
# 転送のオプションの検査は fetch・pull・push・ls-remote にだけ掛ける。それ以外の許可名が同じオプションを
# 解釈するなら検査の漏れになるため、-h の出力にそれらが無いことを確かめる
for c in "${git_allowed_subcommands[@]}"; do
  case "$c" in fetch|pull|push|ls-remote) continue ;; esac
  if "$REAL_GIT" "$c" -h 2>&1 | grep -qE -- '--(upload-pack|receive-pack|exec)( |,|=|\[|$)'; then
    failures+=("allowlist: $c has a transport option in -h")
  fi
done
work_git config alias.status '!touch alias-ran'
push_run alias_shadow dev git status
[[ "$RC" == 0 ]] || failures+=("alias_shadow: exit $RC, want 0: $OUT")
[[ ! -e "$PUSH_DIR/work/alias-ran" ]] || failures+=("alias_shadow: alias for a builtin ran")
work_git config --unset alias.status

# 呼び出し元の環境変数（GIT_CONFIG_COUNT・GIT_CONFIG_PARAMETERS）と repo の設定で core.hooksPath・include.path を
# 渡しても、一時フックの確認で止まり、remote は変わらない（agent-gh の -c が後で効く）。
expect_hook_block() {
  local label="$1"
  shift
  OUT="$(cd "$PUSH_DIR/work" && env "$@" PATH="$RGBIN:$PATH" bash "$SCRIPT" dev git push origin HEAD 2>&1)"
  RC=$?
  [[ "$RC" != 0 ]] || failures+=("env_block ($label): exit 0, want non-zero")
  [[ "$OUT" == *"push を止めました"* ]] || failures+=("env_block ($label): not stopped by the hook: $OUT")
  [[ "$(remote_ref agent/author_block)" == "$before" ]] || failures+=("env_block ($label): remote ref changed")
}
expect_hook_block count GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=core.hooksPath GIT_CONFIG_VALUE_0=/dev/null
expect_hook_block count-include GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=include.path GIT_CONFIG_VALUE_0="$PUSH_DIR/nohook.cfg"
expect_hook_block parameters "GIT_CONFIG_PARAMETERS='core.hookspath'='/dev/null'"
expect_hook_block parameters-include "GIT_CONFIG_PARAMETERS='include.path'='$PUSH_DIR/nohook.cfg'"
work_git config core.hooksPath /dev/null
expect_hook_block repo-hookspath X=1
work_git config --unset core.hooksPath
work_git config include.path "$PUSH_DIR/nohook.cfg"
expect_hook_block repo-include X=1
work_git config --unset include.path
# 設定のフック（hk が入れる hook.<名前>.*）が pre-push にあっても、一時フックの確認は走る
work_git config hook.other.event pre-push
work_git config hook.other.command true
expect_hook_block config-hook X=1
work_git config --unset hook.other.event
work_git config --unset hook.other.command

# 名義の確認を通った push では、core.hooksPath が指す既存の pre-push を続けて呼ぶ。パスにタブを含んでも
# 値全体を保って読む（git config のスコープと値の区切りはタブのため、タブ区切りで読むとパスの後ろが落ちて
# 別の場所を見に行き、元フックの拒否をすり抜ける）。拒否を返すフックが呼ばれ、remote の ref は変わらない。
work_git reset -q --hard "$before"
(cd "$PUSH_DIR/work" && as 'pelway-dev[bot]' commit -q --allow-empty -m orig-hook)
tabhookdir="$PUSH_DIR/hooks$(printf 'with\ttab')"
mkdir -p "$tabhookdir"
printf '#!/usr/bin/env bash\ntouch "$ORIGHOOK_CALLED"\nexit 1\n' >"$tabhookdir/pre-push"
chmod +x "$tabhookdir/pre-push"
work_git config core.hooksPath "$tabhookdir"
OUT="$(cd "$PUSH_DIR/work" && ORIGHOOK_CALLED="$PUSH_DIR/orighook-called" PATH="$RGBIN:$PATH" bash "$SCRIPT" dev git push origin HEAD 2>&1)"
RC=$?
work_git config --unset core.hooksPath
[[ "$RC" != 0 ]] || failures+=("orighook_tab: exit 0, want non-zero")
[[ -e "$PUSH_DIR/orighook-called" ]] || failures+=("orighook_tab: original pre-push not called")
[[ "$(remote_ref agent/author_block)" == "$before" ]] || failures+=("orighook_tab: remote ref changed")

# core.hooksPath の読み取り自体に失敗するとき（存在しないユーザの `~` を含む値は --type=path の展開で
# 失敗する。フック内では agent-gh の -c が command スコープで効くため、git log 等は動くが全値を
# 展開する --get-all --type=path は失敗する）は、既存のフックを確認できないまま push しない
work_git config core.hooksPath '~agent-gh-no-such-user/hooks'
OUT="$(cd "$PUSH_DIR/work" && PATH="$RGBIN:$PATH" bash "$SCRIPT" dev git push origin HEAD 2>&1)"
RC=$?
work_git config --unset-all core.hooksPath 2>/dev/null || sed -i '/hooksPath/d' "$PUSH_DIR/work/.git/config"
[[ "$RC" != 0 ]] || failures+=("cfg_read_fail: exit 0, want non-zero")
[[ "$OUT" == *"core.hooksPath の読み取りに失敗"* ]] || failures+=("cfg_read_fail: not stopped by the read failure: $OUT")
[[ "$(remote_ref agent/author_block)" == "$before" ]] || failures+=("cfg_read_fail: remote ref changed")

# 6. review の push も同じ確認を通る。正しい名義は pelway-review[bot]。
setup_push_repo review_push
(cd "$PUSH_DIR/work" && as 'pelway-review[bot]' commit -q --allow-empty -m r1)
PELWAY_AGENT_SKIP_RESIGN=1 push_run review_push review git push -u origin HEAD
[[ "$RC" == 0 ]] || failures+=("review_push: exit $RC, want 0: $OUT")
before="$(remote_ref agent/review_push)"
[[ "$before" == "$(work_git rev-parse HEAD)" ]] || failures+=("review_push: remote ref not updated")
(cd "$PUSH_DIR/work" && as 'pelway-dev[bot]' commit -q --allow-empty -m r2)
PELWAY_AGENT_SKIP_RESIGN=1 push_run review_push_block review git push
[[ "$RC" != 0 ]] || failures+=("review_push_block: exit 0, want non-zero")
[[ "$(remote_ref agent/review_push)" == "$before" ]] || failures+=("review_push_block: remote ref changed")
[[ "$OUT" == *"pelway-review[bot]"* && "$OUT" == *"author=pelway-dev[bot]"* ]] || failures+=("review_push_block: stderr lacks identity: $OUT")
unset GIT_CONFIG_GLOBAL GIT_CONFIG_NOSYSTEM

# --- gh の `gh event listen` の案内（#596）: agent-gh の出力に残さない。警告・エラー・stdout・終了コードは残す ---
for sub in pr api; do
  name="hint_removed_$sub"
  if [[ "$sub" == pr ]]; then args=(pr view 1 --json number); else args=(api repos/hatai/Pelway); fi
  FAKE_GH_HINT=1 FAKE_GH_EXIT=7 run "$name" dev "${args[@]}"
  [[ "$RC" == 7 ]] || failures+=("$name: exit $RC, want 7")
  [[ "$OUT" != *'gh event listen'* && "$OUT" != *'hint:'* && "$OUT" != *'Event subscriptions'* ]] ||
    failures+=("$name: gh event listen hint left in output")
  for want in STDOUT-BODY 'warning: before the hint' 'error: after the hint' '  - indented error detail'; do
    grep -qxF -- "$want" <<<"$OUT" || failures+=("$name: '$want' missing from output")
  done
done
# 案内と同じ書式の先頭行でも、続く行が `gh event listen` でなければ別の診断として残す（最後の行で終わる場合も）。
FAKE_GH_LOOKALIKE=1 run hint_lookalike_kept dev pr view 1
for want in 'hint: to see other fields, use this instead of polling:' '  - gh pr view --json' 'hint: trailing lookalike instead of polling:'; do
  grep -qxF -- "$want" <<<"$OUT" || failures+=("hint_lookalike_kept: '$want' missing from output")
done

# stdout と stderr は別のまま（案内の除去で stderr が stdout に混ざらない）
export CALL_LOG="$ROOT/hint_streams.log"
hint_stdout="$(FAKE_GH_HINT=1 bash "$SCRIPT" dev pr view 1 2>/dev/null)"
[[ "$hint_stdout" == STDOUT-BODY ]] || failures+=("hint_streams: stdout is '$hint_stdout', want only STDOUT-BODY")

if ((${#failures[@]})); then
  printf 'FAIL %s\n' "${failures[@]}"
  exit 1
fi
echo ok
