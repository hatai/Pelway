#!/usr/bin/env bash
# demo-check の自己テスト。mktemp の一時ディレクトリに合成データの小さなツリーを作り、
#   - 違反のないツリーで 4 段階とも exit 0
#   - 違反を 1 つ仕込むごとに該当段階が exit 1 で「ファイル:行」を出す
#   - 除外対象（docs/・tests/fixtures/・Rust のコメント中の UI 語・xmlns）では失敗しない
#   - 秘密に見える値（SYNTHETIC を含む合成値）を出力しない
#   - run.sh が段階名以外の引数（パス・未知の名前）を拒否し、何も実行しない
#   - 段階指定の実行が PARTIAL RUN と表示される
#   - DEMO_CHECK_ACCEPTANCE=1 のとき loopback の runtime PENDING と canary の走査ゼロが
#     exit 1 になり、ACCEPTANCE MODE と表示される（未設定時の挙動は不変）
#   - PELWAY_BIND_ADDR を経由しない bind("...") の宛先も許可値（127.0.0.1/[::1]:0-65535）と
#     完全一致で照合される（DC-01）。同じ行に複数の bind があってもすべて検査する（slot5 指摘1）。
#     ポートは 0〜65535 で 65536〜99999 は違反（DC-06）
#   - 引用符の直後の「//ホスト名」（JS などの文字列内のプロトコル相対参照）を検出し、
#     文字列外の // コメントや path//file は誤検知しない（DC-02）。
#     行コメント内の引用符付き URL も誤検知しない（slot5 指摘3）
#   - 走査中のシンボリックリンクをたどらない。ルート自体が作業ツリーの外を指すリンクなら
#     読まずに exit 2（mbx 共有ビルドキャッシュ配下のみ例外として走査する）（DC-03・slot5 指摘2）
#   - 改行を含むファイル名でも壊れない（DC-04）
#   - find などの走査失敗（読めないディレクトリ）で exit 2（DC-05: pipefail）。
#     canary の目印検索でもファイルが読めなければ exit 2（slot5 指摘4）
# を確かめる。本物のリスト（forbidden-phrases.txt / allowed-hosts.txt / canaries.txt）を使う。
# リポジトリには書き込まない。
set -uo pipefail
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

# --- DEMO_CHECK_ACCEPTANCE=1（受入モード）。未設定時の挙動は変えない ---
make_clean "${tmp}/case"
run_stage "${tmp}/case" loopback DEMO_CHECK_ACCEPTANCE=1
if [ "${RC}" -eq 1 ] && printf '%s\n' "${OUT}" | grep -q "ACCEPTANCE MODE"; then
  ok "loopback: runtime PENDING is a failure under ACCEPTANCE (exit 1)"
else ng "loopback: ACCEPTANCE runtime PENDING (exit ${RC})"; show "${OUT}"; fi

run_stage "${tmp}/case" canary DEMO_CHECK_ACCEPTANCE=1
if [ "${RC}" -eq 1 ] && printf '%s\n' "${OUT}" | grep -q "ACCEPTANCE MODE"; then
  ok "canary: no artifacts to scan is a failure under ACCEPTANCE (exit 1)"
else ng "canary: ACCEPTANCE no artifacts (exit ${RC})"; show "${OUT}"; fi

make_clean "${tmp}/case"
mkdir -p "${tmp}/case/target/debug"
printf 'clean artifact\n' > "${tmp}/case/target/debug/app"
run_stage "${tmp}/case" canary DEMO_CHECK_ACCEPTANCE=1
if [ "${RC}" -eq 0 ]; then
  ok "canary: artifacts present and no canary passes under ACCEPTANCE"
else ng "canary: ACCEPTANCE with artifacts (exit ${RC})"; show "${OUT}"; fi

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

# --- run.sh: 受入モード（全段階・一部段階の両方） ---
make_clean "${tmp}/case"
OUT="$(DEMO_CHECK_ROOT="${tmp}/case" DEMO_CHECK_ACCEPTANCE=1 bash "${here}/run.sh" 2>&1)"; RC=$?
if [ "${RC}" -eq 1 ] && printf '%s\n' "${OUT}" | grep -q "ACCEPTANCE MODE" && printf '%s\n' "${OUT}" | grep -q "failed: loopback canary"; then
  ok "run.sh: full run under ACCEPTANCE fails (loopback PENDING + canary no artifacts) and prints ACCEPTANCE MODE"
