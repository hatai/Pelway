# Google Chat / Gemini（Google）

確認日：2026-11-11（JST、日本時間）

一覧と比較の前提は[競合調査](../competitors.md)を参照する。「確認」は公式資料・公開リポジトリに記載があるという意味で、実機検証済みではない。「未確認」は今回の資料で裏付けられなかった項目で、機能がないという意味ではない。

本書は次の経路を分けて扱う。Google Chatに組み込まれた個人向けのAsk Gemini、Gemini in Chat（会話の要約など）、Google Workspace Studioのフロー（flows）、開発者がGoogle Chatアプリとして作るAIエージェント。ある経路の仕様を、他の経路に当てはめない。

## 概要

| 項目 | 内容 |
|---|---|
| 提供元 | Google。 |
| 公式URL | [Ask Gemini in Chatの発表][blog]、[Ask Geminiの使い方][askhelp]、[Gemini in Chat][collab]、[Workspace Studioの承認][studioappr] |
| 製品の説明 | Ask GeminiはGoogle Chatの中の個人用のコマンドライン。Workspace Intelligenceを使い、Gmail・Drive・Calendarを横断した検索、画像や更新文の作成、会話の追いつき、会議とタスクの管理、トピック別のセッションでの作業をする。[発表][blog] |
| 提供段階 | Ask Geminiは英語のみ。毎日のまとめ（daily brief）はGemini AlphaとGoogle Workspace Experimentsで提供。[Ask Gemini][askhelp] |
| ライセンス | プロプライエタリ（クローズドな商用サービス）。A2Aプロトコルの仕様リポジトリはApache-2.0。[a2aproject/A2A][a2arepo] |
| 価格 | Ask Geminiの対象はBusiness Standard／Plus、Enterprise Standard／Plus、教育機関向けのGoogle AI Pro for Education。Business Standardは月額$16.80 USD per user（Flexible Plan、契約期間なし）、$14 USD per user（Annual/Fixed-Term Plan、1年以上の契約）。Business Plusは$26.40（Flexible）、$22（Annual）。Annualは契約期間中にユーザーを減らせず、途中解約でも残額を払う。Enterpriseの価格は営業・販売店経由。料金ページは閲覧地域によって通貨が変わる。Ask Geminiの利用上限は別に設定される。[発表][blog] [支払いプラン比較][plans] [料金][price] |

## 機能

| 観点 | 内容 | 出典 |
|---|---|---|
| 複数人＋複数AIの同一チャンネル参加 | Ask Geminiは個人のセッションで使う。Google Chatアプリは、スペースに追加されると、DMやスペースでメンションされたメッセージを受け取る。複数の専門AIがスペースのメンバーとして協働する仕様は未確認。 | [発表][blog] [Chatアプリの操作イベント][interact] |
| AIの役割・専門性の設定 | Workspace Studioで作ったスキルを、Ask Geminiから`@スキル名`で呼べる（例：スペースの会話から状況報告を作る「Project Summary」）。GemsはChatのAsk Geminiからは使えず、他のWorkspaceアプリのGeminiサイドパネルで使える。 | [Ask Gemini][askhelp] [発表][blog] |
| 実行能力 | Docs・Slides・Sheetsの作成、Driveのファイル管理、タスクの作成・変更・削除、主カレンダーへの予定作成・変更・削除、画像生成、Deep Research。VMやコード実行は未確認。 | [Ask Gemini][askhelp] |
| 承認・権限管理 | 管理者がGemini in Workspace in ChatとWorkspace Intelligenceを全アプリで有効にし、利用者がWorkspace Smart Featuresに登録している必要がある。Ask Geminiが送信・削除の前に確認を求めるかは未確認。 | [発表][blog] [Ask Gemini][askhelp] |
| 承認（Workspace Studioのフロー） | 管理者はセキュリティポリシーで、フローの特定の操作に本人の承認を必須にできる。対象の例はメール送信、スペースへの投稿、共有ファイルの変更、外部ゲストを含む予定の作成。承認が必要な手順でフローは止まり、承認すると実行し、却下するとその手順を飛ばす。 | [承認][studioappr] |
| 外部連携 | Ask Geminiから`@HubSpot`、`@Asana`、`@Salesforce`などのサードパーティ連携を呼ぶ。連携はGoogle Workspace Marketplaceで接続済みのものだけで、管理者のインストールが必要な場合がある。回答は連携先を出典として示す。Ask Geminiに任意のMCPサーバーをつなぐ仕様は未確認。 | [Ask Gemini][askhelp] |
| 開発者向けエージェント | Google ChatアプリからAgent Development Kit（ADK）で作ったA2Aエージェント（Vertex AI Agent Engineで実行）や、Gemini EnterpriseのAIエージェントを呼ぶ手順がある。A2Aエージェントの開発と利用はPreview。 | [A2Aクイックスタート][qa2a] [Gemini Enterpriseクイックスタート][qge] |
| 履歴の書き出し | Geminiの会話履歴は管理者がデータエクスポートで書き出せ、組織が許可すれば利用者もGoogle Takeoutでダウンロードできる。保存先の区分は「Gemini in Workspace」。 | [発表][blog] [Gemini in Chat][collab] |

