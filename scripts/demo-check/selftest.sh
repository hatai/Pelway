#!/usr/bin/env bash
# demo-check の自己テスト。mktemp の一時ディレクトリに合成データの小さなツリーを作り、
#   - 違反のないツリーで 4 段階とも exit 0
#   - 違反を 1 つ仕込むごとに該当段階が exit 1 で「ファイル:行」を出す
#   - 除外対象（docs/・tests/fixtures/・Rust のコメント中の UI 語・xmlns）では失敗しない
#   - 秘密に見える値（SYNTHETIC を含む合成値）を出力しない
#   - run.sh が段階名以外の引数（パス・未知の名前）を拒否し、何も実行しない
#   - 段階指定の実行が PARTIAL RUN と表示される
# を確かめる。本物のリスト（forbidden-phrases.txt / allowed-hosts.txt / canaries.txt）を使う。
# リポジトリには書き込まない。
set -u
export LC_ALL=C.UTF-8
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "${tmp}"' EXIT

pass=0
fail=0
ok() { echo "ok   - $1"; pass=$((pass + 1)); }
ng() { echo "FAIL - $1"; fail=$((fail + 1)); }
show() { printf '%s\n' "$1" | sed 's/^/       | /'; }

make_clean() {
  local r="$1"
  rm -rf "${r}"
  mkdir -p "${r}/crates/x/src" "${r}/crates/x/tests/fixtures" "${r}/assets" "${r}/docs" "${r}/scripts/demo-check"
  printf '[env]\nPELWAY_BIND_ADDR = "127.0.0.1:0"\n' > "${r}/mise.toml"
  cat > "${r}/crates/x/src/lib.rs" <<'RS'
// 安全・安心・暗号化済み・safe・secure という語はコメントなら copy 規則の対象外
// SAFETY: 合成データのみ
pub const ADDR: &str = "127.0.0.1:0";
pub const URL: &str = "http://127.0.0.1:8080/x";
pub const URL6: &str = "http://[::1]:3000/";
pub const N: &str = "1024 hours notification";
pub fn f() { let safety = 1; let _ = safety; }
RS
  printf '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"><a href="http://127.0.0.1:1/">x</a></svg>\n' > "${r}/assets/logo.svg"
  printf '{"note": "safety first is not a match for the word rule"}\n' > "${r}/assets/en.json"
  local bad='GDPR準拠 安全 safe https://example.com http://localhost 0.0.0.0 PELWAY_LLM_API_KEY PELWAY_CANARY_BODY_7f3a'
  printf '%s\n' "${bad}" > "${r}/docs/excluded.md"
  printf '%s\n' "${bad}" > "${r}/crates/x/tests/fixtures/excluded.txt"
  printf '%s\n' "${bad}" > "${r}/crates/x/tests/fixtures/excluded.log"
  printf '%s\n' "${bad}" > "${r}/scripts/demo-check/excluded.log"
}

OUT=""
RC=0
run_stage() { # run_stage <root> <stage> [VAR=value ...]
  local r="$1" stage="$2"
  shift 2
  OUT="$(env DEMO_CHECK_ROOT="${r}" DEMO_CHECK_LIST_DIR="${here}" "$@" bash "${here}/${stage}.sh" 2>&1)"
  RC=$?
}

# expect_case <stage> <exit> <label> <file-relative-path> <content (printf %b)> [append|replace]
expect_case() {
  local stage="$1" want="$2" label="$3" rel="$4" content="$5" mode="${6:-append}"
  local r="${tmp}/case"
  make_clean "${r}"
  if [ -n "${rel}" ]; then
    mkdir -p "$(dirname "${r}/${rel}")"
    if [ "${mode}" = "replace" ]; then printf '%b' "${content}" > "${r}/${rel}"; else printf '%b' "${content}" >> "${r}/${rel}"; fi
  fi
  run_stage "${r}" "${stage}"
  if [ "${RC}" -ne "${want}" ]; then
    ng "${stage}: ${label} (exit ${RC}, want ${want})"; show "${OUT}"; return
  fi
  if [ "${want}" -eq 1 ] && ! printf '%s\n' "${OUT}" | grep -qF "${rel}:"; then
    ng "${stage}: ${label} (no ${rel}:<line> in output)"; show "${OUT}"; return
  fi
  if printf '%s\n' "${OUT}" | grep -q "SYNTHETIC"; then
    ng "${stage}: ${label} (secret-like value printed)"; return
  fi
  ok "${stage}: ${label}"
}

