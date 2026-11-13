# Reason / Reason Network（Reason Machines）

確認日：2026-11-11（JST、日本時間）

一覧と比較の前提は[競合調査](../competitors.md)を参照する。「確認」は公式資料・公開リポジトリに記載があるという意味で、実機検証済みではない。「未確認」は今回の資料で裏付けられなかった項目で、機能がないという意味ではない。

依頼時の表記`reasone machine`・`reasoning machine`と、本書のReason Machinesとの一意の対応は未確認。本書は関連する名称と機能を持つ候補として扱い、同一製品とは断定しない。本書ではエージェント本体のReasonと、エージェント間通信のReason Networkを分けて書く。[競合調査](../competitors.md)

## 概要

| 項目 | 内容 |
|---|---|
| 提供元 | Reason Machines。Y Combinator（F26）の支援先と記載。[公式][reason] |
| 公式URL | [reasonmachines.com][reason]、[ドキュメント][docsidx]、[reason-machines/reason-network][netrepo] |
| 製品の説明 | Reasonはリポジトリ、ツール、計算機を使うコーディングエージェント。利用者のPCとReasonのcloud computerでコード編集とコマンド実行を行う。Reason Networkは、利用者や知人のPC上のコーディングエージェント同士が仕事を頼み合うpeer-to-peerのネットワーク。[ドキュメント][docsidx] [利用ガイド][reasonguide] [Network][netdoc] |
| 利用経路 | Webアプリ、デスクトップアプリ（Mac）、CLI、HTTP API、MCP、ACP、Slack。[ドキュメント][docsidx] [公式][reason] [AI employees][reasonjobs] |
| ライセンス | Reason本体の公開リポジトリは未確認。Reason Networkのエンジンは`MIT OR Apache-2.0`。[Cargo.toml][netcargo] [Network][netdoc] |
| 価格 | Pro $50/人/月（週1,250 credits）、Max $200/人/月（週5,000 credits）、Enterpriseは要問い合わせ。1 credit = $0.04。追加購入は$20〜$500の一回払いで、期限なしでworkspace共有。持ち込みモデル（ChatGPT、Claude、API keyなど）は提供元が請求。Reason Networkは無料。[プラン][reasonprice] [Network][netdoc] |
| 席 | 他のメンバーがいるworkspaceでは、各人にProまたはMaxの席が必要（Enterpriseなどを除く）。[プラン][reasonprice] |

## 機能

| 観点 | 内容 | 出典 |
|---|---|---|
| 複数人＋複数AIの同一チャンネル参加 | workspaceにはOwner、Admin、Memberの役割がある。Slackのスレッドで@Reasonに依頼できる。同じ業務チャンネルに複数人と複数AIが常駐する仕様は未確認。 | [Workspaces][ws] [公式][reason] |
| AIの役割・専門性の設定 | Reasonは名前と継続指示を持つエージェント。指示には仕事、信頼する参照元、返信してよい相手、禁止事項を書く。修正は記憶に残る。公式トップは「名前とチームを付ける」と説明。 | [AI employees][reasonjobs] [公式][reason] |
| 起動条件 | Loopで定時（cronなど）、webhook、GitHubのイベント、Slackチャンネルの新着で動く。 | [AI employees][reasonjobs] |
| 実行能力 | 1つのSessionで利用者のPCとcloud computerの両方でコマンドを実行できる。cloud computerはブラウザとterminalを持つ。非公開リポジトリにはReason GitHub Appを入れ、PRを作る。 | [利用ガイド][reasonguide] [Computers][computers] [AI employees][reasonjobs] |
| 承認・権限管理 | GmailコネクタではプレビューのメッセージだけがSendできる。Settings › Policiesで、メール前に必ず確認するworkspaceルールを置ける。顧客の目に触れる投稿、請求の変更、マージは指示で人の確認を求められる。コード変更はPRとして届く。複数のPCが使える場合は先に読み取りだけを行い、変更前に確認する。 | [AI employees][reasonjobs] [Computers][computers] |
| 共有の単位 | Vault、コネクタ、skill、記憶、モデルのアカウント、計算機はworkspace全体で共有。個人用の秘密情報はworkspace内に置けず、分けたい場合は個人のworkspaceを作る。コネクタは接続した人のアカウントで動く。 | [Workspaces][ws] |
| エージェント間通信（Reason Network） | Claude Code、Codex、Cursor、Gemini CLI、opencode、Piなどが参加する。エージェントは`list_team`、`send_message`、`check_inbox`、`ask_user`の4つのツールを持つ。宛先は1つのsession、ある人の全session、あるリポジトリで作業中の全session、全員から選ぶ。Grok Bot、Muse、ChatGPTはMCPサーバーのURLを追加して参加する。 | [Network][netdoc] |
| Networkの承認 | 受信メッセージを保留し、`reason network held`・`approve`・`reject`で扱う。他人のエージェントからの依頼は、自分の利用者の指示ではなく同僚の依頼として扱う。 | [Network][netdoc] |
| 外部連携 | Google Workspace、Slack、Stripe、PostHog、GitHubなど。MCPサーバー（HTTP APIと同じ操作）、HTTP API、CLI、ACP。 | [AI employees][reasonjobs] [MCP][mcp] [利用ガイド][reasonguide] |
| 監査 | SettingsにAudit logがある。Sessionsで各実行を読める。 | [AI employees][reasonjobs] |

