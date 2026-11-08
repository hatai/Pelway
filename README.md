# Pelway

![PelwayのA案ロゴ](assets/logo/svg/pelway-logo-light.svg)

人間と、役割を持つ複数のAIが、同じチャンネルで仕事を進める独立したビジネスチャットの構想です。AIに仕事を依頼し、人間の判断と外部操作の承認を分け、成果物と残作業を次の担当者へ引き継ぐ体験を目指します。AIであることは明示します。

## このリポジトリの状態

本資料は2026年11月8日（日本時間）時点の構想と設計です。記載した機能が実装済み、提供中、動作検証済みであることを意味しません。確定した方針、提案、仮説、未決事項を分けています。

ブラウザ版を先行し、将来はRust製デスクトップアプリを想定します。社内wikiと外向けQ&Aの自動生成は現在の対象外です。

## 文書

| 文書 | 内容 |
|---|---|
| [ビジョン](docs/vision.md) | 対象顧客の仮説、課題、差別化を検証する方法 |
| [競合調査](docs/competitors.md) | 公式出典による比較、製品同定の留保、名称衝突 |
| [プロダクト要件](docs/prd.md) | 具体ユースケース、状態、承認、MVPと受け入れ条件 |
| [ロードマップ](docs/roadmap.md) | 顧客検証、実装、障害試験、限定試用の順序 |
| [技術・権限・原価](docs/technical.md) | 構成案、セキュリティ、障害時の挙動、単価と費用の仮定 |
| [ブランドとUI](docs/brand-ui.md) | 選定済みAロゴ、現在の画面案、改善提案 |
| [決定ログ](docs/decisions.md) | 確定した方針、統合時の提案、未決事項 |

## 読むときの注意

共有チャット、専門AI、実行、承認は既存サービスにも公式説明があります。それだけをPelway独自の機能とは主張しません。差別化と料金は検証前の仮説です。費用の表は一部の費用だけを計算した例で、総原価や販売価格を保証しません。

Pelway名と選定済みAロゴを維持します。同名のAI教育製品に関する第三者掲載があり、名称調査と機能比較を分けています。商標・ドメインの利用可否を保証しません。

## AIエージェントでの開発の進め方

開発には [mattpocock/skills](https://github.com/mattpocock/skills) のエージェントスキルを使います。スキルは Claude Code や Codex などのコーディングエージェントから `/スキル名` で呼び出します。

### 準備

1. 使うエージェントにスキルを入れます。プラグインとして入れる方法と `npx skills` でファイルとして入れる方法のどちらか一方にしてください。両方入れるとスキルが重複します。他のエージェントでの入れ方は[上流の README](https://github.com/mattpocock/skills#installation-30-second-setup) を参照してください。

   ```bash
   # Claude Code（自動更新）
   claude plugin install mattpocock-skills@claude-plugins-official

   # Codex（自動更新）
   codex plugin marketplace add mattpocock/skills
   codex plugin add mattpocock-skills@mattpocock

   # その他のエージェント、または編集できるファイルとして入れる場合（手動更新）
   npx skills@latest add mattpocock/skills -a <agent>
   ```

2. このリポジトリ向けの設定は済んでいます。`/setup-matt-pocock-skills` を再実行する必要はありません。

   | 設定 | 内容 | ファイル |
   |---|---|---|
   | 課題管理 | GitHub Issues（hatai/Pelway）を `gh` CLI で操作 | [docs/agents/issue-tracker.md](docs/agents/issue-tracker.md) |
   | トリアージラベル | `needs-triage` `needs-info` `ready-for-agent` `ready-for-human` `wontfix` | [docs/agents/triage-labels.md](docs/agents/triage-labels.md) |
   | ドメイン文書 | ルートの `GLOSSARY.md` と `docs/adr/`（必要になった時点で作成） | [docs/agents/domain.md](docs/agents/domain.md) |

3. エージェント向けの指示は [AGENTS.md](AGENTS.md) にあります。Claude Code は、上位ディレクトリに `CLAUDE.md` があると `AGENTS.md` を読みません。両方読ませるには `~/.claude/settings.json` に次を追加します（v2.1.285 以降）。

   ```json
   "pluginConfigs": {
     "cc-plugin-agents-md@builtin": {
       "options": { "instructionFiles": "claude-md-and-agents-md" }
     }
   }
   ```

### 基本の流れ（構想から実装まで）

1. **`/grill-with-docs`**：エージェントの質問に答えながら構想を詰めます。決まった用語は `GLOSSARY.md` に、戻しにくい判断は `docs/adr/` に残ります。
2. 会話だけで決められない問い（状態の持ち方、画面の見え方など）は **`/prototype`** で捨てるつもりのコードを書いて確かめます。
3. 1セッションで終わらない規模なら、**`/to-spec`** で仕様を Issue にまとめ、**`/to-tickets`** で依存関係つきのチケットに分けます。
4. **`/implement`** でチケットを1件ずつ実装します。1件ごとに `/clear` で文脈を空にします。まとめて並列に進めるなら **`/implement-spec`** を使います。実装は **`/tdd`** で進め、最後に **`/code-review`** が入ります。
5. **`/retro`** でセッションを振り返り、次の開発の前に環境（指示ファイル、チェック、規約）を直します。

手順1〜3は同じ会話の中で続けて行い、`/to-tickets` が終わるまで `/clear` や `/compact` をしないでください。

### 状況別の入り口

| 状況 | スキル |
|---|---|
| 1セッションでは見通せない大きな構想（MVP全体など） | `/wayfinder` で判断事項を Issue の地図にし、1件ずつ決めてから `/to-spec` に進む |
| 外から来た不具合報告や要望が溜まった | `/triage`（`/to-tickets` が作ったチケットには使わない） |
| 原因がすぐ分からない不具合 | `/diagnosing-bugs` |
| 一次情報を調べてほしい | `/research` |
| どのスキルを使えばよいか分からない | `/ask-matt` |

## 利用条件

文書、ロゴ、書体を含む素材の再配布条件は未決です。公開されていることをもってオープンソースの利用許諾があるとは扱わないでください。参照先のコードや素材には、それぞれの提供元の条件が適用されます。
