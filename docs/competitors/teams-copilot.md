# Microsoft Teams / Microsoft 365 Copilot・Copilot Studio（Microsoft）

確認日：2026-11-11（JST、日本時間）

一覧と比較の前提は[競合調査](../competitors.md)を参照する。「確認」は公式資料・公開リポジトリに記載があるという意味で、実機検証済みではない。「未確認」は今回の資料で裏付けられなかった項目で、機能がないという意味ではない。

本書は次の経路を分けて扱う。Copilot Studioで作りTeamsに公開するエージェント、Teams標準のChannel AgentとFacilitator、Microsoft 365 Copilotの宣言型エージェント（declarative agent）、Teamsアプリとして作るエージェント（bot）。ある経路の仕様を、他の経路やMicrosoft製品全体に当てはめない。

## 概要

| 項目 | 内容 |
|---|---|
| 提供元 | Microsoft。 |
| 公式URL | [Copilot Studio][studio]、[Microsoft 365 Copilot料金][msprice]、[Teamsへの公開手順][tmpub]、[Teamsのエージェントに関するFAQ][tmfaq] |
| 製品の説明 | Copilot Studioはエージェントを作る製品で、会話型と自律型のエージェント、複数エージェントの連携、データとシステムへの接続を説明する。作ったエージェントはTeamsとMicrosoft Copilotに公開でき、チームのチャンネル、グループチャット、会議チャットに追加できる。[Copilot Studio][studio] [公開手順][tmpub] |
| 提供段階 | Channel Agentはパブリックプレビュー。Copilot Studio全体の提供段階の表記は未確認。[Channel Agentの設定][chagent] |
| ライセンス | プロプライエタリ（クローズドな商用サービス）。開発用のTeams SDK（TypeScript）とMicrosoft 365 Agents SDKのリポジトリはMITライセンス。[teams.ts][teamsts] [microsoft/Agents][m365agents] |
| 価格 | Microsoft 365 Copilot（add-on）は$30.00 user/month, paid yearly（Annual subscription–auto renews）。月払いの選択肢は表示されるが金額は未確認。対象のMicrosoft 365プランが別に必要。Copilot Chatは対象Microsoft 365プランのMicrosoft Entraアカウント利用者に追加費用なしで、エージェントの利用は従量課金。エージェントの利用にはAzureサブスクリプションが必要。Copilot Studioは25,000 Copilot Creditsのパックが$200.00/pack/month、または従量課金。Microsoft 365 Copilotのライセンス利用者は、Microsoft 365 Copilotに公開したエージェントの利用がライセンスに含まれる。[Copilot料金][msprice] [Copilot Studio][studio] |

## 機能

