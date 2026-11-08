# UI担当レビュー：技術文書 v0.3（最終差分確認）

- 対象：`reviews/v0.3-snapshot/pelway-technical-docs/`（03、04、06、08、10、11）、`reviews/v0.2-to-v0.3.diff`、`reviews/disposition-v0.2.md`
- 重点：取消確認の表示、操作の種類、境界の説明の文言と翻訳
- 日時：2026-11-08 23:20ごろ（日本時間）
- 範囲：文書の整合だけを見ています。外観は評価も変更もしていません。固定版、本文、ほかの担当の原稿は変更していません。

## 判定：条件付き賛成

賛成の条件は、下の必須修正2件です。どちらも文書の1〜2行で直せます。

## v0.2の指摘の解消状況

| 項目 | 結果 | 確認した箇所 |
|---|---|---|
| U-08（未送信下書きを08の未決に入れる） | 解消 | 08の49行 |
| 作業中の取消 | 解消 | 03の42・78行、11の70行。止まったことを確認できるまでは作業中のままで、「停止確認中」を添えます。第10の状態は作りません |
| 操作の種類 | 解消 | 04の92行に4つの値と翻訳キー（`action-customer-send`、`action-agent-handoff`、`action-tool-read`、`action-tool-write`）があり、06の30行と11の74行も同じ値を引いています。`agent_handoff`という名前は3文書で一致しているので受け入れます |
| 境界表示の一貫性 | 解消 | 11の94〜95行、06の28行、10の63行（D9） |

## 必須修正

| ID | 箇所 | 指摘 | 修正案 |
|---|---|---|---|
| U3-01 | 08の13行（A5） | 「stopped＋停止確認中」が残っていて、03の42行・11の70行と逆になっています。技術担当のT3-01、プロダクト担当の指摘と同じです | 「止まったと確認できるまでは元の状態のまま、理由に停止確認中を添え、確認できたら止まっている（理由：取り消し依頼）へ移す」。`input/ui.md` §8（160行）は原入力なので変えず、差分として1行残す |
| U3-02 | 03の翻訳キー表（109〜116行）、10の74行（D11） | 送信開始の記録より後の取消の表示文がありません。このままでは「取り消しました」と出す実装を防げません（23:13にルームで合意した内容が未反映） | 03の表に1行追加：`reason-cancel-after-start`、日本語「取り消し依頼済み（送信は撤回されない可能性があります）」、English「Cancellation requested (the send may not be reversed)」。状態は送信中または結果不明のまま。D11の期待値に「この文言を出し、『取り消しました』とは表示しない」を追加 |

## 文言の最終確認（11の94行の「UI担当の確認待ち」への回答）

| 安定ID | 日本語 | English | 判定 |
|---|---|---|---|
| boundary-selected-external（ラベル） | 国内限定の外 | Outside Pelway's domestic-only scope | このままで可 |
| boundary-selected-external の説明（新しいキー案：`boundary-selected-external-desc`） | Pelway本体の国内限定の範囲外です。送り先で国外処理・保存される可能性があります（未確認の場合を含む）。 | Outside Pelway's domestic-only scope. The destination may process or store data outside Japan, or this has not been confirmed. | 追加を推奨。「国外に置かれている」とも「使ってはいけない」とも読めない書き方にしています |
| content-external-unverified | 外部由来・未確認 | External content, unverified | このままで可 |
| reason-stop-confirming | 停止確認中 | Awaiting stop confirmation | このままで可 |
| reason-cancel-requested | 取り消し依頼 | Cancellation requested | このままで可 |
| reason-integration-revoked | 連携の取り消し | Integration revoked | このままで可 |
| action-agent-handoff | 外部への受け渡し | Handoff to external agent | English案。`tool_read`はRead from tool、`tool_write`はUpdate in tool、`customer_send`はSend to customer |

どの文言にも「準拠」「適合」「国内で完結」は含めていません。

## 実装前の未決（変わらず）

- 390pxの承認幅
- Slintのライセンスの選び方と、AboutSlintの部品を置く場所
- 複数のメッセージにまたがる選択とコピー
- 外部メール、国外端末、データの持ち出し
- 結果不明の顧客向け送信を人が確かめた後にどう閉じるか（プロダクト担当の(a)と(b)）。UIからは、(b)を選ぶ場合、二重送信のおそれを再送ボタンの直前に文字で出すことを条件にしたいです
