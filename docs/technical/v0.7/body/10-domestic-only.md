<a id="ch10-s01"></a>
# 10 Pelway本体の国内限定と外部連携の責任分界

<a id="ch10-s02"></a>
## 最新要件と保証範囲

2026-11-08 22:25（日本時間）の修正を優先する。Pelway本体のAI処理・保存は国内限定を維持する。利用者が選ぶ外部サービス連携は別枠で、国外処理・保存の可能性を表示して明示有効化できる。国内未確認を理由にその連携を一律禁止しない。

「国内保存」は保存場所の条件。「国外送信なし」は推論・ツール・ログ・運用閲覧等の経路も含む別条件。外部連携を有効にした全システムについて後者を保証しない。本体の適合もまだ未検証であり、今回の成果は設計と評価計画に留まる。

国内設置hostは物理配置の前提であって確認済み事実ではない。運用者・サポート・再委託先による国外閲覧も本体の確認対象。IPやTokyoというendpoint名だけで国内完結と判定しない。

<a id="ch10-s03"></a>
## 場所・経路の台帳

| データ・処理 | 本体の保存・処理場所案 | 越境候補 | 初期制御/未保証 |
|---|---|---|---|
| chat/添付/成果物 | 国内hostのDB・同期外volume | cloud sync、外部preview、利用者選択連携へのexport | 自動preview/外部画像取得なし。exportは明示scopeと操作承認 |
| 本体LLMの入出力 | 国内hostのスタブ/ローカル推論 | API/fallback、model download、trace | 外部LLM fallbackなし。機材/品質/全通信未確認 |
| embedding/RAG | 必要時のみ国内modelとDB | embedding API、index telemetry | 初版の必須機能にしない。使う前に同じ審査 |
| Botメモリ/step/checkpoint | 国内DB/volume | remote storage、ツールによる持出し | 明示メモリは作成/根拠確認した更新から最長365日かつ全source期限内。閲覧で延長しない。checkpointは13 R07。sandboxへDB鍵を置かない |
| ツール実行 | 国内sandbox、gateway | MCP/API/CLI子processの通信 | sandboxはgateway以外拒否。stdioでも無通信と見なさない |
| client cache | 管理端末のSQLite/IndexedDB | 端末同期、国外端末閲覧 | 初期loopback利用。管理済み国内端末以外へ拡張は未決 |
| log/audit/crash | 国内DB/ローカルfile | OTLP、crash upload、support添付 | exporterなし。秘密除去。hash chainでも管理者耐性は未保証 |
| backup/復旧 | 国内管理記憶装置 | cloud backup、国外復旧担当 | 日次02:00 JST、復旧対象は作成から最大14日。世代の物理消去24時間は内部目標。独立制限記録を復旧前に再適用。device/運用者/鍵/容量/復旧は未確認、RPO/RTO保証なし |
| font/emoji/画像 | 同梱/国内server | CDN/外部font/remote media | 同梱権利と無通信を確認 |
| update/依存/model取得 | 構築専用経路から国内へ事前搬入 | registry、build script、model code、診断 | 機密実行と分離、固定hash、取得tokenとproject名漏えいも棚卸し |
| DNS/NTP/TLS検証/OS/driver/IME | 国内管理端末の機能 | resolver、OCSP、telemetry、cloud変換、翻訳 | 全host/VMを棚卸し。アプリ設定だけで遮断済みとしない |
| 選択済み外部連携 | 提供元の処理/保存/運用先。国外あり得る | export payload、検索語、成果物、外部model | 別責任分界。送信先・data・国外可能性を表示、初期off、明示有効化とscope/承認/監査 |
| 外部メール | 初期はdummyのみ | 宛先、配送経路、受信先 | 利用者選択の外部連携の例外をmail全般へ適用してよいか未決。自動で例外化しない |

