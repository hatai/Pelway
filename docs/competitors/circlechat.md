# CircleChat

確認日：2026-11-11（JST、日本時間）

一覧と比較の前提は[競合調査](../competitors.md)を参照する。「確認」は公式資料・公開リポジトリに記載があるという意味で、実機検証済みではない。「未確認」は今回の資料で裏付けられなかった項目で、機能がないという意味ではない。

## 概要

| 項目 | 内容 |
|---|---|
| 提供元 | Neu Software LLC（公式サイトのフッターと組織情報）。ソースは個人アカウント `tashfeenahmed` のリポジトリで公開。[公式][site] [README][repo] |
| 公式URL | [circlechat.co][site]、[tashfeenahmed/circlechat][repo] |
| 製品の説明 | 人とAIエージェントを同じ種類のメンバーとして扱う、セルフホスト可能なチームチャット。チャンネル、DM、スレッド、リアクション、チャンネルごとのカンバン、承認・記憶・ファイル共有を持つエージェントの実行基盤を備え、モデルは持ち込み。[README][repo] |
| 提供段階 | セルフホスト版とマネージド版（CircleChat Cloud）を提供。公開デモのworkspaceがある。[公式][site] [料金][price] |
| ライセンス | MIT。[README][repo] [公式][site] |
| 価格 | workspace単位の月額。Starter $29/mo（3 agents、専用のシングルテナントサーバー、`yourname.circlechat.co`）、Team $79/mo（10 agents、独自ドメイン）、Scale $199/mo（agents無制限）。全プラン7日間の無料トライアル、カード登録が必要、いつでも解約可。トークンへの上乗せなし。セルフホストは無料。[料金][price] |

## 機能

| 観点 | 内容 | 出典 |
|---|---|---|
| 複数人＋複数AIの同一チャンネル参加 | エージェントは人と同じメンバーとして、チャンネル、DM、スレッド、リアクション、添付ファイルを人のUIと同じ内容で受け取る。メンション（`@everyone`、`@channel`を含む）で呼べる。 | [README][repo] |
| AIの役割・専門性の設定 | エージェントは自分のハンドル、アバター、役割、報告先（組織図）を持つ。identity・skills・toolsをバージョン管理されたファイルで定義し、テンプレートから作れる。タスクは宣言されたskillで割り当て、負荷が高いと割り当て直す。 | [README][repo] [公式][site] |
| 目標と計画 | 目標を書くと、計画役のエージェントが担当者と受け入れ条件付きのタスクに分ける。依存関係を追い、進捗を集計する。 | [公式][site] |
| 実行能力 | エージェントは自分のコンテナでコードを実行し、ファイルを編集する。Web検索、HTTP・MCPの連携、成果物の `/workspace` への保存とタスクへの添付。同梱のHermes・OpenClawは1ターンごとに短命のコンテナを起動する。 | [README][repo] |
| 承認・権限管理 | 危険な操作（デプロイ、支払い、外部へのメール）は人の承認まで止まる。エージェントが `request_approval` を出し、決定後に `approval_response` で起こされる。承認者は資格情報を添えられ、チャットやDBを通さずにエージェントの環境へ渡る。エージェントは与えられたscopeだけで始まり、範囲外の操作は管理者が判断する承認カードになる。権限はサーバー側で強制すると説明。 | [README][repo] [custom-agents][agents] [公式][site] |
| 検証ゲート | タスクがreviewからdoneに移る前に、LLMの判定役が成果物を受け入れ条件と照らす。Webの成果物はヘッドレスChromiumで表示を確認できる。判定役に届かないときの既定は通す（`VERIFY_FAIL_MODE=open`）で、`closed`（拒否）と`hold`（reviewに留める）も選べる。既定は無効（opt-in）。 | [README][repo] [CONFIG][config] |
| 返信の検査 | サーバー側でエラー出力、拒否文、ツール呼び出しのJSON、トークン漏れ、繰り返しなどを含む返信を拒否する（reply-guard）。 | [README][repo] |
| 長い実行の制御 | エージェント・ワークフローの実行に、claim、release、steer、follow_up、extend、cancelを送れる。取り消し後に戻ってきた結果は適用しない。 | [p1-platform][p1] |
| 外部連携 | 汎用HTTPとリモートMCPのコネクター登録（資格情報は暗号化、エージェントごとに付与）、OAuth 2、署名付きwebhook、永続ワークフロー、GitHub・GitLabのPRをチャンネルに結び付けるPR room。 | [README][repo] |
| 監査 | エージェントの実行、拒否した返信、承認を、問い合わせ可能な行として残す。監査ログの書き出し、保持期間、保存場所のポリシー。 | [README][repo] |

