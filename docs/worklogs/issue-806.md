# Issue #806 の作業ログ

## 実装者メモ（round 1）

対象 head: fc94d5e79184d535085d265b1f95480e09519221

1. 詰まった箇所: 手順 4: 新しい wording pin が backtick の境界まで厳密に指定しすぎ、最初の green 確認で pin 側の指定誤りが出た。コマンド部分を backtick を含む 1 文言に直した。
2. 曖昧だった文: 「例外が認められた（ラベルとコメントがある）あとの push と再開の手順が書いてある」（Issue #806 の受け入れ条件）
3. 仮定と解釈: 例外は既存の PR にだけ付けられると解釈した。選んだ解釈: 初回 push 前に PR が無いときは例外で再開できず分割を待つ。選ばなかった解釈: 例外があれば初回 push もできる。
4. 足りなかった情報・検証手段: PR が無い状態で `pr-size-exception` ラベルを用意する運用は `scripts/agent-gh-pr-size-gate` には無く、ゲートは開いた PR のラベルだけを見る。verify は sandbox の `/tmp` が nobody 所有で `verify-run.test.sh` の `dir_chain_safe` に合わないため、同じ exec の中で作った自分所有の一時ディレクトリを `TMPDIR` に指定してやり直す（既出の環境起因。`docs/worklogs/issue-689.md` と同じ）。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「受け入れ条件の書き方」に「例外の再開条件を書くときは、その例外を付ける対象（PR・Issue）がその時点に存在するかを明記する」を足す。

使ったスキル: implement-issue, test-driven-development, writing-for-agents, ja-text-communication, stop-ai-slop-jp

## 実装者メモ（round 2）

対象 head: e08f1027a43aaac008e07591528fc3de2f20be89

1. 詰まった箇所: 手順 9: 上限超過で未 push の実装コミットを残して止まったあと例外が認められた場合、再開表が GitHub 上の古い head だけを見て手順 8〜10 を選び、手順 6 の push に戻れないことがレビューで指摘された。
2. 曖昧だった文: 「状態の行と GitHub の実状態が食い違えば、GitHub の実状態を正とし、その状態に当たる手順から再開する」（`.agents/skills/implement-issue/SKILL.md`「巡回から定型の指示を受けたとき」）
3. 仮定と解釈: 手元の `git rev-parse HEAD` が Issue の止まりのコメントの SHA と一致し、PR の head の後に積まれている場合だけを、上限超過で未 push のコミットを残した停止と解釈した。選ばなかった解釈: remote より先のコミットすべてを pr-size 停止とみなす。
4. 足りなかった情報・検証手段: pr-size の止まりのコメントに機械判定用の目印が無く、再開表では SHA の一致と例外のラベル・コメントの照合に頼る。確認は `bash scripts/tests/pr-size-stop-docs.test.sh` の文言の存在確認までで、実際の停止から再開までの通し実行はない。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「止まる条件」に「pr-size の止まりのコメントの行頭を `<!-- pr-size-stop -->` とする」を足し、再開表の照合を SHA と文面の一致ではなく目印で行えるようにする。

使ったスキル: implement-issue, writing-for-agents, ja-text-communication, stop-ai-slop-jp

## 実装者メモ（round 3）

対象 head: 1a1055f480703293403d8e2b379e903affc1dd42

1. 詰まった箇所: 手順 9: Approve の `[文書のずれ]` nit で、command-team の承認コメントの条件だけが `行頭` で、implement-issue とゲートの `本文先頭` にずれていることが残った。
2. 曖昧だった文: 「行頭が `<!-- pr-size-exception -->` で始まるコメント」（`.agents/skills/command-team/SKILL.md`「マージ」）
3. 仮定と解釈: `行頭` はコメント全体の本文先頭を指すと解釈し、ゲートの `startswith` と同じ意味になるよう `本文先頭` にそろえた。選ばなかった解釈: コメント中のどれかの行の先頭でも承認とみなす。
4. 足りなかった情報・検証手段: 目印の一致を行単位と本文単位のどちらで書くかの用語の置き場が無く、今回は `scripts/agent-gh-pr-size-gate` の `startswith` を一次情報にした。確認は `bash scripts/tests/pr-size-stop-docs.test.sh` の文言の存在確認。
5. 改善案: `GLOSSARY.md` に「本文先頭: 文字列全体の先頭。複数行の文書では 2 行目以降の行頭を含まない」を足し、承認コメントや状態行の一致条件で同じ語を使う。

使ったスキル: implement-issue, writing-for-agents, ja-text-communication, stop-ai-slop-jp
