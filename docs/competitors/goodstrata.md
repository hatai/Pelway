# GoodStrata（Good Strata Pty Ltd）

確認日：2026-11-11（JST、日本時間）

一覧と比較の前提は[競合調査](../competitors.md)を参照する。「確認」は公式資料・公開リポジトリに記載があるという意味で、実機検証済みではない。「未確認」は今回の資料で裏付けられなかった項目で、機能がないという意味ではない。

## 概要

| 項目 | 内容 |
|---|---|
| 提供元 | Good Strata Pty Ltd（ABN 55 684 135 760）。オーストラリアの企業。[公式][top] |
| 公式URL | [goodstrata.com.au][top]、ソースは[goodstrata/goodstrata][repo]。公式トップからデモ環境（demo.goodstrata.com.au）へリンクしている。[公式][top] |
| 製品の説明 | 区分所有建物の管理組合（owners corporation、strata）の管理業務をAIエージェントが実行し、人は承認・投票・差し戻しだけを行うプラットフォーム。まずオーストラリア・ビクトリア州の Owners Corporations Act 2006 向けに作られている。[README][repo] [公式][top] |
| 提供段階 | 公式サイトにbetaなどの段階表記は見当たらない。READMEは「Working today」と「Next」で実装済みと予定を分けている。[README][repo] [Open source][oss] |
| ライセンス | Apache-2.0。公式サイトは「open core」ではなくコード全体が対象と説明する。[Open source][oss] [README][repo] |
| 価格 | 公式表示は「Free for your strata, and we never touch your money.」。管理組合から料金を取らず（per-lot fee、subscription、setup fee、exit feeなし）、管理費の入金も GoodStrata を経由しないと説明する。将来の「paid managed service for owners corporations that want us to run it, priced up front」を予告しているが、金額は未確認。セルフホストはApache-2.0で無料。[公式][top] [Open source][oss] |

## 機能

| 観点 | 内容 | 出典 |
|---|---|---|
| 複数人＋複数AIの同一チャンネル参加 | 未確認。人どうしのメッセージ機能はあるが、ログインした会員に限定し、agent・system のactorを拒む実装になっている。エージェントの作業はチャットではなく、Agents・Decisions・Activity の各画面に出る。 | [messaging.ts][msg] [README][repo] |
| AIの役割・専門性の設定 | 業務ごとのエージェント（finance、maintenance、meetings）がイベントを購読して動く。ほかに会議を進行する chair、動作確認用の echo の定義がある。利用者が汎用の役割を作る機能は未確認。 | [README][repo] [chair.ts][chair] [echo.ts][echo] [finance.ts][finance] |
| 実行能力 | 管理費の請求、入金照合、滞納の督促、修繕依頼の振り分けと業者手配、会議の招集通知、議事録の下書き。滞納督促の段階と時期はコードが決め、エージェントは文面を書く。議事録の下書きは役員の承認まで委員会内だけに置く。 | [README][repo] [finance.ts][finance] [meetings.ts][meetings] |
| 金額計算の分離 | 管理費の按分、遅延利息、入金照合、議決権の重み付けはLLMではなく決定論的なコードで行い、網羅的なテストを持つと説明する。金額は整数のセントで扱う。お金に関するメールには、コードが作る明細を付ける。 | [README][repo] [How it works][how] |
| 承認・権限管理 | お金を使う、建物として約束する、規則の例外を作る操作は decisions inbox に入り、人が決める。決定を解決できるのはログインした利用者だけで、エージェントやシステムは解決できない。決定ごとに決める人の区分（treasurer、committee、all_owners）があり、treasurer は1票で確定、committee と all_owners は有資格者の過半数で確定する。承認時に実行する処理（登録済みのアクション名と引数）は決定の作成時に保存され、承認後にコードの実行器がその処理だけを行う。修繕は建物ごとのしきい値以下なら自動で手配し、超えると委員会の承認を待つ。緊急手配は事後に委員会へ確認を出す。 | [How it works][how] [decisions.ts][dsvc] [maintenance.ts][maint] |
| 判断の期限 | 決定に期限（dueAt）を持たせられる（修繕の承認では7日）。期限切れ時のエスカレーションの仕組みはREADMEで「Next」に挙がっている。 | [maintenance.ts][maint] [README][repo] |
| 外部連携 | MCPサーバーを持つ。お金や法定手続きに関わるツール（管理費請求の発行、会議招集通知、決定の解決、投票、閉会）は `mcp:govern` スコープが必要で、確認トークンなしの呼び出しは状態を変えない試算（dry-run）と署名付きの確認トークンを返す。同じ利用者が同じツールを同じ引数で10分以内にトークン付きで呼んだときだけ実行する。トークンは使い捨てではなく、再送はドメイン側の重複チェック（ALREADY_ISSUED、ALREADY_RESOLVED など）で止める。決済はMonoovaのNPP/PayIDを管理サービスで使うとREADMEは説明するが、Monoovaの本番ドライバーは「Next」に挙がっている。メール、SMS、ストレージはオフラインの既定値を持つ。 | [governed.ts][mcp] [confirm.ts][confirm] [README][repo] [Open source][oss] |
| 監査 | すべての操作は追記専用のイベントログに残り、行為者（人、名前付きエージェント、システム）と、原因になったイベントへのリンクを持つ。エージェントの実行は、モデルのツール呼び出しまで記録する。 | [How it works][how] [README][repo] |
| ローカルモデル | `AI_PROVIDER=local` で Ollama のモデル（例：Qwen3-30B-A3B）を使え、APIキーなしで外部にデータを送らないと説明する。Anthropic のモデルも選べる。 | [公式][top] [Open source][oss] [README][repo] |

