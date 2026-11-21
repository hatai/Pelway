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
expect_eq "resp_status 200" "$(resp_status < "${tmp}/ok.resp")" "200"
expect_eq "resp_body after blank line" "$(resp_body < "${tmp}/ok.resp")" '{"a":1}'
printf 'HTTP/1.1 400 Bad Request\r\n\r\n{"code":"bad_request"}' > "${tmp}/bad.resp"
expect_eq "resp_status 400" "$(resp_status < "${tmp}/bad.resp")" "400"
expect_eq "resp_body without trailing newline" "$(resp_body < "${tmp}/bad.resp")" '{"code":"bad_request"}'

ok_root() { evidence_root_ok "$@" && echo 0 || echo 1; }
expect_eq "evidence_root_ok: sibling path" "$(ok_root /srv/ev /home/u/Pelway /home/u/Pelway/.git)" 0
expect_eq "evidence_root_ok: inside work tree" "$(ok_root /home/u/Pelway/target/ev /home/u/Pelway /home/u/Pelway/.git)" 1
expect_eq "evidence_root_ok: work tree itself" "$(ok_root /home/u/Pelway /home/u/Pelway /home/u/Pelway/.git)" 1
expect_eq "evidence_root_ok: prefix-sharing sibling" "$(ok_root /home/u/Pelway-ev /home/u/Pelway /home/u/Pelway/.git)" 0
expect_eq "evidence_root_ok: git common dir inside main checkout" "$(ok_root /home/u/Pelway/.git/pelway/evidence /home/u/Pelway /home/u/Pelway/.git)" 0
expect_eq "evidence_root_ok: shared git dir from a worktree" "$(ok_root /r/Pelway/.git/pelway/evidence /wt/568 /r/Pelway/.git)" 0
expect_eq "evidence_root_ok: .. back into the work tree" "$(ok_root /home/u/Pelway/.git/../crates/ev /home/u/Pelway /home/u/Pelway/.git)" 1
mkdir -p "${tmp}/wt/inner" "${tmp}/wt/.git"
ln -s "${tmp}/wt/inner" "${tmp}/link"
expect_eq "evidence_root_ok: symlink into the work tree" "$(ok_root "${tmp}/link/ev" "${tmp}/wt" "${tmp}/wt/.git")" 1

expect_eq "decision_absent: empty array" "$(decision_absent '[]' dec-1 && echo 0 || echo 1)" 0
expect_eq "decision_absent: other ids" "$(decision_absent '[{"decision_id":"dec-2"}]' dec-1 && echo 0 || echo 1)" 0
expect_eq "decision_absent: still pending" "$(decision_absent '[{"decision_id":"dec-1"}]' dec-1 && echo 0 || echo 1)" 1
expect_eq "decision_absent: broken json" "$(decision_absent 'not json' dec-1 && echo 0 || echo 1)" 1
expect_eq "decision_absent: empty body" "$(decision_absent '' dec-1 && echo 0 || echo 1)" 1
expect_eq "decision_absent: object, not array" "$(decision_absent '{"code":"x"}' dec-1 && echo 0 || echo 1)" 1

rl() { ready_line_ok "$1" "$2" && echo 0 || echo 1; }
expect_eq "ready_line_ok: good" "$(rl 'demo-serve ready addr=127.0.0.1:45000 pid=42 llm=script' 42)" 0
expect_eq "ready_line_ok: other pid" "$(rl 'demo-serve ready addr=127.0.0.1:45000 pid=43 llm=script' 42)" 1
expect_eq "ready_line_ok: non-loopback" "$(rl 'demo-serve ready addr=10.0.0.5:45000 pid=42 llm=script' 42)" 1
expect_eq "ready_line_ok: port out of range" "$(rl 'demo-serve ready addr=127.0.0.1:70000 pid=42 llm=script' 42)" 1
expect_eq "ready_line_ok: two lines" "$(rl $'demo-serve ready addr=127.0.0.1:1 pid=42 llm=script\nx' 42)" 1

