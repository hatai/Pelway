# Elsewhere（elsewhere.so）

確認日：2026-11-11（JST、日本時間）

一覧と比較の前提は[競合調査](../competitors.md)を参照する。「確認」は公式資料・公開リポジトリに記載があるという意味で、実機検証済みではない。「未確認」は今回の資料で裏付けられなかった項目で、機能がないという意味ではない。

本書は指定URL（elsewhere.so）で提供されている現行製品を記載する。この製品はチームの稼働量計画（capacity planning）のツールで、人と複数のAIが協業するチャットではない。調査で意図していたサービスが同名の別製品である可能性があり、本書では同名の別サービスの情報を混ぜていない。

## 概要

| 項目 | 内容 |
|---|---|
| 提供元 | 運営法人名は未確認。サイトの表記は「Elsewhere」のみで、プライバシーポリシーにも法人名は見当たらない。創業者として Matthew Zammit が名前を出している。[公式][top] [Privacy Policy][privacy] [Story][story] |
| 公式URL | [elsewhere.so][top] |
| 製品の説明 | 小規模で分散した製品・開発チーム向けに、誰が稼働できるかと、現実に何を届けられるかを見せる計画ツール。見出しは「Plan with your team's real capacity, not guesses」。HRシステムとJira・Linear・Notionなどの作業管理ツールの間に置く計画の層で、HRシステムではないと説明する。[公式][top] [Story][story] |
| 提供段階 | Changelogに「Pre-launch」「Alpha」の表記。トップの行動喚起は「Join the waitlist」で、FAQは「still in its early stages」「bringing people in gradually」と説明する。[Changelog][changelog] [公式][top] |
| ライセンス | 公開リポジトリは見つからなかった。ライセンスは未確認。 |
| 価格 | 公式サイトに料金の記載は見当たらない。未確認。[公式][top] |

## 機能

| 観点 | 内容 | 出典 |
|---|---|---|
| 複数人＋複数AIの同一チャンネル参加 | 未確認。チャット機能の説明は見当たらない。 | [公式][top] |
| AIの役割・専門性の設定 | 未確認。エージェントの説明は見当たらない。 | [公式][top] |
| AIの扱い | トップは「AI-native」として「Get early warnings when deadlines are at risk.」と書く。Planning Intelligence は、メンバーの過負荷や期限の遅れを早めに知らせると説明する。Storyページは、AIは「Over time」背後で手助けするようになると書き、容量変化のパターン検出、リスクの早期警告、作業の配分案を挙げる。 | [公式][top] [Story][story] |
| 実行能力 | 未確認。AIが作業を実行する説明は見当たらない。 | [公式][top] |
| 計画・稼働 | Capacity Planning、Time off management（休暇・病欠の管理）、Availability Forecasting、Team Visibility（所在地、タイムゾーン、重なる勤務時間）。週の作業から四半期の目標まで計画する。 | [公式][top] |
| 承認・権限管理 | 休暇申請を承認・却下する機能があり、通知のタイトルに操作した人の名前が出る。これは人どうしの休暇承認で、AIの操作を承認する機能は未確認。管理者がプロフィールから上長を設定できる。 | [Changelog][changelog] |
| 通知 | プロジェクトリーダーは、メンバーが休暇を申請・承認されたときに通知を受け、担当プロジェクトに影響する休暇のメール通知を選べる。今後20営業日の人手不足をダッシュボードで警告し、休暇が重なりすぎると知らせる。 | [Changelog][changelog] |
| 外部連携 | Linear、Jira、Slackと同期し、タスクを取り込み、更新を送ると説明する。MCPやAPIの説明は見当たらない。 | [公式][top] |
| 監査 | 未確認。 | |

## UI/UX

| 要素 | 内容 | 出典 |
|---|---|---|
| 製品画面 | トップに製品のスクリーンショットは見当たらない。 | [公式][top] |
| 休暇の申請 | 2列の申請フォームで、休暇の種類をその場で切り替え、メモ欄を折りたためる。休暇の種類は色分けのバッジで表示する。 | [Changelog][changelog] |
| 一覧・カレンダー | 申請の行に上長とアバターを表示する。休暇カレンダーには影響する日付へのリンク付きバナーが出る。ダッシュボードとカレンダーはプロジェクトとチームで絞り込める。 | [Changelog][changelog] |
| 初期設定 | ワークスペースの設定ウィザードで、選んだ国の祝日を登録する。招待されたメンバー向けの案内があり、設定が終わるまで利用を止める。 | [Changelog][changelog] |
| 表示 | ダークモードと、そのモノクロ版がある。 | [Changelog][changelog] |

## 技術

`gh search repos` と `gh search code` で「elsewhere.so」と製品名を検索したが、この製品の公開リポジトリは見つからなかった。公式資料に技術構成の記載は見当たらない。

| 層 | 技術 | 出典 |
|---|---|---|
| サーバー言語・フレームワーク | 未確認 | |
| DB | 未確認 | |
| AIの実装 | 未確認 | |
| フロントエンド | 未確認 | |

## Pelwayとの比較［提案］

この節は本書の整理と提案であり、Elsewhereの資料が述べている内容ではない。Pelway側は[ビジョン](../vision.md)、[PRD](../prd.md)、[技術](../technical.md)の構想で、実装済みの機能ではない。

| 観点 | Elsewhere | Pelway（構想） | Pelwayでの扱い |
|---|---|---|---|
| 製品の領域 | 稼働量の計画と休暇管理 | 人とAIが同じチャンネルで協業するチャット | 直接の競合ではない。指定URLが意図したサービスかを先に確かめる |
| 人とAIの同席 | 未確認 | 中核の方針 | 比較の対象外 |
| 承認 | 人どうしの休暇承認 | AIの外部操作を承認し、中身を固定する | 承認の対象が異なるため比較しない |
| 誰が動けるかの把握 | 休暇とタイムゾーンから稼働を見せる | 担当交代と引き継ぎを扱う | 担当者が不在のときに判断待ちを誰へ回すかを考える材料にする |
| 外部連携 | Linear、Jira、Slackと同期 | 外部ツール連携を検討 | 既存の作業管理ツールと併用させる位置付けの例として扱う |

## 未確認事項

- 調査で意図していたサービスが elsewhere.so の現行製品か、同名の別サービスか。
- 運営法人名、所在地。
- 料金、契約条件、提供開始時期。
- AI機能の実装状況（トップの「AI-native」とStoryの「Over time」の関係）。
- Linear、Jira、Slack連携の範囲、技術構成、日本語対応、保存地域。

## 出典

- [Elsewhere公式トップ](https://elsewhere.so/)
- [Elsewhere：Story](https://elsewhere.so/story)
- [Elsewhere：Changelog](https://elsewhere.so/changelog)
- [Elsewhere：Privacy Policy](https://elsewhere.so/privacy-policy)

[top]: https://elsewhere.so/
[story]: https://elsewhere.so/story
[changelog]: https://elsewhere.so/changelog
[privacy]: https://elsewhere.so/privacy-policy