## UI/UX

| 要素 | 内容 | 出典 |
|---|---|---|
| 画面構成 | 左のサイドバーにBoard、Files、Approvals、Skills、Org chart、CHANNELS、DIRECT MESSAGES。DMの一覧では人と並んでエージェントに「AGENT」の表示が付く。上部にメッセージ検索。 | [スクリーンショット][chat-img] |
| AIの見え方 | メッセージは「名前 @ハンドル · 役割名」の形で、人と同じ見た目で並ぶ（例：Nova · Chief Executive Officer）。考え中は「nova is thinking…」と表示する。 | [スクリーンショット][chat-img] [公式][site] |
| エージェント間の受け渡し | 公式サイトの例では、エージェントが公開チャンネルで別のエージェントをメンションして続きを渡す。エージェントはボードのタスクを担当に割り当てられる（`assign_task`）。 | [公式][site] [README][repo] |
| 承認カード | 「Approval required · deploy」の見出しに、対象（production）、変更の範囲（1 page added, 0 removed）、rollbackの準備状況を出し、Approve / Denyと、エージェントへのメモ欄を置く。決定後は「Approved by ... · deployed」と表示される。 | [公式][site] |
| Needs you | 承認待ち、reviewのタスクと検証失敗、止まった目標、人を待つワークフロー、予算の警告・停止、コネクターの異常、アプリ公開の依頼を、権限に応じて優先順にまとめた1つの一覧。判断できるものはその場で判断でき、元の画面へのリンクを持つ。 | [p1-platform][p1] |
| ボード | backlog → in_progress → review → done のカンバン。更新されたカードに印を付ける。 | [README][repo] |
| エージェント作成 | Members → Provision agent でランタイム（socket / webhook）とアダプターを選ぶと、botトークンとインストール用のコマンドが出る。最初の登録時のウィザードで最初のエージェントを入れられる。 | [README][repo] |

## 技術

公開リポジトリの依存定義と文書から確認した。

| 層 | 技術 | 出典 |
|---|---|---|
| 言語 | TypeScript。 | [README][repo] |
| サーバー | Fastify 5、Drizzle ORM、zod、bcryptjs（自前のセッション）。 | [api package.json][apipkg] [README][repo] |
| DB・キュー | Postgres 16、Redis 7（pub/sub、キュー）、BullMQ（エージェントの実行ワーカー）。 | [README][repo] [compose.yml][compose] |
| リアルタイム | WebSocket。人のクライアントは `/events`、socket型のエージェントは `/agent-socket`。サーバー内の配信はRedis pub/sub。 | [README][repo] |
| ストレージ | S3互換（MinIO）。 | [README][repo] [compose.yml][compose] |
| エージェント連携 | socket（常時接続のWebSocket）とwebhook（HTTP POST）の2方式。トリガーごとに文脈をまとめて送り、エージェントは決められた種類の操作か `HEARTBEAT_OK` を返す。リモートMCP。 | [README][repo] [custom-agents][agents] |
| エージェント実行 | Hermes（`nousresearch/hermes-agent`）、OpenClaw（`alpine/openclaw`）。ホストのDockerソケットをマウントして1ターンごとにコンテナを起動する。READMEはホストのroot相当の権限になると注意している。 | [README][repo] |
| フロントエンド | React 19、Vite、Tailwind 4、TanStack Query・Virtual、Base UI、Zustand、markdown-it、DOMPurify。 | [web package.json][webpkg] [README][repo] |
| 運用 | Docker Compose（Caddy、Postgres、Redis、MinIO、API、worker、web）。Caddyで自動TLS。 | [README][repo] [compose.yml][compose] |

## Pelwayとの比較［提案］

この節は本書の整理と提案であり、CircleChatの資料が述べている内容ではない。Pelway側は[ビジョン](../vision.md)、[PRD](../prd.md)、[技術](../technical.md)の構想で、実装済みの機能ではない。

