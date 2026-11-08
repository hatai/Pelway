<a id="ch04-s01"></a>
# 04 データモデル・API・同期の契約案

この章は初版の参照仕様。実装コードや確定したDBスキーマではない。

<a id="ch04-s02"></a>
## 永続データ

会社の主キーは `workspace_id`。各tenant表はこれを持ち、複合外部キーで他社参照を拒否する。`workspaces`自体は `id = workspace_id` が根。Botの会社IDをLLMの文字列から決めない。会社・チャンネル閲覧権限は取得・検索・添付・イベント・監査参照すべてに適用する。

| 表 | 主な項目と制約 |
|---|---|
| actors / members | actor ID、kind(human/bot/external_agent/external_tool)、所有者、会社。チャンネル参加はmembers側。外部主体は単独で権限を持たず委任元を参照 |
| bot_profiles | member、役割、許可policy、国内メモリ参照（source/根拠/期限付き、R02の最長365日以内）、利用上限 |
| channels / channel_members | channel、参加者、役割、次のmessage seq |
| messages | channel、seq、author、client_msg_id、本文版、thread_root。unique(workspace,channel,seq)、unique(workspace,author,client_msg_id) |
| tasks | source message、親task、担当、9状態、version、判断者・論点、stop_reason、cancel_accepted_at（受付commit）、外部停止証拠 |
| runs / sessions / task_steps | task、attempt、外部session参照、executor、lease、fence、checkpoint、step結果。sessionは認証cookieや秘密ではない |
| artifacts / attachments | task/run/message、内容hash、版、国内storage key、MIME、size、信頼区分、検査結果、原提供元ref |
| decisions / handoffs | 不変の案版、判断者・時刻、引継ぎの5項目と参照 |
| action_snapshots / approvals | action_kind・操作・宛先・本文・添付hash・account・scope・policy版、承認decision集合、個別の承認票、承認者・期限・状態。snapshotは不変、票は集合へ多対1 |
| external_actions | logical operation、固定payload、action_kind、提供元key対応行。unique(workspace,logical_operation_id)。承認/試行履歴は別表とし、同一operationへ関連付ける |
| dispatch_attempts | attempt ID、operation、approval_version_id、予約、attempt_started_at、検査時epoch/version/fence/hash、拒否/結果、開始資格を得たgateway実行ID。unique(workspace,approval_version_id)、同一operationの未解決試行は最大1。同じattempt_startedから資格を再発行しない |
| provider_operation_keys | unique(provider,provider_account_ref,API冪等名前空間,key)、logical operationへの不変対応、保証範囲/期限。再承認や別attemptは同じ行を参照 |
| integrations / grants | provider、owner/admin、有効化版、scope、送信data分類、送信先、国外可能性、社内有効化説明版と通信当事者の根拠refを分離、credential ref、失効epoch |
| inbox / outbox / sync_events | unique(workspace,integration,external_account,source_event_id)、payload hash、stream_pos、subject version、未処理/処理済み。取込と業務適用を分離 |
| audit_events | actor、subject、操作、結果、snapshot hash、policy版、日時、prev_hash。秘密・prompt全文を既定で含めない |

同一 `client_msg_id` の再送で本文が違えば409を返し、異なる本文で既存成功を偽装しない。添付storage keyは利用者指定パスをそのまま使わず、path traversalを拒否する。編集・削除・権限取消は本文追加と別のversion付きイベント。

<a id="ch04-s03"></a>
## API

入口はHTTP JSON、配信はWebSocket。全操作で認証・会社/チャンネル権限・versionと、12の目的・source・必要当事者を覆う根拠版/epoch・保持/出力範囲を確認する。localhostでもブラウザOriginとCSRF、WebSocket認証を検査する。正式なOpenAPI/JSON Schema生成は後の実装段階。

| method/path案 | 入力/動作 |
|---|---|
| POST /v1/channels/{id}/messages | client_msg_idと本文、添付版。重複でも同じ結果 |
| GET /v1/channels/{id}/messages | before_seq/after_seqとlimit。権限の範囲だけ |
| POST /v1/tasks/{id}/handoffs | expected_version、新担当、引継ぎ参照。未決判断者の付替え |
| POST /v1/tasks/{id}/resume | checkpoint ref、expected_version。外部成功操作は再dispatchしない |
| POST /v1/tasks/{id}/cancel | 取消受付をcommitし以後の新規内部開始受理を拒否。受付の事実と既受理で未確定の件数を返す。停止確認/未送信の確定と同義ではない |
| POST /v1/decisions/{id}/confirm | server提示の不変案版/選択肢ID、expected_version。権限ある人の明示UI確認だけで判断確定。チャット文は候補まで |
| POST /v1/approvals/{id}/decide | snapshot hash、expected_version、決定。期限・actor再検査、競合409 |
| GET /v1/actions/{id}/evidence | 国内に保存済みの証拠だけ返す。新たな外部通信を起こさない |
| POST /v1/actions/{id}/refresh-evidence | 外部照会は別の明示要求とgrant/送出範囲の検査へ。通常grant取消後は自動呼出しない |
| POST /v1/actions/{id}/resolve | 人の送れていた/いない判断と証拠ref。再送は新承認へ |
| POST /v1/integrations/{id}/enable | scope、送信先・データ分類、国外可能性の説明版と明示同意、対象scopeのintegration.enable権限（委譲された利用者または管理者） |
| POST /v1/integrations/{id}/revoke | epoch増加、broker拒否、未dispatchを停止、外部停止確認を追跡 |
| POST /v1/integrations/{id}/events | 将来の受信口。署名・再送検証とinbox永続化のみでACK。初期ローカルは公開しない |
| GET /v1/sync?since= | 権限付き差分またはresync_requiredとsnapshot watermark |

