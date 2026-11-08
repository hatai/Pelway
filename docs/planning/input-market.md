# Pelway・24か月マイルストーンへの市場担当入力

作成・価格再確認日：2026-11-08（JST、日本時間）。段階配分、検証方法、移行条件、価格設計はすべて**提案**であり、決定ではない。
初期顧客は「顧客案件を進める日本の小規模ITチーム」という仮説。社内wiki・外向けQ&A自動生成は対象外。既存調査は `/workspace/pelway-docs/research/competitors.md` を参照したが、価格は今回公式ページで再確認した。

## 1. 競合の再確認ゲート（提案）

### 共通の確認票（提案）

各段階で、対象製品の公式機能資料、変更履歴、公式料金に日付とURLを付け、次の項目を「記載確認／実機確認／未確認」で記録する。未確認を「非対応」に置き換えない。
- 複数人＋複数AIが同じチャンネルに参加するか。モデル切替、単一AIとのグループ会話、裏側のagent連携は分ける。
- AIごとの実行環境か、利用者・workspace・タスク単位か。永続性、ファイル・資格情報・ネットワークの分離、外部ツールとMCPの権限も確認する。
- 誰の資格で実行し、誰が承認できるか。承認後の宛先・本文・添付・対象ファイル・権限変更で旧承認が失効し、再承認になるか。
- 外部操作の「結果不明」を成功・失敗と区別するか。送信後に応答が途切れた場合、実行先照合・人の判断を経て、二重実行せず再開できるか。単なる検索不能やエラーメッセージとは分ける。
- 引き継ぎ時に依頼、成果物の版、未完了理由、担当者、権限、承認対象、外部操作の状態を保つか。単にAI同士が話すこととは分ける。
- 一般提供・beta、対象プラン、日本語・日本提供条件、席数・AI数・実行量・VM・ツールの追加料金、上限到達時の動作を確認する。

| 段階・期間案 | 再確認する競合（提案） | 特に確認すること（共通票全項目に追加・提案） | 見直し条件と次の判断（提案） |
|---|---|---|---|
| 顧客検証・M1〜3 | Context、CircleChat、Slack agents、Teams Copilot agents、Google Chat、Otti ai | 各公式URLは下表・末尾。既存チャット＋AIで案件業務が完結する範囲、同席と権限、基本料金を整理。顧客には現行手順と比較してもらう。 | 引き継ぎ・結果不明時の復旧が顧客の困り事でない、または既存製品で足りるなら、対象業務・独立チャットへの移行理由を再設定する。 |
| MVP・M4〜8 | 上記の重点6製品＋Grok Bot、OpenBot、Reason、Rerun、Mantle | 実行環境の分離、操作単位の承認、変更時再承認、引き継ぎ後の継続、結果不明の照合・再開。公開デモと公式仕様を分けて記録。 | 同じ対象業務で、状態と承認を保つ引き継ぎ、または結果不明からの安全な再開を競合が公式に一般提供したら、差別化の表現を止め、比較シナリオと対象顧客を見直す。 |
| パイロット・M9〜12 | Context、CircleChat、Slack、Teams、Google Chatを必須。導入先の併用製品＋Otti、Mantle、Cosmo、Bullpen、GroupChat | 担当変更、権限剥奪、承認後変更、通信断を同じ条件で比較。顧客の再説明時間、確認負担、復旧時の重複操作、既存チャットとの二重管理を記録。価格も更新。 | 標準機能で同等の復旧ができ、顧客が乗り換え・併用の手間に見合う差を認めないなら、有償化拡大を保留し、業務範囲か提供形態を見直す。 |
| 有償化・M13〜18 | 上記の全直接競合、特にContext、CircleChat、Slack、Teams、Google Chat。Grok Bot、Reason、Rerunの実行原価に近い課金も再確認 | 見積提示前に料金、契約期間、最低席数、ゲスト課金、利用枠、超過、VM・連携費を公式で確認。顧客自身の既存契約を含む総額と、レビュー後の実益を比較。 | 同じ業務量の競合総額が顧客の支払意思額を下回り、Pelwayに実益の差がなければ、値下げ前に対象・課金単位・原価構造を見直す。粗利基準は技術担当と濱田さんが決める。 |
| 拡大・M19〜24 | 既存調査の全製品＋顧客の失注・解約理由に出た製品。GoodStrataは承認設計の参考、Elsewhereは用途の再同定 | 全共通票を更新。案件・顧客間の分離、外部ゲスト、日本語、組織管理、復旧・監査条件、販売チャネル、契約と価格の変化を確認。 | 直接競合が差別化仮説を満たす、または別業種の方が継続利用・支払理由が明確なら、機能追加を先行せず、対象市場と訴求を再決定する。 |

