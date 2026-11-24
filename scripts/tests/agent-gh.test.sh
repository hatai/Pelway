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
# 偽の git が FAKE_GIT_REAL=1 で本物に委譲するためのパス（#642 の実リポジトリ試験用）
REAL_GIT="$(command -v git)"; export REAL_GIT

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
{ printf 'GH_TOKEN=%s\n' "${GH_TOKEN:-}"; printf 'GH_CONFIG_DIR=%s\n' "${GH_CONFIG_DIR:-}"; printf 'GH_NO_UPDATE_NOTIFIER=%s\n' "${GH_NO_UPDATE_NOTIFIER:-}"; printf 'GH_TELEMETRY=%s\n' "${GH_TELEMETRY:-}"; printf 'DO_NOT_TRACK=%s\n' "${DO_NOT_TRACK:-}"; printf 'ARGS=%s\n' "$*"; } >>"$CALL_LOG"
# #642: pr-size 例外の照会用の応答。FAKE_GH_FAIL があれば、どの問い合わせもそのコードで失敗する。
if [[ -n "${FAKE_GH_FAIL:-}" ]]; then
  exit "$FAKE_GH_FAIL"
fi
case "${1:-} ${2:-}" in
  "pr list") printf '%s\n' "${FAKE_GH_PR_LIST:-[]}" ;;
  "issue view") printf '%s\n' "${FAKE_GH_ISSUE_VIEW:-"{\"comments\":[]}"}" ;;
esac
exit 0
EOF

# 偽の git: GH_TOKEN と引数を追記する（push 後の agent-gh-resign からの呼び出しも残すため追記）。
# GH_CONFIG_DIR / GH_TELEMETRY / DO_NOT_TRACK も記録する（credential helper として呼ばれる gh への伝搬確認用）。
cat >"$BIN/git" <<'EOF'
#!/usr/bin/env bash
{ printf 'GH_TOKEN=%s\n' "${GH_TOKEN:-}"; printf 'GH_CONFIG_DIR=%s\n' "${GH_CONFIG_DIR:-}"; printf 'GH_TELEMETRY=%s\n' "${GH_TELEMETRY:-}"; printf 'DO_NOT_TRACK=%s\n' "${DO_NOT_TRACK:-}"; printf 'ARGS=%s\n' "$*"; } >>"$CALL_LOG"
# FAKE_GIT_REAL があれば、記録だけして本物の git に委譲する（#642 の pr-size ゲート試験の実リポジトリ用）。
if [[ -n "${FAKE_GIT_REAL:-}" ]]; then
  exec "$REAL_GIT" "$@"
fi
# FAKE_GIT_HEAD があれば、symbolic-ref（今のブランチ名の問い合わせ）にその名前を返す（#641）。
[[ "$*" != *symbolic-ref* || -z "${FAKE_GIT_HEAD:-}" ]] || printf '%s\n' "$FAKE_GIT_HEAD"
# FAKE_GIT_RECURSE があれば、config --get-regexp（push.recurseSubmodules / submodule.recurse）にその値を返す（#641）。
# 旧形の config --get push.recurseSubmodules にも、その中の push.recursesubmodules の最後の値を返す。
# GIT_CONFIG があると git config はそのファイルだけを読むため、そのときは FAKE_GIT_CONFIG_RECURSE を返す
# （push は GIT_CONFIG を見ずに通常の設定を読む。実装側の判定が同じ設定を読むことを確かめる）。
if [[ "$*" == "config --get push.recurseSubmodules" && -n "${FAKE_GIT_RECURSE:-}" && -z "${GIT_CONFIG:-}" ]]; then
  printf '%s\n' "$FAKE_GIT_RECURSE" | sed -n 's|^push\.recursesubmodules ||p' | tail -n 1
fi
if [[ "$*" == config\ --get-regexp* ]]; then
  if [[ -n "${GIT_CONFIG:-}" ]]; then
    [[ -z "${FAKE_GIT_CONFIG_RECURSE:-}" ]] || printf '%s\n' "$FAKE_GIT_CONFIG_RECURSE"
  else
    [[ -z "${FAKE_GIT_RECURSE:-}" ]] || printf '%s\n' "$FAKE_GIT_RECURSE"
  fi
