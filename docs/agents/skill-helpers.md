# Skill helpers: スキル同梱のヘルパーとテストの規約

スキルの本文(`SKILL.md`)は判断と手順の指示を書く場所で、検証可能な部分は実行可能なヘルパーとそのテストとして `SKILL.md` と同じディレクトリに同梱できる。機械な手順がヘルパーに落ちたらテストを付けて同梱し、スキルの振る舞いをテストで守る。

- 原型: elie222/rakazo の `.agents/skills/pr-watch`（pr-digest とそのテスト）と `.agents/skills/translate`（catalog-gaps とそのテスト）
- Pelway での実例: `.agents/skills/pr-watch/pr-digest` と `pr-digest.test.sh`（#217）

## 同梱できるものの範囲

- スキルの手順のうち機械な部分（集計、検査、変換、整形、条件待ち）をヘルパーに落とせる。落ちたものは同梱する。`SKILL.md` には判断（何を選ぶか、何を迷うか）とヘルパーの呼び出し方を書き、ヘルパーが計算・変換の中身を担う。
- 文章でしか書けない判断（コードの良し悪し、指摘の重み）は無理にコード化しない。
- 同梱は skills-lock.json が管理しないローカル管理のスキルに限る。skills-lock.json が外部リポジトリ（actionbook/rust-skills、penpot/penpot-ai-kit）からハッシュで固定するスキルには足せない（更新時に上書きされて消える）。
- ヘルパーは bash スクリプト（`#!/usr/bin/env bash`、拡張子なし、実行可能ビット付き）。名前は動作を言う短い kebab-case 英語（pr-digest、catalog-gaps）。依存は bash と POSIX の標準コマンド（grep、sed、awk、sort、comm、cut、head、tail など）、git、jq、scripts/agent-gh の範囲に限り、追加のランタイムやパッケージ（node、python、追加の cargo ツールなど）を持ち込まない。
- GitHub との通信は `scripts/agent-gh dev` 経由（AGENTS.md の決まりと同じ）。

## テストの置き場と実行方法

- テストはヘルパーと同じディレクトリに `<ヘルパー名>.test.sh` として置く（pr-digest なら pr-digest.test.sh）。
- テストの形式は `scripts/tests/*.test.sh` と同じ: `set -uo pipefail`、`mktemp -d` の一時ディレクトリの中で閉じて動く、外部コマンド（gh、agent-gh）はモックに差し替える、失敗を配列に溜めて最後に一括報告し、1 つでもあれば非 0 で終わる。
- 実行は `mise run script-test`。ヘルパーを同梱したら mise.toml の `tasks.script-test` の列挙に test.sh を足す。`mise run verify`（verify-test）と CI で走る。

## 両側ツリーでの同梱物の扱い

- 実体は `.agents/skills/<name>/` だけに置く。`.claude/skills/<name>` はディレクトリごとのシンボリックリンクなので、ヘルパーもテストも両側から同じ物が見える。
- スキルに変更を入れたら `diff -r .claude/skills/<name> .agents/skills/<name>` が空であることを確かめる（AGENTS.md の決まりと同じ）。
- ヘルパーとテストの行数は pr-size に数える。`.agents/skills/**` は .gitattributes で linguist-generated が付くため、ヘルパーを同梱するスキルは `.gitattributes` の `-linguist-generated` 列挙に入れて行数を可視化する（pr-watch がこの形）。

## 同梱の候補（機械な手順を持つ既存スキル）

| スキル | 機械な手順 | ヘルパーの案 |
|---|---|---|
| command-team | 振り分け: pr-size の行数・変更パス・ラベル・修正回数からモデルと effort を決める表の適用。高リスク判定（unsafe、セキュリティ境界、workflows、追加 400 行超）。Codex の見落とし率の集計 | 行数・パス・ラベルから振り分け行と高リスク判定を返す |
| review-pr | high-risk 判定（security-review の起動条件）、PR のチェックの検査コマンド群 | PR の差分から high-risk トリガーを列挙して返す |
| write-pull-request | 「事実を集める」: verify の出力、pr-size の行数、コミット一覧の整形 | 差分とコマンド出力から PR 本文の材料を整形 |
| write-issue | 本文の形式検査: 見出しの構成、受け入れ条件に検証方法が付くかの形チェック | Issue 本文を受け取り形式の欠落を列挙 |
| implement-issue | 実装者メモの形式検査（5 項目の存在、引用の存在） | メモを受け取り欠落項目を列挙 |

各候補の実装は候補ごとに別 Issue で行う。この表は候補であって作業計画ではない。