| 観点 | 内容 | 出典 |
|---|---|---|
| 複数人＋複数AIの同一チャンネル参加 | Copilot Studioのエージェントは、チームに追加すればチームのどのチャンネルでもメンションでき、返答はチーム全員に見える。グループチャットと会議チャットにも追加できる。Teamsアプリのエージェントは、既定ではメンションされたメッセージだけを受け取る。RSC（resource-specific consent）で全メッセージを受け取れるが、他のエージェントやbotが書いたメッセージは配信されない。 | [公開手順][tmpub] [channel・group会話][tmbots] |
| Channel Agent | チャンネルごとに作るエージェントで、チャンネルの名前とアイコンを使う。会話と会議を知識にし、状況報告の作成、会議の予定、タスク管理、質問への回答をする。1チャンネルに1つだけで、作ったチャンネル以外には追加できない。プライベートチャンネル、外部参加者のいる会話、Customer Keyを使うテナントでは使えない。US英語以外は公式サポート外。 | [FAQ][tmfaq] [Channel Agentの設定][chagent] |
| AIの役割・専門性の設定 | Copilot Studioで、名前、説明、指示、トリガー、知識、ツール、推奨プロンプトを設定する。複数エージェントの連携で、専門知識や操作が必要な作業を適したエージェントに振り分ける。 | [作成手順][create] [Copilot Studio][studio] |
| 実行能力 | 1,500以上のコネクタ、MCPサーバー、ワークフロー自動化（agent flows）。AI別のVMや画面操作は未確認。 | [Copilot Studio][studio] |
| 承認・権限管理 | 組織全体に配るには管理者の承認を受ける。承認後の内容更新は再承認不要で、エージェントの詳細を変えたときだけ再申請する。チームやグループチャットへの追加は作成者が設定で許可・禁止できる。グループチャットとチャンネルでは、利用者認証が必要な知識（SharePointなど）を使えず、それらは1対1チャットだけで使える。 | [公開手順][tmpub] |
| 外部操作の確認（宣言型エージェント） | Microsoft 365 CopilotのMCP・APIプラグインは、最初の利用時に許可かキャンセルを尋ねる。以後、データを読むGETは確認なし、それ以外は送るデータを示して許可・拒否を尋ねる。開発者は`readOnlyHint`や`x-openai-isConsequential`で確認の有無を変え、確認文を変更できる。この仕様は宣言型エージェントのプラグインの説明で、Teamsに公開したCopilot Studioのエージェントにも同じ方式が適用されるかは未確認。 | [確認プロンプト][tmconfirm] |
| 人への問い合わせ（agent flows） | Request for informationアクションでフローを止め、指定した人に入力を求めてから続きを実行する。依頼はOutlookだけで送られ、最初に回答した人の内容が使われる。 | [Request for information][rfi] |
| 外部連携 | Copilot StudioはStreamableトランスポートのMCPサーバーに接続する。MCPはPower Platformのコネクタ経由なので、コネクタを規制するデータポリシーがMCPサーバーにも適用される。A2A（Agent2Agent）プロトコルで外部のエージェントに作業を任せられる。 | [MCP接続][tmmcp] [A2A][a2a] |
| 監査・運用 | エージェントのやり取りとセキュリティ上の出来事をMicrosoft Purviewの監査ログに送る。バージョン管理、リリースパイプライン、承認ゲートを持つ開発・本番の管理。利用者グループごとのクレジット予算と上限。 | [Copilot Studio][studio] |

## UI/UX

| 要素 | 内容 | 出典 |
|---|---|---|
| エージェントの追加 | チャンネル右上の「Add people, agents and bots」から追加する。Channel Agentがまだないチャンネルでは、追加を促すバナーが出る。会議では右上の参加者一覧から直接エージェントを呼べる。 | [FAQ][tmfaq] |
| チャンネルでのAI | エージェントは名前でメンションして呼ぶ。開発ガイドは、チームに追加されたときの歓迎メッセージを個別に送らず、その会話に投稿して追加した人をメンションするよう推奨する。プライベートチャンネルでは、エージェントはメッセージもAdaptive Cardも投稿できない。 | [FAQ][tmfaq] [channel・group会話][tmbots] |
| 見えない情報の扱い | Channel Agentは、質問者が見られないファイルの内容を回答に使わない。質問者は見られるが他のメンバーが見られない場合は、全員に共有するか本人に確認し、本人が回答を見てから共有するか決める。 | [Channel Agentの設定][chagent] |
| 状況報告と承認者 | Channel Agentの状況報告はLoopで共有する。報告の承認者と、作成する時刻・頻度・日付を設定できる。 | [FAQ][tmfaq] [Channel Agentの設定][chagent] |
| 会議のAI | Facilitatorは会議中にAIメモを作り、議題の進み具合をチェックマークで示し、決定事項と未解決の質問をまとめる。メモはFacilitatorではなく参加者の名義になる。Facilitatorは設定を変更できない。 | [FAQ][tmfaq] |
| 確認画面（宣言型エージェント） | 確認の種類に`AdaptiveCard`を指定し、タイトルと本文を設定する例がある。確認文は多言語化できる。 | [確認プロンプト][tmconfirm] |
| エージェント作成 | Copilot Studioのホームで、やってほしいことを1,024文字以内で書くと、AIが名前、説明、指示を作り、トリガー、チャネル、知識、ツールを提案する。提案は採用・無視できる。テスト用のチャットで挙動を確かめながら直す。 | [作成手順][create] |
| 公開の流れ | Teams and Microsoft Copilotのチャネル設定で外観を変え、自分用のインストール、リンク共有、管理者承認の申請を選ぶ。承認されると状態が「Approved」になる。 | [公開手順][tmpub] |

