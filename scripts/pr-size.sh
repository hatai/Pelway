#!/usr/bin/env bash
# PR の差分行数を追加行と削除行に分けて数え、どちらかの上限を超えたら失敗する。
# base 側の .gitattributes で linguist-generated を付けたファイルとバイナリは数えない。
# 使い方: scripts/pr-size.sh [--base <ref>] [--limit <n>] [--delete-limit <n>]（既定: origin/main, 800, 2000）
# 終了コード: 0 = 両方とも上限以下、1 = 上限超え、2 = 引数の誤り
set -euo pipefail

base="origin/main"
limit=800
delete_limit=2000

usage_text="usage: $0 [--base <ref>] [--limit <n>] [--delete-limit <n>]"

usage() { echo "$usage_text" >&2; exit 2; }

help() {
  cat <<EOF
$usage_text

PR の差分行数を追加行と削除行に分けて数え、どちらかの上限を超えたら失敗する。
base 側の .gitattributes で linguist-generated を付けたファイルとバイナリは数えない。

options:
  --base <ref>          比較元の ref（既定: origin/main）
  --limit <n>           追加行数の上限（既定: 800）
  --delete-limit <n>    削除行数の上限（既定: 2000）
  -h, --help            この使い方を標準出力に出して終了する

終了コード: 0 = 両方とも上限以下、1 = 上限超え、2 = 引数の誤り
EOF
  exit 0
}

while (($#)); do
  case "$1" in
    -h|--help) help ;;
    --base) (($# >= 2)) || usage; base="$2"; shift 2 ;;
    --limit) (($# >= 2)) || usage; limit="$2"; shift 2 ;;
    --delete-limit) (($# >= 2)) || usage; delete_limit="$2"; shift 2 ;;
    *) usage ;;
  esac
done
[[ "$limit" =~ ^[0-9]+$ ]] || usage
[[ "$delete_limit" =~ ^[0-9]+$ ]] || usage
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

added_total=0
deleted_total=0
per_file=()

# -z 形式: 通常は "add<TAB>del<TAB>path\0"、リネームは "add<TAB>del<TAB>\0old\0new\0"。
while IFS= read -r -d '' record; do
  added="${record%%$'\t'*}"
  rest="${record#*$'\t'}"
  deleted="${rest%%$'\t'*}"
  path="${rest#*$'\t'}"
  if [[ -z "$path" ]]; then
    IFS= read -r -d '' _ # rename の旧パス（数えない）
    IFS= read -r -d '' path
  fi
  [[ "$added" == "-" ]] && continue
  is_generated "$path" && continue
  added_total=$((added_total + added))
  deleted_total=$((deleted_total + deleted))
  per_file+=("$((added + deleted))"$'\t'"$path"$'\t'"+$added"$'\t'"-$deleted")
done < <(git diff --numstat -z -M "$base...HEAD")

echo "pr-size: +$added_total / -$deleted_total（追加の上限 $limit、削除の上限 $delete_limit）"

if ((added_total > limit || deleted_total > delete_limit)); then
  if ((added_total > limit)); then
    echo "追加行が上限（$limit）を超えています。" >&2
  fi
  if ((deleted_total > delete_limit)); then
    echo "削除行が上限（$delete_limit）を超えています。" >&2
  fi
  echo "行数の多いファイル:" >&2
  printf '%s\n' "${per_file[@]}" | sort -rn | head -n 10 | cut -f2- >&2
  exit 1
fi
