#!/usr/bin/env bash
# ルーティンの安全制限（Issue #194）が仕様に落ちているかのテスト。
# 対象は prose の差分なので、各受け入れ条件を grep したときの必須文言で確かめる。
# 文言を変えるときはこのテストと同じ PR で直す。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
failures=()

# check <name> <file> <pattern...> — すべての pattern が file にあれば合格。
check() {
  local name="$1" file="$2"
  shift 2
  local p
  for p in "$@"; do
    if ! grep -qF "$p" "$ROOT/$file"; then
      failures+=("$name: $file に無い: $p")
    fi
  done
}

# 条件1: 制限値（最短間隔・件数上限・連続失敗の自動停止回数）が決まり、仕様に明記されている。
check cond1_min_interval docs/prd.md \
  '最短実行間隔' \
  '15分'
check cond1_max_active docs/prd.md \
  '有効な件数の上限' \
  'AI Bot 1つにつき20件'
check cond1_auto_stop docs/prd.md \
  '連続失敗の自動停止' \
  '10回連続'

# 条件2: 自動停止したときに利用者へ知らせる方法が書かれている
# （チャンネルへの1行表示、判断待ちの一覧への掲載、人の確認による再開）。
check cond2_notify_line docs/prd.md \
  '止めた旨と最後の失敗の理由を1行'
check cond2_notify_list docs/prd.md \
  '判断待ちの一覧' \
  '「今日」画面'
check cond2_resume_by_human docs/prd.md \
  '失敗の原因を確認してから'

# 条件3: 制限を超える設定を依頼された場合の AI の振る舞い（断る／上限を提示する）が書かれている。
check cond3_refuse docs/prd.md \
  '制限を超えたルーティンを作らない' \
  '制限値を依頼者に伝え'
check cond3_alternative docs/prd.md \
  '範囲内に収まる代替'
check cond3_no_silent_fix docs/prd.md \
  '黙って間隔を広げ'

# 出典と判断の記録: ADR 0011（OpenBot 出発の選定理由）が存在し、PRD から参照されている。
# ADR 側は比較（OpenBot/Rakazo）・時刻解釈（ADR 0010）・初期値を明記する。
if [[ ! -f "$ROOT/docs/adr/0011-routine-safety-limits.md" ]]; then
  failures+=("adr_0011: docs/adr/0011-routine-safety-limits.md が存在しない")
fi
check adr_0011_comparison docs/adr/0011-routine-safety-limits.md \
  'OpenBot' \
  'ADR 0010' \
  '10回連続'
check prd_links_adr docs/prd.md \
  'adr/0011-routine-safety-limits.md'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "routine-safety-docs: ok"
