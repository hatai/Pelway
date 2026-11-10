# 作業ログ: GitHub の出来事を Grok Bot の webhook に送る workflow（platform-gh-notify-r1）

- 日付: 2026-11-10（火）JST
- 担当: Pelway 開発基盤エージェント（T3 Code / OpenCode + zai-coding-plan/glm-5.3）
- このラウンドでは push・PR 作成・実際の送信は一切行わない（担当が実施する）。

## 作業環境（最初に記録）

| コマンド | 出力 |
|---|---|
| `pwd` | `/home/hatai/.t3/worktrees/Pelway/demo-t3-platform-gh-not-ddd906b6716f0fb5` |
| `git branch --show-current` | `demo-t3/platform-gh-notify-r1` |
| `git rev-parse HEAD` | `7ab308edb8f9e351c1b6a37022931be3589cb4c9`（指示の base と一致） |

## 目的

GitHub の出来事（PR 系イベント・verify workflow の完了・手動実行）を、決まった 13 キーのメタデータ JSON として Grok Bot の webhook に POST する workflow を作る。本文・タイトル等の内容は送らない。実際の送信はこのラウンドでは一度もしない。

## 成果物

1. `.github/workflows/notify-grokbot.yml` — 新規（checkout なし・`permissions: {}`・Secret は `GROKBOT_WEBHOOK_URL` / `GROKBOT_WEBHOOK_AUTH`）
2. `docs/dev-environment.md` — 「GitHub 通知（Grok Bot webhook・notify-grokbot）」節を追加
3. `docs/worklogs/gh-notify-samples/` — イベント別の見本 payload 7件と `check.sh`
4. 本ログ

## 先行事例の比較（Prior art）

制約上、実装エージェントからは GitHub 検索（`gh search`）・Web 参照ができない（ネット接続禁止）。前例（`docs/worklogs/2026-11-10-platform-pr-ci-r1.md`）と同じく、既知の公開パターンの知識で比較し、オンライン再確認が必要な項目は「未検証」に列挙した。

| 案 | 内容 | 採否 |
|---|---|---|
| A. workflow 内で `jq` + `curl` を直接実行（通知系 workflow の最小構成） | 第三方 Action なし。`permissions: {}`・checkout なしで済む。payload のキーを workflow で完全に統制できる | **採用**（指示の構成と一致） |
| B. 通知系の定番公式 Action を使う（Slack 通知 Action のような `uses:` + `with:` パターン） | Action が payload 組立を担う | 不採用。カスタム schema（13 キー固定）に合わない。Action 参照の sha 固定（actions.lock）の管理が増える。`uses:` を1つも持たない構成にすれば jactionlint の unpinned-uses / actions.lock 対象もゼロになる |
| C. 一段挟む（repository_dispatch 経由・外部 webhook サービス経由） | 通知を別 workflow / 別サービスに委譲 | 不採用。二段構えで管理要素が増え、指示の「1 つの workflow」から外れる |

（検証は本 worktree 内のローカル実行のみ。実送信はしない。）

## workflow の構成（notify-grokbot.yml）

- トリガー: `pull_request`（opened / reopened / closed / synchronize / ready_for_review）、`pull_request_review`（submitted）、`pull_request_review_comment`（created）、`issue_comment`（created・PR 由来のみ）、`workflow_run`（workflows: `verify` ＝ verify.yml の `name:`、completed）、`workflow_dispatch`
- `permissions: {}`・checkout なし・`uses:` ゼロ（jactionlint の unpinned-uses / actions.lock 対象なし）
- ジョブ1つ（`ubuntu-24.04`・timeout 5分）: gate step（送る/送らないの判定）→ Send step（jq で payload 組立 → curl で POST）
- `${{ }}` はすべて step の `env:` にのみ記述。`run:` 内は env 変数とランナー既定 env（`GITHUB_EVENT_NAME` 等）のみ

## 迷った点と安全側の判断

