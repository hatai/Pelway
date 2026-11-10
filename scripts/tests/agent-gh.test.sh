#!/usr/bin/env bash
# scripts/agent-gh のテスト。偽の fnox・gh・git を PATH の先頭に置き、渡されたトークンと引数を確かめる。
set -uo pipefail

SCRIPT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/agent-gh"
ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT
failures=()

BIN="$ROOT/bin"
mkdir -p "$BIN"

# 偽の fnox: `fnox get NAME` に FAKE_<NAME> の値を返す。値が空なら失敗する。
cat >"$BIN/fnox" <<'EOF'
#!/usr/bin/env bash
[[ "$1" == "get" ]] || exit 9
var="FAKE_$2"
[[ -n "${!var:-}" ]] || exit 1
printf '%s\n' "${!var}"
EOF

# 偽の gh: `gh token generate` は JSON を返す。それ以外は GH_TOKEN と引数を記録する。
cat >"$BIN/gh" <<'EOF'
#!/usr/bin/env bash
if [[ "$1 $2" == "token generate" ]]; then
  printf '%s\n' "$*" >"$CALL_LOG.token-args"
  printf '{"token":"review-token","expires_at":"x"}\n'
  exit 0
fi
{ printf 'GH_TOKEN=%s\n' "${GH_TOKEN:-}"; printf 'ARGS=%s\n' "$*"; } >"$CALL_LOG"
EOF

# 偽の git: GH_TOKEN と引数を記録する。
cat >"$BIN/git" <<'EOF'
#!/usr/bin/env bash
{ printf 'GH_TOKEN=%s\n' "${GH_TOKEN:-}"; printf 'ARGS=%s\n' "$*"; } >"$CALL_LOG"
EOF
chmod +x "$BIN"/*

export PATH="$BIN:$PATH"
export FAKE_PELWAY_AGENT_GH_TOKEN=dev-token
export FAKE_PELWAY_AGENT_GIT_EMAIL=1+pelway-agent@users.noreply.github.com
export FAKE_PELWAY_REVIEW_APP_ID=111
export FAKE_PELWAY_REVIEW_APP_INSTALLATION_ID=222
export FAKE_PELWAY_REVIEW_APP_KEY=pem-body

# run <name> <args...>: CALL_LOG を新しくして agent-gh を実行し、出力を OUT、終了コードを RC に入れる。
run() {
  local name="$1"
  shift
  export CALL_LOG="$ROOT/$name.log"
  rm -f "$CALL_LOG"
  OUT="$(bash "$SCRIPT" "$@" 2>&1)"
  RC=$?
}

expect_log() {
  local name="$1" pattern="$2"
  grep -qF -- "$pattern" "$ROOT/$name.log" 2>/dev/null || failures+=("$name: log lacks '$pattern'")
}

run dev_sets_token dev pr list
expect_log dev_sets_token "GH_TOKEN=dev-token"
expect_log dev_sets_token "ARGS=pr list"

run token_not_leaked dev pr list
[[ "$OUT" != *dev-token* ]] || failures+=("token_not_leaked: token printed")

run review_mints_token review pr review 1 --approve
expect_log review_mints_token "GH_TOKEN=review-token"
expect_log review_mints_token "ARGS=pr review 1 --approve"
grep -qF -- "--app-id 111 --installation-id 222" "$ROOT/review_mints_token.log.token-args" 2>/dev/null ||
  failures+=("review_mints_token: token generate args wrong")
[[ "$OUT" != *review-token* && "$OUT" != *pem-body* ]] || failures+=("review_mints_token: secret printed")

run dev_git_identity dev git commit -m x
expect_log dev_git_identity "GH_TOKEN=dev-token"
expect_log dev_git_identity "-c user.name=pelway-agent"
expect_log dev_git_identity "-c user.email=1+pelway-agent@users.noreply.github.com"
expect_log dev_git_identity "-c credential.helper= -c credential.helper=!gh auth git-credential"
expect_log dev_git_identity "commit -m x"

run bad_role admin pr list
[[ "$RC" == 2 ]] || failures+=("bad_role: exit $RC, want 2")

FAKE_PELWAY_AGENT_GH_TOKEN="" run missing_token_fails_closed dev pr list
[[ "$RC" == 3 ]] || failures+=("missing_token_fails_closed: exit $RC, want 3")
[[ ! -e "$ROOT/missing_token_fails_closed.log" ]] || failures+=("missing_token_fails_closed: gh was called")

FAKE_PELWAY_AGENT_GIT_EMAIL="" run missing_email_fails_closed dev git push
[[ "$RC" == 3 ]] || failures+=("missing_email_fails_closed: exit $RC, want 3")
[[ ! -e "$ROOT/missing_email_fails_closed.log" ]] || failures+=("missing_email_fails_closed: git was called")

if ((${#failures[@]})); then
  printf 'FAIL %s\n' "${failures[@]}"
  exit 1
fi
echo ok
