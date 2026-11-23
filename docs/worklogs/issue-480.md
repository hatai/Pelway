# Issue #480 の作業ログ

## 実装者メモ（round 1）

対象 head: 0fbf2f57c70aa81dd77e9844bf2a9636eeaa008b

1. 詰まった箇所: 手順 4（テストを先に書く）: 文書だけの PR で受け入れ条件の検証の置き場所に迷った。検証スクリプトを /tmp/opencode に置こうとして sandbox に拒否され（worktree 外への書き込み）、worktree 内の一時ファイル `.tmp-verify-480.sh`（コミットせず、検証後に削除）で grep 54 項目の存在チェックを回した。実装前に 51 項目 NG、実装後に ALL OK を確認（両方の出力を PR 本文に記載）。
2. 曖昧だった文: 「effort は、セキュリティと仕様の PR では1回目から上げておく。3回目で上げる運用はやめる。」（Issue #480 本文の「B. レビューのラウンドのルール」7）
3. 仮定と解釈: B7 の「PR」はレビューの effort を指すと仮定した（変更対象に implement-issue 207 行「3 回目のレビューの委託では effort を 1 段上げる」が挙がっているため）。レビュー側のみ 1 回目から上げる形に直し、command-team 124 行の実装側 effort 引き上げ（修正 2 回超過で 1 段上げ）は維持した。選ばなかった解釈: 実装側の effort 引き上げも廃止する読み。ほかに、B2 の領域分類の正本を command-team「レビューのラウンド」と implement-issue 手順 9 に置き、review-pr skill は Issue の変更対象に挙がっていないため変更しないと判断した。
4. 足りなかった情報・検証手段: review-pr SKILL.md に「3 回目以降の must-fix に領域分類を付す」規定が無い（B2 の「レビュー役が分類する」をレビュー役が実行するための記述）。QA（人間の役割）の定義が GLOSSARY.md に無い（Issue の背景にあるエンジニアチーム 4 名の QA と解釈し、形式のみ規定した）。
5. 改善案: `.agents/skills/review-pr/SKILL.md` のレビュー本文の形式に「3 回目以降の must-fix には領域分類（セキュリティ領域か、別 Issue に回せるか）を付ける」を足す。ただし本 Issue の変更対象外のため今回は見送り。

使ったスキル: implement-issue

## 実装者メモ（round 2）

対象 head: 93d1384067603eddd001b554c1fbbb5500b50ac0

1. 詰まった箇所: `.agents/skills/command-team/SKILL.md` の「マージ」節: round 1 で「（例外はこれ以外に広げない）」と書いた直後にキューの切り戻しと `pr-size-exception` を admin マージの対象として列挙し続け、文が自己矛盾していた（レビュー指摘のとおり）。
2. 曖昧だった文: 「例外（濱田さんが判断する）：round 4 以降の例外 PR、`.github/` の下を変える PR、`fnox.toml` を変える PR。これ以外には広げない。」（Issue #480 本文の「A. マージの運用」2）
3. 仮定と解釈: 切り戻し・`pr-size-exception` は admin マージの技術的なフォールバックで A2 の「例外」とは別枠と書き分ける解釈もあったが、レビューの must-fix（A2 に列挙した例外だけにする）に従い、admin マージの対象を A2 の 3 つだけに統一した。切り戻し・`pr-size-exception` の PR は「例外には加えず、hatai が 3 つのどれかに該当すると判断したときだけ admin マージされる」とした。選ばなかった解釈: フォールバックを例外と別カテゴリとして並列に書く（must-fix の指摘する「広がり」が残るため不採用）。pr-size-exception の運用（L60「マージしてよい」）は今回の対象外のため、hatai の例外判断を経る形で両立させた。
4. 足りなかった情報・検証手段: `pr-size-exception` ラベル付き PR が A2 厳格適用のもとで従来どおりマージまで至るか（hatai の例外判断を毎回経る運用で良いか）は、Issue 本文に明示が無く、運用で確認するしかない。
5. 改善案: `docs/superpowers/specs/agent-team.md` の「制約」に「`pr-size-exception` と A2 の例外の関係（hatai の例外判断を経由してマージする）」を 1 文で足す、または次回の運用見直し Issue で pr-size 例外のあり方を問い直す。

