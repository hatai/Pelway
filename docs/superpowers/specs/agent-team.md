# AI 開発チームの運用仕様

指揮役（正本は Grok Bot）・T3 Code・opencode（glm-5.3）・Devin CLI（SWE-2）・Codex・Claude で組む開発チームの運用ルール。人間の手間を最小にし、品質を保ち、指揮役のトークン消費を抑えることを目的とする。この文書は体制と上限の正本で、イベントごとの対応は `docs/agents/grokbot-runbook.md`、指揮役の判断の手順は skill `command-team` に置く。

## 役割と GitHub 上の名義

| 役割 | 実体 | GitHub 名義 | 認証 |
|---|---|---|---|
| 人間（濱田さん） | — | `hatai` | 本人 |
| 指揮 | Grok Bot（クラウド）。動かないときは人間が指名したエージェントが代行する（runbook の「代行」） | — | T3 Connect 経由の MCP |
| 実装 | T3 スレッド・opencode `zai-coding-plan/glm-5.3`・Devin CLI（`acpRegistry_devin`、`swe-2-high`） | GitHub App `pelway-dev`（`pelway-dev[bot]`） | App のインストールトークン（呼び出しごとに発行） |
| レビュー | 実装担当からのレビュー委託（`delegate_task` の子タスク）・Codex（`codex_review`）・Devin CLI（`devin_review`、通常の PR の Codex とのローテーションと高リスクの PR）と Claude（`claude_review`、高リスクの PR と抜き取り）。指揮役がレビュースレッドを立てるのは、指揮役自身の PR と外部投入（skill `command-team` の「外部からのレビュースレッド投入」）の分だけ | GitHub App `pelway-review`（`pelway-review[bot]`） | App のインストールトークン（呼び出しごとに発行） |

App は 2 つに分ける。同じ名義では自分の PR を Approve できないため。App の秘密鍵と ID は `~/.config/pelway-agent/fnox.toml` に age で暗号化して置き、`scripts/agent-gh` が呼び出しごとにトークンを発行する。

役割の変更は、同じ主体への役割の追加として行わない。担当を変える必要があるとき（調査から実装への引き渡しなど）は、別の担当への割り当て替えとして扱う。

人間が行うのは次の 2 つだけ。

1. タスク定義: Issue Form `agent-task` で Issue を書き、`ready-for-agent` を付ける
2. マージの委任: 指揮役が、`pelway-review` の Approve と必須チェックの通過を確かめた PR を `--admin` でマージすることを認める（判定が割れた PR、`needs-human` の PR は人間が判断する）

## タスクの入口と流れ

タスクは 3 つの入口のどれかで始まり、どの入口も起票後は同じ流れ（起票 → 実装 → レビュー → マージ）に合流する。入口B と入口C では、起票の前に調査の段階が入る。

| 入口 | 対象 | 始まり |
|---|---|---|
| A: 通常の開発 | 機能追加・改善・原因が分かっている修正 | 人間が skill `write-issue` で起票し `ready-for-agent` を付ける |
| B: バグ・インシデント | 原因が未理解のバグ・インシデントの報告 | 指揮役が調査スレッド（skill `investigate-bug`）を立てる |
| C: 定期検出 | nightly 検証と eval ケースの自動検出 | 検出された異常の報告を入口B と同じ扱いにする |

入口の対象範囲:

- 原因が分かっている修正、既存 Issue への修正依頼、実装中に見つかったバグは、入口A（通常の起票）で扱う。調査スレッドを立てない
- 入口C は自動検出が有効になった後の入口で、有効になるまでは異常の報告を受けた時点で入口B と同じ扱いにする

### 入口A（通常の開発）の流れ

1. `ready-for-agent` の Issue を Grok Bot が拾い、`agent:working` を付けて実装スレッドを立てる
2. 実装スレッドは skill `implement-issue` に従い、`pelway-dev` の名義でブランチ `agent/<issue番号>-<slug>` に push し、Draft PR（本文に `Closes #<issue番号>`）を作る
3. 実装スレッドは、Draft PR の必須チェックが通り次第、自分でレビューを委託する（`delegate_task`。手順は skill `implement-issue`）。レビュー委託の子は skill `review-pr` に従い、PR に Approve か Request changes を投稿する
4. Request changes なら、実装スレッドが must-fix を直して push し、レビューを再委託する（修正とレビューの繰り返しは実装スレッドの内側で回し、指揮役を経由しない）。修正は 3 回まで。回数と打ち切りの判断は skill `implement-issue`
5. Approve されたら、実装担当が自分で `pr ready` で Draft を解除する（レビュー担当の App には Draft を解除する権限がない）し、終わりの報告をする。人間（hatai）のレビューの指摘は、指揮役が `t3 thread send` で実装スレッドに伝える
6. CI 成功がそろったら、指揮役が skill `command-team` の「マージ」に従ってマージする
7. 終わった worktree は人間が `mise run agent-worktree-prune` で消す

