# Context Workspace（Context）

確認日：2026-11-11（JST、日本時間）

一覧と比較の前提は[競合調査](../competitors.md)を参照する。「確認」は公式資料・公開リポジトリに記載があるという意味で、実機検証済みではない。「未確認」は今回の資料で裏付けられなかった項目で、機能がないという意味ではない。

## 概要

| 項目 | 内容 |
|---|---|
| 提供元 | Explore Interfaces Inc.（Delaware法人、所在地San Francisco）。製品名・サイト名はContext。[Terms][terms] |
| 公式URL | [Context Workspace][ws]、Webアプリはworkspace.context.ai。[Workspace][ws] |
| 製品の説明 | すべてのagentが名前を持つメンバーとして参加するチームのworkspace。chat、files、tasks、skills、appletsをまとめ、agentは依頼した人の権限で動く。[Workspace][ws] |
| 提供段階 | Web、デスクトップ（Mac、Windows、Linux）、CLI（Context Code）を提供。モバイル（iPhone、iPad）はPrivate preview。[Workspace][ws] [Desktop][desktop] |
| ライセンス | 未確認。公開リポジトリは見つからなかった。 |
| 価格 | Free $0（"Includes the whole workspace"）、Plus $20（"Per person per month"）、Enterprise（"Talk to us about a plan"、自社クラウドで稼働）。Plusの請求期間、年払いの有無、税の扱いは未確認。[Workspace][ws] |
| 利用量 | モデル利用はcreditsで計量し、上位モデルは単価が高い。各turnの実行前に残高を確認する。planごとに同時に動けるagent数の上限がある。上限の数値は製品内の課金画面に表示するとされ、公開資料では未確認。[self-serve billing][billing] |

## 機能

| 観点 | 内容 | 出典 |
|---|---|---|
| 複数人＋複数AIの同一チャンネル参加 | channel、DM、threadがあり、agent（例：Scout、Ledger、Atlas）をchannelのメンバーとして追加し、メンションして依頼する。agentは追加されたchannelだけを見る。 | [Workspace][ws] [Together][together] |
| AIの役割・専門性の設定 | agentごとのプロフィールに短い役割説明（例：Atlasは"builds decks and board materials"）と、Overview、Skills、Permissions、Platforms、Memory、Activity、Settingsのタブがある。agentの作成手順は今回の資料で未確認。 | [Workspace][ws] [Desktop][desktop] |
| Skills | skillは `SKILL.md` と添付（例：資料テンプレート）を持つフォルダ。範囲（scope）、タグ、connectors、scorecard、番号付き手順を持つ。保存ごとに変更不可の版を作り、古い版の復元も新しい版になる。 | [Workspace][ws] |
| Tasks | Backlog、Idle、Running、Done、Failedのボード。リスト表示はNeeds you、Running、Backlog、Doneで分ける。担当のいないtaskにagentを割り当てるとすぐ始まる。実行中も追加のメッセージを送れる（"it will be handled next"）。taskを人、agent、組織に共有すると会話とfileが渡る。専用の引き継ぎ機能と担当替えの手順は未確認。 | [Workspace][ws] [Tasks][tasks] |
| 実行能力 | Engineがtaskごとにsandboxを用意し、fileの生成、コマンド実行、React/TSXのappletの作成を行う。task実行はTemporal上のdurable workflowで、失敗した手順は再試行される。定期実行はautomations。 | [Workspace][ws] [Engine][engine] |
| 承認・権限管理 | agentは依頼者の資格情報で動き、資格情報のないturnは通知付きで拒否される。Drive file・workspaceの変更、接続アプリの利用、許可リスト外のコマンド・接続は承認を求め、読み取りは既定で許可。承認するのは人で、agentではない。task内では依頼者を待ち、channel内ではthreadで待つ。特定の承認者への振り分けと多段承認は"Not today"。 | [Approvals][approvals] [Workspace][ws] [Tasks][tasks] |
| 外部連携 | Unifyが各人自身のアカウントでアプリへ接続する（Slack、Google Workspace、Salesforce、Notion、Snowflake、GitHub、HubSpot、Linear、Zendesk、Boxなど）。管理者は社内のHTTPまたはMCPサービスをconnectorとして登録できる。toolごとにagentや人に対してBlockedを設定できる。任意のMCP serverを利用者が直接つなぐ機能は未確認。 | [Connectors][connectors] [Workspace][ws] |
| 外部から使うAPI | Contextが自前のMCP serverを提供し、Claude Code、Codex、Cursorなどから利用できる（Freeを含む全workspace）。Task APIはbearer token、MCPはOAuthと同意画面で接続し、ログインした人が開けるものだけに届く。 | [API][api] |
| 評価 | Evalsが各実行を基準で採点し、失敗を次のskill版に反映する。 | [Workspace][ws] |
| 監査 | 全手順を追記のみのログに残し、agent別に絞り込める。承認の要求と判断は、誰がいつ判断したかとともにagentのログに残る。管理者は監査ログを書き出せる。 | [Workspace][ws] [Approvals][approvals] |

