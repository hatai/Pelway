#!/usr/bin/env bash
# scripts/agent-gh-resign のテスト。実際の git リポジトリと裸の origin を使い、
# gh だけをモックに差し替えて、createCommitOnBranch に渡す差分と ref の付け替えを確かめる。
# モックの gh は GitHub 側の動作（既存ファイルの mode 維持、expectedHeadOid の照合、
# ブランチ ref の移動、署名付きコミットオブジェクトの生成）を裸リポジトリで真似る。
set -uo pipefail

SCRIPT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/agent-gh-resign"
ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT
failures=()

BIN="$ROOT/bin"
REMOTE_DIR="$ROOT/remote.git"
REPO="$ROOT/repo"
mkdir -p "$BIN" "$REPO"

# 偽の gh。
#   repo view                → リポジトリ名を返す
#   api graphql --input -    → createCommitOnBranch を真似る（expectedHeadOid を照合し、
#                              既存ファイルは mode を保ち、新規は 100644 で tree を作り、
#                              gpgsig 入りのコミットを作って ref を進める）
#   api repos/.../git/refs/heads/* (GET/PATCH) → 裸リポジトリの ref を読む/書く
cat >"$BIN/gh" <<'MOCK'
#!/usr/bin/env bash
printf 'ARGS=%s\n' "$*" >>"$CALL_LOG"
if [[ "$1 $2" == "repo view" ]]; then
  printf 'hatai/Pelway\n'
  exit 0
