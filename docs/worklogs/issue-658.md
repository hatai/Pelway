# Issue #658 の作業ログ

## 実装者メモ（round 1）

対象 head: 464977c077204ef53c5da2979b16316b530aba34

1. 詰まった箇所: 手順 6: 1 回目の `mise run verify` が `deny-check` の advisory DB のロック取得失敗（`/home/hatai/.cargo/advisory-dbs/db.lock`、他の worktree の verify と共有）で exit=1。変更は SKILL.md のみで、再実行して切り分けた。
2. 曖昧だった文: 「上限超過時でも、指揮役・人間への相談（`<!-- pr-size-exception -->` の判断材料や縮小指示の対象）に必要なら push してよい」（Issue #658 の受け入れ条件）。push した後に止まるのか、PR まで作るのかが書かれていない。
3. 仮定と解釈: 相談の push の後も「止まる条件」の pr-size の行のとおり止まり（`needs-human`・`stopped`）、例外のラベルは PR に付くので PR が無ければ Draft PR を作る、と解釈した。例外判断の「着いた」は review-pr 手順 2 と同じ 2 点（ラベルと hatai 作の目印コメント）とし、例外ありの pr-size fail は委託してよいとした（「fail なら一律に委託しない」とは読まなかった）。
4. 足りなかった情報・検証手段: PR #610 round 7 の委託と返答の実物（Issue には要約だけ）。文書の意味を確かめる LLM judge が無く、受け入れ条件 1・2 は PR 本文の引用とレビューで判定する。
5. 改善案: `.agents/skills/command-team/SKILL.md` の「マージ」の pr-size の項に「実装担当が相談のために push した pr-size fail の PR は、例外を認めるか縮小を指示するまで `needs-human` のまま置く」を足す。

使ったスキル: なし

## 実装者メモ（round 2）

対象 head: 75fd0854a955783700082519d1c24abca5cd639a

1. 詰まった箇所: `.agents/skills/implement-issue/SKILL.md` 手順 8: 委託しない理由の括弧書きを Issue の背景（PR #610 round 7 で両担当が「pr-size fail のため未投稿」で返した）から書いたが、`review-pr` 手順 2 では verify fail・例外なしの pr-size fail は must-fix として Request changes を投稿する決まりで、両レビュー担当から `[文書のずれ]` の nit を受けた。
2. 曖昧だった文: 「レビュー担当の決まりどおりの挙動で委託が空振り」（Issue #658 の「背景・前提」）。決まり（`review-pr` 手順 2）と、#610 での実際の挙動（未投稿）が一致していない。
3. 仮定と解釈: 理由は `review-pr` の決まりを正として「must-fix に数えるため内容を読む前に結論が決まる」に直した。#610 の未投稿の挙動に合わせて `review-pr` を直す解釈は、対象外（review-pr は委託を止める差分の範囲外）として選ばなかった。
4. 足りなかった情報・検証手段: PR #610 round 7 のレビュー担当の返答の実物（未投稿とした根拠の行）。
5. 改善案: Issue の背景に挙動を書くときは、`.agents/skills/write-issue/SKILL.md` の「背景・前提」に「観察した挙動と、それを決める決まりの文（ファイル:行）を並べて書き、食い違えば食い違いと書く」を足す。

使ったスキル: なし
