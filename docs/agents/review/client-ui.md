---
name: client-ui
paths:
  - /crates/pelway-client-ui/**
  - /crates/pelway-client-core/**
  - /crates/dev-client-probe/**
  - /crates/**/*.slint
  - /client/apps/**
  - /client/packages/**
  - /client/test/**
default_class: nit
attention: HIGH
---

# クライアント UI のレビュー観点

クライアント UI では、model や property ではなく、利用者に見える状態・読み上げられる状態・実際に instantiate された要素を確かめる。必須の識別・表示が消えたり誤解させたりする問題を重く扱い、範囲外の見た目の希望と分ける。

## 報告するもの

### must-fix で報告するもの

- AI 識別・法的表示・契約上必須の文字が、clip・ellipsis・stretch・layout によって消える、読めない、離れる、または誤解させる。
- 既存の画面遷移や card・component の materialization が退行する。
- virtualization で row の identity・連続性・順序が崩れる、または model にある項目が実際には instantiate されない。
- clipping・固定幅・`max-lines`・property-only assertion のため、必須要素が消えていても test が通る。
- accessibility の label・読み上げ・role の契約が崩れ、必須内容が重複・欠落・誤解させる形で読まれる。

### nit で報告するもの

- 必須内容や accessibility に影響しない、今回の差分で入った token・spacing・visual consistency のずれ。
- invariant は確かめているが、環境差で不安定になりやすい今回の差分の assertion。

## 報告しないもの

- PR が悪化させていない既存の visual・accessibility・design の不足は報告しない。全局的な invariant を壊す場合・新しい UI を誤解させる場合は報告し、それ以外はレビューに載せない。必要ならレビューの外で別 Issue にする。
- 範囲外の design や accessibility の希望を毎 round の nit に繰り返し置かない。現行の diff で悪化した場合は報告する。
- 再現条件の無い Slint 内部動作の一般論は書かない。現在の UI invariant と結び付く具体例がある場合は報告する。
- font・OS・描画環境の差を考慮しない pixel-perfect の差異は報告しない。計測可能な invariant に反する場合は報告する。
- 表示・accessibility の契約を満たしている実装形の違いは報告しない。契約を満たしていない場合は、その契約を報告する。

## 証拠にするもの

- model state ではなく、geometry・visibility・instantiation・accessibility tree の観測結果。
- Slint の stretch・clip・viewport・estimated position・conditional element の挙動。
- version・layout nesting・binding 元・viewport size・言語・font 環境などの再現条件。
