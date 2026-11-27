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
#   issue edit <N> --body-file <F>         F の中身を Issue の本文にする。FAKE_GH_EDIT_NOOP なら何も書き換えずに成功を返す。
#                                          FAKE_GH_TRIM_NL なら末尾の改行を全部落として保存する。
#                                          FAKE_GH_CLOBBER_BODY なら読む前に ORIG_BODY のファイルを空にする
#   issue view <N> --json body --jq .body  今の本文を出す（本物の --jq と同じく末尾に改行を 1 つ足す）
#   issue comment <N> --body-file <F>      F の中身を CALL_LOG.comment に写す。FAKE_GH_CLOBBER_COMMENT なら読む前に
#                                          ORIG_COMMENT のファイルを別の文に差し替える
# FAKE_GH_FAIL_EDIT・FAKE_GH_FAIL_VIEW・FAKE_GH_FAIL_COMMENT で、その呼び出しだけを失敗させる。
# FAKE_GH_CLOBBER_* は、agent-gh が入力を検査した後で別のプロセスが入力ファイルを変えたことにする。
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
    [[ -z "${FAKE_GH_CLOBBER_BODY:-}" ]] || : >"$ORIG_BODY"
    if [[ -n "${FAKE_GH_EDIT_NOOP:-}" ]]; then
      :
    elif [[ -n "${FAKE_GH_TRIM_NL:-}" ]]; then
      printf '%s\n' "$(cat -- "$body_file")" >"$STATE.body"
    else
      cat -- "$body_file" >"$STATE.body"
    fi
    ;;
  "issue view")
    [[ -z "${FAKE_GH_FAIL_VIEW:-}" ]] || exit 1
    cat -- "$STATE.body"
    printf '\n'
    ;;
  "issue comment")
    [[ -z "${FAKE_GH_FAIL_COMMENT:-}" ]] || exit 1
    [[ -z "${FAKE_GH_CLOBBER_COMMENT:-}" ]] || printf '%s\n' '差し替えられたコメント' >"$ORIG_COMMENT"
    cat -- "$body_file" >"$CALL_LOG.comment"
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
# 終了コードを RC に入れる。標準入力には STDIN の中身を送る。CWD は実行するディレクトリ。どちらも呼び出しの
# 前に `STDIN='...' run ...` の形で渡す。無ければ標準入力は空で、カレントディレクトリで実行する。
run() {
  local name="$1"
  shift
  export CALL_LOG="$ROOT/$name.log"
  rm -f "$CALL_LOG" "$CALL_LOG.token" "$CALL_LOG.comment"
  printf '古い本文\n' >"$STATE.body"
  OUT="$(cd "${CWD:-.}" && printf '%s' "${STDIN:-}" | bash "$SCRIPT" "$@" 2>&1)"
  RC=$?
}

# calls <name>: 偽 gh が受けた呼び出しの引数（1 行 1 呼び出し）。mktemp が作った一時ファイルのパスは <copy> に置き換える
calls() {
  sed -n 's/^ARGS=//p' "$ROOT/$1.log" 2>/dev/null | sed -E 's#--body-file [^ ]*/tmp\.[A-Za-z0-9]{10}$#--body-file <copy>#'
}

# sent_file <name> <呼び出しの先頭>: 偽 gh がその呼び出しの --body-file に受け取ったパス
sent_file() { sed -n "s/^ARGS=$2 --body-file //p" "$ROOT/$1.log" 2>/dev/null; }

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
EDIT="issue edit 42 --body-file <copy>"
VIEW="issue view 42 --json body --jq .body"
COMMENT_CALL="issue comment 42 --body-file <copy>"

# 受け入れ条件 1・4: 本文を書き換え、読み直してから、コメントを付ける（この順で呼ぶ）。本文とコメントは
# ファイルの中身のまま渡り、その文字列は argv に載らない。gh に渡すのは入力の写し（一時ファイル）で、
# 写しは agent-gh の終了後に残らない。
run ok dev issue rewrite 42 --body-file "$BODY" --comment-file "$COMMENT"
[[ "$RC" == 0 ]] || failures+=("ok: exit $RC, want 0 ($OUT)")
cmp -s "$STATE.body" "$BODY" || failures+=("ok: issue body differs from the body file")
cmp -s "$ROOT/ok.log.comment" "$COMMENT" || failures+=("ok: comment differs from the comment file")
[[ "$(calls ok)" == "$EDIT"$'\n'"$VIEW"$'\n'"$COMMENT_CALL" ]] || failures+=("ok: calls are '$(calls ok)'")
! grep -qF '書き直した本文' "$ROOT/ok.log" 2>/dev/null || failures+=("ok: body text on argv")
! grep -qF '確かめ方を足した' "$ROOT/ok.log" 2>/dev/null || failures+=("ok: comment text on argv")
copy="$(sent_file ok 'issue edit 42')"
[[ -n "$copy" && "$copy" != "$BODY" && ! -e "$copy" ]] || failures+=("ok: body not sent from a removed copy ($copy)")
copy="$(sent_file ok 'issue comment 42')"
[[ -n "$copy" && "$copy" != "$COMMENT" && ! -e "$copy" ]] || failures+=("ok: comment not sent from a removed copy ($copy)")

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