使ったスキル: implement-issue

## 実装者メモ（round 3）

対象 head: 21640d6c1cb949ce0048f44d0470c84af8e4fda3

1. 詰まった箇所: 手順 9 の「レビュー本文の形式」: round 1 で「別 Issue」の記載先を実装側のラウンドサマリーにだけ置き、レビュー本文（review-pr 手順 4）の形式を定義していなかった。レビュー役が分類を「どこにどう書くか」が未定義のため B2 が実行されない、という指摘のとおりだった。
2. 曖昧だった文: 「それ以外を別 Issue に回すかは、レビュー役が分類する。実装役は分け直せない。」（Issue #480 本文の「B. レビューのラウンドのルール」2）
3. 仮定と解釈: round 1 では「review-pr は Issue の変更対象リストに挙がっていないため変えない」と解釈したが、レビュー round 2 の must-fix は「B2 を機能させるにはレビュー役の手順書への記載が必要」と判断したものであり、受け入れ条件の「B のとおりに直り」を機能面で満たすには review-pr の変更が必要と読めるため、指摘に従って review-pr に加えた。選ばなかった解釈: command-team の記載だけで十分という立場を貫き反駁する（分類の実行者の手順が無いままでは B2 が運用に乗らないため不採用）。「3 回目以降か」の確認方法はラウンドサマリーの `### review round <R>` コメントと `pelway-review` のレビュー件数の併用とした（B1 の同じ head は 1 ラウンドにより件数だけでは決まらない場合があるため）。
4. 足りなかった情報・検証手段: ラウンドサマリーが投稿されていない PR（初回レビュー）でレビュー件数だけからラウンドを決める際の正確な手順。運用でラウンドサマリーの投稿漏れが出たときのフォールバックが未定義。
5. 改善案: `.agents/skills/review-pr/SKILL.md` の手順 3 に「ラウンドサマリーが見つからないときはレビュー件数をラウンドとして扱う」の 1 文を足す。今回は分類手順の本体が主眼のため、ここまで書かない判断をした。

使ったスキル: implement-issue

## 実装者メモ（round 4）

対象 head: dc0e7f06c2ca3391040dbef3ddd2bf99311e108e

1. 詰まった箇所: `.agents/skills/review-pr/SKILL.md` の手順 3: round 3 で「ラウンドサマリーとレビュー件数から確かめる」と書き、食い違ったときの正が決まっていなかった（レビュー指摘のとおり）。round 3 の must-fix 残存で一度停止し、hatai の例外許可（2026-11-19 17:23 JST）で round 4 としてこの修正を行った。
2. 曖昧だった文: 「→ ラウンドサマリーを正とするか、複数レビューと同じ head の再レビューを含めて一つに決まるラウンドの数え方を明記する。」（round 3 レビューの must-fix の指示文）
3. 仮定と解釈: 提示された選択肢のうち「ラウンドサマリーを正とする」を採った。最も大きい `<R>` + 1 をこのレビューのラウンドとし、件数は判定に使わない。選ばなかった解釈: 委託記録コメント（`[review-request]`）の件数で数える方法（同じラウンドでの head 変更を伴う再委託で件数が増えるため、これも一意にならない）。既知の限界として、停止ラウンドのように対応完了時にラウンドサマリーを投稿しない運用（手順 9 は Request changes 対応と Approve 対応のときだけ投稿を定める）では、次のレビューのラウンドが実際より 1 小さく出る。round 3 以降かの閾値判定には影響しないため受け入れた。
4. 足りなかった情報・検証手段: 停止ラウンドの扱いを implement-issue 側で「停止時もラウンドサマリーを投稿する」ようにする修正は、今回の指揮役の指示（must-fix 1 件だけ直す）の範囲外だったため見送り。ラウンドの上限検査は実装側・指揮役側の記録でも行われるため、レビュー側の判定のずれは閾値判定に影響しないことの裏付け。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の止まり方に「停止で終わった round もラウンドサマリーを投稿して終える」を 1 行足す（別 Issue として起票するのが適切）。

使ったスキル: implement-issue

## 実装者メモ（round 5）

