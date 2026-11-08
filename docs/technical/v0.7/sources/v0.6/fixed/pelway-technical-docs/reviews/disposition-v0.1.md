# v0.1レビューの採否と修正先（v0.2改訂中）
対象はv0.1固定版への4担当＋Codex/Claudeの実レビュー。外部レビューは文書整合/設計だけで、出典再検証/実装/実機合格ではない。採用は文書への反映案であり、最終差分確認・ユーザーの仕様/実行判断とは別。
| 指摘ID | 修正先 | 採否・理由と残り |
|---|---|---|
| TR-1 | 03 遷移表 | 採用。未実行時の取消/失効を追加提案で明記。PRD未承認。 |
| TR-2 | 03 固定承認 / 04 external_actions | 改訂指摘を採用。承認時と呼出直前の検査を残し、予約とattempt_startedを分離。開始後は取消で結果を隠さない。 |
| TR-3 | 02 / 09 / Discord証拠 | 精密化して採用。移行方針/GPUI表明のみ確認、完了/一般配布/比較数値は未確認。 |
| TR-4 | 01 全体図 / 11 bridge | 採用。利用者端末bridgeを管理host外へ。接続方式/国外端末許可は未決。 |
| TR-5 | 04 順序 | 将来の代替として採用。性能不成立時のdispatcher案で、今の方式を変更しない。 |
| TR-6 | 11 能力定義 | 採用。event key/期限/順序/cursor/照会契約をAdapter能力へ。 |
| TR-7 | 05 / 10 | 阻害を維持。Apple内部network未確認なら機密実行を始めない。 |
| PR-1 | 03 遷移表 | 採用。取消行と承認失効。sending/unknownは取消要求だけでは遷移させず照会。attempt前の未送信確定拒否は区別。 |
| PR-2 | 03 / 11 状態対応 | 採用案。外部受渡しは承認待ち→送信中→作業中。操作種別で顧客送信と区別、PRD未承認と表示。 |
| PR-3 | 08 未決 / 11 | 採用。再開/次runの承認粒度を未決にし、追加promptや副作用再試行で迂回させない。 |
| PR-4 | 08 試験追跡表 | 採用。T1〜T24/T21′とD/章/段階をcanonical内へ。T6 keyの追加差分は明示し確認待ち。 |
| PR-5 | 11 三つの往復例 | 採用。各段階へ状態名を付記。 |
| U-01 | 11 表示契約 / 06 / D9 | 採用。国内限定の外/外部由来未確認/4主体の文字ラベル/保証禁止。操作種類を状態横と一覧に表示する最新補足も反映。 |
| U-02 | 02 UI推奨 | 採用。Slint採用案であり試験/固定版許諾前の未確定。 |
| U-03 | 06 読み上げ | 採用。4主体と外部連携名を読み上げる。 |
| U-04 | 03 理由表 / 06 翻訳 | 採用。状態理由/境界/操作種類を翻訳キー化。 |
| U-05 | 02 見出し | 採用。Slintの重複語を修正。 |
| U-06 | 02 比較表 | 採用。表示範囲/読んでいる位置/変換中の文字/候補窓へ。 |
| U-07 | 07 描画試験 | 採用。200%文字拡大/高コントラストを別測定。 |
| M-01 | 02 / 09 / 市場レビュー | 採用。市場原稿の旧説明を自己訂正した根拠を記録。GPUIの読み上げ阻害は別問題で維持。 |
| M-02 | 次版固定方法 | 採用。v0.1は保持し、v0.2はworkspace内の相対配置を維持して本文と根拠原稿を固定/別manifest化する。既存下書き内の欠落4リンクは原文を変えず、固定パッケージ内だけの参照案内で補う。静的検査結果は別記録。 |
| M-03 | 02 / 09 依存台帳 | 採用。技術照合記録の15件/比較候補を09へ統合し02/08から参照。rmcp移行中MIT/Slint選択/固定版未照合を保持。全推移依存/NOTICE/SBOMは未完成。 |
| M-04 | 02 工数根拠 | 採用。input/ui.md §3a/5と統合07へ明記。係数による定量推計済みとはしない。 |
| CX-R01 | 03 検査/attempt境界 / D11 | 採用。失効と受理を同じepoch/versionで順序化し、予約前後/開始後の注入を追加。 |
| CX-R02 | 04 外部イベント帰属 | 採用。登録済job/account→run対応から解決し、payload内IDを権限根拠にしない。 |
| CX-R03 | 04 再同期 / 08 試験 | 採用。全subject/権限/watermarkは同一REPEATABLE READ snapshot。分割取得は固定manifestを要する。 |
| CX-R04 | 11 不変export/session | 採用。manifest/hash/repo版/session所有を固定。taskをまたぐ履歴利用を認めない初期案。 |
| CX-R05 | 03 論理操作 / 04 / 08 T6差分 | 採用案。operation/provider keyと承認版keyを分離。原入力T6と異なるので最終差分確認待ち。未実行証拠がない再試行を保留。 |
| CX-R06 | 04 / 11 inbox | 採用。workspace/integration/account/event keyへ統一し、同一key別hashは隔離。 |
| CX-R07 | 03 / 11 状態対応 | 採用案。PR-2と統合、9状態を増やさず未承認の追加遷移で区別。 |
| CX-R08 | 11 取消後の限定確認 / 08 | 採用。限定停止/照会は別の明示権限案。全面取消なら自動確認不能。 |
| CX-R09 | 04 成果物耐久化 / 08 | 採用。ファイル確定→DB参照/checkpoint、欠損時保留。整合backup手順/実測は未決。 |
| CX-R10 | 08 安全ゲート表 / references/plan-gates.md | 採用。原版/節/行/hashと反復条件/注入/永続期待値を追跡。未指定の回数は未決。 |
| CL-R1 | 01 / 08 | 採用。初期は単一host多アカウントの合成検証だけ。LAN/TLS/認証は未決。 |
| CL-R2 | 03 / 11 | 採用案。PR/TRの遷移修正と統合、送信済みから顧客修正で作業中も追加提案。 |
| CL-R3 | 11 専用CLI区画 / 08 | 採用。host直実行/本体出口緩和で代用しない。認証主体・区画の実装方式は未決。 |
| CL-R4 | 01 / 11 Adapterとbroker | 趣旨を採用。更に資格情報はtrusted gateway transport内で付与し、Adapterへrefresh token/自由出口を渡さない案。 |
| CL-R5 | README / 04 / 10 / 11 | 採用。integration.enable対象scope権限に統一し、閲覧とexport可否を区別する。 |
| CL-R6 | 03 承認集合 / 04 | 採用。個別賛成票ではなくlogical operation単位の一意性/未解決試行制約。 |
| CL-R7 | 04 evidence API / 11 | 限定採用。GETは保存済証拠、外部照会をPOSTと明示grantへ分離。GETが外部readを行うだけでHTTP違反という一般論は根拠にしない。 |
| CL-R8 | 03 / 08 T6差分 | CX-R05と統合採用案。再承認でも同じ副作用のprovider keyを維持する条件を記述。 |
| CL-R9 | 01 矢印 / 03 / 04 | 採用。gateway→serverの予約/attempt記録経路と前後を明示。 |
| CL-R10 | 03 / 04 decision API / 08 | 子task入口を追加提案。LLMだけの判断確定を拒否しUI確認を初期安全提案とする。自然返信のserver確定規則はプロダクト確認待ち。 |
| CL-R11 | 05 / D4 / 08 | 採用案。controller高権限/3backend工数未決、Windows試験はPodman一つを第一候補。host推論へのsandbox到達不可を試験へ。 |
| CL-R12 | 08 暗号化 | 採用。server/backup保存時暗号化と鍵管理は未決。完了済みとはしない。 |
| CL-R13 | 04 同期/lock/監査 | 採用。workspace→subject→operation順、lock中外部I/Oなし、REPEATABLE READ、workspace hash鎖。 |
| CL-R14 | 04 / 11 inbox | CX-R06と統合採用。 |
| CL-R15 | 04 / 10 policy | 採用。core_domestic_policyへ統一。 |
| CL-R16 | 01 loopback表記 | 採用。合成試験HTTP/WS。実チームのLAN/TLSは別未決。 |
| CL-R17 | 11 PlacementPolicy | 採用。影響grant/未実行承認の再検査と必要時再同意。地域機能を増やさない。 |
| CL-R18 | 08 / references/plan-gates.md | 採用。版/節/原文/hash、A/B/C/相対31日/反復条件を明記。 |
| CL-R19 | status 要件受領原本 | 範囲限定採用。ルームの転送受領記録を根拠とし、本人DMを直接確認済みとはしない。 |
| CL-R20 | 06 formatter | 採用。browser/ICU/CLDR比較版とfixture差異を固定/記録。 |

各レビュー原文は同じreviews内のcodex-v0.1.md、claude-v0.1.md、technical-v0.1.md、product-v0.1.md、market-v0.1.mdとinput/ui-review.md。全53指摘。M-03差分を統合済み。次版固定/参照検査を別記録し、4担当/外部の最終差分確認を依頼する。最終賛同は未取得。既存v0.1 snapshot/manifestを変更しない。
