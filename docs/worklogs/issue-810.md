# Issue #810 の作業ログ

## 実装者メモ（round 1）

対象 head: 1888f852c9bdeb7f4c81025042ad0d25b89c67ad

1. 詰まった箇所: 手順 6: `mise run verify` の初回（20:29 JST）は、並行して `scripts/red-check`（一時 worktree の作成と削除）を走らせている最中に、書き出した直後の `.rmeta`・`.o` が見つからなくなる形のビルド失敗で終わった（exit 101）。`red-check` と並行させずに同じコマンドでやり直すと exit 0 になった（`Finished in 208.94s`）。並行実行が原因かは確かめていない。
2. 曖昧だった文: 「Issue 番号・新しい本文のファイル・コメントの文を受け取り、本文を書き換えてからコメントを付けるコマンドがある」（#810 の受け入れ条件）。コマンドの置き場所・名前・引数の形が書かれていない。
3. 仮定と解釈: 関連情報の `scripts/agent-gh` と `qa comment <番号> --body-file <path|->` の形にならって、`scripts/agent-gh dev issue rewrite <番号> --body-file <path|-> --comment-file <path|->` に置いた。「同じであること」は末尾の改行を除いて比べる（`gh --jq .body` の出力は末尾に改行が 1 つ付く。#810 で実測: 本文 2320 バイトに対し出力は 2321 バイト）。読み直した本文が違うときの終了コードは 6 にした。空の本文・コメントは送る前に終了コード 2 で拒む（受け入れ条件に無いが、空の本文で Issue の本文を消すのを防ぐため）。
4. 足りなかった情報・検証手段: 偽の gh では、App のトークンで `gh issue edit --body-file` が通ることは確かめられない。#810 の本文を同じ内容で実物の GitHub に書き換えて確かめた（[道具の動作確認のコメント](https://github.com/hatai/Pelway/issues/810#issuecomment-7190329080)。前後で本文は同一の 2320 バイト）。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「決まり」の「素の `gh` と素の `git push` は使わない」を「素の `gh` と、ネットワークへ出る素の git（`fetch`・`pull`・`push`）は使わない」に直す。

使ったスキル: `implement-issue`、`write-pull-request`（Draft PR の作成）、`ja-text-communication`

## 実装者メモ（round 2）

対象 head: 1246e82c1ec0f28ef657794146694123b2779323

1. 詰まった箇所: `scripts/agent-gh` の `issue_rewrite_check_args`・`issue_rewrite`（round 1 の実装）: 入力ファイルを検査したあとに同じパスを gh に渡していた。検査後に本文ファイルが空になると、空の本文が送られて Issue の本文が消える（Codex の must-fix 1）。本文とコメントを `cat --` で一時ファイルに写し、写しを検査して送るようにした。読み直しの比較はコマンド置換で末尾の改行をすべて除いていた（Codex の must-fix 2）。写しに gh の `--jq` が足す改行 1 つを足したものと `cmp` で比べるようにした。
2. 曖昧だった文: 「出力形式が足した改行だけを取り除き、本文自身の改行は保持して比較してください」（PR #837 の round 1 のレビュー、Codex の must-fix 2）。読み直した出力から改行を取り除く代わりに、渡した本文の写しに同じ改行を足して比べた（GNU 専用の `head -c -1` に頼らないため）。結果は同じになる。
3. 仮定と解釈: `[文書のずれ]` の nit は Codex と Claude で同じ 2 か所（skill の手順 7 の「0 以外で終わったとき」の説明と、`--help` の「本文とコメントが空のとき」）だったので、両方を直した。Claude の印の無い nit（`cat` の `--`）は must-fix 1 の中で直した。main の取り込みはしていない（`mergeable` が MERGEABLE で、main の変更はこの PR のファイルに触れない）。
4. 足りなかった情報・検証手段: 本物の GitHub が `gh issue edit --body-file` の本文の末尾の改行を正規化するかは、末尾の改行が 2 つ以上の本文では確かめていない。読み取りだけでは、リポジトリの Issue・PR 842 件の本文のうち末尾の改行が 2 つ以上のものが 101 件あった（#834 は 3 つ、#833 は 6 つ）。末尾の改行 1 つが保たれることは round 1 の実行で確かめた。正規化されていれば終了コード 6 でコメントは付かない。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「受け入れ条件の書き方」に「『同じであること』を条件に書くときは、比べないでよい差（末尾の改行・空白など）を明記する。書かなければバイト単位で同じこととする」を足す。

使ったスキル: `implement-issue`、`write-pull-request`、`ja-text-communication`

## 実装者メモ（round 3）

