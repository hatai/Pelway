# 決定後のメール下書き

状態: 失敗（main に決定後のメール下書き生成が無い）。

## Sub-features

- 決定の確定をきっかけに、決まった案からハルが顧客宛てメールの下書きを作り、チャンネルに出す
- 下書きの保存（作られた下書きを API で読み直せること）

## How to get to it (user POV)

山本が案A を選んで決定を確定すると、ハルが #案件B に、決まった内容を書いたメールの下書きを投稿する。

## Driving it with verify-run

`verify-run --feature mail-draft`。`decision` と同じ段 1〜6 のあと、段 7 で 10 秒待ち、決定の前の最後の `seq` より後にハルの投稿があるかを `GET .../messages?after_seq=<seq>` で読む。応答は `steps/NN-messages-after-decide.resp`。

## Gotchas

- main の runner-lite は判断待ち（AwaitingDecision）で止まり、決定の後に何も投稿しない（`crates/pelway-runner/src/lib.rs` の冒頭のコメント「決定後のメール下書き生成は PR-3 で加える」）。下書き生成は Issue #330 の PR-3（PR #399）で入る予定で、main には無い。そのため段 7 は FAIL になる。待ち時間を延ばしたり段 7 を外したりして通さない。
- 下書きの保存先（下書きを読み直す経路）も main には無い。下書き生成が入ったら、この機能の段 7 を「保存された下書きを読み直す」形に直す。
- 外部へのメール送信はこのスキルの対象外（疑似送信だけを使う）。