1. **URL と鍵の渡し方 → `curl --config -`（標準入力の設定ファイル）を採用**。`-H @-` だとヘッダーは隠せても URL が argv に残るため、指示の「curl の引数やエラー出力に URL・鍵が出ないようにする」を完全に満たすのは `--config`。設定ファイルの引用符構文に合わせて値中の `\` と `"` をエスケープ。curl の stderr は破棄し、ログには HTTP ステータス（失敗時は curl 終了コード）のみを出す。
2. **concurrency を付けた**（指示では任意）。理由: (a) jactionlint の concurrency-limits 規則に引っかかると CI の verify ジョブ（全 workflow を検査）が落ちる、(b) 同一PRへの通知の乱発を直列化する意味もある。`cancel-in-progress` は **false 固定**: true にすると短時間に続いた異種イベント（review submitted の直後に comment created 等、同一PRグループ）の古い通知が逸失する。通知は出来事ごとに 1 通届くべきなので逸失は許容しない。グループは PR 番号 / 対象 run id / run id（その他）。
3. **jactionlint の `dangerous-triggers` ×2（issue_comment・workflow_run）は inline ignore で抑制**。この2トリガーは指示で必須。危険性の緩和は実装側で済ませている: `permissions: {}`・checkout なし・PR コードの実行なし・fork 由来は gate で skip・payload には自由記述テキスト（タイトル・本文等）を含めない。なお ignore コメントは**報告行と同じ行**に置く必要があった（直前のコメント行では効かないことを実行して確認）。
4. **issue_comment の fork 扱い**（指示どおりの記録）: payload だけでは当該 PR が fork 由来か判定できない。GitHub API を呼ばず `permissions: {}` を維持するため、`head_branch` / `head_sha` は **null のまま送る**。`pr_number` は `issue.number`（PR のコメントでは PR 番号と一致）を使用。
5. **Secret 未設定・fork は gate step で処理してジョブを成功終了**（job if にしない）。job 全体の `if:` だと結論が skipped になり「何も送らずに成功で終える」の見た目が変わるため。job if は issue_comment の PR 以外（ジョブごと skip）のみ。
6. **workflow_dispatch の解釈**: 「手動実行で `event: test` を送る」を、JSON の `event` キーを `"test"` にする（`action` は null、`pr_number` は null）と解釈した。
7. **`MERGED` の env 描画の保険**: GitHub は env 内で boolean を `true`/`false` 文字列、null を空文字列に描画する想定で実装。万が一 `false` が空文字列に描画されても `--argjson "${MERGED:-null}"` は `null` に落ちるだけで JSON は壊れない（安全側）。
8. **run_url**: workflow_run のときは対象 run の `html_url`、それ以外は `run:` 内でランナー既定 env（`$GITHUB_SERVER_URL/$GITHUB_REPOSITORY/actions/runs/$GITHUB_RUN_ID`）から組み立て（`${{ }}` を使わないので run: に直接書ける）。
9. **runs-on は `ubuntu-24.04`**（verify.yml に合わせ、ubuntu-latest 不使用）。timeout は 5分。
10. **gate step の fork 判定は bash で実装**（`GITHUB_EVENT_NAME` / `GITHUB_REPOSITORY` の既定 env を使用）。`github.event.pull_request.head.repo` が null（削除済み fork 等）のときも「≠ 自リポジトリ」と判定されて skip される安全側。
11. AGENTS.md の「Prior art first」（`gh search repos/code`）は、本タスクの gh・ネット接続禁止と衝突するため実施不可。前例 worklog（platform-pr-ci-r1）と同じく既知パターンのオフライン比較（上表）に切り替えた。この判断をここに記録する。

## 検証コマンドと結果（実行済み・この worktree 内のみ。curl は実行していない）

