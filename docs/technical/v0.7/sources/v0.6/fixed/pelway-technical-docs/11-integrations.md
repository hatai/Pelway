# 11 AIネイティブ連携の共通契約

Claude Code、Cursor、Datadog、Google Workspaceは例示であり限定しない。他のAIエージェントや業務ツールを同じ能力定義とAdapterで追加できる設計とする。今回は文書上の契約とデータモデルだけで、traitやbridgeを実装しない。サービスの接続・登録・公開ingressの準備もしない。

## 共通の単位

`actor`は人、Pelway Bot、外部agent、外部toolの4種。外部主体はintegration、提供元のID、有効化した所有者、依頼者、委任scopeを持つ。サービス名やLLMが名乗った名前だけで認証しない。

`task`に複数の`run`を結び、runに`session`参照、`step`、`artifact`を関連付ける。外部sessionは複数runを含み得るので、外部session IDを全run共通の一意キーにしない。外部job・run・sessionのIDはintegrationと提供元accountで名前空間を分ける。再開で同じsessionを使うか新runを使うかは提供元の契約に合わせる。

`artifact`は国内保存コピーと外部ref、内容hash、version、検査結果、未信頼区分を持つ。外部URLだけを成果物として信頼しない。取得時にも権限、redirect、size、MIME、archive展開、path traversalを検査する。

## IntegrationPortの能力

| 能力/操作案 | 契約 |
|---|---|
| describe_capabilities | submit、observe、resume、cancel、fetch_artifact、read、write等。対応の有無、beta条件、提供元の冪等key/結果照会/scope、source event IDの名前空間と有効期間、重複判定・version/順序・cursor失効時の照合規則を返す |
| describe_data_boundary | 送信先、送信data、処理/保存場所、国外可能性/未確認、region選択、保持/運用条件の説明版 |
| submit | operation ID、承認snapshot、task/run、input artifact、scopeを受け、外部refまたはunknownを返す |
| observe | cursor付きpoll/stream、または署名付きcallbackを正規化。受信を実行命令にしない |
| resume | 外部契約で可能なrun/sessionへの追加指示。新しい外部payloadとして既定で新snapshot/個別送信承認を要する。非冪等steerの自動再送は禁止 |
| cancel | 取消要求を送る。acceptedとstopped_confirmedを別に返す。未対応なら未対応と示す |
| fetch_artifact | 取得許可・内容hash・size/型を検査し国内保存へ取り込む |
| execute_tool | tool名と型付き引数、read/write、副作用の可能性、必要承認を明示 |
| revoke | 新dispatch停止、epoch失効、資格情報broker拒否、外部停止/保持確認の未完を返す |

同じevent keyに違う内容hashは隔離し、黙って重複扱いしない。外部IDから内部runへの帰属は04の登録済対応から決める。

上表はRust traitへ後で落とす設計で、固定コードではない。Adapterが持たない能力はUIで「できる」と表示しない。サービス固有のraw eventは保持できるが、業務状態を直接上書きしない。

権限は、会社ルール、依頼者、Bot役割、task scope、integration grant、接続account、個別承認の共通部分を超えない。grantの初期値はoff。国外性未確認でも、権限ある利用者/管理者が説明を読んで明示有効化できる。提供元のscopeが広い場合でもPelwayの操作範囲を広げない。

## 接続手段の選び方

| 手段 | 主な役割 | 限界と採用案 |
|---|---|---|
| MCP | 型付きtool/resource呼出し | 長期job、sandbox、業務承認の全てを代替しない。rmcpは条件付き依存候補 |
| REST/GraphQL | jobの作成・取得・取消、文書操作 | endpointごとの冪等性・version/ETag・結果照会をAdapterが表す |
| webhook | 進捗/成果/alertのpush | 署名・timestamp/replay確認、inbox永続化後ACK、順不同・再送を扱う。初期は外部公開しない |
| SSE/polling | 長時間jobの観測 | cursor期限切れや切断で再submitしない。状態照会へ戻る |
| CLIとlocal bridge | 国内hostまたは別審査した利用者端末のagentとの入出力 | shell文字列を連結せず引数を型付け。bridge認証・OS権限・作業directoryを限定 |
| hooks | lifecycle進捗、tool実行前後の通知 | 設定によりfail-openもあり得る。hookだけを強制承認境界にしない |
| A2A | agent間task/context/artifactの委任 | 参考候補。社内権限・秘密・国内性・exactly-onceの保証ではない |