fi
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

# --- push 先を resign に渡す（#641）---
# resign は位置引数で対象を受け取ると、その追跡 ref を `rev-parse --verify -q refs/remotes/<remote>/<branch>` で引く。
# 位置引数が無いときだけ `@{u}` を引く。偽の git の記録から、resign に渡った対象を読み取る。
# push_run <name> <git push の引数...>: リポジトリ外で agent-gh dev git push を実行する（出力は <name>.out）
push_run() {
  local name="$1"
  shift
  export CALL_LOG="$ROOT/$name.log"
  rm -f "$CALL_LOG"
  (cd "$ROOT/norepo" && bash "$SCRIPT" dev git push "$@" >"$ROOT/$name.out" 2>&1)
  RC=$?
}
# expect_push_targets <name> <want>: resign が引いた追跡 ref（remote/branch を空白区切り）と @{u} の照会回数
expect_push_targets() {
  local name="$1" want="$2" want_upstream="$3" got upstream
  got="$(sed -n 's|^ARGS=rev-parse --verify -q refs/remotes/||p' "$ROOT/$name.log" | paste -sd' ' -)"
  upstream="$(grep -c '^ARGS=rev-parse --abbrev-ref --symbolic-full-name @{u}$' "$ROOT/$name.log")"
  [[ "$RC" == 0 ]] || failures+=("$name: exit $RC, want 0 ($(cat "$ROOT/$name.out"))")
  [[ "$got" == "$want" ]] || failures+=("$name: resign targets '$got', want '$want'")
  [[ "$upstream" == "$want_upstream" ]] || failures+=("$name: @{u} lookups $upstream, want $want_upstream")
}