## UI/UX

| 要素 | 内容 | 出典 |
|---|---|---|
| 画面構成 | Agentsの画面は左のサイドバーにsession、中央に会話、右に進捗。同じアプリにteammateと他のエージェント（Claude Code、Codex、Cursor）が並ぶ。 | [公式][reason] |
| エージェント作成 | WebアプリまたはMacアプリでReasonを作り、名前と指示を書く。コネクタとskillはCustomizeで追加する。 | [AI employees][reasonjobs] |
| 作業の続き | 同じSessionをアプリ、CLI、エディタ、APIのどこからでも続けられる。返答には各手順をどの計算機で実行したかが書かれる。 | [利用ガイド][reasonguide] [ドキュメント][docsidx] |
| 計算機の追加 | Settings > Cloud and local > Add computerで表示されるコマンドを端末に貼る。コードは一度だけ使え、10分で失効。計算機ごとのDirect device accessで利用を止められる。 | [Workspaces][ws] |
| Slack | チャンネルごとに、聞き取り、アクセス、返信のタイミングを設定する。 | [AI employees][reasonjobs] |
| Networkの確認 | `ask_user`はPC上のエージェントだけが使え、利用者にpop-upで尋ねる。 | [Network][netdoc] |

## 技術

Reason本体は公式ドキュメントから、Reason Networkは公開リポジトリの依存定義から確認した。

| 層 | 技術 | 出典 |
|---|---|---|
| cloud computer | コマンドは「sandbox」で動くと説明。仮想化技術（VM、microVM、コンテナ）は未記載。操作がない状態が約2分続くと停止し、プロセスも止まる。 | [Computers][computers] |
| ローカル実行 | `reason connect`で接続し、許可したフォルダだけでSessionを動かす。コマンドは利用者の権限で動く。 | [Computers][computers] |
| API | HTTP API `/v3`。CLI、ACP、MCPサーバーは同じAPIの上にある。進捗はSSEではなくlong-pollingで読む。 | [ドキュメント][docsidx] |
| MCP | Streamable HTTP（POSTのみ）。OAuth（PKCE、動的クライアント登録）またはAPI key。scopeで使えるツールを絞る。 | [MCP][mcp] |
| Network：言語 | Rust（edition 2024）。 | [Cargo.toml][netcargo] |
| Network：通信 | iroh（P2P、mDNSでの探索）。直接つながらない場合は暗号化された通信だけを中継するrelayを使う。PC間はend-to-endで暗号化。 | [Cargo.toml][netcargo] [Network][netdoc] |
| Network：その他 | tokio、rmcp（MCP）、rusqlite、clap、ring、blake3。 | [Cargo.toml][netcargo] |
| Network：設計 | チーム、人、計算機、sessionの階層を持つ名簿。sessionはエージェントの種類、作業ディレクトリ、リポジトリ、ブランチ、作業中か待機中か、一行の作業内容を持つ。 | [DESIGN.md][netdesign] |

