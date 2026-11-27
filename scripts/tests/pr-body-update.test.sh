#!/usr/bin/env bash
# scripts/pr-body-update のテスト（Issue #809）。偽のリポジトリ（本物の scripts/pr-body-update と偽の
# scripts/agent-gh）で、PR 本文の決まった 2 行（「レビューと QA」の最後に検証した sha、「対象」の HEAD）
# だけが PR の headRefOid と指定した sha に書き換わること、決まった行が無いときは何も書き換えずに
# 失敗してどの行が無いかを出すこと、日本語の本文を argv でなく --input のファイルで渡すことを確かめる。
# 印はコードの外（HTML コメント・コードブロック・インラインコード・字下げされた行の中を除く）にあるものだけを
# 数えること、sha の直後に英数字が続く語を sha にしないこと、読み取りの後に PR が変わったら書き換えないことも
# 確かめる。
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT
failures=()

TOOL="$REPO/scripts/pr-body-update"
FAKE="$ROOT/repo"
MOCK="$ROOT/mock"
mkdir -p "$FAKE/scripts" "$MOCK"
cp "$TOOL" "$FAKE/scripts/"
: >"$MOCK/calls.log"

# 偽の scripts/agent-gh。GitHub 側の PR の状態（body: 本文、head: PR の headRefOid）を $MOCK に持つ。
#   dev api repos/hatai/Pelway/pulls/<N>                             → {body, head.sha, html_url} を返す
#   dev api --method PATCH repos/hatai/Pelway/pulls/<N> --input <F> → <F> の body を保存して {body, head.sha} を返す
# MOCK_READ_FAIL・MOCK_PATCH_FAIL があるときはその呼び出しが終了コード 1。ほかの呼び出しは終了コード 9。
# MOCK_CHANGE=body|head があるとき、2 回目の GET の前に本文の末尾に 1 行足すか head を替える（別の編集・push の重なり）。
# MOCK_PATCH_HEAD があるとき、PATCH の応答の head は書き込みの直前に進んだ別の値になる。
# MOCK_PATCH_BODY があるとき、PATCH の応答の本文は送った本文と違う（末尾に改行が 1 つ多い）ものになる。
# 引数は $MOCK/calls.log に 1 行 1 引数で残る（呼び出しの区切りは空行）。
cat >"$FAKE/scripts/agent-gh" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$@" >>"$MOCK/calls.log"
printf '\n' >>"$MOCK/calls.log"
[[ "$1 $2" == "dev api" ]] || exit 9
method=GET endpoint="" input=""
while (($#)); do
  case "$1" in
    --method) method="$2"; shift 2 ;;
    --input) input="$2"; shift 2 ;;
    repos/hatai/Pelway/pulls/*) endpoint="$1"; shift ;;
    *) shift ;;
  esac
done
[[ -n "$endpoint" ]] || exit 9
n="${endpoint##*/}"
if [[ "$method" == GET && -z "$input" ]]; then
  [[ -z "${MOCK_READ_FAIL:-}" ]] || { echo "injected read failure" >&2; exit 1; }
  gets=$(( $(cat "$MOCK/gets" 2>/dev/null || echo 0) + 1 ))
  echo "$gets" >"$MOCK/gets"
  if ((gets == 2)) && [[ -n "${MOCK_CHANGE:-}" ]]; then
    [[ "${MOCK_CHANGE}" != *body* ]] || echo "- 別の担当が編集した行" >>"$MOCK/body"
    [[ "${MOCK_CHANGE}" != *head* ]] || echo 4444444444444444444444444444444444444 >"$MOCK/head"
  fi
  jq -n --rawfile body "$MOCK/body" --arg head "$(cat "$MOCK/head")" --arg n "$n" \
    '{body: $body, head: {sha: $head}, html_url: ("https://github.com/hatai/Pelway/pull/" + $n)}'
