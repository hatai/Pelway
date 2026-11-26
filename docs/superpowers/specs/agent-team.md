# AI 開発チームの運用仕様

指揮役（正本は Grok Bot）・T3 Code・opencode・Devin CLI・Codex・Claude で組む開発チームの運用ルール。人間の手間を最小にし、品質を保ち、指揮役のトークン消費を抑えることを目的とする。この文書は体制の正本で、同時数の上限と担当の登録情報（T3 の provider インスタンス名・モデル）の正本は `agent-team.toml` に置く。イベントごとの対応は `docs/agents/grokbot-runbook.md`、指揮役の判断の手順は skill `command-team` に置く。

## 役割と GitHub 上の名義

| 役割 | 実体 | GitHub 名義 | 認証 |
|---|---|---|---|
| 人間（濱田さん） | — | `hatai` | 本人 |
| 指揮 | Grok Bot（クラウド）。動かないときは人間が指名したエージェントが代行する（runbook の「代行」） | — | T3 Connect 経由の MCP |
| 実装 | T3 スレッド・opencode・Devin CLI（登録情報は `agent-team.toml` の `[agents]`） | GitHub App `pelway-dev`（`pelway-dev[bot]`） | App のインストールトークン（呼び出しごとに発行） |
| レビュー | 実装担当からのレビュー委託（`delegate_task` の子タスク）・Codex・Devin CLI（通常の PR の Codex とのローテーションと高リスクの PR）と Claude（高リスクの PR と抜き取り。登録情報は `agent-team.toml` の `[agents]`）。指揮役がレビュースレッドを立てるのは、指揮役自身の PR と外部投入（`docs/agents/command-team-details.md` の「外部からのレビュースレッド投入」）の分だけ | GitHub App `pelway-review`（`pelway-review[bot]`） | App のインストールトークン（呼び出しごとに発行） |

App は 2 つに分ける。同じ名義では自分の PR を Approve できないため。App の秘密鍵と ID は `~/.config/pelway-agent/fnox.toml` に age で暗号化して置き、`scripts/agent-gh` が呼び出しごとにトークンを発行する。

役割の変更は、同じ主体への役割の追加として行わない。担当を変える必要があるとき（調査から実装への引き渡しなど）は、別の担当への割り当て替えとして扱う。

人間が行うのは次の 3 つだけ。

1. タスク定義: Issue Form `agent-task` で Issue を書き、`ready-for-agent` を付ける（GitHub Projects のボードの Backlog に載る）
2. 処理の合図: ボードで、処理してよい Issue のカードを Backlog から Todo へ動かす
3. マージの委任: 指揮役が、`pelway-review` の Approve と必須チェックの通過と 現在の head と同じ SHA の QA の合格コメント（`pelway-qa` 名義。形と数え方は `docs/agents/qa-checklist.md`）を確かめた PR に `automerge` ラベルを付けて、Kodiak（マージキューの GitHub App。「マージキュー」参照）にマージを委任することを認める（判定が割れた PR は修正側の指摘を優先して対応し、`needs-human` の PR は人間が判断する）

## タスクの入口と流れ

タスクは 4 つの入口のどれかで始まり、どの入口も起票後は同じ流れ（起票 → 実装 → レビュー → マージ）に合流する。入口B と入口C では、起票の前に調査の段階が入る。

| 入口 | 対象 | 始まり |
|---|---|---|
| A: 通常の開発 | 機能追加・改善・原因が分かっている修正 | 人間が skill `write-issue` で起票し `ready-for-agent` を付ける（ボードの Backlog に載る）。人間がカードを Todo へ動かすと処理が始まる |
| B: バグ・インシデント | 原因が未理解のバグ・インシデントの報告 | 指揮役が調査スレッド（skill `investigate-bug`）を立てる |
| C: 定期検出 | nightly 検証と eval ケースの自動検出 | 検出された異常の報告を入口B と同じ扱いにする |
| D: 週次の棚卸し | 長命 PR の棚卸しと作業ログの定期キュレーション | 指揮役が週 1 回棚卸し Issue を起票して棚卸し担当スレッドを立てる（`ready-for-agent` は付けずボードにも載せない。`docs/agents/command-team-details.md`「週次: 長命 PR の棚卸し」） |

入口の対象範囲:

- 原因が分かっている修正、既存 Issue への修正依頼、実装中に見つかったバグは、入口A（通常の起票）で扱う。調査スレッドを立てない
- 入口C は自動検出が有効になった後の入口で、有効になるまでは異常の報告を受けた時点で入口B と同じ扱いにする

### 入口A（通常の開発）の流れ

