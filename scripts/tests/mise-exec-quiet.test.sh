#!/usr/bin/env bash
# Issue #616 の受け入れ条件を確かめるテスト: 導入済みの環境で mise exec を続けて呼んだとき、
# 2 回目の出力（stdout と stderr）に、ツールの入れ直しの行（rust の同期・mise lock の一覧）と
# WARN missing が無いこと。mise.toml の rust.components の名前が rustup の一覧の名前と
# ずれると（llvm-tools-preview。#596）、mise は rust を毎回入れ直してこれらを出す。
# 出力が空であることは求めない（mise の別の正当な警告で落ちないようにするため）。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

mise exec -- true >/dev/null 2>&1
out="$(mise exec -- true 2>&1)"
rc=$?
if [[ "$rc" -ne 0 ]]; then
  printf 'FAIL mise exec -- true exited %s: %s\n' "$rc" "$(head -n3 <<<"$out")"
  exit 1
fi
noise="$(grep -E '^mise (lock |rust@|by @jdx .* installing|WARN +missing:)' <<<"$out")"
if [[ -n "$noise" ]]; then
  printf 'FAIL the second mise exec -- true reinstalled tools or warned missing:\n%s\n' "$(head -n5 <<<"$noise")"
  exit 1
fi
echo ok