printf 'PATH=/usr/bin\0PELWAY_PENPOT_MCP_TOKEN=x\0PELWAY_LLM_API_KEY_OTHER=y\0HOME=/h\0' > "${tmp}/environ"
expect_eq "forbidden_env_present: name match only" "$(forbidden_env_present "${tmp}/environ" | tr '\n' ' ')" "penpot_mcp_token "
printf 'PATH=/usr/bin\0HOME=/h\0' > "${tmp}/environ-clean"
expect_eq "forbidden_env_present: clean" "$(forbidden_env_present "${tmp}/environ-clean")" ""

printf 'tcp LISTEN 0 128 127.0.0.1:4000 0.0.0.0:*\ntcp ESTAB 0 0 127.0.0.1:4000 127.0.0.1:5000\n' > "${tmp}/lo.ss"
expect_eq "count_nonloopback: loopback only" "$(count_nonloopback "${tmp}/lo.ss")" 0
printf 'tcp ESTAB 0 0 10.0.0.2:5000 203.0.113.9:443\ntcp SYN-SENT 0 0 127.0.0.1:5001 192.0.2.1:443\n' >> "${tmp}/lo.ss"
expect_eq "count_nonloopback: external peers" "$(count_nonloopback "${tmp}/lo.ss")" 3
printf 'udp ESTAB 0 0 127.0.0.1:5353 192.0.2.53:53\nudp UNCONN 0 0 0.0.0.0:5354 0.0.0.0:*\n' > "${tmp}/udp.ss"
expect_eq "count_nonloopback: UDP to a resolver and an unbound UDP socket" "$(count_nonloopback "${tmp}/udp.ss")" 2

expect_eq "redact: body and question" "$(redact <<< '{"body":"secret text","question":"q","n":{"body":"x"},"seq":4}')" \
  '{"body":"<redacted len=11>","question":"<redacted len=1>","n":{"body":"<redacted len=1>"},"seq":4}'
expect_eq "redact: non-json" "$(redact <<< 'plain words')" '<redacted non-json len=11>'

expect_eq "final_verdict: finished PASS" "$(final_verdict PASS 1)" PASS
expect_eq "final_verdict: finished FAIL" "$(final_verdict FAIL 1)" FAIL
expect_eq "final_verdict: interrupted PASS" "$(final_verdict PASS 0)" FAIL
expect_eq "final_verdict: unset mark" "$(final_verdict PASS '')" FAIL

# pitchfork_list: 偽の mise で、supervisor が止まっているときは list を呼ばない（起動させない）ことを固定する。
mkdir -p "${tmp}/fakebin"
cat > "${tmp}/fakebin/mise" <<'FAKE'
#!/usr/bin/env bash
echo "args=$* auto_start=${PITCHFORK_SUPERVISOR_AUTO_START:-unset}" >> "${FAKE_LOG}"
[ "$*" = "exec -- pitchfork supervisor status" ] && exit "${FAKE_STATUS}"
exit 0
FAKE
chmod +x "${tmp}/fakebin/mise"
pf() { : > "${tmp}/mise.log"; (PATH="${tmp}/fakebin:${PATH}" FAKE_LOG="${tmp}/mise.log" FAKE_STATUS="$1" TOP="${tmp}" pitchfork_list > /dev/null); cat "${tmp}/mise.log"; }
expect_eq "pitchfork_list: supervisor stopped -> no list" "$(pf 1)" "args=exec -- pitchfork supervisor status auto_start=false"
expect_eq "pitchfork_list: supervisor running -> list without auto start" "$(pf 0 | tail -n 1)" "args=exec -- pitchfork list auto_start=false"

# make_run_dir: 新しく作り所有者だけの権限にする。既にある・置き場の下のリンクで外へ出る場合は失敗する。
mkdir -p "${tmp}/ev-root" "${tmp}/elsewhere"
got="$(make_run_dir "${tmp}/ev-root" "568-abc/run1")"
expect_eq "make_run_dir: creates the run dir" "${got}" "${tmp}/ev-root/568-abc/run1"
expect_eq "make_run_dir: owner-only permissions" "$(stat -c '%a' "${tmp}/ev-root/568-abc/run1")" 700
expect_eq "make_run_dir: steps dir" "$([ -d "${tmp}/ev-root/568-abc/run1/steps" ] && echo yes)" yes
expect_eq "make_run_dir: refuses an existing run dir" "$(make_run_dir "${tmp}/ev-root" "568-abc/run1" > /dev/null 2>&1 && echo 0 || echo 1)" 1
ln -s "${tmp}/elsewhere" "${tmp}/ev-root/568-link"
expect_eq "make_run_dir: refuses a symlink under the root" "$(make_run_dir "${tmp}/ev-root" "568-link/run1" > /dev/null 2>&1 && echo 0 || echo 1)" 1
expect_eq "make_run_dir: wrote nothing through the symlink" "$(find "${tmp}/elsewhere" -mindepth 1 | grep -c .)" 0

