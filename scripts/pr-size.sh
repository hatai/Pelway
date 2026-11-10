#!/usr/bin/env bash
# PR の差分行数（追加 + 削除）を数え、上限を超えたら失敗する。
# base 側の .gitattributes で linguist-generated を付けたファイルとバイナリは数えない。
# 使い方: scripts/pr-size.sh [--base <ref>] [--limit <n>]（既定: origin/main, 800）
# 終了コード: 0 = 上限以下、1 = 上限超え、2 = 引数の誤り
set -euo pipefail

base="origin/main"
limit=800

usage() { echo "usage: $0 [--base <ref>] [--limit <n>]" >&2; exit 2; }

while (($#)); do
  case "$1" in
    --base) (($# >= 2)) || usage; base="$2"; shift 2 ;;
    --limit) (($# >= 2)) || usage; limit="$2"; shift 2 ;;
    *) usage ;;
  esac
done
[[ "$limit" =~ ^[0-9]+$ ]] || usage
if ! git rev-parse --verify --quiet "$base^{commit}" >/dev/null; then
  echo "pr-size: base $base が見つかりません（git fetch origin main を先に実行する）" >&2
  exit 2
fi

is_generated() {
  local value
  # 属性は base 側の .gitattributes から読む。PR 自身の .gitattributes で自分を除外させない。
  value="$(git check-attr --source "$base" linguist-generated -- "$1" | sed 's/^.*: linguist-generated: //')"
  [[ "$value" == "set" || "$value" == "true" ]]
}

total=0
per_file=()

# -z 形式: 通常は "add<TAB>del<TAB>path\0"、リネームは "add<TAB>del<TAB>\0old\0new\0"。
while IFS= read -r -d '' record; do
  added="${record%%$'\t'*}"
  rest="${record#*$'\t'}"
  deleted="${rest%%$'\t'*}"
  path="${rest#*$'\t'}"
  if [[ -z "$path" ]]; then
    IFS= read -r -d '' _old
    IFS= read -r -d '' path
  fi
  [[ "$added" == "-" ]] && continue
  is_generated "$path" && continue
  n=$((added + deleted))
  total=$((total + n))
  per_file+=("$n"$'\t'"$path")
done < <(git diff --numstat -z -M "$base...HEAD")

echo "pr-size: $total / $limit lines"

if ((total > limit)); then
  echo "PR の差分が上限を超えています。行数の多いファイル:" >&2
  printf '%s\n' "${per_file[@]}" | sort -rn | head -n 10 >&2
  exit 1
fi
