# OpenBot（CopilotKit）

確認日：2026-11-11（JST、日本時間）

一覧と比較の前提は[競合調査](../competitors.md)を参照する。「確認」は公式資料・公開リポジトリに記載があるという意味で、実機検証済みではない。「未確認」は今回の資料で裏付けられなかった項目で、機能がないという意味ではない。

## 概要

| 項目 | 内容 |
|---|---|
| 提供元 | CopilotKit。READMEはCopilotKitをAG-UIの開発元と紹介している。[README][repo] |
| 公式URL | [open.bot/business][biz]、[CopilotKit/OpenBot][repo] |
| 製品の説明 | AIの同僚（Bot）ごとにブラウザ・ファイル・ツールを持つコンピューターを与え、すべての操作を実行前に判定し、実行後に記録するセルフホスト可能なworkspace。自作のAG-UI agentを持ち込める。[README][repo] |
| 提供段階 | Mac・Windowsのデスクトップアプリは配布中。iOS・Android・Slack・Teamsはearly access、Webは「Coming soon」。ホスト版はearly access。[Business公式][biz] [料金][price] |
| ライセンス | アプリのテンプレートはMIT。永続的な会話・記憶などを担うCopilotKit Intelligenceは別ライセンスで、ローカル利用は無料のライセンス、Kubernetesでの本番利用は本番ライセンスが必要。[README][repo] |
| 価格 | 個人利用は無料（モデル費は提供元が別請求）。OpenBot for Business Pay as you goは「Up to $29 / user / month」（説明文は「per active user per month」）、チームが大きくなると単価が下がる、カードで月払い、契約期間の縛りなし。Annualは個別見積もりで、SSO・RBAC、オフライン・エアギャップ環境、24時間サポートなどを含む。両プランとも組織ごとの月間モデル利用額が$5,000を超えた分に5%の利用料。セルフホストとホスト版は同じプラン価格。[料金][price] |

## 機能

| 観点 | 内容 | 出典 |
|---|---|---|
| 複数人＋複数AIの同一チャンネル参加 | 宛先（To:）に2つ以上のBotを入れるとグループ会話になる。グループにはメールアドレスで人を追加でき、追加された人は共有の会話記録全体を見られる。各Botは自分専用のスレッドで動き、人が読むのは発言者ごとに記録された共有の会話記録。 | [README][repo] [architecture][arch] [group.ts][group] |
| AIの役割・専門性の設定 | Botは名前、肩書き、役割の説明を持つ。役割の説明は毎回の実行にsystemメッセージとして送られる。権限は役割とは別に、ツール・アプリ・スキルごとに付与する。Businessの事例では、1人の経営者と20のAI同僚の構成を紹介し、オーケストレーターが専門のエージェントに仕事を割り振り、受け渡しを調整する。 | [coworkers][cw] [Business公式][biz] |
| 実行能力 | Botごとのコンピューター（Chromium、ファイル、シェル）で閲覧、フォーム入力、コマンド実行を行う。READMEでは、標準は人ごとに1コンテナで、その中にBotごとのフォルダーとブラウザープロファイルを持ち、「Isolated」を付けたBotは専用コンテナを持つ。gVisor（`runsc`）での実行も選べる。定期実行（Routines）は15分間隔が下限、有効な定期実行は20件まで。 | [README][repo] [architecture][arch] |
| 承認・権限管理 | 削除、他人への送信、共有、投稿、支払い、パスワード・アカウント設定の変更は、操作の名前やボタンのラベルから判定して承認を求める。判定順は、操作の種類 → 個人・管理者・会話ごとのルール（最も厳しいものが優先）→ 自動レビュー → 「Extra vigilant」。ルールはCELで書き、拒否を先に評価し、ポリシーがなければ何も許可せず、壊れたルールは拒否する（fail closed）。ブラウザ・ファイル操作はすべてサーバーのゲートウェイを通り、判定と監査記録の後でだけ実行される。 | [approvals][appr] [README][repo] [architecture][arch] |
| 管理者による制御 | 操作の種類ごとに「Members choose」「Ask every time」「Members do it themselves」（Botには実行させず人が行う）を選べる。SSO（Google、Microsoft、Okta、SAML、OIDC）、SCIM、役割・グループ別の権限。 | [approvals][appr] [README][repo] [Business公式][biz] |
| エージェント間の引き継ぎ | Botは `message_bot` で別のBotに仕事を渡す。渡す内容は、タスク、制約、良い答えの条件に分けた型付きの入力。宛先、回答の届け先、依頼元、連鎖の深さはモデルではなくサーバーが決める。深さと1回あたりの件数に上限がある。回答は、依頼した会話に回答したBotの名前で書き込まれる。最終的に失敗した引き継ぎは、依頼したBotが会話の中で失敗を伝える。 | [architecture][arch] |
| 人への質問 | `ask_person` で判断を人に戻せる。テンプレートでは会話にいる人が相手。質問は `agent.escalated`、届かなかった質問は `agent.escalation_failed` として記録する。 | [architecture][arch] |
| 外部連携 | AG-UI（LangGraph、Mastra、CrewAI、Pydantic AI、Google ADK、Claude Agent SDKなど）、MCP、Composio・Arcade経由のアプリ連携、Google Drive・Notion。Slack・Teams・SMSで会話を続ける機能。 | [README][repo] [Business公式][biz] |
| 監査 | 許可・拒否・失敗した操作を記録し、拒否には原因のルールを付ける。各行に「何が実行を始めたか」（人、定期実行、目標、記憶、引き継ぎ）を持たせ、誰も見ていない実行を絞り込める。SIEMへの書き出し。 | [README][repo] [architecture][arch] [Business公式][biz] |