# close_fds_above_stderr: 引き継いだ fd だけを閉じ、標準入出力は残す（子に /dev/ptmx などを渡さない）。
expect_eq "close_fds_above_stderr: inherited fd is closed" \
  "$(exec 9>/dev/null; (close_fds_above_stderr; echo x 2>/dev/null >&9 && echo open || echo closed))" "closed"
expect_eq "close_fds_above_stderr: stdout stays open" \
  "$(exec 9>/dev/null; (close_fds_above_stderr; echo ok))" "ok"

# steps_complete: 送った件数ぶんの .req と .resp がそろっているときだけ 0。
mkdir -p "${tmp}/steps"
: > "${tmp}/steps/01-a.req"; : > "${tmp}/steps/01-a.resp"; : > "${tmp}/steps/02-b.req"; : > "${tmp}/steps/02-b.resp"
expect_eq "steps_complete: both pairs" "$(steps_complete "${tmp}/steps" 2 && echo 0 || echo 1)" 0
expect_eq "steps_complete: more requests sent than recorded" "$(steps_complete "${tmp}/steps" 3 && echo 0 || echo 1)" 1
rm -f "${tmp}/steps/02-b.resp"
expect_eq "steps_complete: a missing .resp" "$(steps_complete "${tmp}/steps" 2 && echo 0 || echo 1)" 1

# observation_errors: ss の取得に失敗した印を数える（該当なしとは区別する）。
printf 'tcp LISTEN 0 128 127.0.0.1:4000 0.0.0.0:*\n' > "${tmp}/obs.txt"
expect_eq "observation_errors: none" "$(observation_errors "${tmp}/obs.txt")" 0
printf '# ss の取得に失敗 01:02:03\n' >> "${tmp}/obs.txt"
expect_eq "observation_errors: one failed snapshot" "$(observation_errors "${tmp}/obs.txt")" 1

# dir_chain_safe / make_run_dir: ほかの利用者が書ける既存の親を拒む。
mkdir -p "${tmp}/wide-root/shared"; chmod 777 "${tmp}/wide-root/shared"
expect_eq "make_run_dir: refuses a group/other-writable parent" "$(make_run_dir "${tmp}/wide-root" "shared/run1" > /dev/null 2>&1 && echo 0 || echo 1)" 1
expect_eq "make_run_dir: nothing created under the writable parent" "$(find "${tmp}/wide-root/shared" -mindepth 1 | grep -c .)" 0
chmod 755 "${tmp}/wide-root/shared"
expect_eq "make_run_dir: accepts the parent once it is not writable by others" "$(make_run_dir "${tmp}/wide-root" "shared/run1" > /dev/null 2>&1 && echo 0 || echo 1)" 0
mkdir -p "${tmp}/wide2"; chmod 777 "${tmp}/wide2"
expect_eq "make_run_dir: refuses an other-writable evidence root" "$(make_run_dir "${tmp}/wide2" "a/run1" > /dev/null 2>&1 && echo 0 || echo 1)" 1
chmod 755 "${tmp}/wide2"

# transport_status: 送受信の失敗は、成功の形の応答があっても 000。
expect_eq "transport_status: ok" "$(transport_status 0 0 $'HTTP/1.1 200 OK\r\n\r\n[]')" 200
expect_eq "transport_status: receive timed out" "$(transport_status 0 124 $'HTTP/1.1 200 OK\r\n\r\n[]')" 000
expect_eq "transport_status: send failed" "$(transport_status 1 0 $'HTTP/1.1 200 OK\r\n\r\n[]')" 000

