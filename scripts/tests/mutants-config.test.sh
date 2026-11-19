#!/usr/bin/env bash
# Issue #437 の受け入れ条件を確かめる検査（ミューテーションテスト A）。
# - mise.toml の [tools] に cargo-mutants が版固定で入っている
# - test_tool = "nextest" で回すために cargo-nextest も版固定で入っている
# - mise run mutants / mise run mutants:diff の 2 タスクがある
# - 設定は .cargo/mutants.toml 1 か所にまとまり、--offline と gitignore 除外が
#   宣言されている（安全条件: ネットワークに出ない・gitignore 済みの秘密ファイルを
#   一時ディレクトリにコピーしない）
# - 対象は examine_globs で pelway-contract と承認ポリシー
#   （pelway-client-core/src/approval.rs の承認カード状態機械）に絞られている
# - 実行環境から PELWAY_LLM_API_KEY を除く（安全条件）
# - #[mutants::skip] / --include-ignored は使わない
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MISE="$ROOT/mise.toml"
CONF="$ROOT/.cargo/mutants.toml"
failures=()

need() {
  local name="$1" file="$2" pattern="$3"
  if ! grep -Eq "$pattern" "$file" 2>/dev/null; then
    failures+=("$name: $file に /$pattern/ が無い")
  fi
}

deny() {
  local name="$1" file="$2" pattern="$3"
  if grep -Eq "$pattern" "$file" 2>/dev/null; then
    failures+=("$name: $file に /$pattern/ がある")
  fi
}

need "cargo-mutants pin" "$MISE" '"github:sourcefrog/cargo-mutants"[[:space:]]*=[[:space:]]*"[0-9]+\.[0-9]+\.[0-9]+"'
need "cargo-nextest pin" "$MISE" '"github:nextest-rs/nextest"[[:space:]]*='
need "mutants task" "$MISE" '\[tasks\.mutants\]'
need "mutants:diff task" "$MISE" '\[tasks\."mutants:diff"\]'
need "mutants:diff uses --in-diff" "$MISE" 'in-diff'

[[ -f "$CONF" ]] || failures+=("config: $CONF が無い")
if [[ -f "$CONF" ]]; then
  need "nextest" "$CONF" 'test_tool[[:space:]]*=[[:space:]]*"nextest"'
  need "offline" "$CONF" 'additional_cargo_args.*--offline'
  need "gitignore copy filter" "$CONF" 'gitignore[[:space:]]*=[[:space:]]*true'
  need "scope pelway-contract" "$CONF" 'pelway-contract'
  need "scope approval policy" "$CONF" 'pelway-client-core/src/approval\.rs'
  deny "no include-ignored" "$CONF" 'include.ignored'
  deny "no mutants::skip in config" "$CONF" 'mutants::skip'
fi

if grep -rEq "mutants::skip" "$ROOT/crates" 2>/dev/null; then
  failures+=("no mutants::skip: crates/ 内に mutants::skip がある")
fi
need "task unsets PELWAY_LLM_API_KEY" "$MISE" 'env -u PELWAY_LLM_API_KEY'

if ((${#failures[@]})); then
  printf 'FAIL %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "mutants-config.test.sh: ok"