対象 head: 86873f7c71a9f3551775171dca7e8fa405cb2c32

1. 詰まった箇所: `scripts/agent-gh` の `issue_rewrite`（round 2 の実装）: 読み直した本文との比較を `{ cat -- "$issue_rewrite_body"; printf '\n'; }` と書いていた。写しを読めなくても `printf` が成功するため左辺は成功を返し、読み直した本文が空だと両辺が改行 1 バイトだけになって等しいと判定された（Codex の must-fix）。`cat -- "$issue_rewrite_body" && printf '\n'` に直した。
2. 曖昧だった文: 「同じ種類の must-fix が 2 round 続いたら（同じ種類かは自分で判断する）、指摘された経路だけを直さず、その種類の全経路に当てる決まりを決めてから直し、決めた決まりを実装者メモの「仮定と解釈」に書く」（skill `implement-issue` の「Request changes のとき」）。round 1 の must-fix 2（末尾の改行を除いて比べる）と今回の must-fix を、同じ種類（比較が誤って等しいと返す）と数えた。
3. 仮定と解釈: 決めた決まりは「読み直しの比較は、両辺を最後まで読めて一致したときだけ等しいと判定する。材料を読めない・比べられないときは、すべて違うとしてコメントを付けない」。写しを読めない経路は `&&` で受けて、試験 `copy_unreadable` で確かめた。読み直しの呼び出しの失敗は試験 `view_fails` で確かめている。読み直した本文のファイルを読めないときは `cmp` が 0 以外で倒れる。写しが検査の後に同じユーザーの別プロセスに書き換えられる経路は対象外とした（写しは `mktemp` の 0600・乱数名で、同じユーザーは道具自身も変えられるため）。Codex の印の無い nit（`calls` は一時ファイルのパスに空白を許さない）は直していない。
4. 足りなかった情報・検証手段: 読み直した本文のファイルを読めない経路（`cmp` が 0 以外で倒れる）は、試験で再現していない。
5. 改善案: `docs/agents/skill-helpers.md` の「テストの置き場と実行方法」に「試験は `TMPDIR` に空白を含むパスを指定しても通るように書く（呼び出しの引数を 1 行の文字列に組み立てて比べない）」を足す。

使ったスキル: `implement-issue`、`write-pull-request`、`ja-text-communication`

## 実装者メモ（round 4）

対象 head: f434b4f6c512b69c7f9028c505d7b2bb7a180b9d

1. 詰まった箇所: `scripts/agent-gh` の `issue_rewrite` のエラー文と、ファイル冒頭のコメント・`--help` の終了コード 6 の説明（round 3 の実装）: 「読み直した本文が渡した本文と違う」場合だけを書いていて、比べる材料を読めない場合も同じ 6 で返す実装と食い違っていた（Codex の `[文書のずれ]` の nit）。「読み直した本文と渡した本文の一致を確認できなかった」に揃えた。`scripts/tests/agent-gh-issue-rewrite.test.sh` の `FAKE_GH_TRIM_NL` の説明も、偽 gh の動き（末尾の改行を 1 つに整えて保存する）と食い違っていた（同 nit）。
2. 曖昧だった文: 「有効な Approve のレビュー（複数の担当ならその全員）の nit に、行頭が `[文書のずれ]` のものが残っていれば、`pr ready` の前に直す」（skill `implement-issue` の「Approve のとき」）。Claude の nit はなしで、Codex に印の付いた nit が 2 件あったため、両方を直してから Draft を解除することにした。
3. 仮定と解釈: この round の変更は文面だけ（コメント・ヘルプ・エラー文・試験の説明）で、終了コードと gh の呼び出しは変わらない。印の nit だけを直す round の通算 1 回目として数えた。push で head が動くため両担当の Approve は現 head に向かなくなり、差分が `scripts/agent-gh` を含むことから、レビューは指揮役のレビュースレッドでやり直しになる。
4. 足りなかった情報・検証手段: エラー文やヘルプの文面を確かめる試験は無い（試験は終了コードと呼び出しだけを見る）。変更した文面は `scripts/agent-gh --help` の出力と、試験と同じ偽の fnox・gh で `copy_unreadable` と同じ場面を実行したときの標準エラー出力を読んで確かめた。
5. 改善案: `.agents/skills/command-team/SKILL.md` の「レビューのラウンド」に、Approve 後の `[文書のずれ]` の修正がコメント・ヘルプ・文書だけのとき（試験と実行の経路が変わらないこと）に、再レビューの範囲をどう決めるかを書く。

使ったスキル: `implement-issue`、`write-pull-request`、`ja-text-communication`
