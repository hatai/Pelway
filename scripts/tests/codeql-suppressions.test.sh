#!/usr/bin/env bash
# CodeQL アラート 32 件(Issue #301)の検証。方式 (c): 抑制コメントは使わない。
# 経緯: 行末の // codeql[rust/...] はこのリポジトリの CodeQL(デフォルト
# セットアップ)では Rust のインライン抑制として処理されない実測(PR #313
# のスキャンで、抑制コメント挿入行と同一行から 32 件のアラートが出た)。
# そのため解消はマージ後に指揮役が API で 32 件を理由付き dismiss する
# (dismissed_reason は本番コードの誤検出が false positive、テストコードの
# 固定値が used in tests)。リポジトリ側の記録(場所・分類・理由)は
# docs/worklogs/issue-301.md の対応表が持つ。
# このテストは Issue #301 の受け入れ条件(差し替え後)のうち次を確かめる:
#   1. アラート 32 件(行としては 30 箇所)のコード断片が変わっていない
#      (テストベクトル等の固定値の不改変。dismiss の対象を固定する)
#   2. 対象 8 ファイルに抑制コメント(codeql[...])が残っていない
# 断片は行番号に依らずファイル内で一意に位置を特定できる部分(行全体)。
# コードが変わると断片が見つからなくなり失敗する(固定値の保護)。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
failures=()

# check <path> <アラート番号> <コード断片>
check() {
  local path="$1" num="$2" snippet="$3"
  local file="$ROOT/$path"
  if [[ ! -f "$file" ]]; then
    failures+=("#$num: ファイルが無い: $path")
    return
  fi
  local hits
  hits="$(grep -cF "$snippet" "$file" || true)"
  if [[ "$hits" != 1 ]]; then
    failures+=("#$num: コード断片が一意に定まらない($hits 行): $path: $snippet")
  fi
}

# ---- 対応表: Issue #301 の全 32 件(行単位 30。#15/#16 と #17/#18 は同一行)。
# 形式: path<TAB>アラート番号<TAB>コード断片
while IFS=$'\t' read -r path num snippet; do
  [[ -z "$path" || "$path" == \#* ]] && continue
  check "$path" "$num" "$snippet"
done <<'EOF'
crates/pelway-at-rest/src/lib.rs	27	let mut wrap_nonce = [0u8; NONCE_LEN];
crates/pelway-at-rest/src/lib.rs	28	let mut payload_salt = [0u8; SALT_LEN];
crates/pelway-at-rest/src/lib.rs	29	const SALT: [u8; SALT_LEN] = [0x44; SALT_LEN];
crates/pelway-at-rest/src/lib.rs	30	seal_with_material(&master, pt, &FILE_KEY, [0x22; NONCE_LEN], SALT).unwrap()
crates/pelway-at-rest/src/lib.rs	1	assert!(try_chunk(body, chunk_nonce(0, true), header));
crates/pelway-at-rest/src/lib.rs	2	assert!(!try_chunk(body, chunk_nonce(0, true), &[]));
crates/pelway-at-rest/src/lib.rs	3	assert!(!try_chunk(body, chunk_nonce(0, true), &other));
crates/pelway-at-rest/src/lib.rs	4	assert!(try_chunk(first, chunk_nonce(0, false), header));
crates/pelway-at-rest/src/lib.rs	5	assert!(try_chunk(second, chunk_nonce(1, true), header));
crates/pelway-at-rest/src/lib.rs	6	assert!(!try_chunk(first, chunk_nonce(0, true), header));
crates/pelway-at-rest/src/lib.rs	7	assert!(!try_chunk(first, chunk_nonce(1, false), header));
crates/pelway-at-rest/src/lib.rs	8	assert!(!try_chunk(second, chunk_nonce(1, false), header));
crates/pelway-at-rest/src/lib.rs	9	assert!(!try_chunk(second, chunk_nonce(0, true), header));
crates/pelway-at-rest/src/container.rs	26	let mut nonce = [0u8; NONCE_LEN];
crates/pelway-contract/src/pseudonym.rs	10	let p = pseudonymize(b"salt", "card-1");
crates/pelway-contract/src/pseudonym.rs	11	let base = pseudonymize(b"salt-a", "m-human-1");
crates/pelway-contract/src/pseudonym.rs	12	assert_eq!(base, pseudonymize(b"salt-a", "m-human-1"));
crates/pelway-contract/src/pseudonym.rs	13	assert_ne!(base, pseudonymize(b"salt-b", "m-human-1"));
crates/pelway-contract/src/pseudonym.rs	14	assert_ne!(base, pseudonymize(b"salt-a", "m-human-2"));
crates/pelway-contract/src/pseudonym.rs	15	assert_ne!(pseudonymize(b"ab", "c"), pseudonymize(b"a", "bc"));
crates/pelway-contract/src/pseudonym.rs	17	assert_ne!(pseudonymize(b"", "abc"), pseudonymize(b"abc", ""));
crates/pelway-contract/src/pseudonym.rs	19	let e = pseudonymize(b"", "");
crates/pelway-contract/src/pseudonym.rs	20	assert_ne!(e, pseudonymize(b"", "x"));
crates/pelway-contract/src/pseudonym.rs	21	assert_ne!(e, pseudonymize(b"x", ""));
crates/pelway-http/src/gdpr02_routes.rs	22	const SALT: &[u8] = b"gdpr02-routes-test-salt";
crates/pelway-http/src/lib.rs	23	const KNOWN_SALT: &[u8; crate::log::SALT_BYTES] = b"known-salt-for-entry-test-012345";
crates/pelway-http/src/no_body_logs.rs	24	const SALT: &[u8] = b"no-body-logs-test-salt";
crates/pelway-http/src/log.rs	25	serde_json::json!(pseudonymize(b"unit-salt", "card-unit"))
crates/pelway-llm/tests/pure.rs	31	assert!(debug.contains("redacted"), "{debug}");
crates/pelway-llm/tests/pure.rs	32	assert!(!debug.contains("sk-secret-do-not-print-1234"), "{debug}");
EOF

# ---- 抑制コメントが残っていないこと(受け入れ条件: 抑制コメントが残っていない)。
FILES=(
  crates/pelway-at-rest/src/lib.rs
  crates/pelway-at-rest/src/container.rs
  crates/pelway-contract/src/pseudonym.rs
  crates/pelway-http/src/gdpr02_routes.rs
  crates/pelway-http/src/lib.rs
  crates/pelway-http/src/log.rs
  crates/pelway-http/src/no_body_logs.rs
  crates/pelway-llm/tests/pure.rs
)
for f in "${FILES[@]}"; do
  if grep -qF 'codeql[' "$ROOT/$f"; then
    failures+=("$f: 抑制コメント(codeql[...])が残っている: $(grep -nF 'codeql[' "$ROOT/$f" | head -1 | cut -d: -f1) 行目")
  fi
done

if ((${#failures[@]})); then
  printf 'FAIL %s\n' "${failures[@]}"
  exit 1
fi
echo ok