### 入口B（バグ・インシデント）の調査と起票

1. 指揮役は原因が未理解の報告を受けたら、調査スレッド（skill `investigate-bug`）を立てる。調査スレッドは GLM の枠に数える（下の「制約」）
2. 調査スレッドは原因の調査と報告（skill `investigate-bug` の手順 5）までを担い、修正のコードは書かない。報告は `docs/worklogs/bugs/<slug>/report.md` に書き、そのファイルだけの Draft PR（ブランチ `agent/investigate-<slug>`）として残す
3. 指揮役は報告の PR で報告を確認して起票可否を判断し、起票するときは skill `write-issue` で行う（判断の詳細は skill `command-team` の「調査の引き渡し」）。人間は `ready-for-agent` で承認し、入口A の流れに合流する。報告の PR は起票の可否にかかわらずレビューを経てマージされ、報告が main に残る。このレビューは指揮役がレビュースレッドを立てて行う（実装担当のレビュー委託の対象外。調査スレッドは調査に専念させる）
4. 実装は、調査を担ったスレッドへの役割の追加としては行わない。実装スレッドとして割り当て替える（「役割と GitHub 上の名義」の役割の変更の原則）

## 制約

- `pelway-dev` には GitHub App の Workflows 権限を付けない。`.github/workflows/` の変更は人間が行う（push で動く workflow は承認前に secret を読めるため）。例外は Workflows 権限が必要なため人間の名義でしか行えない操作だけで、現在は `actions.lock` の再生成（`ci(deps):` の PR のマージ前。実施は skill `command-team` の「マージ」）の 1 つ
- PR 1 本の差分は追加 800 行・削除 2000 行まで（`mise run pr-size` が追加行と削除行を分けて数える）。`.gitattributes` で `linguist-generated` を付けたファイルは数えない。指揮役が Issue のコメントで超過を認め `pr-size-exception` ラベルを付けた PR は、pr-size の失敗のままマージしてよい（レビュー担当はこのラベルがあれば pr-size の失敗を must-fix に数えない）
- GLM の実装スレッドと調査スレッドは合算して同時に 3 本まで、Devin CLI の実装スレッドは同時に 6 本まで。`agent:working` が付いていて PR が Ready for review になっていない Issue は、対応するスレッド（ブランチ `agent/<N>-`）の `providerInstanceId` で GLM 枠・Devin CLI 枠に振り分けて数える。調査スレッドは GLM 固定なので GLM の枠に数える。実装の枠を超えた分は指揮役が順番待ちにする（満杯時の優先の判断は skill `command-team` の「調査の担当と effort」）
- レビューの同時数（Codex 2 本、Devin CLI 2 本、Claude 1 本）は目安で、順番待ちの管理をしない。通常の PR のレビューは実装担当が委託するため全局管理はせず、実装担当は自分の PR のレビューを 1 件ずつ順に委託し、修正とレビューの繰り返しは直列に回す。指揮役が立てるレビュースレッド（指揮役自身の PR・外部投入）の同時数はこの目安内に収める。待ち時間や利用上限に実害が出たら、改善ループで見直す
- GLM は終日使う。時間帯による使用制限はない。Devin CLI のセッションに時間帯・時間量の上限はない
- Devin CLI のスレッドは `mode` を `smart` までに限って立てる（`bypass` は使わない）。レビュー用の `devin_review` ではこの上限は要らず、sandbox のため権限モードは Autonomous 一択になる（「レビュー担当の権限」）。レビューへの使用は、OS レベル sandbox とドメイン許可リストで GitHub 以外への通信と任意コマンドの影響を実行時に遮断した `devin_review` に限る（手順上のルールだけでは外部通信を遮断できず、PR の説明や差分に埋め込まれた攻撃者制御の指示でレビュー環境の情報を外部へ送信しうるため）
- 実装の振り分け: 仕様が固まった実装は GLM と Devin CLI の空いている方（同時に空いていれば Devin CLI 優先）、曖昧で長い調査・デバッグ系は GLM、「Devin CLI のハーネスごと任せる」チケットは SWE-2 一任（人間の明示か指揮役の判断）。条件と effort（Devin CLI は `thought_level`）の値は skill `command-team` に置く
- Devin CLI に割り当てた Issue には `agent:devin` ラベルを付け、GitHub 上で担当が見えるようにする（`agent:working` と同じタイミングで付け外し。一覧は `docs/agents/triage-labels.md`）
- SWE-2 は Devin CLI のプランの無料枠で使う。プランの無料モードの同時セッション上限（現在 2 本）は Devin Cloud のセッションの制限で、Devin CLI 経由のスレッドには適用されない。無料枠を超える利用が必要になったら、有料枠の追加か上限の見直しを改善ループで判断する
- Issue の書き方（タイトル、本文の見出し、受け入れ条件、分割、`ready-for-agent` の前のゲート、差し戻されたときの直し方）は skill `write-issue` に置く。`.github/ISSUE_TEMPLATE/agent-task.yml` はその写しで、空の Issue は作れない（`blank_issues_enabled: false`）
- 指揮役が渡すプロンプトは Issue 番号と使う skill 名だけにする。作業ルールは AGENTS.md と skill に置く。例外として、スレッドを立てる指示には担当・モデル・effort を示す定型の割り当ての行を入れ、実装担当は PR 本文に `Implementer:`、レビュー担当はレビュー本文に `Reviewer:` の定型の行で写す（書式は skill `command-team`）
- 指揮役が T3 で立てるスレッドには、必ず「役割・対象の番号・日本語の要約」を含むタイトル（例 `[実装] #101 …`、`[レビュー] PR #132 …`）を立てるときに付ける。タイトルが形式に合わないまま作業を始めさせない。形式と直し方は skill `command-team` の「スレッド名」
- 実装とレビューのモデル・effort は、pr-size の行数、変更パス、ラベル、修正の回数で振り分ける。しきい値は skill `command-team` に置き、改善ループで調整する。レビューの振り分け（通常・高リスク・抜き取りの条件と effort）は、実装担当がレビュー委託のときに同じ表で適用する
- 指揮役が T3 のスレッドとして動いているとき、作業をさせるスレッド（実装・調査・レビュー）の作成はネイティブの `t3_thread_launch` で行う（T3 Connect の MCP や CLI のサブプロセスを経由しない）。指揮役が Grok Bot（クラウド）のときは T3 Connect の MCP の同じ操作を使う。実行中のスレッドへの指示送信（`t3_thread_send`）はこの限りではない
- 終わったスレッドの片付け（settle）: PR を作ったスレッドはその PR をスレッドに紐付け（T3 の PR リンク）、マージで自動的に settle される。PR を持たないスレッド（調査・試験・失敗打ち切り・マージされずに閉じた PR のスレッド）は、指揮役が結果を消費した後に `t3_thread_organize`（`action: settle`）で settle する。`needs-human` 中のスレッドは settle しない（人間が再開しうるため）
- main はブランチ保護で守る。`CODEOWNERS` の `* @hatai` により人間の Approve が必須、CI の `mise run verify` と `pr-size` の成功が必須
- hatai 自身が作った PR は hatai が Approve できないため、hatai が管理者として保護を迂回してマージする（`enforce_admins: false`）。エージェントの PR と指揮役が作った PR は、`pelway-review` の Approve の後、指揮役が委任を受けて保護を迂回してマージする

