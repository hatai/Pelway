#!/usr/bin/env bash
# scripts/deny-warn-extract.sh のテスト。cargo-deny 0.20.2 の実際の出力
# （`cargo deny --config deny-warn.toml check advisories`）を写した固定サンプルを
# 与えて、unmaintained / unsound の警告行と advisory の ID（RUSTSEC-…）が正しく
# 取り出せることを確かめる（ID が空になることが無いことの保証）。
# サンプルの採取: 2026-11-13、worktree agent/168-cargo-deny（Issue #168 round 4 時点の
# 依存ツリー）。cargo-deny の出力形式が変わったら（版更新後など）採取し直す。
set -uo pipefail

SCRIPT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/deny-warn-extract.sh"
failures=()

# run_case <name> <expected-exit> <sample-file>
run_case() {
  local name="$1" want_rc="$2" sample="$3"
  local out rc
  out="$(bash "$SCRIPT" <"$sample" 2>/dev/null)"
  rc=$?
  if [[ "$rc" != "$want_rc" ]]; then
    failures+=("$name: exit $rc, want $want_rc")
  fi
  REPLY_OUT="$out"
}

# ---- サンプル1: 実際の出力の写し（unmaintained 2 件と vulnerability 1 件）。
# pin-utils と rsa のブロックは cargo-deny 0.20.2 の出力からそのまま写した
# （樹形図の罫線 ├ └ │ ┌ ━ と行頭の空白もそのまま。rsa の説明文は長いため
# 中略せずそのまま）。
sample1="$(mktemp)"
trap 'rm -f "$sample1" "$sample2"' EXIT
cat >"$sample1" <<'EOF'
error[unmaintained]: `pin-utils` is unmaintained
    ┌─ /home/hatai/.t3/worktrees/Pelway/agent-168-cargo-deny-67cd4eb184dd1c40/Cargo.lock:433:1
    │
433 │ pin-utils 0.1.1 registry+https://github.com/rust-lang/crates.io-index
    │ ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ unmaintained advisory detected
    │
    ├ ID: RUSTSEC-2026-0441
    ├ Advisory: https://rustsec.org/advisories/RUSTSEC-2026-0441
    ├ The [upstream repository] was archived on October 1, 2026, after the
      release of version 0.1.1.

      For local pinning, replace `pin_mut!` with [`core::pin::pin!`], available
      since Rust 1.68. For pin projection, replace `unsafe_pinned!` and
      `unsafe_unpinned!` with [`pin-project`] or [`pin-project-lite`].

      [upstream repository]: https://github.com/rust-lang/pin-utils
    ├ Announcement: https://github.com/rust-lang/team/pull/2787
    ├ Solution: No safe upgrade is available!
    ├ pin-utils v0.1.1
      └── spin_on v0.1.1
          ├── i-slint-renderer-skia v1.18.1
          │   └── i-slint-backend-winit v1.18.1
          │       └── i-slint-backend-selector v1.18.1
          │           └── slint v1.18.1
          │               ├── pelway-client-ui v0.0.0
          │               └── pelway-dev-client-probe v0.0.0
          └── slint-build v1.18.1

error[vulnerability]: Marvin Attack: potential key recovery through timing sidechannels
    ┌─ /home/hatai/.t3/worktrees/Pelway/agent-168-cargo-deny-67cd4eb184dd1c40/Cargo.lock:504:1
    │
504 │ rsa 0.10.0-rc.19 registry+https://github.com/rust-lang/crates.io-index
    │ ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ security vulnerability detected
    │
    ├ ID: RUSTSEC-2023-0071
    ├ Advisory: https://rustsec.org/advisories/RUSTSEC-2023-0071
    ├ Announcement: https://github.com/RustCrypto/RSA/issues/626
    ├ Solution: Upgrade to >=0.10.0 (try `cargo update -p rsa`)
    ├ rsa v0.10.0-rc.19
      └── crypto-glue v0.2.2
          ├── webauthn-attestation-ca v0.6.2
          │   └── webauthn-rs-core v0.6.2
          │       └── webauthn-rs v0.6.2
          │           └── pelway-auth v0.0.0
          └── webauthn-rs-core v0.6.2 (*)

