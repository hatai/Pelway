#!/usr/bin/env bash
# 技術仕様の出口検査節（Issue #196）の必須文言を固定するテスト（wording pin）。
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
check cond2_block docs/technical.md \
  '投稿を止める'
check cond2_generic_bounce docs/technical.md \
  '汎用の差し戻し'
check cond2_notify_human docs/technical.md \
  '人間の画面' \
  '監査ログ'
check cond2_no_contradiction_with_192 docs/technical.md \
  '#192'

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

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "reply-guard-docs: ok"
