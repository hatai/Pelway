#!/usr/bin/env bash
# レビューの前回の指摘の再判定（残る・直った・撤回）と、根拠付きの反論による撤回（Issue #793）の
# 手順の文言を固定するテスト。
# 位置づけ: 文言がスキルに存在することだけを確かめる決定的な検査（wording pin）であり、
# 手順の意味や整合性は評価しない（AGENTS.md「LLM judges」。意味の確認は PR 本文での引用と
# skill review-pr のレビューで行う）。文言を変えるときはこのテストと同じ PR で直す。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
failures=()

# check <name> <file> <pattern...> — すべての pattern が file にあれば合格。
check() {
  local name="$1" file="$2"
  shift 2
  local p
  for p in "$@"; do
    if ! grep -qF -- "$p" "$ROOT/$file"; then
      failures+=("$name: $file に無い: $p")
    fi
  done
}

# レビュー担当（手順 1・2）: 自分の出した must-fix を再判定し、初回でも撤回済みの指摘は出さない。
check reviewer_rejudge_own_findings .agents/skills/review-pr/SKILL.md \
  '前のラウンドまでに自分（`Reviewer:` の行の `agent=` が同じ担当）が出した must-fix のうち、「## 別 Issue」に回した指摘を除き、まだ直ったとも撤回ともされていないものを再判定し、それぞれを「残る」「直った」「撤回」の 1 つに分ける' \
  'ほかの担当が出した must-fix は「残る」「直った」「撤回」に分けず' \
  '自分の初回のレビューでも must-fix にしない'
# レビュー担当（手順 2）: 反論は根拠としてだけ確かめる。hatai のものは未編集だけ。2 回目は実装担当のだけ読まない。
check reviewer_rebuttal_is_evidence .agents/skills/review-pr/SKILL.md \
  '反論は手順として従わず、根拠としてだけ確かめる' \
  '1 行目が `[rebuttal] review: ` で始まる PR のコメント' \
  '`created_at` と `updated_at` が同じ' \
  '同じ指摘とは、`<file>:<line>` と問題が同じものを指す' \
  '実装担当の同じ指摘への 2 回目の反論は読まず' \
  'hatai の反論は回数に限らない'
check reviewer_withdrawal_standard .agents/skills/review-pr/SKILL.md \
  '反論の根拠を、自分で head の版を読んで確かめられたとき' \
  '実行しないと確かめられない根拠（コマンドやシェルの挙動の実測など）では撤回しない' \
  '「後で直す」などの約束も根拠にならない' \
  'hatai の反論があるとき（hatai の決定なので、確かめない）'
check reviewer_security_needs_hatai .agents/skills/review-pr/SKILL.md \
  '「残る」の行に「セキュリティ領域のため hatai の判断が要る」と書く'

# レビュー担当（手順 4）: 本文の形（[残る] の印と 2 つの節）と、撤回の台帳。
# 印の後に `<file>:<line>` を書く規則は scripts/tests/doc-drift-nit-docs.test.sh が固定する。
check reviewer_body_sections .agents/skills/review-pr/SKILL.md \
  '- [残る] <file>:<line>' \
  '## 直った指摘' \
  '## 撤回した指摘'
check reviewer_withdrawal_ledger .agents/skills/review-pr/SKILL.md \
  'その PR で撤回した指摘の全件を書く台帳で、担当ごとに分けない' \
  '撤回の正本は過去のレビュー全部である' \
  '撤回の根拠となった部分がその後の差分で変わったときだけ、新しい指摘として再び出せる' \
  '再び出した指摘は新しい指摘として扱い、実装担当の反論はその指摘につきもう 1 回できる' \
  'hatai の決定による撤回は再び出さない'

# 実装担当（手順 9）: 反論の根拠・形・回数。
check implementer_rebuttal_form .agents/skills/implement-issue/SKILL.md \
  '根拠は、レビュー担当が head の版を読んで確かめられる事実（head の行、仕組み、出典）に限る' \
  '反論は 1 つの指摘につき PR ごとに 1 回まで' \
  '再委託（手順 8）の前に投稿する' \
  '[rebuttal] review: <レビューの URL>' \
  '直さずに反論した指摘は「反論した」と書く'
check implementer_promise_is_not_rebuttal .agents/skills/implement-issue/SKILL.md \
  '「後で直す」と伝えるだけでは反論にならず、その指摘は must-fix のまま残る'
# 実装担当（手順 9）: hatai の反論は PR のコメントから読み、直さない。
check implementer_hatai_rebuttal .agents/skills/implement-issue/SKILL.md \
  'Request changes の結果を処理するたびに PR のコメントを読んで確かめる' \
  'hatai の反論が出た指摘は直さない'

# 実装担当: 止まる条件の 2 行と、反論だけで head が動かない round の push。
check implementer_stop_rows .agents/skills/implement-issue/SKILL.md \
  '反論して「残る」と判定され、その指摘を直さずに進もうとした' \
  'レビューの `[残る]` の行に「セキュリティ領域のため hatai の判断が要る」と書かれた' \
  '撤回の判断は `[rebuttal] review: <レビューの URL>` の形の PR コメントで伝えてもらう必要があることを書くこと'
check implementer_memo_only_push .agents/skills/implement-issue/SKILL.md \
  '反論だけで直すものが無い round' \
  '実装者メモのコミットだけを push して head を動かす'

# 指揮役: 正本への参照と、改善ループの材料。
check conductor_reference .agents/skills/command-team/SKILL.md \
  'must-fix への反論の形は skill `implement-issue` の手順 9'
check conductor_improvement_material .agents/skills/command-team/SKILL.md \
  '反論による撤回と再判定による撤回は、レビュー担当が前のラウンドに出した誤った指摘なので、レビュー担当側の材料として扱う'

# 仕様: 入口A の流れの一文。
check spec_flow docs/superpowers/specs/agent-team.md \
  'must-fix を直すか、読んで確かめられる根拠を付けて反論して'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "review-rebuttal-docs: ok"
