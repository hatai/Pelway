<a id="ch03-s01"></a>
# 03 Bot実行・権限・承認・隔離

<a id="ch03-s02"></a>
## 役割と起動

Botは会社・チャンネルのメンバーで、AI表示を持つ。メンションされた依頼だけを既存の起動条件とする。外部アラートをメンションなしで受け付ける機能は将来案で未承認。外部文書やログの「実行せよ」は依頼に昇格させない。

`task`は仕事、`run`は実行の試行、`step`は再開できる手順、`operation`は外部副作用の単位。runの再作成とoperationの再送を同一視しない。人格・会話・永続メモリは実行環境の外に置く。

<a id="ch03-s03"></a>
## 9状態と遷移

正規定義はRustの `TaskState` 一つ。API schema、TypeScript型、翻訳キー、両UIの表示をそこへ対応付ける。

| enum | 日本語 | 英語の表示案 |
|---|---|---|
| requested | 依頼済み | Requested |
| working | 作業中 | Working |
| awaiting_decision | 判断待ち | Awaiting decision |
| awaiting_send_approval | 送信承認待ち | Awaiting send approval |
| sending | 送信中 | Sending |
| sent_awaiting_reply | 送信済み・顧客回答待ち | Sent, awaiting customer reply |
| outcome_unknown | 結果不明 | Outcome unknown |
| stopped | 止まっている | Stopped |
| done | 完了 | Done |

| 移る前 | 移る先 | きっかけと条件 |
|---|---|---|
| なし | 依頼済み | 人のメンションによる依頼をserverが受付 |
| 依頼済み | 作業中 | runnerが実行リースを取得 |
| 作業中 | 判断待ち | Botの質問。判断者と論点を必須保存 |
| 判断待ち | 作業中 | 権限ある人の決定。短い「Aで」は決定候補。server提示の不変版/選択肢を権限ある人がUIで確認して決定する初期安全提案。自然な返信からのserver確定規則は[プロダクトv0.2の5条件案](../sources/v0.6/fixed/pelway-technical-docs/reviews/product-v0.2.md)と08を参照。濱田さんの正式仕様判断待ち |
| 作業中 | 送信承認待ち | 外へ出す操作のpayloadを固定 |
| 送信承認待ち | 作業中 | 人が修正を依頼 |
| 送信承認待ち | 送信中 | 有効な固定承認と権限を検査し、dispatchを予約。外部呼出はまだ行わない |
| 送信中 | 送信済み・顧客回答待ち | 顧客向け送信の成功証拠を保存 |
| 送信中 | 止まっている | 開始受理前の検査で拒否し、開始更新が成立しておらず未送信が確定している場合だけ。理由は権限・承認の失効。I/O直前の追加中止検査とは区別する |
| 送信中 | 結果不明 | attempt_started記録後の応答断・実行所有者喪失、または開始受理の永続commit/応答が不明で未送信を確定できない場合。送らず照合し、実際には未送信でも保守的に扱う |
| 結果不明 | 送信済み・顧客回答待ち | 顧客向け送信の照会証拠を見た人が「送れていた」と判断 |
| 結果不明 | 送信承認待ち | serverが技術的未実行の確証または提供元の有効な冪等保証を検証し、人が再承認を選ぶ場合だけ。証拠不足なら結果不明を保持。予約時にも再検査 |
| 送信済み・顧客回答待ち | 完了 | 顧客の合意を確認 |
| 作業中 | 完了 | 社内完結の成果と必要な人の確認が揃う |
| 作業中 | 止まっている | 障害・上限・権限不足。ただし未停止の外部runがあれば停止済みとは扱わず、以下の取消確認を適用 |
| 作業中 | 作業中 | ［P方針の具体化・遷移は要確認］取消commit後の新規内部開始受理を拒否。既受理の操作と外部runは停止を試みるが保証しない。集約行にreason-cancel-accepted、未確定があればreason-cancel-accepted-pending |
| 作業中 | 止まっている | ［P方針の具体化・遷移は要確認］取消要求済みで対象runの終了/停止を証拠で確認。sending/unknownの操作は結果を別途保持 |
| 止まっている | 作業中 | 人が復旧条件を確認して再開 |
| なし | 依頼済み | ［提案・要確認］serverが親taskと委任scopeから子taskを作成。Botの文章だけでは作成しない |
| 送信済み・顧客回答待ち | 作業中 | ［提案・要確認］顧客の修正要求を確認。既送信を取り消した扱いにしない |
| 依頼済み | 止まっている | ［P方針の具体化・遷移は要確認］取消要求、権限/連携喪失で受付を止める |
| 判断待ち | 止まっている | ［P方針の具体化・遷移は要確認］取消要求、判断者/権限/連携の喪失。未停止の外部runがないか、停止証拠を確認できた場合だけ |
| 判断待ち | 判断待ち | ［P方針の具体化・遷移は要確認］取消要求時に外部runの停止未確認なら元状態を保ち、取消受付済みと未確定件数を集約表示し、外部run行には停止要求済み（結果確認中）を添える。判断の確定や新dispatchは許可しない |
| 送信承認待ち | 止まっている | ［P方針の具体化・遷移は要確認］取消、期限切れ、権限/連携失効。未実行の承認を失効する |
| 送信中 | 作業中 | ［提案・要確認］agentへの委任/業務tool操作を提供元が受理したと確認。顧客向け送信以外に限る |
| 結果不明 | 作業中 | ［提案・要確認］顧客向け以外の操作で、人が照会証拠を見て受理済みと判断 |

