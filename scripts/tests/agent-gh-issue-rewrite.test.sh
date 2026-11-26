#!/usr/bin/env bash
# scripts/agent-gh の `dev issue rewrite <番号> --body-file <path|-> --comment-file <path|->`（#810）のテスト。
# 偽の fnox・gh を PATH の先頭に置く（scripts/tests/agent-gh.test.sh と同じ作り）。偽 gh は Issue の本文を
# 1 件分だけ覚えて GitHub の代わりをし、受けた引数を CALL_LOG に 1 行 1 呼び出しで追記する。
set -uo pipefail

SCRIPT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/agent-gh"
ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT
failures=()

BIN="$ROOT/bin"
mkdir -p "$BIN"

# 偽の fnox: `fnox -c <config> get NAME` に FAKE_<NAME> の値を返す。
cat >"$BIN/fnox" <<'EOF'
#!/usr/bin/env bash
var="FAKE_$4"
[[ "$1" == -c && "$3" == get && -n "${!var:-}" ]] || exit 1
printf '%s\n' "${!var}"
EOF

# 偽の gh。`gh token generate` はトークンを返す。それ以外は引数を CALL_LOG に追記して、次の 3 つに応じる。
#   issue edit <N> --body-file <F>         F の中身を Issue の本文にする。FAKE_GH_EDIT_NOOP なら何も書き換えずに成功を返す
#   issue view <N> --json body --jq .body  今の本文を出す（本物の --jq と同じく末尾に改行を 1 つ足す）
#   issue comment <N> --body-file <F>      F の中身を CALL_LOG.comment に写す
# FAKE_GH_FAIL_EDIT・FAKE_GH_FAIL_VIEW・FAKE_GH_FAIL_COMMENT で、その呼び出しだけを失敗させる。
# ほかの呼び出し（実装前に gh へそのまま渡される `issue rewrite` など）は引数を記録して終了コード 0。
cat >"$BIN/gh" <<'EOF'
#!/usr/bin/env bash
if [[ "$1 $2" == "token generate" ]]; then
  : >"$CALL_LOG.token"
  printf '{"token":"dev-token","expires_at":"x"}\n'
  exit 0
fi
printf 'ARGS=%s\n' "$*" >>"$CALL_LOG"
prev=""
for a in "$@"; do
  [[ "$prev" != --body-file ]] || body_file="$a"
  prev="$a"
done
case "${1:-} ${2:-}" in
  "issue edit")
    [[ -z "${FAKE_GH_FAIL_EDIT:-}" ]] || exit 1
    [[ -n "${FAKE_GH_EDIT_NOOP:-}" ]] || cat "$body_file" >"$STATE.body"
    ;;
  "issue view")
    [[ -z "${FAKE_GH_FAIL_VIEW:-}" ]] || exit 1
    cat "$STATE.body"
    printf '\n'
    ;;
  "issue comment")
    [[ -z "${FAKE_GH_FAIL_COMMENT:-}" ]] || exit 1
    cat "$body_file" >"$CALL_LOG.comment"
    ;;
