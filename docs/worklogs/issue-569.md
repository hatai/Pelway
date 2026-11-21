# Issue #569 の作業ログ

## 実装者メモ（round 1）

対象 head: fb0ee77ae6f8927f6c869daf0b8a5115ebf9d8d8

1. 詰まった箇所: `scripts/tests/conductor-watch-stall.test.sh` の試験 14: 自分の所有でないファイルを sudo の無いローカルで作れない。`sudo -n chown`（CI の runner）と `unshare --map-auto -r` の中での `chown 1:1`（ローカル）の二段にし、どちらも使えないときは `ok` ではなく `skip 14` を出す形にした。
2. 曖昧だった文: 「受信箱が 4 MiB を超えるとき、読むのは末尾の 4 MiB 以内で、読み始めの断片を捨てる処理も含めて読み取りの量に上限がある → 同試験（改行の無い 64 MiB のファイルで、巡回が 10 秒以内に終わる）で ok」（Issue #569 の受け入れ条件）。今の読み手も 64 MiB を 10 秒以内に読み終えるので、時間だけでは red にならない。
3. 仮定と解釈: 末尾 4 MiB の外（先頭）に置いた `waiting-review` の行が読まれないこと（試験 15）と、末尾の中の行が読まれること（試験 16）を観測に足し、時間の条件と合わせて満たすと解釈した。`readline()` の無上限は静止したファイルでは試験で区別できないため、`read(INBOX_TAIL)` 1 回で読む形の構造で上限を持たせた。最後の LF の後ろは `inbox_offset` の数え方に合わせて読まない。
4. 足りなかった情報・検証手段: Issue の「採用する方式」が「なし」だが、背景に PR #561 head 57cc371 の叩き台があり、それを方式として扱った。CI の runner で `sudo -n` が使えるかは、push 後の verify のログ（`info 14:` の行）でしか確かめられない。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「受け入れ条件の書き方」に「時間の上限で上限処理を確かめる条件は、今のコードでその時間を超えるかを先に測り、超えないときは観測できる効果（読まれない行など）を条件に足す」を足す。

使ったスキル: implement-issue、write-pull-request、pr-watch（輸入スキルは使っていない。手順 4 の red→green は implement-issue の本文どおりに行い、`.agents/skills/tdd` は試行の割り当てでないため読んでいない）