提案：月初に重点製品を確認し、各段階へ移る前と価格提示前に更新する。重大な公式発表・価格改定があれば臨時再確認する。市場担当は変更点、根拠URL、未確認、影響、見直し案を企画担当へ渡す。競合追随だけを理由に開発範囲を増やさない。
現時点でも[CircleChat](https://circlechat.co/)と[Rerun](https://rerun.build/)はAI間の引き継ぎを公式に説明し、[Context](https://www.context.ai/product/workspace)は依頼者権限・書き込み承認・タスクの記録を説明する。「引き継ぎがある」自体は独自性としない。変更時の承認失効、結果不明の照合・再開、引き継ぎ後の状態整合性は今回の資料では未確認。

## 2. 顧客検証の材料

統計は初期顧客の需要・支払意思を証明しない。企業数、事業所数、回答企業数、個人モニターの勤務先回答を区別する。

| 材料・調査年 | 確認した数値と出典 | 初期仮説への使い方・限界 |
|---|---|---|
| 小規模IT・ソフトウェア企業の母数。2021年経済センサス再編加工 | 「小規模×ソフトウェア×顧客案件」という条件の企業数は見つからなかった。[中小企業庁公表](https://www.chusho.meti.go.jp/koukai/chousa/chu_kigyocnt/2023/231213chukigyocnt.html)、[産業別規模別企業数の表入口](https://www.chusho.meti.go.jp/pamflet/hakusyo/2024/chusho/f1.html)を確認したが、該当セルの数値抽出は未完了。 | 「中小企業」は少人数チームと同義でない。チーム規模・受託案件の有無を定義してから産業分類・規模別に再集計する提案。市場規模の数値にはまだ使わない。 |
| 情報サービス業。2021年情報通信業基本調査、2020年度実績 | 情報サービス業3,735社。ただし調査票の有効回答企業数。対象は原則資本金・出資金3,000万円以上で、小規模会社全体の母数ではない。[総務省・結果と対象基準](https://www.soumu.go.jp/menu_news/s-news/01tsushin02_02000156.html) | 「国内のIT企業総数」「IT中小企業総数」として使わない。小規模層が欠ける調査であることを確認する材料。 |
| ビジネスチャット。2024年10月、日経BPコンサルティング | モニター3,300人の勤務先導入回答は全体51.1%、従業員99人以下21.6%。導入済み層の主利用はTeams 67.8%、Google Chat 7.8%、Slack 7.5%。[NTT西日本掲載・調査説明](https://business.ntt-west.co.jp/bizclip/articles/bcl00014-049.html) | IT受託企業に限定しない個人回答。企業の悉皆導入率や初期顧客層のシェアとはしない。面談で既存チャットと移行制約を聞く材料。 |
| 総務省の通信利用動向。2025年調査 | ビジネスチャット単独の導入率は今回見つからなかった。企業編の対象は常用雇用者100人以上。[公式報告書](https://www.soumu.go.jp/johotsusintokei/statistics/pdf/HR202500_002.pdf)、[e-Stat表一覧](https://www.e-stat.go.jp/stat-search/files?cycle=0&layout=datalist&month=0&page=1&tclass1=000001243758&toukei=00200356&tstat=000001243701&year=20250) | クラウド・テレワーク・IoT/AIの率をチャット率に代用しない。少人数企業の実態へそのまま外挿しない。 |
| 生成AI導入。IPA「DX動向2026」、調査実施2026年4月17日〜6月12日、報告の年度表記は2025年度 | 国内有効回答1,799社。生成AI「導入している」44.0%。[公式本文・図表3-5](https://www.ipa.go.jp/digital/chousa/dx-trend/rcu1hd0000017uk8-att/dx-trend-2026.pdf)、[公式データ集・調査概要](https://www.ipa.go.jp/digital/chousa/dx-trend/rcu1hd0000017uk8-att/dx-trend-data-collection-2026.pdf) | 生成AIの導入であり、専門AI同僚・ビジネスチャットへの需要率ではない。業種・規模を限定した初期顧客の率は未確認。 |
| 生成AIの組織利用。同じIPA調査 | 個人で業務利用63.3%、部署の業務プロセスへの組込み11.9%。導入・試験利用・利用検討企業が対象の複数回答。[公式本文・図表3-6](https://www.ipa.go.jp/digital/chousa/dx-trend/rcu1hd0000017uk8-att/dx-trend-2026.pdf)、[公式データ集の設問注記](https://www.ipa.go.jp/digital/chousa/dx-trend/rcu1hd0000017uk8-att/dx-trend-data-collection-2026.pdf) | 全企業を分母にせず、両者の差を潜在顧客率とも解釈しない。個人利用から共同案件へ移す際の障害を聞く提案。 |

顧客検証の提案：初期顧客を「小さい会社」だけで選ばず、外部顧客案件、複数担当、AI利用、承認者の有無で層分けする。直近案件の匿名化した手順から、担当交代時の再説明、送信結果の不明、承認後変更の頻度と損失、現在の対処、データ持出し制約、購入決裁者を聞く。
パイロットの提案：現在の方法とPelwayで同じ業務を比べ、確認・修正時間を含む完了時間、手戻り、復旧、継続利用、支払意思を記録する。統計から率を借りて目標顧客数や売上を逆算しない。必要標本数と合格基準は未決。

## 3. 有償化段階の価格比較の目安

下表は2026-11-08（JST）に公式で再確認した米ドル表示。年契約と月払い、席課金とworkspace課金を区別する。AI上限数を割った「AI一人当たり価格」は作らない。税、為替、既存契約、実行量が違うため機能同等の総額ではない。

| 製品・公式出典 | 課金単位と確認価格 | 追加費用・未確認 |
|---|---|---|
| [Context](https://www.context.ai/product/workspace) | Free $0、Plus $20/人/月、Enterprise個別相談 | 実行上限・モデル費内訳は未確認。 |
| [CircleChat](https://circlechat.co/) | workspace/月。Starter $29・3 agents、Team $79・10 agents、Scale $199・無制限agents | 自前モデルキー。トークン上乗せなしと説明。self-host無料でも運用・モデル費は総額に含める。 |
| [Slack](https://www.salesforce.com/slack/pricing/)＋[Agentforce](https://www.salesforce.com/agentforce/pricing/) | Slack通常月払いPro $8.75/人/月、Business+ $18/人/月。Agentforce User License $5/人/月＋Flex Credits $500/100,000 credits | Agentforce利用は別ライセンスが必要。適用SKU・総額は未確認。別の$125/人/月add-onを全顧客必須の最低額とはしない。 |
| [Teams・M365 Copilot](https://www.microsoft.com/en-us/microsoft-365-copilot/pricing/enterprise) | Copilot add-on $30/人/月・年払い | 対象M365とTeamsライセンスが別途必要。agentの含有範囲・従量課金は経路と契約に依存。AI一体ごとの価格は未確認。 |
| [Google Chat・Workspace](https://workspace.google.com/pricing/?hl=en&country=US) | ChatのGeminiを含む対象プランの最新数値は未確認 | 取得本文で対象プランと契約条件の価格対応を確定できず。安いStarter表示を比較対象のAI機能価格としない。 |
| [Otti ai・米国App Store](https://apps.apple.com/us/app/otti-ai/id6786263675) | 無料＋IAP。Pro $19.99、Unlimited $200、Business 25 $150など | 購入項目の期間・含有量・席数は未確認。「25」を25席とは解釈しない。 |
| [Grok Bot・Cursor料金](https://cursor.com/pricing) | Bot掲載はPremium $30/月、Super $100/月、Ultra $200/月、Teams $30〜/人/月 | 他製品と利用枠共有、追加利用あり。Bot単独・AI別価格ではない。 |
| [Mantle](https://mantle.chat/pricing) | 年払いCore $10/人/月、Plus $25/人/月、Max $99/人/月。Freeあり | 月換算額。年払い総額は各$120・$300・$1,188。追加10,000 credits $12、税別。 |
| [Reason](https://docs.reasonmachines.com/docs/plans) | Pro $50/人/月、Max $200/人/月 | 週次credits、モデル・cloud PCが消費。持ち込みモデル費別。Networkは別の無料サービス。 |
| [Rerun](https://rerun.build/) | $29/workspace/月、人・Coworkers無制限、machine込み | Rerun creditsがモデル・外部ツール要求にも必要。実行込みの無制限定額としない。 |
| [Muse・Meta](https://www.meta.com/help/subscriptions/1021145227643680/) | 上限付き無料、Power $20/月、Maximum $100/月 | 個人用。チーム共有席・AI別単価、日本での適用は未確認。 |
| [OpenBot Business](https://open.bot/business) | 数値料金は未確認、early accessの導入相談 | 運用・モデル費を含む見積りは未確認。 |
| [Cosmo](https://www.perseides.ai/cosmo) | 数値料金は未確認、private beta | 席・AI・実行量単価は未確認。 |
| [Bullpen](https://bullpen.chat/) | Free最大5人、個人用Ace $12/月から。team有料席単価は未確認 | helpersはplan同梱、枠共有。音声minutesは別カウント。 |
| [GroupChat](https://www.groupchat.ai/) | workspace $0、agents等無制限、持ち込みagent費は提供元へ | managed agentsのtoken単価は未確認。 |
| [GoodStrata](https://goodstrata.com.au/) | owners corporation向けhosted無料tier・self-host。汎用席・AI単価は未確認 | 将来managed service価格は未確認。異業種の設計参考で、直接の価格基準にはしない。 |
| [Elsewhere](https://elsewhere.so/) | 数値料金は未確認 | 指定URLはcapacity planning。AI協業チャットとの同一性・直接比較の妥当性は未確認。 |

価格検証の提案：有償化前に、顧客が実行した案件の件数・人とAIの参加・実行時間・承認回数から、競合とPelwayの月額を同条件で試算する。ゲスト、待機VM、復旧の再実行、導入・確認作業も含める。競合価格だけでPelwayの価格下限は決めない。

## 4. 濱田さんの判断が必要な点

- 提案：初期対象を受託開発会社、制作・運用チーム、社内ITのどこに置くか。会社規模でなくチーム規模・顧客案件・承認者をどう定義するか。
- 提案：「状態と権限を保つ引き継ぎ」「結果不明からの安全な再開」を優先検証するか。他社より優れることはまだ証明していない。
- 提案：既存チャットからの移行か、案件単位の併用か。顧客・外注ゲストを含める範囲、実データを持ち込める条件を決める。
- 提案：段階移行の証拠、顧客数・継続期間・支払意思・確認時間の合格基準、未達時の延長・縮小・中止を決める。上記期間案は開発確約ではない。
- 提案：課金単位、許容する月額上限、無料枠、必要粗利、競合追随に使える工数を決める。料金・予算の決定や契約は本資料では行わない。

## 5. 未確認事項と次の調査（提案）

- 小規模IT企業の正確な母数と受託案件チーム数。経済センサスの産業分類・企業規模別数値の抽出が必要。情報通信業基本調査の回答数を代用しない。
- 初期対象層だけのチャット・生成AI導入率、移行意思、価格許容度、復旧・引き継ぎの問題頻度。公開統計では見つからず、顧客検証が必要。
- 競合の変更時再承認、結果不明の実行先照合・再開、引き継ぎ後の承認状態・権限、AI別の隔離。公式で未確認の項目と実機検証未実施を分けて残す。
- Google対象プラン価格、Ottiの課金期間・席数、OpenBot・Cosmo・Bullpen team・Elsewhere・GroupChat managed単価、全社の日本向け税込総額と実行量込みの比較。
- `reasone machine`等とReason Machinesの同一性、同名Muse・OpenBotの意図、Elsewhereの対象同定は既存調査の留保を維持。

### 主要な機能確認先

各段階での再確認先（提案）。料金表のリンクに加え、[Slack Agentforce利用ガイド](https://slack.com/help/articles/36218786859667-Use-Agentforce-in-Slack)、[Teams agent会話仕様](https://learn.microsoft.com/en-us/microsoftteams/platform/bots/how-to/conversations/channel-and-group-conversations)、[Ask Gemini in Chat公式発表](https://workspaceupdates.googleblog.com/2026/08/ask-gemini-in-chat.html)を追う。既存調査の未確認欄も引き継ぎ、公式発表だけで安全性を保証しない。