push_run push_src_dst origin feature:agent/X
expect_push_targets push_src_dst "origin/agent/X" 0
push_run push_force_refs_heads origin +feature:refs/heads/agent/Y
expect_push_targets push_force_refs_heads "origin/agent/Y" 0
push_run push_single origin agent/X
expect_push_targets push_single "origin/agent/X" 0
FAKE_GIT_HEAD=agent/cur push_run push_u_head -u origin HEAD
expect_push_targets push_u_head "origin/agent/cur" 0
FAKE_GIT_HEAD=agent/cur push_run push_head_dst origin HEAD:agent/X
expect_push_targets push_head_dst "origin/agent/X" 0
# 値を取るオプション（-o <値>）の値を refspec と取り違えない
push_run push_option_value -o ci.skip origin feature:agent/Z
expect_push_targets push_option_value "origin/agent/Z" 0
# refspec を省いた push は、今までどおり位置引数なしで resign を呼ぶ（resign が @{u} から決める）
push_run push_no_refspec
expect_push_targets push_no_refspec "" 1
push_run push_remote_only origin
expect_push_targets push_remote_only "" 1
# 複数のブランチの push は、ブランチごとに 1 回ずつ
push_run push_multi origin a:agent/A b:agent/B
expect_push_targets push_multi "origin/agent/A origin/agent/B" 0
# タグと削除では resign を呼ばない
push_run push_tag origin refs/tags/v1
expect_push_targets push_tag "" 0
push_run push_tag_dst origin v1:refs/tags/v1
expect_push_targets push_tag_dst "" 0
push_run push_delete_colon origin :agent/X
expect_push_targets push_delete_colon "" 0
push_run push_delete_flag --delete origin agent/X
expect_push_targets push_delete_flag "" 0
push_run push_delete_short -d origin agent/X
expect_push_targets push_delete_short "" 0
# 上のどれにも当たらない形（--all・--mirror）は、今までどおり位置引数なしで呼ぶ
push_run push_all --all origin
expect_push_targets push_all "" 1
push_run push_mirror --mirror origin
expect_push_targets push_mirror "" 1
# 試行（--dry-run / -n、まとめ書きの -fn も）では何も push されないため、resign を呼ばない
push_run push_dry_run --dry-run origin feature:agent/N
expect_push_targets push_dry_run "" 0
push_run push_dry_run_short -n origin feature:agent/N
expect_push_targets push_dry_run_short "" 0
push_run push_dry_run_bundle -fn origin feature:agent/N
expect_push_targets push_dry_run_bundle "" 0
# Git は一意に特定できる接頭辞の長いオプション省略形を受理する（--dry、--dr も試行になる。
# --d は --delete と曖昧で git が拒否するため push_targets には届かない）
push_run push_dry_run_abbrev_dr --dr origin feature:agent/N
expect_push_targets push_dry_run_abbrev_dr "" 0
push_run push_dry_run_abbrev_dry --dry origin feature:agent/N
expect_push_targets push_dry_run_abbrev_dry "" 0
push_run push_dry_run_abbrev_dry_ru --dry-ru origin feature:agent/N
expect_push_targets push_dry_run_abbrev_dry_ru "" 0
# まとめ書きの削除（-fd）でも resign を呼ばない
push_run push_delete_bundle -fd origin agent/X
expect_push_targets push_delete_bundle "" 0
# --delete の一意な省略形も削除
push_run push_delete_abbrev --del origin agent/X
expect_push_targets push_delete_abbrev "" 0
# 否定形（--no-）は直前の指定を戻す。-n --no-dry-run は実 push になるため署名を付け直す
push_run push_no_dry_run -n --no-dry-run origin feature:agent/N
expect_push_targets push_no_dry_run "origin/agent/N" 0
push_run push_no_dry_abbrev -n --no-dr origin feature:agent/N
expect_push_targets push_no_dry_abbrev "origin/agent/N" 0
push_run push_no_delete -d --no-delete origin agent/X
expect_push_targets push_no_delete "origin/agent/X" 0
# fallback を立てる形の否定は fallback を戻す
push_run push_no_all --all --no-all origin a:agent/A
expect_push_targets push_no_all "origin/agent/A" 0
# 値を別の引数で取る長いオプションの省略形は、次の引数を値として消費する（remote と取り違えない）
push_run push_pushopt_abbrev --push-o ci.skip origin feature:agent/Z
expect_push_targets push_pushopt_abbrev "origin/agent/Z" 0
push_run push_exec_abbrev --ex pack origin feature:agent/Z
expect_push_targets push_exec_abbrev "origin/agent/Z" 0
push_run push_recurse_abbrev --recu check origin feature:agent/Z
expect_push_targets push_recurse_abbrev "origin/agent/Z" 0
# =<値> の連結形は次の引数を取らない
push_run push_pushopt_joined --push-option=x origin feature:agent/Z
expect_push_targets push_pushopt_joined "origin/agent/Z" 0
# fallback を立てる形の一意な省略形も fallback
push_run push_all_abbrev --al origin
expect_push_targets push_all_abbrev "" 1
# --repo は値を取る: 値として --no-dry-run を消費するため dry-run のまま（何も push されない）
push_run push_repo_value_no_dry -n --repo --no-dry-run --no-all origin feature:agent/X
expect_push_targets push_repo_value_no_dry "" 0
# --repo が -n を値として消費すると実 push になる（fallback で resign を呼ぶ）
push_run push_repo_value_flag --repo -n origin HEAD:agent/X
expect_push_targets push_repo_value_flag "" 1
# --end-of-options 以降はオプションとして解釈しない。--no-dry-run も位置引数のため dry-run のまま
push_run push_end_of_options -n --end-of-options origin --no-dry-run
expect_push_targets push_end_of_options "" 0
# --recurse-submodules=only はサブモジュールだけを送り親リポジトリを更新しないため resign を呼ばない
push_run push_submod_only --recurse-submodules=only origin a:agent/X
expect_push_targets push_submod_only "" 0
# 連結形の一意な省略形でも同じ
push_run push_submod_only_abbrev --recu=only origin a:agent/X
expect_push_targets push_submod_only_abbrev "" 0
# 別引数で取る省略形でも同じ
push_run push_submod_only_sep --recu only origin a:agent/X
expect_push_targets push_submod_only_sep "" 0
# 後に書いた指定が勝つ: 否定形と別の値で解除される
push_run push_submod_only_undone --recu=only --no-recurse-submodules origin a:agent/X
expect_push_targets push_submod_only_undone "origin/agent/X" 0
push_run push_submod_only_recheck --recu=only --recurse-submodules=check origin a:agent/X
expect_push_targets push_submod_only_recheck "origin/agent/X" 0
# push.recurseSubmodules=only の設定でも resign を呼ばない（CLI で上書きすれば呼ぶ）。
# Git は submodule.recurse も読み、後に読んだ設定を使うため、両キーの最後の項目で決める
export FAKE_GIT_RECURSE='push.recursesubmodules only'
push_run push_submod_only_config origin a:agent/X
expect_push_targets push_submod_only_config "" 0
push_run push_submod_config_cli --recurse-submodules=check origin a:agent/X
expect_push_targets push_submod_config_cli "origin/agent/X" 0
# submodule.recurse が後に読まれれば only は解除される（真偽値で only を取り得ないため親も送る）
export FAKE_GIT_RECURSE='push.recursesubmodules only
submodule.recurse true'
push_run push_submod_config_conflict origin a:agent/X
expect_push_targets push_submod_config_conflict "origin/agent/X" 0
# push.recurseSubmodules=only が後に読まれれば only のまま
export FAKE_GIT_RECURSE='submodule.recurse true
push.recursesubmodules only'
push_run push_submod_config_last_only origin a:agent/X
expect_push_targets push_submod_config_last_only "" 0
# GIT_CONFIG が別のファイルを指していても、判定は push が読む通常の設定で行う（#641 r11）。
# 通常の設定が only（GIT_CONFIG 側には only が無い）なら親を送っていないため resign を呼ばない
export FAKE_GIT_RECURSE='push.recursesubmodules only'
GIT_CONFIG="$ROOT/gitconfig-elsewhere" push_run push_submod_gitconfig_ignored origin a:agent/X
expect_push_targets push_submod_gitconfig_ignored "" 0
# 逆に GIT_CONFIG 側のファイルにだけ only があっても、push は通常の設定で親を送るため resign を呼ぶ
unset FAKE_GIT_RECURSE
GIT_CONFIG="$ROOT/gitconfig-elsewhere" FAKE_GIT_CONFIG_RECURSE='push.recursesubmodules only' \
  push_run push_submod_gitconfig_only origin a:agent/X