上表の追加提案を除く行は作業用PRD §5を具体化したもの。予約前の編集は前承認を失効させ送信承認待ちのままとする。予約済み（送信中）で開始受理前の編集/失効は、未送信を確定できる場合だけ表の送信中→止まっている（承認の失効）を使い、送信承認待ちへ直接戻さない。再開には新しい承認決定版が必要で、同じ承認版を再予約しない。開始後は元snapshot/operation/試行/結果を不変で残し、編集を後続案として保存する。元操作が結果不明なら後続案の別ID/別承認による代替送信も解決まで保留する。既送信確定後の修正送信は元送信に関連する別操作とする。P方針の意味は暫定採用済みだが、取消遷移の行単位はその具体化案として08 U06で確認する。agent/tool受理後の作業中への遷移も未確認で、元PRDの採用範囲を自動で広げない。送信中/結果不明は取消要求だけでstoppedへ移さず、結果照会を継続する。送信中で未送信が確定した直前拒否だけは表の既存拒否行を使える。attempt_started後はこの例外を使わない。

再接続中や重複受信だけではタスクを結果不明にしない。接続状態は別の欄に持つ。外部runの終了と仕事の完了は別で、成果物未取込・未確認なら完了にしない。

<a id="ch03-s04"></a>
## 暫定採用した取消P方針と残るS1

通常のtask/operation取消は、取消commit後の新しい内部開始受理を禁止し、既に開始受理した一操作は停止・撤回を保証しない。開始受理後かつ物理送出前の競合も含む。これは濱田さんがJ-S07で暫定採用した明示的な要件変更であり、旧S1の技術的解消ではない。開始受理と取消を同じ操作状態で一意に順序化し、未開始の後続・resume・再試行で取消を迂回しない。

