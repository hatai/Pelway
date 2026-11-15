# 実装者メモ: Issue #347

- Issue: [#347](https://github.com/hatai/Pelway/issues/347)
- PR: [#348](https://github.com/hatai/Pelway/pull/348)
- 対象 head: bc855eab2d204366c21a04f9c0e13205df1d48c4（ガード実装コミット・署名付け替え後）

## 変更 ↔ 一次記録の引用 ↔ 期待する変化

| 変更 | 根拠（一次記録） | 期待する変化 |
|---|---|---|
| `scripts/agent-gh`: 引数に `\u`+16進4桁のリテラルを検出したら終了コード 5・stderr 案内で止める（トークン発行前） | `docs/worklogs/issue-308.md`: 日本語の引数が `\uXXXX` にエスケープされて届く障害と「ファイル経由で渡す」回避策。2026-11-15 指揮役ログ: PR #311/#314/#319 で再発。本 Issue 起票の直接の契機: `issue create --title` で起票した Issue #340–#345 のタイトルがすべて `\uXXXX` リテラルで登録された | 潰れた文字列が GitHub に確定する前に止まり、案内どおりファイル経由に切り替えられる。修復（タイトルの PATCH し直し等）の手間が消える |
| `scripts/agent-gh` help・ヘッダの終了コード一覧に 5 を追加 | 同上（案内の一貫性） | 拒否された利用者が正しい渡し方を即座に知れる |
| `scripts/tests/agent-gh.test.sh`: dev/review/git 経路で終了コード 5・fnox/gh/git 未呼出・案内出力を検査、`\u`+3桁と大文字 `\U` の誤検知なしを検査 | 既存テストの形（偽 fnox/gh/git）に合わせた | ガードの退行を CI の script-test で検出できる |
| `.agents/skills/write-issue/SKILL.md`: 非 ASCII タイトルは `api ... -X POST --input` で起票する一文を追加 | 今回の踏んだ経路がまさにこの手順（`issue create --title`）だった | 手順レベルでも同じ落とし穴を踏みにくくする |

## 対象外（意図的にやらなかった）

- `--title` を argv で教えている他の箇所（`write-pull-request`、`implement-issue` の `pr create`）: skill 文書側の網羅的な整備は Issue #343 の範囲。本変更の guard が全経路を実行時に止めるため、文書整備は #343 に委ねる
- 引数の自動デコード（`\uXXXX` → 文字）: 意図的な `\uXXXX` リテラル（このガード自体のテスト等）を壊す誤判定になるため、拒否＋案内にした

## 検証

- `bash scripts/tests/agent-gh.test.sh` → ok（新規 5 ケース含む）
- `bash scripts/tests/skill-self-containment.test.sh` / `board-patrol-docs.test.sh` → ok
- `diff -r .claude/skills/write-issue .agents/skills/write-issue` → 差分なし（symlink）
- Issue #340–#345 のタイトルは `api -X PATCH --input` で修復済み（この修正とは別操作）

## 詰まった箇所・仮定

- exec 層が `$変数` を食うため、タイトル修復はスクリプトファイル（/tmp/fix-titles.sh）で実行
- 仮定: 拒否にしたため、正当に `\uXXXX` リテラルを渡したい場合はファイル経由（`-F` / `--input` / heredoc）が必要。エラーメッセージにその旨を記載済み