## UI/UX

| 要素 | 内容 | 出典 |
|---|---|---|
| 画面構成 | `/` でチャンネルの開始と一覧、`/channel/:id` で1体のBotとの会話、画面の確認、実行内容の確認、`/bots` でBot一覧と作成。管理画面は `/admin/*` に、承認、境界（ポリシー）、監査、コンピューター、プラグインなどが並ぶ。 | [README][repo] |
| Botの作業の見え方 | 会話の横にComputerパネルがあり、Screen（ライブのブラウザ画面）とTerminal（Botのシェル）の2タブ。その下のActivityに、実行したコマンドと出力、読み書きしたファイルを新しい順に表示する。保存したファイルはパスとサイズだけを表示し、中身は出さない。 | [README][repo] [architecture][arch] |
| 人による操作の引き継ぎ | ログイン画面や2要素認証でBotが止まると助けを求め、同じパネルで人が操作を引き取る（Take control）。人が操作している間、Botの操作はキューに入れずに拒否する。 | [README][repo] |
| 承認カード | 「Allow once」「Allow for this conversation」「Always allow here」「Deny」の4択。会話単位で許可している内容は入力欄の上に表示され、「Turn off」で取り消せる。Webサイトでのパスワード変更は人に渡され、「I did it myself」「Don't do it」の2択になる。 | [approvals][appr] |
| Botごとの画面 | `/bots/:id` に、そのBotが人を待っていること、実行中のこと、設定をまとめる。`/bots/:id/approvals` で、確認なしに実行してよいことを設定する。 | [README][repo] |
| エージェント作成 | `/bots` で名前と指示（役割の説明になる）、公開範囲（private / public）、任意のAG-UIエンドポイント、任意の認証ヘッダーを入力する。保存前に接続テストができる。YAMLでも定義できる。 | [README][repo] [coworkers][cw] |
| スキル作成 | `skill-creator` を持つBotが会話の中で利用者に聞き取りをしてスキルを書き、カード上のボタンが押されたときだけ保存する。 | [README][repo] [architecture][arch] |

