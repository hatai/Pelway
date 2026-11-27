#!/usr/bin/env bash
# scripts/worktree-setup（Issue #804）のテスト。worktree 作成時に本体の checkout から
# agent-team.toml（基点が古くて無いときの補填）と agent-team.local.toml（開発者ごとの
# 上書きの持ち越し）を写すことを、一時の git リポジトリで確かめる。
# 併せて、T3 の runOnWorktreeCreate（t3.json）・worktreeinclude 機構（.worktreeinclude）・
# gitignore・hk のコミット防止・雛形の検証可能性の、ファイルレベルの存在を確かめる
# （文言の存在確認で、意味はレビューで引用を読んで判定する）。
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT
failures=()

check() {
  local name="$1"; shift
  if "$@"; then printf 'ok   %s\n' "$name"; else printf 'FAIL %s\n' "$name"; failures+=("$name"); fi
}
has() { grep -qF -- "$2" "$1"; }
run_setup() { bash "$REPO/scripts/worktree-setup" "$@" >"$ROOT/out" 2>"$ROOT/err"; }

# 一時の git リポジトリ: 初回コミットには agent-team.toml を置かず、2 回目で追加する
# （基点が古い worktree に追跡ファイルが無い状況を作る）
git -C "$ROOT" init -q main
git -C "$ROOT/main" config user.name test
git -C "$ROOT/main" config user.email test@example.invalid
cd "$ROOT/main" || exit 1 # ここより下の echo はこのディレクトリへの書き込み（失敗時に呼び出し元の cwd を汚さない）
git commit -qm init --allow-empty && old="$(git rev-parse HEAD)"
echo '[agents.x]' > agent-team.toml
echo 'use = []' > unrelated.txt
git add agent-team.toml unrelated.txt && git commit -qm add-toml
echo 'local' > agent-team.local.toml
echo 'token=sekret' > fnox.local.toml
git worktree add -q "$ROOT/wt" "$old" --detach   # agent-team.toml の無い版からの worktree

# 1. 古い版からの worktree に agent-team.toml が写され、内容が本体と一致する
run_setup --from "$ROOT/main" --dest "$ROOT/wt"; rc=$?
check "1: setup exits 0" [ "$rc" -eq 0 ]
check "1: copies agent-team.toml" [ -f "$ROOT/wt/agent-team.toml" ]
check "1: copied content is identical" cmp -s "$ROOT/main/agent-team.toml" "$ROOT/wt/agent-team.toml"
check "1: copies agent-team.local.toml" [ -f "$ROOT/wt/agent-team.local.toml" ]
check "1: local content is identical" cmp -s "$ROOT/main/agent-team.local.toml" "$ROOT/wt/agent-team.local.toml"
check "1: does not copy unrelated tracked files" [ ! -e "$ROOT/wt/unrelated.txt" ]
check "1: does not copy other local files" [ ! -e "$ROOT/wt/fnox.local.toml" ]
check "1: stderr has no secret value" bash -c '! grep -q sekret "$1/err" && ! grep -q sekret "$1/out"' _ "$ROOT"

# 2. 既にあるものは上書きしない（追跡ファイルでも、手で置いた local でも）
echo 'mine' > "$ROOT/wt/agent-team.toml"
echo 'mylocal' > "$ROOT/wt/agent-team.local.toml"
run_setup --from "$ROOT/main" --dest "$ROOT/wt"; rc=$?
check "2: exits 0 with files present" [ "$rc" -eq 0 ]
check "2: existing agent-team.toml is untouched" [ "$(cat "$ROOT/wt/agent-team.toml")" = "mine" ]
check "2: existing local is untouched" [ "$(cat "$ROOT/wt/agent-team.local.toml")" = "mylocal" ]

