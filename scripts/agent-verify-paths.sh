# shellcheck shell=bash
# Git のパスをバイト列に近い形で受け取るための共通関数。command substitution が
# 末尾の LF をすべて取るため sentinel を足し、Git の区切り LF 1 個だけを外す。
git_rev_path() {
  local out
  out="$(git rev-parse "$1" 2>/dev/null && printf x)" || return 1
  out="${out%x}"
  out="${out%$'\n'}"
  REPLY="$out"
}

physical_path() {
  local out
  out="$(CDPATH='' cd -- "$1" 2>/dev/null && pwd -P && printf x)" || return 1
  out="${out%x}"
  out="${out%$'\n'}"
  REPLY="$out"
}
