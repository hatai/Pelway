#!/usr/bin/env bash
# scripts/actions-lock-check.sh のテスト（Issue #838）。実際の jactionlint を一時の git
# repository に対して実行し、lock の coverage・dependencies・YAML 解析失敗・bypass 防止を確かめる。
set -uo pipefail

SCRIPT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/actions-lock-check.sh"
failures=()

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
fixture="$work/repo"
mkdir -p "$work/home" "$work/xdg-config"
git init -q "$fixture"
mkdir -p "$fixture/.github/workflows"

commit_sha="3d3c42e5aac5ba805825da76410c181273ba90b1"

write_lock() {
  cat >"$fixture/.github/workflows/actions.lock" <<EOF_LOCK
version: 'v0.0.3'
workflows:
    '.github/workflows/a.yml':
        - 'actions/checkout@v7.0.1'
dependencies:
    'actions/checkout@v7.0.1':
        ref: 'v7.0.1'
        commit: 'sha1-$commit_sha'
        owner_id: 44036562
        repo_id: 197814629
EOF_LOCK
}

write_empty_lock() {
  cat >"$fixture/.github/workflows/actions.lock" <<'EOF_LOCK'
version: 'v0.0.3'
workflows: {}
dependencies: {}
EOF_LOCK
}

write_workflow() {
  cat >"$fixture/.github/workflows/a.yml" <<'EOF_WF'
on: push
jobs:
  x:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7.0.1
EOF_WF
}

reset_fixture() {
  rm -f "$fixture/.github/workflows/new.yml" "$fixture/.github/jactionlint.yaml"
  write_lock
  write_workflow
}

reply_out=""
run_check() {
  local name="$1" want_rc="$2" rc
  reply_out="$(cd "$fixture" && HOME="$work/home" XDG_CONFIG_HOME="$work/xdg-config" \
    PATH="$PATH" bash "$SCRIPT" 2>&1)"
  rc=$?
  if [[ "$rc" != "$want_rc" ]]; then
    failures+=("$name: exit $rc, want $want_rc: $reply_out")
  fi
}

want_contains() {
  [[ "$2" == *"$3"* ]] || failures+=("$1: missing \"$3\": $reply_out")
}

# lock どおりなら成功する。
reset_fixture
run_check baseline 0

# workflow が使う action が lock の一覧に無ければ失敗する。
reset_fixture
cat >>"$fixture/.github/workflows/a.yml" <<'EOF_WF'
      - uses: actions/cache@v6.1.0
EOF_WF
run_check missing-action 1
want_contains missing-action "$reply_out" '[actions-lock]'

# 一覧にあっても dependencies の entry が無ければ失敗する。
reset_fixture
python3 -I - "$fixture/.github/workflows/actions.lock" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
s = s.split('dependencies:')[0] + 'dependencies: {}\n'
open(p, 'w').write(s)
PY
run_check missing-dependency 1
want_contains missing-dependency "$reply_out" 'has no entry under "dependencies"'

# dependencies の固定コミットの形式が不正なら失敗する。
reset_fixture
sed -i "s/sha1-$commit_sha/sha1-xyz/" "$fixture/.github/workflows/actions.lock"
run_check malformed-commit 1
want_contains malformed-commit "$reply_out" 'is not a digest'

# 一覧から外れ、dependencies だけが残った状態は完全一致でないため失敗する。
reset_fixture
sed -i "/        - 'actions\\/checkout@v7.0.1'/d" "$fixture/.github/workflows/actions.lock"
run_check inconsistent-lock 1
want_contains inconsistent-lock "$reply_out" '[actions-lock]'

# lock に onboard されていない新しい workflow は失敗する。
reset_fixture
cp "$fixture/.github/workflows/a.yml" "$fixture/.github/workflows/new.yml"
run_check new-workflow 1
want_contains new-workflow "$reply_out" '[unpinned-uses]'

# inline ignore があっても --no-ignores で失敗する。
reset_fixture
cat >"$fixture/.github/workflows/new.yml" <<'EOF_WF'
on: push
jobs:
  x:
    runs-on: ubuntu-latest
    steps:
      # jactionlint ignore=unpinned-uses
      - uses: actions/checkout@v7.0.1
EOF_WF
run_check inline-ignore 1
want_contains inline-ignore "$reply_out" '[unpinned-uses]'

# 設定ファイルで rules を off にしても、明示した rule 選択による検査は止められない。
reset_fixture
cp "$fixture/.github/workflows/a.yml" "$fixture/.github/workflows/new.yml"
cat >"$fixture/.github/jactionlint.yaml" <<'EOF_CFG'
rules:
  actions-lock: off
  unpinned-uses: off
  yaml-syntax: off
EOF_CFG
run_check config-off 1
want_contains config-off "$reply_out" '[unpinned-uses]'

# 設定ファイルで rule を警告に下げても、controlled な空の設定を使うため失敗する。
reset_fixture
cp "$fixture/.github/workflows/a.yml" "$fixture/.github/workflows/new.yml"
cat >"$fixture/.github/jactionlint.yaml" <<'EOF_CFG'
rules:
  actions-lock: warn
  unpinned-uses: warn
  yaml-syntax: warn
EOF_CFG
run_check config-warn 1
want_contains config-warn "$reply_out" '[unpinned-uses]'

# 設定ファイルの policy で tag を許可しても、controlled な空の設定を使うため失敗する。
reset_fixture
cp "$fixture/.github/workflows/a.yml" "$fixture/.github/workflows/new.yml"
cat >"$fixture/.github/jactionlint.yaml" <<'EOF_CFG'
rules:
  unpinned-uses:
    policies:
      "*": any
EOF_CFG
run_check config-policy 1
want_contains config-policy "$reply_out" '[unpinned-uses]'

# reusable workflow は actions.lock に表現できないため、タグ参照は失敗する。
reset_fixture
cat >>"$fixture/.github/workflows/a.yml" <<'EOF_WF'
  call:
    uses: octo/reusable/.github/workflows/r.yml@v1
EOF_WF
run_check reusable-tag 1
want_contains reusable-tag "$reply_out" '[unpinned-uses]'

# workflow の YAML が壊れていれば、lock の rules だけでなく yaml-syntax で失敗する。
reset_fixture
cat >"$fixture/.github/workflows/a.yml" <<'EOF_WF'
on: push
jobs:
  x:
    runs-on: ubuntu-latest
    steps:
      - uses: [broken
EOF_WF
run_check malformed-workflow 1
want_contains malformed-workflow "$reply_out" '[yaml-syntax]'

# actions.lock の YAML が壊れていれば失敗する。
reset_fixture
printf 'not yaml: [\n' >"$fixture/.github/workflows/actions.lock"
run_check malformed-lock 1
want_contains malformed-lock "$reply_out" '[actions-lock]'

# docker イメージの mutable tag は digest 固定でないため失敗する。
reset_fixture
cat >>"$fixture/.github/workflows/a.yml" <<'EOF_WF'
      - uses: docker://alpine:3.20
EOF_WF
run_check docker-mutable 1
want_contains docker-mutable "$reply_out" '[unpinned-uses]'

# 完全 SHA・digest・local path は immutable または同じリポジトリの中身なので lock 無しで通る。
write_empty_lock
cat >"$fixture/.github/workflows/a.yml" <<EOF_WF
on: push
jobs:
  x:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@$commit_sha
      - uses: ./local-action
      - uses: docker://alpine@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
  call:
    uses: octo/reusable/.github/workflows/r.yml@$commit_sha
EOF_WF
run_check direct-pins 0

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}"
  exit 1
fi
echo "actions-lock-check.test.sh: all cases passed"
