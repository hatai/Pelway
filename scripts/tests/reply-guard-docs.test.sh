#!/usr/bin/env bash
# 技術仕様の出口検査節（Issue #196。#440 で round 4 レビューの must-fix を反映）の
# 必須文言を固定するテスト（wording pin）。
# 位置づけ: このスクリプトは「必須文言が docs/technical.md に存在する」という観測可能な
# 事実だけを確かめる決定的な検査であり、受け入れ条件の意味や仕様の整合性は評価しない
# （AGENTS.md「LLM judges」: 自然言語の意味の評価は rubric 付きの LLM judge が担い、
# 決定的な検査は観測可能な効果に限る。語句が残っていれば矛盾した仕様でも通るため、
# 意味の確認は Issue #196 の受け入れ条件どおり目視で行う）。
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
  local name="$1" file="$2" pattern="$3"
  if grep -qF "$pattern" "$ROOT/$file"; then
    failures+=("$name: $file に残っている: $pattern")
  fi
}

# 条件1: 出口検査の対象パターン（漏洩系・形式の異常系・暴走系）の一覧が仕様に明記されている。
check cond1_classes docs/technical.md \
  '漏洩系' \
  '形式の異常系' \
  '暴走系'
check cond1_leak_patterns docs/technical.md \
  'スタックトレース' \
  'ツール呼び出しのJSON' \
  '資格情報らしき文字列' \
  'エラー文'
check cond1_form_patterns docs/technical.md \
  '文字化け' \
  'メタ発言'
check cond1_runaway_patterns docs/technical.md \
  '同一内容の繰り返し'

# 条件2: 検出時の扱い（投稿を止める、AI に汎用的に差し戻す、人へ知らせる）が決まっていて、
# #192 の却下理由の規則（詳細は人間の画面と監査のみ、AI へは汎用の拒否のみ）と矛盾しない。
# round 4（hatai の例外決定・2026-11-17）: 監査ログには元の文を残さず理由コードと
# ハッシュだけを残す。
check cond2_block docs/technical.md \
  '投稿を止める'
check cond2_generic_bounce docs/technical.md \
  '汎用の差し戻し'
check cond2_notify_human docs/technical.md \
  '人間の画面'
check cond2_audit_log docs/technical.md \
  '監査ログ' \
  '理由コード' \
  '元の文を残しません'
check cond2_no_contradiction_with_192 docs/technical.md \
  '#192'

# 条件1の round 4 追加分（hatai の例外決定）: 添付の中身も検査対象にする。
# 取り出せない添付と大きすぎる添付は止める（fail-closed）。
check cond1_attachments docs/technical.md \
  '添付の内容' \
  '大きすぎる添付'

# round 4 の暗号化添付の規則: 人のアップロードだけ注記付きで通す。Bot の投稿は
# 人のファイルを変えずに転送して hash が一致したときだけ通す。
check cond4_encrypted docs/technical.md \
  '暗号化された添付' \
  'hashが一致'

# 条件3: 正当な発言（コードやログを扱う仕事）を誤って止めないための扱い
# （チャンネルや役割による適用範囲）が書かれている。
check cond3_scope docs/technical.md \
  'チャンネル' \
  '役割による適用範囲'
check cond3_relief docs/technical.md \
  '緩和'

# 出典の明記（採用する方式が CircleChat 由来であること）。
check source_circlechat docs/technical.md \
  'CircleChat'

# #440（PR #414 round 4 レビューの must-fix 反映）: 漏洩系のうち誤検出の少ないパターンは
# 発言の文中一致でも検出する（先頭固定・発言全体の条件は漏洩系には適用しない。
# 添付の内容も同じ規則）。
check cond440_leak_midtext docs/technical.md \
  '発言の文中一致でも検出します' \
  '漏洩系には適用しません' \
  '同じ規則です'
# #440: フェンス囲みの短いコード片・ログの引用の除外は、形式の異常系の大量貼り付けの
# 検査に限定（漏洩系・暴走系の検査は免除しない）。
check cond440_fence_scope docs/technical.md \
  '大量貼り付けの検査から外します' \
  '漏洩系・暴走系の検査は免除しません'
# #440: 監査ログの記録項目は理由コードと本文・添付の内容hashだけに縮小
# （時刻・対象チャンネル・Bot は記録しない）。
check cond440_audit_minimal docs/technical.md \
  '本文と添付の内容hashだけ'
check_absent cond440_audit_no_ops_metadata docs/technical.md \
  '・時刻・対象チャンネル・Botだけ'
# #440: 暗号化添付の hash の比較対象を未決事項に足す。
check cond440_hash_compare_target docs/technical.md \
  '暗号化添付の「hashが一致」の比較対象'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "reply-guard-docs: ok"