R=crates/x/src/v.rs

# --- 合格するべきツリー ---
for s in loopback leak-phrases external-refs canary; do expect_case "${s}" 0 "clean tree passes" "" ""; done
run_stage "${tmp}/case" loopback
if printf '%s\n' "${OUT}" | grep -q "runtime: PENDING"; then ok "loopback: runtime PENDING is reported"; else ng "loopback: runtime PENDING is reported"; fi
run_stage "${tmp}/case" canary
if printf '%s\n' "${OUT}" | grep -q "no artifacts to scan"; then ok "canary: 'no artifacts to scan' is reported when nothing exists"; else ng "canary: 'no artifacts to scan' is reported"; show "${OUT}"; fi

# --- run.sh: 引数の検査と PARTIAL RUN ---
make_clean "${tmp}/case"
OUT="$(DEMO_CHECK_ROOT="${tmp}/case" bash "${here}/run.sh" 2>&1)"; RC=$?
if [ "${RC}" -eq 0 ] && ! printf '%s\n' "${OUT}" | grep -q "PARTIAL RUN" && printf '%s\n' "${OUT}" | grep -q "full run: all 4 stages"; then
  ok "run.sh: full run on a clean tree passes and is not PARTIAL"
else ng "run.sh: full run on a clean tree (exit ${RC})"; show "${OUT}"; fi

OUT="$(DEMO_CHECK_ROOT="${tmp}/case" bash "${here}/run.sh" loopback 2>&1)"; RC=$?
if [ "${RC}" -eq 0 ] && printf '%s\n' "${OUT}" | grep -q "PARTIAL RUN: not valid for acceptance" && printf '%s\n' "${OUT}" | grep -q "acceptance requires a full run"; then
  ok "run.sh: single stage prints PARTIAL RUN and the full-run requirement"
else ng "run.sh: single stage PARTIAL RUN (exit ${RC})"; show "${OUT}"; fi

printf '#!/usr/bin/env bash\ntouch "%s/pwned"\n' "${tmp}" > "${tmp}/evil.sh"
rel_evil="$(realpath --relative-to="${here}" "${tmp}/evil")"
for arg in "${rel_evil}" "../../x/ok" "/tmp/x" "nosuch" "loopback.sh" ""; do
  rm -f "${tmp}/pwned"
  OUT="$(DEMO_CHECK_ROOT="${tmp}/case" bash "${here}/run.sh" "${arg}" 2>&1)"; RC=$?
  if [ "${RC}" -eq 2 ] && [ ! -e "${tmp}/pwned" ] && ! printf '%s\n' "${OUT}" | grep -q "== demo-check:"; then
    ok "run.sh: rejects argument '${arg}' with exit 2 and runs nothing"
  else ng "run.sh: argument '${arg}' (exit ${RC}, pwned=$([ -e "${tmp}/pwned" ] && echo yes || echo no))"; show "${OUT}"; fi
done
OUT="$(DEMO_CHECK_ROOT="${tmp}/case" bash "${here}/run.sh" loopback "${rel_evil}" 2>&1)"; RC=$?
if [ "${RC}" -eq 2 ] && [ ! -e "${tmp}/pwned" ] && ! printf '%s\n' "${OUT}" | grep -q "== demo-check:"; then
  ok "run.sh: a valid stage mixed with a path is rejected before running anything"
else ng "run.sh: valid+path mix (exit ${RC})"; show "${OUT}"; fi

