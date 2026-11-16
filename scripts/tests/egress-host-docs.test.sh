#!/usr/bin/env bash
# 実行環境の egress 制限のホスト側担保（Issue #205）が技術仕様に落ちているかのテスト。
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

# 条件1: egress 制限をホスト側で担保する方式（許可先の管理方法を含む）が技術仕様に明記されている。
# ホスト側は DOCKER-USER / INPUT チェーンの iptables 規則でブロックリスト方式により担保し、
# 許可先（allowlist）はアプリ側が持つ。永続化は systemd。
check cond1_host_firewall docs/technical.md \
  'DOCKER-USER' \
  'INPUT' \
  'ブロックリスト' \
  '169.254.169.254' \
  'systemd'
check cond1_allowlist_owner docs/technical.md \
  '許可先の一覧はアプリ側の送信先制御の規則だけが持ち、ホスト側では管理しない'

# 条件2: 制限が効いていることを smoke テストで検証する観点が定義されている
# （オフラインの規則検査と実機プローブ、#183 のテスト分類との接続）。
check cond2_smoke_offline docs/technical.md \
  'オフラインの規則検査'
check cond2_smoke_probe docs/technical.md \
  '実機プローブ'
check cond2_smoke_183 docs/technical.md \
  '#183'

# 条件3: アプリ側の規則（#182）と矛盾しないことが書かれている（層の分担と多層防御）。
check cond3_no_conflict docs/technical.md \
  '#182' \
  '多層'

# 条件4: 後続 Issue #389 との分担の注記（人間の判断 B、2026-11-16）。
# runtime 前提が Linux Docker Engine であること、#389 が入るまでホスト側はブロックリストであること、
# v0.7 の既定拒否は #389 で扱うこと。
check cond4_followup_389 docs/technical.md \
  '#389' \
  '既定拒否' \
  'rootless Podman'

# 出典の明記（採用する方式が Rakazo 由来であること）と、外部提供元の未決事項の記録。
check source_rakazo docs/technical.md \
  'Rakazo'
check open_external_provider docs/technical.md \
  '提供元が外部'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "egress-host-docs: ok"