| コマンド | exit code / 結果 |
|---|---|
| `command -v actionlint` | 見つからず（exit 1）→ **actionlint 単体では未実行** |
| `~/.local/share/mise/installs/jactionlint/2.5.0/jactionlint .github/workflows/notify-grokbot.yml` | **exit 0**（初回は concurrency-limits 1件 + dangerous-triggers 2件で exit 1 → 下記「指摘への対応」のとおり修正後に 0。注記: shellcheck 未導入のため run スクリプトの shellcheck 検査はスキップ — verify.yml と同じ状態） |
| 同上・引数なし（CI の verify ジョブと同じ全 workflow 検査） | **exit 0** |
| `bash docs/worklogs/gh-notify-samples/check.sh` | **exit 0**（7 サンプルすべて PASS） |
| `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/notify-grokbot.yml'))"` | OK（yaml_ok） |
| `run:` 内の `${{ }}` 有無（awk で run ブロックを抽出して検査） | なし（一致 1件は YAML のコメント行のみ） |
| 行末空白（`grep -n ' $'`）・最終改行（`tail -c1`） | 行末空白なし・全ファイルに最終改行あり |

jactionlint は actionlint と同じ検査をするリポジトリピンの fork（`mise.toml` で 2.5.0。CI の verify ジョブが全 workflow に実行する）。mise 自身の自動解決に絡むネット接続を避けるため、前例（platform-pr-ci-r1 追記6）と同じくインストール済みバイナリを直接実行した。

### check.sh の確かめ方（見本 payload での jq 検証）

1. `docs/worklogs/gh-notify-samples/*.json` に7イベント分の見本 payload（`github.event` 相当・架空値。`example-org/pelway` 等）を用意した。本文・タイトル等、送ってはいけない欄には `*-MUST-NOT-APPEAR` のマーカー文字列を入れた。
2. 各見本から workflow の `env:` 式と同じ対応（対応表は check.sh 冒頭コメントに明記）で env 値（`REPO` / `EVENT` / `ACTION` / `PR_NUMBER` / `HEAD_BRANCH` / `HEAD_SHA` / `MERGED` / `CONCLUSION` / `WORKFLOW_NAME` / `RUN_URL` / `ACTOR`）を jq で作り、workflow の "Send notification" step の jq 部分を**逐語コピー**した `build_payload()` を同じ env 名で実行する。
3. 確認項目: (1) 出力 JSON のキーが13個だけ、(2) `MUST-NOT-APPEAR` マーカーの混入なし、(3) `schema` 固定値と `occurred_at` の ISO-8601 UTC 形式、(4) イベントごとの期待値（null の扱い、`merged` の true/false、`run_url` の切り替え、dispatch の `event=test`）。
4. curl は一切実行していない（build_payload は jq のみ）。

出力例（`pull_request.closed-merged.json`）— キー13個・本文等を含まない:

```json
{
  "schema": "pelway.github-notify/v1",
  "repo": "example-org/pelway",
  "event": "pull_request",
  "action": "closed",
  "pr_number": 42,
  "head_branch": "feature/demo",
  "head_sha": "3333333333333333333333333333333333333333",
  "merged": true,
  "conclusion": null,
  "workflow_name": null,
  "run_url": "https://github.com/example-org/pelway/actions/runs/123456789",
  "actor": "example-user",
  "occurred_at": "2026-11-10T10:27:59Z"
}
```

（`occurred_at` は実行時刻により変動。check.sh は形式を正規表現で検査。なお check.sh の env 導出は jq の `//`（空文字列を真とする）を使い、GitHub 式の `||`（空文字列を偽とする）と微妙に挙動が違うが、見本の値域では等価。）

## docs と code の整合（AGENTS.md「Code over docs」）

- `docs/dev-environment.md` に本 workflow の節（Secret 名・登録手順・送る JSON のキー・送らない条件。値は書かない）を追加した。
- 確認した範囲で既存ドキュメントとコードの不整合は見つからなかった: `workflow_run.workflows` は verify.yml の `name: verify` と一致、jactionlint ピン 2.5.0 は `mise.toml` と一致、verify.yml の content（checkout v4.2.2 / mise-action v2.4.4 / `cache: false`）は dev-environment.md 記載と一致。

## 守った制約

- worktree の外への書き込みなし（一時ファイル作成もなし）。`rm -rf` なし。
- 意図的なネット接続なし: webfetch / websearch / webReader / execute / MCP 不使用、curl / wget 不使用、`gh` 不使用、git push / fetch なし。
- 製品コード・他 workflow・`.octocov.yml`・`Cargo.lock` 不変更（`git status` で確認のとおり）。ツールのインストール・グローバル設定の変更なし。
- 実際の送信は一度もしていない。Secret の値はこの worktree に存在しない。

