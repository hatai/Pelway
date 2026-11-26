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
# pr-size の例外の照会（#642）: 呼び出しを $CALL_LOG.gate に追記し、FAKE_GH_PR_LIST・FAKE_GH_ISSUE_VIEW を返す。
# FAKE_GH_FAIL_PR・FAKE_GH_FAIL_ISSUE があれば、その照会だけを失敗させる
case "${1:-} ${2:-}" in
  "pr list") printf 'ARGS=%s\n' "$*" >>"$CALL_LOG.gate"; [[ -z "${FAKE_GH_FAIL_PR:-}" ]] || exit 1; printf '%s\n' "${FAKE_GH_PR_LIST:-[]}" ;;
  "issue view") printf 'ARGS=%s\n' "$*" >>"$CALL_LOG.gate"; [[ -z "${FAKE_GH_FAIL_ISSUE:-}" ]] || exit 1; printf '%s\n' "${FAKE_GH_ISSUE_VIEW:-"{\"comments\":[]}"}" ;;
esac
EOF

# 偽の git: GH_TOKEN と引数を追記する（push 後の agent-gh-resign からの呼び出しも残すため追記）。
# GH_CONFIG_DIR / GH_TELEMETRY / DO_NOT_TRACK も記録する（credential helper として呼ばれる gh への伝搬確認用）。
cat >"$BIN/git" <<'EOF'
#!/usr/bin/env bash
{ printf 'GH_TOKEN=%s\n' "${GH_TOKEN:-}"; printf 'GH_CONFIG_DIR=%s\n' "${GH_CONFIG_DIR:-}"; printf 'GH_TELEMETRY=%s\n' "${GH_TELEMETRY:-}"; printf 'DO_NOT_TRACK=%s\n' "${DO_NOT_TRACK:-}"; printf 'ARGS=%s\n' "$*"; } >>"$CALL_LOG"
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

# CDPATH に項目（`.` など）があると、bash の cd は絶対パスと、先頭の要素が `.`・`..` のパス（`./x`・`../x`）を除く相対パス（`.hidden/x` も含む）を
# CDPATH の項目から探し、空でない項目で見つけたとき移動先を標準出力に出す（#700）。標準の呼び方 `scripts/agent-gh` では $0 が相対のため、`$(cd "$(dirname "$0")" && pwd)` の値にその行が混ざり、resign のパスが壊れて
# push の成功後に終了コード 4 になる。`./` で始まらない相対の $0（シンボリックリンク bin 経由）と CDPATH=. で、resign まで走って 0 で終わる。
ln -s "$(dirname "$SCRIPT")" "$ROOT/norepo/bin"
export CALL_LOG="$ROOT/cdpath_rel0_resign.log"
rm -f "$CALL_LOG"
(cd "$ROOT/norepo" && CDPATH=. bash bin/agent-gh dev git push -u origin HEAD >"$ROOT/cdpath_rel0_resign.out" 2>&1)
push_rc=$?
[[ "$push_rc" == 0 ]] || failures+=("cdpath_rel0_resign: exit $push_rc, want 0: $(cat "$ROOT/cdpath_rel0_resign.out")")
expect_log cdpath_rel0_resign "rev-parse --abbrev-ref --symbolic-full-name @{u}"
rm -f "$ROOT/norepo/bin"

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
# --ex は --exec の一意な省略形。次の引数を値として消費する形だが、転送コマンドの指定なので push_targets より前に拒否される
push_run push_exec_abbrev --ex pack origin feature:agent/Z
[[ "$RC" == 2 ]] || failures+=("push_exec_abbrev: exit $RC, want 2: $(cat "$ROOT/push_exec_abbrev.out")")
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

# --- push するコミットの名義の確認（#613）。本物の git とローカルの bare remote で確かめる ---
# 利用者の git 設定（署名・フック）に左右されないよう、この節では global / system の設定を読ませない。
# このファイルの既存の push 試験は名義・署名・pr-size の門に絞るため、verify の門（#807）は外す。
# verify の門の既定の動きは scripts/tests/agent-gh-verify-gate.test.sh で確かめる。
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1 PELWAY_AGENT_SKIP_VERIFY_GATE=1
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
# setup_push_repo <name>: remote.git（bare）と、それを clone した work を作り、PUSH_DIR に置き場所を入れる。
# main には hatai のコミットと kodiakhq[bot]（committer は GitHub）のマージコミットを置き、work では agent/<name> ブランチを切る。
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

# 1. 範囲は「push する ref から辿れて origin の追跡 ref から辿れないコミット」。main から取り込んだ hatai のコミットと remote に既にある kodiakhq[bot] のマージコミットがあっても push は通る。
setup_push_repo author_range
(cd "$PUSH_DIR/work" && as 'pelway-dev[bot]' commit -q --allow-empty -m a1)
(cd "$PUSH_DIR/seed" && as hatai commit -q --allow-empty -m main2 && "$REAL_GIT" push -q "$PUSH_DIR/remote.git" main)
work_git fetch -q origin
(cd "$PUSH_DIR/work" && as 'pelway-dev[bot]' merge -q --no-ff -m 'Merge main' origin/main)
PELWAY_AGENT_SKIP_RESIGN=1 push_run author_range dev git push -u origin HEAD
[[ "$RC" == 0 ]] || failures+=("author_range: exit $RC, want 0: $OUT")
[[ "$(remote_ref agent/author_range)" == "$(work_git rev-parse HEAD)" ]] || failures+=("author_range: remote ref not updated")