elif [[ "$method" == PATCH && -n "$input" ]]; then
  [[ -z "${MOCK_PATCH_FAIL:-}" ]] || { echo "injected patch failure" >&2; exit 1; }
  cp "$input" "$MOCK/patch-input.json"
  jq -j .body "$input" >"$MOCK/body"
  cp "$MOCK/body" "$MOCK/resp-body"
  [[ -z "${MOCK_PATCH_BODY:-}" ]] || printf '\n' >>"$MOCK/resp-body"
  [[ -z "${MOCK_PATCH_HEAD:-}" ]] || echo 5555555555555555555555555555555555555 >"$MOCK/head"
  jq -n --rawfile body "$MOCK/resp-body" --arg head "$(cat "$MOCK/head")" '{body: $body, head: {sha: $head}}'
else
  exit 9
fi
EOF
chmod +x "$FAKE/scripts/agent-gh"
echo 6940d00be0179d3157f73ff912ff5ef3a333189e >"$MOCK/head"

# upd [args...]: リポジトリの外（$ROOT。本体は自分の置き場所の隣の agent-gh を使う）から本体を実行して
# 終了コードを返す。出力は $ROOT/out・$ROOT/err。一時ファイルは TMPDIR に作られる
upd() { : >"$MOCK/gets"; (cd "$ROOT" && MOCK="$MOCK" timeout 30 python3 -I "$FAKE/scripts/pr-body-update" "$@") >"$ROOT/out" 2>"$ROOT/err"; }
# patches: 偽の agent-gh が受けた PATCH の回数
patches() { grep -c '^PATCH$' "$MOCK/calls.log" || true; }
# input_file: --input に渡したファイルのパス
input_file() { awk '$0 == "--input" { getline; print; exit }' "$MOCK/calls.log"; }
check() {
  local name="$1"; shift
  if "$@"; then printf 'ok   %s\n' "$name"; else printf 'FAIL %s\n' "$name"; failures+=("$name"); fi
}
eq() { [[ "$1" == "$2" ]]; }
has() { grep -q -- "$2" "$1"; }
has_all() { local f="$1"; shift; local w; for w in "$@"; do grep -q -- "$w" "$f" || return 1; done; }
no_nonascii() { ! LC_ALL=C grep -q '[^[:space:][:print:]]' "$1"; }
# fails_quietly <終了コード>: 0 でなく、PATCH を送っていない
fails_quietly() { (( $1 != 0 )) && [[ "$(patches)" == 0 ]]; }
# bad_args [args...]: 終了コード 2 で、agent-gh を呼ばない
bad_args() { local rc=0; upd "$@" || rc=$?; [[ "$rc" == 2 && ! -s "$MOCK/calls.log" ]]; }
# put_body: 標準入力の本文を GitHub 側に置き、呼び出しの記録を空にする。期待値の元として $ROOT/given にも残す
put_body() { cat >"$MOCK/body"; cp "$MOCK/body" "$ROOT/given"; : >"$MOCK/calls.log"; }
# expected_body [sed の式...]: 置いた本文の OLD_HEAD と OLD_VER（追加の差分があればそれも）を HEAD と VER に
# 替えた本文（書き換えの期待値）を標準出力に出す
expected_body() { sed -e "s|$OLD_HEAD|$HEAD|g" -e "s|$OLD_VER|$VER|g" "$@" "$ROOT/given"; }
# verify_on <PR 番号>: VER を --verified にして本体を 1 回実行し、終了コードを rc に残して返す
verify_on() { upd "$1" --verified "$VER"; rc=$?; return "$rc"; }
# expect_ok <PR 番号> <名前> [sed の式...]: 実行して、終了コード 0 で PATCH が 1 回、GitHub 側の本文が期待値どおり
expect_ok() { local pr="$1" name="$2"; shift 2; verify_on "$pr"; check "$name" ok_as "$@"; }
ok_as() { (( rc == 0 )) && [[ "$(patches)" == 1 ]] && cmp -s "$MOCK/body" <(expected_body "$@"); }
# expect_fail <PR 番号> <名前> <stderr にあるべき文字列...>: 実行して、0 以外で PATCH が無く、stderr にその文字列がある
expect_fail() { local pr="$1" name="$2"; shift 2; verify_on "$pr"; check "$name" fails_with "$@"; }
fails_with() { (( rc != 0 )) && [[ "$(patches)" == 0 ]] && has_all "$ROOT/err" "$@"; }

OLD_HEAD=2222222222222222222222222222222222222222
OLD_VER=3333333333333333333333333333333333333333
HEAD=6940d00be0179d3157f73ff912ff5ef3a333189e
VER=a06ddced9e4885a727974aedcbcf7398a8b7bad9