else ng "run.sh: full run under ACCEPTANCE (exit ${RC})"; show "${OUT}"; fi

OUT="$(DEMO_CHECK_ROOT="${tmp}/case" DEMO_CHECK_ACCEPTANCE=1 bash "${here}/run.sh" loopback 2>&1)"; RC=$?
if [ "${RC}" -eq 1 ] && printf '%s\n' "${OUT}" | grep -q "PARTIAL RUN" && printf '%s\n' "${OUT}" | grep -q "ACCEPTANCE MODE"; then
  ok "run.sh: partial run under ACCEPTANCE still fails and prints both marks"
else ng "run.sh: partial run under ACCEPTANCE (exit ${RC})"; show "${OUT}"; fi

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
# 数字の境目: Rust の文字列リテラルの先頭・末尾にある 24/72（日本語・英語）も見逃さない
expect_case leak-phrases 1 "\"72 hours...\" at the start of a Rust literal" "${R}" 'pub const NB1: &str = "72 hours notice";\n'
expect_case leak-phrases 1 "\"24 hours\" fills a Rust literal exactly" "${R}" 'pub const NB2: &str = "24 hours";\n'
expect_case leak-phrases 1 "\"...within 72 hours\" at the end of a Rust literal" "${R}" 'pub const NB3: &str = "within 72 hours";\n'
expect_case leak-phrases 1 "\"24時間以内...\" at the start of a Rust literal" "${R}" 'pub const NB4: &str = "24時間以内にお知らせ";\n'
expect_case leak-phrases 1 "\"72時間\" fills a Rust literal exactly" "${R}" 'pub const NB5: &str = "72時間";\n'
expect_case leak-phrases 0 "\"1024 hours\" is not a 24/72 boundary match" "${R}" 'pub const NB6: &str = "1024 hours";\n'
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
# DC-02: 引用符の直後の「//ホスト名」（JS などの文字列内のプロトコル相対参照）
expect_case external-refs 1 "quoted //host in a JS string (assignment)" web/app.js 'const ep = "//api.example.com/v1";\n'
expect_case external-refs 1 "quoted //host inside fetch()" web/app2.js 'fetch("//static.example.io/x")\n'
expect_case external-refs 1 "quoted //host in a Rust string literal" "${R}" 'pub const PR: &str = "//cdn.example.com/a.js";\n'
expect_case external-refs 1 "backtick template //host in JS" web/app3.js 'const t = `//cdn.example.dev/a`;\n'
expect_case external-refs 0 "a // comment naming a host is not a reference" web/guard.js '// api.example.com には依存しない（コメント行）\nlet q = a / b / c;\n'
expect_case external-refs 0 "path//inside/a string is not a reference" web/guard2.js 'const p = "path//to/file";\n'
# 指摘3（slot5）: 行コメント内の引用符付き URL は誤検出しない。実コードの引用符付き参照は引き続き検出する
expect_case external-refs 0 "quoted //host inside a JS line comment is not a ref" web/c.js '// セットアップ: "//api.example.com/v1" を参照\n'
expect_case external-refs 0 "quoted https URL inside a JS line comment is not a ref" web/c2.js '// ドキュメント: "https://docs.example.com/intro" を参照\n'
expect_case external-refs 1 "quoted //host in code with a trailing // comment" web/c3.js 'const u = "//cdn.example.com/a"; // 本物はこちら\n'

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
# DC-01: PELWAY_BIND_ADDR を経由しない bind("...") の宛先も許可値と完全一致で照合
expect_case loopback 1 "TcpListener::bind(\"localhost:8080\") is a violation" "${R}" 'fn b1() { let _ = std::net::TcpListener::bind("localhost:8080"); }\n'
expect_case loopback 1 "bind to a non-loopback IPv4 is a violation" "${R}" 'fn b2() { let _ = std::net::TcpListener::bind("192.168.10.4:9000"); }\n'
expect_case loopback 0 "bind(\"127.0.0.1:8080\") passes" "${R}" 'fn b3() { let _ = std::net::TcpListener::bind("127.0.0.1:8080"); }\n'
expect_case loopback 0 "bind(\"[::1]:0\") passes" "${R}" 'fn b4() { let _ = std::net::TcpListener::bind("[::1]:0"); }\n'
# 指摘1（slot5）: 同じ行に複数の bind があってもすべて検査する
expect_case loopback 1 "allowed bind then localhost bind on one line" "${R}" 'fn m1() { let _ = std::net::TcpListener::bind("127.0.0.1:1").unwrap(); let _ = std::net::TcpListener::bind("localhost:2").unwrap(); }\n'
expect_case loopback 1 "localhost bind then allowed bind on one line" "${R}" 'fn m2() { let _ = std::net::TcpListener::bind("localhost:2").unwrap(); let _ = std::net::TcpListener::bind("[::1]:3").unwrap(); }\n'
expect_case loopback 0 "two allowed binds on one line" "${R}" 'fn m3() { let _ = std::net::TcpListener::bind("127.0.0.1:1").unwrap(); let _ = std::net::TcpListener::bind("[::1]:2").unwrap(); }\n'
# DC-06: ポートは 0〜65535（0 は自動割り当て）。65536〜99999 は違反
expect_case loopback 1 "PELWAY_BIND_ADDR port 65536 is a violation" mise.toml '[env]\nPELWAY_BIND_ADDR = "127.0.0.1:65536"\n' replace
expect_case loopback 1 "PELWAY_BIND_ADDR port 99999 is a violation" mise.toml '[env]\nPELWAY_BIND_ADDR = "127.0.0.1:99999"\n' replace
expect_case loopback 1 "PELWAY_BIND_ADDR [::1]:65536 is a violation" mise.toml '[env]\nPELWAY_BIND_ADDR = "[::1]:65536"\n' replace
expect_case loopback 0 "PELWAY_BIND_ADDR port 65535 passes" mise.toml '[env]\nPELWAY_BIND_ADDR = "127.0.0.1:65535"\n' replace
expect_case loopback 0 "PELWAY_BIND_ADDR port 0 (auto-assign) still passes" mise.toml '[env]\nPELWAY_BIND_ADDR = "127.0.0.1:0"\n' replace
expect_case loopback 1 "bind port 65536 in Rust is a violation" "${R}" 'fn b5() { let _ = std::net::TcpListener::bind("127.0.0.1:65536"); }\n'
# DC-06 確定（開発リード 17:02）: 許可ホストでなければポート 0 でも違反。65536 以上はどのホストでも違反
expect_case loopback 1 "PELWAY_BIND_ADDR = localhost:0 (port 0 does not allow a bad host)" mise.toml '[env]\nPELWAY_BIND_ADDR = "localhost:0"\n' replace
expect_case loopback 1 "PELWAY_BIND_ADDR = localhost:65536 (bad host and bad port)" mise.toml '[env]\nPELWAY_BIND_ADDR = "localhost:65536"\n' replace

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