# 2・3. hatai 名義のコミット（author と committer の両方、または片方だけ）を含む push は止まる。remote の ref は変わらず、標準エラーに短い sha・名義・作り直しの案内が出る。
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

# 5. 確認を外す環境変数・引数は無い: PELWAY_AGENT_SKIP_RESIGN=1・--no-verify（その省略形）・core.hooksPath の上書きのどれでも push されない。
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

# -c と設定値を連結した形（-ccore.hooksPath=...）も agent-gh が終了コード 2 で拒否する（git 2.56 は連結形を unknown option の 129 で拒むが、受け付ける版ではフックを外す引数になる）。
for attached in "-ccore.hooksPath=/dev/null" "-cCORE.HOOKSPATH=/dev/null"; do
  push_run attached_c dev git "$attached" push origin HEAD
  [[ "$RC" == 2 ]] || failures+=("attached_c ($attached): exit $RC, want 2: $OUT")
  [[ "$OUT" == *"core.hooksPath"* ]] || failures+=("attached_c ($attached): stderr lacks reason: $OUT")
  [[ "$(remote_ref agent/author_block)" == "$before" ]] || failures+=("attached_c ($attached): remote ref changed")
done

# 名前や設定の読み込み先を変えてフックを外す経路も、agent-gh が終了コード 2 で拒否し、remote は変わらない:
# -c include.path / includeIf.<条件>.path（読み込んだ設定の core.hooksPath を効かせる）・-c alias.<x>=push と '!git push' 形（サブコマンドの名前を変える）・repo の設定に置いた alias・pre-push フックを通らない配管コマンド（send-pack）
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
# 許可リストの名前を空白でつないだ 1 つの引数は、部分一致では通ってしまう。空白を含む subsection 形の alias（環境変数でも repo の設定でも置ける）で push になるため、完全一致で拒否する。
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
# -o 以外の値を取るオプション（--jobs・-o の省略形 --push-opt など）の値は区別せず一律に走査する（git は値として読むが、誤って値と見なしても拒否側に倒れる）。それらの値が `--`・-o・ls-remote の位置引数に見えるときも同じ（`push --repo -- --no-verify` では `--` が --repo の値で --no-verify は効く）
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
# 子はオプションとして解釈する（実測: `git pull -- . --upload-pack=<cmd>` は <cmd> を実行する）ため、`--` を終端にせず末尾まで走査する
expect_rejected ddash-pull pull -- . "--upload-pack=touch $ran; false"
expect_rejected ddash-pull-after-o pull -o x -- "--receive-pack=touch $ran; false"
expect_rejected ddash-repo push -- "--receive-pack=touch $ran; false"
expect_rejected ddash-after-o push -o x -- --exec
expect_rejected ddash-submodule push --recurse-submodules=on-demand -- origin --no-verify
[[ ! -e "$ran" ]] || failures+=("transport options: a command ran")

# 値として消費される引数・参照パターン・確かな `--` の後ろの位置引数は、git がオプションとして解釈しないため通す。agent-gh が終了コード 2 を返さないことと、コマンドが実行されないことを確かめる
expect_not_rejected() {
  local label="$1"
  shift
  push_run "$label" dev git "$@"
  # 終了コード 2 が agent-gh の引数の拒否（pre-push フックの名義確認は 1。git 自身は 2 を返さない）
  [[ "$RC" != 2 ]] || failures+=("not_rejected ($label): exit $RC, want != 2: $OUT")
  [[ "$(remote_ref agent/author_block)" == "$before" ]] || failures+=("not_rejected ($label): remote ref changed")
}
# -o（push の --push-option・fetch・pull・ls-remote の --server-option）の値はオプションとして解釈されない（-o の値は remote が受け付けず git が失敗するが、agent-gh の拒否ではない）
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
# fetch・ls-remote では clean な `--` の後ろは refspec・参照パターンで、オプションとしては解釈されない（`--upload-pack` が refspec・パターン扱いで git が失敗するのは、agent-gh の拒否ではない）
expect_not_rejected ddash-fetch fetch -- "--upload-pack=touch $ran; false" origin
expect_not_rejected ddash-ls-remote ls-remote -o x -- --exec
[[ ! -e "$ran" ]] || failures+=("transport passthrough: a command ran")

# 検査対象外のサブコマンドでは、`--` の後ろに転送オプションと同じ名前のファイルがあっても通す（`git add -- --exec` のようなパスの指定を拒否しない）。
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

