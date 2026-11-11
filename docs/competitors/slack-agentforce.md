# Slack / Agentforce（Salesforce）

確認日：2026-11-11（JST、日本時間）

一覧と比較の前提は[競合調査](../competitors.md)を参照する。「確認」は公式資料・公開リポジトリに記載があるという意味で、実機検証済みではない。「未確認」は今回の資料で裏付けられなかった項目で、機能がないという意味ではない。

本書は次の3つを分けて扱う。Salesforceで作りSlackに追加するAgentforceのエージェント、Slack標準のSlackbot（個人向けAIエージェント）、Slackアプリとして第三者や社内が作るエージェント。Slack Developer Docsの設計ガイドは、エージェントを作る開発者向けの推奨であり、Slack上の全エージェントの挙動を説明したものではない。

## 概要

| 項目 | 内容 |
|---|---|
| 提供元 | Slack Technologies, LLC（Salesforceの企業）。[Slackヘルプ][afslack] |
| 公式URL | [Agentforce in Slackを使用する][afslack]、[Agentforce in Slackをセットアップして管理する][afsetup]、[AI in Slack（開発者向け）][devai] |
| 製品の説明 | Agentforceは、組織が業務の特定の側面を処理するカスタムエージェントを作る製品。Salesforceで作ったエージェントをSlackに追加すると、メンバーはエージェントタブやチャンネルでやり取りできる。Slackのエージェントは社内の担当者が作り、ITヘルプや福利厚生の案内など特定の目的に合わせて設計される。[セットアップ][afsetup] [利用ガイド][afslack] |
| 提供段階 | 一般提供の段階表記は未確認。 |
| ライセンス | プロプライエタリ（クローズドな商用サービス）。開発用のBolt for JavaScript／PythonはMITライセンス。[bolt-js][bolt] [bolt-python][boltpy] |
| 価格 | Slack Pro $8.75 USD/User/Month（Billed monthly）、Business+ $18 USD/User/Month（Billed monthly）、Enterprise+はContact sales。年払いは約16%安いとFAQに記載。Agentforce in Slackは、Slackのすべてのプランで、Salesforce Agentforceのライセンスを割り当てられたメンバーが使える。Agentforce User Licenseは$5 USD/User/Month（Flex Creditsが必要）、Flex Creditsは$500 USD/100k Credits（1アクション20 credits）。未使用のFlex Creditsは次の契約期間に繰り越さない。Slack用の単独価格は料金ページに未記載。[Slack料金][slprice] [Agentforce料金][afprice] [利用ガイド][afslack] |

## 機能

