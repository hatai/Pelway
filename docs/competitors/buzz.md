# Buzz（Block）

確認日：2026-11-11（JST、日本時間）

一覧と比較の前提は[競合調査](../competitors.md)を参照する。「確認」は公式資料・公開リポジトリに記載があるという意味で、実機検証済みではない。「未確認」は今回の資料で裏付けられなかった項目で、機能がないという意味ではない。

## 概要

| 項目 | 内容 |
|---|---|
| 提供元 | Block, Inc.（Jack Dorseyが率いる企業）。[TechCrunch][tc] |
| 公式URL | [block/buzz][repo0]（0.X、リレーとクライアント一式）、[block/buzz-app][repo1]（1.0のデスクトップアプリ） |
| 製品の説明 | 人間とAIエージェントが同じ部屋で作業する、セルフホスト可能なworkspace。中身はNostrのリレーで、メッセージ、リアクション、ワークフローの手順、レビュー承認、gitイベントをすべて署名付きイベントとして1本のログに残す。[README][repo0] |
| 提供段階 | 1.0はREADMEでbetaと表記。0.Xも引き続き提供・サポートされる。[buzz-app README][repo1] [README][repo0] |
| ライセンス | Apache License 2.0。[README][repo0] |
| 価格 | セルフホストは無料。ホスト版の料金は公式資料で未確認。Railwayにリレーをデプロイする手順がある。[README][repo0] |

## 機能

| 観点 | 内容 | 出典 |
|---|---|---|
| 複数人＋複数AIの同一チャンネル参加 | エージェントは人と同じ方法でチャンネルに追加するメンバーとして扱われる。チャンネルにいないエージェントをメンションすると、招待するか確認される。 | [README][repo0] [buzz-app README][repo1] |
| AIの役割・専門性の設定 | エージェントごとに名前、harness、model、指示を設定する。ペルソナのパック（`buzz-persona`）がある。 | [buzz-app README][repo1] [README][repo0] |
| 実行能力 | Claude、Codex、Goose、Piを利用者のPCで動かす。リポジトリを開く、パッチを送る、コードレビュー、ワークフロー実行、canvas編集、他エージェントの指揮、音声huddleへの参加を説明。リモートで動かす構想（Kubernetesなど）は設計文書の段階。 | [buzz-app README][repo1] [README][repo0] [VISION_REMOTE_AGENTS][remote] |
| 承認・権限管理 | エージェントは自分の鍵、チャンネル参加、監査ログを持ち、権限フラグではなく身元で範囲を決める。ワークフローの承認ゲートは、スキーマ、REST、MCPツール、UIはあるが、実行器が承認待ちで止まらず、その実行は失敗扱いになる（WF-08）。1.0のCodexは `approvalPolicy: "never"` と `danger-full-access` で動き、通常のturnでは承認を求めない。止める判断は保護プラグインとhookに任される。 | [README][repo0] [VISION][vision] [agent hooks][hooks] [agent protection][protect] |
| 外部連携 | エージェント向けCLI `buzz-cli`（JSON入出力）、ACPハーネス、MCP（`rmcp`）、YAMLワークフロー（メッセージ・リアクション・スケジュール・webhookで起動）、NIP-34のgitイベントとgitホスティング。 | [README][repo0] [Cargo.toml][cargo0] |
| 監査 | ハッシュチェーンの監査ログ（`buzz-audit`）。 | [README][repo0] |

## UI/UX

| 要素 | 内容 | 出典 |
|---|---|---|
| 画面構成 | 左のサイドバーにSolo（ひとりの会話）とTeam（チャンネル）を切り替えるタブ（All / Solo / Team）、上部にMessages、Reminders、Agents、Workflowsのナビゲーション。 | [buzz-app README スクリーンショット][solo-img] |
| Soloの会話をチャンネルにする | Soloの会話をチャンネルに変換する機能（`ConvertToChannel`）がある。 | [solo feature][solo-src] |
| エージェントの作業状態 | エージェントのアバターのオンライン表示が、作業中・入力中は「…」の吹き出しに変わる。チャンネル画面の入力欄の下に「Honey: Working」のような状態が出る。 | [presence][presence] [スクリーンショット][thread-img] |
| エージェント間の受け渡し | スクリーンショットでは、エージェントが返答の最後で別のエージェントをメンションして次の作業を渡している。 | [スクリーンショット][thread-img] |
| エージェント作成 | アバターと名前を大きく表示し、Harness、Model、Instructions（任意）だけを入力する。会話中のツール `agent_create` は未保存の下書きだけを作り、所有者が入力済みフォームを確認して作成する。 | [create agent画像][create-img] [agents][agents] |
| Activity | All / Threads / Mentions / DMs / Reactions のタブ、送り手をAnyone / People / Agentsで絞るFromメニュー、Unreadの切り替え。 | [inbox][inbox] |
| Attention | エージェントを起こす条件（イベント、タイマー）をエージェントごとのAttentionタブで確認・一時停止・削除する方向。提案段階の文書。 | [attention][attention] |
| 情報の公開範囲 | 信頼できる仲介役がエージェントの署名鍵を持ち、非公開の情報を見たエージェントのインスタンスが、より広い範囲へ投稿するのを拒む設計案。草稿段階の文書。 | [information flow][ifc] |
| 鍵の扱い | 秘密鍵はOSの安全な保管場所に置き、Buzzはバックアップしない。利用者に自分で写しを保管するよう求める。 | [buzz-app README][repo1] |

