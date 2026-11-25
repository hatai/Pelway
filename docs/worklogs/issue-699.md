# Issue #699 の作業ログ

## 実装者メモ（round 1）

対象 head: a864729b5ae84b83f155e883722b7eed9a68b428

1. 詰まった箇所: `scripts/tests/conductor-watch-patrol-state.test.sh` の試験 1: 「symlink を読まない」を `neq "$(since61)" "$SEEDED_SINCE"` で書いたら、記録が symlink のまま残る main でも `since61` が `not-regular` を返して通った（red にならない）。通常ファイルとして書き戻され、かつ時刻が引き継がれていないことを見る `fresh` に書き直した。
2. 曖昧だった文: 「`scripts/conductor-watch.sh` の patrol-state.json を読む箇所すべてに #680 の `read_ps` と同等の検査（O_NOFOLLOW・O_NONBLOCK・通常ファイル限定・サイズ上限）が適用されている」（Issue #699 の「受け入れ条件」）。書き戻し（背景の `:651`）と旧置き場からの移行の `cp` が「読む箇所」に入るかが書かれていない。
3. 仮定と解釈: 背景の `:651` は書き戻しの `open(ps_path, "w")` と仮定した（#680 のマージ直後の main `0c6a50d9` の 651 行目で確かめた）。FIFO の試験は読み取りだけを直しても書き戻しの open で止まるため、書き戻しも範囲に含め、一時ファイルからの置き換えにした。移行の `cp` も旧置き場を読む箇所として含めた。読めない記録は新しい記録で置き換える（既存の「JSON として読めない → 空」と同じ扱い）解釈を選び、書き戻しをやめる解釈は選ばなかった（`conductor-watch-pending-sends.test.sh` 試験 12 の「symlink でも書き戻しは止まらない」と食い違うため）。受信箱にある所有者の検査は、受け入れ条件の 4 つに無いので足さなかった。
4. 足りなかった情報・検証手段: 移行の `O_EXCL`（`lexists` で確かめた後に置かれた symlink の先に書かない）は競合の窓を試験で作れず、`O_EXCL` を外した変異が生き残った。試験 9（旧置き場の FIFO）と試験 10（一時ファイルの後始末）は main でも通るため、`O_NONBLOCK` と `unlink` をそれぞれ外した変異で red になることを確かめた。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「受け入れ条件の書き方（このスキルの核）」に「レビューの範囲外指摘から起票するときは、指摘が挙げた行ごとに、それが読み取りか書き込みかを書き、その行を確かめる受け入れ条件を 1 つずつ置く」を足す。

使ったスキル: implement-issue