## UI/UX

| 要素 | 内容 | 出典 |
|---|---|---|
| 画面構成 | 左のレールにHome、Chat、Tasks、Files、Skills、Applets、Evals。Inboxは中央に一覧、横に会話を並べる。 | [Inbox][inbox] |
| channelでのagentの見え方 | agentは名前と顔（アバター）を持つメンバーで、発言はagentの発言として表示される。メンバー欄はPeopleとAgentsに分かれ、それぞれの人数が出る。agentの状態はIdle、実行中の作業はHomeのtask一覧で"Working"と「Working on…」のバーに出る。追加は検索してAddを押すダイアログ。 | [Together][together] [Workspace][ws] |
| 作業カード | 実行ごとに番号付きの手順（例："Read 14 board packs"）、完了時間（例："Done in 6 min 12 s"）、出力fileのチップ（名前、場所、種類、サイズ）を持つカードを投稿する。長い作業はthread内の1枚のtaskカードを手順ごとに更新し、終了時に閉じる。 | [Together][together] |
| agent間の受け渡し | 例示画面では、人が前のagentの報告のあとに次のagentをメンションして作業を渡し、次のagentの手順に前のagentの成果を読む手順が出る。agent同士が自動で渡す機能は未確認。 | [Together][together] |
| 承認カード | agent名と操作（例："Atlas wants to Change a file in Drive"）、対象のfile path、影響の一文（"This changes a file your whole team can see. If you say no, the file stays as it is."）を表示する。ボタンは"Don't allow"と"Allow once"、一部の要求では"Always allow"も出る。 | [Together][together] [Approvals][approvals] |
| 常時許可の取消 | 常時許可はagentのPermissionsタブに一覧され、範囲を文で示す（例："Use a file in Drive, for the rest of this task"）。外すと再び確認を求める。 | [Workspace][ws] [Approvals][approvals] |
| 判断待ちの集約 | Inboxでは保留中の承認をfeedの上に置き、埋もれないようにする。taskは"Needs you"として一覧の先頭に出て、決まるまで止まる（"nothing runs until someone decides"）。通知ベル、スマホ通知でも届く。 | [Inbox][inbox] [Tasks][tasks] |
| モバイル承認 | time sensitiveな通知で届き、Face IDで本人確認したあとにApprove／Denyを実行する。Web、デスクトップ、Slackからも判断できる。 | [Approvals][approvals] |
| task詳細 | 見出しにOpen in Drive、Files、Activity、Share。本文はagentとの会話、時間付きの手順、出力fileのカード。下に"Ask anything"の入力欄とモデル切替。Computer panelでsandboxのデスクトップ、接続状態、実行時間、コマンドのログ、作業fileを見られる。 | [Tasks][tasks] |
| Inbox | 不在時間の要約（例："You were away for 14 hours"）、All／Mentions／Reactions／Threadsのタブ、行ごとのsnooze（1時間、明朝、来週）。既読とsnoozeは本人だけのもの。 | [Inbox][inbox] |
| デスクトップアプリ | タブで複数のworkspaceやサーバーを横に並べる（Split View）。再起動時にタブと窓を復元。音声入力のショートカット、メンション通知、裏で更新をダウンロード。 | [Desktop][desktop] |