## UI/UX

| 要素 | 内容 | 出典 |
|---|---|---|
| 画面構成 | 建物（scheme）ごとに Finance、Decisions、Agents、Activity などのタブを持つWebアプリ（PWA）。Maintenance、Meetings などのタブのコンポーネントもある。iOS・Android向けのアプリ（Expo、Capacitor）もある。 | [README][repo] [web components][webcomp] [native package.json][nativepkg] [mobile package.json][mobilepkg] |
| decisions inbox（説明） | 公式サイトは、各項目に「the figures, the quote, the photo, the relevant clause, the reason」が付き、スマホで開けると説明する。操作は Approve（「the exact action shown on screen, run by code, not the model」）、Vote、Escalate（追加情報、相見積もり、委員会の会議へ戻す）の3つを挙げる。 | [How it works][how] |
| decisions inbox（Web実装） | 画面見出しは「Decisions」、説明文は「Choices the agents have prepared and put to the people who hold the authority.」。未決の決定は期限切れ、期限が近い順、古い順に並び、自分が決められるものがあると見出しが「Waiting on you」になる。各カードは左端の色帯（期限切れは警告色）、タイトル、「for the committee · respond by 日付」、種類のバッジ、Markdownの要約で構成する。その下に任意の「Note for the record」欄と選択肢のボタン（既定は Approve と Decline）が並ぶ。5秒ごとに再取得する。 | [DecisionsTab.tsx][dtab] |
| 根拠の表示（Web実装） | 決定はデータとして `evidence` を持つが、Webのカードが描画するのは要約（`summaryMd`）で、`evidence` を表示するコードはこのコンポーネント内にない。修繕承認の要約は Request、Contractor、Estimated cost、Scope の箇条書きで、相見積もりのしきい値を超えると推奨の注記が付く。ネイティブアプリは `evidence` から金額を取り出して表示に使う。 | [DecisionsTab.tsx][dtab] [maintenance.ts][maint] [native decisions][ndec] |
| 合議の投票 | committee と all_owners の決定では、賛成数・反対数のバッジ、「N needed · M eligible」、過半数までの進捗バー、投票者ごとの選択とメモを表示する。投票後は「You voted for — waiting on the other eligible voters.」と出る。投票状況を取得できないときは、二重投票を避けるためボタンを出さずエラーを表示する。決める権限がない人には「This decision is with the …」と表示し、集計だけを見せる。 | [DecisionsTab.tsx][dtab] |
| 選択肢の言い換え | 緊急手配の事後確認では、選択肢の表示名を Acknowledge と Flag for discussion に変える。 | [maintenance.ts][maint] |
| 決定の履歴 | 「History」に解決済みの決定を1行ずつ並べ、展開すると選んだ選択肢、決めた人、日時、メモ、要約が出る。未決がないときは「Nothing to decide」と表示する。 | [DecisionsTab.tsx][dtab] |
| エージェントの作業 | Agentsタブの説明は「Every action an agent takes is a recorded run with a full tool-call transcript.」。実行ごとに状態とツール呼び出しの記録を表示し、失敗したツールにはバッジが付く。 | [AgentsTab.tsx][atab] [README][repo] |

## 技術

公開リポジトリの依存定義、README、ソースから確認した。