esac
exit 0
EOF
chmod +x "$BIN"/*

export PATH="$BIN:$PATH"
export PELWAY_AGENT_FNOX_CONFIG="$ROOT/agent-fnox.toml"
export STATE="$ROOT/state"
export FAKE_PELWAY_DEV_APP_ID=333 FAKE_PELWAY_DEV_APP_INSTALLATION_ID=444 FAKE_PELWAY_DEV_APP_KEY=dev-pem-body
export FAKE_PELWAY_DEV_GIT_EMAIL="9+pelway-dev[bot]@users.noreply.github.com"

# run <name> <args...>: CALL_LOG と偽 GitHub の本文（古い本文）を新しくして agent-gh を実行し、出力を OUT、
# 終了コードを RC に入れる。標準入力には STDIN の中身を送る（呼び出しの前に `STDIN='...' run ...` で渡す。
# 無ければ空）。
run() {
  local name="$1"
  shift
  export CALL_LOG="$ROOT/$name.log"
  rm -f "$CALL_LOG" "$CALL_LOG.token" "$CALL_LOG.comment"
  printf '古い本文\n' >"$STATE.body"
  OUT="$(printf '%s' "${STDIN:-}" | bash "$SCRIPT" "$@" 2>&1)"
  RC=$?
}

# calls <name>: 偽 gh が受けた呼び出しの引数（1 行 1 呼び出し）
calls() { sed -n 's/^ARGS=//p' "$ROOT/$1.log" 2>/dev/null; }

# reject <name> <args...>: 引数の誤りは、fnox（トークン発行）と gh のどちらも呼ぶ前に終了コード 2 で拒否する
reject() {
  local name="$1"
  shift
  run "$name" "$@"
  [[ "$RC" == 2 ]] || failures+=("$name: exit $RC, want 2 ($OUT)")
  [[ ! -e "$ROOT/$name.log" && ! -e "$ROOT/$name.log.token" ]] || failures+=("$name: fnox or gh was called")
}

BODY="$ROOT/body.md"
COMMENT="$ROOT/comment.md"
EMPTY="$ROOT/empty.md"
printf '### 目的\n\n書き直した本文（「日本語」と $dollar・"quote"・\\backslash）。末尾に改行が無い' >"$BODY"
printf '受け入れ条件 2・3 に確かめ方を足した\n' >"$COMMENT"
: >"$EMPTY"
EDIT="issue edit 42 --body-file $BODY"
VIEW="issue view 42 --json body --jq .body"
COMMENT_CALL="issue comment 42 --body-file $COMMENT"

# 受け入れ条件 1・4: 本文を書き換え、読み直してから、コメントを付ける（この順で呼ぶ）。本文とコメントは
# ファイルの中身のまま渡り、その文字列は argv に載らない。
run ok dev issue rewrite 42 --body-file "$BODY" --comment-file "$COMMENT"
[[ "$RC" == 0 ]] || failures+=("ok: exit $RC, want 0 ($OUT)")
cmp -s "$STATE.body" "$BODY" || failures+=("ok: issue body differs from the body file")
cmp -s "$ROOT/ok.log.comment" "$COMMENT" || failures+=("ok: comment differs from the comment file")
[[ "$(calls ok)" == "$EDIT"$'\n'"$VIEW"$'\n'"$COMMENT_CALL" ]] || failures+=("ok: calls are '$(calls ok)'")
! grep -qF '書き直した本文' "$ROOT/ok.log" 2>/dev/null || failures+=("ok: body text on argv")
! grep -qF '確かめ方を足した' "$ROOT/ok.log" 2>/dev/null || failures+=("ok: comment text on argv")

# 受け入れ条件 2: 書き換えが失敗したら、コメントを付けず 0 以外で終わる
FAKE_GH_FAIL_EDIT=1 run edit_fails dev issue rewrite 42 --body-file "$BODY" --comment-file "$COMMENT"
[[ "$RC" != 0 ]] || failures+=("edit_fails: exit 0")
[[ "$(calls edit_fails)" == "$EDIT" ]] || failures+=("edit_fails: calls are '$(calls edit_fails)'")
[[ "$(cat "$STATE.body")" == 古い本文 ]] || failures+=("edit_fails: issue body changed")

# 受け入れ条件 3: gh が成功を返しても本文が書き換わっていなければ、読み直した本文が渡した本文と違うので
# コメントを付けない。読み直し自身が失敗したときも付けない。
FAKE_GH_EDIT_NOOP=1 run readback_differs dev issue rewrite 42 --body-file "$BODY" --comment-file "$COMMENT"
[[ "$RC" == 6 ]] || failures+=("readback_differs: exit $RC, want 6")
[[ "$(calls readback_differs)" == "$EDIT"$'\n'"$VIEW" ]] || failures+=("readback_differs: calls are '$(calls readback_differs)'")
FAKE_GH_FAIL_VIEW=1 run view_fails dev issue rewrite 42 --body-file "$BODY" --comment-file "$COMMENT"
[[ "$RC" != 0 ]] || failures+=("view_fails: exit 0")
[[ "$(calls view_fails)" == "$EDIT"$'\n'"$VIEW" ]] || failures+=("view_fails: calls are '$(calls view_fails)'")

# コメントだけが失敗したときは 0 以外で終わる（本文は書き換わっている）
FAKE_GH_FAIL_COMMENT=1 run comment_fails dev issue rewrite 42 --body-file "$BODY" --comment-file "$COMMENT"
[[ "$RC" != 0 ]] || failures+=("comment_fails: exit 0")
cmp -s "$STATE.body" "$BODY" || failures+=("comment_fails: issue body not rewritten")

# `-` は標準入力から受ける。受けた一時ファイルは終了後に残らない。両方に `-` は渡せない。
STDIN=$'標準入力の本文\n' run stdin_body dev issue rewrite 7 --body-file - --comment-file "$COMMENT"
[[ "$RC" == 0 ]] || failures+=("stdin_body: exit $RC, want 0 ($OUT)")
[[ "$(cat "$STATE.body")" == 標準入力の本文 ]] || failures+=("stdin_body: issue body is '$(cat "$STATE.body")'")
tmp="$(sed -n 's/^ARGS=issue edit 7 --body-file //p' "$ROOT/stdin_body.log")"
[[ -n "$tmp" && ! -e "$tmp" ]] || failures+=("stdin_body: body not received in a removed temp file ($tmp)")
STDIN=$'標準入力のコメント\n' run stdin_comment dev issue rewrite 7 --body-file "$BODY" --comment-file -
[[ "$RC" == 0 ]] || failures+=("stdin_comment: exit $RC, want 0 ($OUT)")
[[ -e "$ROOT/stdin_comment.log.comment" && "$(cat "$ROOT/stdin_comment.log.comment" 2>/dev/null)" == 標準入力のコメント ]] ||
  failures+=("stdin_comment: comment differs")

# 受け入れ条件 1 の補い: 末尾の改行だけが違う本文は、読み直して同じと見なす（gh の --jq の出力は末尾に改行が付く）
printf '末尾に空行のある本文\n\n\n' >"$ROOT/blank.md"
run blank_lines dev issue rewrite 42 --body-file "$ROOT/blank.md" --comment-file "$COMMENT"
[[ "$RC" == 0 ]] || failures+=("blank_lines: exit $RC, want 0 ($OUT)")
[[ "$(calls blank_lines)" == "issue edit 42 --body-file $ROOT/blank.md"$'\n'"$VIEW"$'\n'"$COMMENT_CALL" ]] ||
  failures+=("blank_lines: calls are '$(calls blank_lines)'")

# 引数の誤りと、空の本文・コメントは、何も送る前に拒否する（空の本文で Issue の本文を消さないため）
reject rejects_no_number dev issue rewrite
reject rejects_number_only dev issue rewrite 42
reject rejects_bad_number dev issue rewrite abc --body-file "$BODY" --comment-file "$COMMENT"
reject rejects_missing_comment_flag dev issue rewrite 42 --body-file "$BODY" "$COMMENT"
reject rejects_wrong_order dev issue rewrite 42 --comment-file "$COMMENT" --body-file "$BODY"
reject rejects_extra_arg dev issue rewrite 42 --body-file "$BODY" --comment-file "$COMMENT" --silent
reject rejects_body_file_missing dev issue rewrite 42 --body-file "$ROOT/nonexistent.md" --comment-file "$COMMENT"
reject rejects_empty_body dev issue rewrite 42 --body-file "$EMPTY" --comment-file "$COMMENT"
reject rejects_empty_comment dev issue rewrite 42 --body-file "$BODY" --comment-file "$EMPTY"
STDIN='本文' reject rejects_both_stdin dev issue rewrite 42 --body-file - --comment-file -

if ((${#failures[@]})); then
  printf 'FAIL %s\n' "${failures[@]}"
  exit 1
fi
echo ok