## 技術

公開リポジトリの依存定義から確認した。

| 層 | 技術 | 出典 |
|---|---|---|
| サーバー言語 | Rust（toolchain 1.98.1）。目的別のcrateに分けたworkspace。 | [Cargo.toml][cargo0] [README][repo0] |
| Web・リアルタイム | axum、tokio、tower、tokio-tungstenite（WebSocket）。 | [Cargo.toml][cargo0] |
| プロトコル | Nostr。NIP-01、NIP-42/NIP-98のSchnorr認証、NIP-34（git）、Blossom（メディア）。 | [README][repo0] |
| DB・検索 | Postgres（sqlx）。全文検索もPostgres FTS。 | [README][repo0] [Cargo.toml][cargo0] |
| 配信・状態 | Redis（pub/sub、在席、入力中）。 | [README][repo0] |
| ストレージ | S3互換（開発時はMinIO）。 | [README][repo0] |
| エージェント連携 | rmcp（MCP）、ACP。 | [Cargo.toml][cargo0] [README][repo0] |
| 運用 | kube-rs、OpenTelemetry、Prometheus、Docker Compose。P2P通信にiroh。 | [Cargo.toml][cargo0] |
| デスクトップ | Tauri 2（wry）、rusqlite。 | [src-tauri Cargo.toml][tauri] |
| フロントエンド | React 19、TanStack Router、Base UI、Tailwind CSS 4、ProseMirror、dockview-react、virtua、xterm.js、nostr-tools。プラグイン基盤にCordis。 | [package.json][pkg] |
| ビルド・テスト | pnpm、Vite、TypeScript、Vitest、Playwright、Biome、just、lefthook、cargo-deny。 | [package.json][pkg] [README][repo0] |

## Pelwayとの比較［提案］

この節は本書の整理と提案であり、Buzzの資料が述べている内容ではない。Pelway側は[ビジョン](../vision.md)、[PRD](../prd.md)、[技術](../technical.md)の構想で、実装済みの機能ではない。

| 観点 | Buzz | Pelway（構想） | Pelwayでの扱い |
|---|---|---|---|
| 人とAIの同席 | 確認 | 中核の方針 | 同席だけでは差別化にならない |
| 外部操作前の承認 | ワークフローの承認ゲートは未完成。1.0のエージェントは通常承認を求めない | 承認を実行内容に結び付ける | 差を出せる領域。失敗時は止める（fail closed）方針を保つ |
| 引き継ぎ・判断待ち・再開 | 専用の機能は未確認。ログと検索で追う | MVPの受け入れ条件の中心 | 受け渡しを状態として持てば差を出せる |
| 実行環境 | 1.0は利用者のPC | AIごとのVM相当を検討 | 端末の稼働に依存しない点を比較の観点にする |
| 個人の成果の共有 | Soloの会話をチャンネルへ変換 | 個人とAIの会話を共有する負担を課題仮説にしている | 変換の操作を参考にする |
| 作業状態の表示 | アバターと入力欄の下で作業中を表示 | 判断待ちの一覧をMVP候補にしている | 「誰の番か」を示す表示の参考にする |
| 鍵の管理 | 利用者が秘密鍵を保管 | 対象は開発以外を含む小規模チームの仮説 | 利用者に鍵を管理させない |
| 情報の公開範囲 | 仲介役で広い範囲への投稿を拒む設計案 | 企業間分離と情報管理が未決事項 | 仕組みで止める方式を検討材料にする |

## 未確認事項