# 3. 本体に local が無ければ成功のまま何も写さない
git worktree add -q "$ROOT/wt2" "$old" --detach 2>/dev/null || git worktree add -q "$ROOT/wt2" "$old" --detach
mv "$ROOT/main/agent-team.local.toml" "$ROOT/main/local.bak"
run_setup --from "$ROOT/main" --dest "$ROOT/wt2"; rc=$?
check "3: missing local still succeeds" [ "$rc" -eq 0 ]
check "3: agent-team.toml still copied" [ -f "$ROOT/wt2/agent-team.toml" ]
check "3: no local file appears" [ ! -e "$ROOT/wt2/agent-team.local.toml" ]
mv "$ROOT/main/local.bak" "$ROOT/main/agent-team.local.toml"

# 4. 本体にも agent-team.toml が無いときは非 0 で、理由が標準エラーに出て worktree に何も置かない
git worktree add -q "$ROOT/wt3" "$old" --detach
mv "$ROOT/main/agent-team.toml" "$ROOT/main/team.bak"
run_setup --from "$ROOT/main" --dest "$ROOT/wt3"; rc=$?
check "4: missing main agent-team.toml fails" [ "$rc" -ne 0 ]
check "4: names the missing file" has "$ROOT/err" "agent-team.toml"
check "4: nothing written" bash -c 'test -z "$(ls -A "$1" | grep -v "^\.git$")"' _ "$ROOT/wt3"
mv "$ROOT/main/team.bak" "$ROOT/main/agent-team.toml"

# 5. 本体側が symlink・ディレクトリのときは写さずに失敗する
git worktree add -q "$ROOT/wt4" "$old" --detach
mv "$ROOT/main/agent-team.toml" "$ROOT/main/team.bak"
ln -s /etc/hostname "$ROOT/main/agent-team.toml"
run_setup --from "$ROOT/main" --dest "$ROOT/wt4"; rc=$?
check "5a: symlink main file fails" [ "$rc" -ne 0 ]
check "5a: reason on stderr" has "$ROOT/err" "agent-team.toml"
check "5a: symlink not copied" [ ! -e "$ROOT/wt4/agent-team.toml" ]
rm "$ROOT/main/agent-team.toml"; mkdir "$ROOT/main/agent-team.toml"
run_setup --from "$ROOT/main" --dest "$ROOT/wt4"; rc=$?
check "5b: directory main file fails" [ "$rc" -ne 0 ]
check "5b: directory not copied" [ ! -e "$ROOT/wt4/agent-team.toml" ]
rmdir "$ROOT/main/agent-team.toml"; mv "$ROOT/main/team.bak" "$ROOT/main/agent-team.toml"
# local が symlink でも失敗にはしない（任意ファイル）。ただし写さない
git worktree add -q "$ROOT/wt5" "$old" --detach
mv "$ROOT/main/agent-team.local.toml" "$ROOT/main/local.bak"
ln -s /etc/hostname "$ROOT/main/agent-team.local.toml"
run_setup --from "$ROOT/main" --dest "$ROOT/wt5"; rc=$?
check "5c: symlink local is skipped, not fatal" [ "$rc" -eq 0 ]
check "5c: required file still copied" [ -f "$ROOT/wt5/agent-team.toml" ]
check "5c: symlink local not copied" [ ! -e "$ROOT/wt5/agent-team.local.toml" ]
rm "$ROOT/main/agent-team.local.toml"; mv "$ROOT/main/local.bak" "$ROOT/main/agent-team.local.toml"

# 5c. 本体に通常ファイルの local があるのに worktree 側へ配置できないときは
# 非 0 で止まる（枠・担当の上書きを失ったまま交代させない）
git worktree add -q "$ROOT/wt5c" "$old" --detach
# agent-team.toml だけ先に置き、local の配置が失敗する状況にする
# （本体の agent-team.local.toml は前の試験のものが残っている）
cp "$ROOT/main/agent-team.toml" "$ROOT/wt5c/"
chmod a-w "$ROOT/wt5c"
run_setup --from "$ROOT/main" --dest "$ROOT/wt5c"; rc=$?
chmod u+w "$ROOT/wt5c"
check "5c: local write failure: exits non-zero" [ "$rc" -ne 0 ]
check "5c: local write failure: names agent-team.local.toml" has "$ROOT/err" "agent-team.local.toml"
check "5c: no partial local left" [ ! -e "$ROOT/wt5c/agent-team.local.toml" ]
check "5c: no leftover temp" bash -c '! ls "$1"/agent-team.local.toml.worktree-setup.*.tmp >/dev/null 2>&1' _ "$ROOT/wt5c"

