# Rakazo（elie222 / Inbox Zero Inc.）

確認日：2026-11-13（JST、日本時間）

一覧と比較の前提は[競合調査](../competitors.md)を参照する。「確認」は公式資料・公開リポジトリに記載があるという意味で、実機検証済みではない。「未確認」は今回の資料で裏付けられなかった項目で、機能がないという意味ではない。

## 概要

| 項目 | 内容 |
|---|---|
| 提供元 | GitHub上の開発者はelie222（Elie Steinbock、[Inbox Zero](https://getinboxzero.com/)の作者）。公式のAboutは「Inbox Zero Inc.がプロジェクトを維持する」と記載。[About][about] |
| 公式URL | [rakazo.com][site]、[ドキュメント][docs]、[elie222/rakazo][repo] |
| 製品の説明 | オープンソースのGrok Bot代替。名前・指示・記憶・コンピュータを持つボット（AIの同僚）に、チャットやSlackなどで仕事を頼む。Web・デスクトップ・iOS・Androidの各アプリが同じサーバーにつながる。[README][repo] |
| 提供形態 | セルフホスト（公開イメージとDocker Compose）と、ホスト版（app.rakazo.com。トップページにBeta表示）。[README][repo] [料金][pricing] |
| 対応環境 | Web、デスクトップ（Electron。macOS・Windows・Linux）、iPhone・iPad・Android（Expo）。UI言語に日本語を含む。[README][repo] |
| ライセンス | Apache-2.0。[README][repo] |
| 価格 | セルフホスト無料。ホスト版はStandard $15/月（年払い$12/月）、Pro $49/月、Max $99/月。ボットのコンピュータはStandard 25時間/月（4 vCPU/8 GB）、Pro 100時間（8 vCPU/16 GB）、Max 200時間（16 vCPU/32 GB）。Rakazo creditsは$10/月から（モデル利用は提供元価格に上乗せなし。$30/月でコンピュータ30時間など）。7日間試用（コンピュータ10時間付き）。超過課金なし、同時実行は最大5台。[料金][pricing] [公式][site] |
| データの所在地 | 料金の比較表で、ホスト版の各プランに「EU hosting」の含む表示。既定の保存地域と日本リージョンは未確認。セルフホストの置き場所は運営者の選択。[料金][pricing] |
| コミュニティ | スター約8.1k、fork約1.5k（2026-11-13時点）。リポジトリ作成は2026-08-13、最新版v0.11.0（2026-11-12 JST公開）。[リポジトリAPI][repoapi] [v0.11.0][release] |

## 機能

| 観点 | 内容 | 出典 |
|---|---|---|
| 複数人＋複数AIの同一チャンネル参加 | ボットごとに1つ続くチャットを持つ。group chatは自分のボットを2〜10体選んで作り、@Name・@everyoneで呼ぶ。人がgroup chatに参加する仕様は未確認。共有スペースではメンバーがボット・チャット・記憶・ルーティンを共有し、ボットはメッセージを送った人のアカウントで動く。 | [ボットとチャット][bots] [グループ][groups] [スペース][spaces] |
| AIの役割・専門性の設定 | ボットごとに名前・アバター・常設の指示（上限2万字）・モデル・思考レベル・音声・通知を設定。テンプレート（朝のブリーフィング、受信箱の仕分けなど）から作れる。他のボットへの見え方の説明と、ツール単位の無効化もある。指示の変更履歴と復元。 | [ボットとチャット][bots] [ボット管理][fleet] |
| AI間の引き継ぎ | ボットは別のボットに直接メッセージを送って質問や委譲ができ、やりとりは閲覧専用で表示される。大きい仕事は短命のサブエージェントに分割し、終了とともに消える（チャット・記憶を持たない）。スペースをまたぐ通信は既定でオフ、設定で許可する。 | [グループ][groups] |
| 実行能力 | 各ボットにコンピュータ（ブラウザ・ターミナル・ファイル・デスクトップ）。Team（スペースで1台を共有し、画面とブラウザはボット別）またはPrivate。デスクトップアプリの設定で、自分のPC上でのコマンド実行とアプリ操作も許可できる。 | [コンピュータ][computer] |
| 承認・権限管理 | 3モード。Ask（変更を伴う操作で尋ねる）、Auto（承認モデルが依頼との一致を審査し、外れるものと取り消せないものは尋ねる）、Never ask。承認カードはAllow once・Always allow（ツール単位・サイト単位）・Deny。Allow/Blockリストは設定で管理し、Blockは全モードでカードなしの拒否として優先する。ボットの指示・モデル変更とスペース作成、コマンドの監視や外部コンテンツ由来のサイトを含むルーティンは常にカードを出し、Always allowでも省略しない。支出上限は日次・週次・月次または無期限でリセットし、アカウントの全ボットとスペースが対象。承認設定はスペース単位でも変えられる。 | [承認][approvals] |
| 承認者 | ボットはメッセージを送った人のアカウントで動き、その質問に答えられるのはその人だけ。ルーチンは所有者のアカウントで動き、編集すると自分のものになる。承認者を依頼者と別に指定する仕組みは未確認。 | [スペース][spaces] |
| 記憶 | ボットごとのMEMORY.md（1行1事実、上限4 KB。変われば書き直し、成り立たないものは削除）。スペース単位の共有メモリ。外部プロバイダ（Supermemory・Serenity・Honcho）はIsolated/Sharedの範囲を選べる。記憶はスペースをまたがない。未完了のOpen workリストは記憶とは別。 | [記憶][memory] |
| スキル | 手順を書いたスキルのライブラリを持ち、ボット自身も保存する。GitHubリポジトリやプラグインからの取り込みは指示のみで、同梱スクリプトは実行しない。ボットの画面で手順を録画して教えるTeach a taskもある。 | [スキル][skills] |
| 外部連携 | アプリカタログ（Gmail・Slack・GitHub・Notion・Linearなど。セルフホストでは運営者がComposioかPipedreamを設定した場合のみ表示）。任意のHTTPS MCPサーバー、OpenAPI・GraphQL、Executor、GitHubのプラグイン。資格情報は暗号化して保管し、モデルには渡さない。MCPセッションはrun単位で再利用する。 | [連携][integrations] [README][repo] |
| 起動条件（ルーティン） | 時刻（cron対応）、変更監視（5分〜1日間隔でファイル・コマンド出力・ツール。v0.11.0で公開HTTPSアドレスも）、Webhook、GitHubイベント（push・issue・pull request）、メール（ボット専用アドレス）、Slackメッセージ。Webhook・メール・Slack由来のrunは所有者の承認モードとAlways allowに従い、変更を伴うツールは審査し、購入は拒否する。 | [ルーティン][routines] [v0.11.0][release] |
| ボットによるボット管理 | Fleet management（list_fleet・create_group・set_child_instructionsなど）。自分が開始したrunだけが使え、AskとAutoでは他ボットの指示・モデル変更は常に承認カードを出し、Auto審査とAlways allowでは省略しない。 | [ボット管理][fleet] |
| 監査 | 常時有効。ツール呼び出し、承認の判断、資格情報の変更、サインインなどを記録。行の更新をPostgresが拒否し、削除は保持期間の剪定のみ。JSONL書き出しとSIEM Webhook連携（at-least-once）。保持は既定365日。メッセージ本文・ツール引数・秘密の値は記録しない。 | [監査ログ][audit] |
| 音声 | マイク入力、返信の読み上げ、ボットとの通話。ボットからユーザーへの発信もある。プロバイダはElevenLabs・OpenAI・Fish Audio・xAI・Cartesia・自己ホスト（OpenAI互換）・macOS。 | [音声][voice] |

## UI/UX

| 要素 | 内容 | 出典 |
|---|---|---|
| 画面構成 | サイドバーに「+」（ボット・group chat・スペースの作成）、Search、Activity（全runを状態と日付で絞り込み）、Artifacts（ボットが作ったファイル一覧）。チャット上部のAgent computerでボットの画面を開く。 | [ボットとチャット][bots] |
| ボット作成 | サインアップ時に最初のボットが自動で作られ、仕事は最初のメッセージで平文で伝える。テンプレートからも作れる（指示・スキル・停止中のルーティンを追加し、アプリには接続しない）。 | [ボットとチャット][bots] |
| 承認カード | チャットにカードが出て、チャットは止まらない。通知はボット設定の「完了時・入力必要時」。Allowリストは設定で一覧・管理する。 | [承認][approvals] |
| 画面の引き継ぎ | Take controlで人が画面を使い、CAPTCHAやコード入力ではNeeds youと表示して求める。I'm doneかSkipで返す。ログインの保存を頼めば次回は自動。 | [コンピュータ][computer] [資格情報][secrets] |
| 対応待ちの一覧 | Activityに全run、ボットの設定にOpen work（未完了）。期限付きの未解決はボットが自分にフォローアップを設定することもある。 | [ボットとチャット][bots] |
| ページ | ダッシュボードや報告書をボットが作り、全画面で開く。リンク共有（サインインなしで閲覧可）とPDF保存。ルーティンで同じページを更新し続ける。 | [ページ][pages] |
| スペース共有 | 招待リンクは1回限りで7日で失効。参加者はそのスペースだけに入り、組織・席・監査ログには現れない。Team Computerのサインインはスペースの全員に共有され、画面は1人が操作し他は見る。退出者の作ったものは所有者へ移り、ルーティンは停止する。 | [スペース][spaces] |
| チャットアプリ | Slackはワークスペース追加とDM連携。記憶とメモは接続した本人のメッセージだけで使い、ワークスペース内の他の人にはgroup chatと同じ応答をする。Telegram・WhatsApp・iMessage・SMS・FeishuとTeams DMはセルフホストでの設定による。 | [チャットアプリ][messaging] |

## 技術

公開リポジトリ（Apache-2.0）と公式ドキュメントから確認した。

| 層 | 技術 | 出典 |
|---|---|---|
| 構成 | サインイン後の製品は、長時間動くAPI、Graphile Worker、Postgres、コンピュータ提供元（サンドボックス管理）。TypeScript、HonoとoRPC、Prisma、Better Auth。フロントはReact 19・Vite・Tailwind、デスクトップはElectron、モバイルはExpo。 | [セルフホスト][selfhost] [README][repo] |
| 分離の単位 | スペースに1台のTeam Computer（ファイル・ツール・サインインを共有し、画面とブラウザはボット別）。Private Computerも選べる。 | [コンピュータ][computer] |
| サンドボックス提供元 | デスクトップアプリで自分のコンピュータを使う道のほか、料金表は13種を挙げる（Docker〔既定〕、Kubernetes、Incus、Boat、Boxd、CreateOS、Daytona、E2B、InstaBox、Novita、Railway、Wuying、Apple container）。 | [料金][pricing] [提供元][providers] |
| データベース | アプリは専用ログイン（rakazo_app）で接続し、RLSを適用する。移行はownerとして実行。 | [README][repo] |
| 資格情報 | Postgresのvault_credentialsにAES-256-GCMで暗号化。保存時と完全一致するoriginへの要求だけ復号して注入し、モデルに値を渡さない。保存したログインは、保存したoriginのHTTPSページ上でのみ入力する。 | [資格情報][secrets] |
| モデル接続 | ChatGPTサインイン（Codex枠）、OpenAI・Anthropic・Google・OpenRouterなどのAPIキー、OpenAI互換サーバー（Chat Completions・Responses・Anthropic Messages）。フォールバック連鎖と、ボット・ルーティン単位のモデル選択。ClaudeはAnthropic APIキー必須（Pro/MaxにAPIクレジットは含まれない。サインイン欄はあるが、Anthropicの規約から推奨しないと明記）。OpenCode GoのAPIキーにも対応。 | [モデル][models] [サブスク][subs] |
| セルフホスト | 公開イメージ（install-images.sh）かソースから。Docker Engine 26以上とComposeで、API・worker・Postgres・サンドボックス管理を自分の環境で動かす。 | [セルフホスト][selfhost] |

## Pelwayとの比較［提案］

この節は本書の整理と提案であり、Rakazoの資料が述べている内容ではない。Pelway側は[ビジョン](../vision.md)、[PRD](../prd.md)、[技術](../technical.md)の構想で、実装済みの機能ではない。

| 観点 | Rakazo | Pelway（構想） | Pelwayでの扱い |
|---|---|---|---|
| 段階と速度 | 提供中で課金もあり（ホスト版とクレジット）、3か月でv0.11.0、スター約8.1kのコミュニティ | 設計段階。実装済みの機能はない | 速度差を前提に、検証する業務を絞って出す価値を測る |
| 承認の思想 | 権限付与（Allowリスト）と本人への承認カードが中心。Autoモードは承認モデルによる一致審査、支出上限は金額面の拘束 | 承認者をカードの作成時に決めて期限を置く（ADR-0002）。操作の段階は連携ごとの宣言と管理者の確定で、決まらないものは止める（ADR-0004） | 依頼者と承認者を分ける業務での違いを試作で測る |
| 自動化の境目 | Always allow（ツール・サイト単位）と承認モデルの審査 | 決裁済みの「型」に一致する外部操作だけ自動実行する案（ハイブリッド）は別スレッドでADRとして起草中 | この文書では示唆として扱い、起草中のADRの番号・結論には依存しない |
| 仕事の状態 | run履歴、Activity、Open work、フォローアップ | 仕事の状態機械と判断待ちの集約（「今日」画面）、結果不明時の停止と照合、引き継ぎ欄 | 状態機械・照合・構造化した引き継ぎ記録は今回の資料では確認できず、評価項目として扱う |
| 権限の交差 | ボットはメッセージを送った人のアカウントで動き、Team Computerのサインインはスペース全員に共有 | AIは独立した身元を持ち、依頼者が委ねた権限と役割の権限が重なる範囲だけで動く（ADR-0001）。実行は依頼されたチャンネルの読者に結び付き、より広い範囲への投稿は承認を要する（ADR-0003） | 「誰の権限で動いたか」の記録と、範囲が広がる投稿の止め方を比較観点にする |
| 保存地域 | ホスト版はEU hostingを含む表示。既定の保存地域は未確認。セルフホストは運営者の選択 | 国内完結の方針（ADR-0008。BYOKは契約者の責任で例外） | 保存地域の明示が日本向けの条件になるかを確かめる |

## 未確認事項

- group chatに複数の人が参加できるか。
- 承認者を依頼者と別に指定する仕組み、承認内容の固定と変更時の再承認、承認カードの期限、承認の使い回しの防止。ボット管理ツールの承認カードでは、待機中に指示やモデルが変わったら承認しても上書きせず聞き直す仕様を公式が説明する。一般の操作カードへの同じ仕様の適用は未確認。[ボット管理][fleet]
- ホスト版の既定の保存地域と日本リージョン。料金比較表の「EU hosting」の含む表示は確認済み。
- 日本での税込価格とSLA。
- 運営法人Inbox Zero Inc.の所在地などの法人情報。
- 公式トップの利用者の声にClaudeのOAuth利用をうかがわせるものがあり、公式ドキュメント（ClaudeはAnthropic APIキー必須、サインインは推奨しない）と食い違う。本文書はドキュメントを優先した。[公式][site] [サブスク][subs]

## 出典

- [Rakazo公式サイト](https://rakazo.com/)
- [Rakazo：Pricing](https://rakazo.com/pricing/)
- [Rakazo：About](https://rakazo.com/about/)
- [Rakazo：ドキュメント](https://rakazo.com/docs/)
- [Rakazo：elie222/rakazo README](https://github.com/elie222/rakazo)
- [Rakazo：v0.11.0リリース](https://github.com/elie222/rakazo/releases/tag/v0.11.0)
- [Rakazo：リポジトリ情報（GitHub API）](https://api.github.com/repos/elie222/rakazo)
- [Rakazo docs：Bots and chats](https://rakazo.com/docs/bots-and-chats/)
- [Rakazo docs：Groups and teamwork](https://rakazo.com/docs/groups/)
- [Rakazo docs：Approvals](https://rakazo.com/docs/approvals/)
- [Rakazo docs：Memory](https://rakazo.com/docs/memory/)
- [Rakazo docs：Skills](https://rakazo.com/docs/skills/)
- [Rakazo docs：Spaces](https://rakazo.com/docs/spaces/)
- [Rakazo docs：Models](https://rakazo.com/docs/models/)
- [Rakazo docs：Model subscriptions](https://rakazo.com/docs/model-subscriptions/)
- [Rakazo docs：Connect apps](https://rakazo.com/docs/integrations/)
- [Rakazo docs：The bot's computer](https://rakazo.com/docs/computer/)
- [Rakazo docs：Routines](https://rakazo.com/docs/routines/)
- [Rakazo docs：Pages](https://rakazo.com/docs/pages/)
- [Rakazo docs：Voice and calls](https://rakazo.com/docs/voice-and-calls/)
- [Rakazo docs：Chat apps](https://rakazo.com/docs/messaging-apps/)
- [Rakazo docs：Reusable credentials](https://rakazo.com/docs/bot-secrets/)
- [Rakazo docs：Bots that manage bots](https://rakazo.com/docs/fleet-management/)
- [Rakazo docs：Self-hosting](https://rakazo.com/docs/self-host/)
- [Rakazo docs：Computer providers](https://rakazo.com/docs/self-host-sandbox-providers/)
- [Rakazo docs：Audit log](https://rakazo.com/docs/self-host-audit-log/)

[site]: https://rakazo.com/
[pricing]: https://rakazo.com/pricing/
[about]: https://rakazo.com/about/
[docs]: https://rakazo.com/docs/
[repo]: https://github.com/elie222/rakazo
[repoapi]: https://api.github.com/repos/elie222/rakazo
[release]: https://github.com/elie222/rakazo/releases/tag/v0.11.0
[bots]: https://rakazo.com/docs/bots-and-chats/
[groups]: https://rakazo.com/docs/groups/
[approvals]: https://rakazo.com/docs/approvals/
[memory]: https://rakazo.com/docs/memory/
[skills]: https://rakazo.com/docs/skills/
[spaces]: https://rakazo.com/docs/spaces/
[models]: https://rakazo.com/docs/models/
[subs]: https://rakazo.com/docs/model-subscriptions/
[integrations]: https://rakazo.com/docs/integrations/
[computer]: https://rakazo.com/docs/computer/
[routines]: https://rakazo.com/docs/routines/
[pages]: https://rakazo.com/docs/pages/
[voice]: https://rakazo.com/docs/voice-and-calls/
[messaging]: https://rakazo.com/docs/messaging-apps/
[secrets]: https://rakazo.com/docs/bot-secrets/
[fleet]: https://rakazo.com/docs/fleet-management/
[selfhost]: https://rakazo.com/docs/self-host/
[providers]: https://rakazo.com/docs/self-host-sandbox-providers/
[audit]: https://rakazo.com/docs/self-host-audit-log/
