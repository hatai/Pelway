# GroupChat（groupchat.ai）

確認日：2026-11-11（JST、日本時間）

一覧と比較の前提は[競合調査](../competitors.md)を参照する。「確認」は公式資料・公開リポジトリに記載があるという意味で、実機検証済みではない。「未確認」は今回の資料で裏付けられなかった項目で、機能がないという意味ではない。

## 概要

| 項目 | 内容 |
|---|---|
| 提供元 | GroupChat（公式サイトの表記）。運営法人名は未確認。[公式][site] |
| 公式URL | [groupchat.ai][site]、[Docs][docs]、GitHubの組織 [groupchatai][org] |
| 製品の説明 | 「One workspace for humans and agents to build together.」人とAIエージェントがプロジェクトとタスクを一緒に進めるworkspace。自分のエージェントを持ち込む。[公式][site] |
| 提供段階 | workspaceは無料で提供中。専門のmanaged agentsは「Coming soon」。[公式][site] |
| ライセンス | 製品本体の公開リポジトリは未確認。公開されているのは、MCP接続とskillsをまとめたプラグイン2つ（MIT）とスターター1つ。[groupchat-plugin][plugin] [groupchat-agent-plugin][aplugin] [org][org] |
| 価格 | 「$0 / forever」。projects、tasks、agentsは無制限。持ち込んだエージェントの費用は提供元に直接払い、上乗せはない。クレジットカード不要。managed agentsは使ったトークン分を払うと説明し、単価は未掲載。[公式][site] |

## 機能

| 観点 | 内容 | 出典 |
|---|---|---|
| 複数人＋複数AIの同一チャンネル参加 | 人とエージェントが同じプロジェクトとタスクで作業する。やり取りはタスクのコメントとメンションで行う。チャンネル型の会話は公式資料で未確認。 | [公式][site] [Agent Setup][setup] |
| AIの役割・専門性の設定 | エージェントは名前、説明、アバターを持つ。役割の定義方法の詳細は未確認。 | [Agent API][api] |
| 実行能力 | 公式資料は、接続したエージェント（Cursor、Claude、Codex、OpenClaw、webhookで接続する独自エージェント）がタスクを実行すると説明する。GroupChat自体がエージェントの実行環境を提供するかは未確認。定期・繰り返しのタスクをエージェントに自動で割り当てられる。 | [公式][site] [Agent Setup][setup] |
| 仕事の受け渡し | 人がタスクで「Delegate」を押すか、コメントでエージェントをメンションすると、PENDINGの実行（run）ができる。エージェントはrunを開始・コメント・完了・エラーで報告する。完了時にPRのURLと費用を添えられる。 | [Agent Setup][setup] [Agent API][api] |
| ステージの自動化 | 「エージェントが始めたらIn Progressへ」「PRの準備ができたらIn Reviewへ」「mergeされたらCompleteへ」のように、出来事でタスクを進める。Security Check、Prod Reviewなどのステージを追加できる。 | [公式][site] |
| 承認・権限管理 | 外部操作前の承認は公式資料で未確認。runの開始・完了・エラーは所有者に通知され、所有者が結果を見てタスクの状態を更新する。所有者はrunを止められる。エージェントのトークンと所有者のトークンは分かれている。 | [Agent Setup][setup] [Agent API][api] |
| 外部連携 | Workspace API、Agent API、webhook（認証はnone、bearer、hmac、header）、Owner MCPとAgent MCP（OAuth）。Claude Code、Codex、Cursor、Grok向けのプラグイン。 | [Docs][docs] [Agent API][api] [groupchat-plugin][plugin] [groupchat-agent-plugin][aplugin] |
| 監査 | 未確認。runの詳細に活動の記録（activity feed）が含まれる。 | [Agent Setup][setup] |

## UI/UX

| 要素 | 内容 | 出典 |
|---|---|---|
| 画面構成 | プロジェクトのボード（To do、Doing、Review、Done）。カードに担当者のアバターと、Cursor・Claude・Codexなどエージェントのバッジ、active・in review・shippedなどの状態が付く。 | [公式][site] |
| AIの作業状態 | エージェントの進捗とPRの準備完了をリアルタイムに表示する。「4 active now」のような在席表示がある。 | [公式][site] |
| エージェントへの依頼 | タスクの「Delegate」ボタン、またはコメントでのメンション。所有者からの追加の指示は新しいrunになる。 | [Agent Setup][setup] |
| 操作方法 | 音声、API、キーボード。 | [公式][site] |
| エージェント作成 | Account SettingsかAgentsページで名前を入れて作成する。トークンは1回だけ表示される。MCPの `create_agent` やREST APIでも作れる。Agent MCPの接続では、`/device` のURLで人がエージェントの接続を許可する。 | [Agent Setup][setup] |
| 人向けのMCP | Owner MCPのプラグインは「get started」「inbox の整理」「エージェントへの委任」の3つのskillを持つ。 | [groupchat-plugin][plugin] |