# sample <HEAD の sha> <最後に検証した sha>: テストで使う PR 本文（.github/pull_request_template.md の並び）。
# 19 行目が「最後に検証した sha」の行、29 行目が HEAD の行
sample() {
  cat <<EOF
## 概要

Issue #734。クライアントの道具を用意する。「」・：・→の日本語の記号を含む。

Closes #734

## 変更内容

- 節の外にある形の同じ行は書き換えない: 「最後に検証した sha: \`0000000000000000000000000000000000000000\`」・「HEAD: \`1111111111111111111111111111111111111111\`」

## 確認方法/検証

- \`scripts/red-check 0ec024d\` → red
- \`mise run pr-size\` → \`pr-size: +194 / -0\`

## レビューと QA

- 実装者メモ：\`docs/worklogs/issue-734.md\`
- 最後に検証した sha：\`$2\`
- QA: 未検証（pelway-qa 未実行）

## 対象外

- PR 本文のほかの欄の自動生成

## 対象

- ブランチ：\`agent/734-client-tooling\`（base：main）
- HEAD：\`$1\`
Implementer: agent=claude model=unknown effort=unknown
EOF
}

# --- 書き換わること（Issue #809 受け入れ条件 1）と、ほかの行が変わらないこと（受け入れ条件 3） ---
sample "$OLD_HEAD" "$OLD_VER" | put_body
verify_on 743
check "exit 0 on success" eq "$rc" 0
check "HEAD line becomes headRefOid and the verified line becomes the given sha" ok_as
check "diff before and after has only the two fixed lines on each side" eq "$(diff "$ROOT/given" "$MOCK/body" | grep -c '^[<>]')" 4
check "the decoy lines outside the two sections are untouched" has "$MOCK/body" '0000000000000000000000000000000000000000'
check "reads twice (before planning and right before the write) and patches once" eq "$(grep -c '^dev$' "$MOCK/calls.log")" 3
check "stdout reports the PR and the new values" has_all "$ROOT/out" 743 "$HEAD" "$VER"
upd 743 --verified "${VER^^}"
check "an uppercase --verified is written in lowercase" cmp -s "$MOCK/body" <(expected_body)

# --- 日本語の本文はファイル経由で渡す（受け入れ条件 4。skill write-pull-request の決まり） ---
check "no non-ASCII bytes in the argv of any call" no_nonascii "$MOCK/calls.log"
check "PATCH sends the whole new body through --input" cmp -s <(jq -j .body "$MOCK/patch-input.json") <(expected_body)
check "the temporary input file is removed" bash -c '[[ -n "$1" && ! -e "$1" ]]' _ "$(input_file)"

# --- 書き換えるのは印の直後の sha だけ。コロン・バッククォート・行の前後の文・短い sha はそのまま残る ---
put_body <<EOF
## レビューと QA

実装者メモ: \`docs/worklogs/issue-633.md\`（round 1〜13）。最後に検証した sha: \`fedcba9\`。

## 対象

- HEAD: 0123456（Kodiak のマージコミット）
EOF
expect_ok 633 "variant forms: only the sha after each label is replaced" -e "s|fedcba9|$VER|" -e "s|HEAD: 0123456|HEAD: $HEAD|"

# --- 印は自分の節の中だけで探す。前の節・後ろの節にある同じ形の行や、HEAD を含む長い単語は書き換えない ---
put_body <<EOF
## 変更内容

- HEAD: \`1111111111111111111111111111111111111111\`

## 対象

- HEAD: \`$OLD_HEAD\`
- OVERHEAD: \`4444444444444444444444444444444444444\`

## レビューと QA

- 最後に検証した sha: \`$OLD_VER\`
- 参考にした HEAD: \`5555555555555555555555555555555555555\`
EOF
expect_ok 745 "sections in another order: lines outside their own section and longer words containing HEAD are left alone"

# --- テンプレートの HTML コメント（<!-- 〜 -->、複数行も含む）は残してよい。注釈の中の見出しや印は数えない ---
put_body <<EOF
## レビューと QA

<!-- 事実と検証時点の sha のみ記載。未検証の項目は「未検証」と明記する -->
- 実装者メモ: \`docs/worklogs/issue-809.md\`
- <!-- round 1 --> 最後に検証した sha: \`$OLD_VER\`

## 対象

<!-- ブランチ名と HEAD の sha -->
- ブランチ: \`agent/809-pr-body-sha-tool\`
<!--
## 対象
- HEAD: \`5555555555555555555555555555555555555\`
-->
<!-- HEAD: \`1111111111111111111111111111111111111111\` -->
- HEAD: \`$OLD_HEAD\` <!-- 前は HEAD: 4444444444444444444444444444444444444 -->
EOF
expect_ok 740 "HTML comments kept from the template: headings and labels inside them are not counted and the comments stay"
sed '26i<!-- 閉じ忘れ' <(sample "$OLD_HEAD" "$OLD_VER") | put_body
expect_fail 743 "an unclosed HTML comment opened above the heading hides what follows" '「対象」の見出しが本文に無い'

# --- Markdown のコードは本文の構造ではない（レビュー round 2 の must-fix）---
# コメントの外では、インラインコードの中の `<!--` や `-->` は注釈の開き・閉じとして数えない
put_body <<EOF
## 変更内容

- \`<!--\` は HTML コメントの開始の記号（インラインコードの中は注釈として数えない）
- 閉じる記号 \`-->\` も同じ

## レビューと QA

- 最後に検証した sha: \`$OLD_VER\`

## 対象

- HEAD: \`$OLD_HEAD\`
EOF
expect_ok 746 "comment signs inside inline code are not comment signs: only the real lines change"
# 行頭から始まった注釈は、バッククォートで囲んだ `-->` でもその行で閉じる（GitHub の表示と同じ）
put_body <<EOF
## レビューと QA

- 最後に検証した sha: \`$OLD_VER\`

## 対象

<!-- 注釈の中では終端が最優先: \`-->\` で閉じる
HEAD: \`$OLD_HEAD\`
EOF
expect_ok 750 "inside a comment the terminator wins over backticks: the comment line stays and the real line changes"
# 行をまたぐインラインコードの中の `<!--` は注釈として数えない
put_body <<EOF
## 変更内容

- \`<!--
  は注釈の開始\` の記号（行をまたぐインラインコード）

## レビューと QA

- 最後に検証した sha: \`$OLD_VER\`

## 対象

- HEAD: \`$OLD_HEAD\`
EOF
expect_ok 751 "an inline code span continued on the next line: only the real lines change"
# 行をまたぐインラインコードは、新しいリスト項目の行には及ばない
put_body <<EOF
## レビューと QA

- 最後に検証した sha: \`$OLD_VER\`

## 対象

- 閉じていないバッククォート \`abc
- HEAD: \`$OLD_HEAD\`
EOF
expect_ok 754 "an unclosed backtick does not run into the next list item: only the real lines change"
# フェンス付きコードブロック（``` か ~~~）の中の見出し・印・注釈は数えない
put_body <<EOF
## 変更内容

\`\`\`md
## レビューと QA
- 最後に検証した sha: \`0000000000000000000000000000000000000000\`
<!-- フェンスの中の注釈
\`\`\`

~~~text
## 対象
- HEAD: \`1111111111111111111111111111111111111111\`
~~~

## レビューと QA

- 最後に検証した sha: \`$OLD_VER\`

## 対象

- HEAD: \`$OLD_HEAD\`
EOF
expect_ok 747 "headings and labels inside fenced code blocks are not counted: only the real lines change"

# --- 印はコードの外になければならない（レビュー round 3 の must-fix）。コード例や字下げされた行を決まった行にしない ---
put_body <<EOF
## レビューと QA

- 書き方の例: \`最後に検証した sha: abcdef0\`（インラインコードの中は候補にしない）

## 対象

- 書き方の例は \`HEAD: abcdef0\` と書く
EOF
expect_fail 748 "only examples with the label inside inline code: both real lines are reported missing" \
  '「レビューと QA」に「最後に検証した sha」の行が無い' '「対象」に HEAD の行が無い'
put_body <<EOF
## レビューと QA

- 書き方の例: \`最後に検証した sha: abcdef0\`
- 最後に検証した sha: \`$OLD_VER\`

## 対象

- ブランチ: \`agent/809-HEAD-fix\`
- 書き方の例は \`HEAD: abcdef0\` と書く
    HEAD: 9999999999999999999999999999999999999999
- HEAD: \`$OLD_HEAD\`
EOF
expect_ok 749 "examples and a real line in both sections: only the real lines are updated and the examples stay"
put_body <<EOF
## レビューと QA

最後に検証した sha: $OLD_VER（バッククォート無し）

## 対象

- 旧形式の例 \`HEAD: abcdef0\` → 新形式: HEAD: $OLD_HEAD
EOF
expect_ok 752 "a label outside code: only the sha after the label outside code changes"
# 行の途中から始まった注釈は `-->` で閉じ、その後ろは表示される（行頭から始まった注釈とは扱いが違う）
put_body <<EOF
## レビューと QA

- 最後に検証した sha: \`$OLD_VER\`

## 対象

- メモ <!-- 始まり
  続き --> HEAD: \`9999999999999999999999999999999999999999\`
- HEAD: \`$OLD_HEAD\`
EOF
expect_fail 753 "a comment started mid-line ends at its terminator and the HEAD after it is counted" '「対象」に HEAD の行が 2 つある（8 行目・9 行目）'

# --- 改行コードと末尾の改行はそのまま残る ---
sed 's/$/\r/' <(sample "$OLD_HEAD" "$OLD_VER") | put_body
expect_ok 744 "CRLF is kept on every line"
printf '%s' "$(sample "$OLD_HEAD" "$OLD_VER")" | put_body
expect_ok 744 "a body without a trailing newline stays without one"
{ sample "$OLD_HEAD" "$OLD_VER"; echo; } | put_body
expect_ok 744 "a trailing blank line is kept"

# --- 決まった行が無いときは書き換えずに失敗し、どの行が無いかを出す（受け入れ条件 2） ---
# without <sed のスクリプト>: sample の本文に sed を当てたものを GitHub 側に置く
without() { sed "$1" <(sample "$OLD_HEAD" "$OLD_VER") | put_body; }
NO_HEAD='「対象」に HEAD の行が無い'
NO_VER='「レビューと QA」に「最後に検証した sha」の行が無い'
NO_SHA_HEAD='で、HEAD の直後に sha が無い'
NO_SHA_VER='で、「最後に検証した sha」の直後に sha が無い'
without '/^- HEAD：/d'
expect_fail 743 "missing HEAD line" "$NO_HEAD"
without '/最後に検証した sha：/d'
expect_fail 743 "missing verified line" "$NO_VER"
without '/^- HEAD：/d; /最後に検証した sha：/d'
expect_fail 743 "both lines missing: both are reported in one run" "$NO_HEAD" "$NO_VER"
without '/^## 対象$/d'
expect_fail 743 "missing section heading" '「対象」の見出しが本文に無い'
without '26a## 対象'
expect_fail 743 "two section headings" '「対象」の見出しが本文に 2 つある（26 行目・27 行目）'
without '29a\- HEAD：`4444444444444444444444444444444444444`'
expect_fail 743 "two HEAD lines" '「対象」に HEAD の行が 2 つある（29 行目・30 行目）'
without '29s/.*/- HEAD：マージコミット 6940d00 が先端/'
expect_fail 743 "HEAD line without a sha right after the label" '「対象」の HEAD の行（29 行目）'"$NO_SHA_HEAD"
without '19s/.*/- 最後に検証した sha：`3333333333333333333333333333333333333333`（round 1 の最後に検証した sha：`2222222222222222222222222222222222222222`）/'
expect_fail 743 "two labels on one line" '「レビューと QA」の「最後に検証した sha」の行（19 行目）に「最後に検証した sha」が 2 つある'
# sha の直後に英数字が続く語（2222222xyz など）と 41 桁以上の並びは sha にしない（レビュー round 2 の must-fix）
without '29s/.*/- HEAD：`2222222xyz`/'
expect_fail 743 "sha-like token followed by other letters on the HEAD line" '「対象」の HEAD の行（29 行目）'"$NO_SHA_HEAD"
without '29s/.*/- HEAD：2222222xyz/'
expect_fail 743 "sha-like token without backticks on the HEAD line" '「対象」の HEAD の行（29 行目）'"$NO_SHA_HEAD"
without '19s/.*/- 最後に検証した sha：`3333333wxyz`/'
expect_fail 743 "sha-like token followed by other letters on the verified line" '「レビューと QA」の「最後に検証した sha」の行（19 行目）'"$NO_SHA_VER"
without '29s/.*/- HEAD：`22222222222222222222222222222222222222222`/'
expect_fail 743 "a token of 41 hex digits is not a sha" '「対象」の HEAD の行（29 行目）'"$NO_SHA_HEAD"
printf '' | put_body
expect_fail 743 "empty body: both headings are reported missing" '「レビューと QA」の見出しが本文に無い' '「対象」の見出しが本文に無い'

# --- 引数の誤り ---
: >"$MOCK/calls.log"
check "no arguments: exit 2" bad_args
check "missing --verified: exit 2" bad_args 743
check "non-numeric PR number: exit 2" bad_args abc --verified "$VER"
check "verified sha must be 40 hex digits: exit 2" bad_args 743 --verified "${VER:0:39}"
check "unknown option: exit 2" bad_args 743 --verified "$VER" --force

# --- GitHub への読み書きの失敗 ---
sample "$OLD_HEAD" "$OLD_VER" | put_body
(export MOCK_READ_FAIL=1; verify_on 743); rc=$?
check "read failure: exit 1, the error is passed through, nothing is patched" bash -c '(( $1 == 1 )) && grep -q "injected read failure" "$2" && [[ "$(cat "$3")" == 0 ]]' _ "$rc" "$ROOT/err" <(patches)
(export MOCK_PATCH_FAIL=1; verify_on 743); rc=$?
check "PATCH failure: exit 1 and the error is passed through" bash -c '(( $1 == 1 )) && grep -q "injected patch failure" "$2"' _ "$rc" "$ROOT/err"
echo not-a-sha >"$MOCK/head"
: >"$MOCK/calls.log"
verify_on 743
check "headRefOid that is not a sha: exit 1 and nothing is patched" bash -c '(( $1 == 1 )) && [[ "$(cat "$2")" == 0 ]]' _ "$rc" <(patches)
echo "$HEAD" >"$MOCK/head"

# --- 読み取りの後に PR が変わったら書き換えない（書き込みの直前にもう一度読む。レビュー round 2 の must-fix） ---
sample "$OLD_HEAD" "$OLD_VER" | put_body
(export MOCK_CHANGE=body; verify_on 743); rc=$?
check "body edited after the read: fails without patching and the other edit is kept" bash -c '(( $1 != 0 )) && [[ "$(cat "$2")" == 0 ]] && cmp -s "$3" <(cat "$4"; echo "- 別の担当が編集した行")' _ "$rc" <(patches) "$MOCK/body" "$ROOT/given"
check "body edited after the read: stderr says the PR changed" has "$ROOT/err" "読み取った後に変わった"
sample "$OLD_HEAD" "$OLD_VER" | put_body
(export MOCK_CHANGE=head; verify_on 743); rc=$?
check "head moved after the read: fails without patching" fails_quietly "$rc"
check "head moved after the read: stderr says the PR changed" has "$ROOT/err" "読み取った後に変わった"
echo "$HEAD" >"$MOCK/head"
(export MOCK_PATCH_HEAD=1; verify_on 743); rc=$?
check "head moved at the moment of the write: exit 1 and stderr says to run again" bash -c '(( $1 == 1 )) && grep -q "もう一度実行" "$2"' _ "$rc" "$ROOT/err"
echo "$HEAD" >"$MOCK/head"
(export MOCK_PATCH_BODY=1; verify_on 743); rc=$?
check "GitHub returns a different body: exit 1 and stderr says to check it on GitHub" bash -c '(( $1 == 1 )) && grep -q "GitHub が返した本文が送った内容と違います" "$2"' _ "$rc" "$ROOT/err"

# --- 使い方 ---
: >"$MOCK/calls.log"
upd --help; rc=$?
check "--help: exit 0, usage on stdout, no call" bash -c '(( $1 == 0 )) && [[ -s "$2" && ! -s "$3" ]]' _ "$rc" "$ROOT/out" "$MOCK/calls.log"

if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}" >&2
  exit 1
fi
printf '\nall passed\n'
