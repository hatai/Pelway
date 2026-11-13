# Grok Bot（Cursor / x.ai）

確認日：2026-11-11（JST、日本時間）

一覧と比較の前提は[競合調査](../competitors.md)を参照する。「確認」は公式資料・公開リポジトリに記載があるという意味で、実機検証済みではない。「未確認」は今回の資料で裏付けられなかった項目で、機能がないという意味ではない。

## 概要

| 項目 | 内容 |
|---|---|
| 提供元 | 製品資料はCursorの公式ドキュメント（cursor.com）とx.aiの公式ドキュメント（docs.x.ai）にある。Bot用の計算機はCursorのクラウドで動くと説明。Cursorとx.aiの契約上の関係は未確認。[概要][grok] [組織向け][grokadmin] |
| 公式URL | [Grok Bot概要][grok]、[Work with Grok Bot][grokwork]、[Grok Bot for teams and enterprises][grokadmin] |
| 製品の説明 | 名前、仕事、文脈を持ち続けるAIの同僚（Bot）を作り、メッセージで仕事を頼む。各Botはブラウザ、ファイル、terminalを持つ永続的なクラウドの計算機で動く。[概要][grok] |
| 対応環境 | デスクトップはmacOS、Windows、Linux。モバイルはiPhoneとAndroid。[概要][grok] |
| ライセンス | 公開リポジトリは未確認。 |
| 価格 | Cursorの料金表ではPremium $30/月、Super $100/月、Ultra $200/月、Teams $30+/人/月、Enterpriseは個別見積もりにGrok Botを表示。Free $0/月とPlus $8/月には表示なし。年額表示はない。税別で、第三者モデルの利用は別請求。[価格][grokprice] |
| 利用枠 | 週ごとにリセットされる利用上限。The X SubscriptionではCursorとGrokアプリと同じ週次の枠を使う。SuperGrok系の契約やGrok Businessの席を連携して使える。[プラン][grokplans] |

## 機能

| 観点 | 内容 | 出典 |
|---|---|---|
| 複数人＋複数AIの同一チャンネル参加 | group chatは自分のBotを2〜6体選んで作る。group chatの説明は「自分のBotを1つの会話に入れる」で、複数の人が同じgroup chatに入る仕様は未確認。共有Bot（Team Bot）はSlackのチャンネル、スレッド、group DMで、チャンネルの誰からでも仕事を受ける。 | [作業ガイド][grokwork] [group chats][grokgroup] [Team Bots][grokteam] |
| AIの役割・専門性の設定 | 最初のメッセージで仕事、参照元、承認の境界を伝え、覚えるよう頼む。記憶、skill（再利用できる作業手順）、routine（スケジュールやイベントで動く処理）を持つ。 | [作業ガイド][grokwork] |
| AI間の引き継ぎ | Botは別のBotに非同期でメッセージを送り、仕事と文脈を渡す。受け渡しは会話上に表示される。Bot間の受け渡しを止めるスイッチはなく、指示で制御する。 | [作業ガイド][grokwork] [group chats][grokgroup] |
| 実行能力 | 永続的なクラウドの計算機でブラウザ、コマンドライン、ファイルを使う。接続アプリがない作業は画面操作で行う。コーディング作業はCloud Agentへ委任できる。 | [作業ガイド][grokwork] [組織向け][grokadmin] |
| 承認・権限管理 | 送信、購入、削除、公開、本番環境の変更は承認の対象。独立した審査モデルAuto Reviewが危険な操作を「実行」「承認を求める」「拒否」に振り分ける。利用者は「Ask first」「Allow automatically」のルールを追加できる。Auto Reviewの強制とチーム共通ルールはEnterprise限定。BotはサインインしたメンバーとしてBot固有の身元を持たず、メンバー以上の権限を持たない。 | [セキュリティ][groksec] [組織向け][grokadmin] |
| 承認者 | Team Botは回答している相手に承認を求める。Slackのスレッドとgroup chatでは、依頼した人だけが承認できる。その人が応答しない場合の扱いは未確認。 | [Team Bots][grokteam] |
| 外部連携 | 接続アプリ（MCP）はチームのCursorのコネクタ方針を引き継ぐ。OAuthトークンはCursorのコネクタ側に残り、Botには渡らない。MCPの許可リストはEnterprise限定。 | [組織向け][grokadmin] |
| 認証情報 | Vault（保存したログイン）、連携したパスワード管理ツール、Secure Form（入力値をモデルに渡さず参照で入力するフォーム）の順に使う。 | [作業ガイド][grokwork] [セキュリティ][groksec] |
| 監査 | Action Recording（ツール呼び出し、承認の判断、shellコマンド、ブラウザ遷移などのメタデータ記録）とAudit logsはEnterprise限定。OpenTelemetryで書き出す。 | [組織向け][grokadmin] |