- ホスト版の料金、利用者数、導入実績。
- 1.0でワークフローの承認ゲートが動作するか。
- 情報の公開範囲の設計案とAttentionが実装されているか。
- 日本語対応、保存地域、SLA。

## 出典

- [Block：block/buzz README](https://github.com/block/buzz)
- [Block：block/buzz VISION.md](https://github.com/block/buzz/blob/main/VISION.md)
- [Block：block/buzz VISION_REMOTE_AGENTS.md](https://github.com/block/buzz/blob/main/VISION_REMOTE_AGENTS.md)
- [Block：block/buzz Cargo.toml](https://github.com/block/buzz/blob/main/Cargo.toml)
- [Block：block/buzz-app README](https://github.com/block/buzz-app)
- [Block：buzz-app docs/agent-tool-hooks.md](https://github.com/block/buzz-app/blob/main/docs/agent-tool-hooks.md)
- [Block：buzz-app docs/agent-protection.md](https://github.com/block/buzz-app/blob/main/docs/agent-protection.md)
- [Block：buzz-app docs/agents.md](https://github.com/block/buzz-app/blob/main/docs/agents.md)
- [Block：buzz-app docs/presence.md](https://github.com/block/buzz-app/blob/main/docs/presence.md)
- [Block：buzz-app docs/inbox.md](https://github.com/block/buzz-app/blob/main/docs/inbox.md)
- [Block：buzz-app docs/VISION_ATTENTION_SUBSCRIPTIONS.md](https://github.com/block/buzz-app/blob/main/docs/VISION_ATTENTION_SUBSCRIPTIONS.md)
- [Block：buzz-app docs/practical-information-flow-for-buzz-agents.md](https://github.com/block/buzz-app/blob/main/docs/practical-information-flow-for-buzz-agents.md)
- [Block：buzz-app src/features/solo](https://github.com/block/buzz-app/tree/main/src/features/solo)
- [Block：buzz-app package.json](https://github.com/block/buzz-app/blob/main/package.json)
- [Block：buzz-app src-tauri/Cargo.toml](https://github.com/block/buzz-app/blob/main/src-tauri/Cargo.toml)
- [Block：buzz README スクリーンショット（channel-thread）](https://github.com/block/buzz/blob/main/docs/assets/screenshots/channel-thread.png)
- [Block：buzz-app README スクリーンショット（solo-chat）](https://github.com/block/buzz-app/blob/main/docs/assets/readme/solo-chat.png)
- [Block：buzz-app README スクリーンショット（create-agent）](https://github.com/block/buzz-app/blob/main/docs/assets/readme/create-agent.png)
- [TechCrunch：Jack Dorsey is taking on Slack with Buzz](https://techcrunch.com/2026/07/21/jack-dorsey-is-taking-on-slack-with-buzz-a-group-chat-platform-for-teams-and-their-ai-agents/)

[repo0]: https://github.com/block/buzz
[repo1]: https://github.com/block/buzz-app
[vision]: https://github.com/block/buzz/blob/main/VISION.md
[remote]: https://github.com/block/buzz/blob/main/VISION_REMOTE_AGENTS.md
[cargo0]: https://github.com/block/buzz/blob/main/Cargo.toml
[hooks]: https://github.com/block/buzz-app/blob/main/docs/agent-tool-hooks.md
[protect]: https://github.com/block/buzz-app/blob/main/docs/agent-protection.md
[agents]: https://github.com/block/buzz-app/blob/main/docs/agents.md
[presence]: https://github.com/block/buzz-app/blob/main/docs/presence.md
[inbox]: https://github.com/block/buzz-app/blob/main/docs/inbox.md
[attention]: https://github.com/block/buzz-app/blob/main/docs/VISION_ATTENTION_SUBSCRIPTIONS.md
[ifc]: https://github.com/block/buzz-app/blob/main/docs/practical-information-flow-for-buzz-agents.md
[solo-src]: https://github.com/block/buzz-app/tree/main/src/features/solo
[pkg]: https://github.com/block/buzz-app/blob/main/package.json
[tauri]: https://github.com/block/buzz-app/blob/main/src-tauri/Cargo.toml
[thread-img]: https://github.com/block/buzz/blob/main/docs/assets/screenshots/channel-thread.png
[solo-img]: https://github.com/block/buzz-app/blob/main/docs/assets/readme/solo-chat.png
[create-img]: https://github.com/block/buzz-app/blob/main/docs/assets/readme/create-agent.png
[tc]: https://techcrunch.com/2026/07/21/jack-dorsey-is-taking-on-slack-with-buzz-a-group-chat-platform-for-teams-and-their-ai-agents/
