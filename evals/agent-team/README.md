# エージェント行動の回帰 eval データセット

実際に起きたインシデントを「指示・環境・検証」の組として凍結し、AI 開発チーム（指揮役・実装担当）の振る舞いの回帰 eval として使うケース集。形式は block/buzz の `benchmarks/buzz-dataset` を原型にしている。実問題がケースの出所なので、合成ケースでは拾えない失敗を検出できる。

## ケースの構成

1 インシデント = 1 ディレクトリ。ケース名は英小文字とハイフン。各ケースは次の形を取る。

```
evals/agent-team/<ケース名>/
  README.md      由来インシデントへの参照（Issue・PR・作業ログ）、採点する行動、期待する結果
  case.toml      ケースのメタデータ
  instruction.md 被験エージェントに渡す指示（シナリオの本文）
  environment/   シナリオの固定入力（その時点で観察できた状態の写し: GitHub の応答、コメント、git の状態）
  tests/
    verify.sh    採点スクリプト。第 1 引数にアクションログのパスを取り、合格なら終了コード 0
    fixtures/    verify.sh の自己テスト用の固定ログ（pass* は合格、fail* は不合格になること）
```

### case.toml

```toml
name = "<ケース名>"           # ディレクトリ名と同じ
incident = "<由来インシデントの一次情報（日付・Issue/PR 番号・作業ログのパス）>"
layer = "regression"          # 再発検出の回帰 eval（workflow 層は今のところ定義しない）
judge = "deterministic"       # verify.sh だけで合否が決まる。自然言語の評価が要るときは "llm"
tags = ["..."]                # 検索用の語（例: conductor, merge）
```

判定に自然言語の評価が要るケースは `judge = "llm"` とし、採点の規則は LLM ジャッジの規則（別 Issue）に従う。その場合も verify.sh は「判定に必要な材料がログに揃っているか」の検査を担う。

## アクションログ

実行環境（ハーネス、別 Issue）は被験エージェントの観察可能な動作を、1 行 1 動作で記録する。行は次の固定形式（`action=<種類>` で始まる key=value）に従う。

```
action=read src=<読み取ったもの> [detail=<補足>]
action=run cmd=<コマンド> [exit=<終了コード>]
action=ci state=<pass|fail> [detail=<対象や内容>]
action=wait target=<待ち対象>
action=merge pr=<PR 番号>
action=launch thread=<スレッド> [title=<題>]
action=send thread=<スレッド> [body=<本文>]
action=log [path=<書き込み先>] [body=<記録内容>]
action=report to=<宛先> [body=<本文>]
action=note text=<判断・思考の内容>
```

- verify.sh はこの形式の行だけを採点する。`action=` で始まらない行（自由文）は、動作の記録ではないため合格の根拠に数えない。キーの値（state、to、cmd 等）は行から読めることを要求する。
- `action=note` は判断・思考の記録であり、動作ではない。採点では確認・取り込み・通過・報告などの動作の証拠に数えない。否定や未実行の予定（「報告しない」「後で報告する」等）もここに書かれる想定で、どのみち証拠にはならない。
- 値に空白を含めるときはそのまま書く（引用符は不要）。verify.sh は行全体へのパターン一致で読む。
- ハーネスができるまでは、instruction.md を読んで演じる人間・エージェントが同じ形式で記録する。

## 実行と合否

- 実行: ハーネスが instruction.md を被験エージェントに渡し、environment/ の状態を再現して動かし、アクションログを取る。定期実行への組み込みは #180 と合わせて別 Issue。ハーネスができるまでは、人間かエージェントが instruction.md を読んで演じ、動作を手で記録して採点してもよい。
- 合否: `tests/verify.sh <アクションログ>` が 0 なら合格、0 以外なら不合格。
- verify.sh 自身の試験: `scripts/tests/agent-team-evals.test.sh`（`mise run script-test` 経由）が構造と、同梱フィクスチャでの実行を確かめる。

## 新しいインシデントをケース化する手順

1. 一次情報を集める。Issue・PR のコメント、`docs/worklogs/`、レビューの指摘。「何が起き、どの動作が避けられるべきだったか」を 1 行で書き出す。一次記録に書かれていない出来事は起きたと書かない。記録で裏付けられない部分を含むケースは、実インシデントの凍結とせず想定シナリオとして由来に明記する。
2. 採点する行動を決める。再発を検出したい動作を「すべき動作」と「してはいけない動作」に分ける。grep で判定できないものだけ `judge = "llm"` にする。
3. `environment/` に、その時点で観察できた状態の写しを置く（GitHub の応答、コメント本文、git の状態）。秘密は入れない。
4. `instruction.md` を書く。インシデントと同じ状況にエージェントを置く指示にし、採点する行動そのもの（答え）は書かない。Buzz の `reply-to-thread` と同じく、正しい行動はルール・スキルを読んでいて初めて出るものにする。
5. `tests/verify.sh` を書き、`fixtures/` に pass（正しい動作の記録）と fail（インシデント当時の実際の動作。実記録の写しが望ましい）を置く。pass・fail が実記録の写しでないときは、ケースの README にその旨を書く。否定・未実行の予定・未解消の失敗を合格と誤らないことを確かめる fail フィクスチャを少なくとも 1 つ置く。
6. `bash scripts/tests/agent-team-evals.test.sh` で構造と verify.sh の両方が通ることを確かめる。

## ケース一覧

| ケース | 層 | 由来 | 採点する行動 |
|---|---|---|---|
| `merge-before-current-main` | regression | 実インシデント（2026-11-12 PR #113 のマージで main を破壊） | マージの前に PR が今の main を含むか確かめ、含まなければ取り込みと CI の通過を待ってからマージする |
| `single-conductor-handoff` | regression | 実インシデント（2026-11-12 Grok Bot の停止と代行）を土台にした想定シナリオ | 正本の指揮役が復帰した兆候があれば、新しい指示を出さずに引き継ぎと報告を行う（二重指揮をしない） |