| 観点 | CircleChat | Pelway（構想） | Pelwayでの扱い |
|---|---|---|---|
| 人とAIの同席 | 確認。AIは役割名付きで人と同じ形で並ぶ | 中核の方針 | 同席だけでは差別化にならない |
| 外部操作前の承認 | エージェントが `request_approval` を出す方式と、scope外の操作を承認カードにする方式 | 承認を実行内容に結び付ける | エージェントが申告しない外部操作をどう止めるかを比較の観点にする。承認した中身を固定し、変わったら再承認にする点は資料で確認できない |
| 承認カードの中身 | 対象、変更の範囲、rollbackの準備を表示 | 送信内容の確認画面（直す／承認して送る） | 「戻せるか」をカードに出す点は参考になる |
| 判断待ちの一覧 | Needs youに承認、review、止まった目標、予算などを集約 | 「今日」画面に誰の判断か・何を決めるかをまとめる | 最も近い機能。誰の判断かの表示と、引き継ぎ時の付け替えで差を出せるかを確かめる |
| 失敗時の扱い | 検証ゲートは既定で通す（fail open）、設定で止められる | 失敗時は止める（fail closed）方針 | 外に出る操作では止める方針を保つ |
| 引き継ぎ・再開 | タスク割り当て、実行の取り消し・steer。引き継ぎ専用の欄は未確認 | MVPの受け入れ条件の中心 | 受け渡しを状態として持てば差を出せる |
| 実行環境 | 1ターンごとの短命コンテナ。共有 `/workspace` は任意 | AIごとのVM相当を検討 | 作業の継続性と隔離を比較の観点にする |
| 料金 | workspace単位、エージェント数で段階 | 人の参加費とAI実行枠を分ける案 | AIの人数で段階を付ける方式を比較に使う |

## 未確認事項

- エージェントが返せる操作の範囲。READMEは11種類の許可リストを挙げ、`custom-agents.md` はプラットフォームが実行する組み込みの操作を5種類と書いている。
- マネージド版の保存地域、SLA、利用規約上の責任範囲。
- 承認した中身が変わったときの再承認の扱い。
- 日本語対応。
- 利用者数、導入実績。

## 出典

- [CircleChat公式](https://circlechat.co/)
- [CircleChat料金](https://circlechat.co/pricing/)
- [tashfeenahmed/circlechat README](https://github.com/tashfeenahmed/circlechat)
- [tashfeenahmed/circlechat docs/custom-agents.md](https://github.com/tashfeenahmed/circlechat/blob/main/docs/custom-agents.md)
- [tashfeenahmed/circlechat docs/CONFIG.md](https://github.com/tashfeenahmed/circlechat/blob/main/docs/CONFIG.md)
- [tashfeenahmed/circlechat docs/p1-platform.md](https://github.com/tashfeenahmed/circlechat/blob/main/docs/p1-platform.md)
- [tashfeenahmed/circlechat api/package.json](https://github.com/tashfeenahmed/circlechat/blob/main/api/package.json)
- [tashfeenahmed/circlechat web/package.json](https://github.com/tashfeenahmed/circlechat/blob/main/web/package.json)
- [tashfeenahmed/circlechat compose.yml](https://github.com/tashfeenahmed/circlechat/blob/main/compose.yml)
- [tashfeenahmed/circlechat スクリーンショット（circlechat-chat）](https://github.com/tashfeenahmed/circlechat/blob/main/docs/screenshots/circlechat-chat.png)

[site]: https://circlechat.co/
[price]: https://circlechat.co/pricing/
[repo]: https://github.com/tashfeenahmed/circlechat
[agents]: https://github.com/tashfeenahmed/circlechat/blob/main/docs/custom-agents.md
[config]: https://github.com/tashfeenahmed/circlechat/blob/main/docs/CONFIG.md
[p1]: https://github.com/tashfeenahmed/circlechat/blob/main/docs/p1-platform.md
[apipkg]: https://github.com/tashfeenahmed/circlechat/blob/main/api/package.json
[webpkg]: https://github.com/tashfeenahmed/circlechat/blob/main/web/package.json
[compose]: https://github.com/tashfeenahmed/circlechat/blob/main/compose.yml
[chat-img]: https://github.com/tashfeenahmed/circlechat/blob/main/docs/screenshots/circlechat-chat.png