| 層 | 技術 | 出典 |
|---|---|---|
| 言語・構成 | TypeScript。pnpm workspace と Turborepo のモノレポ。Node 22以上。 | [package.json][rootpkg] [README][repo] |
| API | Hono、better-auth、zod。API、イベントの配信器、エージェントのワーカーを1つのNodeプロセスで動かす。SSEで配信する。 | [api package.json][apipkg] [README][repo] |
| DB | PostgreSQL（READMEのcomposeは18）、Drizzle ORM（約48テーブル）、版管理したSQLマイグレーション。 | [README][repo] [db package.json][dbpkg] |
| キュー・配信 | pg-boss。Postgres の `LISTEN/NOTIFY` で起こし、カーソルによる取りこぼし走査で保証する。Kafka や Redis は使わない。 | [README][repo] [Open source][oss] |
| イベントログ | `event_log` テーブルに stream、type、payload、actor、correlationId、causationId、causationDepth、dedupeKey、連番の seq を持つ。ドメインの書き込みとイベントは同じトランザクションで確定する。DBトリガーで UPDATE と DELETE を拒む。 | [spine.ts][spine] [event_log trigger][trig] [README][repo] |
| エージェント | Vercel AI SDK（`ai`）、`@ai-sdk/anthropic`、`@ai-sdk/openai-compatible`。実行は（起点イベント、エージェント、試行回数）で識別し、ツールの出力に重複排除キーを付ける。ツール呼び出しの記録（`agent_runs.steps`）は1ステップごとに保存する。エージェントのチェーンには深さの上限がある。 | [agents package.json][agentspkg] [spine.ts][spine] [README][repo] |
| 金額・法定ルール | `packages/core` に決定論的なエンジン（管理費、利息、照合、滞納の段階、投票）を置き、APIとエージェントのツールが同じ関数を呼ぶ。投票は普通決議を1区画1票、特別決議を全持分の75%以上などと法の条文に沿って実装する。テストはモックのモデルで実際のツールループと実DBを動かし、CIでAPIを呼ばない。 | [README][repo] [voting.ts][vote] |
| MCP | `@modelcontextprotocol/sdk`。リクエストごとに状態を持たないサーバーを作る。確認トークンは HMAC-SHA256 で、ツール名、引数のハッシュ、利用者ID、期限を署名する。 | [api package.json][apipkg] [server.ts][mcpserver] [confirm.ts][confirm] |
| フロントエンド | React、Vite、TanStack Router・Query・Form、Tailwind CSS 4、Radix UI、Tiptap、sonner。 | [web package.json][webpkg] |
| モバイル | Expo（React Native）と Capacitor。 | [native package.json][nativepkg] [mobile package.json][mobilepkg] |
| 外部連携の実装 | AWS SDK（S3、SES v2）、nodemailer、pdfkit。 | [integrations package.json][intpkg] |
| ビルド・テスト | Biome、Vitest、Playwright、testcontainers、Docker Compose。 | [package.json][rootpkg] [README][repo] |

## Pelwayとの比較［提案］

この節は本書の整理と提案であり、GoodStrataの資料が述べている内容ではない。Pelway側は[ビジョン](../vision.md)、[PRD](../prd.md)、[技術](../technical.md)の構想で、実装済みの機能ではない。

| 観点 | GoodStrata | Pelway（構想） | Pelwayでの扱い |
|---|---|---|---|
| 人とAIの同席 | 同じチャットに参加する形は未確認。エージェントは画面とイベントで働く | 中核の方針 | 業種特化の業務製品であり、チャットとしての直接の競合ではない |
| 承認する中身の固定 | 実行する処理と引数を決定の作成時に保存し、承認後にコードが実行する | 承認した本文・宛先・添付を固定し、変われば再承認 | 「承認＝保存済みの処理を実行」という形は参考になる。中身が変わったときの再承認は比較の観点にする |
| 試算してから確定 | MCPの重要操作は試算と署名付きトークンの2段階。同じ人・同じ引数・10分以内に限る。トークンは使い捨てではなく、再送はドメイン側で止める | 送信前に宛先と内容を確認する。1つの承認を別操作に使い回さない | 外部操作の前に「何が起きるか」を見せる手順の参考にする。承認の使い捨てをどこで保証するかを比較の観点にする |
| 判断待ちの表示 | 決める人の区分、期限、自分の番かどうかをカードで表示 | 「今日」画面で誰の何の判断かを表示 | 期限順の並べ方と「Waiting on you」の見出しを参考にする |
| 根拠の提示 | 公式は根拠付きと説明。Webのカードは要約のMarkdownを表示 | 判断待ちは誰の判断か・何を決めるかを持つ。引き継ぎ欄は決定・送信内容・残作業 | 判断待ちに根拠を添えるか、構造化データか文章かを決める材料にする |
| 合議 | 過半数の投票と進捗バー | 複数の承認者は未決事項 | 2人以上の承認を設計するときの参考にする |
| 判断の期限切れ | 期限は持つが、エスカレーションは「Next」 | 期限切れで送信せず知らせる | 同じ課題が残っている領域として扱う |
| AIと計算の分離 | 金額と法定ルールはコード、AIは文面 | 外部ツールの呼び出しは実行環境の外のツール中継を通す | 外部操作の可否や宛先の確定をコードで判定する方針の参考にする |
| 監査ログ | 追記専用をDBトリガーで強制。原因イベントへの連鎖を持つ | 誰が・いつ・何を・誰の承認で実行したかを追記のみのログに残す。追記専用だけで管理者の改変まで防げるとは主張しない | 原因の連鎖と重複排除キーの持ち方を参考にする |

