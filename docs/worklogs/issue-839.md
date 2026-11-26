# Issue #839 の作業ログ

## 実装者メモ（round 1）

対象 head: b36efc89440ebc0aab61581a05c800294d8df47a

着手の前に hatai と grilling で次を決めた（hatai の決定として扱う）。2026-11-26 21:43〜21:56 JST の起こし 11 回のうち 9 回は、エージェントが立てた実装スレッドの `waiting`（承認待ちの件数が 0 から増えるたびに出る）によるもので、指揮役自身の `finished` は `own-event` で止まっていた。決めたのは、同じスレッド・同じ round の `waiting` は最初の 1 件だけ起こすこと、起こしの最短間隔を 5 分にし（`failed`・`stalled` も例外にしない）、間隔の後に溜まった分で 1 回だけ起こし、読み終えていれば取り消すこと。

1. 詰まった箇所: `crates/pelway-agent-watch/src/main.rs` の `maybe_wake`: 新しい知らせが無い周回では呼ばれず、間隔の後の起こしを出す場所が無かった。起こし待ちの印があるときだけ、行を足さない `append_inbox(&[])` で受信箱の行数を数えて呼ぶようにした。
2. 曖昧だった文: 「push・チェック結果・レビュー結果・stalled・failed・needs-human では、これまでどおり起こされる。」（Issue #839 の「受け入れ条件」）。agent-watch の知らせには push・チェックの通過・needs-human が無い（GitHub の出来事は Grok の webhook にだけ届く）。この PR で確かめるのは、`failed`・`stalled`（`ci-failed-twice` を含む）・報告付き `finished`（レビュー結果）・エージェントが立てたスレッドの `finished` で起こすこと（最短間隔には従う）。Issue 本文の書き直しは指揮役か hatai に頼む。
3. 仮定と解釈: 承認待ちの round は、起こしに数えた時点で記録する（後の起こしが補助記録の読み損ねや `t3 thread send` の失敗で送れなくても、同じ round の繰り返しでは起こし直さない。知らせは受信箱に残る）。既読が進んでいない（`not-yet-read`）ときは、これまでどおり起こし待ちの印を付けない。選ばなかった解釈は、送れなかったときも印を付けて毎周送り直すこと（送り直さないという既存の決まりに反するため）。
4. 足りなかった情報・検証手段: いま動いている agent-watch（`~/.local/bin/pelway-agent-watch`）は 2026-11-21 のビルドで、#727 の判定が入っていない。この変更の効果は、マージの後に `mise run agent-watch-install` を実行し、pitchfork で再起動するまで出ない。指揮役の既読の位置（`patrol-state.json` の `inbox_offset`）が受信箱の末尾より約 35 行遅れている理由は分かっておらず、別の Issue で調べる。
5. 改善案: `docs/agents/notify-payloads.md` の「指揮役の受信箱」の `urgency` の説明に、「agent-watch は `urgency` で起こしを分けない（どの知らせも同じ最短間隔に従う）」と書き足す。

使ったスキル: grilling、implement-issue（ミューテーションテストは、`crates/pelway-agent-watch` が `.cargo/mutants.toml` の examine_globs に無いため回していない）