## 状態の表し方

- 正本は GitHub Issues と GitHub Projects のボード
- ラベルは `ready-for-agent`（人間が付ける）、`agent:working`（Grok Bot が付ける）、`agent:devin`（Devin CLI に割り当てたときに Grok Bot が付ける）、`needs-human`（エスカレーション時）の 4 つ
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
- 修正が何回目かは、実装スレッドがレビューの委託回数で数える（skill `implement-issue`）。3 回目の修正では先にレビューの effort を 1 段上げ、修正を出す前に 4 回目に入るなら（最初の実装 + 修正 3 回）、修正を出さずにエスカレーションする。人間のレビューによる修正（指揮役が `t3 thread send` で伝える）の回数も同じ上限に数える

## 停止・迷走への対処

- `stalled` を受けたら、Grok Bot は状況を要約して Issue にコメントし、`needs-human` を付けて部屋で人間に知らせる
- しきい値は `docs/agents/grokbot-runbook.md` で調整する

## GitHub からの通知

- `notify-grokbot.yml` が送るのは、PR のクローズ（マージを含む）、レビューの投稿（OWNER / MEMBER / COLLABORATOR のもの）、verify と nightly（main の総点検）の完了だけ
- payload に `urgency` を付ける。`digest`（まとめにだけ反映）は workflow_run の成功（verify・nightly）と、Approve のレビュー。それ以外は `immediate`
- Grok Bot は `digest` を 9:00 と 18:00（日本時間）の 1 日 2 回のまとめにだけ反映する。部屋で聞かれたときはその場で答える

## レビュー担当の権限

