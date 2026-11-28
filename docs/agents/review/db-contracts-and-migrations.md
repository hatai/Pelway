---
name: db-contracts-and-migrations
paths:
  - /crates/pelway-db/**
  - /crates/pelway-contract/**
default_class: must-fix
attention: HIGH
---

# DB の移行と契約のレビュー観点

DB の schema、migration、API・ドメイン契約では、データの整合性・tenant 境界・監査の証拠性を優先して確かめる。指摘は、具体的な入力・role・SQL・transaction で壊れる筋道が確認できるものに絞る。

## 報告するもの

### must-fix で報告するもの

- 監査行や append-only の行について、関係する列・参照を認証対象から外したまま、改ざん・再関連付け・削除ができる。
- 監査対象の role が監査行を変更・削除できる、または管理用 role が監査付きの経路を迂回できる。
- RLS・role・ownership の前提が崩れ、tenant 境界または権限境界を越えられる。
- migration が atomic ではない、または idempotent ではなく、失敗時に schema や data を半端な状態に残す。
- contract の validation・serialization・reply guard が、正準形または境界形の不正入力を通す、または正当な入力を拒否する。
- 変更が主張する schema・contract の invariant が、DDL・型・validation の少なくとも1つで強制されていない。

### nit で報告するもの

- 正しさ・互換性・監査に影響しない、今回の差分で入った命名・構造・説明の分かりにくさ。

## 報告しないもの

- invariant を満たしている実装形の違いは報告しない。invariant 自体が欠けている場合は、その invariant を報告する。
- must-fix の中に条件付きで紛れ込ませた任意の hardening は must-fix にしない。独立した nit として価値がある場合だけ報告する。
- 具体的な攻撃・失敗の筋道がない広い設計変更要求は書かない。筋道がある場合は報告し、範囲が大きい場合は分割または判断待ちにする。
- role・入力・SQL・transaction の具体的な筋道を示せない migration の不安は書かない。筋道を特定できた場合は報告する。

## 証拠にするもの

- 誰が、どの入力・SQL・transaction で、何を壊せるか。
- どの行・列・参照・invariant が壊れるか。
- DDL・型・validation・test のどこで防がれるか。
- classifier や guard には、正準形、日本語や Markdown を含む境界形、否定形、代表的な JSON の形。