# 受け入れ条件 3: 本文自身の末尾の改行も比べる（gh の --jq が足す末尾の改行 1 つだけを除く）。読み直した本文の
# 末尾の改行の数が渡した本文と違えばコメントを付けず、同じなら付ける。
printf '末尾に空行のある本文\n\n\n' >"$ROOT/blank.md"
FAKE_GH_TRIM_NL=1 run trailing_newlines_differ dev issue rewrite 42 --body-file "$ROOT/blank.md" --comment-file "$COMMENT"
[[ "$RC" == 6 ]] || failures+=("trailing_newlines_differ: exit $RC, want 6")
[[ "$(calls trailing_newlines_differ)" == "$EDIT"$'\n'"$VIEW" ]] || failures+=("trailing_newlines_differ: calls are '$(calls trailing_newlines_differ)'")
run trailing_newlines_same dev issue rewrite 42 --body-file "$ROOT/blank.md" --comment-file "$COMMENT"
[[ "$RC" == 0 ]] || failures+=("trailing_newlines_same: exit $RC, want 0 ($OUT)")
cmp -s "$STATE.body" "$ROOT/blank.md" || failures+=("trailing_newlines_same: issue body lost its trailing newlines")
[[ "$(calls trailing_newlines_same)" == "$EDIT"$'\n'"$VIEW"$'\n'"$COMMENT_CALL" ]] ||
  failures+=("trailing_newlines_same: calls are '$(calls trailing_newlines_same)'")

# 検査の後に入力ファイルが変わっても、検査した写しを送る。偽 gh に、呼び出しの時点で入力の本文を空にさせる
# （空の本文が送られると Issue の本文が消える）。
cp "$BODY" "$ROOT/clobber-body.md"
FAKE_GH_CLOBBER_BODY=1 ORIG_BODY="$ROOT/clobber-body.md" run clobber_body dev issue rewrite 42 --body-file "$ROOT/clobber-body.md" --comment-file "$COMMENT"
[[ "$RC" == 0 ]] || failures+=("clobber_body: exit $RC, want 0 ($OUT)")
cmp -s "$STATE.body" "$BODY" || failures+=("clobber_body: issue body is not the checked content")
cmp -s "$ROOT/clobber_body.log.comment" "$COMMENT" || failures+=("clobber_body: comment differs")
[[ ! -s "$ROOT/clobber-body.md" ]] || failures+=("clobber_body: the input file was not emptied")
cp "$COMMENT" "$ROOT/clobber-comment.md"
FAKE_GH_CLOBBER_COMMENT=1 ORIG_COMMENT="$ROOT/clobber-comment.md" run clobber_comment dev issue rewrite 42 --body-file "$BODY" --comment-file "$ROOT/clobber-comment.md"
[[ "$RC" == 0 ]] || failures+=("clobber_comment: exit $RC, want 0 ($OUT)")
cmp -s "$ROOT/clobber_comment.log.comment" "$COMMENT" || failures+=("clobber_comment: posted comment is not the checked content")
grep -qF '差し替えられたコメント' "$ROOT/clobber-comment.md" || failures+=("clobber_comment: the input file was not replaced")

# `-` で始まる相対パスを cat のオプションとして読まない。空のファイル `-n` は、標準入力に中身があっても
# 空の入力として拒む。中身のある `-body.md` はその中身を送る。
HYPHEN="$ROOT/hyphen"
mkdir -p "$HYPHEN"
: >"$HYPHEN/-n"
printf 'ハイフンで始まる名前の本文\n' >"$HYPHEN/-body.md"
CWD="$HYPHEN" STDIN=$'標準入力の本文\n' reject rejects_hyphen_empty dev issue rewrite 42 --body-file -n --comment-file "$COMMENT"
CWD="$HYPHEN" run hyphen_body dev issue rewrite 42 --body-file -body.md --comment-file "$COMMENT"
[[ "$RC" == 0 ]] || failures+=("hyphen_body: exit $RC, want 0 ($OUT)")
cmp -s "$STATE.body" "$HYPHEN/-body.md" || failures+=("hyphen_body: issue body differs from the -body.md file")
cmp -s "$ROOT/hyphen_body.log.comment" "$COMMENT" || failures+=("hyphen_body: comment differs")

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