1. `ready-for-agent` の付いた Issue は、ボードの組み込みワークフロー（Auto-add、フィルタ `is:issue label:ready-for-agent`）で Backlog に載る。人間がカードを Backlog から Todo へ動かしたものが処理対象になる（Todo が処理の待ち行列を兼ねる）
2. 指揮役は巡回で、ボードの Todo のカードを並び順に読み、`ready-for-agent` の付いたものから順に `agent:working` を付けて実装スレッドへ割り当てる（`ready-for-agent` は着手後も外さない。巡回の手順は runbook の「ボードと巡回」）
3. 実装スレッドは skill `implement-issue` に従い、`pelway-dev` の名義でブランチ `agent/<issue番号>-<slug>` に push し、Draft PR（本文に `Closes #<issue番号>`）を作る
4. 実装スレッドは、Draft PR の必須チェックが通り次第、自分でレビューを委託する（`delegate_task`。手順は skill `implement-issue`）。レビュー委託の子は skill `review-pr` に従い、PR に Approve か Request changes を投稿する。レビュー担当は開始時に fail-closed のツール一覧チェック（渡されたツール名の一覧を記録し、書き込み系ツールが含まれる、または一覧が読めないときは何もせず終了する）を行い、チェックに失敗したレビューは結果を採用しない（投稿済みのレビューも「有効な Approve」に数えない。数え方は skill `command-team` の「hatai のレビューの扱い」）
5. Request changes なら、実装スレッドが must-fix を直して push し、レビューを再委託する（修正とレビューの繰り返しは実装スレッドの内側で回し、指揮役を経由しない）。修正は 3 回まで（round 4 に入るのは hatai の例外の判断だけ。ラウンドの数え方と上限は skill `command-team` の「レビューのラウンド」）。回数と打ち切りの判断は skill `implement-issue`
6. Approve されたら、実装担当が自分で `pr ready` で Draft を解除する。Approve のレビューに `[文書のずれ]` の nit が残っていれば、先に直してレビューを再委託する（skill `implement-issue` の手順 9）（レビュー担当の App には Draft を解除する権限がない）し、終わりの報告をする。人間（hatai）のレビューの指摘は、指揮役が `t3 thread send` で実装スレッドに伝える
7. Approve と必須チェックがそろったら、QA が同じ SHA で独立に確かめ、SHA と run を書いた合格コメントを PR に残す（`docs/agents/qa-checklist.md`）。skill `command-team` の「マージ」の `automerge` を付ける条件（有効な Approve、その委託の結果の確定、PR に `needs-human` が無いこと、必須チェック、同じ SHA の QA の合格など。条件の正本はその節）をすべて確かめてから、指揮役が `automerge` ラベルを付けて Kodiak のマージキューに入れる。QA が確かめるまでは合格として扱わない
8. 終わった worktree は pitchfork のデーモン `agent-worktree-prune` が毎時消す（settle されていないスレッドの worktree、生きているプロセスのカレントディレクトリが中にある worktree、最後の変更から 6 時間以内の worktree、追跡ファイルに変更が残っている worktree は残す。残ったものは人間が `mise run agent-worktree-prune` で整理する）

### 入口B（バグ・インシデント）の調査と起票

1. 指揮役は原因が未理解の報告を受けたら、調査スレッド（skill `investigate-bug`）を立てる。調査スレッドは GLM の枠に数える（下の「制約」）
2. 調査スレッドは原因の調査と報告（skill `investigate-bug` の手順 5）までを担い、修正のコードは書かない。報告は `docs/worklogs/bugs/<slug>/report.md` に書き、そのファイルだけの Draft PR（ブランチ `agent/investigate-<slug>`）として残す
3. 指揮役は報告の PR で報告を確認して起票可否を判断し、起票するときは skill `write-issue` で行う（判断の詳細は `docs/agents/command-team-details.md` の「調査（入口B）」）。人間は `ready-for-agent` で承認し、入口A の流れに合流する。報告の PR は起票の可否にかかわらずレビューを経てマージされ、報告が main に残る。このレビューは指揮役がレビュースレッドを立てて行う（実装担当のレビュー委託の対象外。調査スレッドは調査に専念させる）
4. 実装は、調査を担ったスレッドへの役割の追加としては行わない。実装スレッドとして割り当て替える（「役割と GitHub 上の名義」の役割の変更の原則）

## 制約