対象 head: 0ec3667de36f7219c7fa29b5b3f21207d46d2eb9

1. 詰まった箇所: `.agents/skills/review-pr/SKILL.md` の手順 3: round 4 でラウンドサマリーを正としたが、投稿者の検証が無く、第三者が大きな `<R>` を投稿するとラウンドを水増しできた（レビュー指摘・security-review HIGH のとおり）。round 4 の must-fix 残存で停止後、hatai の例外許可（2026-11-19 18:54 JST・PR コメント記録）で round 5 としてこの修正を行った。
2. 曖昧だった文: 「ラウンドサマリーの投稿者を許可された実装担当に限定して検証する手順を加えてください。」（round 4 レビューの must-fix の指示文）
3. 仮定と解釈: 「許可された実装担当」を PR の作成者（実装担当の App `pelway-dev`）と同一の `author.login` と定義した。ラウンドサマリーは skill implement-issue の手順 9 に従い実装担当が投稿するものであり、PR 作成者と一致することが正しい投稿者の機械的な判定条件になるため。第三者のコメントは無視する旨と、その理由（ラウンド水増しからの must-fix 回避の入り口を塞ぐ）を併記した。
4. 足りなかった情報・検証手段: PR の作成者とコメントの作成者の `author.login` の表記ゆれ（`app/pelway-dev`・`pelway-dev[bot]` 等）を SKU 内の記述で吸収できるか。実際の API 応答では comments は `pelway-dev`、PR の author は `app/pelway-dev` と返るため、「PR の作成者と一致する」という比較基準の書き方にした（特定の表記をLiteral に書かない）。
5. 改善案: `.agents/skills/review-pr/SKILL.md` に他の信頼できるコメント（目印コメント・委託記録）と同じ「作成者の確認」の共通規則を 1 つにまとめる節を足す（重複記述の削減。別 Issue が適切）。

使ったスキル: implement-issue

## 実装者メモ（round 6）

対象 head: 144316e04d5500650fe2d83be86006f670d85457

1. 詰まった箇所: `.agents/skills/review-pr/SKILL.md` の手順 3: round 5 で「作成者が PR の作成者と一致する」と書いたが、GitHub API の応答では PR の `author.login` が `app/pelway-dev`、コメントの `author.login` が `pelway-dev` と表記が揺れ、完全一致が成立しなかった（レビュー指摘のとおり）。
2. 曖昧だった文: 「実装担当 App のコメントを識別できる条件に直し、第三者コメントは引き続き除外してください。」（round 5 レビューの must-fix の指示文）
3. 仮定と解釈: 「識別できる条件」を、実装担当 App の login の3 表記（`pelway-dev`・`pelway-dev[bot]`・`app/pelway-dev`）の列挙による一致とした。選ばなかった解釈: 接頭辞 `app/` を除いた比較や `[bot]` 接尾辞の正規化を指示に書く方法（レビュー担当が毎回正規化を計算することになり、列挙の方が判定が単純で第三者の除外も明確なため）。列挙が「実装担当 App の表記」であることを文内で宣言し、将来 表記が増えた場合はこの列挙を直す運用とした。
4. 足りなかった情報・検証手段: GitHub API の bot の login 表記の網羅（GraphQL と REST で異なる場合がある）。この PR での実測（PR=`app/pelway-dev`、コメント=`pelway-dev`）に `pelway-dev[bot]`（REST の標準形）を足した 3 表記で受け入れる形にしたが、網羅性の保証は GitHub の仕様変更まで追えない。
5. 改善案: `docs/agents/notify-payloads.md` など GitHub の webhook/API の表記の実際を記録する場所に bot の login 表記の一覧を残す（別 Issue が適切）。

使ったスキル: implement-issue

## 実装者メモ（round 6・main 取り込みの衝突解消）

対象 head: 816fc3adb0441221558092494dc701b320464b06