## 技術

公開リポジトリの依存定義と文書から確認した。

| 層 | 技術 | 出典 |
|---|---|---|
| 言語 | TypeScript が中心。Rust（デスクトップ）、Python（エージェントの例）。 | [README][repo] [src-tauri Cargo.toml][tauri] |
| サーバー | Bun、Hono、CopilotKit runtime、Drizzle ORM、better-auth（SSO、SCIM）、cel-js（ポリシー）、OpenTelemetry。 | [server package.json][srvpkg] [README][repo] |
| DB | PostgreSQL（製品データ、監査、ポリシー、資格情報、作業キュー）。会話の永続化と記憶はCopilotKit Intelligence。 | [README][repo] [architecture][arch] |
| リアルタイム | サーバーの複数レプリカ間はPostgreSQLの `LISTEN`/`NOTIFY`。Botの画面はWebSocket経由でプロキシする。Intelligenceがリアルタイムのゲートウェイを持つ。ブラウザへのチャンネル更新の配信方式の詳細は未確認。 | [architecture][arch] [events.ts][events] |
| エージェント連携 | AG-UI（`@ag-ui/client`、`@ag-ui/mastra`）、MCP（`@modelcontextprotocol/sdk`）、Composio、Arcade。 | [server package.json][srvpkg] |
| 実行環境 | Botのコンピューターは Playwright と Chromium を載せたコンテナ。supervisorがDocker（dockerode）でコンテナを作る。gVisorを選べる。 | [agent-computer package.json][acpkg] [supervisor package.json][suppkg] [README][repo] |
| フロントエンド | React、Vite、TanStack Router・Query・Form・Table、Base UI、shadcn、Tailwind CSS、xterm.js、streamdown。 | [app package.json][apppkg] |
| デスクトップ・モバイル | Tauri 2。Expo（React Native）。 | [src-tauri Cargo.toml][tauri] [mobile package.json][mobpkg] |
| 運用 | Docker Compose、単一コンテナのイメージ、Helmチャート（EKS、AKS、GKEなど）。 | [README][repo] |

## Pelwayとの比較［提案］

この節は本書の整理と提案であり、OpenBotの資料が述べている内容ではない。Pelway側は[ビジョン](../vision.md)、[PRD](../prd.md)、[技術](../technical.md)の構想で、実装済みの機能ではない。

| 観点 | OpenBot | Pelway（構想） | Pelwayでの扱い |
|---|---|---|---|
| 人とAIの同席 | グループ会話に複数Botと複数人を入れられる | 中核の方針 | 同席だけでは差別化にならない |
| 外部操作前の承認 | 操作の種類で判定する承認カードとCELのポリシー、fail closed | 承認を実行内容に結び付ける | 判定の順序と4択のカードは参考になる。承認した中身（本文・宛先・添付）を固定し、変わったら再承認にする点は資料で確認できず、比較の観点にする |
| 承認の対象を判定する方法 | 操作の名前とボタンのラベルで判定。ラベルのないボタンなどは判定できないと明記 | 外に出る操作を状態として持つ | 名前による判定の取りこぼしを、状態遷移で外に出る操作を必ず通す方式と比べる |
| エージェント間の受け渡し | 型付きの依頼、深さの上限、失敗の通知、監査 | 引き継ぎ欄（決定、送信内容、残作業） | 型付きの依頼と「失敗を会話で伝える」は取り入れる候補。人への引き継ぎ欄はPelwayで具体化する |
| 判断待ち | `ask_person` と Botごとの「待っていること」の画面 | 「今日」画面に誰の判断か・何を決めるかをまとめる | チャンネル横断の一覧が資料で確認できない点を比較の観点にする |
| 作業状態の表示 | ライブ画面、ターミナル、Activity | 静止画と手順のログを候補にし、ライブ映像は約束しない | Activityの形式（コマンドと出力、ファイルはパスとサイズだけ）は参考になる |
| 人による操作の引き継ぎ | Take controlとその記録 | 構想の文書で未確認 | ログインや2要素認証の扱いとして参考にする |
| 料金 | 人単位（最大$29）＋モデル費の5%（$5,000超） | 人の参加費とAI実行枠を分ける案 | 人単位の料金とモデル費の分離を比較に使う |