- `pelway-dev` には GitHub App の Workflows 権限を付けない。`.github/workflows/` の変更は人間が行う（push で動く workflow は承認前に secret を読めるため）。例外は Workflows 権限が必要なため人間の名義でしか行えない操作だけで、現在は `actions.lock` の再生成（`ci(deps):` の PR のマージ前。実施は skill `command-team` の「マージ」）の 1 つ
- PR 1 本の差分は追加 800 行・削除 2000 行まで（`mise run pr-size` が追加行と削除行を分けて数える）。`.gitattributes` で `linguist-generated` を付けたファイルは数えない。指揮役が Issue のコメントで超過を認め `pr-size-exception` ラベルを付けた PR は、レビューでは pr-size の失敗を must-fix に数えない。pr-size の失敗したままの PR は必須チェックを通らないため Kodiak ではマージできず、`automerge` ラベルは付けずに hatai の例外の判断に回す（`pr-size-exception` 自体はマージの例外には加えない）
- 実装と調査のスレッドの同時枠の上限は、`agent-team.toml` の `[developers.<login>.slots]` が正本（GLM は実装と調査の合算、Devin CLI は実装の枠）。枠の消費数は、open な Issue の `agent:working` の数から、実際の停止を確認できるもの（補助記録の作業状態の印が停止中・再開処理中・再開待ちのもののうち、runbook の「ボードと巡回」の「枠の数え方」の手順で停止を確認できたものだけ。確認できないものは使用中として数える）を除いた数に、Issue を持たない入口B の調査の稼働中スレッド数（巡回記録の割り当て表から数える）を加えたもの。GLM 枠と Devin CLI 枠の内訳は Issue の `agent:devin` ラベルの有無で判定する。実装の枠を超えた分は指揮役が順番待ちにする（満杯時の優先の判断は `docs/agents/command-team-details.md` の「調査（入口B）」）
- GitHub Projects のボードの読み書きは人間の名義の `gh` で行う（読み取り `read:project`、書き込み `project` スコープ）。`scripts/agent-gh` は使わない（人間の名義で行う操作の既存の枠と同じ扱い）。スコープが無ければ巡回を止めて hatai に報告する
- レビューの同時数の目安は、`agent-team.toml` の `[developers.<login>.review_limits]` が正本で、順番待ちの管理をしない。通常の PR のレビューは実装担当が委託するため全局管理はせず、実装担当は自分の PR のレビューを 1 件ずつ順に委託し、修正とレビューの繰り返しは直列に回す。指揮役が立てるレビュースレッド（指揮役自身の PR・外部投入）の同時数はこの目安内に収める。待ち時間や利用上限に実害が出たら、改善ループで見直す
- GLM は終日使う。時間帯による使用制限はない。Devin CLI のセッションに時間帯・時間量の上限はない
- Devin CLI のスレッドは `mode` を `smart` までに限って立てる（`bypass` は使わない）。レビュー用の Devin CLI インスタンスではこの上限は要らず、sandbox のため権限モードは Autonomous 一択になる（「レビュー担当の権限」）。レビューへの使用は、OS レベル sandbox とドメイン許可リストで GitHub 以外への通信と任意コマンドの影響を実行時に遮断したレビュー用インスタンスに限る（手順上のルールだけでは外部通信を遮断できず、PR の説明や差分に埋め込まれた攻撃者制御の指示でレビュー環境の情報を外部へ送信しうるため）
- 実装の振り分け: 条件（定型・小規模、通常、曖昧で長い調査・デバッグ系、「Devin CLI のハーネスごと任せる」チケット。判定と effort は skill `command-team` に置く）ごとの担当の優先順位は、`agent-team.toml` の `[developers.<login>.priority.implement]`（開発者ごとの表）が正本。担当の登録情報（T3 の provider インスタンス名・モデル・effort の option と段）は `[agents.<id>]` に置く
- Devin CLI に割り当てた Issue には `agent:devin` ラベルを付け、GitHub 上で担当が見えるようにする（`agent:working` と同じタイミングで付け外し。一覧は `docs/agents/triage-labels.md`）
- SWE-2 は Devin CLI のプランの無料枠で使う。プランの無料モードの同時セッション上限（現在 2 本）は Devin Cloud のセッションの制限で、Devin CLI 経由のスレッドには適用されない。無料枠を超える利用が必要になったら、有料枠の追加か上限の見直しを改善ループで判断する
- Issue の書き方（タイトル、本文の見出し、受け入れ条件、分割、`ready-for-agent` の前のゲート、差し戻されたときの直し方）は skill `write-issue` に置く。`.github/ISSUE_TEMPLATE/agent-task.yml` はその写しで、空の Issue は作れない（`blank_issues_enabled: false`）
- 指揮役が渡すプロンプトは Issue 番号と使う skill 名だけにする。作業ルールは AGENTS.md と skill に置く。例外として、スレッドを立てる指示には担当・モデル・effort を示す定型の割り当ての行を入れ、実装担当は PR 本文に `Implementer:`、レビュー担当はレビュー本文に `Reviewer:` の定型の行で写す（書式は skill `command-team`）
- 指揮役が T3 で立てるスレッドには、必ず「役割・対象の番号・日本語の要約」を含むタイトル（例 `[実装] #101 …`、`[レビュー] PR #132 …`）を立てるときに付ける。タイトルが形式に合わないまま作業を始めさせない。形式と直し方は skill `command-team` の「スレッド名」
- 実装とレビューのモデル・effort は、pr-size の行数、変更パス、ラベル、修正の回数で振り分ける。しきい値は skill `command-team` に置き、改善ループで調整する。レビューの振り分け（通常・高リスク・抜き取りの条件と effort）は、実装担当がレビュー委託のときに同じ表で適用する
- 指揮役が T3 のスレッドとして動いているとき、作業をさせるスレッド（実装・調査・レビュー）の作成はネイティブの `t3_thread_launch` で行う（T3 Connect の MCP や CLI のサブプロセスを経由しない）。指揮役が Grok Bot（クラウド）のときは T3 Connect の MCP の同じ操作を使う。実行中のスレッドへの指示送信（`t3_thread_send`）はこの限りではない
- 終わったスレッドの片付け（settle）: PR を作ったスレッドはその PR をスレッドに紐付け（T3 の PR リンク）、マージで自動的に settle される。それ以外は、指揮役が毎回の巡回で、状態で決めた条件（`docs/agents/grokbot-runbook.md` の「9. 片付け」）に当たるものを `t3_thread_organize`（`action: settle`）で settle する。`needs-human` 中のスレッドは settle しない（人間が再開しうるため）
- main はルールセットで守る。必須チェックは `mise run verify` と `pr-size` の成功、承認は 1 件必須（Code Owners レビューの必須は外し、`pelway-review` の Approve で満たせる形）、新しいコミットで古い Approve を外す、「マージ前にブランチを最新化すること」を必須とする（Kodiak の直列キューと合わせて、チェックが通ったツリーとマージされるツリーを一致させる）。ルールセットの変更前後の値は「マージキュー」の表に転記する
- エージェントの PR・指揮役が作った PR・hatai 自身が作った PR のいずれも、`pelway-review` の Approve と必須チェックと QA 合格コメントの後、指揮役が `automerge` ラベルを付けて Kodiak がマージする。hatai の admin マージの対象は、round 4 以降の例外 PR（ラウンドの上限は skill `command-team` の「レビューのラウンド」）、`.github/` の下を変える PR、`fnox.toml` を変える PR の 3 つだけである（マージの例外はこれ以外に広げない。Kodiak が応答しない等のキューの切り戻しや、必須チェックが失敗したままの `pr-size-exception` は例外には加えず、hatai がこの 3 つのどれかに該当すると判断した PR だけが admin マージされる。`enforce_admins: false` を維持するのはこの例外のためである）