# pending_decision_id: 200 で配列のときだけ id を返す。
pend='[{"decision_id":"dec-1","request_message_id":"msg-4"}]'
expect_eq "pending_decision_id: 200 array" "$(pending_decision_id 200 "${pend}" msg-4)" dec-1
expect_eq "pending_decision_id: non-200 with a success-shaped body" "$(pending_decision_id 500 "${pend}" msg-4)" ""
expect_eq "pending_decision_id: not an array" "$(pending_decision_id 200 '{"decision_id":"dec-1","request_message_id":"msg-4"}' msg-4)" ""

# forbidden_env_present: 読めない environ は「無し」ではなく失敗。
expect_eq "forbidden_env_present: unreadable environ fails" "$(forbidden_env_present "${tmp}/no-such-environ" > /dev/null 2>&1; echo $?)" 2

# run_demo_check_stage: 呼び出し元の DEMO_CHECK_* を引き継がない。
mkdir -p "${tmp}/dc-top/scripts/demo-check"
printf '#!/usr/bin/env bash\nenv | grep "^DEMO_CHECK_" | sort\n' > "${tmp}/dc-top/scripts/demo-check/canary.sh"
expect_eq "run_demo_check_stage: only DEMO_CHECK_ROOT is passed" \
  "$(DEMO_CHECK_LIST_DIR=/x DEMO_CHECK_EXTRA_PATHS=/y DEMO_CHECK_ACCEPTANCE=1 run_demo_check_stage "${tmp}/dc-top" /scan canary)" "DEMO_CHECK_ROOT=/scan"

# publish_verdict: ハッシュを作れないときは最後の PASS を残さない。
mkdir -p "${tmp}/pub-ok" "${tmp}/pub-ng"
printf 'run_id=x\n' > "${tmp}/pub-ok/summary.txt"; printf 'run_id=x\n' > "${tmp}/pub-ng/summary.txt"
out_ok="$(publish_verdict "${tmp}/pub-ok" PASS)"; rc_ok=$?
expect_eq "publish_verdict: ok exit" "${rc_ok}" 0
expect_eq "publish_verdict: ok prints PASS" "$(head -n 1 <<< "${out_ok}")" "verdict=PASS"
expect_eq "publish_verdict: ok sums verify" "$(cd "${tmp}/pub-ok" && sha256sum --quiet -c SHA256SUMS && echo ok)" ok
out_ng="$(SHA256SUM=false publish_verdict "${tmp}/pub-ng" PASS 2> /dev/null)"; rc_ng=$?
expect_eq "publish_verdict: hash failure exits 1" "${rc_ng}" 1
expect_eq "publish_verdict: hash failure prints no PASS" "$(grep -c 'verdict=PASS' <<< "${out_ng}")" 0
expect_eq "publish_verdict: hash failure leaves no PASS in summary" "$(grep -c '^verdict=PASS' "${tmp}/pub-ng/summary.txt")" 0
expect_eq "publish_verdict: hash failure leaves no SHA256SUMS" "$([ -e "${tmp}/pub-ng/SHA256SUMS" ] && echo yes || echo no)" no

# publish_verdict: 照合までは通っても、報告用の SHA256SUMS 自体のハッシュが取れなければ PASS を出さない。
cat > "${tmp}/half-hasher" <<'FAKE'
#!/usr/bin/env bash
case "$*" in *--quiet*|*-c*) exec sha256sum "$@" ;; esac
for a in "$@"; do case "${a}" in */SHA256SUMS) exit 1 ;; esac; done
exec sha256sum "$@"
FAKE
chmod +x "${tmp}/half-hasher"
mkdir -p "${tmp}/pub-half"; printf 'run_id=x\n' > "${tmp}/pub-half/summary.txt"
out_half="$(SHA256SUM="${tmp}/half-hasher" publish_verdict "${tmp}/pub-half" PASS 2> /dev/null)"; rc_half=$?
expect_eq "publish_verdict: report digest failure exits 1" "${rc_half}" 1
expect_eq "publish_verdict: report digest failure prints no PASS" "$(grep -c 'verdict=PASS' <<< "${out_half}")" 0
expect_eq "publish_verdict: report digest failure leaves no PASS in summary" "$(grep -c '^verdict=PASS' "${tmp}/pub-half/summary.txt")" 0

