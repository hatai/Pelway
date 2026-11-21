#!/usr/bin/env bash
# verify-run の機械な部分（応答の分解・本文の伏せ字・loopback 以外の数え方・証拠の置き場の検査・引数の検査）のテスト。
# サーバーは起動しない（端から端までの run は SKILL.md の手順で行う）。
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tool="${here}/verify-run"
tmp="$(mktemp -d)"
trap 'rm -rf "${tmp}"' EXIT
failures=()

expect_eq() {
  [ "$2" = "$3" ] || failures+=("$1: expected [$3], got [$2]")
}

# shellcheck source=/dev/null
source "${tool}"

printf 'HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{"a":1}\n' > "${tmp}/ok.resp"
expect_eq "resp_status 200" "$(resp_status "${tmp}/ok.resp")" "200"
expect_eq "resp_body after blank line" "$(resp_body "${tmp}/ok.resp")" '{"a":1}'
printf 'HTTP/1.1 400 Bad Request\r\n\r\n{"code":"bad_request"}' > "${tmp}/bad.resp"
expect_eq "resp_status 400" "$(resp_status "${tmp}/bad.resp")" "400"
expect_eq "resp_body without trailing newline" "$(resp_body "${tmp}/bad.resp")" '{"code":"bad_request"}'

ok_root() { evidence_root_ok "$@" && echo 0 || echo 1; }
expect_eq "evidence_root_ok: sibling path" "$(ok_root /srv/ev /home/u/Pelway /home/u/Pelway/.git)" 0
expect_eq "evidence_root_ok: inside work tree" "$(ok_root /home/u/Pelway/target/ev /home/u/Pelway /home/u/Pelway/.git)" 1
expect_eq "evidence_root_ok: work tree itself" "$(ok_root /home/u/Pelway /home/u/Pelway /home/u/Pelway/.git)" 1
expect_eq "evidence_root_ok: prefix-sharing sibling" "$(ok_root /home/u/Pelway-ev /home/u/Pelway /home/u/Pelway/.git)" 0
expect_eq "evidence_root_ok: git common dir inside main checkout" "$(ok_root /home/u/Pelway/.git/pelway/evidence /home/u/Pelway /home/u/Pelway/.git)" 0
expect_eq "evidence_root_ok: shared git dir from a worktree" "$(ok_root /r/Pelway/.git/pelway/evidence /wt/568 /r/Pelway/.git)" 0

printf 'PATH=/usr/bin\0PELWAY_PENPOT_MCP_TOKEN=x\0PELWAY_LLM_API_KEY_OTHER=y\0HOME=/h\0' > "${tmp}/environ"
expect_eq "forbidden_env_present: name match only" "$(forbidden_env_present "${tmp}/environ" | tr '\n' ' ')" "penpot_mcp_token "
printf 'PATH=/usr/bin\0HOME=/h\0' > "${tmp}/environ-clean"
expect_eq "forbidden_env_present: clean" "$(forbidden_env_present "${tmp}/environ-clean")" ""

printf 'LISTEN 0 128 127.0.0.1:4000 0.0.0.0:*\nESTAB 0 0 127.0.0.1:4000 127.0.0.1:5000\n' > "${tmp}/lo.ss"
expect_eq "count_nonloopback: loopback only" "$(count_nonloopback "${tmp}/lo.ss")" 0
printf 'ESTAB 0 0 10.0.0.2:5000 203.0.113.9:443\nSYN-SENT 0 0 127.0.0.1:5001 192.0.2.1:443\n' >> "${tmp}/lo.ss"
expect_eq "count_nonloopback: external peers" "$(count_nonloopback "${tmp}/lo.ss")" 3

expect_eq "redact: body and question" "$(redact <<< '{"body":"secret text","question":"q","n":{"body":"x"},"seq":4}')" \
  '{"body":"<redacted len=11>","question":"<redacted len=1>","n":{"body":"<redacted len=1>"},"seq":4}'
expect_eq "redact: non-json" "$(redact <<< 'plain words')" '<redacted non-json len=11>'

repo="${tmp}/repo"
git init -q "${repo}"
run_args() { (cd "${repo}" && bash "${tool}" "$@" > /dev/null 2>&1); echo $?; }
expect_eq "unknown feature exits 2" "$(run_args --feature nope --evidence-root "${tmp}/ev")" 2
expect_eq "unknown argument exits 2" "$(run_args --bogus)" 2
expect_eq "relative evidence root exits 2" "$(run_args --evidence-root rel/ev)" 2
expect_eq "evidence root inside repo exits 2" "$(run_args --evidence-root "${repo}/ev")" 2
[ ! -e "${repo}/ev" ] || failures+=("refused evidence root must not be created")
[ ! -e "${tmp}/ev" ] || failures+=("argument errors must not create evidence")

if [ "${#failures[@]}" -gt 0 ]; then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "verify-run.test.sh: all checks passed"