expect_push_targets push_submod_gitconfig_only "origin/agent/X" 0
unset FAKE_GIT_CONFIG_RECURSE GIT_CONFIG
push_run push_mirror_abbrev --mi origin
expect_push_targets push_mirror_abbrev "" 1
push_run push_tags_abbrev --ta origin v1
expect_push_targets push_tags_abbrev "" 1
# まとめ書きの末尾が -o のとき、次の引数はその値（remote と取り違えない）。
# -f + -o のまとめ書きはスペルチェッカの誤検出になるため -u + -o（-uo）で確かめる
push_run push_bundle_option_tail -uo ci.skip origin feature:agent/Z
expect_push_targets push_bundle_option_tail "origin/agent/Z" 0
# 名前空間つきの宛先 refs/heads/refs/topic は、ブランチ名 refs/topic として署名対象に入る
push_run push_refs_heads_branch origin x:refs/heads/refs/topic
expect_push_targets push_refs_heads_branch "origin/refs/topic" 0

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

# --- review round 1 の must-fix: app_token の subshell 途中終了でも秘密鍵の一時ファイルが残らない ---
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

# --- push 前の pr-size ゲート（#642）---
# 差分の行数を実際に数える必要があるため、実リポジトリで FAKE_GIT_REAL=1（偽の git が
# 記録だけして本物に委譲する）にして試す。origin はローカルの bare リポジトリで、実 push が届く。
gate_repo="$ROOT/gate-repo"
"$REAL_GIT" init -q -b main "$gate_repo"
"$REAL_GIT" -C "$gate_repo" config user.name test
"$REAL_GIT" -C "$gate_repo" config user.email test@example.invalid
"$REAL_GIT" -C "$gate_repo" config commit.gpgsign false
"$REAL_GIT" -C "$gate_repo" commit -q --allow-empty -m base
"$REAL_GIT" -C "$gate_repo" update-ref refs/remotes/origin/main HEAD
"$REAL_GIT" init --bare -q "$ROOT/gate-bare.git"
"$REAL_GIT" -C "$gate_repo" remote add origin "$ROOT/gate-bare.git"
# ブランチ: agent/642-x +810（上限超え）、feature +900、head-small +10、agent/642-small +12
"$REAL_GIT" -C "$gate_repo" switch -qc agent/642-x
seq 810 >"$gate_repo/big.txt"; "$REAL_GIT" -C "$gate_repo" add -A; "$REAL_GIT" -C "$gate_repo" commit -qm add
"$REAL_GIT" -C "$gate_repo" switch -qc feature main
seq 900 >"$gate_repo/big2.txt"; "$REAL_GIT" -C "$gate_repo" add -A; "$REAL_GIT" -C "$gate_repo" commit -qm add
"$REAL_GIT" -C "$gate_repo" switch -qc head-small main
seq 10 >"$gate_repo/small.txt"; "$REAL_GIT" -C "$gate_repo" add -A; "$REAL_GIT" -C "$gate_repo" commit -qm add
"$REAL_GIT" -C "$gate_repo" switch -qc agent/642-small main
seq 12 >"$gate_repo/small2.txt"; "$REAL_GIT" -C "$gate_repo" add -A; "$REAL_GIT" -C "$gate_repo" commit -qm add
# merge-base の確認用: mb-branch / mb-unset は同じ先端（feature と同じ +900）。mb-branch だけ
# gh-merge-base=topic を持ち、origin/topic はその先端を指す（config が効けば差分 0 になる）
"$REAL_GIT" -C "$gate_repo" switch -qc mb-branch feature
"$REAL_GIT" -C "$gate_repo" update-ref refs/remotes/origin/topic HEAD
"$REAL_GIT" -C "$gate_repo" switch -qc mb-unset
"$REAL_GIT" -C "$gate_repo" config branch.mb-branch.gh-merge-base topic
"$REAL_GIT" -C "$gate_repo" tag v1 # 先端が +900 のタグ。タグ push が検査されなければ通る
"$REAL_GIT" -C "$gate_repo" push -q origin mb-unset:refs/heads/tmp-del # 削除試験の対象
"$REAL_GIT" -C "$gate_repo" switch -q agent/642-x