# --- leak-phrases ---
expect_case leak-phrases 1 "GDPR準拠 in a Rust comment (all scope)" "${R}" '// GDPR準拠\n'
expect_case leak-phrases 1 "GDPR compliant, mixed case, in HTML" assets/v.html '<p>GDPR Compliant</p>\n'
expect_case leak-phrases 1 "暗号化済み in a Rust string" "${R}" 'pub const A: &str = "データは暗号化済みです";\n'
expect_case leak-phrases 1 "安全 in a JSON locale" assets/v.json '{"k": "安全です"}\n'
expect_case leak-phrases 1 "安心 in a text asset" assets/v.txt '安心してください\n'
expect_case leak-phrases 1 "完全に削除 in a Rust string" "${R}" 'pub const B: &str = "完全に削除しました";\n'
expect_case leak-phrases 1 "Secure (case-insensitive) in a Rust string" "${R}" 'pub const C: &str = "Your data is Secure";\n'
expect_case leak-phrases 1 "safe in HTML" assets/v.html '<p>Totally safe.</p>\n'
expect_case leak-phrases 1 "encrypted in a locale" locales/en.ftl 'msg = Messages are encrypted\n'
expect_case leak-phrases 1 "fully deleted in HTML" assets/v.html '<p>Your account is fully deleted</p>\n'
expect_case leak-phrases 1 "Permanently  Deleted in a Rust string" "${R}" 'pub const D: &str = "Permanently  Deleted";\n'
expect_case leak-phrases 1 "72時間 in HTML" assets/v.html '<p>72時間以内に対応します</p>\n'
expect_case leak-phrases 1 "24時間 in a Rust string" "${R}" 'pub const E: &str = "24時間";\n'
expect_case leak-phrases 1 "24 hours in HTML" assets/v.html '<p>within 24 hours.</p>\n'
expect_case leak-phrases 1 "72-hours in a locale" locales/en.ftl 'msg = a 72-hours window\n'
expect_case leak-phrases 1 "outward doc docs/public is scanned" docs/public/help.md 'これは安全です\n'
expect_case leak-phrases 1 "PELWAY_LLM_API_KEY referenced (value not printed)" "${R}" 'fn k() { let _ = std::env::var("PELWAY_LLM_API_KEY"); } // SYNTHETICVALUE\n'
expect_case leak-phrases 1 "private key block (content not printed)" assets/v.txt '-----BEGIN RSA PRIVATE KEY-----\nSYNTHETICabc\n'
expect_case leak-phrases 1 "Bearer token (content not printed)" "${R}" 'pub const H: &str = "Authorization: Bearer SYNTHETICabcdefghijklmnop0123";\n'
expect_case leak-phrases 1 "sk- token (content not printed)" "${R}" 'pub const K: &str = "sk-SYNTHETICabcdefghijklmn_01";\n'
expect_case leak-phrases 1 "sk- token inside a .wasm artifact" target/wasm32-unknown-unknown/release/x.wasm '\x00asm\x01\x00\x00\x00sk-SYNTHETICabcdefghijklmn_01\x00'
expect_case leak-phrases 1 "GDPR compliant inside a .wasm artifact" target/wasm32-unknown-unknown/debug/x.wasm '\x00asm\x01\x00\x00\x00GDPR compliant\x00'
expect_case leak-phrases 0 "安全 / safe in Rust comments and identifiers only" "${R}" '// 安全な実装にする\nfn unsafe_free() { let safe_len = 1; let _ = safe_len; }\n'
expect_case leak-phrases 0 "short sk- / Bearer values are not secret-shaped" "${R}" 'pub const S: &str = "sk-short Bearer abc";\n'

# --- external-refs ---
expect_case external-refs 1 "https URL in a Rust string" "${R}" 'pub const U: &str = "https://api.example.com/v1?token=SYNTHETIC";\n'
expect_case external-refs 1 "http://localhost is not allowed" "${R}" 'pub const L: &str = "http://localhost:9/";\n'
expect_case external-refs 1 "ws URL" "${R}" 'pub const W: &str = "ws://chat.example.net/socket";\n'
expect_case external-refs 1 "wss URL with port" "${R}" 'pub const W: &str = "wss://evil.example.org:443/x";\n'
expect_case external-refs 1 "protocol-relative CSS import" assets/v.css '@import url(//fonts.example.net/css);\n'
expect_case external-refs 1 "protocol-relative src= in HTML" assets/v.html '<script src="//cdn.example.com/a.js"></script>\n'
expect_case external-refs 1 "bare host literal" "${R}" 'pub const H: &str = "fonts.googleapis.com";\n'
expect_case external-refs 1 "IPv6 non-loopback URL" "${R}" 'pub const V6: &str = "http://[2001:db8::1]:80/";\n'
expect_case external-refs 1 "IPv4 non-loopback literal" "${R}" 'pub const IP: &str = "203.0.113.5";\n'
expect_case external-refs 1 "URL inside a .wasm artifact" target/wasm32-unknown-unknown/release/x.wasm '\x00asm\x01\x00\x00\x00https://cdn.example.com/a.js\x00'
expect_case external-refs 0 "http://127.0.0.1 and http://[::1] and ws://127.0.0.1 pass" "${R}" 'pub const A: &str = "http://127.0.0.1:9/";\npub const B: &str = "http://[::1]:9/";\npub const C: &str = "ws://127.0.0.1:9/ws";\n'