1. 詰まった箇所: `.agents/skills/command-team/SKILL.md` の「レビュー担当と effort」: main の #565（731f4f26、Codex のモデルを PR の大きさ・リスク・回数で選ぶ）と、この PR の effort の行（セキュリティと仕様の PR は 1 回目から上げる）・判定が割れたときの行が同じ hunk で衝突した。両立するため、この PR の effort の行と判定が割れたときの行を残し、その間に main の「Codex のモデル」の表と注記をそのまま入れた（落とした側は無い）。
2. 曖昧だった文: 「解消後は Request changes と同じ流れで進める（手順 5・6 のやり直し・本文の sha 更新・ラウンドサマリー・手順 8 の再委託）」（`.agents/skills/implement-issue/SKILL.md` の手順 9「修正依頼のとき」）
3. 仮定と解釈: round 6 のレビューはまだ一度も投稿されていない（委託が 2 回 failed・開始できず）ため、衝突解消後の head のレビューも round 6 として扱う。ラウンドサマリーは `### review round <R>` の見出しで投稿すると review-pr のラウンド判定（最大の `<R>` + 1）が round 7 に進んでしまうため、見出しを付けない衝突解消の報告コメントにした。選ばなかった解釈: 見出し付きのサマリーを投稿して round 7 として数える。
4. 足りなかった情報・検証手段: レビューを伴わない修正依頼（衝突の解消）のときのラウンドサマリーの形が手順 9 に無い。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 9「修正依頼のとき」に「レビューを経ない修正では `### review round` の見出しを付けずに解消内容をコメントする」を足す（別 Issue が適切）。

使ったスキル: implement-issue

## 実装者メモ（round 7・例外: 認可の 1 件だけ）

対象 head: e7ab9bd4b2b6b7b6ddc6f19951ec0631c2ec7e67

1. 詰まった箇所: `.agents/skills/command-team/SKILL.md` の「マージ」の QA 合格コメント: 最初の修正（`c238ef0b`）で作成者を `author.login` だけで確かめていた。Claude の例外レビューの指摘のとおり、書き込み権限を持つ `pelway-dev` がほかの名義のコメントを編集でき、古い合格コメントの sha を書き換えるとすり抜けられた。REST の `user.login == "pelway-qa[bot]"`・`user.type == "Bot"`・`created_at == updated_at` の 3 条件に直し、runbook 88 行も同じ形にした。main（`e109a2b1`）を取り込んだときは 5 ファイルが衝突し、`[文書のずれ]` の nit の変更と両立させた。
2. 曖昧だった文: 「push 条件: Claude レビュー合格・同一 sha QA 合格・push 前点検の 3 つ。」（Issue #480 の 2026-11-21T05:34 の再開指示、作成者 `pelway-dev`）
3. 仮定と解釈: 再開指示の作成者は hatai ではなく `pelway-dev` だが、指揮役の T3 の指示で同じ内容を受けたため決定として扱った。push は、指揮役の次の指示（例外レビュー 1 回で済み、QA は新しい head で点検する）を受けてから行った。nit は指示された 2 件（日付の削除、admin マージの例外に読めない書き方）だけを直した。
4. 足りなかった情報・検証手段: `pelway-qa` の実際の投稿の `user.login`・`user.type` は、Issue #483 の実測がまだで確かめていない（`docs/dev-environment.md` に「作者が `pelway-qa[bot]` であることの実測はマージ後」とある）。push は、merge コミットを含むため `agent-gh-resign` の署名付けが行われず、署名なしのままになった。
5. 改善案: レビュー対象外の nit（`pelway-qa` の表記が同名の人のアカウントと一致しうる件。review-pr のラウンドサマリーと `conductor-merge-queue` にも同じ形がある）、`docs/superpowers/specs/agent-team.md` の 22 行と 85 行に作成者の条件が無い件、7〜14 行の名義の表に `pelway-qa` の行が無い件は、別 Issue で `.agents/skills/review-pr/SKILL.md` と `docs/superpowers/specs/agent-team.md` の名義の確かめ方を「REST の `user.type == "Bot"` と `[bot]` 付きの login」に揃える。

使ったスキル: implement-issue

## 実装者メモ（round 8・main 取り込みの衝突解消）

対象 head: aaa3830b54ddfbbf4afefedb1701730f23854676