# push_gate <name> <git push の引数...>: gate_repo で agent-gh dev git push を実行する（OUT/RC に入る）
push_gate() {
  local name="$1"
  shift
  export CALL_LOG="$ROOT/$name.log"
  rm -f "$CALL_LOG"
  OUT="$(cd "$gate_repo" && FAKE_GIT_REAL=1 bash "$SCRIPT" dev git push "$@" 2>&1)"
  RC=$?
}
# push_gate_skip: PELWAY_AGENT_SKIP_RESIGN=1 で同じことをする。通す側の試験（resign の成否を外す）と、
# skip の経路でもゲートが効くことの確認に使う（resign 自体の試験は #641 で別にある）
push_gate_skip() {
  local name="$1"
  shift
  export CALL_LOG="$ROOT/$name.log"
  rm -f "$CALL_LOG"
  OUT="$(cd "$gate_repo" && FAKE_GIT_REAL=1 PELWAY_AGENT_SKIP_RESIGN=1 bash "$SCRIPT" dev git push "$@" 2>&1)"
  RC=$?
}
expect_push_called() {
  grep -qE '^ARGS=.* push ' "$ROOT/$1.log" || failures+=("$1: git push was not invoked")
}
expect_no_push() {
  ! grep -qE '^ARGS=.* push ' "$ROOT/$1.log" || failures+=("$1: git push was invoked")
}
# pr list / issue view（例外の照会）が呼ばれていないことを確かめる
expect_no_pr_query() {
  [[ -e "$ROOT/$1.log" ]] || return 0
  ! grep -qE '^ARGS=(pr list|issue view)' "$ROOT/$1.log" || failures+=("$1: PR query was issued")
}
# 差分を測っていないこと（pr-size の git diff --numstat が無い）を確かめる
expect_no_measure() {
  ! grep -qF 'numstat' "$ROOT/$1.log" || failures+=("$1: pr-size measured the diff")
}

