# Cosmo（Perséides）

確認日：2026-11-11（JST、日本時間）

一覧と比較の前提は[競合調査](../competitors.md)を参照する。「確認」は公式資料・公開リポジトリに記載があるという意味で、実機検証済みではない。「未確認」は今回の資料で裏付けられなかった項目で、機能がないという意味ではない。

## 概要

| 項目 | 内容 |
|---|---|
| 提供元 | Perséides。カナダ・ケベック州の独立した製品会社で、共同創業者はMathieu DurocherとJean-Daniel Tanguay。法人名は未確認。Cosmoのほか、SwitchboardとNaganoを作っている。[Team][team] [Cosmo][cosmo] |
| 公式URL | [Cosmo][cosmo]、[Perséides blog][blog] |
| 製品の説明 | AIのagentが同僚として働くチームチャットのworkspace。public／private channel、DM、thread、inbox、音声・ビデオ通話を持つ。[Cosmo][cosmo] |
| 提供段階 | 製品ページは招待制のprivate betaとし、early accessはcontactページから申し込む。設計記事はstagingのみで招待したtester向け、一般公開と本番環境はまだないと書いている。[Cosmo][cosmo] [設計記事][members] |
| ライセンス | 未確認。公開リポジトリは見つからなかった。 |
| 価格 | 未確認。製品ページに料金の記載はない。[Cosmo][cosmo] |

## 機能

| 観点 | 内容 | 出典 |
|---|---|---|
| 複数人＋複数AIの同一チャンネル参加 | agentはchannelに参加してメンションに答え、DMにも答える。threadでは他の人と同じthreadに作業を投稿する。短い質問と長い作業が同じchannelに同居できる。 | [Cosmo][cosmo] |
| agentの定義 | "An agent in Cosmo is a user record with a model, a system prompt and a set of tools."。`users` の行を `user_type = agent` とし、system prompt、model、triggerを `agent_config` に持つ。 | [設計記事][members] |
| AIの役割・専門性の設定 | agentごとにsystem prompt、model、toolsを設定する。職務テンプレートや専門性の設定画面は未確認。 | [設計記事][members] |
| 応答の条件 | メンション（表示名を含む）、DM、agentが始めたか既に参加したthreadで応答する。任意で追いかけの時間枠（既定120秒、間に3件まで）と、channelの監視（既定はオフ）を設定できる。runnerは後から作られたものを含むpublic channelを購読する。 | [設計記事][members] |
| 実行能力 | 定期taskを会話の一文から設定できる。コードはsandboxで動き、previewを `*.preview.<env>.cosmo.work` に出す。足りないcommand、panel、tabを説明するとagentがチーム全員で使えるpluginを作る。 | [Cosmo][cosmo] [設計記事][members] |
| 承認・権限管理 | agentは人と同じ権限に縛られ、操作の送信時に権限を再確認する。操作ごとの実行前承認は未確認。 | [Cosmo][cosmo] [設計記事][members] |
| 返答内の操作 | 返答内のform、選択肢、actionにその場で答えられ、actionはサーバー側で使い捨ての不透明なIDから実行する。 | [設計記事][members] |
| 完了の確認 | 完了した作業を、agentの文章ではなくtoolの記録（ledger）と照合する。依頼の振り分けはキーワードではなくモデルが判断する。 | [設計記事][members] |
| 記憶 | 会話をまたいで記憶し、channelの履歴を知っている。記憶はworkspace単位で共有。 | [Cosmo][cosmo] [設計記事][members] |
| 外部連携 | 付与したintegrationsを使う。具体的な連携先は製品ページに記載がない。管理者が割り当てたMCP serverをstdioまたはSSEで使う。 | [Cosmo][cosmo] [設計記事][members] |
| 監査 | 全操作をworkspaceのlogに記録する。agentの操作は人と同じ監査・利用量のlogに入る。 | [Cosmo][cosmo] [設計記事][members] |
| 初期設定 | 新しいworkspaceはCosmo agentとの会話で設定し、agentが最初のchannelを作り、招待を送る。 | [Cosmo][cosmo] |

## UI/UX

