#!/usr/bin/env bash
# demo-check の自己テスト。mktemp の一時ディレクトリに合成データの小さなツリーを作り、
#   - 違反のないツリーで 3 段階とも exit 0
#   - 違反を 1 つ仕込むごとに該当段階が exit 1 で「ファイル:行」を出す
#   - 除外対象（docs/・tests/fixtures/・Rust のコメント中の UI 語・xmlns）では失敗しない
#   - 秘密に見える値を出力しない
# を確かめる。本物のリスト（forbidden-phrases.txt / allowed-hosts.txt）を使う。リポジトリには書き込まない。
set -u
export LC_ALL=C.UTF-8
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "${tmp}"' EXIT

pass=0
fail=0
ok() { echo "ok   - $1"; pass=$((pass + 1)); }
ng() { echo "FAIL - $1"; fail=$((fail + 1)); }

make_clean() {
  local r="$1"
  rm -rf "${r}"
  mkdir -p "${r}/crates/x/src" "${r}/crates/x/tests/fixtures" "${r}/assets" "${r}/docs"
  printf '[env]\nPELWAY_BIND_ADDR = "127.0.0.1:0"\n' > "${r}/mise.toml"
  cat > "${r}/crates/x/src/lib.rs" <<'RS'
// 安全・安心・暗号化済み という語はコメントなら copy 規則の対象外
pub const ADDR: &str = "127.0.0.1:0";
pub const URL: &str = "http://127.0.0.1:8080/x";
pub const URL6: &str = "http://[::1]:3000/";
pub const HOST: &str = "localhost";
pub const N: &str = "1024 hours notification";
RS
  printf '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"><a href="http://localhost:1/">x</a></svg>\n' > "${r}/assets/logo.svg"
  printf 'GDPR準拠 安全 https://example.com 0.0.0.0 PELWAY_LLM_API_KEY\n' > "${r}/docs/excluded.md"
  printf 'GDPR準拠 安全 https://example.com 0.0.0.0 PELWAY_LLM_API_KEY\n' > "${r}/crates/x/tests/fixtures/excluded.txt"
}

# expect <stage> <exit> <label> <file-relative-path> <content...>  (content は printf '%b')
expect_case() {
  local stage="$1" want="$2" label="$3" rel="$4" content="$5" mode="${6:-append}"
  local r="${tmp}/case" out rc
  make_clean "${r}"
  if [ -n "${rel}" ]; then
    mkdir -p "$(dirname "${r}/${rel}")"
    if [ "${mode}" = "replace" ]; then printf '%b' "${content}" > "${r}/${rel}"; else printf '%b' "${content}" >> "${r}/${rel}"; fi
  fi
  out="$(DEMO_CHECK_ROOT="${r}" DEMO_CHECK_LIST_DIR="${here}" bash "${here}/${stage}.sh" 2>&1)"
  rc=$?
  if [ "${rc}" -ne "${want}" ]; then
    ng "${stage}: ${label} (exit ${rc}, want ${want})"; printf '%s\n' "${out}" | sed 's/^/       | /'; return
  fi
  if [ "${want}" -eq 1 ] && ! printf '%s\n' "${out}" | grep -qF "${rel}:"; then
    ng "${stage}: ${label} (no ${rel}:<line> in output)"; printf '%s\n' "${out}" | sed 's/^/       | /'; return
  fi
  if printf '%s\n' "${out}" | grep -q "SYNTHETIC-SECRET"; then
    ng "${stage}: ${label} (secret-like value printed)"; return
  fi
  ok "${stage}: ${label}"
}

# --- 合格するべきツリー ---
for s in loopback leak-phrases external-refs; do expect_case "${s}" 0 "clean tree passes" "" ""; done
out="$(DEMO_CHECK_ROOT="${tmp}/case" bash "${here}/loopback.sh" 2>&1)"
if printf '%s\n' "${out}" | grep -q "runtime: PENDING"; then ok "loopback: runtime PENDING is reported"; else ng "loopback: runtime PENDING is reported"; fi