# 上限超えは push しない。終了コードは 0 以外（6）、stderr に行数・base・例外の条件が出る
push_gate gate_over_limit origin HEAD
[[ "$RC" == 6 ]] || failures+=("gate_over_limit: exit $RC, want 6")
expect_no_push gate_over_limit
grep -qF '+810 / -0' <<<"$OUT" || failures+=("gate_over_limit: stderr lacks counts: $OUT")
grep -qF 'base: origin/main' <<<"$OUT" || failures+=("gate_over_limit: stderr lacks base: $OUT")
grep -qF 'pr-size-exception' <<<"$OUT" || failures+=("gate_over_limit: stderr lacks exception conditions: $OUT")

# 例外があれば通す: 開いた PR の pr-size-exception ラベル + 閉じる Issue の hatai の目印コメント
FAKE_GH_PR_LIST='[{"number":42,"labels":[{"name":"pr-size-exception"}],"closingIssuesReferences":[{"number":640}]}]' \
FAKE_GH_ISSUE_VIEW='{"comments":[{"author":{"login":"hatai"},"body":"<!-- pr-size-exception -->\nこの PR の超過を認める"}]}' \
  push_gate_skip gate_exception origin HEAD
[[ "$RC" == 0 ]] || failures+=("gate_exception: exit $RC, want 0 ($OUT)")
expect_push_called gate_exception

# ラベルだけ（閉じる Issue に目印コメントが無い）では止める
FAKE_GH_PR_LIST='[{"number":42,"labels":[{"name":"pr-size-exception"}],"closingIssuesReferences":[{"number":640}]}]' \
FAKE_GH_ISSUE_VIEW='{"comments":[{"author":{"login":"hatai"},"body":"この PR の超過を認める"}]}' \
  push_gate gate_label_only origin HEAD
[[ "$RC" == 6 ]] || failures+=("gate_label_only: exit $RC, want 6")
expect_no_push gate_label_only

# コメントだけ（ラベル無し）では止める
FAKE_GH_PR_LIST='[{"number":42,"labels":[],"closingIssuesReferences":[{"number":640}]}]' \
FAKE_GH_ISSUE_VIEW='{"comments":[{"author":{"login":"hatai"},"body":"<!-- pr-size-exception -->\n認める"}]}' \
  push_gate gate_comment_only origin HEAD
[[ "$RC" == 6 ]] || failures+=("gate_comment_only: exit $RC, want 6")
expect_no_push gate_comment_only

# 目印コメントの作成者が hatai でないときは止める
FAKE_GH_PR_LIST='[{"number":42,"labels":[{"name":"pr-size-exception"}],"closingIssuesReferences":[{"number":640}]}]' \
FAKE_GH_ISSUE_VIEW='{"comments":[{"author":{"login":"someone"},"body":"<!-- pr-size-exception -->\n認める"}]}' \
  push_gate gate_not_hatai origin HEAD
[[ "$RC" == 6 ]] || failures+=("gate_not_hatai: exit $RC, want 6")
expect_no_push gate_not_hatai

# 目印が本文の行頭でないコメントは止める
FAKE_GH_PR_LIST='[{"number":42,"labels":[{"name":"pr-size-exception"}],"closingIssuesReferences":[{"number":640}]}]' \
FAKE_GH_ISSUE_VIEW='{"comments":[{"author":{"login":"hatai"},"body":"x <!-- pr-size-exception -->\n認める"}]}' \
  push_gate gate_marker_not_at_start origin HEAD