1. 詰まった箇所: main（`3afed983`。#577・#585 の QA の書き方を含む）を取り込んだときに 3 ファイルが衝突した: `.agents/skills/command-team/SKILL.md`「マージ」、`docs/agents/grokbot-runbook.md` の 21 行・`merge` の巡回の行、`docs/superpowers/specs/agent-team.md` の手順 7 と役割分担の行。指揮役の指示（main に同じ内容があれば main を採り、ブランチ固有の決まりは残す）に沿って解いた。この PR の `QA-PASS pr=... sha=...` の形は落とし、main の `docs/agents/qa-checklist.md`「合格として数えるもの」（`QA: pass`・`sha:`・`run:`・`pelway-qa[bot]` の Bot）を正本として参照する形にした。ブランチ固有の決まりは残した: 編集されていないこと（`created_at == updated_at`）、hatai の名義と `pelway-qa` の名義を区別すること、投稿できないあいだは `automerge` を付けないこと、admin マージの例外は 3 つだけ。編集の確かめは qa-checklist の条件とコマンドにも足した。
2. 曖昧だった文: 「ブランチ固有の決まりを残しつつ、main 側の新しい表現に揃える形で解消してください（main に既に取り込まれた同一内容は main を採用）。」（指揮役の T3 の指示）
3. 仮定と解釈: 「同一内容」は、QA の合格コメントの形と作成者の確かめ方（main の qa-checklist）と読んだ。編集の検査はブランチ固有の認可の修正なので、正本の qa-checklist に足した。選ばなかった解釈: command-team の側だけに編集の検査を残し、qa-checklist は変えない（正本とずれ、指揮役が qa-checklist だけを見ると編集のすり抜けが残るため不採用）。
4. 足りなかった情報・検証手段: verify の 1〜2 回目は、`[test]` が `rustc 1.75.0` で動いて落ちた。`mise exec -- rustc --version` が `mise WARN missing: rust@1.99.1` と出て、1.99.1 と 1.75.0 の間で揺れていた。取り込みの前の `ea1dfd53` を detach で checkout しても同じ WARN が再現したため、環境の問題として扱った。3 回目は exit 0（36 tests ok）。`mise run pr-size` は +143 / -20。
5. 改善案: `docs/dev-environment.md` に「`mise WARN missing: rust@<版>` が出て `/usr/bin/rustc` に落ちるときの切り分け」を足す（別 Issue が適切）。

使ったスキル: implement-issue

## 実装者メモ（round 9・main 取り込みの衝突解消）

対象 head: c8487dabdae403c58ebf8e6c44daa587a979bd91

1. 詰まった箇所: main（`840e0b93`）を取り込んだときに 2 ファイルが衝突した。`.agents/skills/implement-issue/SKILL.md` は 4 か所で、止まる条件の表、手順 9 の 1、判定が割れたとき、手順 10。`.agents/skills/review-pr/SKILL.md` は 1 か所で、手順 3。main の追加（`設定の不備:`・`auto-accept-edits` の止まる条件、`checks timeout`・`wrong runtime`、同じ種類の must-fix の全経路、PR 外の所見の `out-of-scope:`）と、この PR の追加（round の止まり方、領域分類、セキュリティの修正側優先、#66 の head の確認）は両立するので、両方を残した。`checks pending` は main の `checks timeout`・`wrong runtime` に揃えた。
2. 曖昧だった文: 「新しい head で、Codex と pelway-review の再レビューを受けてください。」（開発リードの T3 の指示）
3. 仮定と解釈: Codex のレビューは `pelway-review` の名義で投稿されるため、「Codex と pelway-review」は Codex の 1 委託で両方を満たすと読んだ。選ばなかった解釈: Codex と別の担当（Claude）の 2 委託。
4. 足りなかった情報・検証手段: 誰が `pelway-qa` の名義で QA を行うのか（実装スレッド自身か、別のスレッドか）。qa-checklist は「QA がレビュー済みの PR を独立に確かめる」とだけ書いている。
5. 改善案: `docs/agents/qa-checklist.md` の冒頭に、QA を行う主体（指揮役が立てる QA スレッドで、実装スレッドではない）を 1 文で足す（別 Issue が適切）。

使ったスキル: implement-issue

## 実装者メモ（round 10・`[文書のずれ]` の nit）

対象 head: 7df909bb7f41ecfc0267a32b5a363037f21c2605

