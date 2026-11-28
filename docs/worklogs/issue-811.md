# Issue #811 の作業ログ

## 失敗の筋（実装前）

- 失敗の筋: 利用の上限で失敗した委託を手で予約して再委託し、自動で再開される実行と重複する
  確かめ方: `scripts/tests/review-delegation-docs.test.sh`（wording pin）と、PR 本文に引用する手順 8 の文
- 失敗の筋: 自動で再開された古い head の実行を待ち、今の head のレビューがいつまでも始まらない
  確かめ方: `scripts/tests/review-delegation-docs.test.sh`（wording pin）と、PR 本文に引用する手順 8 の文
- 失敗の筋: 利用の上限による failed を 2 回連続の failed に数え、再開できる委託で停止する
  確かめ方: `scripts/tests/review-delegation-docs.test.sh`（wording pin）と、手順 9 の `failed` の扱いとの照合

## 失敗の筋（最終）

- 失敗の筋: 利用の上限で失敗した委託を手で予約して再委託し、自動で再開される実行と重複する
  確かめ方: `.agents/skills/implement-issue/SKILL.md` の手順 8 に「自動で再開される」「手で予約しない」があることを読み、`bash scripts/tests/review-delegation-docs.test.sh` で必須文言の存在を確かめる
- 失敗の筋: 自動で再開された古い head の実行を待ち、今の head のレビューがいつまでも始まらない
  確かめ方: `.agents/skills/implement-issue/SKILL.md` の手順 8 に Guard と再委託の時機があることを読み、`bash scripts/tests/review-delegation-docs.test.sh` で必須文言の存在を確かめる
- 失敗の筋: 利用の上限による failed を 2 回連続の failed に数え、再開できる委託で停止する
  確かめ方: `.agents/skills/implement-issue/SKILL.md` の手順 8・手順 9・「止まる条件」の表で同じ除外になっていることを読み、`bash scripts/tests/review-delegation-docs.test.sh` で必須文言の存在を確かめる
- 失敗の筋: 手順 8 だけに説明を足し、手順 9 の「それ以外は 1 回だけ再委託する」と食い違う
  確かめ方: `.agents/skills/implement-issue/SKILL.md` の手順 9 に「利用の上限による failed は手順 8 の扱いに従う」があることを読み、`bash scripts/tests/review-delegation-docs.test.sh` で必須文言の存在を確かめる

実装前の筋のうち外したものはない。実装中に「手順 9 との食い違い」の筋を足した。

## 実装者メモ（round 1）

対象 head: f2f8c49be3a10562606455a909e347b7167ae5c1

1. 詰まった箇所: 手順 8・9: 手順 8 への追加だけでは「止まる条件」の 2 回連続 failed の例外列挙と食い違うため、同じ除外を止まる条件にも足し、手順 9 は手順 8 への参照に留めた。
2. 曖昧だった文: 「`implement-issue` の手順 8 に、利用の上限による failed の扱い（自動の再開、Guard、手で予約しない、委託し直す時機、2 回連続の数え方）が書いてある」（Issue #811「受け入れ条件」）
3. 仮定と解釈: 「手順 8 に書く」を本文の記述の置き場と解釈し、手順 9 では扱いを複写せず参照だけにした。選ばなかった解釈: 手順 9 にも同じ説明を全文で書く。
4. 足りなかった情報・検証手段: 利用の上限の failed を実際に起こす検証手段はなく、確認は wording pin と引用による意味の照合まで。`delegate_task` の limit-resume の確認は #734 の記録を根拠にする。初回の `mise run verify` は sandbox の `/tmp` が nobody 所有で `verify-run.test.sh` に落ちたため、自分が所有する一時ディレクトリを `TMPDIR` にしてやり直した。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「受け入れ条件の書き方」に「別の手順で扱う失敗は、再委託・停止条件との関係を受け入れ条件に書く」を足す。

使ったスキル: implement-issue, writing-for-agents, write-pull-request