[[ "$RC" == 6 ]] || failures+=("gate_marker_not_at_start: exit $RC, want 6")
expect_no_push gate_marker_not_at_start

# 開いた PR が無いときは例外を調べず行数だけで判定する（issue view は呼ばない）
FAKE_GH_PR_LIST='[]' push_gate gate_no_pr origin HEAD
[[ "$RC" == 6 ]] || failures+=("gate_no_pr: exit $RC, want 6")
expect_no_push gate_no_pr
! grep -qF 'issue view' "$ROOT/gate_no_pr.log" || failures+=("gate_no_pr: issue view was called")

# GitHub の問い合わせが失敗したときは止める（fail-closed）
FAKE_GH_FAIL=1 push_gate gate_gh_fail origin HEAD
[[ "$RC" == 6 ]] || failures+=("gate_gh_fail: exit $RC, want 6")
expect_no_push gate_gh_fail

# PELWAY_AGENT_SKIP_RESIGN=1 の経路でも検査される
push_gate_skip gate_skip_resign origin HEAD
[[ "$RC" == 6 ]] || failures+=("gate_skip_resign: exit $RC, want 6")
expect_no_push gate_skip_resign

# src:dst は src の差分を数える（HEAD が上限内でも src が超えれば止める）
"$REAL_GIT" -C "$gate_repo" switch -q head-small # HEAD は +10（上限内）
push_gate gate_src_diff origin feature:agent/642-x
[[ "$RC" == 6 ]] || failures+=("gate_src_diff: exit $RC, want 6")
expect_no_push gate_src_diff
grep -qF '+900 / -0' <<<"$OUT" || failures+=("gate_src_diff: counts not from src: $OUT")

# base は branch.<名>.gh-merge-base があれば origin/<その値>（同じ先端でも config で結果が変わる）
"$REAL_GIT" -C "$gate_repo" switch -q mb-branch
push_gate_skip gate_merge_base origin HEAD
[[ "$RC" == 0 ]] || failures+=("gate_merge_base: exit $RC, want 0 ($OUT)")
expect_push_called gate_merge_base
"$REAL_GIT" -C "$gate_repo" switch -q mb-unset
push_gate gate_merge_base_unset origin HEAD
[[ "$RC" == 6 ]] || failures+=("gate_merge_base_unset: exit $RC, want 6")
expect_no_push gate_merge_base_unset
grep -qF 'base: origin/main' <<<"$OUT" || failures+=("gate_merge_base_unset: stderr lacks base: $OUT")

# 上限以下は例外を調べずに push する（GitHub への問い合わせが無い）
"$REAL_GIT" -C "$gate_repo" switch -q agent/642-small
push_gate_skip gate_under_limit origin HEAD
[[ "$RC" == 0 ]] || failures+=("gate_under_limit: exit $RC, want 0 ($OUT)")
expect_push_called gate_under_limit
expect_no_pr_query gate_under_limit

# タグの push は検査しない（先端が上限超えの内容でも通る）
"$REAL_GIT" -C "$gate_repo" switch -q main
push_gate_skip gate_tag origin v1
[[ "$RC" == 0 ]] || failures+=("gate_tag: exit $RC, want 0 ($OUT)")
expect_push_called gate_tag
expect_no_measure gate_tag

# 削除は検査しない
push_gate gate_delete origin :tmp-del
[[ "$RC" == 0 ]] || failures+=("gate_delete: exit $RC, want 0 ($OUT)")
expect_no_measure gate_delete

# --dry-run は検査しない（上限超えのブランチでも試行は通る）
"$REAL_GIT" -C "$gate_repo" switch -q agent/642-x
push_gate_skip gate_dry_run --dry-run origin HEAD
[[ "$RC" == 0 ]] || failures+=("gate_dry_run: exit $RC, want 0 ($OUT)")
expect_no_measure gate_dry_run

if ((${#failures[@]})); then
  printf 'FAIL %s\n' "${failures[@]}"
  exit 1
fi
echo ok