1. 詰まった箇所: Codex のレビュー（head `35759d5e`）は approve だったが、`[文書のずれ]` の nit が 3 件あった。直したのは次の 3 つ。`.agents/skills/implement-issue/SKILL.md` の手順 10 の SHA の照合（`agent-gh review api` から、実装担当の名義の `agent-gh dev pr view --json headRefOid` へ）。`docs/agents/grokbot-runbook.md`「マージの条件」の 5 項目の写し（command-team「マージ」の正本への参照に置き換えた）。PR 本文の A7 の説明（qa-checklist の形に「未検証」の欄が無いこと）。
2. 曖昧だった文: 「条件の列挙を正本への参照に置き換え、QA 待ちの判定でも正本の条件を使ってください。」（Codex のレビューの nit）
3. 仮定と解釈: runbook には、正本の条件の要約を括弧で残した（読み手が条件の範囲をつかめるよう「などを含む」と書き、確かめ方は正本だけにした）。QA 待ちは「正本の条件のうち QA の合格だけが欠けている PR」と定義した。選ばなかった解釈: runbook から条件の名前を全部消す。
4. 足りなかった情報・検証手段: 手順 10 の「fetch は禁止なので」の前提（Issue #480 A6）と、手順 6 の `scripts/agent-gh dev git fetch origin main` の関係が文書に書かれていない。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 10 の「fetch は禁止なので」を、何の fetch が禁止なのか（PR ブランチの remote を書き換える操作か）を明記する文に直す（別 Issue が適切）。

使ったスキル: implement-issue

## 実装者メモ（round 12・main 取り込み・衝突なし）

対象 head: 9a8a33ea950d2372aca74b98d47bfca1751ed500

1. 詰まった箇所: main（`e045c78f`。#677 の url 2.5.9、#645 の required-checks helper などを含む）を merge で取り込んだ。衝突は無かった。verify の 1 回目は `.agents/skills/verify-pelway/verify-run.test.sh` が失敗した。このマシンの `/tmp`（`/dev/sdd` の ext4 マウント）と `/home` が nobody（uid 65534）所有で、verify-run の `dir_chain_safe` が「置き場より上のディレクトリは自分か root の持ち物」を満たさず、`make_run_dir`・`locks_dir_ready` が 1 を返した。`TMPDIR` を `/dev/shm` 配下（`/dev/shm`・`/dev`・`/` が uid 1000 所有のチェーン）に変えて再実行すると 47 tests ok。merge は verify-pelway に触れていないため、環境の問題として扱った。push はマージコミットを含むため `agent-gh-resign` の署名付けが行われず、署名なしのままになった。
2. 曖昧だった文: 「停止を解いて、main を取り込んでからレビューをやり直す」（開発リードの決定の伝達）。
3. 仮定と解釈: 「レビューをやり直す」は、手順 8 の委託除外（この PR の差分は `.agents/skills/review-pr/` を変える）に従い、実装側の委託ではなく指揮役が main から立てるレビュースレッドを待つと読んだ（開発リードの指示の項目 4 が明示）。選ばなかった解釈: 実装側から `codex_review` に再委託する。
4. 足りなかった情報・検証手段: `/tmp`・`/home` が nobody 所有になった時期と理由（11/22 の verify では `/tmp` は通常の所有だった）。ローカルでは `TMPDIR` で切り分けたが、CI との環境差として残るかは確かめていない。
5. 改善案: `docs/dev-environment.md` に「`/tmp`・`/home` が nobody 所有の環境で `verify-run.test.sh` が `dir_chain_safe` に落ちるときの切り分け（`TMPDIR` を uid 1000 所有のチェーンに向ける）」を足す（別 Issue が適切）。

使ったスキル: implement-issue

## 実装者メモ（round 13・レビューの must-fix 2 件と印の nit 1 件）

対象 head: 52a281b4d01e018dbe4588c95bf23f175a587fb6