## 使った skills

- ハーネスの `verification-before-completion`（完了宣言前に読み込み適用。上表の検証コマンドは適用後の新規実行によるもの）。
- リポジトリ導入済み `.agents/skills/`（Rust 系 m01〜m15 等）に CI workflow YAML 作成に該当するものはなし。サブエージェント不使用。

## 未検証（合格としない）

1. GitHub 上での実際の workflow 実行（トリガー発火・skip 判定・concurrency 挙動）。このラウンドでは push も PR 作成もしていない。
2. 実際の webhook 送信。Secret（`GROKBOT_WEBHOOK_URL` / `GROKBOT_WEBHOOK_AUTH`）の登録状況・実在も未確認（公開リポジトリのため、この worktree に値は無い）。
3. ランナー（ubuntu-24.04）上の curl での `--config -`（標準入力の設定ファイル）と `header = "..."` 構文の実機挙動。ローカルでは curl を一切実行していない。
4. `${{ github.event.pull_request.merged }}` の env 描画（`false` → `"false"` の想定。万一空文字列なら null に落ちる設計にはしてある）。
5. `issue_comment` 由来の通知で fork 判定ができないまま送られること（4の設計判断どおり。受け手は `head_branch`/`head_sha` が null の通知を fork 由来の可能性として扱う必要がある）。

## コミット

- メッセージ: `ci: add Grok Bot webhook notification workflow for GitHub events`
- 対象: 本 workflow・dev-environment.md・本ログ・gh-notify-samples/ 一式

## 事故記録（正直な記録）

- `git commit` がリポジトリ標準の hk pre-commit フック（`mise x -- hk run pre-commit --from-hook`）を発火させ、その中で mise が rust 1.96.0 の channel sync と rustup self-update チェックを行った（"info: syncing channel updates"・"checking for self-update"。成分はすべて up to date で新規ダウンロード無し・`mise.lock` の変化無し）。前例 worklog（platform-pr-ci-r1 追記4・5）と同種の「サンクションドコマンド中の mise 副作用」であり、意図的なネット接続はしていないが通信の可能性は記録する。フック本体の検査（trailing whitespace / 最終改行）はすべてパス。

---

# 追記（r1 + 追加受入条件）: コメント投稿者の権限ゲートと fork/外部見本 — 2026-11-10（リード決定 19:10 JST を反映）

- 指示: 上記5件の追加受入条件（同一ブランチで継続。r1 の作業は破棄せず、本節の変更は `a841ecb` の上への別コミット）。
- 実送信・push は引き続きゼロ。

## 変更内容

1. `.github/workflows/notify-grokbot.yml`
   - job レベル `env.COMMENT_ALLOWED` 式を追加。issue_comment / pull_request_review / pull_request_review_comment の3イベントでは、投稿者の `author_association`（comment または review オブジェクトのもの）が `OWNER` / `MEMBER` / `COLLABORATOR` のいずれか、または `user.login` が `github-actions[bot]` のときだけ `"true"`。3イベント以外は常に `"true"`（投稿者判定なし）。
   - Send step の `if:` を `steps.gate.outputs.should_send == 'true' && env.COMMENT_ALLOWED == 'true'` に拡張（判定は if: のみ。`run:` への `${{ }}` なし）。
   - 「Note comment-author skip」step を追加: 投稿者ゲートで送らないとき、理由（`author_association` とログイン — 公開情報のみ）を notice ログに 1 行残して成功で終える。
2. `docs/dev-environment.md` — 「送らない条件」に投稿者判定を追記し、`github-actions[bot]` 条件が実質動かない見込みの補足を追加。
3. `docs/worklogs/gh-notify-samples/` — 見本 3件追加（`issue_comment.created-external.json`＝`author_association: NONE`、`pull_request.opened-fork.json`、`workflow_run.completed-fork.json`）。既存のコメント系 3見本に `author_association`（OWNER / MEMBER / COLLABORATOR）を追加。
4. `check.sh` — `gate_decision()`（job if / gate step の fork 判定 / `COMMENT_ALLOWED` / Send if: と同じ判定の再現）を追加。全 10見本の送る/送らない判定と、送る 7見本の payload を検査。

