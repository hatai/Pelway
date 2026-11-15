#!/usr/bin/env bash
# CodeQL アラート抑制コメント(Issue #301)の検証。
# GitHub Code Scanning の rust/hard-coded-cryptographic-value 30 件と
# rust/cleartext-logging 2 件(計 32 件。pseudonym.rs 62・63 行は各行 2 件で
# 行としては 30 箇所)について、次を確かめる:
#   1. アラート行のコード断片が変わっていない(テストベクトル等の固定値の不改変)
#   2. その行の行末にルール ID 指定の抑制コメント // codeql[rust/...] がある
#   3. 本番コード 3 件(at-rest lib.rs 119,120 / container.rs 107)は、直前の
#      理由コメントがコードの実態(os_fill による上書き / 連番から構成し再利用が
#      起きない設計)に即した語を含む
# 断片は行番号に依らずファイル内で一意に位置を特定できる部分(行全体のコード部)。
# コードが変わると断片が見つからなくなり失敗する(固定値の保護)。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
failures=()

# check <path> <アラート番号> <コード断片> <ルールID>
check() {
  local path="$1" num="$2" snippet="$3" rule="$4"
  local file="$ROOT/$path"
  if [[ ! -f "$file" ]]; then
    failures+=("#$num: ファイルが無い: $path")
    return
  fi
  local hits
  hits="$(grep -cF "$snippet" "$file" || true)"
  if [[ "$hits" != 1 ]]; then
    failures+=("#$num: コード断片が一意に定まらない($hits 行): $path: $snippet")
    return
  fi
  if ! grep -F "$snippet" "$file" | grep -qF "// codeql[$rule]"; then
    failures+=("#$num: 行末に抑制コメント // codeql[$rule] が無い: $path: $snippet")
  fi
}

# reason <path> <アラート番号> <コード断片> <理由コメントに必須な語>
# 断片を含む行とその直前 3 行の文脈に語が現れることを確かめる。
reason() {
  local path="$1" num="$2" snippet="$3" keyword="$4"
  if ! grep -F -B3 -- "$snippet" "$ROOT/$path" | grep -qF "$keyword"; then
    failures+=("#$num: 理由コメントに「$keyword」が無い(直前3行の文脈): $path")
  fi
}

