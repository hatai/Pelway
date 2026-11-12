# AI 開発チームの運用仕様

指揮役（正本は Grok Bot）・T3 Code・opencode（glm-5.3）・Codex・Claude で組む開発チームの運用ルール。人間の手間を最小にし、品質を保ち、指揮役のトークン消費を抑えることを目的とする。この文書は体制と上限の正本で、イベントごとの対応は `docs/agents/grokbot-runbook.md`、指揮役の判断の手順は skill `command-team` に置く。

## 役割と GitHub 上の名義

| 役割 | 実体 | GitHub 名義 | 認証 |
|---|---|---|---|
| 人間（濱田さん） | — | `hatai` | 本人 |
| 指揮 | Grok Bot（クラウド）。動かないときは人間が指名したエージェントが代行する（runbook の「代行」） | — | T3 Connect 経由の MCP |
| 実装 | T3 スレッド・opencode `zai-coding-plan/glm-5.3` | GitHub App `pelway-dev`（`pelway-dev[bot]`） | App のインストールトークン（呼び出しごとに発行） |
| レビュー | T3 スレッド・Codex（`codex_review`）と Claude（`claude_review`、高リスクの PR と抜き取り） | GitHub App `pelway-review`（`pelway-review[bot]`） | App のインストールトークン（呼び出しごとに発行） |

App は 2 つに分ける。同じ名義では自分の PR を Approve できないため。App の秘密鍵と ID は `~/.config/pelway-agent/fnox.toml` に age で暗号化して置き、`scripts/agent-gh` が呼び出しごとにトークンを発行する。

人間が行うのは次の 2 つだけ。

1. タスク定義: Issue Form `agent-task` で Issue を書き、`ready-for-agent` を付ける
2. マージの委任: 指揮役が、`pelway-review` の Approve と必須チェックの通過を確かめた PR を `--admin` でマージすることを認める（判定が割れた PR、`needs-human` の PR は人間が判断する）

## タスクの流れ

1. `ready-for-agent` の Issue を Grok Bot が拾い、`agent:working` を付けて実装スレッドを立てる
2. 実装スレッドは skill `implement-issue` に従い、`pelway-dev` の名義でブランチ `agent/<issue番号>-<slug>` に push し、Draft PR（本文に `Closes #<issue番号>`）を作る
3. レビュースレッドは skill `review-pr` に従い、PR に Approve か Request changes を投稿する
4. Request changes なら、Grok Bot は同じ実装スレッドに `t3 thread send` で修正を指示する。同じスレッドでの修正は 3 回まで。超えたら要約を渡して新しいスレッドにする
5. Approve されたら、Grok Bot が実装スレッドに知らせ、実装担当が `pr ready` で Draft を解除する（レビュー担当の App には Draft を解除する権限がない）
6. CI 成功がそろったら、指揮役が skill `command-team` の「マージ」に従ってマージする
7. 終わった worktree は人間が `mise run agent-worktree-prune` で消す

## 制約

- `pelway-dev` には GitHub App の Workflows 権限を付けない。`.github/workflows/` の変更は人間が行う（push で動く workflow は承認前に secret を読めるため）
- PR 1 本の差分は追加 800 行・削除 2000 行まで（`mise run pr-size` が追加行と削除行を分けて数える）。`.gitattributes` で `linguist-generated` を付けたファイルは数えない。指揮役が Issue のコメントで超過を認め `pr-size-exception` ラベルを付けた PR は、pr-size の失敗のままマージしてよい（レビュー担当はこのラベルがあれば pr-size の失敗を must-fix に数えない）
- GLM の実装スレッドは同時に 3 本まで、Codex のレビュースレッドは同時に 2 本まで、Claude のレビュースレッドは同時に 1 本まで。超えた分は指揮役が順番待ちにする。上限は、Codex の見落とし率とレビューの待ち時間をもとに見直す
- GLM は終日使う。時間帯による使用制限はない
- 指揮役が渡すプロンプトは Issue 番号と使う skill 名だけにする。作業ルールは AGENTS.md と skill に置く。例外として、スレッドを立てる指示には担当・モデル・effort を示す定型の割り当ての行を入れ、実装担当は PR 本文に `Implementer:`、レビュー担当はレビュー本文に `Reviewer:` の定型の行で写す（書式は skill `command-team`）
- 実装とレビューのモデル・effort は、pr-size の行数、変更パス、ラベル、修正の回数で振り分ける。しきい値は skill `command-team` に置き、改善ループで調整する
- main はブランチ保護で守る。`CODEOWNERS` の `* @hatai` により人間の Approve が必須、CI の `mise run verify` と `pr-size` の成功が必須
- hatai 自身が作った PR は hatai が Approve できないため、hatai が管理者として保護を迂回してマージする（`enforce_admins: false`）。エージェントの PR と指揮役が作った PR は、`pelway-review` の Approve の後、指揮役が委任を受けて保護を迂回してマージする

## 状態の表し方

- 正本は GitHub Issues と GitHub Projects のボード
- ラベルは `ready-for-agent`（人間が付ける）、`agent:working`（Grok Bot が付ける）、`needs-human`（エスカレーション時）の 3 つ
- 「レビュー待ち」「修正待ち」「マージ待ち」は PR の Draft 状態・レビュー状態・CI 結果から Projects の組み込みワークフローで反映する

## 見張り役と通知

