# Mantle Chat

確認日：2026-11-11（JST、日本時間）

一覧と比較の前提は[競合調査](../competitors.md)を参照する。「確認」は公式資料・公開リポジトリに記載があるという意味で、実機検証済みではない。「未確認」は今回の資料で裏付けられなかった項目で、機能がないという意味ではない。

## 概要

| 項目 | 内容 |
|---|---|
| 提供元 | 運営法人名は未確認。利用規約は「Mantle」「Mantle Chat」とだけ書き、準拠法を"the laws of the jurisdiction in which Mantle operates"とする。[Terms][terms] |
| 公式URL | [mantle.chat][top]、[ドキュメント][docsindex] |
| 製品の説明 | "Where your team works with AI together"。SlackやDiscordのようなチームチャットに、誰でもメンションできるAIを組み込む。60以上のモデルを1つの契約で使い、会話の途中で切り替えや比較ができる。[公式][top] |
| 提供段階 | Webアプリ（デスクトップ・モバイルのブラウザ）とmacOSアプリ（macOS 12以降）。Windows、Linux、iOS、Androidのネイティブアプリは予定。[desktop][desktop] |
| ライセンス | サービス本体は未確認。外部AIクライアント向けのpluginリポジトリはMIT。[plugin README][plugin] |
| 価格 | Free $0（"No credit card required"）。Core "$10 per member/month"（"Billed $120 annually"）、Plus "$25 per member/month"（"Billed $300 annually"）、Max "$99 per member/month"（"Billed $1,188 annually"）、Enterprise "Custom"（"Contracted billing"）。Monthly／Annuallyの切替に"Save 15%"とあるが、月払い時の金額は表示から未確認。税別で、Stripeが決済時に税を加える。[料金][price] |
| 利用量 | AI credits：Free 1,500／月、Core 10,000、Plus 25,000、Max 100,000（有料は1人あたり）。追加10,000 creditsを$12で購入できる。接続できるアプリ数：Free 1、Core 3、Plus 10、Max 25。[料金][price] |
| 契約条件 | 月または年の期首に請求。いつでも解約でき、支払済み期間の終わりまで使える。Core、Plus、Maxを1組織で混在できる。Coreのカードに"14-day free trial"、FAQには初回の対象者がPlusの14日trialを支払い方法登録後に始められるとある。[料金][price] |

## 機能

| 観点 | 内容 | 出典 |
|---|---|---|
| 複数人＋複数AIの同一チャンネル参加 | channelで誰でもAIをメンションでき、回答はchannel全体に見える。保存したagentも `@` で選んでchannelの話し合いに呼ぶ。agentはworkspaceの共有資源。 | [公式][top] [Agents][agents] |
| AIの役割・専門性の設定 | agentごとに名前、短い説明、instructions、model、knowledge（fileとcanvas）、tools（接続アプリの操作）を設定する。テンプレートから作ることもできる。 | [Agents][agents] |
| 実行能力 | Tasksは手動、定期、On Complete、接続アプリのイベントで起動し、指定したchannelへ結果を投稿する。自動起動は有料plan。macOSアプリはCodex、Claude Code、Cursor、OpenCode、Grok CLIを利用者のMac上で動かし、fileの編集とコマンドを実行する。 | [Tasks][tasks] [desktop][desktop] |
| 承認・権限管理（Macのcoding agent） | 権限はRead only、Ask first（既定）、Edit automatically（コマンドは承認）、Full accessの4段階。Edit automaticallyとFull accessはフォルダごとに一度確認する。Plan modeもある（Claude Code、Codex、OpenCode）。 | [desktop][desktop] |
| 承認・権限管理（外部MCPクライアント） | MCP経由の削除、workspaceのアクセス設定の変更、channelへの投稿は、外部アプリ側のApprove／Cancelのカードで承認する。要求は10分で失効し、取消すると何も変わらない。カードを出せないアプリはその操作を実行できない。 | [更新][update] [plugin tools][tools] |
| 承認・権限管理（Mantle内のagent・task） | 接続アプリへの操作を実行前に承認する仕組みは今回の資料で未確認。安全性の文書は、送信、金銭、削除、公開の前に人が確認する運用と、必要なtoolだけを有効にすることを推奨している。 | [安全性][safety] [Tasks][tasks] |
| 接続の持ち主 | 接続したアプリのアカウントは接続した本人のもの。agentを共有してもアカウントは共有されず、共有agentを使う人は自分の接続を使う。 | [安全性][safety] [Agents][agents] |
| 外部連携 | 190以上の連携（Linear、Notion、Slack、Gmail、GitHub、Jira、Stripeなど）。REST API、incoming webhook、bot、webhook endpoint。外部AIクライアント向けのMCP server（`https://api.mantle.chat/mcp`）。agentのtools設定には `mcp_server_ids` の項目がある。 | [公式][top] [docs一覧][docsindex] [plugin tools][tools] |
| agent設定の変更提案 | APIとMCPで、agentの下書き（draft）に対する変更案（change set）を提案し、合意後に下書きへ適用する。適用は公開ではなく、公開はMantle内で別に行う。 | [change set API][changeset] [plugin tools][tools] [plugin README][plugin] |
| その他 | 音声のhuddleとAIのメモ、共同編集のcanvas、workspace knowledge。 | [公式][top] [docs一覧][docsindex] |