# 5d. 公開が失敗したあとも一時物は残らない（書き込み先は O_TMPFILE の無名
# inode で名前を持たないため、名差し替え・後始末の経路自体が無い。既に最終名が
# ある場合は FileExistsError で失敗する経路）。worktree-setup-copy を直接
# 呼んで確かめる
git worktree add -q "$ROOT/wt5d" "$old" --detach
echo 'already' > "$ROOT/wt5d/agent-team.local.toml"
exec {wt5dfd}<"$ROOT/wt5d" {main5dfd}<"$ROOT/main"
python3 -I "$REPO/scripts/worktree-setup-copy" \
  "$wt5dfd" "$main5dfd" agent-team.local.toml 2>/dev/null; rc=$?
exec {wt5dfd}<&- {main5dfd}<&-
check "5d: publish failure exits 8" [ "$rc" -eq 8 ]
check "5d: no leftover temp on publish failure" bash -c '! ls "$1"/agent-team.local.toml.worktree-setup.*.tmp >/dev/null 2>&1' _ "$ROOT/wt5d"
check "5d: existing file untouched" [ "$(cat "$ROOT/wt5d/agent-team.local.toml")" = "already" ]

# 6. --dest の既定は $T3CODE_WORKTREE_PATH、--from の既定は $T3CODE_PROJECT_ROOT
git worktree add -q "$ROOT/wt6" "$old" --detach
( cd "$ROOT/wt6" && T3CODE_PROJECT_ROOT="$ROOT/main" T3CODE_WORKTREE_PATH="$ROOT/wt6" run_setup ); rc=$?
check "6: env defaults work" [ "$rc" -eq 0 ]
check "6: file copied via env defaults" [ -f "$ROOT/wt6/agent-team.toml" ]

# 7. worktree の外側には書かない（--dest が無ければ失敗し、--from 側には書かない）
run_setup --from "$ROOT/main" --dest "$ROOT/no-such-dir"; rc=$?
check "7: missing dest fails" [ "$rc" -ne 0 ]
hash_before="$(cd "$ROOT/main" && find . -type f ! -path './.git/*' | sort | xargs sha256sum | sha256sum)"
git worktree add -q "$ROOT/wt7" "$old" --detach
run_setup --from "$ROOT/main" --dest "$ROOT/wt7"
hash_after="$(cd "$ROOT/main" && find . -type f ! -path './.git/*' | sort | xargs sha256sum | sha256sum)"
check "7: main checkout is never modified" [ "$hash_before" = "$hash_after" ]

# 7b. --dest は --from と同じ repo の worktree の最上位でなければならない（repo の外への
# 書き出しを止める。repo でないディレクトリ・worktree 内の途中の階層・別 repo は拒否）
mkdir -p "$ROOT/outside"
run_setup --from "$ROOT/main" --dest "$ROOT/outside"; rc=$?
check "7b: non-repo dest is refused" [ "$rc" -ne 0 ]
check "7b: nothing written outside" bash -c '[ ! -e "$1/outside/agent-team.toml" ] && [ ! -e "$1/outside/agent-team.local.toml" ]' _ "$ROOT"
mkdir -p "$ROOT/wt/sub"
run_setup --from "$ROOT/main" --dest "$ROOT/wt/sub"; rc=$?
check "7b: worktree subdir dest is refused" [ "$rc" -ne 0 ]
check "7b: nothing written to subdir" [ ! -e "$ROOT/wt/sub/agent-team.toml" ]
git -C "$ROOT" init -q other
git -C "$ROOT/other" config user.name test && git -C "$ROOT/other" config user.email test@example.invalid
git -C "$ROOT/other" commit -qm init --allow-empty
run_setup --from "$ROOT/main" --dest "$ROOT/other"; rc=$?
check "7b: different repo's checkout is refused" [ "$rc" -ne 0 ]
check "7b: nothing written to other repo" [ ! -e "$ROOT/other/agent-team.toml" ]