| 観点 | 内容 | 出典 |
|---|---|---|
| 複数人＋複数AIの同一チャンネル参加 | Agentforceのエージェントをチャンネルに追加し、チームメイトと同じようにメンションして使う。AIエージェントを搭載したアプリもチャンネルに追加でき、「エージェントとツール」タブでマルチプレーヤーAIの会話の状態を確認できる。同じチャンネルの複数エージェント同士が協働する挙動は未確認。 | [利用ガイド][afslack] [AIエージェントを利用する][aiapps] |
| AIの役割・専門性の設定 | SalesforceのAgentforce Builderで作る。Slack用テンプレートは3つ（カスタマーインサイト、従業員サポート、研修）。サブエージェントとアクションで範囲と専門知識を決め、Slackアクション（canvas作成、検索など）を追加できる。Slack用は従業員エージェントのタイプでのみ作れる。 | [セットアップ][afsetup] |
| 実行能力 | Agentforce側はSalesforceのアクションとFlowで業務を実行し、Apexの`InvocableMethod`からSlack Web APIを呼ぶカスタムSlackアクションも作れる。エージェントに「タスク」タブで定期実行タスクを設定できる。AI別のVMや画面操作は未確認。 | [カスタムSlackアクション][afactions] [AIエージェントを利用する][aiapps] |
| 承認・権限管理 | エージェントはSlackとSalesforceの権限で回答を作り、依頼者がアクセスできない情報には返信しない。Slackのオーナー・管理者がエージェントに割り当てるSlack内の権限を確認してインストールする。従業員エージェントの利用権はSalesforceの権限から自動で割り当てられ、サービスエージェントは管理者が割り当てるまで誰も使えない。オーナー・管理者はアプリの承認を有効にしてインストール前に審査できる。ゲストはAIアプリとエージェントを使えない。Agentforceの各アクションを実行前に人が承認する仕組みは未確認。 | [利用ガイド][afslack] [セットアップ][afsetup] [AIエージェントを利用する][aiapps] |
| 外部連携 | Slack MCP Serverは、MCPクライアントからSlackの検索、メッセージ送信、canvas管理などを行う。管理者はMCPクライアントを通常のアプリ承認で管理する。Slackbot MCP ClientはリモートのMCPサーバーをSlackにつなぎ、Slackbotがツールを見つけて会話中に呼び出す。Agentforceのエージェント経路でMCPを使う仕様は未確認。 | [Slack MCP Server][mcpserver] [Slackbot MCP Client][mcpclient] |
| 外部操作の確認 | Slackbot MCP Clientのトラブルシューティングに、分類情報がないツールは書き込み扱いになり、読み取りだけのツールでも追加の確認が出ることがある、と記載。確認画面の内容は未確認。 | [Slackbot MCP Client][mcpclient] |
| プロンプトの共有 | 使ったエージェントのプロンプトのリンクをコピーして、チャンネル、canvas、ワークフローで共有できる。共有プロンプトを選んだ人には、その人に合わせた回答が表示される。 | [利用ガイド][afslack] |
| 監査 | Slack開発者ガイドは、アクセスしたチャンネル、実行したアクション、依頼者、使ったモデルの記録を推奨し、管理者レベルの変更はAudit Logs APIで残すよう案内する。Agentforce自体の監査機能は未確認。 | [governance][gov] |

## UI/UX

| 要素 | 内容 | 出典 |
|---|---|---|
| 画面構成 | デスクトップのサイドバーに「エージェントとツール」タブ。人気のエージェント、インストール済みエージェント、AgentExchangeを表示する。Slackbotを含むすべてのエージェントセッションがタブのサイドバーに並ぶ。Agentforceは「エージェント」から選び、「過去のスレッド」で以前の会話を開く。モバイルは画面下部の「Agentforce」から開く。 | [AIエージェントを利用する][aiapps] [利用ガイド][afslack] |
| 分割ビュー | 右上の「エージェントを追加する」から、チャンネルを見たまま横のパネルでエージェントと会話できる。メインビューで開くこともできる。 | [AIエージェントを利用する][aiapps] [Agent design][design] |
| チャンネルでのAI | エージェントをチャンネルに追加し、メッセージでメンションする。設計ガイドは、チャンネルでの返答をスレッドに置き、関連する通知はまとめて1件にするよう推奨する。 | [利用ガイド][afslack] [Agent design][design] |
| 作業状態 | エージェントセッションはスレッド単位で、状態は`processing`、`active`、`suspended`（利用者の入力待ち）、`closed`。`processing`中は「Working...」の表示と停止ボタンが出る（アプリが停止イベントを購読している場合）。利用者はセッションをピン留め、名前変更、アーカイブできる。 | [Agent sessions][sessions] |
| 計画と進み具合 | plan blockで手順の一覧を出し、各手順を`pending`、`in_progress`、`complete`と更新できる。ストリーミング中のタスク表示は`plan`または`timeline`を選べる。設計ガイドは、外部システムを平易な名前で示し、書き込み操作を目に見える形で出すよう推奨する。 | [Developing an agent][develop] [governance][gov] [Agent design][design] |
| 確認と承認の推奨 | 作成・送信・削除など現実に影響する操作の前に、実行内容のプレビューと「承認・修正・却下」の手段を出すよう推奨する。確認が多すぎると読まずに押すようになるので、必要な場面に絞るよう書いている。承認ゲート、途中の計画を利用者が編集できること、一時停止・再開・方向転換・停止もあげている。いずれも開発者向けの推奨。 | [Agent design][design] [governance][gov] |
| 代理実行の表示 | 利用者の身元で動くときは「on behalf of [user]」と表示し、レビュー前のAI生成物にはその旨を示し、App Homeで代理実行の履歴を確認できるようにするよう推奨する。 | [Agent design][design] |
| AIであることの表示 | 名前は機能を先に（「Deal Desk」など）、アバターは人間でないとわかるものにするよう推奨する。アプリをエージェントとして宣言すると、Slack UIでエージェントと表示される。 | [Agent design][design] [Developing an agent][develop] |
| Code channels | エージェントをメンションすると作業ごとのcode channelを作り、差分やプレビューやcanvasを**Artifacts**パネルに添付する。依頼者を自動で招待し、公開範囲は元のチャンネルに合わせる。完了したら元のスレッドに要約を返してアーカイブする、という流れを推奨する。 | [Agent design][design] |
| エージェントの追加 | Agentforce Studioでエージェントを選び、接続に「Slack」を追加して「バージョンをコミット」し、有効にする。その後Slackの管理者画面「Agentforceを管理する」で「エージェントを確認する」から権限を確かめてインストールする。 | [セットアップ][afsetup] |

