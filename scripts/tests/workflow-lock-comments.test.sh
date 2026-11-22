#!/usr/bin/env bash
# workflow のコメントに書いた action の版が actions.lock とずれないことの検査（Issue #615）。
# action のコミット sha と版の正本は .github/workflows/actions.lock で、コメントに版を書くと
# lock を上げたときに古い写しが残る。そこで、actions.lock を含むコメント行に版番号
# （v に続く 数字.数字.数字）があれば、ファイルと行番号を出して失敗する。
# 対象は actions.lock を含むコメント行だけ（Semgrep のイメージの digest の出所のような、
# actions.lock に触れないコメントの版は対象外）。コメント行は、先頭の空白の後が # の行。
# 前半で検査そのものを一時の workflow で確かめ、後半でリポジトリの workflow を検査する。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
failures=()

# check_dir <dir>: <dir>/*.yml を検査し、違反を「<file>:<行番号>: <行>」で標準出力に出す。
# 違反があれば 1、無ければ 0 を返す。
check_dir() {
  local dir="$1" f found=0 hits
  for f in "$dir"/*.yml; do
    [[ -f "$f" ]] || continue
    hits="$(grep -nE '^[[:space:]]*#.*actions\.lock' "$f" \
      | grep -E 'v[0-9]+\.[0-9]+\.[0-9]+' || true)"
    if [[ -n "$hits" ]]; then
      found=1
      while IFS= read -r line; do
        printf '%s:%s\n' "$f" "$line"
      done <<<"$hits"
    fi
  done
  return "$found"
}

# ---- 検査の試験（一時の workflow）。
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# 版を書いた actions.lock のコメント行があると失敗し、ファイルと行番号を出す。
mkdir -p "$work/bad"
cat >"$work/bad/ci.yml" <<'EOF'
jobs:
  a:
    steps:
      - name: Checkout
        # タグ参照。コミット sha は .github/workflows/actions.lock で固定する（v7.0.1）。
        uses: actions/checkout@v7.0.1
EOF
out="$(check_dir "$work/bad")"
rc=$?
if [[ "$rc" != 1 ]]; then
  failures+=("版を書いた行で失敗しない（rc=$rc）")
fi
if [[ "$out" != "$work/bad/ci.yml:5:"* ]]; then
  failures+=("違反のファイルと行番号が出ない: $out")
fi

# 版の無い actions.lock のコメント行と、actions.lock を含まないコメント行の版は通る。
mkdir -p "$work/good"
cat >"$work/good/ci.yml" <<'EOF'
jobs:
  a:
    steps:
      - name: Checkout
        # タグ参照。コミット sha は .github/workflows/actions.lock で固定する。
        uses: actions/checkout@v7.0.1
      # semgrep/semgrep:1.186.0 の manifest list の digest（版は v1.186.0）
      - run: echo "actions.lock v1.2.3"
EOF
out="$(check_dir "$work/good")"
rc=$?
if [[ "$rc" != 0 ]]; then
  failures+=("版の無い行・actions.lock を含まない行で失敗する（rc=$rc）: $out")
fi

# ---- リポジトリの workflow の検査。
out="$(check_dir "$ROOT/.github/workflows")"
rc=$?
if [[ "$rc" != 0 ]]; then
  failures+=("actions.lock を含むコメント行に版番号がある（版は actions.lock だけに書く）:")
  while IFS= read -r line; do
    failures+=("  ${line#"$ROOT/"}")
  done <<<"$out"
fi

if ((${#failures[@]})); then
  printf 'FAIL %s\n' "${failures[@]}"
  exit 1
fi
echo ok
