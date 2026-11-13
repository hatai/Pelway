# Bullpen

確認日：2026-11-11（JST、日本時間）

一覧と比較の前提は[競合調査](../competitors.md)を参照する。「確認」は公式資料・公開リポジトリに記載があるという意味で、実機検証済みではない。「未確認」は今回の資料で裏付けられなかった項目で、機能がないという意味ではない。

## 概要

| 項目 | 内容 |
|---|---|
| 提供元 | Bullpen Chat LLC（米国New Jersey州のLLC）。[Privacy][privacy] |
| 公式URL | [bullpen.chat][top] |
| 製品の説明 | 常時開いている音声ルームと6体のAI helpersを持つチームチャット。チームのworkspaceを「bullpen」と呼ぶ。channel、private message、検索は全planで無制限。[公式][top] |
| 提供段階 | Web、Macアプリ（Apple Silicon、macOS 12以降）、iPhoneアプリ。Androidはブラウザのみ。[公式][top] |
| ライセンス | 未確認。公開リポジトリは見つからなかった。 |
| 価格 | bullpen単位の料金（"One plan for your bullpen. Seats for your people."）。Free $0（最大5人、helpersのお試し分の利用量、files 200 MB）。Starter $29／月（最大10人、2 GB）、Team $79／月（最大25人、Starterの3倍の利用量、10 GB）、Business $199／月（最大75人、Starterの10倍、50 GB）。表示はいずれも"Billed monthly"。年払いの切替に"2 months free"とあるが、年額は表示から未確認。音声の分数は別に数え、税は必要な場合に決済時に加える。[公式][top] |
| 利用量 | 各planに月の利用枠があり、bullpen全体で共有する。各回答の原価で数え、毎月1日にリセットする。helpersは全planに含まれ、人の席を使わない。[公式][top] [seat記事][seat] |
| 個人用Ace | 個人のassistant。Ace $12／月または$99／年、Ace Plus $29／月または$239／年、Ace Max $79／月または$649／年。利用枠はplan開始日に毎月リセットし、月2回まで追加購入できる（$6、$15、$39）。[Support][support] |

## 機能

| 観点 | 内容 | 出典 |
|---|---|---|
| 複数人＋複数AIの同一チャンネル参加 | helpersは人と同じroster、同じchannel、同じ管理者ルールのもとにいる。名前をメンションしてthreadに呼ぶ。 | [公式][top] [channel members記事][members] |
| AIの役割・専門性の設定 | 6体が職務を分担する。Scout（チームの発言・決定を答え、元のメッセージにリンク）、Wren（音声通話のメモと週の要約）、Pip（#standupへの日次要約）、Nix（reminderと期日のある約束の追跡）、Biscuit（新メンバーの導入とBullpenの使い方）、Rook（file作成と案件の進み具合）。管理者はBullpen settings → AI helpersから独自のhelperを最大6体追加できる。独自helperの設定項目は未確認。 | [公式][top] |
| 実行能力 | 全planでメンションに答える。有料planでは共有fileを読み、spreadsheet、document、slides、PDFを作る。会議メモと日次要約、file作成は有料plan。Bullpen外のサービスへの操作は未確認。 | [公式][top] |
| 承認・権限管理 | helperは依頼した人が見られるchannelと会話だけを見る。権限はhelperごとに一度決めるのではなく、質問ごとに確認する。回答はさらに投稿先channelの読者が開けるものに絞る。管理者がhelperの働くchannel、追加・削除できる人、読める範囲を決める。外部操作前の承認は未確認。 | [公式][top] [channel members記事][members] |
| 監査 | helperの回答はすべて原価と投稿先channelとともに記録され、管理者はhelperの行動と費用を見られる。回答は元のメッセージにリンクするか、見つからなかったと書く。 | [公式][top] |
| Ace | 本人だけが使うassistant。本人が開けるもの（private messageを含む）だけを読み、質問と回答は本人にだけ見える（"No admin sees your questions or its answers"）。Aceが本人について持つメモを読む、消す、記憶を止めることができる。 | [公式][top] [Support][support] |
| 外部連携 | 外部SaaS、API、MCPへの対応は未確認。Scoutのweb検索（`@Scout search:`）はExaを使う。 | [公式][top] [Privacy][privacy] |
| その他 | 常時開いている音声ルーム（Freeに含む）。データの書き出しはAccount settings → Your data → Export my data。 | [公式][top] [Support][support] |

## UI/UX

| 要素 | 内容 | 出典 |
|---|---|---|
| channelでのhelperの見え方 | helperは自分の名前とAIのbadgeで、他の人と同じthreadに投稿する。rosterでは人の隣に並ぶ。 | [channel members記事][members] |
| helperカード | 各helperのカードに、何をするか、自分から何をするか、試せる質問3つを表示する。モーダルの文言は"Each AI helper has one job. Open one to see what you can ask."。 | [公式][top] |
| 出典付きの回答 | Scoutが#generalで「payrollについて何を決めたか」に答え、元のメッセージへのリンクを付ける画面例。 | [公式][top] [出典記事][cite] |
| 期日の一覧 | Nixが今日が期日の項目を並べ、各項目にDoneボタンがある画面例。 | [公式][top] |
| 音声ルーム | 例として"Design corner"のルームに、マイクありか消音で入る選択肢。Wrenがルームの属するchannelにメモを投稿する。 | [公式][top] [channel members記事][members] |
| Ace | Homeにあり、デスクトップアプリのYou → Aceで有効にする。 | [公式][top] [channel members記事][members] |
| Macアプリ | 署名・公証済みで、Terminalは不要。Webアプリも同じ製品。 | [公式][top] |
| 承認画面・helper作成の手順 | 未確認。 | [公式][top] |

## 技術

公式のプライバシーページから確認した。公開リポジトリは見つからず、実装言語・フレームワークは未確認。

| 層 | 技術 | 出典 |
|---|---|---|
| ホスティング | Vercel（アプリ、サーバー、CDN）。 | [Privacy][privacy] |
| DB・認証・ストレージ | Supabase（アカウント、サインイン、DB、非公開のstorage bucket）。全tableにrow-level security。特権のDB鍵はブラウザに渡さない。録音と添付は非公開bucketで、署名付きリクエストで読む。 | [Privacy][privacy] |
| モデル | OpenRouter経由で、Microsoft Azure上のOpenAIモデル。会議音声にはGoogle Vertex上のGemini。providerが保存・学習しない設定で呼ぶとし、内容を学習に使わないと明記。 | [Privacy][privacy] |
| その他の処理者 | Exa（web検索）、Cloudflare（PDFの文字抽出）は保存しない設定の対象外。LiveKit（音声）、Stripe（決済）、Resend（メール）。 | [Privacy][privacy] |
| 保存期間 | メッセージ、file、channelは削除まで保存。crash reportは約30日。録音とsecurity logに期間による自動削除はない。 | [Privacy][privacy] |
| ログ | crash reportは自社DBに置く。security logは短縮したネットワークアドレスだけを持ち、workspaceのownerとadminが読める。 | [Privacy][privacy] |
| 暗号化 | 転送時・保存時の暗号化の記載はなく未確認。 | [Privacy][privacy] |
| エージェント連携のプロトコル | MCPなどは未確認。 | [公式][top] |

## Pelwayとの比較［提案］

この節は本書の整理と提案であり、Bullpenの資料が述べている内容ではない。Pelway側は[ビジョン](../vision.md)、[PRD](../prd.md)、[技術](../technical.md)の構想で、実装済みの機能ではない。

| 観点 | Bullpen | Pelway（構想） | Pelwayでの扱い |
|---|---|---|---|
| 人とAIの同席 | 確認。AIのbadgeで明示 | 中核の方針 | 同席だけでは差別化にならない |
| AIの役割 | 6体の固定の職務と、最大6体の独自helper | 役割と名前を持つAI | 「1体に1つの仕事」とカードで聞けることを示す見せ方を参考にする |
| 権限の確認 | 質問ごとに依頼者と投稿先の両方で絞る | 依頼者・承認者・channelの権限を実行直前に確認 | 投稿先の読者で回答を絞る考え方を、承認内容の表示範囲にも使えるか検討する |
| 外部操作前の承認 | 未確認。外部操作の説明自体が未確認 | 外に出る操作は毎回承認 | 比較対象は会議・要約・file作成。外部操作の比較には向かない |
| 引き継ぎ・判断待ち・再開 | Nixの期日一覧、Pipの日次要約 | 判断待ちを「今日」画面にまとめる | 期日の一覧にDoneを置く画面を「今日」画面の参考にする |
| 費用の記録 | 回答ごとに原価とchannelを記録、管理者が見る | 原価の仮説を検討中 | AIの原価を回答単位で見せる管理画面を参考にする |
| 価格 | bullpen単位、helpersは席を使わない、利用枠を共有 | 人の参加費とAI実行枠を分ける案 | workspace単位の料金とAIを席に数えない方式を比較の対照にする |
| 個人用AI | Aceは本人だけ、管理者も見ない | 個人とAIの会話を共有する負担を課題仮説にしている | 個人用AIの成果をチームへ出す操作の有無を比較の観点にする |

## 未確認事項

- 独自helperの設定項目と作成画面。
- Bullpen外のサービスへの操作、MCP・APIへの対応、外部操作前の承認。
- workspace planの年額。
- 暗号化、保存地域、日本語対応、SLA。

## 出典

- [Bullpen 公式（料金・FAQを含む）](https://bullpen.chat/)
- [Bullpen：Privacy（Securityを含む）](https://bullpen.chat/privacy)
- [Bullpen：Support](https://bullpen.chat/support)
- [Bullpen Blog：AI helpers as channel members](https://bullpen.chat/blog/agents-as-channel-members)
- [Bullpen Blog：AI helpers don't take a seat](https://bullpen.chat/blog/agents-dont-take-a-seat)
- [Bullpen Blog：AI helpers that cite the message](https://bullpen.chat/blog/agents-that-cite-sources)

[top]: https://bullpen.chat/
[privacy]: https://bullpen.chat/privacy
[support]: https://bullpen.chat/support
[members]: https://bullpen.chat/blog/agents-as-channel-members
[seat]: https://bullpen.chat/blog/agents-dont-take-a-seat
[cite]: https://bullpen.chat/blog/agents-that-cite-sources