# --- DC-03: 走査中のシンボリックリンクをたどらない（リンク先を走査ルートの外に置く） ---
r="${tmp}/case"; make_clean "${r}"
printf '// GDPR準拠\n' > "${tmp}/outside-src.rs"
ln -s "${tmp}/outside-src.rs" "${r}/crates/x/src/link.rs"
run_stage "${r}" leak-phrases
if [ "${RC}" -eq 0 ]; then ok "leak-phrases: a symlinked source file is not followed"; else ng "leak-phrases: symlink not followed (exit ${RC})"; show "${OUT}"; fi

printf 'x https://evil.example.com/y\n' > "${tmp}/outside-ref.txt"
ln -s "${tmp}/outside-ref.txt" "${r}/assets/link.svg"
run_stage "${r}" external-refs
if [ "${RC}" -eq 0 ]; then ok "external-refs: a symlinked asset is not followed"; else ng "external-refs: symlink not followed (exit ${RC})"; show "${OUT}"; fi

mkdir -p "${r}/logs"
printf 'PELWAY_CANARY_BODY_7f3a\n' > "${tmp}/outside-canary.log"
ln -s "${tmp}/outside-canary.log" "${r}/logs/link.log"
run_stage "${r}" canary
if [ "${RC}" -eq 0 ]; then ok "canary: a symlinked log is not followed"; else ng "canary: symlink not followed (exit ${RC})"; show "${OUT}"; fi
rm -f "${r}/crates/x/src/link.rs" "${r}/assets/link.svg" "${r}/logs/link.log"

