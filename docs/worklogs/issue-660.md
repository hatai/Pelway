# Issue #660 の作業ログ

## 実装者メモ（round 1）

対象 head: a26ed861e92e04ac59e952b9a3ffe0751b380094

1. 詰まった箇所: 手順 6: `mise run verify` が 600 秒の時間制限を超えたため（約 873 秒）、バックグラウンドに移して完了を待った（差分は文書だけ）。
2. 曖昧だった文: 「`docs/dev-environment.md` に「/home が ro のとき /tmp 側で作業する手順」として、fnox 実体のパス…が書かれている」（Issue #660 の「受け入れ条件」）
3. 仮定と解釈: 置き場所は節の指定が無いため、fnox を扱う既存の節「シークレットの扱い（fnox）」の下の小見出しと解釈した（独立した `##` 節や「既知の事項・未解決」には置かなかった）。shim が失敗する理由は、信頼の記録（`mise trust`）が /home 側にあり書けないためと仮定して書いた。
4. 足りなかった情報・検証手段: /home が ro の環境を手元で再現する手段が無く、手順は Issue の背景の記述どおりに書き、`command -v fnox` と `agent-gh dev api /rate_limit` の確かめ方は ro 環境では未実行。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「受け入れ条件の書き方」に「文書に書く条件では、書き足す先の節（見出し）を指定する」を足す。

使ったスキル: implement-issue, write-pull-request

## 実装者メモ（round 2）

対象 head: ce1763e316c93ccb804e6f93cd165a7d7d976c32

1. 詰まった箇所: 手順 9: codex と claude の must-fix（`mkdir -p /tmp/sshims` が別ユーザーの作った既存ディレクトリを受け入れ、PATH の先頭経由で `fnox` を差し替えられる）は同じ所見だが、直し方が `mktemp -d`（codex）と所有者の確認（claude）で分かれた。
2. 曖昧だった文: 「`/tmp/sshims` に symlink を置く形」（Issue #660 の「受け入れ条件」）
3. 仮定と解釈: `mktemp -d` だとパスが `/tmp/sshims` でなくなり受け入れ条件から外れるため、`/tmp/sshims` を保ったまま、自分が作ったか symlink でない自分所有のディレクトリのときだけ mode 700 にして使う形（claude の案）を選んだ。印の無い nit（原因が推定であること）は手順 9 の決まりどおり直していない。
4. 足りなかった情報・検証手段: 別ユーザー所有の `/tmp/sshims` を作る手段が無く、拒否の経路は root 所有の `/tmp` と symlink を対象に scratchpad で確かめた（どちらも「自分のものではない」で止まり、PATH に足さない）。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「受け入れ条件の書き方」に「共有の一時ディレクトリ（/tmp）の固定パスを条件に含めるときは、所有者の確認を条件に含める」を足す。

使ったスキル: implement-issue

## 実装者メモ（round 3）

対象 head: 1d19d4392203925bff92daf74ef966d8851d486f

1. 詰まった箇所: 手順 9: codex（既存ディレクトリの中身の検査）と claude（1 回の lstat で種類と所有者を確かめる）の must-fix を 1 つの if 条件にまとめた。`find` の中身検査は scratchpad で新規作成・自分の既存・余計なファイルあり・`/tmp/sshims` が symlink・`fnox` がディレクトリの 5 通りを確かめた。
2. 曖昧だった文: 「信頼できない内容が残っていないことを確認してください（安全に確認できなければ利用を拒否してください）」（PR #661 の round 2 レビュー、codex must-fix）
3. 仮定と解釈: 「安全な確認」は `fnox` という名前の symlink 以外の entry を 1 つでも見つけたら（`find -print -quit`）拒否する形と解釈した（entry ごとの所有者検査は選ばなかった）。`stat -c '%F %u'` が `directory <uid>` を返すのは GNU stat の Linux 環境の前提。
4. 足りなかった情報・検証手段: 別ユーザーを実際に作る手段が無く、rename 入れ替えの競合（TOCTOU）の実測はできていない（5 通りの scratchpad 検証で代替）。非英語 locale での `%F` の出力も未確認（環境は C locale）。
5. 改善案: `.agents/skills/review-pr/SKILL.md` の must-fix の書き方に「提案する直し方が Issue の受け入れ条件（例: 固定パス）と矛盾しないか確かめる」を足す。

使ったスキル: implement-issue

## 実装者メモ（round 4）

対象 head: 410b9e19244ffdb7eda5776312bccfa73b03fe6c

1. 詰まった箇所: 手順 6: merge 前の `mise run verify` が `deny-check` の RUSTSEC-2026-0592（`url` 2.5.8）で失敗した。main の PR #677（`url` 2.5.9 への bump）で直るもののため、決定の手順 3 の merge を前倒しし、merge 後のツリーで verify をやり直して通した。
2. 曖昧だった文: 「ラウンド数の上限は無視する方針です。round 4 に進める。」（開発リード経由の決定、2026-11-23）
3. 仮定と解釈: 決定はこの PR の round 4 に限った上限解除と解釈した（他の PR へは一般化しない）。印なしの nit 2 件（`LC_ALL=C`、`[ -k /tmp ]`）は must-fix 1 の条件を正しく書く一部として一緒に入れた（両担当が同じ箇所を指摘）。push は、決定の手順 2→3 の順で途中の赤い head を出さないよう、merge まで済ませた 1 回にまとめた。
4. 足りなかった情報・検証手段: ro 環境での手順の実行は引き続き未検証。`ln -sfn` 失敗時に古い symlink が残ることの実機確認はできていない（gnulib の実装読みによる。レビューの未確認事項と同じ）。`ln`・`find` の失敗経路は scratchpad に偽のコマンド（exit 1 する `ln`・`find`）を置いて再現した。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 6 に「verify の失敗が main で既に直されている既知のもの（アドバイザリ等）のときは、main の取り込みを前倒ししてよい」を足す。

使ったスキル: implement-issue

## 実装者メモ（round 5）

対象 head: 8c730af4fb72b95d1335d6ed3d8e7bea119f4e24

1. 詰まった箇所: 手順 9: claude の round 4 の must-fix（相対パスの `scripts/agent-gh` を `$clone` への `cd` なしで動かす）は、nit 1 の形（clone と `cd` を if の条件に入れ、`|| exit 1` を除く）と不可分だったため、その形で直した。scratchpad では偽の `git`（mkdir するだけ）で clone を模倣し、`cd` 後の pwd と `MISE_TRUSTED_CONFIG_PATHS` まで確認した。
2. 曖昧だった文: 「確かめ方は手順を始めたときのディレクトリで動く」（PR #661 の round 4 レビュー、claude の must-fix）
3. 仮定と解釈: 印なしの nit 1（`|| exit 1` がシェルごと閉じる、`mktemp` の失敗を見ない）は、must-fix の `cd` を入れる箇所と同じ条件に当たるため、レビューが示した形そのもので一緒に直した。`Implementer:` の行は round 4 以降の実装（opencode / GLM-5.3）に合わせた。
4. 足りなかった情報・検証手段: ro 環境での手順の実行は引き続き未検証。実 `git clone` を使った動作確認（scratchpad では偽 `git` で代替）。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「よくある失敗」に「文書の手順ブロックに `exit` を書くと、貼り付けたシェルごと閉じる。失敗は if/else で止める」を足す。

使ったスキル: implement-issue