## UI/UX

| 要素 | 内容 | 出典 |
|---|---|---|
| 画面構成 | 左の「Shortcuts」にAsk Geminiがあり、検索バーの右のボタンや、Ctrl + g（ChromeOS・Windows）／Command + g（macOS）でも開く。GmailのChatからも使える。 | [Ask Gemini][askhelp] |
| セッション | トピックごとに新しいセッションを始め、最近の会話から過去のセッションを開く。開くとそのセッションのやり取りがまとめて表示される。上部に推奨トピックが出る。 | [Ask Gemini][askhelp] |
| 毎日のまとめ | その日のタスク、予定、フォローアップを「Daily brief」というセッションに自動で届ける。既定で有効で、まとめの下から購読をやめられる。 | [Ask Gemini][askhelp] |
| 会話への書き込み | Gemini in Chatでは、生成した文を「Insert」でChatに挿入する。他に「Copy」「Retry with Google Search」「Good suggestion」「Bad suggestion」がある。 | [Gemini in Chat][collab] |
| 出典の表示 | 回答に出典の引用を付ける。サードパーティ連携の回答では連携先のアプリを出典にし、元のリンクを付けることが多い。 | [Ask Gemini][askhelp] |
| Deep Researchの計画確認 | 「Deep research sources」カードでDrive・Gmail・Chat・Webを選び、作られた調査計画を「Customize」で直してから「Start research」で始める。調査は裏で続き、終わるとDocsにレポートを作りメールで知らせる。 | [Ask Gemini][askhelp] |
| 承認の通知と操作（Workspace Studio） | 承認が必要になると、フロー名のChatのDMに詳細とStudioへのリンクが届く。Chatでは操作名の緑のボタン（「Send email」「Add content」など）か「Reject」を押す。Studioの承認一覧では状態（Pending、Approved、Rejected、Expire soon、Expired）と製品で絞り込み、まとめて承認・却下できる。まとめて処理しても記録は1件ずつ残る。 | [承認][studioappr] |
| 代理実行の表示（Workspace Studio） | 承認された手順がメールやChatを送ると、送信者は「[フロー名] for [本人の名前]」と表示される。フローを共有しても承認は実行する本人に届き、コピーした人のフローの承認はコピーした人に届く。 | [承認][studioappr] |
| フローの作成 | 「Describe a task for Gemini」に、いつ始めるか、どのアプリを使うか、どの手順を踏むかを書くと、Geminiが手順を作る。人が各手順を確認・編集し、Test runで試してから「Turn on」で有効にする。 | [フロー作成][studioflow] |

## 技術

Google ChatとGeminiはクローズドなので、公式ドキュメントに書かれた範囲だけを記す。

| 層 | 技術 | 出典 |
|---|---|---|
| Chatアプリの受け取り | 操作イベント（ADDED_TO_SPACE、メッセージ、CARD_CLICKEDなど）を受け、30秒以内に同期で返すか、Chat APIで非同期に返す。実装はApps ScriptかHTTPエンドポイント。UIはカード、ダイアログ、App Home、リンクプレビュー。 | [Chatアプリの操作イベント][interact] |
| エージェント連携 | A2A（Agent2Agent）プロトコル、Agent Development Kit（ADK）、Vertex AI Agent Engine、Gemini Enterprise。クイックスタートのサンプルの依存は`google-adk`、`google-cloud-aiplatform`、`google-genai`、`pydantic`（Python、Poetry）。 | [A2Aクイックスタート][qa2a] [Gemini Enterpriseクイックスタート][qge] |
| 横断検索の基盤 | Workspace Intelligence。内部構成は未確認。 | [発表][blog] |
| オープンソース | A2Aプロトコルの仕様リポジトリ（`a2aproject/A2A`）はApache-2.0。Google ChatとGeminiの本体のソースコードの公開は未確認。 | [a2aproject/A2A][a2arepo] |
| サーバー構成・データ保存 | 未確認。 | — |

