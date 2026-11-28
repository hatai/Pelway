#!/usr/bin/env bash
# PR 本文の「失敗の筋」と「リスクとセキュリティへの影響」（Issue #795）の文言を固定するテスト。
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

# PR テンプレート: 2 つの欄と必須形式。
check pr_template_sections .github/pull_request_template.md \
  '## 失敗の筋' \
  '## リスクとセキュリティへの影響' \
  '`- 失敗の筋:` と `確かめ方:` の 2 行組' \
  '`- 影響:` と `対応:` の 2 行組' \
  '該当なしのときは `- なし（理由: <理由>）` と書く' \
  '攻撃手順や未修正の弱点の詳細は書かない'

# write-pull-request: 8 セクションの番号と、失敗の筋・リスク欄の書き方。
check write_pr_sections .agents/skills/write-pull-request/SKILL.md \
  '1. **概要**' \
  '2. **変更内容**' \
  '3. **確認方法/検証**' \
  '4. **失敗の筋**' \
  '5. **リスクとセキュリティへの影響**' \
  '6. **レビューと QA**' \
  '7. **対象外**' \
  '8. **対象**'

check write_pr_failure_risk .agents/skills/write-pull-request/SKILL.md \
  '最終版の設計に関係する筋は、試験や確認で防げていても残し' \
  '不正な入力や状態、中断、部分書き込み、権限の失敗、再試行と冪等性、リソース枯渇、古い状態' \
  '作業ログの失敗の筋は出発点であり、転記せず、最終的な diff と検証結果に照らして書き直す' \
  '認可と読める人の規則、秘密と認証情報、データの境界とプライバシー' \
  'GitHub Actions・CI・フック、agent の指示と信頼できない入力、入力の上限とリソース枯渇、データの消失と整合性' \
  '影響が大きいほど、対応と根拠を具体的に書く' \
  '攻撃手順、悪用できる入力、未修正の弱点の詳細は書かない' \
  '失敗の筋は「どう壊れるか」、リスクとセキュリティへの影響は「壊れたときや悪用されたときの影響」を書く'

# implement-issue: 実装前・round ごとの見直し・最終版の記録。
check implement_failure_modes .agents/skills/implement-issue/SKILL.md \
  'テストを書く前に、`docs/worklogs/issue-<N>.md` に `## 失敗の筋（実装前）` を書く' \
  '挙動を変える 2 round 目以降では、コードを直す前に `## 失敗の筋の見直し（round <R>）` を書く' \
  'Draft PR を作る前に `## 失敗の筋（最終）` を書く' \
  'round ごとの最終版は消さない' \
  '「失敗の筋」と「リスクとセキュリティへの影響」も、その round の最終版と現在の差分に合わせる'

# review-pr: 構造の欠落と意味の矛盾の判定、作業ログの確認、出力行。
check review_failure_risk .agents/skills/review-pr/SKILL.md \
  'PR 本文の「失敗の筋」と「リスクとセキュリティへの影響」を読む' \
  'diff が示さない網羅性の不足や表現の好みは nit にする' \
  'PR 本文の失敗の筋またはリスクとセキュリティへの影響の欄が決まりを満たしていない（手順 2）' \
  '失敗の筋の確かめ方: 問題なし | 指摘あり' \
  '記録を書いた時刻の証明までは求めない'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "pr-failure-risk-docs: ok"