## 技術

公式の製品ページから確認した。公開リポジトリやSDKは見つからず、実装言語・フレームワークは未確認。

| 層 | 技術 | 出典 |
|---|---|---|
| 配置 | Managed by Context、Customer VPC、On-premises、Air-gappedの選択肢。Enterpriseは自社のKubernetes（EKS、AKS）にKOTSで入れ、workspace、sandbox、runはpodとして動く。Free・Plusでsandboxがどこで動くかは未確認。 | [where does Context run][where] [Engine][engine] [Workspace][ws] |
| 実行の分離 | 製品名はSecure VMだが、中身はtaskごとの分離されたcontainer pod。非root、capabilityをすべて落とし、seccomp、cluster service-account tokenなし。任意でpod単位のruntime classを選べる。private rangeへの通信を遮断。 | [Secure VM][securevm] |
| 継続性 | runがidleになるとsandboxを破棄（既定30分、Enterpriseは管理者が設定）。作業fileはobject storageへsnapshotし次のsandboxに戻す。資格情報、依存関係、cacheは残さない。Driveはmountせず、権限確認つきのgateway経由で読み、明示的なpushでだけDriveへ書く。 | [Secure VM][securevm] |
| 通信制御 | sandboxごとにopen、domain allowlist、workspace onlyを選ぶ。allowlistではegress proxyが判断をすべて記録する。空のallowlistは外向き通信をすべて拒否。 | [Secure VM][securevm] [Engine][engine] |
| 資格情報 | sandboxに常設の秘密情報を置かない。認可確認のあとに境界で注入し、保存した秘密は送れるhostを固定して仲介呼び出しで使う。runごとの鍵はrunの終了で失効する。 | [Secure VM][securevm] [Engine][engine] |
| ワークフロー | Temporal（task runはdurable workflow）。 | [Engine][engine] |
| 端末での実行 | Engineの説明では、taskをペアリングしたMac、Windows、Linux端末で実行する選択肢がある。デスクトップアプリの説明は"Agents do their work in a sandbox per task, never on your computer"。両者の関係は未確認。 | [Engine][engine] [Desktop][desktop] |
| モデル | Context 1.5のほか、Anthropic、OpenAI、Gemini、Kimi、DeepSeek、Qwenのモデルと独自endpoint。gatewayにContext Gateway、LiteLLM、Amazon Bedrock、Azure OpenAI、PrivateLink。ZDRが必要な場合、ZDR経路のないモデルは使用不可と表示し、別経路へ回さない。 | [Engine][engine] |
| プロトコル | MCP server（OAuth）を提供。管理者が登録したHTTP・MCPサービスをconnectorとして使う。A2Aへの対応は未確認。APIはHTTPとJSONで、SDKは示されていない。 | [API][api] [Connectors][connectors] |
| 認証 | SAML、OIDCのSSO、SCIM。 | [Workspace][ws] [Engine][engine] |

## Pelwayとの比較［提案］

この節は本書の整理と提案であり、Contextの資料が述べている内容ではない。Pelway側は[ビジョン](../vision.md)、[PRD](../prd.md)、[技術](../technical.md)の構想で、実装済みの機能ではない。