# ---- 対応表: Issue #301 の全 32 件(行単位 30。#15/#16 と #17/#18 は同一行)。
# 形式: path<TAB>アラート番号<TAB>コード断片<TAB>ルールID
while IFS=$'\t' read -r path num snippet rule; do
  [[ -z "$path" || "$path" == \#* ]] && continue
  check "$path" "$num" "$snippet" "$rule"
done <<'EOF'
crates/pelway-at-rest/src/lib.rs	27	let mut wrap_nonce = [0u8; NONCE_LEN];	rust/hard-coded-cryptographic-value
crates/pelway-at-rest/src/lib.rs	28	let mut payload_salt = [0u8; SALT_LEN];	rust/hard-coded-cryptographic-value
crates/pelway-at-rest/src/lib.rs	29	const SALT: [u8; SALT_LEN] = [0x44; SALT_LEN];	rust/hard-coded-cryptographic-value
crates/pelway-at-rest/src/lib.rs	30	seal_with_material(&master, pt, &FILE_KEY, [0x22; NONCE_LEN], SALT).unwrap()	rust/hard-coded-cryptographic-value
crates/pelway-at-rest/src/lib.rs	1	assert!(try_chunk(body, chunk_nonce(0, true), header));	rust/hard-coded-cryptographic-value
crates/pelway-at-rest/src/lib.rs	2	assert!(!try_chunk(body, chunk_nonce(0, true), &[]));	rust/hard-coded-cryptographic-value
crates/pelway-at-rest/src/lib.rs	3	assert!(!try_chunk(body, chunk_nonce(0, true), &other));	rust/hard-coded-cryptographic-value
crates/pelway-at-rest/src/lib.rs	4	assert!(try_chunk(first, chunk_nonce(0, false), header));	rust/hard-coded-cryptographic-value
crates/pelway-at-rest/src/lib.rs	5	assert!(try_chunk(second, chunk_nonce(1, true), header));	rust/hard-coded-cryptographic-value
crates/pelway-at-rest/src/lib.rs	6	assert!(!try_chunk(first, chunk_nonce(0, true), header));	rust/hard-coded-cryptographic-value
crates/pelway-at-rest/src/lib.rs	7	assert!(!try_chunk(first, chunk_nonce(1, false), header));	rust/hard-coded-cryptographic-value
crates/pelway-at-rest/src/lib.rs	8	assert!(!try_chunk(second, chunk_nonce(1, false), header));	rust/hard-coded-cryptographic-value
crates/pelway-at-rest/src/lib.rs	9	assert!(!try_chunk(second, chunk_nonce(0, true), header));	rust/hard-coded-cryptographic-value
crates/pelway-at-rest/src/container.rs	26	let mut nonce = [0u8; NONCE_LEN];	rust/hard-coded-cryptographic-value
crates/pelway-contract/src/pseudonym.rs	10	let p = pseudonymize(b"salt", "card-1");	rust/hard-coded-cryptographic-value
crates/pelway-contract/src/pseudonym.rs	11	let base = pseudonymize(b"salt-a", "m-human-1");	rust/hard-coded-cryptographic-value
crates/pelway-contract/src/pseudonym.rs	12	assert_eq!(base, pseudonymize(b"salt-a", "m-human-1"));	rust/hard-coded-cryptographic-value
crates/pelway-contract/src/pseudonym.rs	13	assert_ne!(base, pseudonymize(b"salt-b", "m-human-1"));	rust/hard-coded-cryptographic-value
crates/pelway-contract/src/pseudonym.rs	14	assert_ne!(base, pseudonymize(b"salt-a", "m-human-2"));	rust/hard-coded-cryptographic-value
crates/pelway-contract/src/pseudonym.rs	15	assert_ne!(pseudonymize(b"ab", "c"), pseudonymize(b"a", "bc"));	rust/hard-coded-cryptographic-value
crates/pelway-contract/src/pseudonym.rs	17	assert_ne!(pseudonymize(b"", "abc"), pseudonymize(b"abc", ""));	rust/hard-coded-cryptographic-value
crates/pelway-contract/src/pseudonym.rs	19	let e = pseudonymize(b"", "");	rust/hard-coded-cryptographic-value
crates/pelway-contract/src/pseudonym.rs	20	assert_ne!(e, pseudonymize(b"", "x"));	rust/hard-coded-cryptographic-value
crates/pelway-contract/src/pseudonym.rs	21	assert_ne!(e, pseudonymize(b"x", ""));	rust/hard-coded-cryptographic-value
crates/pelway-http/src/gdpr02_routes.rs	22	const SALT: &[u8] = b"gdpr02-routes-test-salt";	rust/hard-coded-cryptographic-value
crates/pelway-http/src/lib.rs	23	const KNOWN_SALT: &[u8; crate::log::SALT_BYTES] = b"known-salt-for-entry-test-012345";	rust/hard-coded-cryptographic-value
crates/pelway-http/src/no_body_logs.rs	24	const SALT: &[u8] = b"no-body-logs-test-salt";	rust/hard-coded-cryptographic-value
crates/pelway-http/src/log.rs	25	serde_json::json!(pseudonymize(b"unit-salt", "card-unit"))	rust/hard-coded-cryptographic-value
crates/pelway-llm/tests/pure.rs	31	assert!(debug.contains("redacted"), "{debug}");	rust/cleartext-logging
crates/pelway-llm/tests/pure.rs	32	assert!(!debug.contains("sk-secret-do-not-print-1234"), "{debug}");	rust/cleartext-logging
EOF

# ---- 本番コード 3 件の理由コメントは実態に即していること(Issue #301 の受け入れ条件)。
# lib.rs 119,120 は「直後の os_fill で OS 乱数から上書きされる初期化」であること。
reason crates/pelway-at-rest/src/lib.rs 27 'let mut wrap_nonce = [0u8; NONCE_LEN];' 'os_fill'
reason crates/pelway-at-rest/src/lib.rs 28 'let mut payload_salt = [0u8; SALT_LEN];' 'os_fill'
# container.rs 107 は「連番から一意に構成され再利用が起きない設計」であること。
reason crates/pelway-at-rest/src/container.rs 26 'let mut nonce = [0u8; NONCE_LEN];' '再利用'

if ((${#failures[@]})); then
  printf 'FAIL %s\n' "${failures[@]}"
  exit 1
fi
echo ok