オフライン中の承認・外部dispatch・権限変更をキューへ積まない。通常メッセージの未送信draftのみ送信キューへ保存し、再接続後に自動送信しない案とし、各メッセージの未送信表示と「送る／破棄」を出す。一括送信を入れるなら件数/宛先channelを確認する。UI提案であり正式決定は08の未決。

<a id="ch04-s04"></a>
## 順序と再接続

messageの `seq` はチャンネル内の発言順。message編集や権限変更には独立の `event_id` と `subject_version` を持つ。両者を混同しない。会社内の `stream_pos` は差分取得用の不透明cursor。DBトランザクションのcommit順で公開する。採番sequenceだけではcommit順が揃わないため、初版はworkspaceごとの行ロックをcommitまで保持して順序化する案。負荷は07で測る。

状態更新・監査・outboxを同一transactionへ書く。配信後にcursorを保存し、クライアントは適用済みevent IDとversionで重複排除する。取得済み最大cursorより前にcommitしたイベントを落とさない。通知のACKは実行・承認の成功ではない。

再接続時は保存cursorから差分を取得する。cursor期限切れなら権限付きsnapshotとwatermarkを一緒に取得し、その後の差分へ切り替える。全subject・権限・watermarkを同じPostgreSQLのREPEATABLE READ読取snapshotから作る。読取途中のcommitでwatermarkだけ先へ進めない。初期の小規模試験は1回で整合したsnapshotを返す案。分割が必要ならimmutableなsnapshot manifest/取得点を固定し、ページごとに新たな最新状態へ混ぜない。転送時の現権限も再検査し、取得中に閲覧を失ったscopeは返さず再同期を要求する。チャンネル履歴だけの取り直しでは、編集・削除・タスク・権限変更が戻らないため、全subjectのsnapshotを対象にする。権限取消で見えなくなったchannelのcacheを消すtombstoneも反映する。

欠番と古いsubject versionは再取得・照合する。指数backoffの上限30秒は設計仮定であり性能保証ではない。外部イベントに順序番号がない場合、受信時刻だけで因果順を決めずproviderの状態を照会する。重複・並べ替えで現在状態を後退させない。

Matrixの増分syncと送信transaction ID、Zulipのevent queueを参考にする。仕様と適用差は [調査原稿](../sources/v0.6/fixed/pelway-technical-docs/research/existing-chat-and-agent-oss.md) を参照。外部副作用のexactly-once保証ではない。

<a id="ch04-s05"></a>
## キャッシュと削除

ネイティブはSQLite、WebはIndexedDBに閲覧範囲の限定cacheを置く。正本ではなく、承認はオンラインserverだけで確定する。端末の暗号化・同期設定は別確認。logout・会社離脱・権限失効の削除と、オフライン残存期限を設計し、未確認の端末で機密cacheを有効にしない。機密cacheは最終オンライン確認から24時間、端末/workspaceごと256MiBまで。失効/離脱/logoutは先に制限する。保持・利用停止・世代削除/復旧は[13のR14/R15/R16とD01〜05](13-operational-defaults.md)に従う。

<a id="ch04-s06"></a>
## 配置/データ所在の最小境界

`workspaces.placement_policy_ref`で同一の配置方針版を参照する案。今の有効値はJP-local単一だけ。方針には保存、AI処理、log、backupの配置先と`core_domestic_policy`、選択外部連携のgrant判定への参照を置く。送信先が国外であり得る外部連携を、この値で国内と再分類しない。設定は構成ファイル上の設計例で、地域選択UIや動的切替APIを今作る意味ではない。

将来US/EU/JPへ広げる場合、同じ境界を実装できる余地だけ残す。region変更時の既存data・backup移行、契約、運用access、停止時間は未検討。自由切替・自動移行・法適合を保証しない。