| 要素 | 内容 | 出典 |
|---|---|---|
| メンバー欄 | agentはメンバー欄に明示的なラベル付きで並ぶ。設計記事では"AI"のchipと書いている。agentはメンションできるusername、表示名、アバターを持つ。 | [Cosmo][cosmo] [設計記事][members] |
| 作業中の表示 | threadに作業を投稿し、作業中はlive status lineを出す。 | [Cosmo][cosmo] |
| 構造化した返答 | 作業のチェックリストを実行中に更新し、status、key-value、table、card、linkのブロックで返す。form、選択肢、actionにはその場で答える。 | [設計記事][members] |
| plugin作成 | 足りないcommand、panel、tabを言葉で説明すると、agentがpluginを作り、チーム全員で使える。 | [Cosmo][cosmo] |
| クライアント | 製品ページはデスクトップアプリ、モバイル、Webを挙げる。設計記事はWeb、Macアプリ、GPUIのデスクトップクライアントを挙げる。 | [Cosmo][cosmo] [設計記事][members] |
| サインイン | パスワードなし（magic linkまたは6桁のコード）、任意でTOTPの2要素認証。 | [設計記事][members] |
| 画面構成・agent作成の手順・承認画面 | 未確認。 | [Cosmo][cosmo] |

## 技術

公式の設計記事から確認した。公開リポジトリは見つからず、サーバーの実装言語は未確認。

| 層 | 技術 | 出典 |
|---|---|---|
| ホスティング | サーバーで運用し、メッセージはgatewayのhost上のPostgresに置く。local-firstではなく、end-to-end暗号化もしていない。 | [設計記事][members] |
| データモデル | agentを `users` の行（`user_type = agent`）と `agent_config` で表す。 | [設計記事][members] |
| モデルprovider | workspaceごとに選ぶ。OpenAI、Anthropic、OpenRouter、Together、Groq、Fireworks、Ollama、LM Studio、shell providerとしてのClaude Code。メッセージ内容は設定したproviderへ送られる。 | [設計記事][members] [Cosmo][cosmo] |
| プロトコル | MCP（stdio、SSE）。toolは `mcp__server__tool` の名前で見える。 | [設計記事][members] |
| 実行の分離 | コードはsandboxで動く。sandboxの方式（container、VMなど）は未確認。 | [設計記事][members] |
| デスクトップ | GPUIのクライアントがある。 | [設計記事][members] |

## Pelwayとの比較［提案］

この節は本書の整理と提案であり、Cosmoの資料が述べている内容ではない。Pelway側は[ビジョン](../vision.md)、[PRD](../prd.md)、[技術](../technical.md)の構想で、実装済みの機能ではない。

| 観点 | Cosmo | Pelway（構想） | Pelwayでの扱い |
|---|---|---|---|
| 人とAIの同席 | 確認。"AI"のchipで明示 | 中核の方針。AIであることを明示 | 同席とラベルだけでは差別化にならない |
| 外部操作前の承認 | 人と同じ権限と再確認。操作ごとの承認は未確認 | 外に出る操作は毎回承認 | 差を出せる候補。ただし未確認であり、欠けるとは扱わない |
| 返答内のaction | form、選択肢、actionを使い捨てIDでサーバー側実行 | 承認を実行内容に結び付ける | 承認ボタンを使い捨てIDで実行する方式を、二重実行の抑止の参考にする |
| 完了の確認 | toolの記録と照合 | 送信結果と状態を記録 | 「AIが完了と言った」ではなく記録で完了を決める考え方を取り入れる |
| 発話の条件 | メンション、DM、参加済みthread、任意の時間枠 | 新規依頼はメンション起点。受付済みの仕事の発話条件は未決 | 未決事項の具体案として参考にする |
| 引き継ぎ・判断待ち・再開 | 専用の機能は未確認 | MVPの中心 | 比較の観点にする |
| 実行環境 | sandbox。方式は未確認 | AI社員ごとの実行環境を検討 | 方式が分かるまで比較しない |
| 提供段階 | 招待制、stagingのみ | 構想 | 一般提供の製品として扱わない |

## 未確認事項

- 法人名、料金、一般提供の時期。
- 操作ごとの承認、承認画面、agent作成の手順。
- sandboxの方式、サーバーの実装言語、保存地域。
- 日本語対応、SLA。

## 出典

- [Perséides：Cosmo](https://www.perseides.ai/cosmo)
- [Perséides：Why agents are members in Cosmo](https://www.perseides.ai/blog/agents-are-members-in-cosmo)
- [Perséides：Blog](https://www.perseides.ai/blog)
- [Perséides：Team](https://www.perseides.ai/team)

[cosmo]: https://www.perseides.ai/cosmo
[members]: https://www.perseides.ai/blog/agents-are-members-in-cosmo
[blog]: https://www.perseides.ai/blog
[team]: https://www.perseides.ai/team
