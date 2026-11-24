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

## 実装者メモ（round 3）

対象 head: 7a7a8649d6ca247fdaf2ecbd7e90b7aa44d5f74c

1. 詰まった箇所: 手順 9「修正依頼」: implement-issue/SKILL.md 手順 8 の段落 1 か所が main と競合。main（PR #654・Issue #645）が同じ段落に required-checks helper による確認機構を入れていた。
2. 曖昧だった文: 「両立する変更は両側残す。片方を採るときは、落とした側と理由を round サマリーに書く」（`.agents/skills/implement-issue/SKILL.md` 手順 9）。同じ段落への両側の変更で、「両側残す」が文の連結か機構の選択かは決まっていない。
3. 仮定と解釈: ゲートの規則の文（#658 の受け入れ条件・レビュー済み文言）と helper による確認（#654）は両立すると解釈し、規則の文を残して確認方法を helper に統一した。旧 4 分岐・pr-digest 参照・例外定義の記述は helper が同じ内容を実装しているため main 側に揃えた（重複を残すと helper 改定時にずれるため）。
4. 足りなかった情報・検証手段: なし（helper の冒頭コメントと実装、コミット 737d49ae を読んで確認した）。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「背景・前提」に「同じファイル・節を変える未マージの PR があるかを確かめ、あれば本文に書く」を足す（#645 と #658 が同一段落を独立に変えて今回の競合になったため）。

使ったスキル: なし

## 実装者メモ（round 4）

対象 head: 19deb28b84e4f823eb4e3fba65c07498095f078c

1. 詰まった箇所: 手順 9「Approve のとき」: codex の `[文書のずれ]` nit 3 件の修正。3 件目で「required-checks の判定を確認」を巡回表の 2 行に書くか、判定を行う 2 の突き合わせに書くかが分かれた（両方に足す形を選んだ）。
2. 曖昧だった文: 「例外の有無で分岐する 401・403 行目では required-checks の判定も確認してください」（PR #663 の codex レビューの nit）。「その行に書け」と「その行の判定時に使え」のどちらとも読める。
3. 仮定と解釈: 行の適用を決める実状態の確認は 2 で行うため、2 に required-checks による確認を足し、指摘された 2 行の括弧にも required-checks への言及を足した（2 だけに足す解釈は、指摘行が黙ったまま再指摘を招く恐れがあるため選ばなかった）。
4. 足りなかった情報・検証手段: このセッションの `/home` が read-only マウントで `target`（mbx の共有キャッシュへの symlink）に cargo が書けず、`/tmp` の所有者が nobody（65534）で `verify-run.test.sh` の `dir_chain_safe` も失敗した。`CARGO_TARGET_DIR=/tmp/ptarget-658 TMPDIR=/dev/shm/tmp658` で verify 完走（47 tests ok）。
5. 改善案: `.agents/skills/implement-issue/required-checks` の冒頭コメントに「`pr-digest` の required 行は例外を反映しないため、巡回の実状態の確認（「巡回から定型の指示を受けたとき」の 2）でも使う」を足す。

使ったスキル: なし

## 実装者メモ（round 5）

対象 head: 71d9c1d7ea4be0f2c28e06e6c3702701746148c6

1. 詰まった箇所: 手順 9「Approve のとき」: 通算 3 回目の `[文書のずれ]` の nit だけを直す round で、手順どおりならエスカレーションだが、指揮役が修正を明示して指示したためその範囲（SKILL.md 394・403 行目と PR 本文の pr-size 併記）で対応した。レビューの再委託は指揮役が行うと明記されたため実施していない。
2. 曖昧だった文: 「1 なら例外の無い失敗」（`.agents/skills/implement-issue/SKILL.md` 巡回の 2）。`required-checks` の rc=1 は verify の fail でも立つため、pr-size だけの失敗と読むと実際の終了コードとずれる。
3. 仮定と解釈: 巡回の 2 の括弧は helper の終了コードの対応表として書き、「1 なら必須チェックのどれかの失敗（どれかは標準出力の判定の行で見る）、2・3 は手順 8 の扱い」とした。403 行目への追記は行の 2 列目（再開する手順）の末尾に置き、相談の push 後にメモ未コミットなら手順 6（171 行目）のとおりメモを push してから手順 8 に進む形にした。
4. 足りなかった情報・検証手段: なし（両レビューの nit と指揮役の指示で修正内容は確定）。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の巡回の表の列「GitHub の実状態」に「必須チェック」の確かめ方は 2 の突き合わせで決まる、と表の直上に 1 行足す（行ごとに判定方法を書かずに済む形）。

使ったスキル: なし