local bridgeと同じ端末のCLIとの接続は、loopbackの認証済みIPCまたはstdioを使う設計。管理hostと別端末のbridgeの接続/認証方式は将来の未決で、localhost APIをそのまま外へ公開しない。国外端末への接続許可を含まない。本人のCLIログインをBotへ複製しない。任意コマンドを外部イベントから起動しない。利用者PCでの実装と環境変更は今回範囲外。

## 公式手段の確認例

確認日は2026-11-08（日本時間）。以下の「今可能」は公式に公開手段があるという意味で、Pelway接続済みの意味ではない。国外性の情報は利用者の判断材料で、所在地だけで有効化を拒否しない。

| 例 | 今可能/制約/未確認 | データと責任分界の確認 |
|---|---|---|
| Claude Code | [headless CLI](https://code.claude.com/docs/en/headless)のprint/stream-json/session resume、[hooks](https://code.claude.com/docs/en/hooks)、MCP。Slack連携のプラン・契約差は利用時点で確認 | prompt、context、repoが提供元へ出る可能性。cloud sessionの処理国/保持は未確認。CLIというだけで国内推論とはしない |
| Cursor | [hooks](https://cursor.com/docs/hooks)、[Slack等](https://cursor.com/docs/integrations/slack)、[Cloud Agents API](https://cursor.com/docs/cloud-agent/api/endpoints)。調査時v1はpublic beta、SSE/取得/取消。v1 webhookは将来案との記載、旧v0と分ける | cloud/self-hostedの実行と管理plane/LLMを分ける。self-hostedを国内完結としない。prompt/repo/環境/成果物の保存先は要確認 |
| Datadog | [MCP](https://docs.datadoghq.com/mcp_server.md)、REST、webhook等。toolsetと権限、betaの範囲は個別確認 | log/query/incident、tool引数が送られる。[site](https://docs.datadoghq.com/getting_started/site.md)と全AI/運用地域を同一視しない。利用情報の収集を説明 |
| Google Workspace | Chat/Drive/Docs/Gmail公式API、[remote MCP](https://developers.google.com/workspace/guides/configure-mcp-servers)は調査時Developer Preview条件、[Events](https://developers.google.com/workspace/events/guides/create-subscription)はPub/Subと製品別制約 | 文書・検索語・mail・tool引数、OAuth scopeを説明。[data regions](https://knowledge.workspace.google.com/admin/compliance/choose-a-geographic-location-for-your-data)の対象製品/editionと処理を確認。Cloud TokyoをWorkspace全体へ外挿しない |

詳細な公式出典と利用者判断欄は [市場原稿](research/existing-chat-and-agent-oss.md) の連携節を正とする。存在未確認のAPIを将来の必須依存にしない。

## 受信、再接続、重複、取消

イベントの共通封筒案は `event_id, integration_id, source_event_id, external_account, task_id, run_id, external_session_id, provider_seq/cursor, subject_version, occurred_at, received_at, type, payload_ref, trust_level`。秘密は入れない。

署名検証後にinboxへ保存し、そのtransactionの成功後にACKする。unique(workspace,integration,external_account,source_event_id)で重複を除く。送信側にevent IDがない場合、payload hashだけでは同じ内容の正当な複数eventも潰してしまう。Adapterごとにresource/version等と組み合わせる規則を決め、根拠がなければ重複と断定せず照会する。

受信順の番号はPelway内の記録順であり、外部の因果順ではない。provider cursor/versionがあれば使用する。なければ取得APIで照合し、古い進捗でterminal結果を戻さない。再接続だけでは結果不明にしない。結果不明は外部副作用の実行結果が確定しない場合に使う。

dispatchのoutbox再試行と外部副作用の再送を分ける。送信した可能性のあるoperationは新IDで勝手に再submitしない。照会証拠を人へ示す。duplicate eventは業務表示と業務監査を二重にせず、受信/replayの診断件数は別に残す。

連携取消で新dispatchとtoken利用を止める。外部停止未確認なら理由「停止確認中」を添え、作業中なら停止確認まで作業中を保持する追加提案に合わせる。取消と完了が競合した場合、遅れて来た成果物を『取消要求後に到着』として保存し、自動採用・送信しない。既に送ったデータや提供元の保持を取消で消せると保証しない。

## 業務状態と外部実行の対応（追加提案・要確認）

9状態は増やさず、ActionSnapshotの固定項目に`action_kind`（04のcustomer_send / agent_handoff / tool_read / tool_write）を付ける案。runの提供元状態と業務状態は別に保持する。次表の非顧客操作を作業中へ戻す遷移は、PRD未承認の追加で、今回PRDを書き換えない。

| 外部との往復 | 業務状態 | 条件 |
|---|---|---|
| 人が依頼/範囲を調整 | 依頼済み→作業中→必要なら判断待ち | 監視alertだけでは自動受付を始めない |
| 外へ送る入力/範囲を固定 | 送信承認待ち | grantの有効化とは別に個別snapshotを承認 |
| gatewayが実行を予約/試行 | 送信中 | 03の直前再確認とattempt_started境界。返事なしは結果不明 |
| agent委任/tool操作の受理確認 | 作業中 | 外部jobが終わった意味ではない。03の追加提案 |
| 進捗/成果物/失敗を取込 | 作業中／判断待ち／止まっている＋理由 | 人の判断が必要なら判断待ち。障害時は理由を付ける |
| 人が成果/差分を採用 | 作業中→完了、または修正の次runへ | 必要な確認なしに外部doneだけで完了にしない |
| 顧客向け送信の成功 | 送信済み・顧客回答待ち | この表示は顧客への送信だけに使う。外部メールは利用可否未決 |
| 取消要求/連携失効 | 03の未実行時取消、または元状態を保ち結果照会 | sending/outcome_unknownの副作用結果を隠さない。外部停止が未確認なら停止確認中 |

入力artifact/範囲を変える次runは新承認。単なる観測の再接続は再submitしない。resume/追加promptを含む新しい外部payloadは、既定で新snapshotと個別送信承認を必須とする。将来承認を省略できる条件だけを08の未決に残す。今は省略条件を定義しない。副作用の再試行は03の証拠/再承認条件も満たす。

## 表示契約

入力は[input/ui.md](input/ui.md)の§8。外観を変えず、既存部品の情報表示だけを揃える。

- 送信承認待ち/送信中の横には、ActionSnapshot由来の操作の種類と連携名を文字で出す（例：送信承認待ち（外部への受け渡し：Cursor））。判断待ち一覧でも顧客送信と区別し、種類の翻訳キー/並び替えに同じ値を使う。
- 有効中の外部連携は、連携一覧と会話の主体表示の両方に「国内限定の外」の文字ラベルを常に付ける。日本単一の配置方針に結び付け、画面ごとに直書きしない。このラベルは「Pelway本体の国内限定の保証範囲外。国外処理・保存の可能性がある」という意味の説明を併記する。国外設置の断定でも、利用禁止の表示でもない。日本語/英語の文言は[UI v0.3の確認済み案](reviews/ui-v0.3.md)を用いる。説明は `boundary-selected-external-desc` として、日「Pelway本体の国内限定の範囲外です。送り先で国外処理・保存される可能性があります（未確認の場合を含む）。」、英「Outside Pelway's domestic-only scope. The destination may process or store data outside Japan, or this has not been confirmed.」とする。外観は変えない。
- 外部由来の本文/logは「外部由来・未確認」の帯を付け、Pelwayの操作/承認欄と区別する。本文に書かれた『ボタン』や指示を操作にしない。
- 人／Bot／外部エージェント／外部ツールを文字ラベルでも区別する。色だけに頼らず、読み上げには種類と外部連携名を含める。
- 本体画面に「国内で完結」「準拠」「適合」と表示しない。配置先は所在の制御であり、法適合や外部サービスまでの保証を意味しない。
- 有効化/個別承認時に送り先・データ・国外処理/保存の可能性/未確認・利用者の責任を表示する。有効化だけで操作権限を広げず、制約されている能力は制約も表示する。

日英の翻訳/読み上げ条件は06、表示の合成データ試験は10のD9へ対応させる。

## 外部agentの権限を過信しない

外部agentの出力・文書・logは未信頼データ。命令、secret要求、URL、tool追加をそのまま実行しない。承認UIはPelway側の型付き操作から作り、本文内の偽のボタンや指示を操作にしない。

外部の自律agentが提供元内で直接push・送信・削除できる場合、Pelway gatewayだけでは各副作用を強制承認できない。初期の委任scopeは読取/隔離作業/差分返却に限定し、外部更新権限を渡さない案。提供元側の権限で制約できないAdapterは、その能力を安全条件付きで使えるとは表示しない。利用者責任の国外処理許可は、個別操作承認を無視する許可ではない。

## 三つの往復例

- コード作業は、人のメンション（依頼済み）から入力/scopeを調整（作業中）、送信承認待ちで人が固定入力を確認、送信中を経て受理確認で作業中へ戻す。進捗/patchを取り込み、人がdiffを判断（判断待ち）、採用後に完了、または修正を次run（作業中→新たな送信承認待ち）へ結ぶ。push/PR/mergeは別の個別操作承認。外部停止未確認では停止済みとしない。
- 監視調査は、alertをinboxに保存するだけでは業務taskを開始せず、人のメンション（依頼済み）で受付。検索範囲を固定（作業中→送信承認待ち）、外部toolを呼ぶ（送信中→受理確認後作業中）。log/調査メモを未確認artifactで返し（判断待ち）、人が判断して朝の担当へ引継ぐ（作業中）。メンションなし自動受付は将来の未決。
- 文書は、人の依頼（依頼済み）から検索入力の承認（作業中→送信承認待ち）を経てtool呼出（送信中→作業中）。版付き下書きを返し（判断待ち）、文書ID/既存revision/ETag/差分を固定して人が更新を承認（送信承認待ち）、更新確認（送信中→作業中）と必要な人の確認を経て完了。外部版が変われば適用せず判断待ちで再取得/再承認。共有設定変更や送付は別操作で、外部メール許可の未決を回避しない。

## 今と後の範囲

今は上記の能力定義・関連ID・policy・表示契約・模擬試験を文書にする。後でAdapter、bridge、callback公開、資格情報brokerの実装と各サービス契約を審査する。4製品の網羅調査や実装は今回増やさない。

合成データで、duplicate/out-of-order、cursor失効、無応答、失効grant、停止未確認、偽の外部指示、scopeを超えるartifact取得、同時承認を試す計画。連携工数は未見積もりで既存人時へ加算しない。

共通契約はPlacementPolicyの版とintegration grant版を記録する。現在の配置はJP-localのみ。将来の地域変更をAdapterの自動fallbackとして実装しない。

## 入力一式・実行区画・資格情報の制約

外部agentへは承認した不変のexport一式だけを渡す。ファイルmanifestと内容hash、repoのcommit/tree、含む/含まないパス、prompt、session所有task/履歴範囲をsnapshotへ結び付ける。作業directory名だけを許可にしない。変更可能なhost directoryや別taskの過去sessionを暗黙に読ませず、初期案ではtaskをまたぐsession再利用をしない。入力以外のファイルを読めない制約が提供元側で確認できない能力は、使えるとはしない。

CLI型agentはhostへ直実行せず、本体DB/volume/secret/runtime socketへ届かない専用の連携実行区画を必要とする。出口は承認した当該提供元/操作だけ、入力mountは上記exportだけ。coreの外部出口なしsandboxの設定を緩めて代用しない。CLIの認証主体・資格情報方式・OSごとの区画方式は未決で、今回作らない。制御不能なCLIにhost権限や本人の既存sessionを渡す実装は採らない。

Adapterは監査/固定版確認を要する信頼側の部品とし、追加しただけでtrusted codeや出口権限を自動付与しない。Adapterは型付きの固定要求をgatewayへ返し、brokerのrefresh tokenを受け取らない。実HTTP transportはgateway側でpayload hash/account/path/data分類を再確認し、brokerが資格情報を付与する。提供元のtokenをscope限定できない場合でも、Bot/モデル/無権限プラグインへそのtokenを渡さない。Adapter自身の任意ネットワーク出口を認めず、連携ごとの許可済みtransportを経由する。これを満たせるかはAdapter採用試験で確認する。

有効化は全章で『integration.enableの対象scope権限を持つ利用者または管理者』に統一する。閲覧できるだけで外部送出を許さず、会社が許可したexportデータ分類・依頼者の閲覧範囲・task scopeの共通部分だけを送れる。会社単位のデータ分類/例外方針の設定は管理者の権限とし、有効化した人の同意を他者/顧客データ全体の持出し許可とみなさない。

## 取消後の限定確認

通常grant取消は新submit/更新と通常broker利用を止める。既存operationだけの停止要求/読取照会を継続するかは別の限定権限として明示同意を得る設計案で、通常grantを黙って残さない。限定機能を分離できない/全面資格情報取消の場合は自動確認不能とし、人が提供元画面等で証拠を確認する。証拠なしの『送れなかった』判断で再試行しない。資格情報を戻したり別accountで迂回したりしない。提供元で止められたか/データを消せたかは別結果として残す。

PlacementPolicyの版・送出データ分類・宛先・必要権限が変わればgrantと未実行承認の適合を再検査する。新しい版を旧同意で自動有効にせず、影響するgrantは再同意まで停止する。将来のリージョン切替機能を追加した意味ではない。


### 実送出の有効化と設定上の同意の区別

本章の明示有効化は将来の設定/権限条件で、実接続の着手許可や安全試験の合格ではない。[08のS1](08-decisions-open.md)は未解決で、実外部I/Oは無効のまま。国内性が未確認だから禁止するのではなく、取消の効力発生点の正式判断と送出制御の検証等が未達のためである。模擬経路の期待値は実接続試験の実施を意味しない。