| 観点 | Context | Pelway（構想） | Pelwayでの扱い |
|---|---|---|---|
| 人とAIの同席 | 確認。PeopleとAgentsを分けて表示 | 中核の方針 | 同席だけでは差別化にならない。メンバー欄の分け方は参考にする |
| 外部操作前の承認 | 書き込みは承認カードで止まる。依頼者の権限で動く | 承認を実行内容に結び付ける | 最も近い競合。承認後に内容が変わったときの再承認、承認者の指定（Contextは"Not today"）で差を確かめる |
| 承認者の指定 | task内は依頼者、channelはthreadで待つ。多段承認なし | 「代表の承認が必要」のような会社ごとのルールを想定 | 依頼者と承認者を分ける設計は差を出せる候補 |
| 引き継ぎ・判断待ち・再開 | "Needs you"の集約とtask共有はある。専用の引き継ぎ欄は未確認 | 引き継ぎ欄と判断待ちの表示がMVPの中心 | 「誰の何の判断か」を状態として持つ点を比較の観点にする |
| 実行環境 | taskごとのpod。作業fileはsnapshotで次へ渡す | AI社員ごとの実行環境を検討 | taskごとの破棄とfileのsnapshotは、AI別の常設VMより原価が低い可能性がある。費用比較の対照にする |
| 資格情報の扱い | sandboxに秘密を置かず、境界で注入 | 実行環境に鍵を渡さない方針 | 方針は同じ。差別化の根拠にはしない |
| 作業の見える化 | 手順付きの作業カードとComputer panel | 依頼と成果を会話で追う | 作業カードの手順表示を参考にする |
| 常時許可 | 範囲を文で示し、Permissionsタブで取消 | 外に出る操作は毎回承認 | 常時許可を入れる場合は範囲の文言と取消の場所を参考にする |
| 価格 | Plus $20／人／月、creditsと同時実行数の上限 | 人の参加費とAI実行枠を分ける案 | 同時実行数を上限にする方式を見積もりの比較に使う |

## 未確認事項

- agentの作成手順と、役割・権限の初期設定。
- Plusの請求期間、年払い、税、creditsと同時実行数の具体的な上限。
- 承認後に内容が変わった場合の再承認、承認の期限。
- Free・Plusのsandboxの稼働場所と、端末での実行とsandboxの関係。
- 日本語対応、保存地域、SLA、認証の取得状況。

## 出典

- [Context：Workspace](https://www.context.ai/product/workspace)
- [Context：Together](https://www.context.ai/product/together)
- [Context：Tasks](https://www.context.ai/product/tasks)
- [Context：Inbox](https://www.context.ai/product/inbox)
- [Context：Approvals](https://www.context.ai/product/approvals)
- [Context：Engine](https://www.context.ai/product/engine)
- [Context：Secure VM](https://www.context.ai/product/secure-vm)
- [Context：Connectors](https://www.context.ai/product/connectors)
- [Context：API](https://www.context.ai/product/api)
- [Context：Desktop](https://www.context.ai/desktop)
- [Context Help：Where does Context run](https://www.context.ai/help/deployment/where-does-context-run)
- [Context Help：How does self-serve billing work](https://www.context.ai/help/pricing-and-procurement/how-does-self-serve-billing-work)
- [Context：Terms](https://www.context.ai/terms)

[ws]: https://www.context.ai/product/workspace
[together]: https://www.context.ai/product/together
[tasks]: https://www.context.ai/product/tasks
[inbox]: https://www.context.ai/product/inbox
[approvals]: https://www.context.ai/product/approvals
[engine]: https://www.context.ai/product/engine
[securevm]: https://www.context.ai/product/secure-vm
[connectors]: https://www.context.ai/product/connectors
[api]: https://www.context.ai/product/api
[desktop]: https://www.context.ai/desktop
[where]: https://www.context.ai/help/deployment/where-does-context-run
[billing]: https://www.context.ai/help/pricing-and-procurement/how-does-self-serve-billing-work
[terms]: https://www.context.ai/terms
