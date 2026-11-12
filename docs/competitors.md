# Pelwayの競合調査

確認日：2026-11-08（JST、日本時間）

## 前提と調査方法

Pelwayは、複数の人間と複数の専門AI同僚が同じチャンネルで働く、独立したビジネスチャットを構想する。AIはAIだと明示する。ブラウザ版を先行し、Rust製デスクトップ版は将来の対象とする。AIごとのVM相当の実行環境、外部ツール連携、権限と承認を検討する。これは比較用のPelway側の構想であり、実装済み機能の説明ではない。**社内wikiと外向けQ&Aの自動生成は対象外**とする。

競合は公式製品ページ、公式ドキュメント、開発者のApp Store掲載説明で確認した。名称衝突に限り、第三者掲載情報を区別して使う。「確認」は公式資料に記載があるという意味で、実機検証済み・安全性監査済みではない。画面例やデモの説明も含む。

「未確認」は今回の資料で裏付けられなかった項目を指し、機能が存在しないという意味ではない。複数モデルの選択と、複数AIの同席も区別する。VM、コンテナ、タスク別sandbox、ローカルPCは別の構成であり、「AIがPCを使う」だけでAI別の分離を推定しない。

価格の確認日は2026-11-08（JST）。Google、Grok Bot、OpenBot、Otti、Bullpenの価格は2026-11-11（JST）に再確認した。Rakazoの価格は2026-11-13（JST）に確認した。金額は公式の米ドル表示を使い、円換算はしない。契約期間と追加費用を区別し、最安総額の順位は作らない。提供元は公式のブランドを基本とし、運営法人名を確認できない場合は明記する。

## サービス別の比較ドキュメント

各サービスの機能、UI/UX、技術、Pelwayとの比較は、サービスごとの文書にまとめている。下の表より後の確認日の情報を含み、内容が下の表と異なる場合はサービス別の文書の出典を優先して確認する。

| 区分 | 文書 |
|---|---|
| 既存チャットと個人・実行型エージェント | [Slack / Agentforce](competitors/slack-agentforce.md)、[Teams / Copilot](competitors/teams-copilot.md)、[Google Chat / Gemini](competitors/google-chat-gemini.md)、[Grok Bot](competitors/grok-bot.md)、[Muse](competitors/muse.md)、[OpenBot](competitors/openbot.md)、[Reason Machines](competitors/reason-machines.md)、[Rakazo](competitors/rakazo.md) |
| 共有チャット・業務workspace | [Buzz](competitors/buzz.md)、[Otti ai](competitors/otti.md)、[CircleChat](competitors/circlechat.md)、[Context](competitors/context.md)、[Mantle Chat](competitors/mantle-chat.md)、[Cosmo](competitors/cosmo.md)、[GroupChat](competitors/groupchat.md)、[Bullpen](competitors/bullpen.md)、[GoodStrata](competitors/goodstrata.md)、[Elsewhere](competitors/elsewhere.md)、[Rerun](competitors/rerun.md) |

## 競合機能比較

### 既存チャットと個人・実行型エージェント

