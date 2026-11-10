# AI 開発チームの運用仕様

Grok Bot・T3 Code・opencode（glm-5.3）・Codex で組む開発チームの運用ルール。人間の手間を最小にし、品質を保ち、Grok Bot のトークン消費を抑えることを目的とする。

## 役割と GitHub 上の名義

| 役割 | 実体 | GitHub 名義 | 認証 |
|---|---|---|---|
| 人間（濱田さん） | — | `hatai` | 本人 |
| 指揮 | Grok Bot（クラウド） | — | T3 Connect 経由の MCP |
| 実装 | T3 スレッド・opencode `zai-coding-plan/glm-5.3` | GitHub App `pelway-dev`（`pelway-dev[bot]`） | App のインストールトークン（呼び出しごとに発行） |
| レビュー | T3 スレッド・Codex | GitHub App `pelway-review`（`pelway-review[bot]`） | App のインストールトークン（呼び出しごとに発行） |

App は 2 つに分ける。同じ名義では自分の PR を Approve できないため。App の秘密鍵と ID は `~/.config/pelway-agent/fnox.toml` に age で暗号化して置き、`scripts/agent-gh` が呼び出しごとにトークンを発行する。

人間が行うのは次の 2 つだけ。

1. タスク定義: Issue Form `agent-task` で Issue を書き、`ready-for-agent` を付ける
2. マージ承認: PR を `hatai` として Approve し、マージする

## タスクの流れ

1. `ready-for-agent` の Issue を Grok Bot が拾い、`agent:working` を付けて実装スレッドを立てる
2. 実装スレッドは skill `implement-issue` に従い、`pelway-dev` の名義でブランチ `agent/<issue番号>-<slug>` に push し、Draft PR（本文に `Closes #<issue番号>`）を作る
3. レビュースレッドは skill `review-pr` に従い、PR に Approve か Request changes を投稿する
4. Request changes なら、Grok Bot は同じ実装スレッドに `t3 thread send` で修正を指示する。同じスレッドでの修正は 3 回まで。超えたら要約を渡して新しいスレッドにする
5. Approve と CI 成功がそろったら、人間が Approve してマージする。マージ後、worktree は自動で削除する

## 制約

- PR 1 本の差分は 800 行まで。`.gitattributes` で `linguist-generated` を付けたファイルは数えない
- GLM の実装スレッドは同時に 3 本まで。超えた分は Grok Bot が順番待ちにする
- GLM は終日使う。時間帯による使用制限はない
- Grok Bot が渡すプロンプトは Issue 番号と使う skill 名だけにする。作業ルールは AGENTS.md と skill に置く
- main はブランチ保護で守る。`CODEOWNERS` の `* @hatai` により人間の Approve が必須、CI の `mise run verify` と `pr-size` の成功が必須
- hatai 自身が作った PR は hatai が Approve できないため、hatai が管理者として保護を迂回してマージする（`enforce_admins: false`）。エージェントの PR は迂回せず、hatai の Approve でマージする

## 状態の表し方

- 正本は GitHub Issues と GitHub Projects のボード
- ラベルは `ready-for-agent`（人間が付ける）、`agent:working`（Grok Bot が付ける）、`needs-human`（エスカレーション時）の 3 つ
- 「レビュー待ち」「修正待ち」「マージ待ち」は PR の Draft 状態・レビュー状態・CI 結果から Projects の組み込みワークフローで反映する

## 停止・迷走の検知と対処

- ローカルの見張りデーモン（pitchfork で常駐、`~/.config/pelway-agent-notify/` に置く）が `t3 thread list --json` と `gh` を見て、次のどれかで `stalled` を Webhook 送信する
  - スレッドが動作中のまま 45 分イベントがない
  - 同じ Issue の修正が 3 回目に入った
  - 同じ PR で CI が 2 回続けて失敗した
- 1 回目の `stalled`: Grok Bot が状況を要約し、新しいスレッドでやり直させる
- 2 回目の `stalled`: Issue に `needs-human` を付け、部屋で人間に知らせる
- しきい値は `docs/agents/grokbot-runbook.md` で調整する

## 通知

- 通知プラグインは自分の worktree 以外のセッションのイベントを `notify.sh` 起動前に捨てる
- インストーラーは古いバージョンのプラグインを上書きする
- payload に `issue_number` と `round` を含める
- 即時対応するイベント: failed、waiting_permission、waiting_input、stalled、Request changes のレビュー、CI 失敗、マージ
- 上記以外は Grok Bot が朝と夕方の 1 日 2 回のまとめにだけ反映する。部屋で聞かれたときはその場で答える

## レビュー担当の権限

- Codex のレビュースレッドは read-only の sandbox で動かし、ネットワークは `gh` だけ許可する

## Grok Bot の運用ルール

- 正本は `docs/agents/grokbot-runbook.md`（役割、同時数の上限、イベントごとの対応表、エスカレーション手順、まとめの書式）
- Grok Bot のルーティンとメモリには「この runbook を読んで従う」とだけ書く

## 導入の段階

1. リポジトリ側: AGENTS.md・skill・Issue Form・ラベル・PR サイズ検査・ブランチ保護・エージェント用アカウント
2. 通知側: プラグインの重複修正、payload の拡張、イベントの絞り込み
3. 見張りデーモン
4. Grok Bot の runbook とレビュー担当の sandbox
5. 試運転: #59 と #62 を 800 行以下の積み重ね PR に分割する
