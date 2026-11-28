---
name: tests
paths:
  - /crates/*/tests/**
  - /crates/dev-llm-eval/data/**
  - /client/**/*.test.ts
  - /client/**/*.test.tsx
  - /client/test/**
  - /client/tsconfig.test.json
  - /scripts/tests/**
  - /scripts/demo-check/selftest.sh
  - /scripts/script-test.sh
  - /.agents/skills/*/*.test.sh
  - /evals/**
  - /.agents/skills/*/evals/**
default_class: nit
attention: HIGH
---

# 試験のレビュー観点

test・eval・検証用 script は、名前や件数ではなく、主張する条件を実際に失敗させられるかで確かめる。受け入れ証拠と安全 gate では、oracle が否定形・無関係な記録・途中状態を成功と誤認しないかを優先する。

## 報告するもの

### must-fix で報告するもの

- grep・抽出・fixture・assertion の構造上、主張する条件では絶対に失敗しない test になっている。
- keyword や部分一致だけで、否定・無関係・失敗した記録を成功として受け入れる eval oracle になっている。
- target の identity、command、ref、exit status、順序、最終状態、必須の具体的値を oracle が確かめていない。
- 受け入れ条件が要求する観測結果ではなく、内部 property・model state だけを見ている。
- 受け入れ証拠または安全 gate の test が主張する failure mode に対応する negative fixture が無い。
- test の削除・skip・assertion の弱化を、効果のある形で隠している。

### nit で報告するもの

- invariant は確かめているが、今回の差分で入った test 名・fixture・setup が invariant を追いにくくしている。

## 報告しないもの

- targeted test が invariant を証明しているのに、網羅的な redundant test が無いことは報告しない。未検証の具体的な failure mode がある場合は報告する。
- 今回の差分や受け入れ条件に関係しない追加 test の希望は書かない。必要なら別 Issue にする。
- invariant が証明されている場合の test framework・unit/integration の配置の好みは報告しない。
- 必要な環境が無いことを、実装担当が試していない場合・環境が無い場合・Issue の要件が不可能な場合に分けずに繰り返さない。環境が無い場合は証拠と制約を示して判断を送る。

## 証拠にするもの

- test が失敗すべき入力、mutation、negative record。
- 主張する条件から assertion までの筋道。
- target、command、exit status、最終状態など、oracle が確かめるべき値。
- 環境依存の測定では、試した環境・使えない環境・再現手順。
