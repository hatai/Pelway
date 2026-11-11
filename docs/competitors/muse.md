# Muse（Meta）

確認日：2026-11-11（JST、日本時間）

一覧と比較の前提は[競合調査](../competitors.md)を参照する。「確認」は公式資料・公開リポジトリに記載があるという意味で、実機検証済みではない。「未確認」は今回の資料で裏付けられなかった項目で、機能がないという意味ではない。

## 概要

| 項目 | 内容 |
|---|---|
| 提供元 | Meta。[公式発表][muse] |
| 公式URL | [muse.ai][home]、[公式発表][muse]、[Muse for Small Business][biz] |
| 製品の説明 | 質問に答えるだけでなく作業をする個人用AIエージェント。利用者ごとの専用クラウド計算機（Muse Secure VM）で動き、目標を追って提案もする。モデルはMuse Spark。[公式発表][muse] [muse.ai][home] |
| 利用経路 | Museのモバイルアプリ、デスクトップ（muse.ai）、WhatsApp。文字と音声。[muse.ai][home] [公式発表][muse] |
| 提供地域 | 公式発表は米国のiOS、Android、muse.aiでの提供を説明。Small Business向けページは米国とカナダの18歳以上と記載。料金ページは試験運用中で地域が限られると記載。[公式発表][muse] [Small Business][biz] [料金][museprice] |
| ライセンス | 公開リポジトリは未確認。 |
| 価格 | Free（上限付き、上限は定期的にリセット）、Power $20/月（週5億Muse tokens）、Maximum $100/月（週30億Muse tokens）。月額の自動更新。提供条件は地域とアカウントで変わり、加入前のonboardingで示される。[料金][museprice] |

## 機能

| 観点 | 内容 | 出典 |
|---|---|---|
| 複数人＋複数AIの同一チャンネル参加 | 個人用エージェントとして説明。複数人や複数AIが同じ会話に入る仕様は未確認。Small Business向けページもチーム機能に触れていない。 | [公式発表][muse] [Small Business][biz] |
| AIの役割・専門性の設定 | 名前、性格、設定はMuseに伝えて変える。アバターは利用者が作り名前を付ける。複数の専門職AIを置く仕様は未確認。 | [利用ガイド][museguide] [設計ブログ][design] |
| 会話の分け方 | 1本の長いメインチャットに加え、話題ごとのside chatを作れる。Museは両方の状況を把握して応答する。 | [利用ガイド][museguide] [設計ブログ][design] |
| 実行能力 | 専用VMのファイルシステム、terminal、ブラウザで作業する。フォーム入力、メール送信、旅行予約、購入。必要な道具がなければ自分でコードを書いて作る。PDF、表計算、文書、画像、podcastの読み取りと生成。 | [公式発表][muse] [設計ブログ][design] [利用ガイド][museguide] |
| 承認・権限管理 | メール送信や購入などの重要な操作の前に確認を求める。必要な許可は操作の種類、コネクタ、既定の許可設定で変わる。承認の検査の多くはAIモデルの外で行う。既定はWeb閲覧を許し、元に戻しにくい操作で止まる。慎重さは利用者が調整する。 | [安全性説明][musesafety] [設計ブログ][design] |
| 外部連携 | 利用者がコネクタと権限の範囲を選ぶ（メールは読むだけか送信もするか）。コネクタは処理に必要なデータだけを扱う。Small Business向けにAsana、Box、Canva、Dropbox、Figma、Notion、Shopify、Slack、Stripe、Zoom、Meta系サービスなど。任意のMCPサーバー接続は未確認。 | [公式発表][muse] [安全性説明][musesafety] [Small Business][biz] |
| 決済・認証情報 | 決済はLink by Stripeの一回限りのカード番号を使う。認証情報はSecure Credentials Storeに置き、モデルはパスワードを見ない。 | [公式発表][muse] [安全性説明][musesafety] |
| 監査 | 実行済みと予定の操作の記録（audit trail）を見られる。 | [公式発表][muse] [muse.ai][home] |
| 記憶 | 会話をまたいで記憶する。記憶はMEMORY.mdなどのファイルで確認・編集でき、「忘れる」skillで削除を頼める。 | [安全性説明][musesafety] [設計ブログ][design] |

## UI/UX

| 要素 | 内容 | 出典 |
|---|---|---|
| 画面構成 | 起動するとチャットタブが開く。左上のメニューからside chatを作る。成果物はライブラリのタブに保存され、スレッドでも共有される。Goalsタブで目標とMuseの計画を追う。 | [利用ガイド][museguide] [設計ブログ][design] |
| 作業状態の表示 | アバターの下に、裏で進めている作業の一部を表示する。押すと活動の記録と承認済みの権限が開く。 | [設計ブログ][design] |
| 承認の表示 | 承認は会話ではなく構造化されたカード（受け入れ／拒否）で行う。会話の外の決定的なUIにした理由として、何でも承認してしまう慣れを避けることを挙げる。承認の画面はチャットではなくクライアントアプリのダイアログに出て、実行する操作を具体的に示す。 | [設計ブログ][design] [安全性ブログ][security] |
| 承認の範囲 | 許可の種類は一回限り、セッション、タスク、期限付き、恒久から選ばれる。 | [安全性ブログ][security] |
| ブラウザの閲覧と引き継ぎ | 共有ブラウザでエージェントの作業を見て、いつでも介入できる。ブラウザ操作は利用者に見えると説明。 | [muse.ai][home] [安全性ブログ][security] |
| 先回りの連絡 | Museは依頼なしにメッセージを送る。送る基準は高く、利用者はこの頻度を上げ下げ・停止できる。順番外に届いたメッセージは吹き出しで区切る。 | [設計ブログ][design] |
| 作業中の入力 | Museの回答を待たずに次の質問を入力できる。 | [利用ガイド][museguide] |