## 未確認事項

- 有料の managed service の金額と契約条件。
- 管理サービスでの決済の流れ（READMEのMonoova利用と、公式サイトの「入金は GoodStrata を経由しない」との関係）。
- ホスト版とデモが公開リポジトリの `main` と同じコードか。リポジトリ内の公式サイト用ソース（`site/`）には「Free for your strata. We earn only on payments that move.」という公開中のサイトと異なる文言があり、本書は公開中のサイトの表示を採った。[site/][site]
- 公式サイトの Escalate と entitlement-weighted な投票が、decisions inbox のどの画面に実装されているか。Webの実装で確認できた選択肢は Approve と Decline（表示名の言い換えを含む）。
- 修繕の自動承認しきい値の既定値。
- 利用者数、導入実績、日本語対応、保存地域、SLA。

## 出典

- [GoodStrata公式トップ](https://goodstrata.com.au/)
- [GoodStrata：How it works](https://goodstrata.com.au/how-it-works/)
- [GoodStrata：Open source](https://goodstrata.com.au/open-source/)
- [goodstrata/goodstrata README](https://github.com/goodstrata/goodstrata)
- [goodstrata/goodstrata package.json](https://github.com/goodstrata/goodstrata/blob/main/package.json)
- [goodstrata/goodstrata apps/api/package.json](https://github.com/goodstrata/goodstrata/blob/main/apps/api/package.json)
- [goodstrata/goodstrata apps/web/package.json](https://github.com/goodstrata/goodstrata/blob/main/apps/web/package.json)
- [goodstrata/goodstrata apps/native/package.json](https://github.com/goodstrata/goodstrata/blob/main/apps/native/package.json)
- [goodstrata/goodstrata apps/mobile/package.json](https://github.com/goodstrata/goodstrata/blob/main/apps/mobile/package.json)
- [goodstrata/goodstrata packages/agents/package.json](https://github.com/goodstrata/goodstrata/blob/main/packages/agents/package.json)
- [goodstrata/goodstrata packages/db/package.json](https://github.com/goodstrata/goodstrata/blob/main/packages/db/package.json)
- [goodstrata/goodstrata packages/integrations/package.json](https://github.com/goodstrata/goodstrata/blob/main/packages/integrations/package.json)
- [goodstrata/goodstrata apps/web/src/components/DecisionsTab.tsx](https://github.com/goodstrata/goodstrata/blob/main/apps/web/src/components/DecisionsTab.tsx)
- [goodstrata/goodstrata apps/web/src/components](https://github.com/goodstrata/goodstrata/tree/main/apps/web/src/components)
- [goodstrata/goodstrata site/](https://github.com/goodstrata/goodstrata/tree/main/site)
- [goodstrata/goodstrata apps/web/src/components/AgentsTab.tsx](https://github.com/goodstrata/goodstrata/blob/main/apps/web/src/components/AgentsTab.tsx)
- [goodstrata/goodstrata apps/native/app/scheme/[id]/decisions.tsx](https://github.com/goodstrata/goodstrata/blob/main/apps/native/app/scheme/%5Bid%5D/decisions.tsx)
- [goodstrata/goodstrata packages/core/src/services/decisions.ts](https://github.com/goodstrata/goodstrata/blob/main/packages/core/src/services/decisions.ts)
- [goodstrata/goodstrata packages/core/src/services/maintenance.ts](https://github.com/goodstrata/goodstrata/blob/main/packages/core/src/services/maintenance.ts)
- [goodstrata/goodstrata packages/core/src/services/messaging.ts](https://github.com/goodstrata/goodstrata/blob/main/packages/core/src/services/messaging.ts)
- [goodstrata/goodstrata packages/core/src/engines/voting.ts](https://github.com/goodstrata/goodstrata/blob/main/packages/core/src/engines/voting.ts)
- [goodstrata/goodstrata packages/db/src/schema/spine.ts](https://github.com/goodstrata/goodstrata/blob/main/packages/db/src/schema/spine.ts)
- [goodstrata/goodstrata packages/db/migrations/0001_event_log_triggers.sql](https://github.com/goodstrata/goodstrata/blob/main/packages/db/migrations/0001_event_log_triggers.sql)
- [goodstrata/goodstrata packages/agents/src/agents/finance.ts](https://github.com/goodstrata/goodstrata/blob/main/packages/agents/src/agents/finance.ts)
- [goodstrata/goodstrata packages/agents/src/agents/meetings.ts](https://github.com/goodstrata/goodstrata/blob/main/packages/agents/src/agents/meetings.ts)
- [goodstrata/goodstrata packages/agents/src/agents/chair.ts](https://github.com/goodstrata/goodstrata/blob/main/packages/agents/src/agents/chair.ts)
- [goodstrata/goodstrata apps/api/src/mcp/tools/governed.ts](https://github.com/goodstrata/goodstrata/blob/main/apps/api/src/mcp/tools/governed.ts)
- [goodstrata/goodstrata packages/agents/src/agents/echo.ts](https://github.com/goodstrata/goodstrata/blob/main/packages/agents/src/agents/echo.ts)
- [goodstrata/goodstrata apps/api/src/mcp/server.ts](https://github.com/goodstrata/goodstrata/blob/main/apps/api/src/mcp/server.ts)
- [goodstrata/goodstrata apps/api/src/mcp/confirm.ts](https://github.com/goodstrata/goodstrata/blob/main/apps/api/src/mcp/confirm.ts)

[top]: https://goodstrata.com.au/
[how]: https://goodstrata.com.au/how-it-works/
[oss]: https://goodstrata.com.au/open-source/
[repo]: https://github.com/goodstrata/goodstrata
[rootpkg]: https://github.com/goodstrata/goodstrata/blob/main/package.json
[apipkg]: https://github.com/goodstrata/goodstrata/blob/main/apps/api/package.json
[webpkg]: https://github.com/goodstrata/goodstrata/blob/main/apps/web/package.json
[nativepkg]: https://github.com/goodstrata/goodstrata/blob/main/apps/native/package.json
[mobilepkg]: https://github.com/goodstrata/goodstrata/blob/main/apps/mobile/package.json
[agentspkg]: https://github.com/goodstrata/goodstrata/blob/main/packages/agents/package.json
[dbpkg]: https://github.com/goodstrata/goodstrata/blob/main/packages/db/package.json
[intpkg]: https://github.com/goodstrata/goodstrata/blob/main/packages/integrations/package.json
[dtab]: https://github.com/goodstrata/goodstrata/blob/main/apps/web/src/components/DecisionsTab.tsx
[atab]: https://github.com/goodstrata/goodstrata/blob/main/apps/web/src/components/AgentsTab.tsx
[ndec]: https://github.com/goodstrata/goodstrata/blob/main/apps/native/app/scheme/%5Bid%5D/decisions.tsx
[dsvc]: https://github.com/goodstrata/goodstrata/blob/main/packages/core/src/services/decisions.ts
[maint]: https://github.com/goodstrata/goodstrata/blob/main/packages/core/src/services/maintenance.ts
[msg]: https://github.com/goodstrata/goodstrata/blob/main/packages/core/src/services/messaging.ts
[vote]: https://github.com/goodstrata/goodstrata/blob/main/packages/core/src/engines/voting.ts
[spine]: https://github.com/goodstrata/goodstrata/blob/main/packages/db/src/schema/spine.ts
[trig]: https://github.com/goodstrata/goodstrata/blob/main/packages/db/migrations/0001_event_log_triggers.sql
[finance]: https://github.com/goodstrata/goodstrata/blob/main/packages/agents/src/agents/finance.ts
[meetings]: https://github.com/goodstrata/goodstrata/blob/main/packages/agents/src/agents/meetings.ts
[chair]: https://github.com/goodstrata/goodstrata/blob/main/packages/agents/src/agents/chair.ts
[mcp]: https://github.com/goodstrata/goodstrata/blob/main/apps/api/src/mcp/tools/governed.ts
[confirm]: https://github.com/goodstrata/goodstrata/blob/main/apps/api/src/mcp/confirm.ts
[echo]: https://github.com/goodstrata/goodstrata/blob/main/packages/agents/src/agents/echo.ts
[mcpserver]: https://github.com/goodstrata/goodstrata/blob/main/apps/api/src/mcp/server.ts
[webcomp]: https://github.com/goodstrata/goodstrata/tree/main/apps/web/src/components
[site]: https://github.com/goodstrata/goodstrata/tree/main/site