## 技術

Teams、Copilot、Copilot Studioの本体はクローズドなので、公式ドキュメントに書かれた範囲だけを記す。オープンソースの開発用SDKは区別して記す。

| 層 | 技術 | 出典 |
|---|---|---|
| エージェント間・外部連携 | MCP（Streamableトランスポート）、A2A（エージェントカードを`/.well-known/agent.json`から読む）、Microsoft 365 Agents SDKで作ったエージェントはActivity Protocolでつなぐ。 | [MCP接続][tmmcp] [A2A][a2a] |
| Teams上の会話 | メンションの実体（mention entity）、RSCによる全メッセージ受信、Adaptive Cards。 | [channel・group会話][tmbots] |
| 確認の制御 | MCPのツール注釈`readOnlyHint`、OpenAPIの`x-openai-isConsequential`、プラグインマニフェストの`confirmation`。 | [確認プロンプト][tmconfirm] |
| 開発用SDK（オープンソース） | Teams SDK（`microsoft/teams.ts`）：TypeScript、MIT。`@microsoft/teams.apps`の依存は`express`、`@azure/msal-node`、`@microsoft/signalr`、`@opentelemetry/api`、`jsonwebtoken`など。Microsoft 365 Agents SDKのリポジトリ（`microsoft/Agents`）はMITで、文書とサンプルを置く。 | [teams.ts][teamsts] [teams.apps package.json][teamsapps] [microsoft/Agents][m365agents] |
| データの扱い | エージェントをTeamsに追加すると、エージェントの内容や利用者のチャット内容をTeamsと共有し、組織のコンプライアンスや地域の境界の外に出る場合があると注意書きがある。Channel Agentのプレビューのデータ処理はEU Data Boundaryの外で行われる場合がある。 | [公開手順][tmpub] [FAQ][tmfaq] |
| サーバー構成・データ保存 | 未確認。 | — |

## Pelwayとの比較［提案］

この節は本書の整理と提案であり、Microsoftの資料が述べている内容ではない。Pelway側は[ビジョン](../vision.md)、[PRD](../prd.md)、[技術](../technical.md)の構想で、実装済みの機能ではない。

| 観点 | Teams / Copilot | Pelway（構想） | Pelwayでの扱い |
|---|---|---|---|
| 人とAIの同席 | 確認。Channel Agentは1チャンネルに1つ。Teamsアプリのエージェントには他のbotのメッセージが届かない | 複数の専門AIが同じチャンネルで協業 | AI同士の受け渡しを同じチャンネルで見せる点を比較の観点にする |
| 外部操作前の承認 | 宣言型エージェントは書き込み操作で送るデータを示して許可を求める。他の経路は未確認 | 承認を実行内容に結び付ける | 「送るデータを見せて許可を求める」形を参考にし、承認後の内容変更で承認を無効にする |
| 人への問い合わせ | agent flowsはOutlookで人に入力を求め、最初の回答を使う | 判断待ちを会話の中で扱う | 会話の外に出さず、誰の判断かを固定する |
| 見えない情報の扱い | Channel Agentは共有前に本人に確認する | 企業間分離と情報管理が未決事項 | 共有前の本人確認を参考にする |
| 状況報告 | Channel AgentがLoopで報告し、承認者を設定できる | 引き継ぎ欄に決定と残作業を残す | 報告に承認者を付ける形を検討材料にする |
| 導入の重さ | Microsoft 365、Teams、Copilotのライセンス、管理者承認、言語の制限 | 独立サービス、日本語の小規模チーム | 日本語で使える点と導入の軽さを比較の観点にする |

## 未確認事項

- Teamsに公開したCopilot Studioのエージェントが、外部への書き込み前に確認画面を出すか。
- Copilot Studioのagent flowsで、Outlook以外（Teamsなど）で人に承認を求められるか。
- 同じチャンネルにある複数のエージェントが、互いの返答を受け取って作業を渡せるか。
- Microsoft 365 Copilotの月払いの金額。
- エージェントの実行環境の分離単位、日本語での動作。