## 開発者ごとの上書き（agent-team.local.toml）

`agent-team.toml` は共有の登録情報として git で追跡し、開発者ごとの差分は追跡しない `agent-team.local.toml` に書いて共有の上に重ねる（Issue #804。雛形は `agent-team.local.toml.example`）。local は `agent-team.toml` と同じディレクトリ（チェックアウトの最上位）に置き、`scripts/agent-team-config` の `validate`・`slots`・`effective` が常に重ねた結果を読む。手順書・skill 側の参照は、`instance.review` など「上書きできない」項目を除き、toml の直接読みではなく重ねた結果（`effective` の出力）を正とする。`conductor-watch.sh` は local が重なっている巡回に `SLOTS overlay agent-team.local.toml` の行を出す。

local を置く場所: 本体の checkout に置くと worktree 作成時に `scripts/worktree-setup`（T3 の `runOnWorktreeCreate` または `conductor-rebuild` から起動）が worktree に持ち越す。worktree ごとに別の値にしたいときはその worktree に置く。追跡しないため、`.worktreeinclude` にも列挙してある（Claude Code・Codex の worktreeinclude 機構用。`agent-team.toml` は追跡ファイルのため列挙しない）。

上書きできる項目・できない項目:

| 項目 | 扱い |
|---|---|
| `developers.<自分>.use` | 上書き可（配列は丸ごと置き換え。外した担当への参照は上限・優先順位からも落ちる） |
| `developers.<自分>.slots.<担当>` | 上書き可（表は深く重ねる。1 以上の整数・use・役割の整合は重ねた結果の検証が掛ける） |
| `developers.<自分>.review_limits.<担当>` | 上書き可（同上） |
| `developers.<自分>.priority.<役割>.<既存の条件名>` | 上書き可（並びは丸ごと置き換え。共有の toml に無い役割・条件名は警告して無視） |
| `agents.<既存の担当>.models` | 上書き可（一覧は丸ごと置き換え） |
| `agents.<既存の担当>.effort.raise` | 上書き可（値の許可リストを丸ごと置き換え。option の名前自体は共有） |
| `agents.<既存の担当>.instance.implement`・`.investigate` | 上書き可（その環境の provider インスタンス名に合わせる） |
| `agents.<既存の担当>.instance.review` | 上書き不可（レビュー担当の隔離の前提。main の最新の版と dev-environment.md の照合に使う共有の値） |
| `agents.<既存の担当>.effort.option` | 上書き不可（委託の引数の組み立て規則は共有） |
| `agents.<既存の担当>.assign_label` | 上書き不可（割り当てのラベルは共有） |
| `agents.<新しい id>`・`developers.<共有に無い login>` | 追加不可（警告して無視。共有の toml の PR で追加する） |
| `developers.<他の開発者>`（slots・effective の呼び出し側以外の表） | 上書き不可（警告して無視） |
| 上記以外のキー・知らない最上位キー | 警告して無視 |