- レビュー担当は PR のコードを実行しない。PR の head は worktree に取り出さずに読み、verify と pr-size は PR のチェック（CI）の結果で判断する（PR のコードはレビュー担当の秘密鍵を読めるため）
- 通常の PR のレビューは、実装担当のスレッドが `delegate_task` でレビュー専用プロバイダーの子タスクとして委託する（`role: "review"`、権限は `auto-accept-edits`、worktree は独立 checkout。手順は skill `implement-issue`）。委託の子がレビュー専用プロバイダーの sandbox（ネットワークを GitHub だけに絞る権限プロファイル）を継承することを、`docs/dev-environment.md` の確認手順で確かめてから使う
- 指揮役が立てるレビュースレッド（指揮役自身の PR・外部投入）は、従来どおりトップレベルのスレッドとして worktree を main から立てる（runbook の「レビューを始める」）
- レビュースレッドは T3 の Codex インスタンス `codex_review` で動かす。ネットワークは `api.github.com` と `github.com` だけに絞る（Codex の `network_proxy` と、`~/.codex/config.toml` の権限プロファイル `review`）
- レビュースレッドは T3 の権限 `auto-accept-edits`（Codex の `workspaceWrite`）で立てる。`full-access` ではコマンドが sandbox の外で動き、ネットワークの制限が効かない。この権限では `.git` が読み取り専用になるので、PR の側のファイルは GitHub API で読む
- Claude のレビュースレッドは T3 の Claude インスタンス `claude_review` で動かす。skill `review-pr` をそのまま使い、使えるコマンドを読み取り系の `git` と `scripts/agent-gh review` に絞り、ネットワークを `api.github.com` と `github.com` だけに絞る。設定と確認の方法は `docs/dev-environment.md` の「レビュー担当の Claude」。確認が済むまでは使わない
- Devin CLI のレビュースレッドは T3 の Devin CLI インスタンス `devin_review` で動かす。skill `review-pr` をそのまま使い、権限モードは Devin CLI 公式 sandbox の Autonomous 一択になる（`bypass` は使わない）。`devin_review` は起動ラッパーで `DEVIN_SANDBOX=true` と専用設定（ドメイン許可リスト・permissions deny）を付けて立ち上がり、GitHub 以外への通信とワークスペース外への書き込みを実行時に遮断する。仕組みと採用の条件（確認の記録）は `docs/dev-environment.md` の「レビュー担当の Devin CLI」
- Devin CLI が実装した PR には Devin CLI のレビューを付けない。同じモデル系列が自分の出力を採点する偏りを避ける
- 指揮役はレビューを兼ねない。指揮役が作った PR もレビュースレッドに回す
- CI 上で動かす助言セキュリティレビューの起動の認可と実行の隔離は `docs/agents/ci-security-review.md` に定める

## 指揮役の運用ルール

- イベントごとの対応表は `docs/agents/grokbot-runbook.md`、毎サイクルの判断の手順は skill `command-team`
- Grok Bot のルーティンとメモリには「この runbook を読んで従う」とだけ書く
- 見張りと状態は、リポジトリのスクリプト（`pelway-agent-watch`、代行時は `scripts/conductor-watch.sh`）と `~/.local/state/pelway/` に置く。セッションの一時フォルダに置かない

## 改善ループ

- 対象は指揮役・実装担当（opencode＋GLM、Devin CLI＋SWE-2）・レビュー担当（Codex、Devin CLI、Claude）の両側
- 材料は、実装担当が残す実装者メモ（`docs/worklogs/issue-<N>.md`、skill `implement-issue` の手順 5）を一次情報とし、レビューの指摘・差し戻しとセッションの記録で裏を取る。食い違ったら成果物とセッションの記録を優先する
- 学びとルールは両方のツールが読む共有層（`AGENTS.md`、`.agents/skills/`、`docs/`）だけに書く。`.claude/skills/<name>` は `.agents/skills/<name>` へのシンボリックリンクにし、差分ゼロを確かめる
- 効果は GLM と SWE-2 の成果物でも測る（同じ種類のレビュー指摘・差し戻しの減少）。古い学び・矛盾する学びは改善のたびに直すか消す
- 手順は skill `command-team` の手順 4

## 導入の段階

1. リポジトリ側: AGENTS.md・skill・Issue Form・ラベル・PR サイズ検査・ブランチ保護・エージェント用アカウント
2. 見張り役と通知: 見張り役の常駐、旧来のフックの廃止、GitHub 通知の絞り込み
3. Grok Bot の runbook とレビュー担当の sandbox
4. 試運転: #59 と #62 を 800 行以下の積み重ね PR に分割する
