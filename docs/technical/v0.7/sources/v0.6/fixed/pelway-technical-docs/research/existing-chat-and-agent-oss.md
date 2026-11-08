# Pelway：既存チャットとAIエージェントOSSの設計事例

確認日：2026-11-08（日本時間）。公開一次資料による設計調査。実装、環境構築、接続、購入、実機試験は行っていない。
本文と表の全項目の確認日は上記。リリース日は公開資料で確認できた日付を記載し、時刻・タイムゾーンが不明な一覧は公開日の表記を維持した。
[事実] は資料に記載、[設計提案] はPelwayへの提案、[未検証] は未確認または実測なしを表す。「未確認」は機能がないという意味ではない。
「参考」は設計を読むだけ、「依存採用候補」は条件確認後の組込み候補、「改変・流用候補」はコードの複製を含む候補。採用決定・実行許可ではない。

## 要点と推奨

- [設計提案] チャットはMatrixの送信トランザクションIDと増分同期、Zulipのイベントキューとローカル表示照合を参考にする。フェデレーション一式の採用を既定にしない。[M1][Z1]
- [設計提案] RustのAI層は公式MCP Rust SDKを条件付き第一候補、rigをLLM呼出し部分の条件付き候補とする。業務状態・送信承認・監査の主体はPelwayに残す。[A3][A9]
- [設計提案] LangGraphのチェックポイントと中断再開、OpenHandsの実行環境分離、Gooseのセッションとツール許可を参考にする。ランタイムをまとめて流用しない。[A1][A2][A8]
- [設計提案] Botの人格・記憶と実行環境を分ける。承認待ちでは実行環境を停止できる構成を比較し、Botごと常時VMを要求しない。
- [設計提案] 国内限定要件の初期案は、日本国内の管理端末・保存先と閉じたローカル推論。本体の外部LLM・埋め込み・検索・通知中継は未承認のまま無効にする。利用者が選ぶ外部サービス連携は後述の別枠。[D1][D2][D3][D4]
- [未検証] OSSの公式対応表明は、3OSの日本語IME、長文操作、NVDA・VoiceOver・Orcaの実機合格を意味しない。国内限定も通信遮断・観測試験なしでは保証しない。
- [未検証] Discordの「ElectronからRust、1.3GB→300MB」は一次資料で確認できない。選定理由にもRust・非WebViewという条件の根拠にも使わない。[D0]

## 既存チャットから学ぶ設計パターン

| 事例 | [事実] データ・配信・復旧の仕組みと出典 | [設計提案] 多人数・複数Bot協業への適用 |
|---|---|---|
| Matrix仕様 | roomのイベントは `event_id`、`sender`、`type`、`content` を持つ。`m.thread` 関係でスレッド、read markerとreceiptで既読を表す。HTTPの `/sync` の `next_batch` を次の `since` に渡す。limited timelineの欠落は履歴ページ取得で補う。[M1] | 会話IDとタスクIDを分け、受信イベントの重複排除と同期カーソルを永続化する。既読を業務完了にしない。 |
| Matrix送信・検索・ファイル | `PUT .../send/{eventType}/{txnId}` は再送を識別し同じ `event_id` を返す。スコープは端末とエンドポイント。local echoとの照合、検索API、media repositoryを仕様化。[M1] | 送信中の仮表示と確定イベントを照合する。ただしチャット内冪等性をメール送信など外部副作用の保証に流用しない。 |
| Matrix application service | 名前空間内の仮想ユーザーとして動作。homeserverは不変のイベント束を `txnId` 付きで配送し、同じIDで再試行。ASは送信済みイベントを阻止・変更できない。[M2] | 複数Botの主体分離と受信inboxを参考にする。ASを外部送信の承認ゲートにしない。 |
| Synapse | Python/TwistedとRustのhomeserver。[M3] | ローカル対照環境候補だが、承認・実行を別サービス化する設計の参考に限定。RustネイティブUIの根拠にはしない。 |
| Conduit・conduwuit系 | ConduitはRust homeserver。公式変更履歴にthreads、media、appservice、sliding syncがある。conduwuitの継続先としてContinuwuityが公表されている。[M4][M5] | 小さいローカルhomeserverの比較対象。名前が似た派生を同一保守系統・同一互換性と扱わない。 |
| Element・matrix-rust-sdk | Element XはRust SDKをFFI経由で共有し、iOS側はSwiftUI。SDKと画面は別。[R1][R3] | 同期・キャッシュとUIの分離が参考。3OS共通Rust UIが実証された事例ではない。 |
| Zulip | channelとtopic、message IDで整理。Django、PostgreSQL、RabbitMQ、Tornadoを使い、長い接続を処理。[Z0] `queue_id` と `last_event_id` でイベントを取得。[Z1] | タスクごとtopicまたはthreadを持たせ、複数Botの脱線を防ぐ。既存の構成規模をそのまま持ち込まない。 |
| Zulip送信・検索・ファイル | 送信時の `local_id` と `queue_id` がイベントの `local_message_id` に対応。これはlocal echo照合であり外部送信のexactly-once保証ではない。narrowはtopic・送信者・検索語で絞る。upload APIもある。[Z2][Z3][Z4] | 楽観表示、権限付き履歴検索、添付を分ける。キュー失効時は再登録と履歴再取得が必要になる設計を参考にする。[Z1] |
| Mattermost | `Post` は `RootId`、`FileIds`、`PendingPostId` を持つ。旧実装はpending IDをTTL付きキャッシュで保存済みpost IDへ対応づける。[C1][C1old] botはUsersとBotsに行を持ち、チャンネル参加と権限を持つ。[C2] | 人とBotを共通主体にしつつBOT表示・所有者を分離。旧TTLキャッシュを長期間の外部副作用台帳にしない。 |
| Mattermostリアルタイム | プラグインはWebSocketイベントを発行できる。[C3] WebSocket切断後の完全な差分復旧契約・現行のpending ID保証範囲は未確認。 | WebSocketを永続ログの代わりにしない。カーソル期限切れ時のsnapshot取得を別途定義する。 |
| Rocket.Chat | Realtime APIはDDPとroom message streamを公開。DDP method callsは非推奨扱いの記載がある。unreadはsubscription操作APIで変更。[C4] | 参考に留め、新規APIをDDPへ固定しない。イベント継続・履歴照合の現行保証は未確認。 |
| Slack | Events APIはHTTPまたはSocket Modeで配信。`event_id` があり再試行ヘッダーを付ける。受信ACKと処理キューの分離を推奨。[C5] | 受信イベントは重複し得る前提でinboxを設ける。Bot自身の投稿や他Botからの無制限応答を止めるルールはPelway側に置く。 |
| Discord | GatewayはWebSocket、heartbeat ACK、sequence、session IDでResumeし欠落イベントを再生。Resume不可なら再Identifyする。[C6] | 接続状態と業務状態を分ける。再接続中でも「結果不明」の外部操作を新規送信しない。 |

[未検証] Slack・Discordの内部検索、ファイル保存、全既読モデルは今回の取得資料だけでは比較確定しない。公開Bot APIと内部製品実装を同一視しない。
[設計提案] Pelwayの検索はメッセージ閲覧権限を検索結果にも適用し、添付は本文と別ID・別保存領域にする。wiki・外向けQ&A自動生成は対象外。

## Rustクライアントと入力・表示・アクセシビリティ