## UI/UX

| 要素 | 内容 | 出典 |
|---|---|---|
| 画面構成 | 左のレールにAgents、Tasksなどがあり、それぞれの横の＋から新規作成する。新規chatのホームには、huddleの実行中に"Live huddles"のchipが出る。 | [Agents][agents] [Tasks][tasks] [更新][update] |
| agent作成 | Agentsの＋またはTemplatesの"Use Template"から始め、名前と説明、instructions（目的、手順、出力形式、してはいけないこと）、model、knowledgeとtoolsを入れて"Create Agent"。作成後に通常の例、難しい例、断るべき例で試すよう案内している。 | [Agents][agents] |
| agentの使い方 | agentページの"Start chat"で1対1、channelで `@` メンション、同じ仕事を繰り返すならTaskにする。 | [Agents][agents] |
| channelでのagentの見え方 | AIの発言につくラベルや作業中の表示は今回の資料で未確認。 | [collaboration][collab] |
| 承認UI（Macのcoding agent） | 要求は元のchatの入力欄の上に出て、実行するコマンドまたはfileの差分と、agentの理由を表示する。選択肢はApprove（1回）、Always allow、Decline（agentは続行）、Stop（返答を終える）。別のchatの要求は「どこで待っているか」の表示だけになる。未回答の要求は2分で拒否される。Full accessはamberで表示。 | [desktop][desktop] |
| 変更の確認 | Changes paneがGitで未コミットの変更と返答ごとの変更を示し、返答単位でundo／redoできる。snapshotは隠しref `refs/mantle` に置き、branchや履歴を変えない。chatごとにgit worktreeを使え、Mantleからcommit、push、PR作成ができる。 | [desktop][desktop] [更新][update] |
| 利用枠の表示 | ローカルのchatでCodex・Claudeの5時間・週の残り枠を表示し、少なくなるとamberにする。 | [desktop][desktop] |
| 引き継ぎの書き方 | 文書は、引き継ぎに元の依頼、AIがしたこと、確認済みのこと、未決の判断を書くよう勧めている。専用の引き継ぎ欄は未確認。 | [collaboration][collab] |
| Taskの管理 | "Run now"で試し実行、"Active"の切替で自動起動を止める。履歴で各runの完了・失敗を見る。taskの所有者が組織を離れると止まり、管理者が付け替える。 | [Tasks][tasks] |

## 技術

公式ドキュメントと、外部AIクライアント向けpluginの公開リポジトリから確認した。サービス本体の実装言語・DB・インフラは未確認。

| 層 | 技術 | 出典 |
|---|---|---|
| API | REST API（OpenAPI定義を公開）。incoming webhook、署名付きwebhook endpoint（署名鍵のrotate、配信のreplay）、bot API。 | [docs一覧][docsindex] |
| MCP server | `https://api.mantle.chat/mcp`。ブラウザでのサインインで接続。承認にはMCP Appsのカードか、MCP 2026-07-28のform elicitationが必要で、対応しないクライアントには `confirmation_unsupported` を返す。承認は要求そのものに結び付き、モデルが渡す確認引数では代えられない。 | [plugin tools][tools] [plugin README][plugin] |
| 再試行の扱い | 作成とchannel投稿のtoolは `idempotency_key` 必須。同じkeyで別の引数を送ると拒否する。更新は `expected_updated_at` で古い版を拒否する。 | [plugin tools][tools] |
| MCP経由のagent呼び出し | 保存したagentはtoolなしか読み取り専用のtoolだけで呼べる。書き込み可能なagentは `read_only_agent_required` で拒否され、Mantle内で直接動かす。回答は24時間で消える。 | [plugin tools][tools] [更新][update] |
| 公開リポジトリ | [null-decision/mantle-chat-plugin][plugin]（MIT）。Claude、Codex、Cursor向けのplugin定義、skill、MCP接続設定で、補助スクリプトはPython（[scripts][pluginscripts]）。plugin定義のauthorは"Mantle Chat"、連絡先はsupport@mantle.chat。公式サイトからこのリポジトリへのリンクは未確認。 | [plugin README][plugin] [plugin.json][pluginjson] |
| ローカル実行 | coding agentは利用者のMac上で、各ツールの公式ログインと利用条件で動く。chatのフォルダとアクセス設定はそのMacに保存される。sandboxなどの分離方式は文書に記載がなく未確認。 | [desktop][desktop] [更新][update] |
| クラウド側の実行環境 | hosted AIやTasksの実行環境（AI別VMなど）は未確認。 | [desktop][desktop] |
| 資格情報 | 保存した接続の資格情報を保護し、通常の画面に秘密の値を出さない。保存方式は未確認。 | [安全性][safety] |
| 監査 | Enterpriseで監査ログとSIEM連携を"on request"で提供。他のplanの監査ログは未確認。 | [料金][price] [安全性][safety] |

## Pelwayとの比較［提案］

