# Issue #723 の作業ログ

## 実装者メモ（round 1）

対象 head: 78ae63f4000eb2bd406e569e11cf1b95f5eccc53

1. 詰まった箇所: 手順 6 の push: `scripts/agent-gh-resign` の `createCommitOnBranch` が HTTP 502 を 4 回返し、実装コミットは署名なしのまま remote に載った（push 自体は成功）。同じ時間帯に App トークンの発行（500）と `pr create`（GraphQL エラー、502）も失敗し、`pr create` は再試行で通った。
2. 曖昧だった文: 「正本の書き方は、v0.7 の本文を消さずに、上書きされた箇所へ「ADR 0014 で置き換え」の注記とリンクを入れる。前例は Octane の採用（8d48a677）。」（Issue #723「採用する方式」）。8d48a677 は 08 の A1・A2 の行を書き換えていて、注記を足す形ではない。
3. 仮定と解釈: hatai がこのスレッドの grilling で選んだ 5 つを決定として扱った（日付と t コードを ADR から外す／openbot の要約を ADR に入れない／運用の仕組みを「決定」と「土台の Issue で決める案」に分ける／v0.7 は元の文を残して注記／GLOSSARY は変えず「デスクトップアプリ」「ブラウザ版」に揃える）。Issue に `ready-for-agent` は無いが、hatai の直接の指示で着手した。v0.7 以外の 7 ファイルも、元の文を残して注記を足す形にそろえた。
4. 足りなかった情報・検証手段: 受け入れ条件の「表の行数が 20 であることを目で確かめる」に、公式の項目の一覧の取り方が無かった。electron/electron の `docs/tutorial/security.md` を `scripts/agent-gh dev api repos/electron/electron/contents/...` で読んで突き合わせた。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「6. 検査してコミットし、行数を確かめてから push する」に「`agent-gh-resign` が GitHub の 5xx で失敗したときは、push は成功しているので先へ進み、次の push で付け替えを再試行する。署名なしのまま残ったら PR 本文に書く」を足す。

使ったスキル: grilling、domain-modeling、implement-issue、write-pull-request
