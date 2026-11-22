#!/usr/bin/env bash
# mise run shellcheck の本体（Issue #265・#621）。<root>/scripts 以下で 1 行目が #! で
# 始まり bash か sh を含むファイルを全部検査する。
#   - scripts/demo-check/*.sh: severity 既定（style 以上の指摘で失敗）。shebang の無い
#     もの（source される lib.sh）も含める
#   - それ以外: --severity=warning（info・style 級は対象にしない）
# 使い方: scripts/shellcheck.sh [<root>]（既定: カレントディレクトリ）
# 終了コード: 0 = 指摘 0 件、それ以外 = 指摘あり・検査不能
set -euo pipefail

root="${1:-.}"
cd "$root"

# demo-check 一式は shebang の有無によらず glob で全部拾う（source される lib.sh は
# shebang を持たないが、#265 から検査してきた）。数は shebang 一致分と別に数える。
demo=()
rest=()
demo_noshebang=0
while IFS= read -r -d '' f; do
  is_demo=0
  [[ "$f" == scripts/demo-check/*.sh && "$f" != scripts/demo-check/*/* ]] && is_demo=1
  if head -n 1 "$f" | grep -qE '^#!.*\b(bash|sh)\b'; then
    if ((is_demo)); then demo+=("$f"); else rest+=("$f"); fi
  elif ((is_demo)); then
    demo+=("$f")
    demo_noshebang=$((demo_noshebang + 1))
  fi
done < <(find scripts -type f -print0 | sort -z)

# shebang で拾った数（find の数え方と一致させる数）
total=$((${#demo[@]} - demo_noshebang + ${#rest[@]}))
if ((total == 0)); then
  echo "shellcheck: no bash/sh scripts found under $root/scripts" >&2
  exit 2
fi

rc=0
if ((${#demo[@]})); then
  shellcheck "${demo[@]}" || rc=1
fi
if ((${#rest[@]})); then
  shellcheck --severity=warning "${rest[@]}" || rc=1
fi
if ((rc)); then
  echo "shellcheck: findings above (${total} files checked)" >&2
  exit 1
fi
# CI のログに実行と成功を出すため、合格時に 1 行出す（shellcheck 自体は指摘 0 件だと何も出さない）。
echo "shellcheck: ${total} files checked (bash/sh shebang; scripts/demo-check/*.sh: $((${#demo[@]} - demo_noshebang)) at default severity, others: ${#rest[@]} at --severity=warning) + ${demo_noshebang} scripts/demo-check/*.sh without shebang at default severity, 0 findings"