## 技術

Slack本体とAgentforceはクローズドなので、公式ドキュメントに書かれた範囲だけを記す。オープンソースの開発用SDKは区別して記す。

| 層 | 技術 | 出典 |
|---|---|---|
| エージェント用API | `agents.sessions.setStatus`、`agents.sessions.rename`、`chat.startStream`／`appendStream`／`stopStream`（ストリームは開始から最長10分）、`app_context_changed`イベント、Block Kit。 | [Agent sessions][sessions] [Developing an agent][develop] |
| MCP | Slack MCP Server（Slackを操作するサーバー）とSlackbot MCP Client（HTTPでリモートMCPサーバーにつなぐ）。MCPのツール注釈`readOnlyHint`の例がある。 | [AI in Slack][devai] [Slack MCP Server][mcpserver] [Slackbot MCP Client][mcpclient] |
| Agentforce連携 | Salesforce組織とSlackを連携し、アカウントを対応付ける。カスタムSlackアクションはApexとNamed Credentialから作る。 | [セットアップ][afsetup] [カスタムSlackアクション][afactions] |
| 開発用SDK（オープンソース） | Bolt for JavaScript：TypeScript、MIT。依存は`@slack/web-api`、`@slack/socket-mode`、`@slack/oauth`、`express`など。Bolt for Python：MIT。 | [bolt-js package.json][boltpkg] [bolt-python][boltpy] |
| サーバー構成・データ保存 | 未確認。 | — |

## Pelwayとの比較［提案］

この節は本書の整理と提案であり、Slack・Salesforceの資料が述べている内容ではない。Pelway側は[ビジョン](../vision.md)、[PRD](../prd.md)、[技術](../technical.md)の構想で、実装済みの機能ではない。

| 観点 | Slack / Agentforce | Pelway（構想） | Pelwayでの扱い |
|---|---|---|---|
| 人とAIの同席 | 確認。複数エージェントの協働は未確認 | 中核の方針 | 同席だけでは差別化にならない |
| 外部操作前の承認 | 開発者への推奨（プレビューと承認・修正・却下）。Agentforceの操作別承認は未確認 | 承認を実行内容に結び付ける | 推奨を仕組みとして必ず通す点で差を出す |
| 判断待ちの表示 | セッションの`suspended`状態で入力待ちを示せる | 誰の・何の判断待ちかを表示 | 「誰の」判断かまで出す点を比較の観点にする |
| 進み具合の表示 | plan blockと手順ごとの状態 | 作業中・判断待ちなどの状態を定義 | 手順の粒度と短い文言の指針を参考にする |
| 引き継ぎ・再開 | 専用の仕組みは未確認。完了時に要約を返す推奨 | MVPの受け入れ条件の中心 | 要約を返すだけでなく、引き継ぎ欄として残す |
| 作業ごとの場所 | code channelとArtifactsパネル | 一つの仕事の中で依頼から引き継ぎまで追う | 成果物を会話から分けて並べる見せ方を参考にする |
| 代理実行の表示 | 「on behalf of」とレビュー前の表示を推奨 | AIであることを明示 | 送信物に承認者とAIを両方示す |
| 導入の重さ | Salesforce組織の連携、管理者のインストール、別ライセンス | 独立サービス | Salesforceを使わない小規模チームを比較の対象にする |