## UI/UX

| 要素 | 内容 | 出典 |
|---|---|---|
| 画面構成 | 左のサイドバーに新規作成の「+」、Connect apps、ピン留めしたBot、セクション、Hidden Bots。中央がチャット、右のパネルに計算機のプレビュー。Botの名前をクリックすると詳細が開き、routineはDetailsタブにある。 | [作業ガイド][grokwork] |
| Bot作成 | 「+」からCreate a Botを選び、名前、形、色を決めてCreateを押す。初回は「Grok Bot」という名前のBotが自動で作られ、チャットが開く。仕事の定義は最初のメッセージで伝える。「設定はワークフロー作成画面ではなくメッセージ」と説明。 | [作業ガイド][grokwork] [Get started][grokstart] [概要][grok] |
| 承認カード | 承認が必要な操作では、会話に操作内容と入力が表示され、Allow once、Always allow（ルールを保存）、Denyを選ぶ。Denyした操作は、次のメッセージまで再試行しない。ローカルコマンドのカードは実際のコマンドを表示する。iPhoneでも同じ操作。 | [セキュリティ][groksec] |
| 画面の引き継ぎ | SSO、passkey、CAPTCHA、端末承認、QRコード、カード認証ではBotが計算機を利用者に渡す。利用者はOpen computerを押して止まった手順だけを済ませ、Botに続行を伝える。 | [作業ガイド][grokwork] [Get started][grokstart] |
| 画面の分け方 | 同じ利用者のBotは1台の計算機を共有し、Botごとに画面を持つ。画面は安全上の境界ではないと明記。 | [作業ガイド][grokwork] |
| group chat | 「@Bot名」で1体に依頼し、「@everyone」で全員に伝える。Botが他のBotや利用者に個別に送ったメッセージは、group chatではなく各Botの会話に出る。音声会話はBot 1体のみ。 | [作業ガイド][grokwork] [group chats][grokgroup] |
| Team Botの共有 | 共有はドキュメントの共有と同じ操作で、公開範囲を本人のみ、招待した人、チーム、組織から選ぶ。公開するまでは下書き。各メンバーとの会話は所有者からも見えない。メンバーがBotに教えた内容は、所有者かManagerがReview learningsで承認してからチーム全体に反映される。 | [Team Bots][grokteam] |
| 停止 | 「Stop now」で作業を止める。完了済みの操作は元に戻らない。 | [作業ガイド][grokwork] |

## 技術

公開資料から確認した。ソースコードは公開されていない（公開リポジトリは未確認）。