上書きの扱いと未決事項:

- 表は深く重ね、配列は丸ごと置き換える。上書きできない項目は「`<file>: warning: <キー>: <理由>`」を標準エラーに出して無視する。警告と検証エラーには local の値を出さず、キー名と理由だけを出す（`effective` などの正常な出力には、設計どおり重ねた結果の値が入る）。黙って適用も黙って無視もしない
- 重ねた結果は共有の toml と同じ検証を通す。通らなければ `validate`・`slots`・`effective` は非 0 で止まる（共有の規則を local が下げることはできない）。use から担当を外したとき、その担当への共有から継いだ参照（`slots`・`review_limits`・`priority` 内の並び）は重ねた結果から落ちる（残すと検証で止まり、外す上書き自体が書けなくなるため。落とした項目も警告する）。local が明示した項目は落とさずに検証にかける。優先順位の並びが空になる条件は、local が並びを置き換えて埋める
- 未決: 共有の規則で必須の役に当たる担当が local の use で使えなくなるとき（例: codex を外すとマージに要る「現在の head への codex の有効な approve」が取れない）は、キー名つきの警告を出す。扱いは実装役が `needs-decision` で指揮役に上げる（自動で別の担当に回さない）

## マージキュー

マージキューとして Kodiak のホスト版（GitHub App。セルフホストはしない）を採用する。マージ待ち・ブランチ更新・マージ実行をキューに任せ、実装スレッドと指揮役がマージ完了まで張り付く負担をなくす。App のインストールとルールセットの変更は hatai の操作であり、リポジトリに届けるのは設定ファイルと文書である。

- 設定ファイル `.kodiak.toml`（リポジトリルート）: `version = 1`、`merge.automerge_label = "automerge"` と `merge.require_automerge_label = true`（ラベルが付いた PR だけがキューの対象）、`merge.method = "merge"`（マージコミット方式）、`merge.blacklist_labels = ["needs-human"]`（人間の判断待ちはラベルが付いていてもマージしない）。Dependabot のレビューなし自動マージ（`merge.automerge_dependencies`）は設定しない。設定項目の意味は https://kodiakhq.com/docs/config-reference
- `automerge` ラベルは「マージの指示」を表す PR ラベル。付けるのは指揮役だけで、実装スレッドは自分の PR に付けない（付け手と手順は skill `command-team` の「マージ」）
- ルールセットの要点: 必須チェック（`mise run verify`・`pr-size`）の維持、承認 1 件必須・Code Owners レビュー必須の解除（`pelway-review` の Approve で満たせる形）、「マージ前にブランチを最新化すること」の必須化。Kodiak はブランチ保護を満たした PR だけをマージするため、この組み替えが導入の前提になる
- 役割分担: レビュー担当は従来どおり Approve・Request changes を投稿する。指揮役は Approve と必須チェックの通過と、現在の head と同じ SHA の QA の合格コメントを確かめて（条件の正本は skill `command-team` の「マージ」。`[文書のずれ]` の nit が残っていないことも含む）`automerge` ラベルを付け、Kodiak のマージを確認してから `agent:working` を外して人間に報告する。Kodiak はブランチの最新化とマージを直列に実行し、コンフリクト等でマージできないときは PR にコメントして `automerge` ラベルを外す（巡回での検知と手当は runbook の「マージキュー待ちの PR」）。Kodiak の自動更新が取り込んだあとは、指揮役が取り込みのコミットに main 以外の差分がないことを確かめる。キューの切り戻し（Kodiak が応答しない等）も、hatai が例外と判断した PR の admin マージによる（例外は上の 3 つだけ）

ルールセットの変更前後の値（変更は hatai の GitHub 設定操作。「変更前」はこの文書の従来の記述からの転記で、pelway-dev のトークンではクラシックブランチ保護の読み取りができないため API の実測値ではない）:

| 設定 | 変更前 | 変更後 |
|---|---|---|
| 必須チェック | `mise run verify`・`pr-size` | 同じ（維持） |
| 承認 | Code Owners（`* @hatai`）のレビューが必須 | 1 件必須（Code Owners レビュー必須は解除） |
| マージ前のブランチ最新化 | 必須でない | 必須 |
| stale な承認の取り消し（新しい push で承認を無効化） | — | 有効化する（新しいコミットで古い Approve を外す。hatai の決定、Issue #480。手順書は入っている前提で書く） |
| `enforce_admins` | `false`（hatai の admin マージによる迂回が可能） | `false`（例外 PR の admin マージを残す） |
| リポジトリのルールセット `main`（id 26958258） | `deletion`・`non_fast_forward` のみ | 同じ（ブランチ保護の必須チェック・承認・最新化とは別に維持） |