## Pelwayとの比較［提案］

この節は本書の整理と提案であり、Reason Machinesの資料が述べている内容ではない。Pelway側は[ビジョン](../vision.md)、[PRD](../prd.md)、[技術](../technical.md)の構想で、実装済みの機能ではない。

| 観点 | Reason | Pelway（構想） | Pelwayでの扱い |
|---|---|---|---|
| 対象 | 開発者とコーディング作業が中心 | 開発以外を含む小規模チーム | 対象が重なる範囲は限られる |
| 承認 | Gmailのプレビュー承認以外は指示とworkspaceルールに依存 | 承認を実行内容に結び付ける | 「承認した中身だけを送れる」Gmailの方式を参考にする |
| 共有の単位 | workspaceで認証情報も共有 | 会社・チャンネルの権限検査 | 個人の資格情報とチーム共通の資格情報を分ける理由として使う |
| エージェント間の受け渡し | Networkの名簿で相手を選び、受信を保留・承認できる | 担当AIへの受け渡しを状態として持つ | 「誰が何をしているか」の名簿と、受信の保留を参考にする |
| 実行場所 | 利用者のPCとcloud computerを1つのSessionで併用 | AIごとの実行環境を検討 | 手順ごとの実行場所を記録に残す点を参考にする |

## 未確認事項

- 依頼時の表記とReason Machinesの同一性。
- 同じ業務チャンネルでの複数人と複数AIの常駐。
- cloud computerの仮想化技術と分離の単位。
- 公開リポジトリ`OpenAra`（MITのcomputer use用MCPサーバー）と`reason-plugins`（MIT）の詳細な位置づけ。[OpenAra][openara] [reason-plugins][plugins]
- 日本語対応、保存地域、SLA。

## 出典

- [Reason Machines公式サイト](https://reasonmachines.com/)
- [Reason Machines：ドキュメント索引（llms.txt）](https://docs.reasonmachines.com/llms.txt)
- [Reason Machines：How to use Reason](https://docs.reasonmachines.com/docs/guide)
- [Reason Machines：Plans, prices and credits](https://docs.reasonmachines.com/docs/plans)
- [Reason Machines：Workspaces](https://docs.reasonmachines.com/docs/workspaces)
- [Reason Machines：Computers](https://docs.reasonmachines.com/docs/build/computers)
- [Reason Machines：MCP](https://docs.reasonmachines.com/docs/mcp)
- [Reason Machines：Reason Network](https://docs.reasonmachines.com/docs/network)
- [Reason Machines：AI employees。自社機能説明のみ参照](https://reasonmachines.com/ai-employees)
- [Reason Machines：reason-machines/reason-network](https://github.com/reason-machines/reason-network)
- [Reason Machines：reason-network Cargo.toml](https://github.com/reason-machines/reason-network/blob/main/Cargo.toml)
- [Reason Machines：reason-network DESIGN.md](https://github.com/reason-machines/reason-network/blob/main/DESIGN.md)
- [Reason Machines：reason-machines/OpenAra](https://github.com/reason-machines/OpenAra)
- [Reason Machines：reason-machines/reason-plugins](https://github.com/reason-machines/reason-plugins)

[reason]: https://reasonmachines.com/
[docsidx]: https://docs.reasonmachines.com/llms.txt
[reasonguide]: https://docs.reasonmachines.com/docs/guide
[reasonprice]: https://docs.reasonmachines.com/docs/plans
[ws]: https://docs.reasonmachines.com/docs/workspaces
[computers]: https://docs.reasonmachines.com/docs/build/computers
[mcp]: https://docs.reasonmachines.com/docs/mcp
[netdoc]: https://docs.reasonmachines.com/docs/network
[reasonjobs]: https://reasonmachines.com/ai-employees
[netrepo]: https://github.com/reason-machines/reason-network
[netcargo]: https://github.com/reason-machines/reason-network/blob/main/Cargo.toml
[netdesign]: https://github.com/reason-machines/reason-network/blob/main/DESIGN.md
[openara]: https://github.com/reason-machines/OpenAra
[plugins]: https://github.com/reason-machines/reason-plugins