# 許可リストの名前は、どれも git の組み込みのサブコマンドである（git の alias は組み込みを上書きできないため、同じ名前の alias を設定に置いても使われない）。
git_allowed_subcommands=()
eval "$(grep '^git_allowed_subcommands=' "$SCRIPT")"
((${#git_allowed_subcommands[@]})) || failures+=("allowlist: git_allowed_subcommands not found in agent-gh")
builtins="$("$REAL_GIT" --list-cmds=builtins)"
for c in "${git_allowed_subcommands[@]}"; do
  grep -qxF -- "$c" <<<"$builtins" || failures+=("allowlist: $c is not a git builtin")
done
# 転送のオプションの検査は fetch・pull・push・ls-remote にだけ掛ける。それ以外の許可名が同じオプションを解釈するなら検査の漏れになるため、-h の出力にそれらが無いことを確かめる
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

# 呼び出し元の環境変数（GIT_CONFIG_COUNT・GIT_CONFIG_PARAMETERS）と repo の設定で core.hooksPath・include.path を渡しても、一時フックの確認で止まり、remote は変わらない（agent-gh の -c が後で効く）。
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

# 名義の確認を通った push では、core.hooksPath が指す既存の pre-push を続けて呼ぶ。パスにタブを含んでも値全体を保って読む（git config のスコープと値の区切りはタブのため、タブ区切りで読むとパスの後ろが落ちて別の場所を見に行き、元フックの拒否をすり抜ける）。拒否を返すフックが呼ばれ、remote の ref は変わらない。
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

# core.hooksPath の読み取り自体に失敗するとき（存在しないユーザの `~` を含む値は --type=path の展開で失敗する。フック内では agent-gh の -c が command スコープで効くため git log 等は動くが、全値を展開する --get-all --type=path は失敗する）は、既存のフックを確認できないまま push しない
work_git config core.hooksPath '~agent-gh-no-such-user/hooks'
OUT="$(cd "$PUSH_DIR/work" && PATH="$RGBIN:$PATH" bash "$SCRIPT" dev git push origin HEAD 2>&1)"
RC=$?
work_git config --unset-all core.hooksPath 2>/dev/null || sed -i '/hooksPath/d' "$PUSH_DIR/work/.git/config"
[[ "$RC" != 0 ]] || failures+=("cfg_read_fail: exit 0, want non-zero")
[[ "$OUT" == *"core.hooksPath の読み取りに失敗"* ]] || failures+=("cfg_read_fail: not stopped by the read failure: $OUT")
[[ "$(remote_ref agent/author_block)" == "$before" ]] || failures+=("cfg_read_fail: remote ref changed")

# core.hooksPath の照会先を GIT_CONFIG で別ファイルに固定された環境では、フック内の照会はそのファイルだけを読む（agent-gh が -c で渡した一時フック設定も照会の結果には出ない）。終了コード 1 を「キー無し＝既定の場所」と読む旧来の判定では、この照会で command スコープの値に一時フックの dir が無いとき既存のフックを見逃したまま push してしまう。照会結果の不整合も fail closed で止め、既存の拒否フックの置き場所が GIT_CONFIG の側にあっても remote の ref は変わらない。
deny_hooks="$PUSH_DIR/deny-hooks"
mkdir -p "$deny_hooks"
printf '#!/usr/bin/env bash\nexit 1\n' >"$deny_hooks/pre-push"
chmod +x "$deny_hooks/pre-push"
printf '[core]\n\thooksPath = %s\n[remote "origin"]\n\turl = %s\n' "$deny_hooks" "$PUSH_DIR/remote.git" >"$PUSH_DIR/hooks.cfg"
work_git reset -q --hard "$before"
(cd "$PUSH_DIR/work" && as 'pelway-dev[bot]' commit -q --allow-empty -m cfg-env-override)
OUT="$(cd "$PUSH_DIR/work" && GIT_CONFIG="$PUSH_DIR/hooks.cfg" PATH="$RGBIN:$PATH" bash "$SCRIPT" dev git push origin HEAD 2>&1)"
RC=$?
[[ "$RC" != 0 ]] || failures+=("cfg_env_override: exit 0, want non-zero")
[[ "$OUT" == *"一時フックの設定を確認できない"* ]] || failures+=("cfg_env_override: not stopped by the scope check: $OUT")
[[ "$(remote_ref agent/author_block)" == "$before" ]] || failures+=("cfg_env_override: remote ref changed")

# TMPDIR が相対パスだと mktemp は呼び出しの cwd 基準の相対パスを返すが、git は core.hooksPath の相対パスを worktree ルート基準で解決する。サブディレクトリからの呼び出しでは基準がずれて一時フックを見つけられず、名義の確認なしに push が通る（実測: 存在しない core.hooksPath は git が「フックなし」として扱い、push を失敗させない）。絶対パスへの解決で、サブディレクトリからでも名義の確認が効き、正しい名義の push は通る。
mkdir -p "$PUSH_DIR/work/sub/dir/rel-tmp"
work_git reset -q --hard "$before"
(cd "$PUSH_DIR/work" && as hatai commit -q --allow-empty -m bad-subdir)
bad_sha="$(work_git rev-parse --short HEAD)"
OUT="$(cd "$PUSH_DIR/work/sub/dir" && TMPDIR=rel-tmp PATH="$RGBIN:$PATH" bash "$SCRIPT" dev git push origin HEAD 2>&1)"
RC=$?
[[ "$RC" != 0 ]] || failures+=("rel_tmpdir_block: exit 0, want non-zero")
[[ "$OUT" == *"$bad_sha"* && "$OUT" == *"push を止めました"* ]] || failures+=("rel_tmpdir_block: not stopped by the hook: $OUT")
[[ "$(remote_ref agent/author_block)" == "$before" ]] || failures+=("rel_tmpdir_block: remote ref changed")
work_git reset -q --hard "$before"
(cd "$PUSH_DIR/work" && as 'pelway-dev[bot]' commit -q --allow-empty -m ok-subdir)
OUT="$(cd "$PUSH_DIR/work/sub/dir" && TMPDIR=rel-tmp PELWAY_AGENT_SKIP_RESIGN=1 PATH="$RGBIN:$PATH" bash "$SCRIPT" dev git push origin HEAD 2>&1)"
RC=$?
[[ "$RC" == 0 ]] || failures+=("rel_tmpdir_ok: exit $RC, want 0: $OUT")
[[ "$(remote_ref agent/author_block)" == "$(work_git rev-parse HEAD)" ]] || failures+=("rel_tmpdir_ok: remote ref not updated")
work_git reset -q --hard "$before"
rm -rf "$PUSH_DIR/work/sub"

# 相対 TMPDIR と CDPATH の併用（#700）。CDPATH に項目（`.` など）があると、bash の cd は絶対パスと、先頭の要素が `.`・`..` のパス（`./x`・`../x`）を
# 除く相対パス（`.hidden/x` も含む）を CDPATH の項目から探し、空でない項目で見つけたとき移動先を標準出力に出す。mktemp が返す相対の一時フックの dir を
# `$(cd … && pwd -P)` で解決すると、その行が値に混ざって改行入りのパスになり、一時フックを書けずに正しい名義の push も止まる。その cd でだけ CDPATH を
# 空にして解決すれば、名義の確認は効き（止まるべき push は止まる）、正しい名義の push は通る。
setup_push_repo cdpath
mkdir -p "$PUSH_DIR/work/sub/dir/rel-tmp"
base="$(work_git rev-parse HEAD)"
(cd "$PUSH_DIR/work" && as hatai commit -q --allow-empty -m bad-cdpath)
bad_sha="$(work_git rev-parse --short HEAD)"
OUT="$(cd "$PUSH_DIR/work/sub/dir" && CDPATH=. TMPDIR=rel-tmp PATH="$RGBIN:$PATH" bash "$SCRIPT" dev git push -u origin HEAD 2>&1)"
RC=$?
[[ "$RC" != 0 ]] || failures+=("cdpath_rel_tmpdir_block: exit 0, want non-zero")
[[ "$OUT" == *"$bad_sha"* && "$OUT" == *"push を止めました"* ]] || failures+=("cdpath_rel_tmpdir_block: not stopped by the hook: $OUT")
[[ -z "$(remote_ref agent/cdpath)" ]] || failures+=("cdpath_rel_tmpdir_block: remote ref created")
work_git reset -q --hard "$base"
(cd "$PUSH_DIR/work" && as 'pelway-dev[bot]' commit -q --allow-empty -m ok-cdpath)
OUT="$(cd "$PUSH_DIR/work/sub/dir" && CDPATH=. TMPDIR=rel-tmp PELWAY_AGENT_SKIP_RESIGN=1 PATH="$RGBIN:$PATH" bash "$SCRIPT" dev git push -u origin HEAD 2>&1)"
RC=$?
[[ "$RC" == 0 ]] || failures+=("cdpath_rel_tmpdir_ok: exit $RC, want 0: $OUT")
[[ "$(remote_ref agent/cdpath)" == "$(work_git rev-parse HEAD)" ]] || failures+=("cdpath_rel_tmpdir_ok: remote ref not updated")
# 標準の呼び方 `scripts/agent-gh` では $0 が相対のため、pr-size ゲートのパス（`$(cd "$(dirname "$0")" && pwd)`）にも同じ行が混ざり、
# 一時フックがゲートを起動できずに正しい名義の push が止まる。`./` で始まらない相対の $0（シンボリックリンク bin 経由）と CDPATH=. でも通る。
ln -s "$(dirname "$SCRIPT")" "$PUSH_DIR/work/sub/dir/bin"
(cd "$PUSH_DIR/work" && as 'pelway-dev[bot]' commit -q --allow-empty -m ok-cdpath-rel0)
OUT="$(cd "$PUSH_DIR/work/sub/dir" && CDPATH=. PELWAY_AGENT_SKIP_RESIGN=1 PATH="$RGBIN:$PATH" bash bin/agent-gh dev git push origin HEAD 2>&1)"
RC=$?
[[ "$RC" == 0 ]] || failures+=("cdpath_rel0_gate_ok: exit $RC, want 0: $OUT")
[[ "$(remote_ref agent/cdpath)" == "$(work_git rev-parse HEAD)" ]] || failures+=("cdpath_rel0_gate_ok: remote ref not updated")
rm -rf "$PUSH_DIR/work/sub"

# Git が一時フックを経て起動する既存の pre-push には、呼び出し元の CDPATH が届く（#706）。agent-gh が CDPATH を環境から消すと、共有の検査
# ディレクトリの親を CDPATH に置き、その子ディレクトリへ cd してから検査する既存のフックが失敗し、正しい名義の push も止まる。フックは
# CDPATH でしか見つからない子ディレクトリ（work の直下には無い）へ cd し、着いた先を書く。呼ばれずに通った push を緑にしないため、着いた先も確かめる。
setup_push_repo cdpath_hook
mkdir -p "$PUSH_DIR/shared/precheck" "$PUSH_DIR/hooks"
printf '#!/usr/bin/env bash\nset -euo pipefail\ncd precheck >/dev/null\npwd -P >"$CDPATH_HOOK_MARK"\n' >"$PUSH_DIR/hooks/pre-push"
chmod +x "$PUSH_DIR/hooks/pre-push"
work_git config core.hooksPath "$PUSH_DIR/hooks"
(cd "$PUSH_DIR/work" && as 'pelway-dev[bot]' commit -q --allow-empty -m ok-cdpath-hook)
OUT="$(cd "$PUSH_DIR/work" && CDPATH="$PUSH_DIR/shared" CDPATH_HOOK_MARK="$PUSH_DIR/hook-mark" PELWAY_AGENT_SKIP_RESIGN=1 PATH="$RGBIN:$PATH" bash "$SCRIPT" dev git push -u origin HEAD 2>&1)"
RC=$?
[[ "$RC" == 0 ]] || failures+=("cdpath_orig_hook: exit $RC, want 0: $OUT")
[[ "$(cat "$PUSH_DIR/hook-mark" 2>/dev/null)" == "$(cd "$PUSH_DIR/shared/precheck" && pwd -P)" ]] || failures+=("cdpath_orig_hook: original pre-push did not reach the CDPATH dir")
[[ "$(remote_ref agent/cdpath_hook)" == "$(work_git rev-parse HEAD)" ]] || failures+=("cdpath_orig_hook: remote ref not updated")

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

# --- push 前の pr-size ゲート（#642）。#613 の一時 pre-push フックの中で、Git が送る更新ごとに差分を測る ---
# remote の URL は https://github.com/<owner>/Pelway.git のまま、remote.<名>.vcs の偽の remote helper で輸送する。
# helper は connect で <owner>-Pelway.git（ローカルの bare）につなぐ（github.com でない URL はそのままパスとして使う）。
# フックの第 2 引数（Git が接続する URL）は github.com の URL のまま残る
cat >"$RGBIN/git-remote-gatetest" <<'EOF'
#!/usr/bin/env bash
dir="$2"
[[ "$2" =~ ^https://github\.com/([^/]+)/([^/]+)\.git$ ]] && dir="$GATE_DIR/${BASH_REMATCH[1]}-${BASH_REMATCH[2]}.git"
while IFS= read -r line; do
  case "$line" in
    capabilities) printf 'connect\n\n' ;;
    "connect "*) printf '\n'; exec git "${line#connect git-}" "$dir" ;;
    *) exit 1 ;;
  esac
done
EOF
chmod +x "$RGBIN/git-remote-gatetest"
export GATE_DIR="$ROOT/gate"
for r in hatai someone; do "$REAL_GIT" init -q --bare -b main "$GATE_DIR/$r-Pelway.git"; done
"$REAL_GIT" init -q -b main "$GATE_DIR/work"
gate_git() { "$REAL_GIT" -C "$GATE_DIR/work" "$@"; }
for r in origin:hatai fork:someone; do
  gate_git remote add "${r%%:*}" "https://github.com/${r#*:}/Pelway.git"
  gate_git config "remote.${r%%:*}.vcs" gatetest
done
(cd "$GATE_DIR/work" && as hatai commit -q --allow-empty -m base)
gate_git update-ref refs/remotes/origin/main HEAD
# gate_branch <名前> <起点> <行数>: 起点に <行数> 行のファイルを足すコミット（pelway-dev[bot] の名義）でブランチを作る
gate_branch() {
  gate_git switch -q -c "$1" "$2"
  seq "$3" >"$GATE_DIR/work/${1//\//-}.txt"
  gate_git add -A
  (cd "$GATE_DIR/work" && as 'pelway-dev[bot]' commit -q -m "$1")
}
gate_branch agent/642-x main 810 # 上限超え（下の PR の応答の headRefName）
gate_branch feature main 900
gate_branch small main 10
gate_branch small-big small 900 # small の先に +900（push の途中でブランチが動く試験の動かし先。small から fast-forward）
# gate_run <name> <agent-gh dev git の引数...>: 標準出力を OUT、標準エラーを ERR に分けて入れる（診断は ERR で照合する）
gate_run() {
  local name="$1"
  shift
  export CALL_LOG="$ROOT/$name.log"
  rm -f "$CALL_LOG.gate"
  OUT="$(cd "$GATE_DIR/work" && PATH="$RGBIN:$PATH" bash "$SCRIPT" dev git "$@" 2>"$ROOT/$name.err")"
  RC=$?
  ERR="$(cat "$ROOT/$name.err")"
}
# gate_ref <ref> [<owner>-Pelway]: bare の ref の sha（無ければ空）。gate_clear は通した push が bare に残した ref を消す
gate_ref() { "$REAL_GIT" --git-dir="$GATE_DIR/${2:-hatai-Pelway}.git" rev-parse -q --verify "$1"; }
gate_clear() { "$REAL_GIT" --git-dir="$GATE_DIR/hatai-Pelway.git" update-ref -d "$1"; }
# gate_blocked <name> <ref> [<owner>-Pelway] [<行数>]: ゲートが止め（0 以外・標準エラーに行数）、remote の ref が無いままか
gate_blocked() {
  [[ "$RC" != 0 && "$ERR" == *"${4:-+810 / -0}"* && "$ERR" == *'上限を超えています'* ]] || failures+=("$1: not stopped by the gate: rc=$RC $ERR")
  [[ -z "$(gate_ref "$2" "${3:-}")" ]] || failures+=("$1: remote $2 changed")
}

# 1. 上限超えは push しない。標準エラーに行数・base・例外の条件が出て、remote の ref は変わらない。
#    Git が送る更新を測るため、引数の形（否定形・試行・-q・refspec の省略・リモートだけ・-u）によらず止まる
gate_git switch -q agent/642-x
gate_git config branch.agent/642-x.remote origin
gate_git config branch.agent/642-x.merge refs/heads/agent/642-x
while IFS= read -r form; do
  read -ra form_args <<<"$form"
  gate_run gate_over "${form_args[@]}"
  gate_blocked "gate_over ($form)" refs/heads/agent/642-x
done <<'EOF'
push --dry-run origin HEAD
push --no-dry-run --no-porcelain origin HEAD
push -q origin HEAD
push
push origin
push -u origin agent/642-x
push origin HEAD
EOF
for want in 'base: origin/main' 'pr-size-exception' 'hatai' '<!-- pr-size-exception -->'; do
  [[ "$ERR" == *"$want"* ]] || failures+=("gate_over: stderr lacks '$want': $ERR")
done
PELWAY_AGENT_SKIP_RESIGN=1 gate_run gate_skip_resign push origin HEAD
gate_blocked gate_skip_resign refs/heads/agent/642-x
# グローバルオプション（-C・-c）付きの push は、ゲートの前に agent-gh が終了コード 2 で拒否し、remote は変わらない
for global in "-C $GATE_DIR/work" "-c push.default=current"; do
  read -ra global_args <<<"$global"
  gate_run gate_global "${global_args[@]}" push origin HEAD
  [[ "$RC" == 2 && -z "$(gate_ref refs/heads/agent/642-x)" ]] ||
    failures+=("gate_global (${global%% *}): exit $RC (want 2), remote $(gate_ref refs/heads/agent/642-x): $ERR")
done
# refs/replace で上限超えのコミットを小さいコミット（small）に見せ替えても、Git が送る本体（+810）を測って止める
gate_git replace "$(gate_git rev-parse agent/642-x)" "$(gate_git rev-parse small)"
gate_run gate_replace push origin HEAD
gate_blocked gate_replace refs/heads/agent/642-x
gate_git replace -d "$(gate_git rev-parse agent/642-x)" >/dev/null

# 2. 例外があれば通す: 接続先のリポジトリの開いた PR の pr-size-exception ラベル + PR が閉じる Issue の hatai の目印コメント。
#    Issue はリポジトリ付きの参照（url）で取り、別リポジトリの Issue もその参照で取る
PR_LABELED='[{"number":42,"labels":[{"name":"pr-size-exception"}],"headRefName":"agent/642-x","headRepository":{"nameWithOwner":"hatai/Pelway"},"closingIssuesReferences":[{"number":640,"url":"https://github.com/hatai/Pelway/issues/640"}]}]'
PR_UNLABELED='[{"number":42,"labels":[],"headRefName":"agent/642-x","headRepository":{"nameWithOwner":"hatai/Pelway"},"closingIssuesReferences":[{"number":640,"url":"https://github.com/hatai/Pelway/issues/640"}]}]'
PR_FORK='[{"number":42,"labels":[{"name":"pr-size-exception"}],"headRefName":"agent/642-x","headRepository":{"nameWithOwner":"someone/Pelway"},"closingIssuesReferences":[{"number":640,"url":"https://github.com/hatai/Pelway/issues/640"}]}]'
PR_CROSS_ISSUE='[{"number":42,"labels":[{"name":"pr-size-exception"}],"headRefName":"agent/642-x","headRepository":{"nameWithOwner":"hatai/Pelway"},"closingIssuesReferences":[{"number":640,"url":"https://github.com/other/repo/issues/640"}]}]'
ISSUE_MARKER='{"comments":[{"author":{"login":"hatai"},"body":"<!-- pr-size-exception -->\nこの PR の超過を認める"}]}'
for c in "$PR_LABELED|hatai/Pelway" "$PR_CROSS_ISSUE|other/repo"; do
  FAKE_GH_PR_LIST="${c%|*}" FAKE_GH_ISSUE_VIEW="$ISSUE_MARKER" PELWAY_AGENT_SKIP_RESIGN=1 gate_run gate_exception push origin HEAD
  [[ "$RC" == 0 && "$(gate_ref refs/heads/agent/642-x)" == "$(gate_git rev-parse agent/642-x)" ]] ||
    failures+=("gate_exception (${c#*|}): not pushed: rc=$RC $ERR")
  grep -qF "ARGS=pr list --repo hatai/Pelway --head agent/642-x" "$ROOT/gate_exception.log.gate" ||
    failures+=("gate_exception (${c#*|}): PR not queried in the destination repo")
  grep -qF "ARGS=issue view https://github.com/${c#*|}/issues/640" "$ROOT/gate_exception.log.gate" ||
    failures+=("gate_exception (${c#*|}): issue not fetched by its repo-qualified url")
  gate_clear refs/heads/agent/642-x
done

# 3. 例外の条件が欠けると止める: ラベルだけ・コメントだけ・作成者が hatai でない・目印が行頭でない・head がフォークの同名ブランチ・
#    開いた PR が無い（Issue は照会しない）・PR の照会の失敗・PR の照会は通り Issue のコメントの照会だけが失敗
gate_exception_blocked() {
  FAKE_GH_PR_LIST="$2" FAKE_GH_ISSUE_VIEW="$3" FAKE_GH_FAIL_PR="${4:-}" FAKE_GH_FAIL_ISSUE="${5:-}" gate_run "$1" push origin HEAD
  gate_blocked "$1" refs/heads/agent/642-x
}
gate_exception_blocked gate_label_only "$PR_LABELED" '{"comments":[{"author":{"login":"hatai"},"body":"この PR の超過を認める"}]}'
gate_exception_blocked gate_comment_only "$PR_UNLABELED" "$ISSUE_MARKER"
gate_exception_blocked gate_not_hatai "$PR_LABELED" '{"comments":[{"author":{"login":"someone"},"body":"<!-- pr-size-exception -->\n認める"}]}'
gate_exception_blocked gate_marker_not_at_start "$PR_LABELED" '{"comments":[{"author":{"login":"hatai"},"body":"x <!-- pr-size-exception -->"}]}'
gate_exception_blocked gate_fork_pr "$PR_FORK" "$ISSUE_MARKER"
gate_exception_blocked gate_no_pr '[]' "$ISSUE_MARKER"
! grep -qF 'issue view' "$ROOT/gate_no_pr.log.gate" || failures+=("gate_no_pr: issue view was called")
gate_exception_blocked gate_pr_query_fail "$PR_LABELED" "$ISSUE_MARKER" 1
[[ "$ERR" == *'問い合わせに失敗'* ]] || failures+=("gate_pr_query_fail: stderr lacks the query failure: $ERR")
gate_exception_blocked gate_issue_query_fail "$PR_LABELED" "$ISSUE_MARKER" "" 1
[[ "$ERR" == *'問い合わせに失敗'* ]] || failures+=("gate_issue_query_fail: stderr lacks the query failure: $ERR")
grep -qF 'ARGS=issue view' "$ROOT/gate_issue_query_fail.log.gate" || failures+=("gate_issue_query_fail: issue was not queried")

# 4. 例外は Git が実際に接続する先のリポジトリで確かめる。位置引数のリモート（origin）は --repo（fork）より優先され、
#    origin = hatai/Pelway の承認で通る。branch.<名>.pushRemote が向ける fork・複数の pushurl の someone/Pelway・
#    insteadOf で github.com 以外に書き換わった接続先では、hatai/Pelway の承認を使わずに止める
FAKE_GH_PR_LIST="$PR_LABELED" FAKE_GH_ISSUE_VIEW="$ISSUE_MARKER" PELWAY_AGENT_SKIP_RESIGN=1 gate_run gate_repo_opt push --repo=fork origin HEAD
[[ "$RC" == 0 && -n "$(gate_ref refs/heads/agent/642-x)" && -z "$(gate_ref refs/heads/agent/642-x someone-Pelway)" ]] ||
  failures+=("gate_repo_opt: not pushed to origin: rc=$RC $ERR")
gate_clear refs/heads/agent/642-x
gate_git config branch.agent/642-x.pushRemote fork
FAKE_GH_PR_LIST="$PR_LABELED" FAKE_GH_ISSUE_VIEW="$ISSUE_MARKER" gate_run gate_push_remote push
gate_blocked gate_push_remote refs/heads/agent/642-x someone-Pelway
grep -qF 'ARGS=pr list --repo someone/Pelway ' "$ROOT/gate_push_remote.log.gate" || failures+=("gate_push_remote: PR not queried in fork")
gate_git config --unset branch.agent/642-x.pushRemote
gate_git remote add both https://github.com/hatai/Pelway.git
gate_git config remote.both.vcs gatetest
gate_git config --add remote.both.pushurl https://github.com/hatai/Pelway.git
gate_git config --add remote.both.pushurl https://github.com/someone/Pelway.git
FAKE_GH_PR_LIST="$PR_LABELED" FAKE_GH_ISSUE_VIEW="$ISSUE_MARKER" PELWAY_AGENT_SKIP_RESIGN=1 gate_run gate_multi_url push both HEAD
gate_blocked gate_multi_url refs/heads/agent/642-x someone-Pelway
[[ -n "$(gate_ref refs/heads/agent/642-x)" ]] || failures+=("gate_multi_url: not pushed to the approved hatai/Pelway: $ERR")
gate_clear refs/heads/agent/642-x
gate_git config "url.$GATE_DIR/hatai-Pelway.git.insteadOf" https://github.com/hatai/Pelway.git
FAKE_GH_PR_LIST="$PR_LABELED" FAKE_GH_ISSUE_VIEW="$ISSUE_MARKER" gate_run gate_instead_of push origin HEAD
gate_blocked gate_instead_of refs/heads/agent/642-x
[[ "$ERR" == *'GitHub のリポジトリではない'* && ! -e "$ROOT/gate_instead_of.log.gate" ]] ||
  failures+=("gate_instead_of: exception was looked up for a non-GitHub destination: $ERR")
gate_git config --unset "url.$GATE_DIR/hatai-Pelway.git.insteadOf"

# 5. 上限以下は GitHub に照会せずに push する。-u は元のブランチに upstream を保存する
gate_git switch -q small
PELWAY_AGENT_SKIP_RESIGN=1 gate_run gate_under push -u origin HEAD
[[ "$RC" == 0 && "$(gate_ref refs/heads/small)" == "$(gate_git rev-parse small)" ]] || failures+=("gate_under: not pushed: rc=$RC $ERR")
[[ "$(gate_git config branch.small.remote) $(gate_git config branch.small.merge)" == "origin refs/heads/small" ]] ||
  failures+=("gate_under: upstream not saved")
[[ ! -e "$ROOT/gate_under.log.gate" ]] || failures+=("gate_under: GitHub was queried")
gate_clear refs/heads/small

# 6. 同名のタグとブランチを一緒に送る push と、remote.<名>.push で宛先が変わる push は、Git が解決した宛先のまま通る。
#    タグは測らない（タグ rel の先は上限超えの feature）。削除も測らない
gate_git tag rel feature
gate_git branch rel small
PELWAY_AGENT_SKIP_RESIGN=1 gate_run gate_tag_branch push origin tag rel refs/heads/rel
[[ "$RC" == 0 && "$(gate_ref refs/tags/rel)" == "$(gate_git rev-parse feature)" && "$(gate_ref refs/heads/rel)" == "$(gate_git rev-parse small)" ]] ||
  failures+=("gate_tag_branch: rc=$RC $ERR")
PELWAY_AGENT_SKIP_RESIGN=1 gate_run gate_delete push origin :refs/heads/rel :refs/tags/rel
[[ "$RC" == 0 && -z "$(gate_ref refs/heads/rel)" && -z "$(gate_ref refs/tags/rel)" ]] || failures+=("gate_delete: rc=$RC $ERR")
gate_git config remote.origin.push refs/heads/small:refs/heads/mapped
PELWAY_AGENT_SKIP_RESIGN=1 gate_run gate_push_config push origin small
[[ "$RC" == 0 && "$(gate_ref refs/heads/mapped)" == "$(gate_git rev-parse small)" && -z "$(gate_ref refs/heads/small)" ]] ||
  failures+=("gate_push_config: rc=$RC $ERR")
gate_clear refs/heads/mapped
gate_git config --unset remote.origin.push

# 7. src:dst は送る src の差分を数える（HEAD の small は上限内でも、送る feature は +900）
gate_run gate_src push origin feature:refs/heads/agent/642-new
gate_blocked gate_src refs/heads/agent/642-new "" '+900 / -0'

# 8. base は push 元のブランチの branch.<名>.gh-merge-base があれば origin/<その値>（feature と同じ先端でも、設定で差分が 0 になる）。
#    設定が無いときの origin/main は 1 で確かめた
gate_git branch mb-set feature
gate_git update-ref refs/remotes/origin/topic feature
gate_git config branch.mb-set.gh-merge-base topic
PELWAY_AGENT_SKIP_RESIGN=1 gate_run gate_merge_base push origin mb-set
[[ "$RC" == 0 && "$(gate_ref refs/heads/mb-set)" == "$(gate_git rev-parse feature)" ]] || failures+=("gate_merge_base: rc=$RC $ERR")
gate_clear refs/heads/mb-set

# 9. base と送る sha に共通祖先が無く差分を測れないときも止める（0 行で通さない）
gate_git switch -q --orphan orphan-hist
(cd "$GATE_DIR/work" && as hatai commit -q --allow-empty -m unrelated)
gate_git update-ref refs/remotes/origin/orphan HEAD
gate_git switch -q small
gate_git config branch.small.gh-merge-base orphan
gate_run gate_unmeasurable push origin HEAD
[[ "$RC" != 0 && "$ERR" == *'測れません'* && -z "$(gate_ref refs/heads/small)" ]] || failures+=("gate_unmeasurable: rc=$RC $ERR")
gate_git config --unset branch.small.gh-merge-base

# 10. 測る sha は Git がフックの前に決めた送る sha そのもの。先に走る設定のフックが push 元のブランチを上限超えの先（small-big）へ
#     動かしても、測った（動かす前の）sha が送られる
small_sha="$(gate_git rev-parse small)"
gate_git config hook.mover.event pre-push
gate_git config hook.mover.command "sh -c 'git update-ref refs/heads/small $(gate_git rev-parse small-big)'"
PELWAY_AGENT_SKIP_RESIGN=1 gate_run gate_measured_sha push origin small
[[ "$RC" == 0 && "$(gate_ref refs/heads/small)" == "$small_sha" ]] || failures+=("gate_measured_sha: rc=$RC, remote $(gate_ref refs/heads/small), want $small_sha: $ERR")
gate_git config --remove-section hook.mover
# remote と同じ sha だった（最新のまま）ブランチが後で動いても、次の push は動いた先（+910）を送る更新として測って止める
gate_run gate_moved_after push origin small
[[ "$RC" != 0 && "$ERR" == *'+910 / -0'* && "$(gate_ref refs/heads/small)" == "$small_sha" ]] ||
  failures+=("gate_moved_after: rc=$RC, remote $(gate_ref refs/heads/small), want $small_sha: $ERR")
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