# 7c. 逆向きも拒否する: --from は本体の checkout（リンク済み worktree は不可）、
# --dest は本体の checkout でないリンク済み worktree（本体へは書かない）
run_setup --from "$ROOT/wt" --dest "$ROOT/wt2"; rc=$?
check "7c: linked worktree as --from is refused" [ "$rc" -ne 0 ]

# 7d. 登録済み worktree の管理 dir を指す .git ファイルを持つ未登録の
# ディレクトリは、メタデータの検査を通るように見えても本体側の登録に載らない
# ため拒否する
git -C "$ROOT/main" worktree add -q "$ROOT/real-wt7d" "$old" --detach
mkdir -p "$ROOT/ghost-wt"
cp "$ROOT/real-wt7d/.git" "$ROOT/ghost-wt/.git"
run_setup --from "$ROOT/main" --dest "$ROOT/ghost-wt"; rc=$?
check "7d: unregistered dir with valid metadata: exits non-zero" [ "$rc" -ne 0 ]
check "7d: no copy into unregistered dir" [ ! -f "$ROOT/ghost-wt/agent-team.toml" ]
git -C "$ROOT/main" worktree remove --force "$ROOT/real-wt7d"
run_setup --from "$ROOT/main" --dest "$ROOT/main"; rc=$?
check "7c: main checkout as --dest is refused" [ "$rc" -ne 0 ]
check "7c: main checkout was not written" bash -c '[ "$(cat "$1/main/agent-team.local.toml")" = "local" ]' _ "$ROOT"

# 8. t3.json: worktree 作成時に worktree-setup を走らせる runOnWorktreeCreate の
# スクリプトが 1 つあり、async=false（終わるまで worktree を使わせない）。command は
# worktree 側のファイルを実行しないよう、本体の checkout を指す $T3CODE_PROJECT_ROOT の
# 絶対パスでなければならない（worktree からの相対パスだと PR が差し替えた版が走る）
python3 -I - "$REPO/t3.json" <<'PY' >"$ROOT/t3-check" 2>&1 || echo FAIL >"$ROOT/t3-check"
import json, sys
d = json.load(open(sys.argv[1]))
acts = [a for a in d.get("scripts", []) if a.get("runOnWorktreeCreate")]
assert len(acts) == 1, f"runOnWorktreeCreate scripts: {len(acts)}"
a = acts[0]
cmd = a.get("command", "")
assert "T3CODE_PROJECT_ROOT" in cmd, f"command is not anchored at the main checkout: {cmd}"
assert "scripts/worktree-setup" in cmd, cmd
assert a.get("async") is False, a
print("ok")
PY
check "8: t3.json has exactly one runOnWorktreeCreate script calling the main checkout's worktree-setup with async=false" \
  has "$ROOT/t3-check" "ok"
check "8: the script path exists" [ -x "$REPO/scripts/worktree-setup" ]
# 8b. command の文字列の一致だけでなく、T3 と同じ形（シェルで command を実行）で
# 実際に本体の checkout のスクリプトが起きて worktree に写されることを確かめる
git worktree add -q "$ROOT/wt8" "$old" --detach
mkdir -p "$ROOT/main/scripts" && cp "$REPO/scripts/worktree-setup" "$REPO/scripts/worktree-setup-copy" "$ROOT/main/scripts/"
cmd="$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["scripts"][0]["command"])' "$REPO/t3.json")"
( cd "$ROOT/wt8" && T3CODE_PROJECT_ROOT="$ROOT/main" T3CODE_WORKTREE_PATH="$ROOT/wt8" bash -c "$cmd" ) >"$ROOT/out" 2>"$ROOT/err"; rc=$?
check "8b: the stored command runs the setup (cwd=worktree)" [ "$rc" -eq 0 ]
check "8b: file copied via the stored command" [ -f "$ROOT/wt8/agent-team.toml" ]

# 9. .worktreeinclude: agent-team.local.toml を持ち越し、agent-team.toml は書かない
check "9: .worktreeinclude lists agent-team.local.toml" \
  bash -c "grep -qx 'agent-team.local.toml' '$REPO/.worktreeinclude'"