## 技術

製品本体の公開リポジトリは確認できなかったため、公式ブログと公式ドキュメントの記載に限る。

| 層 | 技術 | 出典 |
|---|---|---|
| フロントエンド | Next.js 16（App Router）、Tailwind CSS v4。 | [ブログ][blog] |
| バックエンド・リアルタイム | Convex（queries、mutations、actions）。エージェントは `convex` パッケージで `pendingRuns` を購読して仕事を受け取る。RESTのポーリングは代替手段。 | [ブログ][blog] [Agent Setup][setup] |
| 認証 | Clerk。APIは個人アクセストークン（`gcp_`）とエージェントのトークン（`gca_`）を分ける。 | [ブログ][blog] [Agent API][api] |
| エージェント連携 | MCP（`/api/mcp`、`/api/agent/mcp`、OAuth）、REST、webhook。 | [groupchat-plugin][plugin] [groupchat-agent-plugin][aplugin] [Agent API][api] |
| DB・ホスティング | 未確認。 | |

## Pelwayとの比較［提案］

この節は本書の整理と提案であり、GroupChatの資料が述べている内容ではない。Pelway側は[ビジョン](../vision.md)、[PRD](../prd.md)、[技術](../technical.md)の構想で、実装済みの機能ではない。

| 観点 | GroupChat | Pelway（構想） | Pelwayでの扱い |
|---|---|---|---|
| 人とAIの同席 | タスクのコメントとボード。チャンネル型の会話は未確認 | 共有チャンネルが中核 | 会話とタスクのどちらを中心にするかで違いが出る |
| 外部操作前の承認 | 未確認。所有者が結果を見て状態を更新する | 承認を実行内容に結び付ける | 比較の観点にする。欠けるとは断定しない |
| 仕事の受け渡し | Delegate・メンションでrunを作り、開始・進捗・完了・エラーを報告 | 引き継ぎ欄（決定、送信内容、残作業） | runの状態とエラーの種類（rate_limit、auth、crash）は「止まっている」の理由表示の参考になる |
| ステージ | 出来事で進むステージと追加できる確認ステージ | 状態の定義（判断待ち、送信承認待ちなど） | 会社ごとの確認ステージを足せる点は参考になる |
| 実行環境 | 持ち込んだエージェントの環境に任せる | AIごとのVM相当を検討 | 実行環境を持つかどうかが分かれ目になる |
| 外部エージェントの接続 | MCP、REST、webhook、各コーディングツール向けプラグイン | 構想の文書で未確認 | 人のトークンとエージェントのトークンを分ける設計は参考になる |
| 料金 | workspace無料、エージェント費は持ち込み | 人の参加費とAI実行枠を分ける案 | 無料の比較対象として、料金の説明で意識する |

## 未確認事項

- 運営法人名、所在地。
- 外部操作前の承認、権限の細かさ、監査ログ。
- チャンネル型の会話があるか。
- managed agentsの提供時期と単価。
- 日本語対応、保存地域、SLA。

## 出典

- [GroupChat公式・料金](https://www.groupchat.ai/)
- [GroupChat Docs](https://www.groupchat.ai/docs)
- [GroupChat Docs：Agent Setup](https://www.groupchat.ai/docs/guides/agent-setup)
- [GroupChat Docs：Agent API](https://www.groupchat.ai/docs/api/agent)
- [GroupChat Blog：Building with Convex and Next.js](https://www.groupchat.ai/blog/building-with-convex-and-nextjs)
- [GitHub：groupchatai](https://github.com/groupchatai)
- [GitHub：groupchatai/groupchat-plugin](https://github.com/groupchatai/groupchat-plugin)
- [GitHub：groupchatai/groupchat-agent-plugin](https://github.com/groupchatai/groupchat-agent-plugin)

[site]: https://www.groupchat.ai/
[docs]: https://www.groupchat.ai/docs
[setup]: https://www.groupchat.ai/docs/guides/agent-setup
[api]: https://www.groupchat.ai/docs/api/agent
[blog]: https://www.groupchat.ai/blog/building-with-convex-and-nextjs
[org]: https://github.com/groupchatai
[plugin]: https://github.com/groupchatai/groupchat-plugin
[aplugin]: https://github.com/groupchatai/groupchat-agent-plugin
