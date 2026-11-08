# v0.6：Claude D5差分の採否（文書補正だけ）

v0.5の4担当はすべて賛成、Codexも文書賛成。ClaudeはD5-01〜04を完成の条件とした（D5-05/06は低）。v0.5とそれ以前の固定版は変更せず、以下の最小差分だけをv0.6候補にする。S1は未解決・実外部I/O無効。方式の採用、効力発生点の確定、実装/試験/実接続/支出の許可はしない。同じ固定版の全体再レビューは行わず、この補正箇所のみ確認する。

| ID | 採否 | 最小補正と根拠 |
|---|---|---|
| D5-01 | 採用 | 08のJ1案Aの同じセル内に、受理＝既送出分類は不採用で、実送出の順序化と検査後の後発送出拒否が条件と明記 |
| D5-02 | 意図を採用、安全条件を弱めない表現に補正 | 見出しをS1提案に変更。要求単位の独立した過去の送出なしの確定と後発送出拒否が両立する場合だけ未送信候補。送出前検査の記録だけでは、過去に送っていない証拠にならないと明記。具体証拠要件は未決 |
| D5-03 | 採用 | 08に提供元即時失効Adapter限定案の未採用・保留と、期限＋時計ずれだけの完了条件を不採用とする表を追加。調査/補助検査の検討まで否定せず、独立した強制点/正式判断を必要とする |
| D5-04 | 採用 | 08に送出能力の取消完了≠外部run停止証拠を明記。停止状態へ移すには03の状態表/A5の外部run終了・停止の証拠を別に必要とする |
| D5-05 | 採用 | 下記でv0.4→v0.5の依存87→102（追加15、削除0）と既存コピー更新を実manifest比較で列挙。固定v0.5のV3-04行（74→78）は変更しない。87→102の内訳は本ファイルとREADME末尾リンクで示す。v0.6同梱の別の証拠コピー（SHA-256: eb5984d6ae462f67578eba9a62f4e4e7cfa5b54fbaa1ac08741ee3424975c87f）に参照を追加した。旧コピー（SHA-256: 2601ca7852299f74fcffeed3c7e948294d42311bd0a69c8dc7f1b4c324a43078）は固定v0.5の台帳値と一致 |
| D5-06 | 採用 | 03にboundary-selected-external-descを1行追加。11の既存日英を完全一致で転記。外観/意味は変えない |

## v0.4→v0.5依存件数の内訳（凍結済みmanifest同士の比較）

本文12、生成した参照案内4は別集計。unchanged_source_copiesは凍結時のコピーを意味し、旧版との不変や現在の上流ファイルの不変を保証する名前ではない。

### 追加15件（削除0）

- `pelway-technical-docs/reviews/claude-v0.3.md`
- `pelway-technical-docs/reviews/codex-s1-analysis.md`
- `pelway-technical-docs/reviews/disposition-claude-v0.3.md`
- `pelway-technical-docs/reviews/disposition-v0.5.md`
- `pelway-technical-docs/reviews/market-v0.4.md`
- `pelway-technical-docs/reviews/product-s1.md`
- `pelway-technical-docs/reviews/product-v0.4.md`
- `pelway-technical-docs/reviews/requirements-map-v0.1.md`
- `pelway-technical-docs/reviews/review-requests-s1.json`
- `pelway-technical-docs/reviews/review-requests-v0.3.json`
- `pelway-technical-docs/reviews/technical-s1.md`
- `pelway-technical-docs/reviews/technical-v0.1-triage.md`
- `pelway-technical-docs/reviews/technical-v0.4.md`
- `pelway-technical-docs/reviews/ui-v0.3.md`
- `pelway-technical-docs/reviews/ui-v0.4.md`

### 既存コピーの内容更新（件数の増減には含めない）

- `pelway-technical-docs/reviews/disposition-v0.3.md`
- `pelway-technical-docs/status.md`

## その他の採否

Claudeの操作種類の英語表記、取消完了後の表示文は今回の必須修正でないため実装前の文言確認へ保留。既存9状態とS1-J1/J2の未決を変えない。名称衝突アンカーはstatic-v0.5.jsonを生成後に実見出しを確認済みで、初回レビューの提示資料はその更新前だった。最終静的記録では検査範囲と確認済み見出しを明記する。
