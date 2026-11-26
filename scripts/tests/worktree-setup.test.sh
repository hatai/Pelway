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
cd "$ROOT/main" && git commit -qm init --allow-empty && old="$(git rev-parse HEAD)"
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
check "1: stderr has no secret value" bash -c '! grep -q sekret "$ROOT/err" && ! grep -q sekret "$ROOT/out"'

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
check "4: nothing written" bash -c 'test -z "$(ls -A "$ROOT/wt3" | grep -v "^\.git$")"'
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

# 8. t3.json: worktree 作成時に scripts/worktree-setup を走らせる runOnWorktreeCreate の
# アクションが 1 つあり、async=false（終わるまで worktree を使わせない）
python3 -I - "$REPO/t3.json" <<'PY' >"$ROOT/t3-check" 2>&1 || echo FAIL >"$ROOT/t3-check"
import json, sys
d = json.load(open(sys.argv[1]))
acts = [a for a in d.get("actions", []) if a.get("runOnWorktreeCreate")]
assert len(acts) == 1, f"runOnWorktreeCreate actions: {len(acts)}"
a = acts[0]
assert a.get("type") == "script" and a.get("script") == "scripts/worktree-setup", a
assert a.get("async") is False, a
print("ok")
PY
check "8: t3.json has exactly one runOnWorktreeCreate action for scripts/worktree-setup with async=false" \
  has "$ROOT/t3-check" "ok"
check "8: the script path exists" [ -x "$REPO/scripts/worktree-setup" ]

# 9. .worktreeinclude: agent-team.local.toml を持ち越し、agent-team.toml は書かない
check "9: .worktreeinclude lists agent-team.local.toml" \
  bash -c "grep -qx 'agent-team.local.toml' '$REPO/.worktreeinclude'"
check "9: .worktreeinclude does not list agent-team.toml" \
  bash -c "! grep -qx 'agent-team.toml' '$REPO/.worktreeinclude'"

# 10. agent-team.local.toml は gitignore 済みで、hk の pre-commit でコミットを止める
check "10: .gitignore covers agent-team.local.toml" \
  bash -c "grep -qx 'agent-team.local.toml' '$REPO/.gitignore'"
check "10: hk guard blocks committing it" \
  bash -c "grep -q 'agent-team.local.toml' '$REPO/hk.pkl' && grep -q 'glob = List(\"agent-team.local.toml\")' '$REPO/hk.pkl'"

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