## 状態の表し方

- 正本は GitHub Issues と GitHub Projects のボード（`users/hatai/projects/7`「Pelway agent team」）
- 指揮役スレッドのコンテキスト圧縮は、状態の正本が GitHub と巡回記録にあるため、外部化と復元検証を伴う手順として仕組み化できる。詳細は skill `command-team` の「コンテキスト管理」
- ボードの Status（Backlog・Todo・Working・In review・Changes requested・Ready to merge・Needs human・Done）が Issue の処理状態を表す。Status は巡回が導出規則（runbook の「ボードと巡回」）で補正した後の確定状態が正
- ラベルは `ready-for-agent`（人間が付ける入場の条件。ボードへの自動追加の合図で、着手後も外さない）、`agent:working`（割り当て中。指揮役が割り当てで付け、実装スレッドが着手時に自分の占有として引き継いで、誰が・何時（JST）に取ったかを Issue にコメントする。付いていなければ実装スレッドが付ける。すでに付いている Issue は取らない。手放すときは実装スレッドが外してコメントする。放置時の期限は未定。手順は skill `implement-issue` の手順 2）、`agent:devin`（Devin CLI に割り当てたときに指揮役が付ける）、`needs-human`（人間の判断待ち）の 4 つ。このほか PR には `automerge`（マージの指示。指揮役が付ける。Kodiak のキューの合図。「マージキュー」参照）がある
- 「レビュー待ち」「修正待ち」「マージ待ち」は PR の Draft 状態・レビュー状態・CI 結果と委託記録から、巡回が Status に反映する

## 見張り役と通知

エージェントの状態の通知は、見張り役 `pelway-agent-watch`（`crates/pelway-agent-watch`、pitchfork で常駐）が一元的に出す。エージェントごとのフック（OpenCode プラグイン、Codex / Claude の hooks）は使わない。

- 30 秒ごとに `t3 thread list --json` を読み、状態が変わったスレッドと実行中のスレッドを `t3 thread show` で詳しく見る
- 送るイベント（すべて即時対応）
  - `finished` / `failed` / `cancelled`: スレッドの実行が終わった
  - `waiting`: 承認や入力の待ちが発生した
  - `stalled`: 次のどれか。理由を `stall_reason` に入れる
    - `no-activity`: 実行中のまま 45 分更新がない
    - `ci-failed-twice`: `agent/` ブランチで verify が 2 回続けて失敗した（5 分ごとに `gh run list` で確かめる）
- payload は `pelway.agent-watch/v1`。スレッド ID、ブランチ、`issue_number`（`agent/<番号>-` から取る）、`round`（スレッドの実行回数）、`report`（最後の返答が `キー: 値` 形式 8 行以内のときだけ、その本文）を含む
- 同じ出来事からイベントを作るのは 1 回だけ。送れなかったものは送信待ちの列に残して送り直す（受け手には重複して届きうる）。受け手が直らない失敗（HTTP 400・404・413）を返したものは送り直さずに列から外す（受信箱には書かれる。受信箱に書けなかったものは、書けるまで状態に残して書き直す。上限 100 件で、超えたら古いものから捨てる）。送り直せる失敗が続く間は、送り直しの間隔を 30 秒から失敗のたびに倍にし、上限は 15 分（`docs/dev-environment.md`「pitchfork について」）。状態は `~/.local/state/pelway/agent-watch.json`（既定の場所。`XDG_STATE_HOME` を設定したときの場所は `docs/dev-environment.md`「pitchfork について」を参照）に置く
- 同じ知らせを、T3 のスレッドとして動く指揮役のための受信箱（共有の git dir の `pelway/conductor-inbox.jsonl`）にも 1 行ずつ書き、ホストの登録ファイルに登録された指揮役のスレッドへ決まった文を送って起こす（Issue #555。人間が立てたスレッドの知らせで起こすのは `failed`・`waiting`・`stalled`・報告のある `finished` のときだけで、それ以外は受信箱に書くだけ。Issue #727。形は `docs/agents/notify-payloads.md`「指揮役の受信箱」、動き方は `docs/dev-environment.md`「pitchfork について」）
- 修正が何回目かは、実装スレッドがレビューの委託回数で数える（skill `implement-issue`）。同じ head は 1 ラウンドに数え、コンフリクトの解消を含まない main の取り込みだけで head が変わっても新しいラウンドにしない。ラウンドの上限は 4 回で、round 3 の must-fix が残れば round 4 には入らず止まる（別 Issue に回せる領域の指摘は除く。分類はレビュー役が行う）。round 4 に入るのは hatai が例外と判断した PR だけ。レビューの effort は、セキュリティと仕様の PR では 1 回目から上げておく（3 回目で改めて上げる運用はやめる）。人間のレビューによる修正（指揮役が `t3 thread send` で伝える）の回数も同じ上限に数える。レビューの `[文書のずれ]` の nit だけを直す round（skill `review-pr`・`implement-issue`）は、この回数に数えない（その後のレビューの委託も数えない）。印の nit だけを直す round は、この PR で通算 2 回まで。印の nit が残っていて通算 3 回目が要るときは（その前のレビューの判定によらず）、3 回目には入らず、止まる条件としてエスカレーションする

