#!/usr/bin/env bash
# Issue #161: coverage ジョブの PostgreSQL 復帰（#157 の集計除外の解除）の検査。
# CI（.github/workflows/verify.yml の coverage ジョブ）で pelway-db の RLS テストが
# スキップされず実行されるには、次の構造が要る（docs/dev-environment.md「カバレッジ」）:
#   1. coverage ジョブに services.postgres（nightly.yml の test ジョブと同じ image）がある
#   2. LCOV 生成（mise run coverage）のステップに PELWAY_DATABASE_URL がある
#   3. mise.toml の coverage タスクから --exclude pelway-db が外れている
# RLS テストが失敗しても品質ゲート（static / lint / test）に影響しないことは、
# coverage ジョブが verify 集約ジョブの needs に含まれない構造（変更しない）で担保する。
# ロール（pelway_app・pelway_admin・pelway_audit）はマイグレーション
# （0001_roles.sql・0004_admin_audit.sql）が advisory lock 付きで作るため、
# ここでは検査しない。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORKFLOW="$ROOT/.github/workflows/verify.yml"
NIGHTLY="$ROOT/.github/workflows/nightly.yml"
MISE="$ROOT/mise.toml"
failures=()

# verify.yml からジョブ 1 つ分のブロック（"^  <job>:" から次の同位ジョブまで）を取り出す。
job_block() {
  awk -v job="$1" '
    $0 == "  " job ":" { in_job = 1; next }
    in_job && /^  [a-z][a-z-]*:$/ { exit }
    in_job { print }
  ' "$WORKFLOW"
}

coverage="$(job_block coverage)"
[[ -n "$coverage" ]] || failures+=("verify.yml: coverage ジョブのブロックが見つからない")

# 1. services.postgres がある（AC: coverage ジョブに services: postgres が追加される）
if [[ -n "$coverage" ]]; then
  grep -q '^      postgres:$' <<<"$coverage" || \
    failures+=("verify.yml: coverage ジョブに services.postgres が無い")

  # 2. image が nightly.yml の test ジョブと同じ（docs/dev-environment.md の開発用と同等）
  nightly_image="$(grep -m1 '^[[:space:]]*image: postgres:' "$NIGHTLY" | sed 's/^[[:space:]]*//')"
  coverage_image="$(grep -m1 '^[[:space:]]*image: postgres:' <<<"$coverage" | sed 's/^[[:space:]]*//')"
  if [[ -z "$nightly_image" ]]; then
    failures+=("nightly.yml: postgres の image 行が見つからない（比較対象）")
  elif [[ "$coverage_image" != "$nightly_image" ]]; then
    failures+=("verify.yml: coverage ジョブの postgres image が nightly と違う: '$coverage_image' ≠ '$nightly_image'")
  fi

  # 3. LCOV 生成（mise run coverage）のステップに PELWAY_DATABASE_URL がある
  #    （AC: ジョブで PELWAY_DATABASE_URL が設定されテストがスキップではなく実行される）
  grep -q 'PELWAY_DATABASE_URL:' <<<"$coverage" || \
    failures+=("verify.yml: coverage ジョブに PELWAY_DATABASE_URL が無い")

  # 4. スキップを防ぐ接続先が空でない（URL として postgres を指す）
  db_url="$(grep -m1 'PELWAY_DATABASE_URL:' <<<"$coverage" | sed 's/.*PELWAY_DATABASE_URL:[[:space:]]*//')"
  [[ "$db_url" == postgres://* ]] || \
    failures+=("verify.yml: PELWAY_DATABASE_URL の値が postgres:// 接続文字列でない: '$db_url'")
fi

# 5. mise.toml の coverage タスクから --exclude pelway-db が外れている
#    （AC: cargo llvm-cov の --exclude から pelway-db が外れる。#157 の暫定除外の解除）
mise_coverage="$(awk '/^\[tasks\.coverage\]/{f=1; next} /^\[/{f=0} f' "$MISE")"
[[ -n "$mise_coverage" ]] || failures+=("mise.toml: [tasks.coverage] セクションが見つからない")
if [[ -n "$mise_coverage" ]]; then
  grep -qF -- '--exclude pelway-db' <<<"$mise_coverage" && \
    failures+=("mise.toml: coverage タスクに --exclude pelway-db が残っている（#157 の除外の解除漏れ）")
fi

# 6. coverage ジョブは品質ゲートから独立している（RLS テストの失敗が他ジョブに波及しない）
verify_needs="$(grep -A1 '^  verify:' "$WORKFLOW" | grep 'needs:')"
if grep -q 'coverage' <<<"$verify_needs"; then
  failures+=("verify.yml: verify 集約ジョブの needs に coverage が含まれている（独立性の喪失）")
fi

if ((${#failures[@]})); then
  printf 'FAIL %s\n' "${failures[@]}"
  exit 1
fi
echo ok