内部開始受理は送信済みの証拠ではない。送出結果と外部run停止を別々に保持する。根拠失効は通常取消とは別に検査し、停止可能な処理と新段階への移行を抑止する。法定義務を非保証で免除しない。S1の方式・障害/復旧検証は未達で実外部I/Oは無効。残確認は[未決U01](08-decisions-open.md#u01)へ集約する。

<a id="ch03-s05"></a>
## 固定承認と外部副作用

1. gatewayが操作名、送信アカウント、宛先、本文、添付の内容hash、参照版、許可scope、integration版を正規化する。添付名だけでは固定しない。
2. serverは不変の `ActionSnapshot` を保存し、diffと送信先・利用データを人に提示する。外部連携では国外処理・保存の可能性と未確認を表示する。
3. 承認者はそのsnapshotのhashとversionへ承認する。決定する権限と送信する権限を分ける。有効化への同意はこの個別承認を代替しない。
4. 承認確定時に目的、source、必要当事者を覆う根拠版/epoch、保持/出力先、会社、依頼者、チャンネル、担当、承認者、連携アカウントの現権限、scope、有効化版、承認期限、snapshot一致を再検査する。
5. DBの条件付き更新でdispatch予約を一度だけ確保する。固定承認版の処理keyは一つ。個々の賛成票ごとには作らない。同時承認にはversion競合を返す。提供元の副作用keyは下記のlogical operationへ固定する。予約は送信を試みた証拠ではない。
6. 予約済みの一操作/attempt/payloadに対しserverが一回限りの開始資格を発行する。発行時刻を耐久記録し、発行から内部開始受理まで5秒を上限とする。task全体/後続queueへ資格をまとめて発行しない。発行は開始受理でも物理送出でもない。
7. gatewayは外部呼出直前にその資格でserverへ開始受理を要求する。serverは現在の権限・目的/根拠・取消・snapshot hash/承認期限/集合・policy/grant epoch/version・lease/fence・5秒期限を同じlock/transaction内で検査し、予約済みかつ未開始で資格が一致する一行だけを消費済み/attempt_startedへ更新する。取消更新と一意に順序化する。永続commitを確認した成功応答は当該gateway実行にだけ返す。重複要求/受理照会/再起動で資格を再発行せず、成功応答を再実行許可へ読み替えない。commit又は受理応答不明なら送信せずoutcome_unknownへ回す。拒否だけで過去の引渡し済み試行の未送信を確定しない。DBの耐久設定・方式は未検証で実I/Oは無効。
8. 受理を確認した当該gatewayだけが固定payloadで外部I/Oを一回試みる。HTTP client/proxyを含む自動再試行を禁止する。開始記録後のクラッシュ/応答断ではoutcome_unknownとし、同じ開始記録から再実行しない。P方針では受理後・実送出前の取消でも送出し得る残余リスクを説明する。Pは暫定採用済みの要件変更だが、方式の実装適合は未検証。I/O直前の追加の中止検査は新しい未送信証拠にせず、開始後の不確実な結果を保持する。DBと外部HTTPを原子的な一処理とはしない。

本文・宛先・添付・送信アカウント・権限scopeの変更は再承認。既存承認を上書きしない。snapshot承認の承認者・期限・複数承認者の規則は[ADR 0002](../../../adr/0002-designated-approver-and-expiry.md)。開始資格の5秒とは別である。

事前決裁された型へ一致する外部操作の自動実行は、ADR 0002の固定承認を個別snapshotから型（生成ルールのテンプレート・宛先範囲・金額上限・データ範囲の組、版管理あり）へ拡張したものである（[ADR 0009](../../../adr/0009-hybrid-approval-with-preapproved-types.md)）。機械的検査（宛先ドメイン・金額・添付有無・初回宛先）とモデル審査（番人。意味的逸脱の検出）の両方へ一致する場合だけ自動実行し、どちらかでも不合格なら送信承認待ちへ落とす（fail-closed）。取り消せない操作（削除・支払い・権限の変更）は型による自動実行の対象外とし、個別snapshot承認のまま扱う。メールやWebhookなど外部起点の実行は対象外とせず一段厳しく扱い、外部の文章に宛先や内容を決めさせない。型一致の判定はPelwayの中で組み立てられた内容に対して行い、外部の文面をそのまま判定の入力にしない。自動実行は全件記録・日次ダイジェストで承認者が確認し、型単位で即時停止できる。実行記録と監査には根拠となった型の版を結び付ける。

提供元が冪等キーや照会を持たない場合、Pelway内の二重dispatch防止しか保証できない。ローカルで送信前記録と実際の外部副作用を完全に一トランザクションにはできない。初期の疑似送信で限界を試し、照会・照合方法がない外部更新は原則保留する。例外採用にはリスクを示した別判断を要する。

<a id="ch03-s06"></a>
## 実行所有者、再開、引継ぎ、取消

`run`のリース取得でfencing tokenを増やす。gatewayとDBは古いtokenの更新・新dispatchを拒否する。期限切れを理由に同じ外部操作を別runnerへ再送させない。checkpointには完了手順、未完了手順、operation結果、成果物参照を保存する。再開は未完了手順からで、成功済み副作用を再実行しない。

引継ぎ欄には、決まった内容と決定者・時刻、送った宛先・本文・添付・時刻、現在状態、残作業、誰の判断待ちかを残す。作業途中のファイルとログへの権限付き参照も渡す。担当交代で承認を有効のまま保つかは入力間に差がある。初版推奨は安全側に新実行者の確認と必要時再承認。内容・アカウント・scopeが変わった承認は必ず失効させる。この差分は08に残す。

取消は第10状態を作らず、`cancel_accepted_at`へ受付commit時刻を記録し新しい内部開始受理を拒否する。既受理の一操作は未送信でも今後送出し得ると説明し、抑止/停止要求を試みるが保証しない。集約行は受付と未確定件数、個別操作は未送信（確証あり）/送信済み（証拠あり）/結果不明/停止要求済み（結果確認中）を区別する。停止要求は要求の事実であって停止完了や送信結果とは別軸。reason-cancel-after-startは個別送信行専用。外部runは停止確認まで元状態を保ち、reason-stop-confirmingを外部run行だけに付す。送信中/結果不明の記録を取消で消さず照会を残し、「取り消しました」と表示しない。行単位の遷移採否は08 U06、長引く外部runの停止確認期限はU07に集約し、操作結果不明のX03〜06と混同しない。取消要求だけで新しい実行runを作らない。

BotがBotへ依頼する子タスクは親taskへ関連付ける。人の確認要否は既存未決。初期案は読み取りと内部下書きだけで、親の権限・承認を複製しない。

<a id="ch03-s07"></a>
## 隔離と秘密

| 案 | 利点 | 制約 | 初版の判断 |
|---|---|---|---|
| Botごとの常時VM | 作業環境・常駐プロセスを維持しやすい | ローカル機材を消費。停止復元の試験が必要 | 比較対象 |
| タスクごとのコンテナ＋永続成果物 | Linuxツールを共通化、必要時だけ起動 | カーネル共有。ログイン済みブラウザの継続等は別課題 | 合成データ検証の推奨案。安全承認前に実行しない |
| microVM / gVisor等 | 分離強化の候補（gVisor runsc、crun＋libkrun） | CPU・OS・syscall・起動費用・egress拒否の維持の検証 | MVP前に比較 |
| ホストOS別sandbox | ホストツールに近い | 3OS別実装、ホスト侵害の検証範囲が大きい | 初版では採らない |

sandboxは非root、capabilityなし、read-onlyルート、task用の書込領域のみ。CPU・メモリ・PID・時間の上限を設定する。ホストホーム、DB資格情報、container runtime socketを渡さない。起動controllerは許可済みイメージとvolumeだけを受け付ける。

秘密はgatewayの資格情報brokerに置く。OS credential storeの参照とコンテナ内プロセスの関係は05の境界で設計し、Bot・LLM・成果物・ログへ値を出さない。短命でtask/scopeを限定したbroker用tokenと本サービスのrefresh tokenを区別する。

MCPはツール呼出し方式の一つで、外部agentの全ジョブ管理・隔離ではない。CLI、hook、API、webhookも同じgatewayの権限・承認を通す。ツール説明や外部文書はデータであり、管理者の許可を拡張しない。

<a id="ch03-s08"></a>
## 監査とローカル推論

監査はDBの専用書込権限による追記型とし、actor、承認snapshot、policy版、操作、結果、時刻、correlation IDを保存する。hash chainは改ざん検知の補助で、ホスト管理者まで変更不能にする仕組みではない。保持・削除・国内バックアップは[13のR11/R15とD01〜05](13-operational-defaults.md)を適用する。

安全試験はスタブを用いる。実推論は提供元のAPIをLLMゲートウェイ経由で使う。MVPではテナントが自分で契約した提供元（BYOK、契約者の責任による国内完結の例外）だけを使い、一般提供では日本リージョンのある提供元を既定にする。提供元の自動切替は禁じる（[ADR 0008](../../../adr/0008-llm-provider-and-byok.md)）。外部コーディングagentが自分の提供元で推論することは11の利用者選択連携であり、本体推論をそこへ無断で転送しない。

<a id="ch03-s09"></a>
## 「作業を見る」の境界

作業画面は自動で開かず、利用者が「作業を見る」を押して開く。基本は数秒おきの静止画と手順log。live remote desktop/動画配信を初期要件へ追加しない。取得対象はtaskに許可された作業領域だけで、host desktop全体・secret入力画面を収集しない。画像もartifactと同じ権限/保持/国内保存を適用し、外部連携へ自動送付しない。

<a id="ch03-s10"></a>
## 理由/境界の共通翻訳キー案

| 安定ID | 日本語 | English |
|---|---|---|
| reason-stop-confirming | 停止要求済み（結果確認中） | Stop requested (confirmation pending) |
| reason-reconnecting | 再接続中 | Reconnecting |
| reason-integration-revoked | 連携の取り消し | Integration revoked |
| reason-cancel-accepted | 取り消しを受け付けました。開始受理前の送信は止めます。開始受理済みの送信は止められない場合があります。 | Cancellation accepted. Sends not yet accepted for internal start will be stopped; accepted sends may not be stoppable. |
| reason-cancel-accepted-pending | 取消受付済み · 開始受理済みの送信{count}件は結果確認中（撤回されない可能性があります） | Cancellation accepted · {count} send(s) accepted for internal start are still being checked (may not be reversed) |
| reason-cancel-after-start | 停止要求済み · 結果確認中（撤回されない可能性があります） | Stop requested · checking result (may not be reversed) |
| send-row-not-started | 未送信（取消により開始していません） | Not sent (not started due to cancellation) |
| send-row-sent | 送信済み（撤回されていません） | Sent (not reversed) |
| send-row-unknown | 結果不明 · 確認中 | Result unknown · checking |
| boundary-selected-external | 国内限定の外 | Outside Pelway's domestic-only scope |
| boundary-selected-external-desc | Pelway本体の国内限定の範囲外です。送り先で国外処理・保存される可能性があります（未確認の場合を含む）。 | Outside Pelway's domestic-only scope. The destination may process or store data outside Japan, or this has not been confirmed. |
| content-external-unverified | 外部由来・未確認 | External content, unverified |
| integration-connected-no-business-data | 接続済み · 業務データは未送信 | Connected · No business data sent |
| notice-ai-input-excluded | 一部の発言を除外して処理しました（利用の同意を確認できない範囲） | Some messages were excluded (consent for this use could not be confirmed) |
| state-use-stopped | 利用停止中（削除は未完了） | Use stopped (deletion not yet complete) |
| state-deletion-in-progress | 削除処理中（受付 {datetime} JST） | Deletion in progress (requested {datetime} JST) |
| state-deletion-completed | 削除完了（オンライン・索引・メモリ・端末で確認済み） | Deletion completed (verified for online data, index, memory, and devices) |
| state-deletion-backup-note | バックアップの復旧可能期間は作成から最大{backup_days}日です。復旧対象からの除外と物理消去の確認状況を分けて表示します。 | Backups are restorable for up to {backup_days} days from creation. Restore exclusion and confirmed physical deletion are shown separately. |

9状態を増やさず理由/接続表示を別キーで付ける。将来地域が増えた場合の表示文は配置方針へ関連付けるが、今は日本単一の文言だけを設計する。

<a id="ch03-s11"></a>
## 論理操作・固定承認・試行の関係

`logical_operation_id`は、宛先/account/型付き操作/目的/payload hashが固定された一つの意図した副作用を表す。同じ文章を別の目的で2回送るなら、人が別操作を指定する。内容hashだけをIDとしない。`action_snapshot`はその入力とpolicyの不変版、`approval_decision`はsnapshotへの承認集合（複数人の票を含め得る）、`dispatch_attempt`は試行の記録である。

1つの固定承認版に1つのローカル処理keyを付ける。一方、提供元へ送る冪等keyはlogical operationへ固定し、同じ副作用の再承認だけで変えない。提供元account/APIとpayloadが同じで、key保持期限・再試行意味が有効な場合に限り再使用する。新payload/別操作なら新operationとkeyにし、旧operationとの関連を保存する。

同一operationに同時に複数の未解決試行を作れない。結果不明の間は別ID/別承認で迂回しない。人の『送れていないと思う』だけを技術的な未実行証拠にしない。提供元の確定未実行証拠も有効な冪等保証もない場合、通常の再試行は保留する。例外は残る二重実行リスクを示した別判断であり、今回採用しない。providerがkeyを無視/期限切れにした場合の正しさを保証しない。

取消/失効とattempt受理は同じoperation/関連policy epochを使って順序化する。予約だけなら新規試行を阻止できるが、attempt_started確定後は外部副作用の取消を保証しない。観測/停止は11の限定経路を用い、資格情報を全面取消したときは自動照会不能と明示する。

<a id="ch03-s12"></a>
### 承認版・提供元キーと復旧の追加条件

`approval_version_id`は固定snapshotに対する一つの承認decision版で、個別の票IDではない。再承認は新しいdecision版を作る。予約にunique(workspace, approval_version_id)を置き、同じ版の重複処理で開始資格を増やさない。

提供元keyは(provider, provider_account_ref, APIの冪等名前空間, key)の一意な対応表で一つのlogical operationへ結ぶ。実際の提供元が保証する範囲/保持期限をAdapter能力に保存し、scopeが広ければより厳しく衝突を拒否する。同じoperationの再承認/別attemptは同じ対応行を参照し、新keyで迂回しない。期限切れ/期限不明なら冪等保証を再試行根拠にせず、独立の確定未実行証拠がない限り保留する。

過去backupからの復旧では外部dispatchを全体で既定停止し、復旧点以後に生じた副作用の可能性を復旧DBとは独立した記録/提供元証拠で照合する。該当操作自体が復旧DBから消えている可能性もあるため、列挙/照合範囲が証明できなければ対象連携の停止を解除しない。古いcheckpointを未実行証拠にしない。具体的なDB耐久設定/照合記録/backup手順は実装前の阻害として残す。

reason-stop-confirmingは外部run行専用。reason-cancel-accepted(-pending)は受付の集約行、reason-cancel-after-startは個別送信行に限定する。開始受理とは内部の記録で物理送出の証拠ではなく、承認前説明にもこの定義を併記する。

<a id="ch03-s13"></a>
## 通信目的と根拠・開始資格・結果不明の追加契約

権限/固定snapshotの検査に加え、[12のP01〜08と実行点別判定](12-communication-secrecy.md)の目的、必要当事者、説明版、根拠版/epoch、source範囲、保持期限を検査する。国内AIへ渡す時、結果を公開する時、メモリへ保存する時にも再検査する。

開始資格はserver発行から内部開始受理まで5秒、operation/attempt/payloadへ一回限り。資格の発行と開始受理を区別し、受理と取消commitの順序を一意にする。上の手順6〜7は開始要求の検査と受理の耐久記録であり、資格の期限を外部HTTP完了まで延ばすものではない。5秒超過時は新しい内部開始受理をしない。引渡し済みの試行の未実行が証明できなければ結果不明とし、runnerの「呼出しなし」logだけで未送信にしない。

結果不明は発生直後に現権限内の判断待ちへ表示する。読取照合は0/5/15分の最大3回、照会不能は即有人確認、24時間未解決で会社の決定権者へ昇格する。タイムアウトは結果の確定ではなく、既存IDの再利用や新IDでの代替再送を許可しない。[13のX01〜06](13-operational-defaults.md)を適用する。

取消の要約{count}は、結果不明又は停止要求済みで結果未確認のoperationの重複しない件数。送信済みは含めず「送信済み{n}件」で別表示し、取り消し線や取消ボタンで撤回可能と見せない。上記4区分は個別送信行の主表示で、停止要求の事実と送信結果は別の記録を保つ。送信済みの証拠があればその結果を優先し、未送信は取消による未開始の確証がある場合だけ、停止要求済みのキーは実際に要求した場合だけ使う。停止APIがなく要求できなければ要求済みと表示せず結果不明等の確認できた状態を使う。