## 停止・迷走への対処

- `stalled` を受けたら、Grok Bot は状況を要約して Issue にコメントし、`needs-human` を付けて部屋で人間に知らせる
- しきい値は `docs/agents/grokbot-runbook.md` で調整する

## GitHub からの通知

- `notify-grokbot.yml` が送るのは、PR の作成（`opened`・`ready_for_review`）と push（`synchronize`）、クローズ（マージを含む）、レビューの投稿（OWNER / MEMBER / COLLABORATOR のもの）、verify と nightly（main の総点検）の完了だけ
- payload に `urgency` を付ける。`digest`（まとめにだけ反映）は workflow_run の成功（verify・nightly）、Approve のレビュー、PR への push（`synchronize`）。それ以外は `immediate`。`synchronize` は同じ PR・同じ head sha では 1 回だけ送る
- Grok Bot は `digest` を 9:00 と 18:00（日本時間）の 1 日 2 回のまとめにだけ反映する。部屋で聞かれたときはその場で答える

## レビュー担当の権限

- レビュー担当は PR のコードを実行しない。PR の head は worktree に取り出さずに読み、verify と pr-size は PR のチェック（CI）の結果で判断する（PR のコードはレビュー担当の秘密鍵を読めるため）
- 通常の PR のレビューは、実装担当のスレッドが `delegate_task` でレビュー専用プロバイダーの子タスクとして委託する（`role: "review"`、権限は `auto-accept-edits`、worktree は独立 checkout。手順は skill `implement-issue`）。委託の子がレビュー専用プロバイダーの sandbox（ネットワークを GitHub だけに絞る権限プロファイル）を継承することを、`docs/dev-environment.md` の確認手順で確かめてから使う
- 指揮役が立てるレビュースレッド（指揮役自身の PR・外部投入）は、従来どおりトップレベルのスレッドとして worktree を main から立てる（runbook の「レビューを始める」）
- レビュースレッドは Codex のレビュー用インスタンス（`agent-team.toml` の `[agents.codex].instance.review`）で動かす。ネットワークは `api.github.com` と `github.com` だけに絞る（Codex の `network_proxy` と、レビュー担当専用の home `~/.codex-review/config.toml` の権限プロファイル `review`。T3 の MCP サーバーと Codex の GitHub 連携もこの home で止める。設定は `docs/dev-environment.md` の「レビュー担当の Codex」）
- レビュースレッドは T3 の権限 `auto-accept-edits`（Codex の `workspaceWrite`）で立てる。`full-access` ではコマンドが sandbox の外で動き、ネットワークの制限が効かない。この権限では `.git` が読み取り専用になるので、PR の側のファイルは GitHub API で読む
- Claude のレビュースレッドは Claude のレビュー用インスタンス（`agent-team.toml` の `[agents.claude].instance.review`）で動かす。skill `review-pr` をそのまま使い、使えるコマンドを読み取り系の `git` と `scripts/agent-gh review` に絞り、ネットワークを `api.github.com` と `github.com` だけに絞る。設定と確認の方法は `docs/dev-environment.md` の「レビュー担当の Claude」。確認が済むまでは使わない
- Devin CLI のレビュースレッドは Devin CLI のレビュー用インスタンス（`agent-team.toml` の `[agents.devin].instance.review`）で動かす。skill `review-pr` をそのまま使い、権限モードは Devin CLI 公式 sandbox の Autonomous 一択になる（`bypass` は使わない）。このインスタンスは起動ラッパーで `DEVIN_SANDBOX=true` と専用設定（ドメイン許可リスト・permissions deny）を付けて立ち上がり、GitHub 以外への通信とワークスペース外への書き込みを実行時に遮断する。仕組みと採用の条件（確認の記録）は `docs/dev-environment.md` の「レビュー担当の Devin CLI」
- 委託・起動で読んだ `[agents.<担当>].instance.review` は、main の最新の版の `docs/dev-environment.md`（読めなければ委託・起動しない）の「レビュー担当の Codex」「レビュー担当の Claude」「レビュー担当の Devin CLI」の節が名指すインスタンスと一致するものだけを使う。一致しなければ委託・起動しない
- Devin CLI が実装した PR には Devin CLI のレビューを付けない。同じモデル系列が自分の出力を採点する偏りを避ける
- 指揮役はレビューを兼ねない。指揮役が作った PR もレビュースレッドに回す
- CI 上で動かす助言セキュリティレビューの起動の認可と実行の隔離は `docs/agents/ci-security-review.md` に定める