| 事例 | [事実] UI・公式課題 | [未検証] 断定しない点 / [設計提案] 協業への適用 |
|---|---|---|
| Fractal | Rust・GTK系Matrixクライアント。[R0] 公式work itemsにスクリーンリーダーのメッセージ改善課題が検索できるが、特定issue番号は取得できなかった。 | 日本語IME、候補窓、確定Enter、複数メッセージ選択、仮想化方式、Orca実機合格は未確認。GTK採用だけで3OS対応を結論しない。Bot発言の読み上げ順の参考。 |
| iamb | Rustの端末クライアントでmodalkit-ratatui系。Unicode列幅の修正がある。[R2] [#622](https://github.com/ulyssa/iamb/issues/622) はメッセージ移動時のシステムカーソル位置改善課題。[#618](https://github.com/ulyssa/iamb/issues/618) は文字幅の不具合報告。 | 端末IMEとGUIのインラインpreeditは同じではない。AccessKit、3スクリーンリーダー、メッセージ跨ぎコピーは未確認。協業ではキーボード操作の参考に限定。 |
| Element X・Rust SDK | SwiftUIとFFI共有。[R3] iOS [#2232](https://github.com/element-hq/element-x-ios/issues/2232) はVoiceOver rotorの操作数とtimeline描画負担のトレードオフ。音声メッセージ読み上げ [#2568](https://github.com/element-hq/element-x-ios/issues/2568) はclosed。[R4] | iOSの結果をWindows・Linuxへ外挿しない。SDK自体のAccessKit対応は未確認。Bot名・状態・操作をまとめて読める設計を検討。 |
| SDKの大量履歴 | timelineのvirtual itemと描画の仮想スクロールは別。SDK [#4821](https://github.com/matrix-org/matrix-rust-sdk/issues/4821) は先頭仮想item、[#5350](https://github.com/matrix-org/matrix-rust-sdk/issues/5350) は全文検索と過去履歴索引のメモリ課題。 | 「virtual itemがあるから大量履歴が軽い」と判断しない。各Botの長文を混ぜた測定が必要。 |
| Element Web | IME問題の公式報告 [#7665](https://github.com/element-hq/element-web/issues/7665)。現行版での修正範囲・日本語実機成否は未確認。 | WebとRust UIで同じ変換確定・送信判定テストを行う。Elementの外観・UIコードを採用する提案ではない。 |
| Zulip | composition中のEnter・矢印を無視すべき課題 [#9396](https://github.com/zulip/zulip/issues/9396)、topic編集とIME [#23595](https://github.com/zulip/zulip/issues/23595)。公式helpに[高コントラスト](https://zulip.com/help/high-contrast-mode)と[キーボード操作](https://zulip.com/help/keyboard-shortcuts)。 | 課題の現行解消とNVDA・VoiceOver・Orca実機は未確認。協業ではtopic編集・承認ダイアログにもIME試験を広げる。 |
| Mattermost | 日本語IMEの確定Enterで新規channelモーダルが誤動作する [#23967](https://github.com/mattermost/mattermost/issues/23967)。公式に[キーボード・読み上げ](https://docs.mattermost.com/end-user-guide/collaborate/keyboard-accessibility)と[適合方針](https://docs.mattermost.com/product-overview/accessibility-compliance-policy)を公開。 | 方針・修正履歴とPelwayの実機合格は別。送信承認画面のフォーカス復帰・本文読み上げを参考にする。 |
| AccessKit・egui・Iced | [AccessKit](https://github.com/accesskit/accesskit) はWindows UIA、macOS、Unix AT-SPI向けadapter。egui READMEにAccessKit統合の記載。[U2] IcedのIME [#2777](https://github.com/iced-rs/iced/pull/2777) とLinux候補窓位置 [#3258](https://github.com/iced-rs/iced/issues/3258) がある。 | toolkitの統合は読み上げ品質保証ではない。Icedを含むUI最終比較はUI担当へ。各Bot発言・承認・結果不明の読み上げは未試験。 |

[設計提案] 共通受入項目は、変換中文字のインライン表示、候補窓のキャレット追随、変換確定Enterで送らない、長文編集、履歴中間への挿入・削除、メッセージ跨ぎ選択コピー。
[設計提案] 仮想スクロールで画面外の選択・読み上げ対象を破棄しないことも試す。NVDA・VoiceOver・Orcaを別々に記録し、UIA・AT-SPIの対応宣言だけで合格にしない。

## DiscordのRust事例の確認結果

- [事実] [GoからRustの記事](https://discord.com/blog/why-discord-is-switching-from-go-to-rust) の対象は既読を扱うサーバー側Read Statesサービス。クライアントのElectron置換ではない。[D0]
- [事実] [2025年3月5日の公式記事](https://discord.com/blog/supercharging-discord-mobile-our-journey-to-a-faster-app) はdesktopをReact、mobileをReact Nativeと説明。Rust移行はcore storesと通信ロジックの取り組みとして述べる。[D0]
- [事実] 同記事のメモリ改善例はサーバーリストの仮想化・native chat listなどの個別改善。記事は「Electron全廃で1.3GB→300MB」と報告していない。[D0]
- [未検証] 指定された対象製品・移行範囲・測定条件・1.3GBと300MBの一次資料は確認できない。「移行が存在しない」とは断定しない。二次情報は根拠に採用しない。
- [設計提案] この調査結果は選定理由にもRust非WebView条件の根拠にも使わない。ユーザーのRust志向は維持し、Pelway自身の性能測定で判断する。

## AIエージェントOSSと隔離基盤の比較

各行の最後は区分。外向き通信・国内限定の条件は後述の棚卸し表も満たす必要がある。チェックポイントと外部副作用のexactly-onceは別。

| OSS | [事実] 再開・承認・隔離 / [未検証] 不足 | [設計提案] ローカル・複数Botへの適用と区分 |
|---|---|---|
| OpenHands | Docker runtimeはbackendからRESTでコンテナ内ActionExecutorを操作。[A1] SDKのrisk analyzerに確認判定。[A1s] 外部送信本文固定・結果照会の一体保証は未確認。 | 実行器と会話を分離する参考。セッション・作業領域の境界を読む。全製品の流用はしない。 |
| LangGraph | checkpointer、thread ID、`interrupt`、`Command(resume=...)`。再開時はnode先頭から再実行。副作用を冪等にする必要がある。[A2] OS隔離の保証は未確認。 | durable taskと中断再開の参考。Pythonランタイム導入は初版では推奨せず、Rust側の永続状態遷移へ適用。 |
| MCP・公式Rust SDK | tokioベースの公式SDK。[A3] protocolにはtool、consent、cancellation、task拡張がある。取消は協調的で副作用の巻戻し保証ではない。[A3s] | 依存採用候補。条件は固定版・ライセンス範囲、外向き通信、ツールごとの接続先・権限・取消を確認すること。MCPはsandboxではない。 |
| AutoGen | Microsoft公式はarchive・新規利用はAgent Frameworkへ移行を案内。[A4] | 複数agent会話・handoffの参考のみ。保守停止の依存は採用しない。 |
| Microsoft Agent Framework | workflowはRequestInfoEvent等で人への質問を中断し、checkpointのpending requestを復元できる。handoffも公開。[A5] | 複数Botの担当受渡しの参考。human-in-loopを送信承認と同一視せず、承認対象と役割をPelway側で固定する。 |
| CrewAI | `human_input` は回答後の確認でありtool実行前承認ではない。[A6] Flowにはhuman feedbackと状態保存。新しいdocsで旧Docker safe実行の非推奨が記載される。[A6s] | 作業分担の参考。旧「safe」名を隔離保証にしない。クラウドsandboxを今回の解決策に戻さない。 |
| Letta | 旧server v0.11.7でtool approval追加。[A7] #3206は未回答承認でブロックする報告、staleによるclosedで解決確認ではない。[A7i] | 承認待ちと会話受付の分離を参考にする。旧serverと現在のLetta Code docsを混ぜて依存採用しない。 |
| Goose | Rust agent。権限モード・セッション管理を公開。取得した旧権限docsでは自律モードがdefaultで、read/write分類はbest effort。[A8] 物理隔離・外部送信冪等保証は未確認。 | Rustのセッション・拡張呼出しを参考。改変・流用候補はcoreの小範囲のみ。版・通信・依存ライセンス確認後に再判断。 |
| rig | RustのLLMアプリ基盤。ToolContextやtool呼出しの実装が公開。[A9] durable checkpoint、送信承認、OS隔離の一体保証は未確認。 | 依存採用候補。LLM adapterに限定し、国内接続先固定とtelemetry監査を条件にする。業務state machineを任せない。 |
| gVisor | userspace application kernelによるcontainer隔離。[S1] | Linux sandbox強化の依存採用候補。syscall互換性・通信遮断を条件にする。承認・再開・冪等は該当なし。 |
| Firecracker | Linux KVMでmicroVM。`/dev/kvm` とjailer等が必要。[S2] | 将来の隔離比較の参考。macOSホストでそのまま動く前提にしない。WSL内KVM可用性は未確認。業務承認等は該当なし。 |
| microsandbox | libkrunを使うsandbox。runtime・firmware・kernel等の組合せ互換性を扱う。[S3] | 依存採用候補。3OSのCPU・仮想化条件、配布物全体のライセンスと無通信を確認してから。Bot単位常駐を要求せずtask/session単位を比較。 |

## Pelwayの9状態との対応

[設計提案] 下表は参考機構の対応であり、既存OSSがPelwayの業務意味を実装するという主張ではない。「該当なし」は対象層が違う場合、「未確認」は資料から確定しない場合。

| Pelway状態 | [事実] 参考になる機構 | [設計提案] Pelwayに残す意味 |
|---|---|---|
| 依頼済み | Slackの受信ACK後queue、Matrix AS transaction。[C5][M2] | 依頼受付を永続化。受信ACKを実作業完了にしない。 |
| 作業中 | OpenHands actionとobservation、LangGraph node。[A1][A2] | task実行所有者・lease・試行IDを記録。Botへのメンション全てを実行にしない。 |
| 判断待ち | LangGraph interrupt、Agent Framework request info。[A2][A5] | 質問、回答者権限、有効期限、再開位置を固定。 |
| 送信承認待ち | OpenHands確認判定、Letta approval message。[A1s][A7] | 本文・宛先・添付・送信元を固定。LLMのrisk判定だけでは送信許可しない。 |
| 送信中 | Matrix local echoとtxnId。[M1] | 外部操作の試行開始を表す。完了印を出さない。 |
| 送信済み・顧客回答待ち | Matrix event確定はチャット内配送まで。[M1] 顧客回答待ちの同一業務意味は未確認。 | 外部サービスの受理証拠・受信IDを持つ。顧客の既読と回答は別。 |
| 結果不明 | Matrix再送識別、LangGraph副作用注意。[M1][A2] 外部送信結果照会の一体実装は未確認。 | 自動再送しない。保存済みoperation IDで照会し、確定できなければ停止・人へ判断を返す。 |
| 止まっている | Letta #3206のpending approval停止報告。[A7i] | 原因・復旧条件を残す。cancel受付と副作用停止を分ける。 |
| 完了 | LangGraph終端は実行終了の機構。[A2] 業務成果の完了条件は該当なし。 | 送信成功だけで業務完了にしない。必要な確認が揃ったときのみ完了にする。 |

[設計提案] `task_id → step_id → operation_id` を永続化し、Bot交代でoperation IDを作り直さない。handoffは担当者変更として記録する。
[設計提案] 承認は不変の `payload_hash` と本文・宛先・添付版へ結び、編集・添付差替え・宛先変更で失効する。送信直前に権限、承認期限、現行payloadの一致を再確認する。
[設計提案] 未送信のoutbox、実行試行、提供元結果、承認判断、権限取消、取消要求を別イベントとして残す。秘密そのものをログに含めない。
[未検証] 「固定承認＋直前権限再確認＋提供元冪等キー＋結果照会＋結果不明の自動再送禁止」を全て満たす汎用OSS実装は今回未確認。独自設計・試験が必要であり、OSS採用で安全合格を代替しない。

## ライセンスと保守の確認台帳

[事実] 以下のリリースは取得した公式一覧での最新表示または確認できた版。[未検証] 最新commit日時・期間内commit件数は、明記した例以外は未確認。リリースの頻度は保守契約や安全保証ではない。
[未検証] 検索indexと可変branchに差がある場合があるため、採用時に固定タグのLICENSE・依存・NOTICEを再確認する。SPDXのonly/or-laterが確定しないものは未確定と明記した。

| OSS | [事実] ライセンス・配布上の影響 | 保守観測と未確認 |
|---|---|---|
| Synapse | AGPL-3.0-or-laterと商用の選択、CLAあり。[M3] | [事実] v1.164.2、2026-11-02。[M3r] [未検証] 直近commit頻度。 |
| Conduit | Apache-2.0。[M4] 通知・改変表示等を維持。 | [事実] v0.10.15、2026-09-30。9月にセキュリティ修正複数。[M4] [未検証] commit頻度。 |
| conduwuit / Continuwuity | [事実] 継続先はApache-2.0。[M5] [未検証] 旧版の全ファイル条件。 | [事実] conduwuitはarchivedの案内、継続先v26.9.1、2026-09-28。[M5] [未検証] 最新commit。 |
| Element Web / X iOS | WebはAGPL-3.0-only、GPL-3.0-only、商用の選択。X iOSはAGPL-3.0-onlyまたは商用。[R3][L1] SDKとは別条件。 | [事実] Web v1.12.34-rc.1、2026-11-06はpre-release。[L1r] X iOSはactiveとの公式表明。[R3] [未検証] X最新release日とcommit頻度。 |
| matrix-rust-sdk | Apache-2.0。[R1] | [事実] 0.19.1、2026-09-18。[R1r] [未検証] 最新commit頻度。 |
| Fractal | [未検証] GPL-3.0との検索結果はあるがLICENSEページがchallenge表示。SPDXのonly/or-laterと本文は未確認。[R0l] 流用不可判定を保留。 | [事実] tagsで13、2025-10-31を確認。[R0] [未検証] 2026年の最新release・commit。 |
| iamb | Apache-2.0。[R2l] | [事実] releasesに0.13.1、2026-10-21、tagsのindexには古い版のみ。[R2] [未検証] 最新性・commit頻度。 |
| Zulip | Apache-2.0。[Z5] | [事実] 12.4、2026-11-06、13.0-rc1は別。[Z5r] [未検証] 最新commit頻度。 |
| Mattermost | ソースはAGPL-3.0、商用選択と例外。指定admin・webapp等はApache-2.0。公式compiled版はMIT扱い。独自ビルドへMITを外挿しない。[L2] | [事実] 12.1.0等を2026-11-05に公開。[L2r] [未検証] 各ファイルのonly/or-later・最新commit。 |
| Rocket.Chat | 対象外ディレクトリはMIT、`ee/` 等は別商用条件。第三者部品は別。[L3] | [事実] 8.9.0 LTS、2026-10-05が確認できる。[L3r] [未検証] 全系列最新性・commit頻度。 |
| OpenHands | MIT本体と別商用enterpriseを区別。[A1l] enterpriseのPolyForm Free TrialはOSSではない。 | [事実] v1.37.0、2026-11-06、11月に連続release。[A1r] [未検証] SDK別repoの最新性。 |
| LangGraph | MIT。[A2l] OSS libraryと商用Agent Server・LangSmithの条件を分ける。 | [事実] 1.2.20、2026-11-05。[A2r] [未検証] 最新commit頻度。 |
| MCP Rust SDK | Apache-2.0移行中、同意未取得の旧寄与はMIT、文書はCC-BY-4.0。単一SPDXで全体を断定しない。[A3l] | [事実] rmcp 3.9.0、2026-11-05。[A3r] Cargo indexには別版もあり採用タグ照合が必要。 |
| AutoGen / Agent Framework | codeはMIT、AutoGen docsはCC-BY-4.0。[A4][A5l] | [事実] AutoGen archived。Agent Framework Python 1.25.0、2026-11-07。[A5r] [未検証] commit頻度。 |
| CrewAI | MIT。[A6l] AMP等のサービス条件は別。 | [事実] 1.15.37、2026-11-07、11月に複数release。[A6r] [未検証] 最新commit頻度。 |
| Letta | 旧server Apache-2.0。[A7l] 現在の別製品・配布物へ外挿しない。 | [事実] 旧server最終表示0.16.8、2026-05-14、retiredの案内。[A7r] [未検証] 新SDKの採用条件。 |
| Goose | Apache-2.0。[A8l] | [事実] v1.56.0、2026-10-23、release後mainへ45 commitsとの表示。[A8r] [未検証] その集計の更新時刻。 |
| rig | MIT。[A9l] | [事実] 0.46.0、2026-10-31。[A9r] [未検証] 最新commit頻度。 |
| gVisor / Firecracker / microsandbox | 各本体はApache-2.0。[S1][S2][S3] kernel・guest imageを同梱する権利は別確認。 | [事実] gVisor 20261102.0、2026-11-02。Firecracker 1.17.0、2026-09-10。microsandbox 0.7.13、2026-11-01。[S1r][S2r][S3r] [未検証] commit頻度。 |
| Slint / egui | SlintはGPLv3・LicenseRef-Slint-Royalty-free-2.0・商用の選択。Royalty-freeはAboutSlint等の表示条件。eguiはMIT OR Apache-2.0、default fontsは別条件。[U1][U2] | [未検証] 最新release日とcommit頻度。SlintのCLAは上流寄与条件であり利用権の代替ではない。[U1] |

[事実] AGPL第13条は改変版をネットワーク越しに使う利用者へのCorresponding Source提供を扱う。GPLと同一のネットワーク条項ではない。[L2]
[設計提案] 非公開商用配布を前提にAGPL/GPLコードをコピーしない。分離APIだけで常に非派生になるとも断定しない。許諾方式・結合方式を決めて法務確認する。
[設計提案] MITは著作権・許諾文、Apache-2.0はLICENSE、必要なNOTICE、改変表示を配布物へ含める。CLAは上流寄与時の別確認であり、下流利用時に自動で商用許諾を得るものではない。[A8l][L2]

## 国内限定：外向き通信の棚卸し

[設計提案] 実行時は日本国内の管理端末にのみ機密データを置き、AI・添付・DB・監査・一時ファイル・crash dump・backupも国内に限定する。端末の所在とOSクラウド同期も確認する。
[未検証] 本調査は静的資料の確認であり、全依存・全版の「通信なし」を証明しない。表の「未確認」は無効化できないという断定ではなく、採用を止める未解決条件。
[未検証] 全行とも、記載以外のcrash送信・更新確認・プッシュ・URL preview・外部IDP・検索の既定値は未確認。仕様と配布製品・インストーラーの既定値を区別する。
表のモデル列で「該当なし」はその部品自体のLLMモデル取得の役割が対象外という意味。第三者pluginのAI通信まで否定しない。

| OSS / 段階 | テレメトリ・診断 | 外部サービスへの既定接続と無効化 | モデル取得とモデルのライセンス |
|---|---|---|---|
| Matrix仕様 / 実行 | [未検証] 実装の既定値は仕様から決まらない。 | [事実] AS・media・pusher等を仕様化。[M1][M2] [未検証] 配布実装の既定接続。 [設計提案] federation・外部push・URL取得を許可しない。 | 該当なし。 |
| Synapse / 実行 | [事実] `report_stats` でusage送信を設定。[N1] [設計提案] false固定。 | [事実] `url_preview_enabled`、`federation_domain_whitelist` を設定できる。[N1] [設計提案] preview=false、federationは空許可一覧。pushも無効化を別確認。 | 該当なし。 |
| Conduit / 実行 | [未検証] stats・crash・更新の既定値と停止方法。[M4] | [未検証] federation・URL preview・pushの全遮断設定。参考に留める。 | 該当なし。 |
| conduwuit・Continuwuity / 実行 | [未検証] 各forkの診断既定値。[M5] | [未検証] 外部server・push・更新等の停止方法を固定版で確認してから。 | 該当なし。 |
| Element Web・X / 構築・実行 | [事実] WebのSentry・PostHogはconfig項目。PostHog未設定ならanalytics無効。[N2] [未検証] X全体の既定値。 | [未検証] 配布版のhomeserver・identity・push・crash・更新・call接続先。自前configだけで無通信とは断定しない。 | 該当なし。 |
| Zulip / 実行 | [事実] usage統計設定とSentry設定は別。[N3] [設計提案] usage false・Sentry未設定。 | [事実] mobile push forwarding serviceを利用できる。[N3] [設計提案] bouncer不使用・push off。URL preview・外部IDPのdefault/停止は未確認。 | 該当なし。 |
| Mattermost / 実行 | [事実] diagnostics・security alerts・error reportingの無効化をair-gap手順で公開。[N4] | [事実] phone-home・hosted push等を止める手順あり。[N4] [設計提案] HPNSを使わず、push全体を無効化。実行中の全通信停止は未試験。 | 該当なし。AI pluginは別。 |
| Rocket.Chat / 実行 | [未検証] 現行版の完全なstats停止方法。旧設定削除の記載あり。[N5] | [事実] push Enable GatewayをoffにできるがAPNs/FCM自前接続と無通信は別。[N5] URL preview・更新・IDPの既定は未確認。 | 該当なし。 |
| Fractal / 実行 | [未検証] 診断・更新既定値。[R0] | [未検証] homeserver discovery、media、URL、desktop通知の全通信境界。 | 該当なし。 |
| iamb / 実行 | [未検証] 診断・更新既定値。[R2] | [事実] Matrixクライアント。[R2] [未検証] remote media・discovery・通知の停止条件。 | 該当なし。 |
| matrix-rust-sdk / 実行 | [未検証] SDK・FFI・組込み製品を含む外部診断の停止証拠。[R1] | [事実] Matrix同期が役割。[R1] [未検証] discovery・media・push全経路の停止設定。 | 該当なし。 |
| OpenHands / 実行 | [事実] Agent Canvasは `AGENT_CANVAS_DISABLE_TELEMETRY=1`、SDK traceはOTEL/LMNR変数をunset。[N6] | [未検証] これだけで全外部接続が止まる証拠。本体LLM・診断・image取得は別審査。利用者が選ぶ外部連携の開示は後述。 | [未検証] 採用版のdefault LLM・埋め込み・モデルライセンス。 |
| LangGraph / 構築・実行 | [事実] CLI既定analyticsは `LANGGRAPH_CLI_NO_ANALYTICS=1`、traceは `LANGSMITH_TRACING=false`。[N7] | [事実] standalone Agent Serverはlicense・run数確認APIを呼ぶ。[N7] [設計提案] Server/Studioを依存採用しない。 | [未検証] graph内のproviderと検索・embeddingのdefault。ライブラリとmodelのlicenseは別。 |
| MCP Rust SDK / 実行 | [未検証] 固定版のtelemetry・crash既定と無通信証拠。[A3] | [事実] stdio/HTTP等でserverを接続するSDK。[A3] [設計提案] 本体の既定toolは国内ローカルstdioのみ許可、server自身のegressも遮断。利用者選択のremote連携は別枠。 | [未検証] tool側モデル取得・license。SDKの許諾はモデルを含まない。 |
| AutoGen / 実行 | [未検証] 採用対象版の診断設定。[A4] | [未検証] provider・tool・更新先。archiveのため新規依存候補から除外。 | [未検証] 指定provider・modelごと。 |
| Agent Framework / 実行 | [未検証] telemetry/exporterの全default。[A5] | [未検証] provider・認証・外部serviceのdefaultと停止。参考に留める。 | [未検証] provider・modelごと。 |
| CrewAI / 実行 | [事実] `OTEL_SDK_DISABLED=true` と `CREWAI_TRACING_ENABLED=false` は別設定。[N8] | [未検証] 全default LLM・検索・AMP接続の停止範囲。参考に留める。 | [未検証] embeddingも別確認。モデル許諾をMITとみなさない。 |
| Letta / 実行 | [未検証] 旧serverと現行Codeの診断設定。[A7] | [事実] 現行docsはlocal toolとcloud computerを区別。[A7p] [未検証] 旧serverのdefault provider・停止。 | [未検証] model・embedding・memory処理先ごと。 |
| Goose / 実行 | [未検証] 採用版の診断・crash・更新を止める全方法。[A8] | [未検証] provider・extension・検索の既定接続。session保存がlocalでも無通信とは限らない。 | [未検証] providerごとのモデル取得・license。 |
| rig / 実行 | [未検証] 固定feature集合のdiagnosticsと無通信証拠。[A9] | [未検証] 選択provider、tool、search、embedding全接続。接続先を明示する条件付き候補。 | [未検証] 選ぶモデルごとの条件。MITは基盤コードのみ。 |
| gVisor / 構築・実行 | [未検証] 配布・監視統合を含む全diagnostics。[S1] | [未検証] guest networkの禁止設定・DNSを含む全遮断試験。隔離がegress禁止を意味しない。 | 該当なし。guest image取得は構築時。 |
| Firecracker / 構築・実行 | [未検証] 配布物全体の診断・更新既定。[S2] | [未検証] guest network禁止の採用構成・観測。microVMでも外部通信は別制御。 | 該当なし。kernel/rootfs licenseは別。 |
| microsandbox / 構築・実行 | [未検証] 更新・診断のdefault。[S3] | [未検証] image自動取得・guest egressの完全停止。3OS同一ポリシーの適用も未確認。 | 該当なし。image/firmware/kernelは別許諾。 |
| Slint / 構築・実行 | [事実] VS Code extension/live previewにtelemetry実装。VS Codeの設定に従う記載。[N9] [未検証] 製品runtimeの全通信。 | [設計提案] 開発ツールを機密実行環境から分離。アプリruntimeの外部画像・更新・診断を別監査。 | 該当なし。フォント・画像は別license。 |
| egui / 構築・実行 | [未検証] core・backend・demo・統合先を含む無通信証拠。[U2] | [未検証] image loader・URL opener・更新等の採用feature別接続。 | 該当なし。default fontsの許諾は別。[U2] |

## 外部LLM・埋め込みAPIの国内性

[設計提案] 「国内保存」「国内推論」「国外送信なし」は別の判定欄にする。運用アクセス、support、再委託、abuse monitoring、障害時迂回を含む全条件が揃うまでPelway本体のAI・埋め込み用外部APIは未承認。利用者が明示的に選ぶ外部サービス連携は次節の別枠。
[未検証] 東京endpoint、国内販売、暗号化、学習不使用、ZDRのいずれも単独で国外送信なしを証明しない。下表は購入候補の承認ではない。

| API | [事実] 国内保存と国内推論の一次資料 | [未検証] 国外送信なし・判断 |
|---|---|---|
| Azure OpenAI | Regional/StandardとGlobal/Data Zoneを区別。前者は指定region/geographyで処理、Globalは他region、Data Zoneは指定zone内に配信。Japan Eastのモデル別可用性は別確認。[D1] | abuse監視のhuman reviewer所在地について日本限定の証拠は未確認。資料間のregion/geography表現、support・再委託・保存featureも契約で確認。Global/APACを国内とみなさず、全て未承認。 |
| AWS Bedrock | in-Regionは指定region内、GeoとGlobalは別。東京からのcross-region profileは宛先一覧を確認する必要がある。abuse保管は宛先regionとの記載。[D2] | APACは日本限定ではない。JP profileという名前だけでも承認しない。model、embedding、保管・運用・subprocessorを確認するまで未承認。 |
| Vertex AI | 保存locationとML processing保証を分けて記載。Tokyo `asia-northeast1` を含むmodel別表があるが、endpointだけでは保証にならない。[D3] | 取得ページの対応印が読み取りで欠落したため、特定モデルの東京処理可否は未確認。support・監視・grounding・global fallbackも未確認で未承認。 |
| OpenAI直API | Japanの保存residencyと `jp.api.openai.com` の案内がある一方、Japanはregional processing対応なしとの表。[D4] | 国内保存を国内推論と呼ばない。国外送信なし条件の採用根拠にならず、未承認。 |
| Anthropic直API | 既定routingは複数国、保存はUS。安全review・support等は拠点国で処理し得ると明記。[D5] | 日本限定の別契約・設定の確認なし。Bedrock/Vertex経由の条件とも別。現状は未承認。 |

## ローカル推論の代替

| 部品 | [事実] ライセンス・通信設定 | [設計提案] 用途 / [未検証] 条件 |
|---|---|---|
| llama.cpp | MIT。[D6l] server docsに `--offline` / `LLAMA_OFFLINE=1` の記載。[D6] | 条件付き第一候補。事前取得した固定modelファイルだけを使用。全依存・driver・OS通信なしは未検証。 |
| vLLM | Apache-2.0。[D7l] usage statsは既定収集、`VLLM_NO_USAGE_STATS=1` または `DO_NOT_TRACK=1` で無効化。[D7] | GPU server比較の条件付き候補。model自動取得・telemetry・remote codeを分けて管理。機材容量と性能は未測定。 |
| Ollama | 公開repoはMIT。[D8l] `OLLAMA_NO_CLOUD=1` でcloud modelとweb searchを無効化できる。[D8] | ローカルAPI候補。ただしGUI配布物のlicense・自動更新・全telemetry停止は別確認し、repoのMITを全製品へ外挿しない。 |
| Qwen3-8B | 公式model cardは日本語を含む多言語対応、Apache-2.0。[D9] | 日本語評価の候補。「強い」は本業務の品質保証ではない。量子化別品質・RAM/VRAM・文脈長別実測は未確認。 |
| llm-jp-3-3.7b-instruct | 公式公開model cardでApache-2.0。[D10] | 日本語中心の比較候補。tool call・送信判断・長文品質、必要機材は未検証。モデルcardの利用制約も固定版で再確認。 |

[設計提案] local infrastructureとlocal inferenceは別軸。国内端末上のcontainerから国外LLMへ送れば国内限定ではない。日本国内のlocal inferenceでもMCP・URL・OS同期から流出し得る。
[設計提案] 当面の初期設定は仮LLMまたは事前取得したlocal modelのみ。機密を載せる前に受信許可、保存場所、通信遮断と監査を確認する。

## 構築時と機密実行時の越境経路

| 段階・経路 | 外へ出得るもの | [設計提案] 初期方針・確認条件 |
|---|---|---|
| 構築：Cargo/npm/PyPI、OS package、container registry | IP、依存名、認証・プロジェクト識別情報。build scriptの追加取得は未確認。 | 機密を載せない取得専用環境と国内mirror/cacheを分離。固定版・hash・licenseを確認し、実行環境で取得しない。 |
| 構築：モデル・tokenizer・embedding weight取得 | IP、model名、取得token。remote code実行の有無はmodelごとに未確認。 | 日本国内保存先へ事前搬入し、検証済みファイルだけ使用。ライセンスと配布権を別台帳にする。[D6][D7][D9] |
| 実行：LLM・embedding・rerank・検索 | 会話、添付、要約、検索語、vector。 | local固定。外部fallback禁止。国内処理・運用アクセスまで確認できないAPIは未承認。[D1]〜[D5] |
| 実行：MCP・browser・URL preview・remote media | tool入力、URL、Cookie、検索語、取得先IP。 | stdioでも子processのnetworkを別制限。本体のURLプレビュー・外部画像自動取得をoffにする案。利用者が有効にする外部連携は別枠。[M1][A3][N1] |
| 実行：telemetry・crash・trace・error reporting | stack、prompt、tool結果、ファイルpath、dump。 | 全exporterを無効化、dumpは国内保存。設定だけでなく再起動・障害時を含め観測。[N2][N4][N6]〜[N9] |
| 実行：push relay、APNs/FCM、メール通知、外部IDP | token、channel情報、本文、ユーザー属性。 | 本体通知はlocal desktopのみを初期案とし、relay・外部認証・外部メールを使わない。利用者選択の業務メール送信等を禁止する意味ではない。[N3]〜[N5] |
| 実行：更新・DNS・NTP・証明書検証・CDN・OS同期 | host情報、接続先、backup、一時ファイル。 | アプリ以外も対象。国内resolver/mirrorとdefault-denyを設計し、OS更新経路と機密実行を分離する。現環境の全経路は未検証。 |
| 実行：support・運用閲覧・backup復旧・再委託 | content、log、key、snapshot。 | 国内保管だけで合格にしない。国外閲覧・再委託・failoverの契約と実運用を確認。本体AIで未確認なら外部接続不可。利用者選択連携は開示と権限・個別承認の別枠。[D1][D2][D5] |

## AIネイティブの軸と利用者が選ぶ外部エージェント連携

確認日：2026-11-08（日本時間）。この節は将来の接続設計のための公開資料調査。接続・登録・実装は行っていない。
[設計提案] AIをメッセージ生成の補助機能に閉じず、権限を持つ主体、task/run/session、成果物、承認、監査を第一級のデータとして扱う。外部agentはadapterを介して参加させる。
[設計提案] 利用者・管理者が自分で有効にする外部サービス連携は、Pelway本体の国内限定とは切り分ける。利用者が情報を見て自己責任で選べる設計とし、所在地未確認だけを理由に利用不能とはしない。
[設計提案] 本体AI処理・保存・telemetry・依存の既定通信は国内限定のまま。外部LLMへの自動fallbackは禁止。外部連携を含めて「国外送信なし」と保証しない。
[設計提案] 有効化は送信の包括承認ではない。操作権限、送信ごとの承認、監査、秘密管理、payload固定・再承認・結果不明停止を維持する。国外端末利用・持出しは未決のまま。
[設計提案] Claude Code・Cursor・Datadog・Google Workspaceは設計の妥当性を確かめるための例示であり、連携対象を限定しない。固定リストや4製品専用の基盤にせず、共通の契約・能力定義とサービス別adapterで他のAIエージェントや業務ツールも追加できる形にする。対象サービスの網羅調査は今回行わない。
[設計提案] 外部連携は初期オフ。ただし国内所在が不明という理由だけで有効化ボタンを消さず、権限のある利用者・管理者が、送り先・送信データ・国外処理/保存の可能性・責任範囲を見たうえで明示的に有効化できる。外部メールを同じ責任分界に含めるかは未決。
[設計提案] 今回は日本向けのローカル単一構成に限る。保存先・AI処理先・ログ/バックアップ先・外部連携の送信方針は日本を直書きで散らさず、一つの配置・データ所在ポリシーに結び付けるだけにする。将来の米国・EU・日本の切替は選択肢として示すだけで、リージョン選択はデータ所在の制御であって法適合の保証ではない。リージョン変更時の既存データ・バックアップ移行や契約・アクセス管理は将来の要検討とし、自動移行や自由な切替は保証しない。

### 公式に提供される連携手段

「今可能」は公開された手段があるという意味で、購入済み・接続済みではない。「制約あり」はプラン、beta、管理者設定などの条件がある。全て参考であり製品コードの依存採用・改変流用ではない。

| 対象 | [事実] 今可能 / 制約あり / [未検証] 将来案 | 利用者が判断するための情報 |
|---|---|---|
| Claude Code | 今可能：CLI `-p`、stream-json、session IDでresume。[E1] hooks、MCP。[E2] [未検証] SDK・GitHub Actionsの個別契約条件。 Slack連携あり。ただし取得時の旧版docsは個人プラン等に制限、新組織版Claude Tagとの違いを説明。[E3] [未検証] 利用時点のSDK契約・プラン別全条件。 | [事実] Slackの依頼文・会話context・repoをClaude cloud sessionへ渡し、Slackに進捗と結果を戻す。[E3] [未検証] 個別cloud sessionの処理/保存国・国内region選択。直APIの国外処理説明[D5]を全配布形態へ自動適用しない。 |
| Cursor | 今可能：CLI headless、MCP、hooks、Slack/Linearからの依頼。[E4] 制約あり：Cloud Agents API v1はpublic beta。agentとrunを分け、SSE/取得/取消がある。v1 webhookはcoming soon、旧v0の対応と混同しない。[E5] | [事実] prompt・repo・環境・MCP設定をCursor APIへ送り、cloudまたはself-hosted workerで実行。artifact例はUS regionのS3 URL。[E5] [未検証] 例以外の全保存先、国内region選択、LLM/管理planeの国内処理。self-hostedだけで国内完結としない。 |
| Datadog | 今可能：Webhooks通知、REST、公式MCP、On-Call Events API。`aggregation_key` でpage重複排除。[E6] 制約あり：Incidents APIの取得資料にpublic beta、権限・toolset要件。[E7] | [事実] MCPはtool名・引数をDatadogへ受け、logs/incident等をclientへ返す。MCP server自体は外部domainへoutboundしない旨の説明。[E6] 利用情報・error・user識別子等の収集は120日保管と記載。大半のtoolはAIなしだが一部はAI providerを使い、組織全体の生成AI無効化はsupport経由。[E6] AP1はJapan site。[E8] [未検証] 全製品のAI推論、support・再委託までJapan限定か。site選択とMCP対応siteを契約ごと確認。 |
| Google Workspace | 今可能：Chat、Drive、Docs、Gmailの公式API。制約あり：公式remote MCPはDeveloper Preview Program会員向け。製品ごとのOAuth・Streamable HTTP endpoint。[E9] Events APIはPub/Sub通知、Drive eventは取得docsでDeveloper Preview。[E10] REST送信APIとMCP公開toolの範囲は同一ではない。[E11] | [事実] OAuth scopeに応じたメール・文書・Chat内容やtool引数がGoogle APIへ送られる。MCPはGoogleの各endpoint。[E9] Workspace data regionsは対象データについてUS/Europeまたは地域指定なし、editionにより域内処理も選べる公式案内。[E12] 国別Cloud Storage bucketへのlocal data storage案内もあるが全Workspaceデータ・推論の同国内化とは異なる。[E12b] [未検証] 対象edition・全APIの処理/保存国、日本選択の保証。Google Cloud Tokyoの存在をWorkspaceへ外挿しない。 |

### 外部エージェントへの仕事の受渡しモデル

表は6観点の公式記述と不足を比較する。全行の分類は参考。外部SaaSをPelwayのランタイム依存にする提案でも、製品コードを改変・流用する提案でもない。

| 製品 | 識別・関連付け | 進捗・承認 | 中断/再開・イベント |
|---|---|---|---|
| Linear Agents | [事実] OAuth appのagent、issueへの委任/mentionでAgentSessionを生成。issue/comment/session IDとexternalUrlsで成果物を結ぶ。agentの5 activity種別と人のpromptを区別。[E13] | [事実] thought/action/elicitation/response/errorで進捗。elicitationは質問・確認の機構。[未検証] 宛先・添付まで固定した外部送信承認の保証。 | [事実] stop signalとstaleからの活動再開。[E14] webhookは失敗再送と署名。[E15] [未検証] 全イベントの厳密順序・副作用exactly-once。 |
| Slack Agents & AI Apps | [事実] agent app表示、channel ID/thread_tsをキーとし、assistant thread旧APIと新agent session APIの互換bridgeを公開。[E16] | [事実] processing/suspended/active、streamまたはpost/update。承認の具体的業務意味はapp側。[E16] [未検証] 汎用artifact台帳・送信固定承認。 | [事実] stop buttonは `agent_session_stopped` 購読と対応。配信はEvents APIの再送規約。[E16][C5] [未検証] stop時の外部実行巻戻し・自動resume。 |
| GitHub Copilot coding/cloud agent | [事実] Copilotのcommit主体と依頼者coauthor、PR・branch・session logの関連付け。[E17] | [事実] 生成PRは人がreview/merge、workflow実行の承認も分離。session/audit logで追跡。[E17] [未検証] Pelwayの社外送信承認との等価性。 | [事実] agent session管理を公開。[E18] [未検証] 取得範囲ではcancel・再開の全契約、通知順序・重複排除の保証。 |
| Cursor Slack/Linear | [事実] Slack mentionやLinear delegationがagentとrunへ接続。成果はbranch/PR/artifactへ。thread follow-upを受け付ける。[E4][E5] | [事実] SSEにstatus/tool_call/result、`callId` を持つ。PR作成設定と人のreviewは区別。[E5] [未検証] 外部送信ごとの固定承認。 | [事実] run cancelはterminal、継続は同agentの新run。SSEはLast-Event-ID、期限切れ410ではGet Run。createのclient agentId重複は409。steer再試行は二重promptになり得ると明記。[E5] |
| Claude Code Slack | [事実] 個人accountでcloud session、threadの依頼contextとrepoへ結び、View session/Create PRへ。[E3] | [事実] Slack status更新・最終返信・session transcript。PR作成操作が別。[E3] [未検証] 各tool承認とSlack操作の一致。 | [事実] View sessionから継続、CLIではsession ID resume。[E1][E3] [未検証] Slackイベントのagent側冪等・cancel契約全体。 |

[設計提案] 外部イベントの取込は `connector_id/external_actor_id/external_task_id/run_id/session_id/artifact_id/source_event_id` を保存するadapterへ集約する。外部状態をそのままPelwayの状態に上書きしない。
[設計提案] 外部sessionの完了は「成果物が戻った」証拠であり、Pelwayの顧客応答待ち・完了を自動で確定しない。Agent ActivityのthoughtをLLMの内部推論保存要件と解釈しない。

| Pelway状態 | [事実] 近い外部状態・機構 / [未検証] 非等価な部分 | [設計提案] 取込規則 |
|---|---|---|
| 依頼済み | Linear pending、Cursor CREATING。[E13][E5] | task受付を保存後に外部enqueue。外部ACKと実行を分離。 |
| 作業中 | Linear active、Slack processing、Cursor RUNNING。[E13][E16][E5] | 進捗はrun単位、複数Botの担当変更を別記録。 |
| 判断待ち | Linear awaitingInput/elicitation、Slack suspended、A2A input-required。[E13][E16][E19] | 人の判断者権限を照合して再開。 |
| 送信承認待ち | 汎用elicitationはあるが固定payload送信承認との等価性は未確認。 | 外部agentの許可をPelwayの送信承認とみなさない。 |
| 送信中 | Cursor tool_call running等は途中経過であり、外部送信中との同一性は未確認。[E5] | operation IDと対象toolが一致した場合のみ対応。 |
| 送信済み・顧客回答待ち | PR生成・最終responseは顧客回答待ちではない。該当なし。[E3][E17] | 提供元受理証拠と対象業務の条件を別確認。 |
| 結果不明 | Cursor steer重複警告・stream期限切れ等。[E5] | 再実行でなく結果取得を先に行い、取得不能なら人へ返す。 |
| 止まっている | Linear error/stale、Cursor ERROR/CANCELLED、A2A canceled/failed。[E13][E5][E19] | 失敗、取消、通信断を原因別に残す。terminal run再開を捏造しない。 |
| 完了 | Linear complete、Cursor FINISHED、A2A completed。[E13][E5][E19] | 外部run終了だけでPelway完了にせず、成果確認後に判断。 |

### インターフェースの役割と限界

| 手段 | [事実] 役割と一次資料 | [設計提案] 限界・責任境界 |
|---|---|---|
| MCP | tool/resource接続、能力交渉・認証・取消等。[A3][A3s] | task委任・chatイベント・配布制約の全ては担わない。remote/stdioどちらも権限・送信承認・egress審査は別。 |
| webhook | Linear/Slack等がイベントをpushし、再送・署名規約を持つ。[E15][C5] | ACK後にinbox永続化、重複排除、順序乱れ補正、結果照会。受信イベントを実行命令へ無条件昇格しない。 |
| REST/GraphQL | Cursor Run取得・取消、Linear Activity mutation、Google文書操作等。[E5][E13][E11] | 操作ID・冪等契約・照会APIはendpointごとに違う。HTTP timeoutを失敗確定にしない。 |
| CLI/hook | Claude stream-json/resume、Cursor/Claudeのlifecycle hook。[E1][E2][E4] | ホスト権限で動くためsandboxと別制御。Cursor hookの一部は既定fail-open、security-criticalにはfailClosed指定との記載。[E4h] |
| A2A | agent間のcontextId/taskId、task状態、artifactとstream更新を定義。terminal taskは再開せず同contextに新task。[E19] | 参考。業務権限・国内処理・exactly-once・成果物版管理の全てを保証しない。MCPの置換ではなく委任adapter候補。 |

- [事実] Claude Slack公式は "Claude may follow directions from other messages in the context" と注意し、信頼できる会話での利用を求める。[E3]
- [事実] Google Workspace MCPも、未信頼のメール・文書内のhidden instructionからデータ改変・窃取・削除が起きるリスクと、操作レビューを明記する。[E9]
- [事実] GitHubはissue/commentのhidden instructionによるprompt injectionを認め、hidden character等のフィルタと権限・network制限を説明。[E17]
- [設計提案] 外部文書・ログ・tool出力はデータとして取り込み、実行権限・送り先変更の根拠にしない。ソースと命令主体を保持し、URL・添付・ログ内の指示でsecret開示や外部送信を許可しない。
- [設計提案] 初版はinterfaceとデータモデルを用意する設計に留める。外部連携を有効にする前に、送信項目・宛先・保管/処理地域・管理者scopeを表示できるadapter契約を定義する。

## 推奨区分・差分と未確認事項

- [設計提案] 参考はMatrix仕様・AS、Zulip、Mattermost bot主体、Element Xのcore/UI分離、LangGraph、OpenHands、Agent Framework、Goose。複数Botの会話はチャット参加モデル、実行はtaskモデルで表す。
- [設計提案] 依存採用候補はMCP Rust SDK、rig、local inference。全て固定版・license・全通信経路の確認が条件。採用決定済みはゼロ。gVisor・microsandboxは後段の隔離比較候補。
- [設計提案] 改変・流用候補はGoose coreの小さいsession/tool部分と、許諾を確認できたMIT/Apacheの例に限定。GPL/AGPL製品UI・server codeの複製は保留する。
- [未検証] UIのIME・大量履歴・複数選択・読み上げ、全OSSの無通信、local model品質と機材、提供元の外部副作用結果照会、全固定版ライセンスを未完として残す。
- [設計提案] [短期v0.1](../../pelway-milestones/short-term-milestones.md) のP0-T・再承認・結果不明停止、実装/実送信未承認を維持。本調査を需要検証・安全実験・環境承認の代わりにしない。
- [設計提案] 差分/要確認は「Botごとの常時VM」からidentity・永続状態と必要時sandboxを分ける案、国内限定のため外部LLMを無効にする初期案。既存要件を更新せず統合担当の判断事項にする。

### 要件追加による変更点

| 対象 | 国内限定追加前の検討 | 追加後の区分・条件 |
|---|---|---|
| MCP Rust SDK・rig | Rust AI層の第一候補 | [設計提案] 無条件採用ではなく、固定版license・全egress・tool/provider先確認後の依存採用候補。 |
| LangGraph・OpenHands・Goose | 再開・runtimeの実装候補も比較 | [設計提案] 初期案は参考へ。Serverのlicense検証通信、telemetry、MCP先まで確認するまでは組み込まない。 |
| 外部LLM・embedding | 利用有無未決 | [設計提案] 国内保存だけでは不可。国外処理・運用・再委託まで未確認は未承認。本体AIの初期設定は外部接続なし。利用者選択の連携は別枠。 |
| local inference | 任意の代替 | [設計提案] 国内閉域初期案の中心へ。ただしmodel取得・license・全telemetry確認は未完。 |
| push・preview・更新 | 補助機能として後段比較 | [設計提案] 初期は無効。構築時取得と機密実行時を分離し、国内経路の根拠が揃うまで有効化しない。 |
| 利用者選択の外部サービス連携 | 国内限定未確認なら無効とする方向も検討 | [設計提案] 今回修正で本体の国内限定と切り分け。利用者/管理者が有効化し、送り先・送信データ・処理/保存国・residency選択の開示で判断。所在地だけで利用禁止にしない。個別承認・監査は維持。 |
| AIネイティブ・将来連携 | MCP中心のtool接続比較 | [設計提案] task/run/session/artifact主体のadapterへ拡張。MCPに加えwebhook・REST・CLI/hook・A2Aを役割別に比較。外部状態の直結と自動LLM fallbackはしない。 |
| 連携対象の範囲 | 4サービスを個別に調査 | [設計提案] 4つは例示で限定しない。共通契約・能力定義＋サービス別adapter。網羅調査はしない。 |
| 将来リージョン | 未記載 | [設計提案] 今は日本のみ。配置・データ所在ポリシーへの拡張境界だけ記載。法適合や自動移行は保証しない。 |
| Slint・egui | UI適性とlicense比較 | [設計提案] 開発tool telemetryとruntime通信を追加審査。UIの実機合格・国内限定を断定しない。 |

## 出典一覧

全出典の確認日：2026-11-08（日本時間）。公式本文・公式repo・公式issueのみ。issueは報告内容の存在の証拠であり、現在の再現・修正完了の証拠ではない。

[M1]: https://spec.matrix.org/latest/client-server-api/
[M2]: https://spec.matrix.org/latest/application-service-api/
[M3]: https://github.com/element-hq/synapse
[M3r]: https://github.com/element-hq/synapse/releases/tag/v1.164.2
[M4]: https://conduit.rs/changelog/
[M5]: https://github.com/continuwuity/continuwuity
[Z0]: https://zulip.readthedocs.io/en/stable/overview/architecture-overview.html
[Z1]: https://zulip.com/api/get-events
[Z2]: https://zulip.com/api/send-message
[Z3]: https://zulip.com/api/construct-narrow
[Z4]: https://github.com/zulip/zulip/blob/main/api_docs/include/rest-endpoints.md
[Z5]: https://github.com/zulip/zulip/blob/main/LICENSE
[Z5r]: https://github.com/zulip/zulip/releases/tag/12.4
[C1]: https://github.com/mattermost/mattermost/blob/c9a4092a/server/public/model/post.go
[C2]: https://developers.mattermost.com/integrate/reference/bot-accounts/
[C3]: https://developers.mattermost.com/blog/plugins-v2/
[C4]: https://developer.rocket.chat/apidocs/realtimeapi
[C5]: https://docs.slack.dev/apis/events-api/
[C6]: https://discord.com/developers/docs/events/gateway
[R0]: https://gitlab.gnome.org/World/fractal/-/tags
[R0l]: https://gitlab.gnome.org/World/fractal/-/blob/main/LICENSE
[R1]: https://github.com/matrix-org/matrix-rust-sdk
[R1r]: https://github.com/matrix-org/matrix-rust-sdk/releases
[R2]: https://github.com/ulyssa/iamb/releases
[R2l]: https://github.com/ulyssa/iamb/blob/master/LICENSE
[R3]: https://github.com/element-hq/element-x-ios/tree/develop
[R4]: https://github.com/element-hq/element-x-ios/issues/2568
[D0]: https://discord.com/blog/why-discord-is-switching-from-go-to-rust
[A1]: https://github.com/OpenHands/docs/blob/main/openhands/usage/architecture/runtime.mdx
[A1s]: https://github.com/OpenHands/software-agent-sdk/blob/main/openhands-sdk/openhands/sdk/security/analyzer.py
[A1l]: https://github.com/OpenHands/OpenHands/blob/main/LICENSE
[A1r]: https://github.com/OpenHands/OpenHands/releases/tag/v1.37.0
[A2]: https://docs.langchain.com/oss/python/langgraph/interrupts
[A2l]: https://github.com/langchain-ai/langgraph
[A2r]: https://github.com/langchain-ai/langgraph/releases/tag/1.2.20
[A3]: https://github.com/modelcontextprotocol/rust-sdk/
[A3s]: https://modelcontextprotocol.io/specification/2026-07-28/server/tools
[A3l]: https://github.com/modelcontextprotocol/rust-sdk/blob/main/LICENSE
[A3r]: https://github.com/modelcontextprotocol/rust-sdk/releases
[A4]: https://github.com/microsoft/autogen/
[A5]: https://learn.microsoft.com/en-us/agent-framework/workflows/checkpoints
[A5l]: https://github.com/microsoft/agent-framework/blob/main/LICENSE
[A5r]: https://github.com/microsoft/agent-framework/releases
[A6]: https://github.com/crewAIInc/crewAI/pull/7096
[A6s]: https://docs.crewai.com/v1.15.23/en/tools/ai-ml/codeinterpretertool
[A6l]: https://github.com/crewAIInc/crewAI/blob/main/LICENSE
[A6r]: https://github.com/crewAIInc/crewAI/releases/tag/1.15.37
[A7]: https://github.com/letta-ai/letta/releases/tag/0.11.7
[A7i]: https://github.com/letta-ai/letta/issues/3206
[A7l]: https://github.com/letta-ai/letta/blob/main/LICENSE
[A7r]: https://github.com/letta-ai/letta/releases
[A7p]: https://docs.letta.com/platform/computers/index.md
[A8]: https://github.com/block/goose/blob/58f3cc9e/documentation/docs/guides/goose-permissions.md
[A8l]: https://github.com/block/goose/blob/main/LICENSE
[A8r]: https://github.com/aaif-goose/goose/releases/tag/v1.56.0
[A9]: https://github.com/0xPlaygrounds/rig/blob/main/crates/rig-agent/README.md
[A9l]: https://github.com/0xPlaygrounds/rig/blob/main/LICENSE
[A9r]: https://github.com/0xPlaygrounds/rig/releases
[S1]: https://github.com/google/gVisor
[S1r]: https://github.com/google/gvisor/releases/tag/release-20261102.0
[S2]: https://github.com/firecracker-microvm/firecracker/blob/main/docs/getting-started.md
[S2r]: https://github.com/firecracker-microvm/firecracker/releases
[S3]: https://github.com/superradcompany/microsandbox/
[S3r]: https://github.com/superradcompany/microsandbox/releases
[L1]: https://github.com/element-hq/element-web/tree/develop
[L1r]: https://github.com/element-hq/element-web/releases/tag/v1.12.34-rc.1
[L2]: https://raw.githubusercontent.com/mattermost/mattermost/master/LICENSE.txt
[L2r]: https://github.com/mattermost/mattermost/releases
[L3]: https://raw.githubusercontent.com/RocketChat/Rocket.Chat/develop/LICENSE
[L3r]: https://github.com/RocketChat/Rocket.Chat/releases/
[U1]: https://github.com/slint-ui/slint/blob/v1.17.1/LICENSE.md
[U2]: https://github.com/emilk/egui/blob/main/README.md
[N1]: https://element-hq.github.io/synapse/latest/usage/configuration/config_documentation.html?highlight=url_preview_url_blacklist
[N2]: https://github.com/element-hq/element-web/blob/develop/docs/config.md
[N3]: https://zulip.readthedocs.io/en/stable/production/mobile-push-notifications.html
[N4]: https://docs.mattermost.com/deployment-guide/air-gapped-operations/disable-phone-home-features
[N5]: https://docs.rocket.chat/docs/push-notifications-faq
[N6]: https://docs.openhands.dev/openhands/usage/agent-canvas/backend-setup/docker
[N7]: https://docs.langchain.com/langsmith/data-storage-and-privacy
[N8]: https://docs.crewai.com/v1.15.35/en/telemetry
[N9]: https://github.com/slint-ui/slint/pull/8414
[D1]: https://learn.microsoft.com/en-us/azure/foundry/foundry-models/concepts/models-sold-directly-by-azure-region-availability
[D2]: https://docs.aws.amazon.com/bedrock/latest/userguide/inference-profiles-support.html
[D3]: https://docs.cloud.google.com/vertex-ai/generative-ai/docs/learn/data-residency
[D4]: https://developers.openai.com/api/docs/guides/your-data
[D5]: https://privacy.claude.com/en/articles/7996890-where-are-your-servers-located-do-you-host-your-models-on-eu-servers
[D6]: https://github.com/ggml-org/llama.cpp/blob/46dba9fc/tools/server/README.md
[D7]: https://docs.vllm.ai/en/stable/usage/usage_stats/
[D8]: https://docs.ollama.com/faq
[D9]: https://huggingface.co/Qwen/Qwen3-8B/raw/main/README.md
[D10]: https://huggingface.co/llm-jp/llm-jp-3-3.7b-instruct
[C1old]: https://github.com/mattermost/mattermost/blob/v5.11.1/app/post.go
[D6l]: https://github.com/ggml-org/llama.cpp/blob/master/LICENSE
[D7l]: https://github.com/vllm-project/vllm/blob/main/LICENSE
[D8l]: https://github.com/ollama/ollama/blob/main/LICENSE
[E1]: https://code.claude.com/docs/en/headless
[E2]: https://code.claude.com/docs/en/hooks
[E3]: https://code.claude.com/docs/en/slack
[E4]: https://cursor.com/docs/integrations/slack
[E4h]: https://cursor.com/docs/hooks
[E5]: https://cursor.com/docs/cloud-agent/api/endpoints
[E6]: https://docs.datadoghq.com/mcp_server.md
[E7]: https://docs.datadoghq.com/api/latest/incidents/get-a-list-of-incidents.md
[E8]: https://docs.datadoghq.com/getting_started/site.md
[E9]: https://developers.google.com/workspace/guides/configure-mcp-servers
[E10]: https://developers.google.com/workspace/events/guides/create-subscription
[E11]: https://developers.google.com/workspace/docs/api/reference/rest
[E12]: https://knowledge.workspace.google.com/admin/compliance/choose-a-geographic-location-for-your-data
[E12b]: https://workspace.google.com/intl/en_in/products/admin/data-regions/
[E13]: https://linear.app/developers/agent-interaction
[E14]: https://linear.app/developers/agent-signals
[E15]: https://linear.app/developers/webhooks
[E16]: https://docs.slack.dev/ai/developing-agents
[E17]: https://docs.github.com/en/copilot/concepts/security-governance-and-network-settings/risks-and-mitigations
[E18]: https://docs.github.com/en/copilot/how-tos/copilot-on-github/use-copilot-agents/manage-and-track-agents
[E19]: https://a2a-protocol.org/latest/topics/life-of-a-task/