## 技術

公式の技術ブログから確認した。ソースコードは公開されていない（公開リポジトリは未確認）。

| 層 | 技術 | 出典 |
|---|---|---|
| 実行環境 | 利用者ごとの専用クラウドLinux VM。VMの仮想化技術（microVMなど）は未記載。エージェント本体、作業領域、ツールはVM内の`systemd-nspawn`コンテナで動き、コンテナ内のrootはホストの非特権ユーザーに対応する。 | [安全性ブログ][security] |
| 外部通信の制御 | Sentinelはコンテナの外（ホスト側）で動く別のエージェントで、コネクタ操作と外部通信を許可・拒否・確認に振り分ける唯一の判断者。L4とL7で通信を検査し、SSRF対策を行う。 | [安全性ブログ][security] [公式発表][muse] |
| 承認の流れ | Sentinelが確認を選ぶと承認待ちを作り、実行を止める。依頼はクライアントアプリに直接届き、利用者の回答はSentinelに直接戻る。 | [安全性ブログ][security] |
| カーネルでの追跡 | eBPFのcgroupプログラムで通信の横取りとプロセスの特定を行う。LSMフックのeBPFで、利用者データを読んだプロセスの外部通信を追跡する（tainted egress）。 | [安全性ブログ][security] |
| 認証情報 | `hatch-authd`がOAuthトークンなどを保管し、エージェントには代理トークンを渡す。ネットワークの境界で本物に差し替える。 | [安全性ブログ][security] |
| コネクタ | CLIとskillで作る。処理は範囲を限った認証情報を持つ`privsep`ワーカーで動く。ブラウザは別のCDPブローカー経由で操作する。MCPの記載はない。 | [安全性ブログ][security] |
| データ | 永続的な状態はPostgresに置き、実行環境と認証情報の保管とは分ける。VMのデータはMetaの広告システムと共有しない。 | [安全性ブログ][security] [安全性説明][musesafety] |
| 予定 | 利用者だけが持つ鍵で暗号化するMuse Confidential VMを予定と記載。 | [公式発表][muse] [安全性説明][musesafety] |

## Pelwayとの比較［提案］

この節は本書の整理と提案であり、Museの資料が述べている内容ではない。Pelway側は[ビジョン](../vision.md)、[PRD](../prd.md)、[技術](../technical.md)の構想で、実装済みの機能ではない。

| 観点 | Muse | Pelway（構想） | Pelwayでの扱い |
|---|---|---|---|
| 対象 | 個人と小規模事業者の1人 | 複数人と複数AIのチーム | 直接の競合ではなく、個人の成果を共有する負担の比較対象 |
| 承認の強制 | ホスト側のSentinelがモデルの外で止める | 承認を実行内容に結び付け、失敗時は止める | 承認の判断をモデルの外に置く構成を参考にする |
| 承認の表示 | 会話の外のカード・ダイアログ | 会話の中で依頼・判断・承認が済む方針 | 会話内に置く場合も、構造化したカードで内容を固定する |
| 許可の期間 | 一回限りから恒久まで5種類 | 1つの承認を複数の操作に使い回さない | 一回限りを既定にし、期間付きの許可を検討材料にする |
| 認証情報 | 代理トークンを境界で差し替える | 実行環境に鍵を渡さない | 同じ方向。ブラウザのログイン状態の扱いを比べる |
| 作業状態 | アバターの下に作業の一部、押すと記録 | 判断待ちを「今日」画面にまとめる | 作業中の表示から記録へ進む導線を参考にする |

## 未確認事項

- 複数人での共有、チーム向けの管理機能。
- 任意のMCPサーバー接続。
- VMの仮想化技術とホストの構成。
- 日本での提供、日本語対応、SLA。

## 出典

- [Meta：Introducing Muse](https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent/)
- [Meta：Muse利用ガイド](https://www.meta.com/help/superintelligence/1331373868832401/)
- [Meta：Muse privacy, safety and security](https://www.meta.com/help/superintelligence/1047255454427887/)
- [Meta：Muse subscriptions](https://www.meta.com/help/subscriptions/1021145227643680/)
- [Meta：muse.ai](https://muse.ai/)
- [Meta：Muse for Small Business](https://muse.ai/business)
- [Meta：How we designed Muse](https://muse.ai/blog/how-we-designed-muse)
- [Meta：Security and safety for AI agents: our approach with Muse](https://muse.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse)

[muse]: https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent/
[museguide]: https://www.meta.com/help/superintelligence/1331373868832401/
[musesafety]: https://www.meta.com/help/superintelligence/1047255454427887/
[museprice]: https://www.meta.com/help/subscriptions/1021145227643680/
[home]: https://muse.ai/
[biz]: https://muse.ai/business
[design]: https://muse.ai/blog/how-we-designed-muse
[security]: https://muse.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse
