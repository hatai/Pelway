# Rerun（Reunit SA）

確認日：2026-11-11（JST、日本時間）

一覧と比較の前提は[競合調査](../competitors.md)を参照する。「確認」は公式資料・公開リポジトリに記載があるという意味で、実機検証済みではない。「未確認」は今回の資料で裏付けられなかった項目で、機能がないという意味ではない。

## 概要

| 項目 | 内容 |
|---|---|
| 提供元 | Reunit SA（ルクセンブルクの株式会社）が運営と利用規約に記載。[利用規約][terms] |
| 公式URL | [rerun.build][rerun]、[ドキュメント][docs]、[rerun-build/rerun][repo] |
| 製品の説明 | AIの同僚（Coworker）を雇い、利用者のツールを使わせ、スケジュールで動かし、重要な操作の前に確認させる。Chief of Staffという取りまとめ役のAIが利用者に聞き取りをして、チームを組む。[README][repo] |
| 提供形態 | 自社ホストのRerun Cloudと、セルフホストできるオープンソース版。両者は同じコードと説明。[README][repo] |
| ライセンス | AGPL-3.0。ただし`ee`ディレクトリ（課金、Rerun credit、マネージドモデル、テンプレートのマーケットプレイス）はRerun Commercial Licenseで、オープンソース版では有効にならない。[README][repo] |
| 価格 | Pro $29/月。席は無制限で人単位の料金はない。Coworker無制限、月500 credits。年額の切り替えに「-20%」表示があるが年額の金額は未表示。追加はLinkedIn $9/月、WhatsApp $9/月、1,000 credits/月で$10/月。試用は5日間$0（カード登録が必要、Coworker 3体まで、350 credits）。Enterpriseは個別見積もり。税別。[料金][pricing] |
| creditの用途 | 料金ページはRerunがホストするモデルに使うと説明。トップページはcreditが3,000以上のツールへの要求にも使われると説明。利用者のClaude・ChatGPTの契約、API key、ローカルモデルで動かす分はRerun側の費用がかからない。[料金][pricing] [公式][rerun] |

## 機能

| 観点 | 内容 | 出典 |
|---|---|---|
| 複数人＋複数AIの同一チャンネル参加 | workspaceのメンバーは同じboard、Box、Coworkerを見て、Coworkerの会話とデータも共有する。会話はCoworkerごとのsession（スレッド）で、複数人と複数AIが1つの会話チャンネルに入る仕様は未確認。 | [Teams][teams] [Concepts][concepts] |
| AIの役割・専門性の設定 | Coworkerは名前、handle（@メンションの宛先）、soul（system prompt）、モデル、使える道具の種類、記憶、skill、ファイル、DBを持つ。モデルは会話ごとに変えられる。テンプレートから作るか、仕事を一文で説明して作る。 | [Concepts][concepts] [公式][rerun] |
| AI間の引き継ぎ | Coworkerは別のCoworkerを呼び出して仕事を渡す（例：見込み客を探す、連絡する、CRMに登録する）。他のCoworkerからの呼び出しも実行（run）の起点になる。 | [公式][rerun] [Concepts][concepts] |
| 実行能力 | Coworkerごとの画面とブラウザで、APIのないアプリも操作する。shell、ファイル、DB。 | [Computer use][cu] [Concepts][concepts] |
| 承認・権限管理 | 製品として強制されるのは支出で、承認なしに購入できない。送信や削除などは利用者がCoworkerに与える指示で決まる。止まったrunはboardと通知に質問を残して待つ。workspaceの誰でも承認でき、最初に応答した人がrunを再開させる。autopilotがオンなら、Chief of Staffが利用者の書いたルールで質問に答える。 | [Human in the loop][hitl] |
| 役割（人） | Owner、Builder（Box、Coworker、アプリ、自動化を作る。招待したメンバーの既定）、Viewer（閲覧）。 | [Teams][teams] |
| 外部連携 | アプリ（Stripe、Gmail、Notion、Slackなど）はCoworkerごとに入れる。任意のMCPサーバーを登録できる。READMEは「200+の連携、任意のMCP、Composio経由の2,000+のアプリ」、トップページは「3,000+の連携」と記載。 | [Concepts][concepts] [README][repo] [公式][rerun] |
| 起動条件 | スケジュール（cronとタイムゾーン、一回限り）とwebhook。自動のrunは別sessionで動き、利用者の会話に混ざらない。 | [Concepts][concepts] |
| 記録 | runごとに手順、ツール呼び出し、トークン、費用を記録し、MonitoringとLogsのタブで読む。 | [Concepts][concepts] |

## UI/UX

| 要素 | 内容 | 出典 |
|---|---|---|
| 画面構成 | アプリそのものが等角投影のboardで、Box（部署・顧客・案件ごとの部屋）が四角形、Coworkerがその上に座る。Coworkerを押すと右側にパネルが開き、会話のタブ、メッセージ欄、モデルの選択が並ぶ。左下に「3 need you」のような対応待ちの件数が出る。左上のworkspaceの切り替えボタンからメンバー管理を開く。 | [Concepts][concepts] [スクリーンショット][screen] [Teams][teams] |
| Coworkerの発言 | boardの上でCoworkerの報告が吹き出しで表示される。会話では使ったツールと結果（例：LinkedIn → 62件読んだ）が一覧で出る。 | [スクリーンショット][screen] |
| 承認・質問のカード | runが止まるとメッセージ欄の上にカードが出る。質問（候補の回答ボタン、複数選択、自由入力）、承認（実行したい操作の一覧とコマンド・対象・差分。項目ごとに承認・却下）、アプリ接続、認証情報（マスクした入力欄）の4種類。質問は1枚のカードにまとめ、一度に1つだけ尋ねる。承認をAPIで代行するツールはない。 | [Human in the loop][hitl-doc] |
| 対応待ちの一覧 | 待っている項目はCoworkerのTo handleタブと、workspaceの通知パネルに集まる。通知は種類と経路（アプリ内、メール）ごとに設定する。 | [Human in the loop][hitl-doc] |
| 画面の引き継ぎ | 実行中の画面を開いて操作を追い、いつでもマウスを取り返し、また渡せる。ログインは一度だけ行い、次のrunで再利用する。 | [Computer use][cu] |
| Coworker作成 | Chief of Staffが利用者に聞き取りをしてチームを計画し、Coworkerを雇う。音声で頼むこともできる。 | [README][repo] |
| Boxの移動 | boardでCoworkerを右クリックしてMove to Boxを選ぶ。移動しても、ファイル、handle、skill、履歴はそのまま。 | [Concepts][concepts] |
| モバイル | スマートフォンのブラウザで使い、ホーム画面に追加できる。リポジトリにはExpo（React Native）のモバイルアプリもある。 | [公式][rerun] [mobile package.json][mobile] |

## 技術

公開リポジトリの依存定義から確認した。

| 層 | 技術 | 出典 |
|---|---|---|
| 構成 | Webアプリ（Next.js、`apps/ui`）、Supabase（Postgres、認証、ストレージ、realtime）、workspaceごとに1つのEngine（`apps/engine`）。 | [README][repo] |
| 分離の単位 | workspaceは1台の非公開のクラウド計算機で動き、他のアカウントから分離される。その計算機がworkspaceの全Coworkerと、共有のファイル、DB、認証情報を持つ。Boxは分離の単位ではない。Rerun Cloudの仮想化技術は未確認。所在地は欧州。 | [Concepts][concepts] [Computer use][cu] |
| Engine | Node.js 22のDockerイメージ。人が見る画面にXvfb、openbox、x11vnc、xdotool。ブラウザはPatchrightのChromium。 | [engine Dockerfile][enginedocker] |
| Engineの依存 | Hono、Vercel AI SDK（Anthropic、OpenAI、Google）、`ai-sdk-provider-claude-code`、`ai-sdk-provider-codex-cli`、`@modelcontextprotocol/sdk`、libSQL、drizzle-orm、croner。 | [engine package.json][enginepkg] |
| フロントエンド | Next.js、React 19、Tailwind、Radix UI、Tiptap、CodeMirror、noVNC（画面の表示）、Stripe、Supabase、Trigger.dev、next-intl。 | [ui package.json][uipkg] [package.json][pkg] |
| モバイル | Expo、React Native。 | [mobile package.json][mobile] |
| セルフホスト | Docker Compose（appとengine）、Supabase。 | [docker-compose.yml][compose] [README][repo] |
| ビルド | pnpm workspaceのmonorepo、TypeScript。 | [package.json][pkg] |

## Pelwayとの比較［提案］

この節は本書の整理と提案であり、Rerunの資料が述べている内容ではない。Pelway側は[ビジョン](../vision.md)、[PRD](../prd.md)、[技術](../technical.md)の構想で、実装済みの機能ではない。

| 観点 | Rerun | Pelway（構想） | Pelwayでの扱い |
|---|---|---|---|
| 人とAIの同席 | boardを共有し、会話はCoworkerごと | 複数人と複数AIが同じチャンネル | 同じ会話での同席は差を出せる可能性がある |
| 承認者 | workspaceの誰でも承認でき、最初の応答で再開 | 判断する人を指定する | 「誰の判断待ちか」を固定する方針を保つ |
| 承認の強制 | 製品で強制するのは支出のみ。送信・削除は指示次第 | 外部操作は承認を実行内容に結び付ける | 送信も製品側で止める点を比較の観点にする |
| 承認の表示 | メッセージ欄の上のカード、項目ごとの承認、差分表示 | 本文・宛先・添付を固定し、変更時は差分を見せる | 項目ごとの承認と差分表示を参考にする |
| 対応待ち | To handleタブ、board左下の件数 | 判断待ちを「今日」画面にまとめる | 件数と一覧の両方を出す方式を参考にする |
| 実行環境 | workspaceで1台。資格情報は全Coworkerで共有 | AIごとの実行環境を検討 | 資格情報の共有範囲の違いを試作で比べる |
| 価格 | workspace単位$29/月、席は無制限 | 人の参加費とAI実行枠を分ける案 | 席を無料にする設計の採算を検討材料にする |

## 未確認事項

- 複数人と複数AIが1つの会話チャンネルに入る機能。
- Viewerが承認できるか。
- Rerun Cloudの仮想化技術とworkspaceの計算機の構成。
- 年額の価格、日本語対応、SLA。

## 出典

- [Rerun公式サイト](https://rerun.build/)
- [Rerun：Pricing](https://rerun.build/pricing)
- [Rerun：Terms](https://rerun.build/terms)
- [Rerun：Human in the loop（機能ページ）](https://rerun.build/features/human-in-the-loop)
- [Rerun：Computer use（機能ページ）](https://rerun.build/features/computer-use)
- [Rerun：ドキュメント](https://docs.rerun.build)
- [Rerun：rerun-build/rerun README](https://github.com/rerun-build/rerun)
- [Rerun：docs Core concepts](https://github.com/rerun-build/rerun/blob/main/apps/docs/concepts.mdx)
- [Rerun：docs Teams and workspaces](https://github.com/rerun-build/rerun/blob/main/apps/docs/teams.mdx)
- [Rerun：docs Human in the loop](https://github.com/rerun-build/rerun/blob/main/apps/docs/agents/human-in-the-loop.mdx)
- [Rerun：package.json](https://github.com/rerun-build/rerun/blob/main/package.json)
- [Rerun：apps/engine/package.json](https://github.com/rerun-build/rerun/blob/main/apps/engine/package.json)
- [Rerun：apps/engine/Dockerfile](https://github.com/rerun-build/rerun/blob/main/apps/engine/Dockerfile)
- [Rerun：apps/ui/package.json](https://github.com/rerun-build/rerun/blob/main/apps/ui/package.json)
- [Rerun：apps/mobile/package.json](https://github.com/rerun-build/rerun/blob/main/apps/mobile/package.json)
- [Rerun：docker-compose.yml](https://github.com/rerun-build/rerun/blob/main/docker-compose.yml)
- [Rerun：README スクリーンショット（screen-team）](https://github.com/rerun-build/rerun/blob/main/.github/assets/screen-team.jpg)

[rerun]: https://rerun.build/
[pricing]: https://rerun.build/pricing
[terms]: https://rerun.build/terms
[hitl]: https://rerun.build/features/human-in-the-loop
[cu]: https://rerun.build/features/computer-use
[docs]: https://docs.rerun.build
[repo]: https://github.com/rerun-build/rerun
[concepts]: https://github.com/rerun-build/rerun/blob/main/apps/docs/concepts.mdx
[teams]: https://github.com/rerun-build/rerun/blob/main/apps/docs/teams.mdx
[hitl-doc]: https://github.com/rerun-build/rerun/blob/main/apps/docs/agents/human-in-the-loop.mdx
[pkg]: https://github.com/rerun-build/rerun/blob/main/package.json
[enginepkg]: https://github.com/rerun-build/rerun/blob/main/apps/engine/package.json
[enginedocker]: https://github.com/rerun-build/rerun/blob/main/apps/engine/Dockerfile
[uipkg]: https://github.com/rerun-build/rerun/blob/main/apps/ui/package.json
[mobile]: https://github.com/rerun-build/rerun/blob/main/apps/mobile/package.json
[compose]: https://github.com/rerun-build/rerun/blob/main/docker-compose.yml
[screen]: https://github.com/rerun-build/rerun/blob/main/.github/assets/screen-team.jpg