## 追加受入条件への対応（番号は指示どおり）

| # | 条件 | 対応 |
|---|---|---|
| 1 | コメント系3イベントの投稿者ゲート（判定は if:・`run:` に `${{ }}` なし） | job env の `COMMENT_ALLOWED` 式 + Send step の `if:`。job の if: にしなかったのは「何も送らずに成功で終える」のため（job if だと結論が skipped になる）。`github-actions[bot]` 条件は指示の補足どおり、GITHUB_TOKEN 起点のコメントでは workflow 自体が起動しないため実質動かない見込み。条件のみ残した |
| 2 | fork 除外は PR 系 `pull_request.head.repo.full_name` と `workflow_run.head_repository.full_name` の両方で | r1 実装のまま（gate step で `HEAD_REPO != GITHUB_REPOSITORY` を4イベントに適用）。head repo が null（削除済み fork 等）でも ≠ となり安全側で skip |
| 3 | `pull_request_target` 不使用・checkout / `uses:` 不使用 | 従来から不使用。grep で確認（`uses:`・`pull_request_target` の実使用なし。一致は「checkout なし」等のコメント行のみ） |
| 4 | URL/鍵を echo しない・`set -x` / `curl -v` 不使用・Secret 無しと curl 失敗で exit 0 | 従来どおり（grep で確認: `set -x` なし・`curl -v` なし・WEBHOOK 変数の echo なし）。Secret 無し → gate step が `should_send=false` で成功終了。curl 失敗 → Send step は非 0 を返さない（`--max-time 10`・stderr 破棄・HTTP ステータスのみログ） |
| 5 | 外部コメント（NONE）と fork の PR・workflow_run の見本 + 送らない判定の確認 | 見本 3件追加。`check.sh` のゲート判定で 3件とも `skip` になることを確認（下表）。送信はしていない |

## 迷った点と安全側の判断（追記分）

1. **判定の実装場所**: 指示は「job か step の if:」。job の if: だと結論が skipped になり「成功で終える」から外れるため、Send step の if: にした。条件式の実体は job レベル `env.COMMENT_ALLOWED` の式展開で、`run:` への `${{ }}` はゼロのまま。
2. **author_association の参照先**: 「投稿者」＝コメント/レビューの作成者と解釈。issue_comment と review comment は `comment.author_association`、review は `review.author_association` を使用（`issue.author_association` は PR 起案者でありコメント投稿者ではないため不使用）。
3. **pull_request イベントへの投稿者ゲートは未適用**: 指示はコメント系3イベントのみ。pull_request 系は fork 除外で外部由来を実質排除済み（同一リポジトリに branch を push できるのは書き込み権限保有者のみのため、同一 repo の PR 作成者は最低 COLLABORATOR）。
4. **スキップ理由の notice**: 判定漏れの調査ができるよう、送らない理由を notice 1行で残した（載せるのは `author_association` とログインという公開情報のみ）。

## 検証コマンドと結果（追記分・すべて実施済み・送信なし）

| コマンド | exit code / 結果 |
|---|---|
| `~/.local/share/mise/installs/jactionlint/2.5.0/jactionlint .github/workflows/notify-grokbot.yml` | **exit 0**（新規の `COMMENT_ALLOWED` 式・notice step・拡張 if: を含む。shellcheck 未導入の注記のみ） |
| 同上・引数なし（CI の verify と同じ全 workflow 検査） | **exit 0** |
| `bash docs/worklogs/gh-notify-samples/check.sh` | **exit 0** — ゲート判定 10件（送る 7・送らない 3: `issue_comment.created-external` / `pull_request.opened-fork` / `workflow_run.completed-fork`）＋ payload 組立 7件、すべて PASS |
| `python3 -c "import yaml; yaml.safe_load(...)"` | OK |
| `grep -nE 'set -x\|curl .*(-v\|--verbose)\|pull_request_target\|uses:\|checkout' .github/workflows/notify-grokbot.yml` | 実使用なし（一致 2件は「checkout なし」等のコメント行のみ） |
| `grep -n 'echo.*WEBHOOK' .github/workflows/notify-grokbot.yml` | なし（URL・鍵の echo なし） |
| `run:` 内の `${{ }}`（awk 抽出・コメント行を除く） | なし |