workspace行lockがボトルネックになった場合は、commit済みoutboxを永続dispatcherが採番して配る方式を将来の代替として比較する。dispatcher再起動/重複/原transaction順の保持を別検証し、今の設計へ無断置換しない。

<a id="ch04-s07"></a>
## 同時更新と成果物の耐久化

lock順序はworkspace順序行→subject（種類/ID順）→承認/operation行の一定順とし、全更新APIとinbox適用で共有する。lock中に外部I/O・ファイル取込を行わず、transactionを短く保つ。監査のprev_hashはworkspace内の鎖で、全社共通lockを作らない。管理者の改ざん耐性を保証するものではない。

ファイルはtask専用の一時領域へ保存→size/MIME/内容hash検査→許可された国内volume上の不変keyへ原子的renameとファイル/親directoryの耐久化→DBのartifact参照/checkpointを確定する。DBとfilesystemの原子commitは主張しない。失敗で残る孤立ファイルは参照/保持を照合して後から回収し、参照確定前の一時ファイルを成功artifactとして出さない。

復旧時にDB参照とhash/ファイルの存在を照合する。欠損があればrun再開を保留し、外部副作用済みのstepを自動再実行しない。DBとvolumeのbackupは同じ整合取得点/参照manifestを持つ手順が必要で、方式は未決。別時点のbackupを組み合わせて復旧成功としない。

<a id="ch04-s08"></a>
## 外部イベントの帰属

submit時にworkspace・integration・account・提供元job/run/session・内部task/runの対応を保存する。署名は送信元の確認で、task所有権の証明ではない。外部payloadのtask_id/run_idは照合用に限り、業務適用先は保存した対応から解決する。未登録/不一致/別channelのイベントは隔離し、状態/成果物を適用しない。署名が正しくてもこの検査を省かない。

重複keyに同じhashなら既存ACKを返す。同じkeyに異なるpayload hashなら不整合として隔離し、通常の重複として捨てない。

<a id="ch04-s09"></a>
## 操作種類の正規値

ActionSnapshotの`action_kind`は`customer_send`（顧客送信）、`agent_handoff`（外部への受け渡し）、`tool_read`（ツール読み取り）、`tool_write`（ツール更新）の4値案に統一する。翻訳キーはそれぞれ`action-customer-send` / `action-agent-handoff` / `action-tool-read` / `action-tool-write`。06/11の文字表示と成功後の遷移はこの値から決め、提供元の文章をenumに昇格しない。

<a id="ch04-s10"></a>
## 外部I/Oの開始受理と復旧

03の現権限/承認/epoch/fenceの最終検査と予約→attempt_startedへの条件付き更新を同一transactionで行う。開始資格は予約済みoperation/attempt/payloadとgateway実行へ一回発行し、発行から5秒以内の開始受理で一回消費する。成功の耐久commitを確認した当該gatewayだけが外部I/Oへ進める。重複要求や再起動で資格を取り直せない。失効/取消更新も同じ順序で競合させる。永続commitと受理応答を確認できないgatewayは外部を呼ばずoutcome_unknownとして照合へ回す。同期commit/ストレージ耐久の設定と障害試験は着手前に確認し、未確認の構成では外部I/Oを始めない。

復旧DBはbackup取得後の外部成功を知らない可能性がある。復旧時はdispatchを既定停止し、復旧点以後の操作を独立記録/提供元証拠で照合する。範囲を列挙できない/照合できない連携は止めたままにする。DB/volumeのhash整合だけでは再送安全としない。

03のP方針はJ-S07で明示的変更として暫定採用済み。取消commit後の新しい内部開始受理を禁止し、受理済みの物理送出前競合を含む停止非保証を説明する。旧S1を解消したとはしない。方式・根拠失効・障害/復旧の検証と別承認前は実外部I/Oを無効のままにする。

<a id="ch04-s11"></a>
## 目的別根拠・派生物・容量予約のデータ契約

[12 §3.2](12-communication-secrecy.md)のcommunication_use_policies、communication_basis_records、context_authorizations、派生物のsource/ACL/目的/根拠/期限、support_access_grant、削除・制限記録を既存workspace境界へ追加する。名称は論理単位で確定DDLではない。根拠未確定の機能は無効とし、管理者設定を同意の代用にしない。

削除の有効受付と制限確定を同じtransactionで行い、sourceの参照先全件の物理削除完了を待たず読取/検索/再利用/公開を拒否する。復旧DBから独立した制限記録を再適用するまでは再開しない。

保存は使用量と同時処理の予約量を合わせてquota検査する。10GiBには本文/添付/成果物/メモリ/索引を含め、作業・診断/安全監視/監査/同意・backup・端末cacheは[13のC01〜08](13-operational-defaults.md)へ分類する。分類不明の永続保存は禁止。法定記録を上限到達で捨てず、新しい業務を止め安全記録のreserveを守る。
