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