画面撮影、コピー、clipboard同期、download後の持出しは製品だけで完全防止できない。これを要件から除外した許可が出たとはしない。UI案は添付downloadを初期無効、コピーは保持し未保証表示。導入端末の管理条件・可否は判断待ちで、本体国内適合を宣言しない。国外端末アクセスの責任分界は外部連携許可と別の未決事項。

<a id="ch10-s04"></a>
## 通信policyを二つに分ける

`core_domestic_policy` は本体のDB、推論、保管、診断、backupに適用する。初期はloopbackと内部networkだけ。外向き通信は既定拒否。本体の推論を外部Adapterへ逃がすfallbackは禁止。

`integration_grant` は会社・連携・アカウント・操作・data分類・宛先・有効化版ごとの許可。初期は空。integration.enableの対象scope権限を持つ利用者または管理者は国外性が未確認でも、説明を読み、通信当事者を覆う必要な利用/提供根拠も別確認して設定上の有効化を行える。実I/Oは無効のまま。一般のBotはgrantを作れない。外部連携が増えても本体network全体のdenyを解除しない。

gatewayは両policyを通して要求の分類と権限を確認する。Botが本体の秘密・会話全体を『外部連携の入力』へ分類し直して送れないように、taskごとのexport対象を固定する。readツールでも検索語や文書IDが外へ出るので、読み取りを無条件の無送信と見なさない。

ホスト・VM・containerのfirewallと限定proxyで強制する。直接IP、IPv6、DNS、proxy環境変数、redirect、別hostname、retryも同じ判定を通す。API path/操作/dataの制限はhostname/IP許可だけでは実現できないためapplication gatewayでも確認する。

外部連携停止はgrant epochを増やし、broker tokenと新dispatchを拒否する。既に外部へ出たデータや実行を回収できるとは言わない。外部停止と提供元での保持は11で追跡する。

<a id="ch10-s05"></a>
## 構築時と実行時

依存・OCI image・model・fontは機密データのない取得環境で検査し、固定hashとlicenseを付け国内の実行環境へ搬入する。構築時も国外への通信が『何も送らない』わけではない。IP、依存名、取得token等を記録し、機密データを載せない。

実行中の自動download、更新確認、crash export、link previewは無効。外部連携を明示有効化してもこれらを一括でオンにしない。localhostのGUIがhostのcloud機能から送信し得る点は管理環境で確認する。

<a id="ch10-s06"></a>
## 試験条件案

全試験は合成データで行い、適合確認前に実データを入れない。P1b合格と別の利用許可後まで実顧客データは扱わない。

| ID | 操作 | 合格条件案 |
|---|---|---|
| D1 | core-onlyで起動・依頼・保存・再接続しhost/VM/containerを記録 | 本体の許可外送信0。拒否した接続試行も別記録し、成功通信0と混同しない |
| D2 | ローカル推論を停止 | stopped理由表示。外部LLM fallback0 |
| D3 | dispatch中のtimeout/retry/runner crash | 許可外送信0、結果不明の自動再送0 |
| D4 | 直接IP/IPv6/redirect/DNS変更/proxy経由で逸脱を試す | grant外への到達0。redirect先も検査。診断に秘密がない |
| D5 | UI/runner/libraryを故意にcrash | 自動外部report0、国内にのみ必要最小の記録 |
| D6 | 完全offline起動と通信遮断 | font/emoji表示、UI応答、ローカル保存が成立。無限retryしない |
| D7 | image/model/update/driver/OS/IME/clipboard/backupを棚卸し | 未確認経路を残して適合とはしない。試験範囲と残リスクを明記 |
| D8 | 未設定mailの操作 | dummy以外へdispatch0。個別例外の許可がないことを確認 |
| D9 | 国内未確認の模擬外部連携を明示有効化 | 説明・社内有効化権限・通信当事者を覆う利用/提供根拠が揃えば設定上の有効化可。実I/O許可ではない。scope内承認済みpayloadだけ送出し本体policyは変わらない。一覧/主体に「国内限定の外」、本文に「外部由来・未確認」、本体に適合/国内完結の保証表示がない |
| D10 | 有効化を行わず同じ連携を呼ぶ/別accountへ変更 | dispatch拒否。国外性だけでなくgrant不足を理由表示 |
| D11 | 外部連携有効化後に失効/取消 | 新dispatch拒否。外部停止未確認を停止済みと見せない |
| D12 | 外部serviceへ本体LLM fallbackや全DB exportを要求 | 利用者選択の例外を転用せず拒否 |