# fd_targets_ok: リンク先を固定の文字列で比べる。正規表現の記号や空白を含む証拠のパスでも正しく判定する。
odd="${tmp}/ev (1 [x"; mkdir -p "${odd}" "${tmp}/fds"
ln -s /dev/null "${tmp}/fds/0"; ln -s "${odd}/server.stdout" "${tmp}/fds/1"; ln -s "${odd}/server.stderr" "${tmp}/fds/2"
ln -s 'socket:[123]' "${tmp}/fds/3"; ln -s 'pipe:[9]' "${tmp}/fds/4"; ln -s 'anon_inode:[eventpoll]' "${tmp}/fds/5"
fdr() { fd_targets_ok "$@" > /dev/null 2>&1; echo $?; }
expect_eq "fd_targets_ok: allowed targets with an odd evidence path" "$(fdr "${tmp}/fds" "${odd}" "${tmp}/fd.out")" 0
expect_eq "fd_targets_ok: records each fd" "$(grep -c ' -> ' "${tmp}/fd.out")" 6
ln -s /etc/passwd "${tmp}/fds/6"
expect_eq "fd_targets_ok: a disallowed file" "$(fdr "${tmp}/fds" "${odd}" "${tmp}/fd.out")" 1
rm -f "${tmp}/fds/6"; ln -s "${tmp}/ev (2/server.stdout" "${tmp}/fds/6"
expect_eq "fd_targets_ok: another run's stdout is not allowed" "$(fdr "${tmp}/fds" "${odd}" "${tmp}/fd.out")" 1
rm -f "${tmp}/fds/6"; : > "${tmp}/fds/7"
expect_eq "fd_targets_ok: a target that cannot be read" "$(fdr "${tmp}/fds" "${odd}" "${tmp}/fd.out")" 2
rm -f "${tmp}/fds/7"

expect_eq "fd_targets_ok: fd dir does not exist (child gone)" "$(fdr "${tmp}/no-such-fd-dir" "${odd}" "${tmp}/fd.out")" 2
mkdir -p "${tmp}/empty-fds"
expect_eq "fd_targets_ok: empty fd dir" "$(fdr "${tmp}/empty-fds" "${odd}" "${tmp}/fd.out")" 2

# stop_sampler: 意図した停止は、送った TERM に trap で応えた終了（4）だけ。先に終わっていた場合は
# どんな終わり方でも、応え方が違っても、観察が切れていたので OBS_FAILED にする。
# shellcheck disable=SC2034  # SAMPLER は source した verify-run の stop_sampler が読む
ss_rc() { OBS_FAILED=0; ( eval "$1" ) & SAMPLER=$!; sleep "${2:-0}"; stop_sampler; echo "${OBS_FAILED}"; }
expect_eq "stop_sampler: intended stop (trap answers TERM with 4)" "$(ss_rc 'trap "exit 4" TERM; while :; do sleep 0.05; done' 0.3)" 0
expect_eq "stop_sampler: sampler already failed" "$(ss_rc 'exit 3' 0.3)" 1
expect_eq "stop_sampler: wrong answer to the stop request" "$(ss_rc 'trap "exit 3" TERM; while :; do sleep 0.05; done' 0.3)" 1
expect_eq "stop_sampler: exits on its own before the stop request" "$(ss_rc 'exit 0' 0.3)" 1
# 停止の要求より前に、ほかのシグナル（TERM を含む）で消えた場合も、観察が切れていたので失敗にする。
ss_sig() { OBS_FAILED=0; ( eval "$1" ) & SAMPLER=$!; sleep "${2:-0.2}"; kill -"${3:-KILL}" "${SAMPLER}" 2> /dev/null; sleep 0.2; stop_sampler; echo "${OBS_FAILED}"; }
expect_eq "stop_sampler: killed by SIGKILL before the stop request" "$(ss_sig 'while :; do sleep 0.05; done')" 1
expect_eq "stop_sampler: killed by SIGTERM before the stop request" "$(ss_sig 'while :; do sleep 0.05; done' 0.2 TERM)" 1
expect_eq "stop_sampler: answered TERM before the request" "$(ss_sig 'trap "exit 4" TERM; while :; do sleep 0.05; done' 0.2 TERM)" 1
expect_eq "stop_sampler: stopped without the trap (no answer)" "$(ss_rc 'while :; do sleep 0.05; done' 0.3)" 1