## Pelwayとの比較［提案］

この節は本書の整理と提案であり、Googleの資料が述べている内容ではない。Pelway側は[ビジョン](../vision.md)、[PRD](../prd.md)、[技術](../technical.md)の構想で、実装済みの機能ではない。

| 観点 | Google Chat / Gemini | Pelway（構想） | Pelwayでの扱い |
|---|---|---|---|
| 人とAIの同席 | Ask Geminiは個人のセッション。複数AIの同席は未確認 | 中核の方針 | 共有チャンネルでAIが働く点は比較の観点にできる |
| 外部操作前の承認 | Workspace Studioのフローに、操作名のボタンと却下、承認一覧がある。Ask Geminiは未確認 | 承認を実行内容に結び付ける | 「Send email」のように操作名をボタンに出す形と、状態で絞れる承認一覧を参考にする |
| 承認者 | フローを動かす本人に届く | 誰の判断待ちかを表示し、承認者を決める | 本人以外（代表など）に承認を回す点で差を出せる |
| 代理実行の表示 | 「[フロー名] for [本人]」 | AIであることを明示 | 送信物にAIの名前と承認者を両方示す |
| AIの書き込み | Gemini in Chatは生成した文を人の会話に挿入する | AIが会話に投稿する | 人の名義の下書きにする形と、AIが承認後に送る形の使い分けを検討する |
| 計画の確認 | Deep Researchで計画を直してから始める | 判断待ちをMVP候補にしている | 長い作業の前に計画を確かめる手順を参考にする |
| 言語 | Ask Geminiは英語のみ | 日本語の小規模チームを想定 | 日本語で使える点を比較の観点にする |

## 未確認事項

- Ask Geminiがタスクや予定の削除、メッセージ送信の前に確認を求めるか。
- 複数のChatアプリやエージェントが同じスペースで互いのメッセージを受け取れるか。
- Ask Geminiに任意のMCPサーバーをつなげるか。
- Ask Geminiの利用上限の値。
- エージェントの実行環境の分離単位、保存地域。

## 出典

- [Google Workspace Updates：Introducing Ask Gemini in Chat](https://workspaceupdates.googleblog.com/2026/08/ask-gemini-in-chat.html)
- [Google Chat Help：Get started with Ask Gemini in Google Chat](https://support.google.com/chat/answer/17036303?hl=en)
- [Google Chat Help：Collaborate with Gemini in Google Chat](https://support.google.com/chat/answer/15345722?hl=en)
- [Workspace Studio Help：Manage flow approval requests](https://support.google.com/workspace-studio/answer/17516423?hl=en)
- [Workspace Studio Help：Create a flow with Gemini AI](https://support.google.com/workspace-studio/answer/16448469?hl=en)
- [Google Workspace Help：Compare Flexible and Annual/Fixed-Term payment plans](https://knowledge.workspace.google.com/admin/billing/compare-flexible-and-annual-fixed-term-payment-plans)
- [Google Workspace：Pricing](https://workspace.google.com/pricing/?hl=en&country=US)
- [Google for Developers：Receive and respond to user interactions](https://developers.google.com/workspace/chat/receive-respond-interactions?hl=en)
- [Google for Developers：Build a Google Chat app with an Agent2Agent agent](https://developers.google.com/workspace/add-ons/chat/quickstart-a2a-agent?hl=en)
- [Google for Developers：Build a Google Chat app with a Gemini Enterprise AI agent](https://developers.google.com/workspace/add-ons/chat/quickstart-ge-agent?hl=en)
- [A2A Project：a2aproject/A2A](https://github.com/a2aproject/A2A)

[blog]: https://workspaceupdates.googleblog.com/2026/08/ask-gemini-in-chat.html
[askhelp]: https://support.google.com/chat/answer/17036303?hl=en
[collab]: https://support.google.com/chat/answer/15345722?hl=en
[studioappr]: https://support.google.com/workspace-studio/answer/17516423?hl=en
[studioflow]: https://support.google.com/workspace-studio/answer/16448469?hl=en
[plans]: https://knowledge.workspace.google.com/admin/billing/compare-flexible-and-annual-fixed-term-payment-plans
[price]: https://workspace.google.com/pricing/?hl=en&country=US
[interact]: https://developers.google.com/workspace/chat/receive-respond-interactions?hl=en
[qa2a]: https://developers.google.com/workspace/add-ons/chat/quickstart-a2a-agent?hl=en
[qge]: https://developers.google.com/workspace/add-ons/chat/quickstart-ge-agent?hl=en
[a2arepo]: https://github.com/a2aproject/A2A