エージェントの状態の通知は、見張り役 `pelway-agent-watch`（`crates/pelway-agent-watch`、pitchfork で常駐）が一元的に出す。エージェントごとのフック（OpenCode プラグイン、Codex / Claude の hooks）は使わない。

- 30 秒ごとに `t3 thread list --json` を読み、状態が変わったスレッドと実行中のスレッドを `t3 thread show` で詳しく見る
- 送るイベント（すべて即時対応）
  - `finished` / `failed` / `cancelled`: スレッドの実行が終わった
  - `waiting`: 承認や入力の待ちが発生した
  - `stalled`: 次のどれか。理由を `stall_reason` に入れる
    - `no-activity`: 実行中のまま 45 分更新がない
    - `ci-failed-twice`: `agent/` ブランチで verify が 2 回続けて失敗した（5 分ごとに `gh run list` で確かめる）
- payload は `pelway.agent-watch/v1`。スレッド ID、ブランチ、`issue_number`（`agent/<番号>-` から取る）、`round`（スレッドの実行回数）、`report`（最後の返答が `キー: 値` 形式 5 行以内のときだけ、その本文）を含む
- 同じ出来事からイベントを作るのは 1 回だけ。送れなかったものは送信待ちの列に残して送り直す（受け手には重複して届きうる）。状態は `~/.local/state/pelway/agent-watch.json` に置く
- 修正が何回目かは `round` で指揮役が判断する。`round` が 3 の修正では、先に effort を上げるかモデルを替える。修正を出す前に `round` が 4 以上なら（最初の実装 + 修正 3 回）、修正を出さずにエスカレーションする

## 停止・迷走への対処

- `stalled` を受けたら、Grok Bot は状況を要約して Issue にコメントし、`needs-human` を付けて部屋で人間に知らせる
- しきい値は `docs/agents/grokbot-runbook.md` で調整する

## GitHub からの通知

- `notify-grokbot.yml` が送るのは、PR のクローズ（マージを含む）、レビューの投稿（OWNER / MEMBER / COLLABORATOR のもの）、verify の完了だけ
- payload に `urgency` を付ける。`digest`（まとめにだけ反映）は verify の成功と、Approve のレビュー。それ以外は `immediate`
- Grok Bot は `digest` を 9:00 と 18:00（日本時間）の 1 日 2 回のまとめにだけ反映する。部屋で聞かれたときはその場で答える

## レビュー担当の権限

- レビュー担当は PR のコードを実行しない。PR の head は worktree に取り出さずに読み、verify と pr-size は PR のチェック（CI）の結果で判断する（PR のコードはレビュー担当の秘密鍵を読めるため）
- レビュースレッドは T3 の Codex インスタンス `codex_review` で動かす。ネットワークは `api.github.com` と `github.com` だけに絞る（Codex の `network_proxy` と、`~/.codex/config.toml` の権限プロファイル `review`）
- レビュースレッドは T3 の権限 `auto-accept-edits`（Codex の `workspaceWrite`）で立てる。`full-access` ではコマンドが sandbox の外で動き、ネットワークの制限が効かない。この権限では `.git` が読み取り専用になるので、PR の側のファイルは GitHub API で読む
- Claude のレビュースレッドは T3 の Claude インスタンス `claude_review` で動かす。skill `review-pr` をそのまま使い、使えるコマンドを読み取り系の `git` と `scripts/agent-gh review` に絞り、ネットワークを `api.github.com` と `github.com` だけに絞る。設定と確認の方法は `docs/dev-environment.md` の「レビュー担当の Claude」。確認が済むまでは使わない
- 指揮役はレビューを兼ねない。指揮役が作った PR もレビュースレッドに回す

## 指揮役の運用ルール

- イベントごとの対応表は `docs/agents/grokbot-runbook.md`、毎サイクルの判断の手順は skill `command-team`
- Grok Bot のルーティンとメモリには「この runbook を読んで従う」とだけ書く
- 見張りと状態は、リポジトリのスクリプト（`pelway-agent-watch`、代行時は `scripts/conductor-watch.sh`）と `~/.local/state/pelway/` に置く。セッションの一時フォルダに置かない

## 改善ループ

- 対象は指揮役・実装担当（opencode＋GLM）・レビュー担当（Codex、Claude）の両側
- 材料は、実装担当が残す実装者メモ（`docs/worklogs/issue-<N>.md`、skill `implement-issue` の手順 5）を一次情報とし、レビューの指摘・差し戻しとセッションの記録で裏を取る。食い違ったら成果物とセッションの記録を優先する
- 学びとルールは両方のツールが読む共有層（`AGENTS.md`、`.agents/skills/`、`docs/`）だけに書く。`.claude/skills/<name>` は `.agents/skills/<name>` へのシンボリックリンクにし、差分ゼロを確かめる
- 効果は GLM の成果物でも測る（同じ種類のレビュー指摘・差し戻しの減少）。古い学び・矛盾する学びは改善のたびに直すか消す
- 手順は skill `command-team` の手順 4

## 導入の段階

1. リポジトリ側: AGENTS.md・skill・Issue Form・ラベル・PR サイズ検査・ブランチ保護・エージェント用アカウント
2. 見張り役と通知: 見張り役の常駐、旧来のフックの廃止、GitHub 通知の絞り込み
3. Grok Bot の runbook とレビュー担当の sandbox
4. 試運転: #59 と #62 を 800 行以下の積み重ね PR に分割する