check "9: .worktreeinclude does not list agent-team.toml" \
  bash -c "! grep -qx 'agent-team.toml' '$REPO/.worktreeinclude'"

# 10. agent-team.local.toml は gitignore 済み・未追跡で、hk の pre-commit がコミットを実際に止める
check "10: .gitignore covers agent-team.local.toml" \
  bash -c "grep -qx 'agent-team.local.toml' '$REPO/.gitignore'"
check "10: repo does not track agent-team.local.toml" \
  bash -c "! git -C '$REPO' ls-files --error-unmatch agent-team.local.toml >/dev/null 2>&1"
# 文言の grep だけでは hook の設定ミスを見逃すため、一時 repo で実際にコミットを試す。
# 先に対象外のファイルだけで hook が通ることを確かめてから、対象ファイルを足して
# 止まることを確かめる（hk の起動・設定の読み込みが失敗しても合格にならないように）
check "10: hk is on PATH" command -v hk
git -C "$ROOT" init -q hkrepo
git -C "$ROOT/hkrepo" config user.name test && git -C "$ROOT/hkrepo" config user.email test@example.invalid
cp "$REPO/hk.pkl" "$ROOT/hkrepo/hk.pkl"
echo x > "$ROOT/hkrepo/unrelated.txt"
git -C "$ROOT/hkrepo" add hk.pkl unrelated.txt
( CDPATH='' cd "$ROOT/hkrepo" && hk run pre-commit --step no_agent_team_local ) >"$ROOT/hk-ok" 2>&1; rc=$?
check "10: hk pre-commit passes with no local file staged" [ "$rc" -eq 0 ]
echo 'local' > "$ROOT/hkrepo/agent-team.local.toml"
git -C "$ROOT/hkrepo" add -f agent-team.local.toml
( CDPATH='' cd "$ROOT/hkrepo" && hk run pre-commit --step no_agent_team_local ) >"$ROOT/hk-out" 2>&1; rc=$?
check "10: hk pre-commit blocks agent-team.local.toml" [ "$rc" -ne 0 ]
check "10: hk block names the rule" has "$ROOT/hk-out" "agent-team.local.toml"

# 11. 雛形 agent-team.local.toml.example が存在し、重ねても validate を通る
# （雛形の有効な記述を実際に local として重ねて確かめる。コメントだけでも良いが、
# 有効な上書き例を含めても通ることを重ねた検証で確認する）
check "11: example file exists" [ -f "$REPO/agent-team.local.toml.example" ]
mkdir -p "$ROOT/ex" && cp "$REPO/agent-team.toml" "$ROOT/ex/agent-team.toml"
cp "$REPO/agent-team.local.toml.example" "$ROOT/ex/agent-team.local.toml"
bash "$REPO/scripts/agent-team-config" validate "$ROOT/ex/agent-team.toml" >"$ROOT/ex-out" 2>"$ROOT/ex-err"; rc=$?
check "11: example validates as local overlay" [ "$rc" -eq 0 ]

# 12. 重ねられる例を雛形に足しても validate を通る（書き方の間違いが混入していないこと）
cat >> "$ROOT/ex/agent-team.local.toml" <<'EOF'
[agents.devin]
models = ["swe-2-high"]

[developers.hatai.slots]
devin = 2
EOF
bash "$REPO/scripts/agent-team-config" validate "$ROOT/ex/agent-team.toml" >"$ROOT/ex-out" 2>"$ROOT/ex-err"; rc=$?
check "12: example + real overrides validate" [ "$rc" -eq 0 ]

# 13. 上書きできる項目・できない項目の表が文書にある（存在確認）
check "13: agent-team.md has the override table section" \
  bash -c "grep -q '開発者ごとの上書き' '$REPO/docs/superpowers/specs/agent-team.md' && grep -q 'agent-team.local.toml' '$REPO/docs/superpowers/specs/agent-team.md'"

if ((${#failures[@]})); then
  printf '\n%d failed\n' "${#failures[@]}"
  exit 1
fi
printf '\nall passed\n'