# --- leak-phrases ---
R=crates/x/src/v.rs
expect_case leak-phrases 1 "GDPR準拠 in a Rust comment (all scope)" "${R}" '// GDPR準拠\n'
expect_case leak-phrases 1 "GDPR compliant, mixed case, in HTML" assets/v.html '<p>GDPR Compliant</p>\n'
expect_case leak-phrases 1 "暗号化済み in a Rust string" "${R}" 'pub const A: &str = "データは暗号化済みです";\n'
expect_case leak-phrases 1 "安全 in a JSON locale" assets/v.json '{"k": "安全です"}\n'
expect_case leak-phrases 1 "安心 in a text asset" assets/v.txt '安心してください\n'
expect_case leak-phrases 1 "完全に削除 in a Rust string" "${R}" 'pub const B: &str = "完全に削除しました";\n'
expect_case leak-phrases 1 "72時間 ... 通知 in HTML" assets/v.html '<p>72時間以内に通知します</p>\n'
expect_case leak-phrases 1 "notify ... 24 hours in HTML" assets/v.html '<p>We will notify you within 24 hours.</p>\n'
expect_case leak-phrases 1 "PELWAY_LLM_API_KEY referenced (value not printed)" "${R}" 'fn k() { let _ = std::env::var("PELWAY_LLM_API_KEY"); } // sk-SYNTHETIC-SECRET-123\n'
expect_case leak-phrases 1 "GDPR compliant inside a .wasm artifact" target/wasm32-unknown-unknown/debug/x.wasm '\x00asm\x01\x00\x00\x00GDPR compliant\x00'
expect_case leak-phrases 0 "安全 in a Rust comment only" "${R}" '// 安全な実装にする\n'

# --- external-refs ---
expect_case external-refs 1 "https URL in a Rust string" "${R}" 'pub const U: &str = "https://api.example.com/v1?token=SYNTHETIC-SECRET";\n'
expect_case external-refs 1 "protocol-relative CSS import" assets/v.css '@import url(//fonts.example.net/css);\n'
expect_case external-refs 1 "bare host literal" "${R}" 'pub const H: &str = "fonts.googleapis.com";\n'
expect_case external-refs 1 "wss URL with port" "${R}" 'pub const W: &str = "wss://evil.example.org:443/x";\n'
expect_case external-refs 1 "IPv6 non-loopback URL" "${R}" 'pub const V6: &str = "http://[2001:db8::1]:80/";\n'
expect_case external-refs 1 "IPv4 non-loopback literal" "${R}" 'pub const IP: &str = "203.0.113.5";\n'
expect_case external-refs 1 "URL inside a .wasm artifact" target/wasm32-unknown-unknown/release/x.wasm '\x00asm\x01\x00\x00\x00https://cdn.example.com/a.js\x00'
expect_case external-refs 0 "loopback URLs and xmlns only" "${R}" 'pub const L: &str = "http://localhost:9/";\n'

# --- loopback ---
expect_case loopback 1 "PELWAY_BIND_ADDR = 0.0.0.0 in mise.toml" mise.toml '[env]\nPELWAY_BIND_ADDR = "0.0.0.0:8080"\n' replace
expect_case loopback 1 "PELWAY_BIND_ADDR missing in mise.toml" mise.toml '[env]\nOTHER = "1"\n' replace
expect_case loopback 1 "PELWAY_BIND_ADDR=localhost in .env" .env 'PELWAY_BIND_ADDR=localhost:80\n'
expect_case loopback 1 "bind 0.0.0.0 in Rust" "${R}" 'fn b() { let _ = std::net::TcpListener::bind("0.0.0.0:80"); }\n'
expect_case loopback 1 "Ipv6Addr::UNSPECIFIED in Rust" "${R}" 'use std::net::Ipv6Addr; const A: Ipv6Addr = Ipv6Addr::UNSPECIFIED;\n'
expect_case loopback 1 "[::]:port in a config asset" assets/v.json '{"listen": "[::]:8080"}\n'

echo "selftest: ${pass} passed, ${fail} failed"
[ "${fail}" -eq 0 ]