この節は本書の整理と提案であり、Mantle Chatの資料が述べている内容ではない。Pelway側は[ビジョン](../vision.md)、[PRD](../prd.md)、[技術](../technical.md)の構想で、実装済みの機能ではない。

| 観点 | Mantle Chat | Pelway（構想） | Pelwayでの扱い |
|---|---|---|---|
| 人とAIの同席 | 確認。モデルも保存したagentもメンションで呼ぶ | 中核の方針 | 同席だけでは差別化にならない |
| 外部操作前の承認 | 経路ごとに別。Macのcoding agentとMCP経由は確認、Mantle内agentの接続アプリ操作は未確認 | 外に出る操作は毎回承認 | 承認の経路を1つの仕組みにまとめる設計を比較の観点にする |
| 承認と要求の結び付き | MCP経由は要求に結び付いた承認、idempotency key、版の確認 | 承認した中身を固定し、重複発行を抑える | 同じ方向の実装例として、key必須と古い版の拒否を参考にする |
| 承認の期限 | Macは2分で拒否、MCPは10分で失効 | 期限の長さは未決 | 期限を短く置いて自動で拒否する例として参考にする |
| 引き継ぎ・判断待ち・再開 | 書き方の推奨はある。専用の欄は未確認 | 引き継ぎ欄と判断待ちの表示がMVPの中心 | 推奨の4項目（依頼、実施、確認済み、未決）を引き継ぎ欄の項目と照らし合わせる |
| 実行環境 | coding agentは利用者のMac | AI社員ごとの実行環境を検討 | 端末の稼働に依存しない点を比較の観点にする |
| 接続の持ち主 | 接続は本人のもの、共有agentは使う人の接続で動く | 依頼者・承認者・接続アカウントの権限を分ける | 共有agentと個人の接続を分ける考え方を参考にする |
| agent設定の変更 | 変更案を提案し、下書きへ適用、公開は別 | AIの役割設定を管理する | AIが自分の設定を変える場合の段階分けとして参考にする |
| 価格 | 人単位とAI credits、接続数の上限 | 人の参加費とAI実行枠を分ける案 | creditsと接続数の両方で上限を置く方式を見積もりの比較に使う |

## 未確認事項

- 運営法人名と所在地。
- Mantle内のagent・Tasksが接続アプリを操作する前の承認の有無と画面。
- channelでAIの発言につくラベル、作業中の表示。
- 月払い時の金額と、trialがCoreとPlusのどちらに付くか。
- hosted AIの実行環境、保存地域、日本語対応、SLA。

## 出典

- [Mantle Chat 公式](https://mantle.chat/)
- [Mantle Chat Pricing](https://mantle.chat/pricing)
- [Mantle Chat Terms](https://mantle.chat/terms)
- [Mantle：October 2026 update](https://mantle.chat/blog/monthly-update-october-2026)
- [Mantle Docs：llms.txt（ドキュメント一覧）](https://docs.mantle.chat/llms.txt)
- [Mantle Docs：Agents](https://docs.mantle.chat/features/agents)
- [Mantle Docs：Tasks](https://docs.mantle.chat/features/tasks)
- [Mantle Docs：Work together in Mantle](https://docs.mantle.chat/features/collaboration)
- [Mantle Docs：Web and desktop apps](https://docs.mantle.chat/features/desktop-app)
- [Mantle Docs：Integration access and safety](https://docs.mantle.chat/integrations/security-and-delivery)
- [Mantle Docs：Propose a change set](https://docs.mantle.chat/api-reference/agent-change-set-mutations/propose-a-change-set)
- [null-decision/mantle-chat-plugin README](https://github.com/null-decision/mantle-chat-plugin)
- [null-decision/mantle-chat-plugin docs/tools.md](https://github.com/null-decision/mantle-chat-plugin/blob/main/docs/tools.md)
- [null-decision/mantle-chat-plugin scripts](https://github.com/null-decision/mantle-chat-plugin/tree/main/scripts)
- [null-decision/mantle-chat-plugin .claude-plugin/plugin.json](https://github.com/null-decision/mantle-chat-plugin/blob/main/.claude-plugin/plugin.json)

[top]: https://mantle.chat/
[price]: https://mantle.chat/pricing
[terms]: https://mantle.chat/terms
[update]: https://mantle.chat/blog/monthly-update-october-2026
[docsindex]: https://docs.mantle.chat/llms.txt
[agents]: https://docs.mantle.chat/features/agents
[tasks]: https://docs.mantle.chat/features/tasks
[collab]: https://docs.mantle.chat/features/collaboration
[desktop]: https://docs.mantle.chat/features/desktop-app
[safety]: https://docs.mantle.chat/integrations/security-and-delivery
[changeset]: https://docs.mantle.chat/api-reference/agent-change-set-mutations/propose-a-change-set
[plugin]: https://github.com/null-decision/mantle-chat-plugin
[tools]: https://github.com/null-decision/mantle-chat-plugin/blob/main/docs/tools.md
[pluginscripts]: https://github.com/null-decision/mantle-chat-plugin/tree/main/scripts
[pluginjson]: https://github.com/null-decision/mantle-chat-plugin/blob/main/.claude-plugin/plugin.json