fi
[[ "$1" == "api" ]] || exit 9
shift
endpoint="" jqexpr="" method=GET has_input=0
prev=""
for a in "$@"; do
  case "$prev" in
    -X) method="$a" ;;
    --jq) jqexpr="$a" ;;
  esac
  case "$a" in
    repos/*) endpoint="$a" ;;
    graphql) endpoint="graphql" ;;
    --input) has_input=1 ;;
  esac
  prev="$a"
done
input=""
((has_input)) && input="$(cat)"
if [[ -n "$input" ]]; then
  n=0
  while [[ -e "$CALL_LOG.body.$n" ]]; do n=$((n + 1)); done
  printf '%s' "$input" >"$CALL_LOG.body.$n"
fi
out=""
if [[ "$endpoint" == "graphql" ]]; then
  if [[ "${FAIL_ON:-}" == "graphql" ]]; then
    echo '{"message":"injected failure"}' >&2
    exit 1
  fi
  inp="$(jq -r '.variables.input' <<<"$input")"
  b="$(jq -r '.branch.branchName' <<<"$inp")"
  exp="$(jq -r '.expectedHeadOid' <<<"$inp")"
  cur="$(git -C "$REMOTE_DIR" rev-parse "refs/heads/$b")"
  if [[ "$cur" != "$exp" ]]; then
    printf '{"errors":[{"message":"expectedHeadOid does not match"}]}\n'
    exit 0
  fi
  idx="$(mktemp)"
  GIT_INDEX_FILE="$idx" git -C "$REMOTE_DIR" read-tree "$exp^{tree}"
  while IFS= read -r a; do
    p="$(jq -r .path <<<"$a")"
    blob="$(jq -r .contents <<<"$a" | base64 -d | git -C "$REMOTE_DIR" hash-object -w --stdin)"
    mode="$(GIT_INDEX_FILE="$idx" git -C "$REMOTE_DIR" ls-files -s -- "$p" | awk '{print $1}')"
    GIT_INDEX_FILE="$idx" git -C "$REMOTE_DIR" update-index --add --cacheinfo "${mode:-100644},$blob,$p"
  done < <(jq -c '.fileChanges.additions[]?' <<<"$inp")
  while IFS= read -r p; do
    GIT_INDEX_FILE="$idx" git -C "$REMOTE_DIR" update-index --force-remove -- "$p"
  done < <(jq -r '.fileChanges.deletions[]?.path' <<<"$inp")
  tree="$(GIT_INDEX_FILE="$idx" git -C "$REMOTE_DIR" write-tree)"
  rm -f "$idx"
  {
    printf 'tree %s\nparent %s\n' "$tree" "$exp"
    printf 'author pelway-dev[bot] <354990214+pelway-dev[bot]@users.noreply.github.com> %s +0000\n' "$(date +%s)"
    printf 'committer GitHub <noreply@github.com> %s +0000\n' "$(date +%s)"
    printf 'gpgsig fake-github-signature\n\n%s\n\n%s\n' \
      "$(jq -r .message.headline <<<"$inp")" "$(jq -r .message.body <<<"$inp")"
  } >"$REMOTE_DIR/.new-commit"
  sha="$(git -C "$REMOTE_DIR" hash-object -t commit -w --stdin <"$REMOTE_DIR/.new-commit")"
  git -C "$REMOTE_DIR" update-ref "refs/heads/$b" "$sha"
  out="$(printf '{"data":{"createCommitOnBranch":{"commit":{"oid":"%s"}}}}' "$sha")"
else
  case "$method:$endpoint" in
    GET:repos/*/git/refs/heads/*)
      ref="${endpoint#*git/refs/}"
      out="$(printf '{"ref":"refs/%s","object":{"sha":"%s"}}' "$ref" "$(git -C "$REMOTE_DIR" rev-parse "refs/$ref")")"
      ;;
    PATCH:repos/*/git/refs/heads/*)
      ref="${endpoint#*git/refs/}"
      if [[ "${FAIL_ON:-}" == "refs" ]]; then
        echo '{"message":"injected failure"}' >&2
        exit 1
      fi
      git -C "$REMOTE_DIR" update-ref "refs/$ref" "$(jq -r .sha <<<"$input")"
      out='{}'
      ;;
    *)
      echo "mock gh: unexpected endpoint $method $endpoint" >&2
      exit 9
      ;;
  esac
fi
if [[ -n "$jqexpr" ]]; then
  jq -r "$jqexpr" <<<"$out"
else
  printf '%s\n' "$out"
fi
MOCK
chmod +x "$BIN/gh"

# 本物の git でリポジトリを用意する。origin = 裸リポジトリ（ローカルパス）。
git init -q --bare "$REMOTE_DIR"
git -C "$REMOTE_DIR" symbolic-ref HEAD refs/heads/main
git init -q "$REPO"
git -C "$REPO" config user.name "pelway-dev[bot]"
git -C "$REPO" config user.email "9+pelway-dev[bot]@users.noreply.github.com"
# 環境の署名設定（commit.gpgsign 等）が混入しないよう、テスト用リポジトリでは署名を切る
git -C "$REPO" config commit.gpgsign false
git -C "$REPO" remote add origin "$REMOTE_DIR"
echo base >"$REPO/exec.sh"
chmod +x "$REPO/exec.sh"
git -C "$REPO" add exec.sh
git -C "$REPO" commit -qm "chore: base"
git -C "$REPO" push -q origin HEAD:main

export PATH="$BIN:$PATH"
export GH_TOKEN=fake-token
export REMOTE_DIR
export CALL_LOG="$ROOT/calls.log"

# run_resign <name>: CALL_LOG を空にして agent-gh-resign を実行。RC に終了コード。
run_resign() {
  local name="$1"
  : >"$CALL_LOG"
  rm -f "$CALL_LOG".body.*
  (cd "$REPO" && GH_TOKEN=fake-token bash "$SCRIPT" >"$ROOT/$name.out" 2>"$ROOT/$name.err")
  RC=$?
}

# mutation_bodies: fileChanges を含む graphql リクエスト本文だけを順に出す
mutation_bodies() {
  for f in "$CALL_LOG".body.*; do
    [[ -e "$f" ]] || continue
    jq -e '.variables.input.fileChanges' "$f" >/dev/null 2>&1 && cat "$f"
  done
}

# ブランチに 2 コミット積んで push する（署名なし）。
git -C "$REPO" switch -qc agent/test-branch
echo one >"$REPO/one.txt"
git -C "$REPO" add one.txt
git -C "$REPO" commit -qm "feat: first"
echo "# probe change" >>"$REPO/exec.sh"
echo two >"$REPO/two.txt"
git -C "$REPO" add exec.sh two.txt
git -C "$REPO" commit -qm "feat: second"
git -C "$REPO" push -qu origin HEAD
unsigned_tip="$(git -C "$REPO" rev-parse HEAD)"
first_sha="$(git -C "$REPO" rev-parse HEAD~1)"

# --- 1. push 後の署名なし 2 コミットが mutation で積み直され、ref が付け替わる ---
run_resign resign_two_commits
[[ "$RC" == 0 ]] || failures+=("resign_two_commits: exit $RC, want 0 ($(cat "$ROOT/resign_two_commits.err"))")

mutations=$(grep -c 'api graphql' "$CALL_LOG" || true)
[[ "$mutations" == 2 ]] || failures+=("resign_two_commits: $mutations mutations, want 2")
rewinds=$(grep -c -- '-X PATCH' "$CALL_LOG" || true)
[[ "$rewinds" == 1 ]] || failures+=("resign_two_commits: $rewinds ref PATCHes, want 1 (巻き戻しのみ)")

signed_tip="$(git -C "$REMOTE_DIR" rev-parse refs/heads/agent/test-branch)"
[[ "$signed_tip" != "$unsigned_tip" ]] || failures+=("resign_two_commits: remote tip unchanged")
git -C "$REMOTE_DIR" cat-file -p "$signed_tip" | grep -q '^gpgsig ' ||
  failures+=("resign_two_commits: signed tip lacks gpgsig")

# tree・message・author が元コミットと一致する
[[ "$(git -C "$REMOTE_DIR" rev-parse "$signed_tip^{tree}")" == "$(git -C "$REPO" rev-parse "$unsigned_tip^{tree}")" ]] ||
  failures+=("resign_two_commits: tree differs")
[[ "$(git -C "$REMOTE_DIR" log -1 --format=%s "$signed_tip")" == "feat: second" ]] ||
  failures+=("resign_two_commits: message differs")
[[ "$(git -C "$REMOTE_DIR" log -1 --format=%an "$signed_tip")" == "pelway-dev[bot]" ]] ||
  failures+=("resign_two_commits: author differs")

# 既存の実行ファイルは内容を変えても mode が 100755 のまま（createCommitOnBranch の実測と同じ）
[[ "$(git -C "$REMOTE_DIR" ls-tree "$signed_tip" exec.sh | awk '{print $1}')" == "100755" ]] ||
  failures+=("resign_two_commits: exec.sh mode not preserved")

# 2 個目の mutation の expectedHeadOid が 1 個目の生成 SHA（= 署名付き parent）に繋がる
second_exp="$(mutation_bodies | jq -r '.variables.input.expectedHeadOid' | sed -n 2p)"
signed_parent="$(git -C "$REMOTE_DIR" rev-parse "$signed_tip^")"
[[ "$second_exp" == "$signed_parent" && "$signed_parent" != "$first_sha" ]] ||
  failures+=("resign_two_commits: second mutation not chained to first signed commit")

# ローカルのブランチと追跡 ref が署名付き SHA を指す
[[ "$(git -C "$REPO" rev-parse HEAD)" == "$signed_tip" ]] ||
  failures+=("resign_two_commits: local branch not moved to signed tip")
[[ "$(git -C "$REPO" rev-parse refs/remotes/origin/agent/test-branch)" == "$signed_tip" ]] ||
  failures+=("resign_two_commits: tracking ref not updated")

# --- 2. 先頭がすでに署名付きなら API を呼ばず終了する ---
run_resign resign_idempotent
[[ "$RC" == 0 ]] || failures+=("resign_idempotent: exit $RC, want 0")
[[ ! -s "$CALL_LOG" ]] || failures+=("resign_idempotent: api was called: $(cat "$CALL_LOG")")

# --- 3. 積み増した署名なしコミットだけを署名する（parent は直前の署名付き tip） ---
echo three >"$REPO/three.txt"
git -C "$REPO" add three.txt
git -C "$REPO" commit -qm "feat: third"
git -C "$REPO" push -q origin HEAD
run_resign resign_incremental
[[ "$RC" == 0 ]] || failures+=("resign_incremental: exit $RC, want 0")
mutations=$(grep -c 'api graphql' "$CALL_LOG" || true)
[[ "$mutations" == 1 ]] || failures+=("resign_incremental: $mutations mutations, want 1")
inc_exp="$(mutation_bodies | jq -r '.variables.input.expectedHeadOid')"
[[ "$inc_exp" == "$signed_tip" ]] ||
  failures+=("resign_incremental: expectedHeadOid should be previous signed tip")

# --- 4. 表現できないコミット（新規の実行ファイル）は署名せず警告で終わる ---
echo '#!/bin/sh' >"$REPO/newtool.sh"
chmod +x "$REPO/newtool.sh"
git -C "$REPO" add newtool.sh
git -C "$REPO" commit -qm "feat: new tool"
git -C "$REPO" push -q origin HEAD
tip_before="$(git -C "$REMOTE_DIR" rev-parse refs/heads/agent/test-branch)"
run_resign resign_unrepresentable
[[ "$RC" == 0 ]] || failures+=("resign_unrepresentable: exit $RC, want 0")
grep -q 'newtool.sh' "$ROOT/resign_unrepresentable.err" ||
  failures+=("resign_unrepresentable: no warning about newtool.sh")
[[ "$(git -C "$REMOTE_DIR" rev-parse refs/heads/agent/test-branch)" == "$tip_before" ]] ||
  failures+=("resign_unrepresentable: remote ref moved")

# 以後のコミットがあっても、範囲に表現不能が残る限り署名しない
echo four >"$REPO/four.txt"
git -C "$REPO" add four.txt
git -C "$REPO" commit -qm "feat: fourth"
git -C "$REPO" push -q origin HEAD
run_resign resign_still_unrepresentable
mutations=$(grep -c 'api graphql' "$CALL_LOG" || true)
[[ "$mutations" == 0 ]] || failures+=("resign_still_unrepresentable: signed despite unrepresentable commit in range")

# --- 5. mutation 失敗時は ref を元の先頭に復元して非 0 で終わる ---
git -C "$REPO" reset -q --hard HEAD~2 2>/dev/null # 表現不能な 2 コミットを捨てる
git -C "$REPO" push -qf origin HEAD
echo five >"$REPO/five.txt"
git -C "$REPO" add five.txt
git -C "$REPO" commit -qm "feat: fifth"
git -C "$REPO" push -q origin HEAD
restore_sha="$(git -C "$REPO" rev-parse HEAD)" # push 直後の（署名なしの）先頭
FAIL_ON=graphql run_resign resign_mutation_failure
[[ "$RC" != 0 ]] || failures+=("resign_mutation_failure: exit 0, want non-zero")
# 巻き戻し（PATCH で base）→ 失敗 → 復元（PATCH で元の先頭）の順で PATCH が 2 回
patches=$(grep -c -- '-X PATCH' "$CALL_LOG" || true)
[[ "$patches" == 2 ]] || failures+=("resign_mutation_failure: $patches PATCHes, want 2 (巻き戻し+復元)")
[[ "$(git -C "$REMOTE_DIR" rev-parse refs/heads/agent/test-branch)" == "$restore_sha" ]] ||
  failures+=("resign_mutation_failure: remote ref not restored")

# --- 6. 失敗のあと再実行すると残りを署名して追いつく ---
run_resign resign_retry
[[ "$RC" == 0 ]] || failures+=("resign_retry: exit $RC, want 0 ($(cat "$ROOT/resign_retry.err"))")
git -C "$REMOTE_DIR" cat-file -p "$(git -C "$REMOTE_DIR" rev-parse refs/heads/agent/test-branch)" | grep -q '^gpgsig ' ||
  failures+=("resign_retry: tip still unsigned")
[[ "$(git -C "$REPO" rev-parse HEAD)" == "$(git -C "$REMOTE_DIR" rev-parse refs/heads/agent/test-branch)" ]] ||
  failures+=("resign_retry: local not aligned")

# --- 7. 単一引数上限（128KiB）を超える大きいファイルを含むコミットも署名できる ---
head -c 200000 /dev/zero | tr '\0' 'a' >"$REPO/big.txt"
git -C "$REPO" add big.txt
git -C "$REPO" commit -qm "feat: big file"
git -C "$REPO" push -q origin HEAD
run_resign resign_big_file
[[ "$RC" == 0 ]] || failures+=("resign_big_file: exit $RC, want 0 ($(head -c 200 "$ROOT/resign_big_file.err"))")
git -C "$REMOTE_DIR" cat-file -p "$(git -C "$REMOTE_DIR" rev-parse refs/heads/agent/test-branch)" | grep -q '^gpgsig ' ||
  failures+=("resign_big_file: tip unsigned")
[[ "$(git -C "$REMOTE_DIR" show refs/heads/agent/test-branch:big.txt | wc -c)" == 200000 ]] ||
  failures+=("resign_big_file: big.txt content differs")

# --- 8. リポジトリ外では何もせず終了コード 0 ---
(cd "$ROOT" && GH_TOKEN=fake-token bash "$SCRIPT" >/dev/null 2>&1)
[[ "$?" == 0 ]] || failures+=("outside_repo: non-zero exit outside repo")

if ((${#failures[@]})); then
  printf 'FAIL %s\n' "${failures[@]}"
  exit 1
fi
echo ok
