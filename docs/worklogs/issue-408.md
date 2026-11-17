# Issue #408 の作業ログ

## 実装者メモ（round 1）

対象 head: b489b802ea8cdcff0dbc95f7cf5cd340b06a5a29

1. 詰まった箇所: scripts/tests/assign-blockby-docs.test.sh: 必須文言 `--paginate` を `grep -qF "$p"` に渡すと grep がオプションに解釈して落ちた。`grep -qF -- "$p"` と `--` を入れて直した
2. 曖昧だった文: 「見張り（conductor-watch）への組み込みの要否。実装担当が正本への記載内容から判断する。判断に迷う場合は needs-human で hatai に確認する」（Issue #408 本文の未決事項）
3. 仮定と解釈: conductor-watch.sh は検知のみで割り当て可否の判定ロジックを持たない（割り当て判断は指揮役が正本を読んで行う）と読んだ。4 通りの試験はその判定ロジックの試験なので、組み込まないなら置き場所が無い。よって組み込まず、組み込む場合の決まり（合成データの 4 通り）のみ正本に記載した。選ばなかった解釈: `agent:working` 付与イベントから割り当てコメントの存在を事後検査する形で入れる（コメント様式が 10 項目で規定されておらず、実装側の解釈で様式を固定することになるため選ばず）
4. 足りなかった情報・検証手段: 割り当てコメントの定型様式（blocker の番号・状態・取得時刻の並び）。様式が決まれば事後検査の機械化は可能
5. 改善案: docs/agents/grokbot-runbook.md の「割り当て・停止・再開」に割り当てコメントの定型様式（例: 行頭 `[assign-blocker]`）を足す。様式が決まると conductor-watch での事後検査が様式の grep だけで書ける

使ったスキル: implement-issue、write-pull-request