| 製品・提供元と公式URL | 複数人＋複数AIの同一チャンネル参加 | AIの役割・専門性の設定 | 実行能力（VM・PC・コード・ツール） | 承認・権限管理 | 外部連携（MCPなど） | 公式価格・条件（2026-11-08 JST確認） |
|---|---|---|---|---|---|---|
| Slack / Agentforce。Salesforce・Slack。[公式ガイド][slack] | チャンネルにagentを追加し、チームがメンションできる。同一チャンネルの複数AI間の協働挙動は未確認。[利用ガイド][slack] | 社内でIT支援や福利厚生など目的別に作成。[利用ガイド][slack] | Agentforceのレコード更新、Flow、カスタムプロンプトなど。AI別の汎用VM・画面操作は未確認。[Agentforce][afprice] | 管理者が利用者を割り当て、回答は依頼者のSlack・Salesforce権限に従う。各書き込みの実行前承認は未確認。[利用ガイド][slack] | Salesforce、Slackアプリ、ワークフロー。Slack MCP Server（MCPクライアントからSlackを検索・投稿）とSlackbot MCP Client（リモートMCPサーバーをSlackbotにつなぐ）を確認。Agentforceのエージェント経路のMCPは未確認。[料金・機能表][slackprice] [Slack MCP Server][slackmcps] [Slackbot MCP Client][slackmcpc] | Slack通常月払いPro $8.75/人/月、Business+ $18/人/月。Agentforceは別ライセンスが必要。User License $5/人/月＋Flex Credits、Flex Credits $500/100,000 credits。適用SKUと総額は要確認。[Slack][slackprice] [Agentforce][afprice] [利用条件][slack] |
| Teams / Microsoft 365 Copilot・Copilot Studio。Microsoft。[公開手順][teams] | チャンネル・グループ・会議チャットへのagent追加を確認。同席AI数は未確認。他agent・botのメッセージは全メッセージ受信権限でも配信されないと明記。[公開手順][teams] [会話仕様][teamsbots] | カスタムagent、専門知識・actionに応じた複数agentの連携。[製品説明][studio] | ツールとワークフローで業務実行。AI別の汎用VMは未確認。[製品説明][studio] | 公開・配布の管理者承認、アクセス設定、データポリシー。M365宣言型agentのMCP/API操作確認もあるが、全Teams agentに同じ方式が適用されるとは断定しない。[公開手順][teams] [MCP][msmcp] [確認仕様][msapprove] | Copilot StudioのMCP接続とコネクタ。MCPもデータポリシーの対象。[MCP接続][msmcp] | M365 Copilot $30/人/月・年払い。対象M365とTeamsライセンスが必要。Copilot Studioは$200/25,000 Copilot Credits/月または従量課金。含有範囲はライセンス・実行先に依存し、単純加算を総額とはしない。[Copilot価格][msprice] [Studio料金FAQ][studio] |
| Google Chat / Gemini。Google。[公式発表][google] | Ask GeminiはChatのShortcutsから個別セッションで利用。複数専門AIがチャンネルメンバーとして同席する仕様は未確認。[公式発表][google] | 専門AIの複数配置は未確認。Ask Gemini導入時、ChatのGeminiサイドパネルからGemsへアクセスできなくなると記載。[公式発表][google] | Workspace横断検索、画像・更新文生成、会議・タスク管理。汎用VM・コード実行は未確認。[公式発表][google] | 管理者のGemini・Workspace Intelligence設定が利用条件。操作別承認は未確認。[公式発表][google] | Gmail、Drive、Calendar。Ask Gemini in Chatの任意MCP接続は未確認。[公式発表][google] | 対象はBusiness Standard/Plus、Enterprise Standard/Plusなど。Business Standard $16.80/人/月（Flexible、契約期間なし）・$14/人/月（Annual、1年以上の契約）、Business Plus $26.40・$22。Annualは期間中にユーザーを減らせない。Enterpriseは営業・販売店経由。[対象条件][google] [支払いプラン][googleplans] [公式料金][googleprice] |
| Grok Bot。Cursor公式とx.ai公式資料で同定。[公式概要][grok] | 複数Botのgroupと引き継ぎを確認。標準groupへの複数人参加は未確認。共有BotのSlackチャンネル・group DM利用は説明あり。[概要][grok] [作業ガイド][grokwork] [組織向け][grokadmin] | 名前、仕事、役割、記憶、スキル、ルーティンを設定。[作業ガイド][grokwork] | 永続クラウドPCのブラウザ・ファイル・terminal。同一利用者のBotはPCとログインを共有し、画面はBot別。利用者間はFirecracker microVMで分離。共有Botには別条件がある。[作業ガイド][grokwork] [構成][grokadmin] | Auto-reviewと機微操作の承認、チーム・コネクタ管理。ネットワーク制御、Auto-review強制、アクション記録などはEnterprise限定。[組織向け][grokadmin] | MCP接続アプリ、ブラウザ、Cloud Agentへの開発委任。[組織向け][grokadmin] | 公式料金表はPremium $30/月、Super $100/月、Ultra $200/月、Teams $30〜/人/月にBotを表示。利用枠は他製品と共有。料金表のFree $0/月・Plus $8/月の欄にGrok Botの表示はない。概要は「全有料個人プラン」と書いており、料金表と記載が異なる。[価格][grokprice] [概要][grok] |
| Muse。Meta。[公式発表][muse]・[muse.ai](https://muse.ai/) | 個人用agentとして説明。複数人＋複数専門AIの同一チャンネル参加は未確認。[発表][muse] [利用ガイド][museguide] | 名前・性格・好みの変更。複数の専門職AI配置は未確認。[利用ガイド][museguide] | Muse Secure VMの専用クラウドPC・ブラウザ、アプリ操作、文書生成。[発表][muse] [利用ガイド][museguide] | メール・購入などの確認、アクセス変更・解除、活動履歴、モデル外の重要な検査。確認要否は操作・connector・設定に依存。[安全性説明][musesafety] | 利用者が選ぶConnectorsとブラウザ。任意MCP接続は未確認。[安全性説明][musesafety] | 上限付き無料、Power $20/月、Maximum $100/月。地域・アカウントにより提供条件が変わる。[公式料金][museprice] |
| OpenBot。CopilotKit。[Business公式][openbot] | 共有Bot、team workspace、Slack・Teams利用。宛先に2つ以上のBotを入れるとグループ会話になり、メールアドレスで人を追加できる。[Business公式][openbot] [グループ会話の実装][opengroup] | 自作AG-UI agent、組み込みBot、専門職とオーケストレーターの構成。[Business公式][openbot] | ブラウザ、ファイル、許可ツール、人による画面の引き継ぎ。ローカル・自社Kubernetes・ホスト運用。AI別VMの分離仕様は未確認。[Business公式][openbot] [公式README][openrepo] | Bot別ツール許可、禁止・承認ポリシー、監査、SSO・SCIM。[Business公式][openbot] | MCP、AG-UI、Slack、Teams、既存agent基盤の持ち込み。[Business公式][openbot] | MITライセンス、個人利用は無料でモデル費は提供元で別請求。Business Pay as you goは最大$29/利用者/月（月払い、契約期間の縛りなし）、Annualは個別見積もり。両プランとも組織の月間モデル利用額が$5,000を超えた分に5%。[公式README][openrepo] [公式料金][openprice] |
| Reason / Reason Network。Reason Machines。[公式][reason] | teammate招待とagent間通信を確認。同一業務チャンネルで複数人＋複数AIが常駐する仕様は未確認。NetworkとReason本体を区別。[公式][reason] [プラン][reasonprice] | 名前と継続指示を持つReasonを職務別に作成、schedule・eventで起動。[AI employees][reasonjobs] | 自分のPCとcloud PCのコード編集・command実行、ブラウザ、ファイル、PR作成。[利用ガイド][reasonguide] [AI employees][reasonjobs] | 接続アカウントの権限。Gmail送信はプレビュー承認。他の重要操作はworkspaceルール・指示で承認を設定すると説明。[AI employees][reasonjobs] | Slack、Google Workspace、Stripe、PostHog、GitHub。開発連携にMCP・API・CLI。Networkは他agentとの通信。[利用ガイド][reasonguide] [AI employees][reasonjobs] | Pro $50/人/月、Max $200/人/月。週次creditsと追加購入、持ち込みモデルは提供元で別請求。Reason Networkは別の無料サービス。[公式プラン][reasonprice] |
| Rakazo。開発はelie222（Elie Steinbock）、維持はInbox Zero Inc.。Apache-2.0で公開。[公式][rakazo] [リポジトリ][rakazorepo] | group chatは自分のボット2〜10体で、@Name・@everyoneで呼ぶ。複数人が同じgroup chatに入る仕様は未確認。共有スペースではメンバーがボットとチャットを共有し、ボットはメッセージを送った人のアカウントで動く。[グループ][rakazogroups] [スペース][rakazospaces] | ボットごとに名前・指示（上限2万字）・モデル・記憶（MEMORY.md、1行1事実・上限4 KB）を設定。テンプレートとスキル、ボットによるボット管理もある。[ボットとチャット][rakazobots] [ボット管理][rakazofleet] | ボットごとのコンピュータ（ブラウザ・ターミナル・ファイル・デスクトップ）。スペースで1台のTeam ComputerまたはPrivate。サンドボックス提供元は13種で、デスクトップアプリの自分のコンピュータも使える。[コンピュータ][rakazocomputer] [提供元][rakazoproviders] | Ask、Auto（承認モデルが依頼との一致を審査）、Never askの3モード。Allow once・Always allow（ツール・サイト単位）、Blockリスト優先、支出上限。カードへの応答はメッセージを送った本人だけ。承認者の別指定は未確認。[承認][rakazoapprovals] [スペース][rakazospaces] | カタログ連携（セルフホストではComposio/Pipedreamの設定時のみ）、任意のHTTPS MCPサーバー、OpenAPI・GraphQL。資格情報は暗号化。監査ログは常時有効で更新不可。[連携][rakazointegrations] [監査][rakazoaudit] | セルフホスト無料（Apache-2.0）。ホスト版はStandard $15/月（年払い$12/月、コンピュータ25時間）、Pro $49/月（100時間）、Max $99/月（200時間）、Rakazo creditsは$10/月から。7日間試用。2026-11-13 JST確認。[料金][rakazopricing] |

`Muse`はMetaの製品、`open bot`はCopilotKitのOpenBotを、公式に確認できる比較対象として採用した。同名の別製品を混ぜていない。ただし元の表記だけから、依頼対象との対応を一意に確定したとは扱わない。[Meta][muse] [CopilotKit][openbot]

`reasone machine`と`reasoning machine`という表記自体は、提供元・公式URLとの一意の対応が未確認。上表のReason Machinesは関連名称と機能を持つ候補として、正式名称を分けて掲載した。元の表記と同一製品とは断定しない。[Reason Machines][reason]

### 共有チャット・業務workspace

| 製品・提供元と公式URL | 複数人＋複数AIの同一チャンネル参加 | AIの役割・専門性の設定 | 実行能力（VM・PC・コード・ツール） | 承認・権限管理 | 外部連携（MCPなど） | 公式価格・条件（2026-11-08 JST確認） |
|---|---|---|---|---|---|---|
| Otti ai。運営Pardes Communities Ltd.。App Store開発者・販売者表示はRaz Noiman。[公式掲載][otti] | グループ・DMで名前を呼ぶと参加。「one AI at its heart」と説明。複数専門AIの同席は未確認。[公式掲載][otti] | 同じAI companionの記憶・好みの管理。職務別AI作成は未確認。[公式掲載][otti] | reminder、調査、下書き、計画と進捗。VM・任意コード実行は未確認。[公式掲載][otti] | 別chat・mail・文書由来の内容の送信確認カード、承認済み受信者、目標のApprove/Changeを更新履歴で説明。組織RBACは未確認。[公式掲載][otti] | WhatsApp接続・同期、mail関連機能。MCPは未確認。[公式掲載][otti] | 無料＋アプリ内課金。米国表示Pro $19.99、Unlimited $200、credit 10/20/50が$10/$20/$50、Business 25/50/120が$150/$250/$500。期間・含有量・Businessの内容は未確認で、月額とは断定しない。[公式掲載][otti] |
| CircleChat。Neu Software LLC。ソースはMITライセンスで公開。[公式][circle] [README][circlerepo] | 複数人・複数agentのチャンネル例、メンション、agent間引き継ぎを明示。[公式][circle] | identity・skills・toolsをファイル定義。宣言した専門skillに応じたtask割り当て。[公式][circle] | 計画、成果物、検証、deployを説明、self-host可能。AI別VM・コンテナ分離は未確認。[公式][circle] | リスク操作の人間承認、server側で強制するagent別権限、run・成果物・判断の記録。[公式][circle] | 汎用HTTPとリモートMCPのコネクター登録、OAuth 2、署名付きwebhook。[README][circlerepo] | managed workspaceはStarter $29/月・3 agents、Team $79/月・10 agents、Scale $199/月・無制限agents。モデルキー持ち込み、token上乗せなし。self-host無料。[公式料金][circle] |
| Context。Explore Interfaces Inc.。[Workspace公式][context] [規約][contextterms] | 同じchannelの人4人＋AI3体という画面例、名前付きagentをmemberとして扱う。[Workspace公式][context] | agentとSkillsの設定・共有、仕事別割り当て、Skillsの版管理。[Workspace公式][context] | Engineのtask別sandbox、ファイル生成、コードによるapplet作成。Enterpriseは自社Kubernetes。AI別の永続VMとは説明していない。[Workspace公式][context] | 依頼者の権限・認証。書き込みは原則承認、対象file・commandをカードに表示、許可・拒否を記録。常時許可の取消も例示。[Workspace公式][context] | Unifyが利用者自身のアカウントでappsへ接続。管理者は社内のHTTP・MCPサービスをconnectorとして登録できる。利用者が任意のMCP serverを直接つなぐ機能は未確認。[Workspace公式][context] [Connectors][contextconn] | Free $0、Plus $20/人/月、Enterprise個別相談。plan別の実行上限やモデル費の詳細は未確認。[Workspace公式][context] |
| Mantle Chat（運営法人名未確認）。[公式][mantle] | teamのchannelでAIにメンション、複数の共有agentを作成。モデル切替だけでなく名前付きagentの共有を説明。[公式][mantle] [Agents][mantleagents] | 名前・役割・指示・model・参照資料・toolsを設定。[Agents][mantleagents] | 外部ツール、定期・event task。Mac版はlocal coding agentのcommand・file編集。cloud上のAI別VMは未確認。[公式][mantle] [desktop][mantledesktop] | 接続は利用者別。local agentはAsk firstが標準。MCP経由の削除・アクセス変更・channel投稿にも承認。hosted AIの全外部操作が同じ承認方式かは未確認。[desktop][mantledesktop] [安全性][mantlesafety] [更新][mantleupdate] | apps、API、bot、webhook。外部client向けMCP serverを提供。任意MCP serverの取り込みとは区別する。[公式][mantle] [更新][mantleupdate] | Free。年払いCore $10/人/月・年$120、Plus $25/人/月・年$300、Max $99/人/月・年$1,188。AI credits・接続数に上限。追加10,000 credits $12。税別。[料金][mantleprice] |
| Cosmo。Perséides。[公式][cosmo] | channel・DM・threadにagentがmemberとして参加。AIと明示し、人と同じ権限と説明。[公式][cosmo] | agentへのメンション、model provider選択を確認。職務テンプレートや専門性設定の詳細は未確認。[公式][cosmo] | 定期task、連携利用、workspace用plugin生成。VM・汎用PC操作は未確認。[公式][cosmo] | 人と同じ権限、全actionのworkspace log。操作別の実行前承認は未確認。[公式][cosmo] | 付与したintegrationsと生成plugin。管理者が割り当てたMCP serverをstdio・SSEで使う。[公式][cosmo] [設計記事][cosmoblog] | 招待制private beta。数値料金は未確認。[公式][cosmo] |
| GroupChat（運営法人名未確認）。[公式][groupchat] | 人と複数agentの共同workspace、task進捗を説明。会話channelに全員が同席する仕様は未確認。[公式][groupchat] | 持ち込みagent。code review・test・deployのmanaged専門agentも案内、提供段階詳細は未確認。[公式][groupchat] | 接続agentがtaskを実行。定期実行・stage遷移。GroupChatがAI別VMを提供するかは未確認。[公式][groupchat] | PR準備・mergeに応じたstage。操作前承認と実効権限は未確認。[公式][groupchat] | Cursor、Claude、Codex、OpenClaw、custom agent、webhook、API。[公式][groupchat] | workspace $0、projects・tasks・agents無制限。持ち込みagent費は提供元へ。managed agentは使用tokenに課金と説明、単価未確認。[公式][groupchat] |
| Bullpen。Bullpen Chat LLC。[公式][bullpen] [Privacy][bullpenprivacy] | 人のteamのchannelに6 AI helpers、メンションに応答。独自helperも追加可能。[公式][bullpen] | 検索、会議メモ、進捗要約、reminder、導入支援、file作成を職務分担。[公式][bullpen] | 有料planでfile読取、表計算・文書・slides・PDF生成。VM・汎用PCは未確認。[公式][bullpen] | 依頼者と投稿先の閲覧権限で回答を制約。回答とcostを記録。外部操作前承認は未確認。[公式][bullpen] | 外部SaaS・MCPの具体的対応は未確認。[公式][bullpen] | bullpen単位の料金で、人の席にhelpersは含まない。Free $0（最大5人）、Starter $29/月（最大10人）、Team $79/月（最大25人）、Business $199/月（最大75人）、いずれもBilled monthly。年額は未確認。音声minutes別カウント、必要時課税。[公式FAQ][bullpen] |
| GoodStrata。Good Strata Pty Ltd。ソースはApache-2.0で公開。[公式][goodstrata] [リポジトリ][goodstratarepo] | strata管理用の業務製品。複数人＋複数AIの同一chatは未確認。[公式][goodstrata] | 徴収・入金照合・滞納・修繕・通知を担当。汎用職務作成は未確認。[公式][goodstrata] | agent業務と決定論的な金額計算コードを分離。self-hostはPostgreSQL＋Node、local model対応。AI別VMは未確認。[公式][goodstrata] | 支出・契約・ルール例外は根拠付きdecision inboxで人が承認。改変できないevent logを説明。[公式][goodstrata] | 口座への直接払い・照合を説明。MCPサーバーを持ち、お金や法定手続きのツールは専用スコープと、試算後の署名付き確認トークンを要求する。[公式][goodstrata] [リポジトリ][goodstratarepo] | owners corporation向けself-hostまたはhosted無料tier。現時点の管理費・subscription等を取らないと説明。将来の有料managed serviceは予告で価格未確認。[公式][goodstrata] |
| Elsewhere（運営法人名未確認）。[指定公式URL][elsewhere] | 現在の指定URLはcapacity planning製品。人と複数AIの同一channel参加は未確認。[公式][elsewhere] | 人員の空き時間・計画・負荷を管理。専門AI配置は未確認。[公式][elsewhere] | 計画リスクの早期検知を説明。VM・PC・code実行は未確認。[公式][elsewhere] | AI操作承認・権限仕様は未確認。[公式][elsewhere] | Linear、Jira、Slackの同期。MCPは未確認。[公式][elsewhere] | 公式topで数値料金は未確認。[公式][elsewhere] |
| 追加比較：Rerun。Reunit SA。ソースはAGPL-3.0（`ee`ディレクトリを除く）で公開。[公式][rerun] [規約][rerunterms] [リポジトリ][rerunrepo] | team招待とAI間引き継ぎを説明。同一会話channelへの複数人＋複数AI参加は未確認。主にboardとBoxesを提示。[公式][rerun] | 専門職Coworker、自然言語のjob、AIごとのmodel選択。[公式][rerun] | cloud PCの画面操作・継続実行・file保持。分離単位はworkspaceのmachineと説明。[公式][rerun] | 製品として承認を強制するのは支出。送信・削除などで止まるかは利用者の指示による。止まったrunはworkspaceの誰でも承認でき、最初の応答で再開する。画面引き継ぎ・run履歴。[公式][rerun] [Human in the loop][rerunhitl] | MCP、apps、schedule、webhook。[公式][rerun] | 現行FAQは$29/workspace/月、Coworkers・人のteam無制限、private machine込み。別のRerun creditsはmodelだけでなく外部tool要求にも消費。[公式FAQ][rerun] |

### 比較上の注意

ContextとCircleChatは、共有channel、専門AI、実行、承認を同じ製品の説明に含めている。「AI同僚がchannelにいる」「承認がある」だけをPelway独自の機能とは説明できない。[Context][context] [CircleChat][circle]

Teamsには他agentのメッセージ配信制約に加え、Copilot Studio agentをgroup chat・channelで使う場合、利用者認証が必要なSharePoint等のknowledge sourceを使えないという制約が記載される。これは特定経路の制約であり、Microsoft製品全体の非対応という意味ではない。[Teams会話仕様][teamsbots] [Copilot Studio制約][teams]

Cosmoはprivate beta、OpenBot Businessはearly access。公開説明を一般提供・本番導入の保証へ読み替えない。Elsewhereは指定URLの現行製品を記載し、同名の別サービスを混ぜていない。[Cosmo][cosmo] [OpenBot][openbot] [Elsewhere][elsewhere]

## 名称衝突

名称の既存利用と機能上の競合は別の調査である。ここではPelway関連だけを記録する。

| 名称・URL | 再確認したこと | 出典の確度・未確認点 |
|---|---|---|
| PELWAY 普惠智能课教云 / [pelway.cn][pelwaycn] | 第三者2件が英字PELWAYを使い、K12数理化教材、AIなどを使う個別診断・授業支援を説明。MDPDAは公式URLをpelway.cn、爱东东は開発元を广东普惠智能教育技术有限公司と記載。[MDPDA][mdpda] [爱东东][idongdong] | 第三者による同名利用の証拠。掲載更新日はMDPDAが2022-04-14、爱东东が2020-11-04。古い掲載から現在の継続や権利者を断定しない。公式URLのHTTP・HTTPS本文を取得できず、現行提供元・機能・活動状況は未確認。[MDPDA][mdpda] [爱东东][idongdong] [公式と記載されたURL][pelwaycn] |
| Pellway / [pellway.com][pellway] | 近い英字のdomain。通常取得は本文取得失敗。curlで得たHTMLは`/lander`への遷移scriptで、遷移先の本文も取得できなかった。[対象URL][pellway] | GoDaddyの空ページという状態を今回は再現できず、現在の用途・提供元は未確認。空いている、販売中、取得可能とは判断しない。[対象URL][pellway] |

### 命名検討時に見送った候補の既存利用（参考）

Pelwayに決まる前に検討し、既存利用を理由に見送った候補の記録。Pelwayとの衝突ではない。2026-11-08（JST）に英字の完全一致でWeb検索して確認した。カタカナ表記と近い綴りは網羅していない。

| 候補 | 確認した既存利用 | 関係する範囲・未確認点 |
|---|---|---|
| Goodstead | 「エージェントのチーム」を打ち出すAIの節約サービス [Goodstead AI](https://www.goodstead.us/)。AIが追加作業を提案する建物点検SaaS [goodstead.app](https://goodstead.app/)。同名の投資助言サービス [goodstead.co](https://www.goodstead.co/faq/getting-started)。 | AIエージェントと業務SaaSの分野で重なる。各社の規模と活動状況は未確認。 |
| Tidemere | 会員制クラブや家の運営向けに、スタッフの業務と依頼をまとめるSaaS [tidemere.com](https://tidemere.com/)。米国のクラス42の商標出願の記載が[第三者サイト](https://markinton.com/trademark/tidemere-99882903)にある。 | 業務SaaSで重なる。商標出願は第三者サイトの記載で、公的なデータベースでは確認していない。 |
| Daylark | ChatGPTの中で動くAIのホームスクール支援アプリ [usedaylark.com](https://usedaylark.com/)。iPhoneのToDoアプリ（[App Store](https://apps.apple.com/us/app/daylark-daily-to-do-plan/id6786701493)）。未リリースの育児記録アプリ [daylark.net](https://daylark.net/)。AI会話型の予定管理ソフトとしての米国商標出願の記載が[第三者サイト](https://trademarks.justia.com/998/43/daylark-99843013.html)にある。 | AIアプリと生産性アプリで重なる。商標出願は第三者サイトの記載で、公的なデータベースでは確認していない。 |
| Goodstriva（参考） | 完全一致は見つからなかった。近い綴りの [GoodStrata](https://goodstrata.com.au/) は、AIエージェントが作業し、人が判断を受け取る受信箱を持つ、集合住宅管理のプラットフォーム。 | GoodStrataは機能比較の表にも入れている。混同の度合いは未確認。 |

**商標の登録・出願・類否、ドメインの取得可否・現在の所有者・価格について判断していない。** 法的な利用可否や名称変更は、この調査の結論に含めない。

## Pelwayへの示唆

以下は提案であり、採用済み仕様・料金・対象顧客ではない。

1. 共有チャットと専門AIだけで差別化しない。Context・CircleChat・Mantle・Cosmo・Bullpenには、名前を持つAIをチームの場へ呼ぶ説明がある。Pelwayでは依頼、担当決定、実行、承認、成果物の引き継ぎを一つの業務で試し、確認の手戻りが減るかを測る案がよい。[Context][context] [CircleChat][circle] [Mantle][mantleagents] [Cosmo][cosmo] [Bullpen][bullpen]
2. 承認の有無より、対象と責任者を具体化する。Contextの依頼者権限、CircleChatの危険操作の停止、GoodStrataの根拠付きinboxは比較基準になる。Pelwayでは「誰の資格で、どの宛先へ、何を変えるか、誰が承認できるか」をtaskに結び付ける設計を提案する。内容変更時の再承認、再試行の二重送信防止も試す。競合のこれらの詳細は未確認であり、欠けるとは断定しない。[Context][context] [CircleChat][circle] [GoodStrata][goodstrata]
3. AI別実行環境は費用と引き継ぎを含めて検証する。Grok Botの利用者単位共有、Rerunのworkspace単位共有、Contextのtask別sandboxは異なる構成。Pelwayでは資格情報・fileの分離、共同成果物へのアクセス、停止・再開費用を試作で比べる。VMだけで安全性や採算が証明されたとは扱わない。[Grok Bot][grokadmin] [Rerun][rerun] [Context][context]
4. 独立チャットへ移る理由を一つの業務で検証する。Slackは既存channelで目的別agentを使え、Teamsは既存の組織管理・配布経路を持つ。Pelwayでは、調査AIと資料AIの提案書作成を二人の人間がレビューし、外部送信だけ承認する業務などで、履歴と判断待ちをまとめる価値を試す。[Slack][slack] [Teams][teams]
5. 人の席数と実行原価を分けた価格説明を試す。CircleChat・Rerun・Bullpenはworkspace単位、Context・Mantleは人単位、Reasonは席とcredits、GroupChatは持ち込みagent費用を別にする。Pelwayも人の参加費、AI実行枠、VM、外部tool費、上限時の停止を分けて見積もる案がある。競合の表示価格だけから料金下限は決めない。[CircleChat][circle] [Rerun][rerun] [Context][context] [Mantle][mantleprice] [Reason][reasonprice] [GroupChat][groupchat] [Bullpen][bullpen]

確認できた差は、Teamsのagent間メッセージ配信や実行環境の単位など、特定の仕様・経路に限られる。日本語業務への適合、承認疲れ、AIの割り込み頻度、複数人の権限衝突を各社がどこまで解決するかは未確認。「他社にはできない」ではなくPelwayの評価項目として扱う。

## 未確認事項

- 製品同定：`reasone machine`・`reasoning machine`とReason Machinesの同一性。`Muse`・`open bot`が本書の提供元を指すかという名称上の一意性。Elsewhereの別サービスを意図した可能性。
- 同席仕様：Slack・Teamsの複数AIの詳細制限、Grok Bot標準groupへの複数人参加、OpenBot独自workspace・Reason・GroupChatの同席、Google Chat・Muse・Ottiの複数専門AI。
- 実行境界：表で未確認としたVM仕様、永続性、資格情報の共有、network制御。公式説明に対する実機検証・攻撃耐性の検証は未実施。
- 承認：表で未確認とした承認、複数承認者の競合、権限剥奪、内容変更時の再承認、二重実行防止、AI間で引き継いだ承認の範囲、監査log保全期間。
- 連携：表で未確認としたMCP対応。MCP提供と任意MCP接続の違い、connector別read/write権限・業務アカウント条件。
- 価格：Bullpenの年額、Cosmo、OpenBot Annual、Elsewhere、GroupChat managed agent単価、Ottiの課金期間・含有量、Contextの実行上限、Google Enterprise。日本向け税込総額・大量実行時の実費は未確認。
- 提供条件：運営法人名が未確認の製品（Mantle Chat、GroupChat、Elsewhereなど）、日本語、地域、保存国、保持・削除、一般提供とbetaの範囲、SLA、導入実績。個別に裏付けた項目以外は未確認。
- 名称：PELWAY教育製品の公式現行情報、Pellwayの用途。商標・domain利用可否は未調査。

## 出典

以下は2026-11-08（JST）にWebFetchまたはWebSearchで再確認した。Rakazoの出典は2026-11-13（JST）に確認した。製品自身による説明を根拠とし、安全性や成果の第三者保証とは扱わない。他社比較ページに載る他社価格は使っていない。数値を載せた価格は該当製品自身の表示に限る。

- [Slack：Agentforce利用ガイド](https://slack.com/help/articles/36218786859667-Use-Agentforce-in-Slack)
- [Salesforce：Agentforce Pricing](https://www.salesforce.com/agentforce/pricing/)
- [Salesforce：Slack Pricing](https://www.salesforce.com/slack/pricing/)
- [Microsoft Learn：Teams公開手順・制約](https://learn.microsoft.com/en-us/microsoft-copilot-studio/publication-add-bot-to-microsoft-teams)
- [Microsoft Learn：channel・groupのagent会話仕様](https://learn.microsoft.com/en-us/microsoftteams/platform/bots/how-to/conversations/channel-and-group-conversations)
- [Microsoft：Copilot Studio製品説明・料金FAQ](https://www.microsoft.com/en-us/microsoft-copilot/microsoft-copilot-studio#Pricing)
- [Microsoft Learn：MCP接続・データポリシー](https://learn.microsoft.com/en-us/microsoft-copilot-studio/mcp-add-existing-server-to-agent)
- [Microsoft Learn：M365 Copilot宣言型agentのMCP/APIプラグイン確認prompt仕様](https://learn.microsoft.com/en-us/microsoft-365/copilot/extensibility/plugin-confirmation-prompts)
- [Microsoft：Copilot enterprise pricing](https://www.microsoft.com/en-us/microsoft-365-copilot/pricing/enterprise)
- [Google：Ask Gemini in Chat公式発表](https://workspaceupdates.googleblog.com/2026/08/ask-gemini-in-chat.html)
- [Google Workspace料金。閲覧地域で通貨が変わる](https://workspace.google.com/pricing/?hl=en&country=US)
- [Google Workspace：FlexibleとAnnualの支払いプラン](https://knowledge.workspace.google.com/admin/billing/compare-flexible-and-annual-fixed-term-payment-plans)
- [Cursor：Grok Bot概要](https://cursor.com/docs/grok-bot)
- [Cursor：Work with Grok Bot](https://cursor.com/docs/grok-bot/work)
- [x.ai：Grok Bot for teams and enterprises](https://docs.x.ai/grok-bot/teams-and-enterprises)
- [Cursor：公式料金](https://cursor.com/pricing)
- [Meta：Muse公式発表](https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent/)
- [Meta：Muse利用ガイド](https://www.meta.com/help/superintelligence/1331373868832401/)
- [Meta：Muse privacy, safety and security](https://www.meta.com/help/superintelligence/1047255454427887/)
- [Meta：Muse subscriptions](https://www.meta.com/help/subscriptions/1021145227643680/)
- [CopilotKit：OpenBot for Business](https://open.bot/business)
- [CopilotKit：OpenBot公式README](https://github.com/CopilotKit/OpenBot)
- [Reason Machines公式サイト](https://reasonmachines.com/)
- [Reason Machines：How to use Reason](https://docs.reasonmachines.com/docs/guide)
- [Reason Machines：Plans, prices and credits](https://docs.reasonmachines.com/docs/plans)
- [Reason Machines：AI employees。自社機能説明のみ参照](https://reasonmachines.com/ai-employees)
- [Otti ai：開発者のApp Store説明・履歴・米国IAP](https://apps.apple.com/us/app/otti-ai/id6786263675)
- [CircleChat公式・料金](https://circlechat.co/)
- [Context Workspace公式・料金](https://www.context.ai/product/workspace)
- [Mantle Chat公式](https://mantle.chat/)
- [Mantle Docs：Agents](https://docs.mantle.chat/features/agents)
- [Mantle Docs：Web and desktop apps](https://docs.mantle.chat/features/desktop-app)
- [Mantle Docs：Integration access and safety](https://docs.mantle.chat/integrations/security-and-delivery)
- [Mantle：October 2026 update](https://mantle.chat/blog/monthly-update-october-2026)
- [Mantle Chat Pricing](https://mantle.chat/pricing)
- [Perséides：Cosmo](https://www.perseides.ai/cosmo)
- [GroupChat公式・料金](https://www.groupchat.ai/)
- [Bullpen公式・料金FAQ](https://bullpen.chat/)
- [GoodStrata公式・料金方針](https://goodstrata.com.au/)
- [Elsewhere指定URLの現行製品](https://elsewhere.so/)
- [Rerun公式・現行料金FAQ](https://rerun.build/)
- [Slack：Slack MCP Server](https://docs.slack.dev/ai/slack-mcp-server)
- [Slack：Slackbot MCP Client](https://docs.slack.dev/ai/slackbot-mcp-client)
- [CopilotKit：OpenBot公式料金](https://open.bot/pricing)
- [CopilotKit：OpenBotのグループ会話の実装](https://github.com/CopilotKit/OpenBot/blob/main/server/src/channels/group.ts)
- [CircleChat：公開リポジトリ](https://github.com/tashfeenahmed/circlechat)
- [Context：Terms](https://www.context.ai/terms)
- [Context：Connectors](https://www.context.ai/product/connectors)
- [Perséides：Agents are members in Cosmo](https://www.perseides.ai/blog/agents-are-members-in-cosmo)
- [Bullpen：Privacy](https://bullpen.chat/privacy)
- [GoodStrata：公開リポジトリ](https://github.com/goodstrata/goodstrata)
- [Rerun：Terms](https://rerun.build/terms)
- [Rerun：公開リポジトリ](https://github.com/rerun-build/rerun)
- [Rerun：Human in the loop](https://rerun.build/features/human-in-the-loop)
- [Rakazo公式サイト](https://rakazo.com/)
- [Rakazo：Pricing](https://rakazo.com/pricing/)
- [Rakazo：elie222/rakazo README](https://github.com/elie222/rakazo)
- [Rakazo：ドキュメント（Approvals、Spaces、Audit logほか）](https://rakazo.com/docs/)
- [第三者：MDPDAの普惠智能掲載](http://www.mdpda.com/app/apk8030171.html)
- [第三者：爱东东の普惠智能掲載](https://www.idongdong.com/app/218269.html)
- [掲載上の公式：pelway.cn。HTTP/HTTPS本文取得失敗](http://www.pelway.cn/)
- [Pellway。本文未取得、取得HTMLは/landerへ遷移](https://pellway.com/)

[slack]: https://slack.com/help/articles/36218786859667-Use-Agentforce-in-Slack
[afprice]: https://www.salesforce.com/agentforce/pricing/
[slackprice]: https://www.salesforce.com/slack/pricing/
[teams]: https://learn.microsoft.com/en-us/microsoft-copilot-studio/publication-add-bot-to-microsoft-teams
[teamsbots]: https://learn.microsoft.com/en-us/microsoftteams/platform/bots/how-to/conversations/channel-and-group-conversations
[studio]: https://www.microsoft.com/en-us/microsoft-copilot/microsoft-copilot-studio#Pricing
[msmcp]: https://learn.microsoft.com/en-us/microsoft-copilot-studio/mcp-add-existing-server-to-agent
[msapprove]: https://learn.microsoft.com/en-us/microsoft-365/copilot/extensibility/plugin-confirmation-prompts
[msprice]: https://www.microsoft.com/en-us/microsoft-365-copilot/pricing/enterprise
[google]: https://workspaceupdates.googleblog.com/2026/08/ask-gemini-in-chat.html
[googleprice]: https://workspace.google.com/pricing/?hl=en&country=US
[grok]: https://cursor.com/docs/grok-bot
[grokwork]: https://cursor.com/docs/grok-bot/work
[grokadmin]: https://docs.x.ai/grok-bot/teams-and-enterprises
[grokprice]: https://cursor.com/pricing
[muse]: https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent/
[museguide]: https://www.meta.com/help/superintelligence/1331373868832401/
[musesafety]: https://www.meta.com/help/superintelligence/1047255454427887/
[museprice]: https://www.meta.com/help/subscriptions/1021145227643680/
[openbot]: https://open.bot/business
[openrepo]: https://github.com/CopilotKit/OpenBot
[reason]: https://reasonmachines.com/
[reasonguide]: https://docs.reasonmachines.com/docs/guide
[reasonprice]: https://docs.reasonmachines.com/docs/plans
[reasonjobs]: https://reasonmachines.com/ai-employees
[otti]: https://apps.apple.com/us/app/otti-ai/id6786263675
[circle]: https://circlechat.co/
[context]: https://www.context.ai/product/workspace
[mantle]: https://mantle.chat/
[mantleagents]: https://docs.mantle.chat/features/agents
[mantledesktop]: https://docs.mantle.chat/features/desktop-app
[mantlesafety]: https://docs.mantle.chat/integrations/security-and-delivery
[mantleupdate]: https://mantle.chat/blog/monthly-update-october-2026
[mantleprice]: https://mantle.chat/pricing
[cosmo]: https://www.perseides.ai/cosmo
[groupchat]: https://www.groupchat.ai/
[bullpen]: https://bullpen.chat/
[goodstrata]: https://goodstrata.com.au/
[elsewhere]: https://elsewhere.so/
[rerun]: https://rerun.build/
[mdpda]: http://www.mdpda.com/app/apk8030171.html
[idongdong]: https://www.idongdong.com/app/218269.html
[pelwaycn]: http://www.pelway.cn/
[pellway]: https://pellway.com/
[slackmcps]: https://docs.slack.dev/ai/slack-mcp-server
[slackmcpc]: https://docs.slack.dev/ai/slackbot-mcp-client
[googleplans]: https://knowledge.workspace.google.com/admin/billing/compare-flexible-and-annual-fixed-term-payment-plans
[opengroup]: https://github.com/CopilotKit/OpenBot/blob/main/server/src/channels/group.ts
[openprice]: https://open.bot/pricing
[circlerepo]: https://github.com/tashfeenahmed/circlechat
[contextterms]: https://www.context.ai/terms
[contextconn]: https://www.context.ai/product/connectors
[cosmoblog]: https://www.perseides.ai/blog/agents-are-members-in-cosmo
[bullpenprivacy]: https://bullpen.chat/privacy
[goodstratarepo]: https://github.com/goodstrata/goodstrata
[rerunterms]: https://rerun.build/terms
[rerunrepo]: https://github.com/rerun-build/rerun
[rerunhitl]: https://rerun.build/features/human-in-the-loop
[rakazo]: https://rakazo.com/
[rakazorepo]: https://github.com/elie222/rakazo
[rakazopricing]: https://rakazo.com/pricing/
[rakazobots]: https://rakazo.com/docs/bots-and-chats/
[rakazogroups]: https://rakazo.com/docs/groups/
[rakazospaces]: https://rakazo.com/docs/spaces/
[rakazofleet]: https://rakazo.com/docs/fleet-management/
[rakazocomputer]: https://rakazo.com/docs/computer/
[rakazoproviders]: https://rakazo.com/docs/self-host-sandbox-providers/
[rakazoapprovals]: https://rakazo.com/docs/approvals/
[rakazointegrations]: https://rakazo.com/docs/integrations/
[rakazoaudit]: https://rakazo.com/docs/self-host-audit-log/