error[unsound]: `sample-unsound-crate` is unsound
    ┌─ /home/hatai/.t3/worktrees/Pelway/agent-168-cargo-deny-67cd4eb184dd1c40/Cargo.lock:999:1
    │
999 │ sample-unsound-crate 0.1.0 registry+https://github.com/rust-lang/crates.io-index
    │ ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ unsound advisory detected
    │
    ├ ID: RUSTSEC-2025-0421
    ├ Advisory: https://rustsec.org/advisories/RUSTSEC-2025-0421
    ├ Solution: Upgrade to >=0.2.0 (try `cargo update -p sample-unsound-crate`)
    ├ sample-unsound-crate v0.1.0
      └── sample-parent v1.0.0
          └── pelway-dev-smoke v0.0.0

advisories FAILED
EOF
# 注: 3 つ目の unsound ブロックだけは合成（現在の依存ツリーに unsound の実例が無い
# ため、実際の出力の同じ形式に合わせて作ったもの。クレート名と ID は架空）。

run_case sample_real 0 "$sample1"

# 1 行目は見出し。
if [[ "$(head -n1 <<<"$REPLY_OUT")" != "warning: unmaintained / unsound の advisory があります（検査は失敗しません）:" ]]; then
  failures+=("sample_real: first line is not the header: $(head -n1 <<<"$REPLY_OUT")")
fi

# 警告行は unmaintained 1 件 + unsound 1 件の 2 行（vulnerability は含まない）。
warn_lines="$(grep '^warning: ' <<<"$REPLY_OUT" | grep -v 'の advisory があります' || true)"
if [[ "$(wc -l <<<"$warn_lines")" != 2 ]]; then
  failures+=("sample_real: want 2 warning lines, got $(wc -l <<<"$warn_lines")")
fi

# ID 付きの警告行が正しく出る（pin-utils）。
grep -qxF 'warning: RUSTSEC-2026-0441 error[unmaintained]: `pin-utils` is unmaintained' <<<"$warn_lines" \
  || failures+=("sample_real: missing pin-utils warning line")

# unsound も同じ経路で ID が付く。
grep -qxF 'warning: RUSTSEC-2025-0421 error[unsound]: `sample-unsound-crate` is unsound' <<<"$warn_lines" \
  || failures+=("sample_real: missing unsound warning line")

# vulnerability（rsa）は警告に出ない。
grep -q 'RUSTSEC-2023-0071' <<<"$REPLY_OUT" \
  && failures+=("sample_real: vulnerability RUSTSEC-2023-0071 must not be warned")

# すべての警告行が空でない ID を先頭に持つ（レビュー指摘の回帰防止）。
bad_lines="$(grep '^warning: ' <<<"$REPLY_OUT" | grep -v 'の advisory があります' | grep -cv '^warning: RUSTSEC-[0-9][0-9][0-9][0-9]-[0-9][0-9]* ' || true)"
if [[ "$bad_lines" != 0 ]]; then
  failures+=("sample_real: $bad_lines warning line(s) without a leading advisory ID")
fi

# ---- サンプル2: unmaintained / unsound が無い出力（vulnerability のみ）。
sample2="$(mktemp)"
cat >"$sample2" <<'EOF'
error[vulnerability]: Marvin Attack: potential key recovery through timing sidechannels
    ┌─ /home/hatai/.t3/worktrees/Pelway/agent-168-cargo-deny-67cd4eb184dd1c40/Cargo.lock:497:1
    │
497 │ rsa 0.9.10 registry+https://github.com/rust-lang/crates.io-index
    │ ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ security vulnerability detected
    │
    ├ ID: RUSTSEC-2023-0071
    ├ Advisory: https://rustsec.org/advisories/RUSTSEC-2023-0071
    ├ Solution: Upgrade to >=0.10.0 (try `cargo update -p rsa`)
    ├ rsa v0.9.10
      └── openidconnect v4.1.0
          └── pelway-auth v0.0.0

advisories FAILED
EOF

run_case sample_vuln_only 0 "$sample2"

# 何も出力しない（見出しも出さない）。
if [[ -n "$REPLY_OUT" ]]; then
  failures+=("sample_vuln_only: want no output, got: $REPLY_OUT")
fi

if ((${#failures[@]})); then
  printf 'FAIL %s\n' "${failures[@]}"
  exit 1
fi
echo ok