# try_lock: ビルド先を鍵にしたロックはビルドの前から run の終わりまで持つ（同じビルド先を共有する別の
# 作業ツリーの run が、検証中の run の実行ファイルをビルドで置き換えるのを防ぐ）。
expect_eq "try_lock: second holder is refused" "$(d="${tmp}/lock-a"; try_lock "${d}"; a=$?; try_lock "${d}" 2> /dev/null; echo "${a}-$?")" "0-1"
expect_eq "try_lock: released dir can be retaken" "$(d="${tmp}/lock-b"; try_lock "${d}" && rm -f "${d}/pid" && rmdir "${d}" && try_lock "${d}"; echo $?)" 0
expect_eq "bin lock is taken before the build" "$(awk '/bin-\$\(printf/{a=NR} /cargo build/{b=NR} END{print (a>0 && b>0 && a<b)?"yes":"no"}' "${tool}")" "yes"

# locks_dir_ready: 共有 git dir の pelway/locks を作り、証拠と同じ規則でほかの利用者が差し替えられない
# 階層だけを通す（安全な証拠の置き場でも、ロックの階層が安全でなければ止まる）。
expect_eq "locks_dir_ready: fresh dir" "$(locks_dir_ready "${tmp}/g1" && echo 0 || echo 1)" 0
mkdir -p "${tmp}/g2/pelway"; ln -s /tmp "${tmp}/g2/pelway/locks"
expect_eq "locks_dir_ready: locks is a symlink" "$(locks_dir_ready "${tmp}/g2" && echo 0 || echo 1)" 1
mkdir -p "${tmp}/g3/pelway/locks"; chmod 777 "${tmp}/g3/pelway/locks"
expect_eq "locks_dir_ready: world-writable locks" "$(locks_dir_ready "${tmp}/g3" && echo 0 || echo 1)" 1
mkdir -p "${tmp}/g4/pelway"; chmod 777 "${tmp}/g4/pelway"
expect_eq "locks_dir_ready: world-writable pelway" "$(locks_dir_ready "${tmp}/g4" && echo 0 || echo 1)" 1

# lock_owner: 持ち主の表示は、pid がリンクでない通常ファイルで数字だけのときだけ。
mkdir -p "${tmp}/l1"; echo 1234 > "${tmp}/l1/pid"
expect_eq "lock_owner: digits" "$(lock_owner "${tmp}/l1")" 1234
mkdir -p "${tmp}/l2"; ln -s /etc/passwd "${tmp}/l2/pid"
expect_eq "lock_owner: symlink pid" "$(lock_owner "${tmp}/l2")" ""
mkdir -p "${tmp}/l3"; echo "hello" > "${tmp}/l3/pid"
expect_eq "lock_owner: non-numeric" "$(lock_owner "${tmp}/l3")" ""
mkdir -p "${tmp}/l4"
expect_eq "lock_owner: no pid file" "$(lock_owner "${tmp}/l4")" ""

# target_dir_of: 相対の CARGO_TARGET_DIR は作業ツリーのルートから解決する（Cargo と同じ）。
expect_eq "target_dir_of: unset" "$(unset CARGO_TARGET_DIR; target_dir_of /wt)" "/wt/target"
expect_eq "target_dir_of: absolute" "$(CARGO_TARGET_DIR=/abs/td target_dir_of /wt)" "/abs/td"
expect_eq "target_dir_of: relative resolves from the work tree" "$(CARGO_TARGET_DIR=target target_dir_of /wt)" "/wt/target"
expect_eq "target_dir_of: relative and absolute share the output" "$(CARGO_TARGET_DIR=target target_dir_of /wt)" "$(CARGO_TARGET_DIR=/wt/target target_dir_of /other)"