# 指摘2（slot5）: ルート自体が作業ツリーの外を指すリンクなら、外を読まずに exit 2。
# ただし実体が mbx 共有ビルドキャッシュ（~/.cache/mbx/targets/ 配下）のときだけ例外として走査する
r="${tmp}/case"; make_clean "${r}"
mkdir -p "${tmp}/realtarget/debug"
printf 'PELWAY_CANARY_APIKEY_9c21\n' > "${tmp}/realtarget/debug/app"
ln -s "${tmp}/realtarget" "${r}/target"
run_stage "${r}" canary
if [ "${RC}" -eq 2 ] && ! printf '%s\n' "${OUT}" | grep -qF "PELWAY_CANARY_APIKEY"; then
  ok "canary: target/ linking outside the work tree exits 2 without reading it"
else ng "canary: outside root link (exit ${RC})"; show "${OUT}"; fi

r="${tmp}/case"; make_clean "${r}"
mkdir -p "${tmp}/fakehome/.cache/mbx/targets/v1/proj/debug"
printf 'PELWAY_CANARY_BODY_7f3a\n' > "${tmp}/fakehome/.cache/mbx/targets/v1/proj/debug/app"
ln -s "${tmp}/fakehome/.cache/mbx/targets/v1/proj" "${r}/target"
run_stage "${r}" canary HOME="${tmp}/fakehome"
if [ "${RC}" -eq 1 ] && printf '%s\n' "${OUT}" | grep -qF "target/debug/app:"; then
  ok "canary: target/ linking into the mbx cache is still scanned"
else ng "canary: mbx cache root link (exit ${RC})"; show "${OUT}"; fi

# --- DC-04: 改行を含むファイル名でも壊れない ---
r="${tmp}/case"; make_clean "${r}"
wname=$'we\nird.rs'
printf 'pub const NL: &str = "GDPR準拠";\n' > "${r}/crates/x/src/${wname}"
run_stage "${r}" leak-phrases
if [ "${RC}" -eq 1 ] && printf '%s\n' "${OUT}" | grep -qF "ird.rs:1"; then
  ok "leak-phrases: a filename containing a newline is scanned and reported"
else ng "leak-phrases: newline filename (exit ${RC})"; show "${OUT}"; fi

# --- DC-05: find などの走査に失敗したら exit 2（pipefail） ---
if [ "$(id -u)" -ne 0 ]; then
  r="${tmp}/case"; make_clean "${r}"
  mkdir -p "${r}/crates/x/src/locked"
  chmod 000 "${r}/crates/x/src/locked"
  run_stage "${r}" leak-phrases
  rc_leak="${RC}"
  run_stage "${r}" loopback
  rc_loop="${RC}"
  chmod 755 "${r}/crates/x/src/locked"
  if [ "${rc_leak}" -eq 2 ]; then ok "leak-phrases: an unreadable directory makes the scan exit 2"; else ng "leak-phrases: scan failure exit (${rc_leak}, want 2)"; show "${OUT}"; fi
  if [ "${rc_loop}" -eq 2 ]; then ok "loopback: an unreadable directory makes the config scan exit 2"; else ng "loopback: scan failure exit (${rc_loop}, want 2)"; fi

  # 指摘4（slot5）: canary の目印検索でファイルが読めなければ exit 2
  r="${tmp}/case"; make_clean "${r}"
  mkdir -p "${r}/target/debug"
  printf 'clean artifact\n' > "${r}/target/debug/app"
  printf 'PELWAY_CANARY_PII_4e8d\n' > "${r}/target/debug/locked"
  chmod 000 "${r}/target/debug/locked"
  run_stage "${r}" canary
  rc_file="${RC}"
  chmod 644 "${r}/target/debug/locked"
  mkdir -p "${r}/target/lockeddir"
  chmod 000 "${r}/target/lockeddir"
  run_stage "${r}" canary
  rc_dir="${RC}"
  chmod 755 "${r}/target/lockeddir"
  if [ "${rc_file}" -eq 2 ]; then ok "canary: an unreadable artifact file exits 2"; else ng "canary: unreadable file (exit ${rc_file}, want 2)"; show "${OUT}"; fi
  if [ "${rc_dir}" -eq 2 ]; then ok "canary: an unreadable directory under target/ exits 2"; else ng "canary: unreadable dir (exit ${rc_dir}, want 2)"; fi
else
  ok "scan-failure checks skipped (running as root: unreadable directories stay readable)"
fi

echo "selftest: ${pass} passed, ${fail} failed"
[ "${fail}" -eq 0 ]
