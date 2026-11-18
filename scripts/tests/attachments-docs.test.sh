#!/usr/bin/env bash
# 添付の保存の仕様（Issue #457）が技術仕様に落ちているかのテスト。
# 対象は prose の差分なので、各受け入れ条件を grep したときの必須文言で確かめる
# （wording pin。前例 reply-guard-docs.test.sh と同じ形式。文言の存在という観測可能な
# 事実だけを確かめる決定的な検査で、節の内容の評価は目視による）。
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

# check_absent <name> <file> <pattern> — pattern が file に無ければ合格。
check_absent() {
  local name="$1" file="$2" p="$3"
  if grep -qF "$p" "$ROOT/$file"; then
    failures+=("$name: $file に残っている: $p")
  fi
}

# 条件1: 「添付の保存（提案）」節があり、NewMessage・StoredMessage と DB に添付を
# 載せる形（項目・型・対応の数）が書かれている。
check cond1_section docs/technical.md \
  '添付の保存（提案）'
check cond1_model docs/technical.md \
  'NewMessage' \
  'StoredMessage' \
  'message_attachments'

# 条件2: 保存先（国内 host の同期外 volume、object_store 経由）と storage key の
# 決め方（利用者の指定したパスを使わない）。
check cond2_storage docs/technical.md \
  '同期外' \
  'object_store' \
  '利用者の指定したパスを使いません' \
  'path traversal'

# 条件3: 内容 hash の方式と、hash を計算する対象（暗号化の前か後か）。
check cond3_hash docs/technical.md \
  'SHA-256' \
  '暗号化の前'

# 条件4: 比べる hash（人のアップロード時の hash と Bot の転送物の hash）を
# どの項目どうしで比べるか。
check cond4_hash_compare docs/technical.md \
  '元の人のアップロード' \
  '添付行の content_hash'

# 条件5: 保存時の暗号化に pelway-at-rest のコンテナを使うかと、鍵の扱いは U05 未決。
check cond5_encryption docs/technical.md \
  'pelway-at-rest' \
  'U05'

# 条件6: 大きさの上限（1件・会社合計）と受け付ける種類（MIME）の決め方。
check cond6_limits docs/technical.md \
  '50MiB' \
  '10GiB' \
  'MIME'

# 条件7: 読める人の規則（source scope 継承・取得のたびの現権限検査・guest の範囲）。
check cond7_readers docs/technical.md \
  'source scope を継承' \
  '取得のたび' \
  'guest'

# 条件8: 削除・権限の取り消し・保持期限切れの扱い（方針 P・R01）。
check cond8_retention docs/technical.md \
  '365日' \
  'ベストエフォート'

# 条件9: 外部との境界（接続だけでは添付を送らない、外へ出すときは ActionSnapshot
# に hash を固定）。
check cond9_boundary docs/technical.md \
  'ActionSnapshot' \
  '接続するだけでは'

# 条件10: クライアントでの受け取りと表示の案（download を初期無効にするか等）。
check cond10_client docs/technical.md \
  'ダウンロード' \
  '最初から有効'

# 条件11: reply-guard の添付の検査（#454）をつなぐ位置（保存の前か、確定の前か）。
check cond11_reply_guard docs/technical.md \
  '#454' \
  '不変 key への移動の前'

# 条件12: 実装の Issue（#458・#459・#460・#461）の範囲への参照。
check cond12_impl_issues docs/technical.md \
  '#458' \
  '#459' \
  '#460' \
  '#461'

# 条件13: U01〜U07 に関わる点は確定させず、未決事項に U 番号つきで書く。
check cond13_open_issues docs/technical.md \
  'U01' \
  'U03' \
  'U05'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "attachments-docs: ok"
