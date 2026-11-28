#!/usr/bin/env bash
# 領域別レビュー観点ファイル（Issue #794）の構造を固定するテスト。
# 位置づけ: ファイル・front matter・見出し・参照が存在することだけを確かめる決定的な検査
# （wording pin、存在の確認）であり、規則の意味や妥当性は評価しない
# （AGENTS.md「LLM judges」。意味の確認は PR 本文での引用と skill review-pr のレビューで行う）。
# 構造や対象パスを変えるときはこのテストと同じ PR で直す。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
failures=()

require_file() {
  local file="$1"
  if [[ ! -f "$ROOT/$file" ]]; then
    failures+=("missing: $file")
  fi
}

# check <name> <file> <pattern...> — すべての pattern が file にあれば合格。
check() {
  local name="$1" file="$2"
  shift 2
  if [[ ! -f "$ROOT/$file" ]]; then
    failures+=("$name: $file が無い")
    return
  fi
  local p
  for p in "$@"; do
    if ! grep -qF -- "$p" "$ROOT/$file"; then
      failures+=("$name: $file に無い: $p")
    fi
  done
}

db_file='docs/agents/review/db-contracts-and-migrations.md'
tests_file='docs/agents/review/tests.md'
ui_file='docs/agents/review/client-ui.md'

require_file "$db_file"
require_file "$tests_file"
require_file "$ui_file"

check_common() {
  local name="$1" file="$2" perspective="$3" default_class="$4"
  check "$name" "$file" \
    '---' \
    "name: $perspective" \
    'paths:' \
    "default_class: $default_class" \
    'attention: HIGH' \
    '## 報告するもの' \
    '### must-fix で報告するもの' \
    '### nit で報告するもの' \
    '## 報告しないもの' \
    '## 証拠にするもの'
}

check_common db_contracts "$db_file" 'db-contracts-and-migrations' 'must-fix'
check "$db_file-paths" "$db_file" \
  '  - /crates/pelway-db/**' \
  '  - /crates/pelway-contract/**'

check_common tests "$tests_file" 'tests' 'nit'
check "$tests_file-paths" "$tests_file" \
  '  - /crates/*/tests/**' \
  '  - /crates/dev-llm-eval/data/**' \
  '  - /client/**/*.test.ts' \
  '  - /client/**/*.test.tsx' \
  '  - /client/test/**' \
  '  - /client/tsconfig.test.json' \
  '  - /scripts/tests/**' \
  '  - /scripts/demo-check/selftest.sh' \
  '  - /scripts/script-test.sh' \
  '  - /.agents/skills/*/*.test.sh' \
  '  - /evals/**' \
  '  - /.agents/skills/*/evals/**'

check_common client_ui "$ui_file" 'client-ui' 'nit'
check "$ui_file-paths" "$ui_file" \
  '  - /crates/pelway-client-ui/**' \
  '  - /crates/pelway-client-core/**' \
  '  - /crates/dev-client-probe/**' \
  '  - /crates/**/*.slint' \
  '  - /client/apps/**' \
  '  - /client/packages/**' \
  '  - /client/test/**'

# shellcheck disable=SC2016 # Rust の test attribute を囲む backtick は文字列として確かめる
check review_pr_references .agents/skills/review-pr/SKILL.md \
  'docs/agents/review/db-contracts-and-migrations.md' \
  'docs/agents/review/tests.md' \
  'docs/agents/review/client-ui.md' \
  'front matter だけを読む' \
  '選ばれたファイルの本文だけを main の版で読む' \
  '観点ファイル: なし' \
  '`#[cfg(test)]`' \
  '`#[test]`' \
  '`#[tokio::test]`' \
  '削除前・rename 前後両方のパス' \
  'verdict: failed'

if ((${#failures[@]})); then
  printf 'FAIL %s\n' "${failures[@]}"
  exit 1
fi
echo ok