# tree_state: コミット済み・変更あり・git の確認の失敗を区別する。
git init -q "${tmp}/ts"; git -C "${tmp}/ts" -c user.email=t@e -c user.name=t commit -q --allow-empty -m init
expect_eq "tree_state: clean" "$(tree_state "${tmp}/ts")" no
: > "${tmp}/ts/new-file"
expect_eq "tree_state: dirty" "$(tree_state "${tmp}/ts")" yes
mkdir -p "${tmp}/not-git"
expect_eq "tree_state: git fails" "$(GIT_CEILING_DIRECTORIES="${tmp}" tree_state "${tmp}/not-git" > /dev/null 2>&1; echo $?)" 1

# observe_sockets: 取得の失敗と記録の失敗は 1、該当なしは 0。
mkdir -p "${tmp}/ssbin" "${tmp}/obs-ev"
cat > "${tmp}/ssbin/ss" <<'FAKE'
#!/usr/bin/env bash
[ -n "${FAKE_SS_FAIL:-}" ] && exit 1
echo 'tcp ESTAB 0 0 127.0.0.1:1 127.0.0.1:2 users:(("x",pid=42,fd=3))'
FAKE
chmod +x "${tmp}/ssbin/ss"
obs() { (PATH="${tmp}/ssbin:${PATH}" EV="$1" PID="$2" ADDR=127.0.0.1:9 observe_sockets > /dev/null 2>&1); echo $?; }
expect_eq "observe_sockets: matching line recorded" "$(obs "${tmp}/obs-ev" 42)" 0
expect_eq "observe_sockets: line written" "$(grep -c 'pid=42,' "${tmp}/obs-ev/network.txt")" 1
expect_eq "observe_sockets: no match is not a failure" "$(obs "${tmp}/obs-ev" 99)" 0
expect_eq "observe_sockets: ss failure" "$(FAKE_SS_FAIL=1 obs "${tmp}/obs-ev" 42)" 1
expect_eq "observe_sockets: ss failure leaves a mark" "$(observation_errors "${tmp}/obs-ev/network.txt")" 1
expect_eq "observe_sockets: write failure" "$(obs "${tmp}/no-such-dir" 42)" 1

repo="${tmp}/repo"
git init -q "${repo}"
# 引数の検査まで必ず届くよう、呼び出し元の環境と道具の有無に依存させない: 禁止の 5 つの名前を外し、
# 前提の確認に使う道具（mise・rustup・ss・pgrep・ps と OS 名）の偽物を PATH の先頭に置く。
mkdir -p "${tmp}/argbin"
for b in mise rustup ss pgrep ps; do printf '#!/usr/bin/env bash\nexit 0\n' > "${tmp}/argbin/${b}"; chmod +x "${tmp}/argbin/${b}"; done
printf '#!/usr/bin/env bash\necho Linux\n' > "${tmp}/argbin/uname"; chmod +x "${tmp}/argbin/uname"
clean_env=(env -u PELWAY_LLM_API_KEY -u PELWAY_PENPOT_MCP_TOKEN -u PELWAY_DATABASE_URL -u PELWAY_LLM_CUSTOM_BASE_URL -u PELWAY_LLM_BASE_URL)
run_args() { (cd "${repo}" && PATH="${tmp}/argbin:${PATH}" "${clean_env[@]}" bash "${tool}" "$@" > /dev/null 2>&1); echo $?; }
expect_eq "unknown feature exits 2" "$(run_args --feature nope --evidence-root "${tmp}/ev")" 2
expect_eq "unknown argument exits 2" "$(run_args --bogus)" 2
expect_eq "relative evidence root exits 2" "$(run_args --evidence-root rel/ev)" 2
expect_eq "evidence root inside repo exits 2" "$(run_args --evidence-root "${repo}/ev")" 2
# 自分の環境に禁止の変数があれば、引数が正しくても起動の前に exit 1 で止まる（名前の有無だけを見る）。
expect_eq "forbidden variable in the caller's environment exits 1" \
  "$( (cd "${repo}" && PATH="${tmp}/argbin:${PATH}" "${clean_env[@]}" PELWAY_DATABASE_URL=x bash "${tool}" --evidence-root "${tmp}/ev" > /dev/null 2>&1); echo $?)" 1
[ ! -e "${repo}/ev" ] || failures+=("refused evidence root must not be created")
[ ! -e "${tmp}/ev" ] || failures+=("argument errors must not create evidence")

if [ "${#failures[@]}" -gt 0 ]; then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "verify-run.test.sh: all checks passed"