# --- loopback ---
expect_case loopback 0 "PELWAY_BIND_ADDR = [::1]:8080 is allowed" mise.toml '[env]\nPELWAY_BIND_ADDR = "[::1]:8080"\n' replace
expect_case loopback 0 "PELWAY_BIND_ADDR = 127.0.0.1:3000 is allowed" mise.toml '[env]\nPELWAY_BIND_ADDR = "127.0.0.1:3000"\n' replace
expect_case loopback 1 "PELWAY_BIND_ADDR = 0.0.0.0 in mise.toml" mise.toml '[env]\nPELWAY_BIND_ADDR = "0.0.0.0:8080"\n' replace
expect_case loopback 1 "PELWAY_BIND_ADDR = 127.0.0.10 in mise.toml" mise.toml '[env]\nPELWAY_BIND_ADDR = "127.0.0.10:8080"\n' replace
expect_case loopback 1 "PELWAY_BIND_ADDR = localhost in mise.toml" mise.toml '[env]\nPELWAY_BIND_ADDR = "localhost:8080"\n' replace
expect_case loopback 1 "PELWAY_BIND_ADDR = ::1 without brackets" mise.toml '[env]\nPELWAY_BIND_ADDR = "::1:8080"\n' replace
expect_case loopback 1 "PELWAY_BIND_ADDR = [::] in mise.toml" mise.toml '[env]\nPELWAY_BIND_ADDR = "[::]:8080"\n' replace
expect_case loopback 1 "PELWAY_BIND_ADDR missing in mise.toml" mise.toml '[env]\nOTHER = "1"\n' replace
expect_case loopback 1 "PELWAY_BIND_ADDR=localhost in .env" .env 'PELWAY_BIND_ADDR=localhost:80\n'
expect_case loopback 1 "bind 0.0.0.0 in Rust" "${R}" 'fn b() { let _ = std::net::TcpListener::bind("0.0.0.0:80"); }\n'
expect_case loopback 1 "Ipv6Addr::UNSPECIFIED in Rust" "${R}" 'use std::net::Ipv6Addr; const A: Ipv6Addr = Ipv6Addr::UNSPECIFIED;\n'
expect_case loopback 1 "[::]:port in a config asset" assets/v.json '{"listen": "[::]:8080"}\n'

# --- canary ---
expect_case canary 1 "body canary in a built binary under target/" target/debug/app '\x7fELF\x00\x00PELWAY_CANARY_BODY_7f3a\x00'
expect_case canary 1 "API-key canary in logs/" logs/app.log 'start\napi_key=PELWAY_CANARY_APIKEY_9c21\n'
expect_case canary 1 "PII canary in a *.log elsewhere" run/demo.log 'name=PELWAY_CANARY_PII_4e8d\n'
expect_case canary 0 "artifacts present but no canary" target/debug/app '\x7fELF\x00\x00clean\x00'
r="${tmp}/case"; make_clean "${r}"; mkdir -p "${tmp}/device/store"
printf 'x\x00PELWAY_CANARY_PII_4e8d\x00' > "${tmp}/device/store/db.bin"
run_stage "${r}" canary DEMO_CHECK_EXTRA_PATHS="${tmp}/nothing-here:${tmp}/device"
if [ "${RC}" -eq 2 ]; then ok "canary: missing DEMO_CHECK_EXTRA_PATHS entry is exit 2"; else ng "canary: missing extra path (exit ${RC})"; show "${OUT}"; fi
run_stage "${r}" canary DEMO_CHECK_EXTRA_PATHS="${tmp}/device"
if [ "${RC}" -eq 1 ] && printf '%s\n' "${OUT}" | grep -qF "${tmp}/device/store/db.bin:(binary)"; then ok "canary: hit in DEMO_CHECK_EXTRA_PATHS (device storage) fails"; else ng "canary: extra path hit (exit ${RC})"; show "${OUT}"; fi

echo "selftest: ${pass} passed, ${fail} failed"
[ "${fail}" -eq 0 ]