## 未検証（追記分も合格としない）

1. GitHub 上での `author_association` の実際の値の分布と判定（`FIRST_TIMER` / `CONTRIBUTOR` 等は通知されない）。実データでは未確認。
2. `github-actions[bot]` 条件の実効性（GITHUB_TOKEN 起点のコメントは workflow が起動しない見込み — 指示の補足どおり条件のみ残している）。→ **追記2で条件ごと削除（下記）**
3. GitHub 上での run・実際の送信は引き続き未実施（push は担当が行う）。

---

# 追記2（r1）: 投稿者ゲートから github-actions[bot] 例外を削除 — 2026-11-10（リード決定 19:13 JST）

- 指示: 前追加条件1の訂正。`github-actions[bot]` を例外として送る条件は入れない（入れていたら外す）。コメント系3イベント（issue_comment / pull_request_review / pull_request_review_comment）は `author_association` が `OWNER` / `MEMBER` / `COLLABORATOR` のときだけ送る。octocov の結果は verify の `workflow_run` 通知で知らせるので十分、のため。
- 対応コミット: `9eb3f8d` の上への別コミット。実送信・push は引き続きゼロ。

## 変更内容

1. `.github/workflows/notify-grokbot.yml`
   - `COMMENT_ALLOWED` 式から `github.event.comment.user.login == 'github-actions[bot]'` と `github.event.review.user.login == 'github-actions[bot]'` の2条件を削除。コメント系3イベントの判定は `author_association` の3値のみになった。
   - notice step のメッセージから「(and not github-actions[bot])」を削除。トリガー行・Send step・env ブロックのコメントを訂正版の内容に更新（env コメントに 19:13 訂正の経緯を明記）。
   - `COMMENTER_LOGIN`（job env）は判定には使わず、スキップ時の notice の説明用として残した（公開情報のみ）。
2. `docs/worklogs/gh-notify-samples/check.sh` — `gate_decision()` から bot 例外（login 比較）を削除し、`comment_allowed` は `author_association` の3値のみで判定。ヘッダコメントも更新。
3. `docs/dev-environment.md` — 「送らない条件」の投稿者判定の記述を3値のみに修正。bot 例外が実質動かない見込みだった旨の補足を削除し、「ボット例外なし。octocov 等の結果は verify の workflow_run 通知で把握する」と訂正理由を記載。
4. 見本 payload は変更なし（bot 例外に依存する見本は無かった。`NONE` コメントは引き続き skip、OWNER / MEMBER / COLLABORATOR の3見本は引き続き send）。

## 検証コマンドと結果（追記2分・すべて実施済み・送信なし）

| コマンド | exit code / 結果 |
|---|---|
| `~/.local/share/mise/installs/jactionlint/2.5.0/jactionlint .github/workflows/notify-grokbot.yml` | **exit 0**（shellcheck 未導入の注記のみ） |
| 同上・引数なし（全 workflow） | **exit 0** |
| `bash docs/worklogs/gh-notify-samples/check.sh` | **exit 0** — ゲート判定 10件（送る 7・送らない 3）＋ payload 7件、すべて PASS |
| `python3 -c "import yaml; yaml.safe_load(...)"` | OK |
| `grep -n 'github-actions\[bot\]'`（workflow / check.sh / dev-environment.md） | 機能的な使用なし（一致 2件は訂正経緯を説明するコメント行のみ） |

## 迷った点（追記2分）

- 特になし（条件の削除のみ）。`COMMENTER_LOGIN` を残すか迷ったが、スキップ理由の notice から投稿者を特定しやすいため説明用に保持した（判定経路には入っていない）。

