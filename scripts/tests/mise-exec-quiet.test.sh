#!/usr/bin/env bash
# Issue #616: mise exec・mise run を呼ぶたびに rust の同期の行・mise lock の一覧・
# WARN missing: rust@<版> が出ないこと（#596）の回帰を防ぐテスト。
# mise は mise.toml の rust.components の各名前を `rustup component list --installed` の一覧
# （<名前>-<host>、またはターゲットに依存しない成分は <名前> の形）に探し、1 つでも
# 見つからなければ rust を入れ直して上の行を出し、
# 入れ直した後も見つからなければ WARN missing を出す。rustup が別名で入れる成分
# （llvm-tools-preview → llvm-tools）を書くと毎回これが起きる。
# 出力の行ではなくこの原因を確かめる: 出力は、古い成分名の別の checkout が共有の
# ツールチェーンを同時に入れ直しているとこちらでも出るため、手元で結果が揺れる。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT" || exit 1

rust_line="$(grep -E '^rust = \{' mise.toml | head -n1)"
version="$(sed -nE 's/.*version = "([^"]+)".*/\1/p' <<<"$rust_line")"
components="$(sed -nE 's/.*components = \[([^]]*)\].*/\1/p' <<<"$rust_line" | tr -d '" ' | tr ',' ' ')"
if [[ -z "$version" || -z "$components" ]]; then
  echo "FAIL mise.toml: cannot read rust version/components from: ${rust_line:-<no rust line>}"
  exit 1
fi

host="$(mise exec -- rustc -vV 2>/dev/null | awk '/^host:/{print $2}')"
if ! installed="$(mise exec -- rustup component list --installed --toolchain "$version" 2>&1)" || [[ -z "$host" ]]; then
  echo "FAIL cannot list installed components of rust ${version} (host '${host}'): $(head -n1 <<<"$installed")"
  exit 1
fi

failures=()
for c in $components; do
  # rust-src のようにターゲットに依存しない成分は <名前> だけで並ぶ。
  grep -qxF -e "${c}-${host}" -e "$c" <<<"$installed" || failures+=("$c")
done
if ((${#failures[@]})); then
  echo "FAIL mise.toml rust.components not in rustup's installed list (mise reinstalls rust on every exec): ${failures[*]}"
  exit 1
fi
echo ok
