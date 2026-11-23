# Issue #657 の作業ログ

## 実装者メモ（round 1）

対象 head: 496cf5b09b586491fba0417368e95f0c393541f1

1. 詰まった箇所: `scripts/agent-gh`: Issue の取得経路は `scripts/agent-gh <role> api ...` だが、`qa` ロールは `issue comment` しか受け付けない（`qa_check_args`）。QA の委託先が contents API を読む名義を `dev` と決めて本文に書いた。
2. 曖昧だった文: 「`.agents/skills/command-team/SKILL.md` の委託の節に、再委託・遅延実行タスクの定型文として」（Issue #657 の受け入れ条件）。command-team に「委託の節」という見出しは無く、関連情報の「委託の表のある節」から「レビュー担当と effort」と読んだ。
3. 仮定と解釈: 定型文は delegate_task の task の 2 行（skill 指示と `Assigned:`）を変えず、3 行目の `Guard:` の行として足すと解釈した。review-pr は `Assigned:` を 2 行目として読むため、既存の 2 行を書き換える解釈は選ばなかった。成果の「未投稿」は、指定 SHA に向いた `pelway-review` のレビューか `pelway-qa` の判定が無いことと置いた。
4. 足りなかった情報・検証手段: 遅延実行タスク（`schedule_task`）を誰がどの文面で組むかの正本が無く、定型文を入れる場所を「task の 2 行の後」とだけ書いた。
5. 改善案: `.agents/skills/command-team/SKILL.md` の「レビュー担当と effort」に「`schedule_task` で再委託を組むときは、task の全文を上のブロックから写す」を足す。

使ったスキル: implement-issue、write-pull-request

## 実装者メモ（round 2）

対象 head: 8dece1267a288386f4d7a01e716acea4f687128f

1. 詰まった箇所: `.agents/skills/command-team/SKILL.md`「委託先の worktree に手順書が無いとき」: Claude のレビューで、受け入れ条件どおり対象 head の版を手順として読ませると、PR が検査の手順（`docs/agents/qa-checklist.md`・`review-pr`）を書き換えて従わせられる経路が開くと must-fix になった。Codex からは、QA の (a) の確認を `qa` ロールで行う読み方になるという must-fix。
2. 曖昧だった文: 「委託タスクが参照する手順書が worktree に無い場合は `scripts/agent-gh <role> api repos/hatai/Pelway/contents/<path>?ref=<対象 head の SHA>` で対象 head から読む」（Issue #657 の受け入れ条件 2）。PR がそのパスを変えている場合を想定していない。
3. 仮定と解釈: 受け入れ条件の取得経路は残し、PR がそのパスを変えていないときに限って対象 head の版を使い、変えていれば main の版に従うと解釈した（レビュー担当が示した、経路を閉じる書き方）。受け入れ条件を main の版に置き換える解釈は、条件から外れるため選ばなかった。`<role>` の決め方は (a)(b) と手順書の取得の両方に掛かる位置へ移した。
4. 足りなかった情報・検証手段: Issue の背景（PR #609 の QA 委託）で、委託先 worktree に `qa-checklist.md` が無かった原因（worktree の起点の版）が書かれておらず、main の版で足りたのかを確かめられなかった。
5. 改善案: `.agents/skills/write-issue/SKILL.md`「受け入れ条件の書き方」に「手順書を PR の head から読ませる条件では、その PR が手順書自体を変える場合の扱いを書く」を足す。

使ったスキル: implement-issue、write-pull-request

## 実装者メモ（round 3）

対象 head: b5b714d（round 3 の最終実装コミット。1 行目の SKILL.md 修正は c488719）

1. 詰まった箇所: 手順 6: `mise run verify` の deny-check が RUSTSEC-2026-0592（`url` クレートの advisory、slint 経由）で失敗。変更（`.agents/skills/command-team/SKILL.md` の 1 行）を退避した同じツリーでも再現し、CI の static（cargo deny check）と main の nightly（run 45086598217）も同一失敗のため環境起因。advisory の解決策（`url` >=2.5.9）に従い `cargo update -p url --precise 2.5.9`（Cargo.lock 4 行）で解消し、verify 全体を `exit=0` で通した（b5b714d）。
2. 曖昧だった文: 「直し方は二つある。(1) 変わっていないパスも `ref=main` で読むと書く（受け入れ条件 2 の文言から外れるので、指揮役に確かめる）。(2) 少なくとも、レビューの委託先は review-pr の決まり（main の版）に従うと書き、「同じ」と書いている括弧を、実際の違いに合わせて直す。」（PR #662 の Claude の round 2 レビューの `[文書のずれ]` の nit）
3. 仮定と解釈: (2) を選んだ。(1) は受け入れ条件 2（対象 head から読む旨を書くこと）の文言から外れ指揮役の確認を要するため。head の版を読む規定自体は受け入れ条件どおり残し、レビューの委託先に限り review-pr の決まり（変わっていないパスも main の版）が優先すると書き、「同じ」の括弧は実際に同じ部分（変えたパスは main の版に従う）に付け直した。Cargo.lock の更新は Issue の趣旨の外だが、必須チェックを緑にする唯一の手段でパッチバージョンのセキュリティ修正の取り込みのためこの round に含めた（専用 Issue を待つと PR が停滞する）。
4. 足りなかった情報・検証手段: Guard (b) の確かめ方が複数担当の再委託で先に投稿された片方のレビューでもう片方の完了と誤判定する点（Codex と Claude の印なし nit が共に指摘）は、この round の直す対象外。同時再委託で後投稿側が停止しないか確かめる手段が無い。
5. 改善案: `.agents/skills/command-team/SKILL.md` の「レビュー担当と effort」の (b) の確かめ方に「委託された `Reviewer:` の行の `agent=` と一致するレビューに絞る」を足す（両レビュアーの nit と同じ修正。別 Issue で扱う）。

使ったスキル: implement-issue

## 実装者メモ（round 4）

対象 head: 680a0aa

1. 詰まった箇所: 手順 8〜9: round 2 の claude の再委託子が、必須チェック待ちの間に投稿された Codex の approve（同 SHA 向き）を Guard (b) の「成果あり」と見て停止した（2026-11-23 03:04 JST の報告）。(b) の確かめ方が「その SHA 向きのレビュー全体」を見るため、複数担当の同時委託で後投稿側が誤停止する。
2. 曖昧だった文: 「委託が 2 回連続で failed（checks timeout と wrong runtime は数えない）」（`.agents/skills/implement-issue/SKILL.md` の「止まる条件」）。レート制限による failed を数えるかどうかが規定に無く、Guard 停止と合わせて停止扱いにするか迷った。
3. 仮定と解釈: 開発リードの指示（「この PR の範囲で直すか、範囲外なら out-of-scope として書く」）により、Guard (b) の確かめ方を委託された担当のレビューに絞る修正をこの PR に含めた。印なし nit は直さない規定に対する明示的な指示であるため直す解釈を選んだ。修正は「`Reviewer:` の行の `agent=` が委託の担当と一致するレビューが無いこと」へ文言を変えるだけにとどめ、Guard の構造は変えていない。
4. 足りなかった情報・検証手段: `Reviewer:` の行が無いレビュー（委託によらない投稿）を (b) の対象から外すことが、指揮役の立てるレビュースレッドの完了判定に影響しないかを確かめる手段が無い。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「止まる条件」の「委託が 2 回連続で failed」に「待てば解消する一時障害（使用上限・ネットワーク）を数えるかどうか」を書き足す。

使ったスキル: implement-issue