## 未検証（追記2分も合格としない）

1. GitHub 上での実際の run・送信（引き続き push は担当が行う）。
2. 実データでの `author_association` の分布（`CONTRIBUTOR` 等の内部協力者も通知対象外になることの確認）。

---

# 直し1: issue_comment trigger と concurrency を削除 — 2026-11-10（リード決定 20:15 JST）

- 開始: 2026-11-10 20:25 JST
- Codex レビューの必須指摘2件への対応（リードが 20:15 JST に直し方を決定）。同一ブランチ（fix1）で継続。実送信・push は引き続きゼロ。
  1. fork から来た PR に OWNER / MEMBER / COLLABORATOR がコメントすると、issue_comment では fork かどうか分からないため Send に届いてしまう → trigger から `issue_comment` を外す（`permissions: {}` はそのまま）
  2. `concurrency` のグループで、待っている run が新しい run に置き換えられ通知が抜ける → `concurrency` を丸ごと消す

## 作業環境（最初に記録）

| コマンド | 出力 |
|---|---|
| `pwd` | `/home/hatai/.t3/worktrees/Pelway/demo-t3-platform-gh-not-7fc68e92ad361f68` |
| `git branch --show-current` | `demo-t3/platform-gh-notify-r1-fix1` |
| `git rev-parse HEAD` | `0e2d5b465383de47c7070c71dc866b1c120c9ad1`（指示の base と一致） |
| `git status --porcelain` | 空（指示どおり） |

## 変更内容

1. `.github/workflows/notify-grokbot.yml`
   - `on:` から `issue_comment`（`# jactionlint ignore=dangerous-triggers` の注記ごと）を削除
   - job の `if:`（issue_comment を `github.event.issue.pull_request` の有無で PR 由来に絞る式）を削除（trigger が消えたため恒真になる）
   - `COMMENT_ALLOWED` 式から `github.event_name != 'issue_comment' &&` の分岐を削除。コメント系2イベント（pull_request_review / pull_request_review_comment）の投稿者制限（OWNER / MEMBER / COLLABORATOR）はそのまま
   - `PR_NUMBER` 式から `github.event.issue.number`（issue_comment 専用の参照）を削除
   - env ブロックのコメント「コメント系3イベント」→「コメント系2イベント」に更新
   - `concurrency:` ブロックと説明コメントを丸ごと削除
   - 変更なし: `permissions: {}`、Secret 未設定の gate、fork（HEAD_REPO 別 repo）の gate、13 キーの jq、`curl --config -`
   - ファイル内に `issue_comment` / `concurrency` の文字はゼロ（grep・check.sh で確認）
2. `docs/dev-environment.md` — 通知の節
   - 対象イベントから「PR への issue_comment」を削除
   - 投稿者ゲートの対象を pull_request_review / pull_request_review_comment の2イベントに修正
   - 「PR 以外の issue コメント（ジョブごと skip）…」の行を削除し、代わりに「対象外」の文を追加: PR の会話欄に書いた普通のコメント（issue_comment）は通知しない。fork から来た PR へのコメントを安全に見分けられないため。あとで足す場合は `pull-requests: read` で PR を調べて fork を除く形にし、別の PR で行う
3. `docs/worklogs/gh-notify-samples/`
   - `issue_comment.created-external.json` / `issue_comment.created-pr.json` を `git rm`
   - `check.sh`: issue_comment 2見本のゲート判定・payload 組立を削除（ゲート判定 10→8、payload 7→6）。`gate_decision()` の case から issue_comment を削除、`PR_NUMBER` 導出から `.issue.number` を削除、ヘッダー・件数コメントを更新。workflow が `issue_comment` / `concurrency` を持たないことの grep 検査を追加
4. 本ログに本節を追記

## 検証コマンドと結果（すべて実施済み・送信なし）