## 未確認事項

- コンピューターの単位。READMEの機能説明は人ごとに1コンテナ（Isolatedで Botごと）、構成図の説明はBotごとに1コンテナと書いている。
- チャンネル更新をブラウザへ届ける方式の詳細。
- ホスト版の提供範囲、保存地域、SLA。Pay as you goの人数別の単価。
- グループ会話で、承認者を依頼者以外にできるか。
- 日本語対応。

## 出典

- [CopilotKit：OpenBot README](https://github.com/CopilotKit/OpenBot)
- [CopilotKit：OpenBot for Business](https://open.bot/business)
- [CopilotKit：OpenBot pricing](https://open.bot/pricing)
- [CopilotKit：OpenBot docs/approvals.md](https://github.com/CopilotKit/OpenBot/blob/main/docs/approvals.md)
- [CopilotKit：OpenBot docs/coworkers.md](https://github.com/CopilotKit/OpenBot/blob/main/docs/coworkers.md)
- [CopilotKit：OpenBot docs/architecture.md](https://github.com/CopilotKit/OpenBot/blob/main/docs/architecture.md)
- [CopilotKit：OpenBot server/src/channels/group.ts](https://github.com/CopilotKit/OpenBot/blob/main/server/src/channels/group.ts)
- [CopilotKit：OpenBot server/src/channels/events.ts](https://github.com/CopilotKit/OpenBot/blob/main/server/src/channels/events.ts)
- [CopilotKit：OpenBot server/package.json](https://github.com/CopilotKit/OpenBot/blob/main/server/package.json)
- [CopilotKit：OpenBot app/package.json](https://github.com/CopilotKit/OpenBot/blob/main/app/package.json)
- [CopilotKit：OpenBot mobile/package.json](https://github.com/CopilotKit/OpenBot/blob/main/mobile/package.json)
- [CopilotKit：OpenBot agent-computer/package.json](https://github.com/CopilotKit/OpenBot/blob/main/agent-computer/package.json)
- [CopilotKit：OpenBot supervisor/package.json](https://github.com/CopilotKit/OpenBot/blob/main/supervisor/package.json)
- [CopilotKit：OpenBot desktop/src-tauri/Cargo.toml](https://github.com/CopilotKit/OpenBot/blob/main/desktop/src-tauri/Cargo.toml)

[repo]: https://github.com/CopilotKit/OpenBot
[biz]: https://open.bot/business
[price]: https://open.bot/pricing
[appr]: https://github.com/CopilotKit/OpenBot/blob/main/docs/approvals.md
[cw]: https://github.com/CopilotKit/OpenBot/blob/main/docs/coworkers.md
[arch]: https://github.com/CopilotKit/OpenBot/blob/main/docs/architecture.md
[group]: https://github.com/CopilotKit/OpenBot/blob/main/server/src/channels/group.ts
[events]: https://github.com/CopilotKit/OpenBot/blob/main/server/src/channels/events.ts
[srvpkg]: https://github.com/CopilotKit/OpenBot/blob/main/server/package.json
[apppkg]: https://github.com/CopilotKit/OpenBot/blob/main/app/package.json
[mobpkg]: https://github.com/CopilotKit/OpenBot/blob/main/mobile/package.json
[acpkg]: https://github.com/CopilotKit/OpenBot/blob/main/agent-computer/package.json
[suppkg]: https://github.com/CopilotKit/OpenBot/blob/main/supervisor/package.json
[tauri]: https://github.com/CopilotKit/OpenBot/blob/main/desktop/src-tauri/Cargo.toml
