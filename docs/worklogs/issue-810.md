# Issue #810 の作業ログ

## 実装者メモ（round 1）

対象 head: 1888f852c9bdeb7f4c81025042ad0d25b89c67ad

1. 詰まった箇所: 手順 6: `mise run verify` の初回（20:29 JST）は、並行して `scripts/red-check`（一時 worktree の作成と削除）を走らせている最中に、書き出した直後の `.rmeta`・`.o` が見つからなくなる形のビルド失敗で終わった（exit 101）。`red-check` と並行させずに同じコマンドでやり直すと exit 0 になった（`Finished in 208.94s`）。並行実行が原因かは確かめていない。
2. 曖昧だった文: 「Issue 番号・新しい本文のファイル・コメントの文を受け取り、本文を書き換えてからコメントを付けるコマンドがある」（#810 の受け入れ条件）。コマンドの置き場所・名前・引数の形が書かれていない。
3. 仮定と解釈: 関連情報の `scripts/agent-gh` と `qa comment <番号> --body-file <path|->` の形にならって、`scripts/agent-gh dev issue rewrite <番号> --body-file <path|-> --comment-file <path|->` に置いた。「同じであること」は末尾の改行を除いて比べる（`gh --jq .body` の出力は末尾に改行が 1 つ付く。#810 で実測: 本文 2320 バイトに対し出力は 2321 バイト）。読み直した本文が違うときの終了コードは 6 にした。空の本文・コメントは送る前に終了コード 2 で拒む（受け入れ条件に無いが、空の本文で Issue の本文を消すのを防ぐため）。
4. 足りなかった情報・検証手段: 偽の gh では、App のトークンで `gh issue edit --body-file` が通ることは確かめられない。#810 の本文を同じ内容で実物の GitHub に書き換えて確かめた（[道具の動作確認のコメント](https://github.com/hatai/Pelway/issues/810#issuecomment-7190329080)。前後で本文は同一の 2320 バイト）。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「決まり」の「素の `gh` と素の `git push` は使わない」を「素の `gh` と、ネットワークへ出る素の git（`fetch`・`pull`・`push`）は使わない」に直す。

使ったスキル: `implement-issue`、`write-pull-request`（Draft PR の作成）、`ja-text-communication`