| コマンド | exit code / 結果 |
|---|---|
| `bash docs/worklogs/gh-notify-samples/check.sh` | **exit 0** — workflow 静的検査（issue_comment / concurrency なし）＋ゲート判定 8件（送る 6・送らない 2: fork×2）＋ payload 組立 6件、すべて PASS |
| `~/.local/share/mise/installs/jactionlint/2.5.0/jactionlint .github/workflows/notify-grokbot.yml` | **exit 1** — concurrency-limits 1件（下記「迷った点」1。shellcheck 未導入の注記付き） |
| 同上・引数なし（CI の verify と同じ全 workflow 検査） | **exit 1** — 同上（notify-grokbot.yml の concurrency-limits 1件のみ） |
| `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/notify-grokbot.yml'))"` | OK |
| `grep -n 'issue_comment\|concurrency' .github/workflows/notify-grokbot.yml` | なし（exit 1） |
| `grep -nE 'set -x\|curl .*(-v\|--verbose)\|pull_request_target\|uses:\|checkout'`（workflow） | 実使用なし（一致1件は「checkout なし」の注記コメント行のみ） |
| `grep -n 'echo.*WEBHOOK'`（workflow） | なし |
| `run:` 内の `${{ }}`（awk 抽出） | 実使用なし（一致1件は注記コメント行のみ） |
| 行末空白・最終改行（変更4ファイル） | 行末空白なし・最終改行あり |

## 迷った点と安全側の判断（直し1分）

1. **jactionlint の concurrency-limits が再燃し exit 1**。concurrency の削除はリード決定だが、本 workflow の r1 では「concurrency-limits 規則に引っかかると CI の verify ジョブが落ちる」ことを理由に concurrency を付けていた（本ログ・追記前の迷った点2 (a)）。今回の削除でこの指摘が復活し、push 後は CI の verify ジョブ（jactionlint で全 workflow を検査）が落ちる状態になる。対応は3通り考えた: (a) `on:` 行に `# jactionlint ignore=concurrency-limits` を付ける — 指示の完了条件「ファイルの中に `issue_comment` と `concurrency` の文字が残らないようにする」に違反するため不採用、(b) verify.yml 側の jactionlint 実行に ignore を追加 — 「ほかの workflow を変えない」禁止に違反するため不採用、(c) concurrency を戻す — リード決定に反するため不採用。字面の指示をすべて守った結果として exit 1 が残ることを本節と報告に明記し、判断はリードに委ねる。
2. **`github.event.issue.number` を PR_NUMBER から削除**。指示の「`issue_comment` に関わる条件・コメント・gate の case・`COMMENT_ALLOWED` の式の分岐をすべて消す」に、PR_NUMBER 式の `issue.number`（issue_comment 専用の参照）も含まれると解釈した。残しても動作には影響しないが、issue_comment 依存が残るため削除した（check.sh の対応導出も同様）。
3. **job の `if:` を丸ごと削除**。この if は issue_comment を PR 由来に絞るためだけのもの。trigger から issue_comment が消えた時点で恒真になるため、コメントごと削除した。
4. **`docs/dev-environment.md` の「2026-11-10 時点で GitHub 上での実際の run は未確認」はそのまま残した**。この文は CI（verify.yml）節にあり、verify.yml も notify-grokbot も今回時点で GitHub 上での run は未確認のまま（push は担当が行うため）。「未確認のままが正しい」状態のため修正不要と判断した。
5. **コメント系の skip 経路を見本で検証できなくなった**: author_association が基準外（NONE 等）のコメント系見本は issue_comment.created-external.json だけだったため、削除後に check.sh で COMMENT_ALLOWED の skip 判定を直接検証する見本が無くなった（fork 2件の skip は検証する）。trigger から issue_comment が消えた以上、同イベントの見本を残す意味がないため指示どおり削除した。投稿者ゲートの式自体は pull_request_review / pull_request_review_comment 見本（send 側）で確認できる。

## コミット

- メッセージ: `fix(ci): drop issue_comment trigger and concurrency from gh-notify`
- hk pre-commit の結果はコミット実行後に本節へ追記する。

## 未検証（直し1分も合格としない）

1. GitHub 上での実際の run・送信（push は担当が行う）。※ jactionlint の concurrency-limits 指摘が残るため、push 後は CI の verify ジョブが落ちる見込み（迷った点1）。