## 未確認事項

- 同じチャンネルの複数エージェントが互いのメッセージを受け取るか。
- Agentforceのエージェントが外部へ送信する前に、人の承認を必ず挟む設定があるか。
- Agentforceのエージェント経路でMCPを使えるか。
- Slackbot MCP Clientの確認画面の表示内容と、利用できるプラン。
- エージェントの実行環境の分離単位、保存地域、日本語での動作。

## 出典

- [Slack：Agentforce in Slack を使用する](https://slack.com/help/articles/36218786859667-Use-Agentforce-in-Slack)
- [Slack：Agentforce in Slack をセットアップして管理する](https://slack.com/help/articles/36218109305875-Set-up-and-manage-Agentforce-in-Slack)
- [Slack：Slack で AI エージェントを利用する](https://slack.com/help/articles/33076000248851)
- [Salesforce：Slack Pricing](https://www.salesforce.com/slack/pricing/)
- [Salesforce：Agentforce Pricing](https://www.salesforce.com/agentforce/pricing/)
- [Slack Developer Docs：AI in Slack](https://docs.slack.dev/ai)
- [Slack Developer Docs：Agent design](https://docs.slack.dev/concepts/agent-design)
- [Slack Developer Docs：Agent sessions](https://docs.slack.dev/ai/agent-sessions)
- [Slack Developer Docs：Developing an agent](https://docs.slack.dev/ai/developing-agents)
- [Slack Developer Docs：Managing governance and trust](https://docs.slack.dev/ai/agent-governance)
- [Slack Developer Docs：Slack MCP server](https://docs.slack.dev/ai/slack-mcp-server)
- [Slack Developer Docs：Slackbot MCP Client](https://docs.slack.dev/ai/slackbot-mcp-client)
- [Slack Developer Docs：Customizing Agentforce agents with Slack actions](https://docs.slack.dev/ai/customizing-agentforce-agents-with-custom-slack-actions)
- [Slack：slackapi/bolt-js](https://github.com/slackapi/bolt-js)
- [Slack：bolt-js package.json](https://github.com/slackapi/bolt-js/blob/main/package.json)
- [Slack：slackapi/bolt-python](https://github.com/slackapi/bolt-python)

[afslack]: https://slack.com/help/articles/36218786859667-Use-Agentforce-in-Slack
[afsetup]: https://slack.com/help/articles/36218109305875-Set-up-and-manage-Agentforce-in-Slack
[aiapps]: https://slack.com/help/articles/33076000248851
[slprice]: https://www.salesforce.com/slack/pricing/
[afprice]: https://www.salesforce.com/agentforce/pricing/
[devai]: https://docs.slack.dev/ai
[design]: https://docs.slack.dev/concepts/agent-design
[sessions]: https://docs.slack.dev/ai/agent-sessions
[develop]: https://docs.slack.dev/ai/developing-agents
[gov]: https://docs.slack.dev/ai/agent-governance
[mcpserver]: https://docs.slack.dev/ai/slack-mcp-server
[mcpclient]: https://docs.slack.dev/ai/slackbot-mcp-client
[afactions]: https://docs.slack.dev/ai/customizing-agentforce-agents-with-custom-slack-actions
[bolt]: https://github.com/slackapi/bolt-js
[boltpkg]: https://github.com/slackapi/bolt-js/blob/main/package.json
[boltpy]: https://github.com/slackapi/bolt-python