## 実装担当の権限

- 実装スレッド（T3 の worktree スレッドで動く opencode と Devin CLI の実装用インスタンス（`agent-team.toml` の `[agents.devin].instance.implement`））は OS レベルの sandbox の中で動かす。遮断は `scripts/agent-sandbox/sandbox-shell`（bwrap）が行い、エージェント側の権限設定は補助層とする。仕組み・導入・確認の方法は `docs/dev-environment.md` の「実装担当の sandbox」
- opencode の実装スレッドは、グローバルプラグイン `pelway-sandbox` が全シェルコマンドを bwrap 経由にし、ファイルツールの worktree 外への書き込みと秘匿パスの読み取りを拒否する。適用は `~/.t3/worktrees/` 配下のセッションに限る
- Devin CLI の実装スレッドは、実装用インスタンスの `commandPath` に指定した `devin-impl` ラッパーが `devin acp` プロセス自体を bwrap で包んで起動する。Devin 内蔵の `--sandbox` は、exec の通信先が GitHub 系のみになる挙動のため実装スレッドでは使わない（レビュー用インスタンスとは別方式）。exec のコマンドは `.devin/hooks.v1.json` のフックでネストした sandbox に送り、`credentials.toml` 等の devin 用資格情報を agent のコマンドから分離する
- sandbox 内では、書き込みはセッションの worktree・共有 git dir・`/tmp`・ツールキャッシュに限られ、`sudo` 等の昇格はできず、`~/.ssh`・`~/.gnupg`・`/mnt` 等の秘匿パスは見えない。共有 git dir の `config`・`hooks/` は read-only のまま
- ネットワークは制限しない（hatai の判断。調査の自由度を優先し、現時点では許可リストにしない）
- 実装スレッドの T3 権限モード（runtimeMode）は sandbox と独立して選んでよい。bwrap が境界を担うため、承認プロンプトの有無がホストへの影響範囲を変えない
- 実装スレッドが sandbox を自身で無効化する経路は塞ぐ。リポジトリ同梱の `opencode.json`・`.opencode/`・`.devin/config.json` は sandbox が read-only でマウントし、worktree 内からプラグイン無効化や `.devin` の permissions/hooks の追加をできなくする

## 指揮役の運用ルール

- イベントごとの対応表は `docs/agents/grokbot-runbook.md`、毎サイクルの判断の手順は skill `command-team`
- Grok Bot のルーティンとメモリには「この runbook を読んで従う」とだけ書く
- 見張りと状態は、リポジトリのスクリプト（`pelway-agent-watch`、代行時は `scripts/conductor-watch.sh`）と `~/.local/state/pelway/` に置く。セッションの一時フォルダに置かない。巡回状態の補助記録 `patrol-state.json` と指揮役が持つ項目の記録 `patrol-items.json` は、sandbox 内の指揮役からも読み書きできるよう、リポジトリの共有 git dir（`git rev-parse --git-common-dir`）内の `pelway/` に置く（git の管理対象外。Issue #477）

## 改善ループ

- 対象は指揮役・実装担当（opencode＋GLM、Devin CLI＋SWE-2）・レビュー担当（Codex、Devin CLI、Claude）の両側
- 材料は、実装担当が残す実装者メモ（`docs/worklogs/issue-<N>.md`、skill `implement-issue` の手順 5）を一次情報とし、レビューの指摘・差し戻しとセッションの記録で裏を取る。食い違ったら成果物とセッションの記録を優先する
- 学びとルールは両方のツールが読む共有層（`AGENTS.md`、`.agents/skills/`、`docs/`、`agent-team.toml`）だけに書く。`.claude/skills/<name>` は `.agents/skills/<name>` へのシンボリックリンクにし、差分ゼロを確かめる
- 効果は GLM と SWE-2 の成果物でも測る（同じ種類のレビュー指摘・差し戻しの減少）。古い学び・矛盾する学びは改善のたびに直すか消す
- 手順は skill `command-team` の手順 4

## 導入の段階

1. リポジトリ側: AGENTS.md・skill・Issue Form・ラベル・PR サイズ検査・ブランチ保護・エージェント用アカウント
2. 見張り役と通知: 見張り役の常駐、旧来のフックの廃止、GitHub 通知の絞り込み
3. Grok Bot の runbook とレビュー担当の sandbox
4. 試運転: #59 と #62 を 800 行以下の積み重ね PR に分割する
