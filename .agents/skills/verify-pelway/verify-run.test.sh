#!/usr/bin/env bash
# verify-run の機械な部分（応答の分解・証拠の置き場の検査・引数の検査）のテスト。
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

outside_repo /srv/ev /home/u/Pelway && r=0 || r=1
expect_eq "outside_repo: sibling path" "${r}" 0
outside_repo /home/u/Pelway/target/ev /home/u/Pelway && r=0 || r=1
expect_eq "outside_repo: inside repo" "${r}" 1
outside_repo /home/u/Pelway /home/u/Pelway && r=0 || r=1
expect_eq "outside_repo: repo itself" "${r}" 1
outside_repo /home/u/Pelway-evidence /home/u/Pelway && r=0 || r=1
expect_eq "outside_repo: prefix-sharing sibling" "${r}" 0

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