| 層 | 技術 | 出典 |
|---|---|---|
| 実行環境 | 利用者ごとにFirecracker microVMを割り当て、他の利用者とハードウェアの水準で分離する。同じ利用者のBotはその1台を共有する。 | [組織向け][grokadmin] |
| Team Botの計算機 | Slackのチャンネル、スレッド、group DM、group chatではTeam Bot専用の計算機を1台使う。本人とのチャットでは本人の計算機を使うことが多い。 | [Team Bots][grokteam] [組織向け][grokadmin] |
| 永続化 | 計算機のファイルとブラウザのセッションは永続ディスクに残る。長く使わない計算機は休止する。所在地は米国。 | [セキュリティ][groksec] |
| ネットワーク制御 | Enterprise限定。全許可、許可リスト型などのモードを持つ。ブロックは「ベストエフォートで安全上の境界ではない」と説明。MCPサーバーへの適用は既定でオフ。 | [セキュリティ][groksec] [組織向け][grokadmin] |
| 推論の検査 | Inference Hooks（Enterprise、beta）は署名付きのモデル要求を顧客のセキュリティサーバーに送り、許可・拒否させる。 | [組織向け][grokadmin] |
| プロトコル | MCP（接続アプリ）。OpenTelemetryでの記録の書き出し。 | [組織向け][grokadmin] |

## Pelwayとの比較［提案］

この節は本書の整理と提案であり、Grok Botの資料が述べている内容ではない。Pelway側は[ビジョン](../vision.md)、[PRD](../prd.md)、[技術](../technical.md)の構想で、実装済みの機能ではない。

| 観点 | Grok Bot | Pelway（構想） | Pelwayでの扱い |
|---|---|---|---|
| 人とAIの同席 | 自分のBot同士のgroupと、SlackでのTeam Bot利用 | 複数人と複数AIが同じチャンネル | 独立したチャット内での複数人の同席を比較の観点にする |
| 承認者 | 依頼した本人だけが承認する | 依頼者と承認者を分ける | 依頼者と承認者が別人の流れを試作で確かめる |
| 承認の操作 | Allow once / Always allow / Deny と、外部のAuto Review | 承認内容を固定し、変われば再承認 | 一回限りの承認を既定にし、常時許可は管理者の範囲に限る案 |
| 実行環境 | 利用者ごとのmicroVM。Bot間で認証情報を共有 | AIごとの実行環境を検討 | 画面は境界でないという注意を参考に、分離単位を決める |
| 画面の引き継ぎ | 止まった手順だけを人が済ませて返す | 未決 | 「人が済ませる手順」を状態として持つ案 |
| 学習内容の共有 | メンバーの教えた内容を所有者が承認 | 引き継ぎ欄に会社のルールを書く案 | チーム共通の知識に承認を挟む方式を参考にする |

## 未確認事項

- group chatに複数の人が参加できるか。
- Team Botの承認で、依頼者が応答しない場合の扱い。
- Cursorとx.aiの提供上の関係。
- 日本語対応、日本の税込価格、SLA。

## 出典

- [Cursor：Grok Bot概要](https://cursor.com/docs/grok-bot)
- [Cursor：Work with Grok Bot](https://cursor.com/docs/grok-bot/work)
- [Cursor：Grok Bot Get started](https://cursor.com/docs/grok-bot/get-started)
- [Cursor：Grok Bot Security](https://cursor.com/docs/grok-bot/security)
- [Cursor：Grok Bot group chats](https://cursor.com/help/grok-bot/group-chats)
- [Cursor：Team Bots](https://cursor.com/help/grok-bot/team-bots)
- [Cursor：Grok Bot plans](https://cursor.com/help/grok-bot/plans)
- [Cursor：Pricing](https://cursor.com/pricing)
- [x.ai：Grok Bot for teams and enterprises](https://docs.x.ai/grok-bot/teams-and-enterprises)

[grok]: https://cursor.com/docs/grok-bot
[grokwork]: https://cursor.com/docs/grok-bot/work
[grokstart]: https://cursor.com/docs/grok-bot/get-started
[groksec]: https://cursor.com/docs/grok-bot/security
[grokgroup]: https://cursor.com/help/grok-bot/group-chats
[grokteam]: https://cursor.com/help/grok-bot/team-bots
[grokplans]: https://cursor.com/help/grok-bot/plans
[grokprice]: https://cursor.com/pricing
[grokadmin]: https://docs.x.ai/grok-bot/teams-and-enterprises