1. 詰まった箇所: 指揮役のレビュースレッド（Devin swe-2-high 代替・codex 上限のため）のレビュー（head `b4ea8e0a`）は request-changes、must-fix 2 件・nit 3 件。直したのは must-fix 2 件と、印 `[文書のずれ]` のある nit 1 件。`docs/agents/grokbot-runbook.md` の 27 行・96 行・143 行（`LABEL unbacked` の裏付け検査: 条件を満たした PR への指揮役の `automerge` 付け外しは指揮役の操作記録を裏付けに数え、例外の PR（round 4 以降・`.github/`・`fnox.toml`）への付与は hatai 本人の記録だけを裏付けにする）。`AGENTS.md` の 37 行（手マージ前提の「a manual merge is used only when the queue cannot merge」を、例外 PR のみに hatai が判断する admin マージという正本への参照に置き換え）。`docs/superpowers/specs/agent-team.md` の 60 行（`pr-size-exception` はレビューで must-fix に数えないが、Kodiak は必須チェック失敗の PR をマージできないため `automerge` は付けず hatai の例外の判断に回す、という形に 76 行と揃えた）。
2. 曖昧だった文: 「条件を満たした PR に指揮役（リード）が付けた `automerge` ラベルは『裏付けあり』として扱い `LABEL unbacked` にせず」（レビューの must-fix と Issue #480 の hatai コメント「追加でやること」1）。
3. 仮定と解釈: 「裏付けあり」の範囲は付与だけでなく解除にも及ぶと読んだ（指揮役が条件崩れで外す操作も正当な操作記録で裏付ける。元の文は「付けたラベル」とあるが、裏付け検査は付け外しの両方を見る決まりのため）。例外 PR では付与のみ hatai 本人の記録を求める書き方にした（解除は不適切なラベルを除く操作で、検出すべき対象は付与側）。選ばなかった解釈: 例外 PR では付け外しの両方に hatai の記録を求める。
4. 足りなかった情報・検証手段: nit の `author.login` の `[bot]` 落ちの件（review-pr:122）は印が無いため直さなかったが、`pelway-dev` の同名ユーザーが登録可能かは未検証のまま（レビュー本文も「未確認」としている）。
5. 改善案: `docs/agents/grokbot-runbook.md` と qa-checklist・名義の確かめ方の `user.type == "Bot"` への揃え（レビューの nit と round 10 のメモの改善案と同じ件）は別 Issue が適切。

使ったスキル: implement-issue

## 実装者メモ（round 14・main 取り込みの衝突解消）

対象 head: 8aae0a1df060da3ea4b8ee848019a32287d5ceb6

1. 詰まった箇所: main（`79bd5a4e`。#673 の運用まわり Issue の事前分割と #666 などを含む）を取り込んだときに `.agents/skills/command-team/SKILL.md` の「マージ」が 1 か所で衝突した。main の変更は、pr-size 例外の段の末尾に「運用まわりの PR が pr-size を超えたときは、例外の提案より先に `<!-- split-plan -->` での分割を提案する（下の「分割を認めるとき」）」を足すもので、この PR が書き換えた節の旧文に対する追記だった。この PR の新しい節を残し、main の追記だけを pr-size の段の末尾に写した（「分割を認めるとき」の節は衝突せず取り込まれた）。push はマージコミットを含むため署名付け替えが行われず、署名なしのままになった。
2. 曖昧だった文: 「PR #482（head 4819469a）は must-fix 2 件の修正 push 済みを確認しましたが、現在 main とのコンフリクトで mergeable_state=dirty です。main を取り込んでコンフリクトを解消し push してください」（指揮役の T3 の指示）。
3. 仮定と解釈: main の追記（運用まわりの PR の分割優先）は、この PR の新しい規則と両立する追加として残した。選ばなかった解釈: 衝突した節を丸ごと main の版に戻す（この PR の書き換えを失うため不採用）。verify は `TMPDIR=/dev/shm` 配下で実施（round 12 の `/tmp`・`/home` が nobody 所有の環境の問題の継続）。
4. 足りなかった情報・検証手段: main の `85069080`（運用まわり Issue の事前分割）とこの PR の `pr-size-exception` の規則の意図の遡及は、コミットメッセージと両側の文面で確かめた。Issue 側の決定記録は読んでいない。
5. 改善案: なし（round 12 の `dir_chain_safe` の環境の件は引き続き別 Issue が適切）。

使ったスキル: implement-issue