通信観測一回で『国外へ1byteも出ない』の永続保証にはしない。対象版、固定設定、OS、driver、ネットワーク、観測点、未観測部分を結果に添える。Apple containerで内部専用networkを作れるかは実装前の阻害。遮断を構成できなければ機密実行を始めない。

<a id="ch10-s07"></a>
## 将来の地域への境界だけ残す

`PlacementPolicy`を保存先・AI処理先・log/backup先・送信方針の共通参照にし、日本という条件を各Adapterへ散らさない。本体の`core_domestic_policy`と外部の`integration_grant`は同じ配置方針へ関連付くが別の判定。現在はJP-local単一のみを構成する。US/EU/JPは長期の選択肢例示で、今回のnetwork/法務/設定UI/移行設計を増やさない。リージョン選択はデータ所在の制御であって法適合の保証ではない。既存data/backup移動、契約、アクセス管理等は将来要検討で、自動/自由移行は保証しない。

D4では、task sandboxおよび将来のCLI連携区画からhostの推論port/DB/runtime APIへ到達できないことを追加確認する。D11は取消/失効をdispatch予約前・予約後attempt_started前・attempt_started直後・応答保存前に注入し、開始前でも未送信を確定できた場合だけ未送信/拒否とし、開始受理の永続commit/応答が不明なら結果不明とする。開始後の不確実な結果も保持し自動再送しないことを記録する。開始記録より後の取消で実際に停止要求を出した個別送信行には03の `reason-cancel-after-start` の日英文言を表示し（要求していなければ結果不明又は証拠のある送信済みを使う）、「取り消しました」とは表示しないこともD11で確認する。受付直後にreason-cancel-accepted、未確定の開始受理済みがあればreason-cancel-accepted-pendingの件数が実記録と一致することも確認する。上記は期待値であり未実施。


<a id="ch10-s08"></a>
### 実送出の有効化と設定上の同意の区別

本章の明示有効化は将来の設定/権限条件で、実接続の着手許可や安全試験の合格ではない。[08のS1](08-decisions-open.md)は未解決で、実外部I/Oは無効のまま。国内性が未確認だから禁止するのではなく、P方針は明示的要件変更として暫定採用済みだが、送出制御・根拠失効・障害/復旧等の検証と実行の別承認が未達のためである。模擬経路の期待値は実接続試験の実施を意味しない。

<a id="ch10-s09"></a>
## 連携設定・個別承認・法的根拠の三層

会社の有効化設定、権限者の個別snapshot承認、通信当事者の利用/提供根拠を別々に記録し、すべてが必要範囲を満たす場合だけ処理を許可する。[12](12-communication-secrecy.md)の目的・提供先台帳と実行点検査を適用する。国外又は国未特定だけを一律禁止の理由にしないが、受領者/目的/取扱条件が白紙なら有効化しない。外国未特定時のAPPI代替説明は通信秘密の同意範囲や受領者確認を省略する根拠ではない。

接続時の認証・能力確認に必要なアカウント識別子/IP等を説明する。接続だけで会話/添付/repo/logを同期・索引化・送出しない。業務検索語/文書ID/promptも提供データとして検査する。通常取消のP非保証と法的根拠失効を分け、停止/照会に許された限定権限だけを使う。全面失効を別account/再認証で迂回しない。