## 出典

- [Microsoft：Copilot Studio製品ページ・料金FAQ](https://www.microsoft.com/en-us/microsoft-copilot/microsoft-copilot-studio)
- [Microsoft：Microsoft 365 Copilot enterprise pricing](https://www.microsoft.com/en-us/microsoft-365-copilot/pricing/enterprise)
- [Microsoft Learn：Connect and configure an agent for Teams and Microsoft Copilot](https://learn.microsoft.com/en-us/microsoft-copilot-studio/publication-add-bot-to-microsoft-teams)
- [Microsoft Learn：Channel and Group Conversation Chat for Agents](https://learn.microsoft.com/en-us/microsoftteams/platform/bots/how-to/conversations/channel-and-group-conversations)
- [Microsoft Support：Frequently asked questions about agents in Microsoft Teams](https://support.microsoft.com/en-us/office/frequently-asked-questions-about-agents-in-microsoft-teams-f7317b78-fd53-4cfe-88f0-f0a0751a4150)
- [Microsoft Learn：Set up a Channel Agent for Microsoft Teams](https://learn.microsoft.com/en-us/microsoftteams/set-up-channel-agent-teams)
- [Microsoft Learn：Confirmation prompts for MCP and API plugins](https://learn.microsoft.com/en-us/microsoft-365/copilot/extensibility/plugin-confirmation-prompts)
- [Microsoft Learn：Request information from human review in agent flows](https://learn.microsoft.com/en-us/microsoft-copilot-studio/flows-request-for-information)
- [Microsoft Learn：Connect your agent to an existing Model Context Protocol (MCP) server](https://learn.microsoft.com/en-us/microsoft-copilot-studio/mcp-add-existing-server-to-agent)
- [Microsoft Learn：Connect an agent available over the Agent2Agent (A2A) protocol](https://learn.microsoft.com/en-us/microsoft-copilot-studio/add-agent-agent-to-agent)
- [Microsoft Learn：Quickstart: Create and deploy an agent with the standard harness](https://learn.microsoft.com/en-us/microsoft-copilot-studio/fundamentals-get-started)
- [Microsoft：microsoft/teams.ts](https://github.com/microsoft/teams.ts)
- [Microsoft：teams.ts packages/apps/package.json](https://github.com/microsoft/teams.ts/blob/main/packages/apps/package.json)
- [Microsoft：microsoft/Agents](https://github.com/microsoft/Agents)

[studio]: https://www.microsoft.com/en-us/microsoft-copilot/microsoft-copilot-studio
[msprice]: https://www.microsoft.com/en-us/microsoft-365-copilot/pricing/enterprise
[tmpub]: https://learn.microsoft.com/en-us/microsoft-copilot-studio/publication-add-bot-to-microsoft-teams
[tmbots]: https://learn.microsoft.com/en-us/microsoftteams/platform/bots/how-to/conversations/channel-and-group-conversations
[tmfaq]: https://support.microsoft.com/en-us/office/frequently-asked-questions-about-agents-in-microsoft-teams-f7317b78-fd53-4cfe-88f0-f0a0751a4150
[chagent]: https://learn.microsoft.com/en-us/microsoftteams/set-up-channel-agent-teams
[tmconfirm]: https://learn.microsoft.com/en-us/microsoft-365/copilot/extensibility/plugin-confirmation-prompts
[rfi]: https://learn.microsoft.com/en-us/microsoft-copilot-studio/flows-request-for-information
[tmmcp]: https://learn.microsoft.com/en-us/microsoft-copilot-studio/mcp-add-existing-server-to-agent
[a2a]: https://learn.microsoft.com/en-us/microsoft-copilot-studio/add-agent-agent-to-agent
[create]: https://learn.microsoft.com/en-us/microsoft-copilot-studio/fundamentals-get-started
[teamsts]: https://github.com/microsoft/teams.ts
[teamsapps]: https://github.com/microsoft/teams.ts/blob/main/packages/apps/package.json
[m365agents]: https://github.com/microsoft/Agents
