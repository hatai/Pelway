# Issue #481 の作業ログ

## 実装者メモ（round 1）

この round は `.github/workflows/` の変更を含むため Issue の進め方どおり push せず停止（対象 head は書かない）。実装コミットは `7d64771a172e007ff5aa622d25bc11eb004836b5`、ブランチは `agent/481-notify-pr-events`。

1. 詰まった箇所: `docs/worklogs/gh-notify-samples/check.sh`: main の workflow（`pelway.github-notify/v2`・14 キー）に対し check.sh が v1（13 キー・`pull_request_review_comment` trigger 前提・`REVIEW_STATE` env なし）のまま残っていた。v2 への差分（`urgency` キー・urgency 判定・`REVIEW_STATE`）を新設して追いつかせた。shellcheck の SC1123（`elif` の前に disable ディレクティブを置けない）に一度引っかかり、`if` ブロック全体の前に移して解消。
2. 曖昧だった文: 「`urgency` の分け方（`opened` / `ready_for_review` / `synchronize` を `immediate` にするか `digest` にするか）を決めて、文書に書く。」（Issue #481「変更内容」4）
3. 仮定と解釈: 新しい 3 イベントは `immediate` とした。見張りを平日 12 時・18 時に減らした分を通知で補うのが目的で、PR の動き（レビュー再委託・マージ判断に関わる）は発生時に知る必要があるため。`synchronize` を `digest` にする案（push は頻度が高い）は、各 push が別の head を持ち遅れると判断が古い head に対して行われるため選ばなかった。スキーマはキー追加が不要（番号・head sha・ブランチ・状態・実行者は既存 14 キーで表現できる）なので `pelway.github-notify/v2` のまま（「キーを増やす必要が出たら」に該当しないと判断）。`pull_request_review_comment.created.json` は現行 workflow の trigger に無いイベントの見本で `send` を期待する stale な検査だったため削除（「新しいイベントに合わせて更新」の範囲と解釈）。fork 由来・Secret 未設定のスキップは既存の gate step がそのまま効くためコード変更なし・文書に明記した。
4. 足りなかった情報・検証手段: 実際の GitHub イベントで `github.event.pull_request.head.repo.full_name`・`github.event.review.state` 等の式が解決される様子はローカルでは試せない（`workflow_dispatch` での実送信確認は Secret が必要）。check.sh の逐語コピー方式での静的な一致確認までが手元でできる検証。
5. 改善案: `docs/worklogs/gh-notify-samples/check.sh` の冒頭コメントに「workflow の `on:`・env 式・Send step の jq を変えたら、このファイルの逐語コピー・`expected_keys`・env 導出も合わせて更新する」と足す（v1→v2 更新時に取り残された前例があるため）。

使ったスキル: `implement-issue`（`.agents/skills/implement-issue`）。輸入スキル `tdd` は試行の指定がないため未使用。

## 実装者メモ（round 2）

この round は `.github/workflows/` の変更を含むため Issue の進め方どおり push せず停止（対象 head は書かない）。ブランチは `agent/481-notify-pr-events`、実装コミットはこのメモを含むコミットの親。濱田さんの決定（issuecomment-7014084933）どおり、`synchronize` を `immediate` から `digest` に変え、同じ PR・同じ head sha では 1 回だけ通知する dedup を足した。

1. 詰まった箇所: この worktree は exec の sandbox 内でファイルシステム全体が read-only（`touch` がどの階層でも EROFS）。`mise` がリポジトリ直下の `.mise.lock` を書けず `mise exec` が失敗したため `MISE_LOCKFILE=0` で抑止し、`cargo` の `target/` 作成も失敗するため `CARGO_TARGET_DIR=/tmp` に切り替えて `mise run verify` を実行した。check.sh の dedup ゲート呼び出しでフラグ値に `true` ではなく `dup` を渡して 1 度 FAIL（テストが意図どおり検知した）。
2. 曖昧だった文: 「同じ PR・同じ head sha では1回だけ通知するように直す」（Issue #481 コメント 7014084933）
3. 仮定と解釈: dedup は `synchronize` の送信だけを抑止し、「送った」判定は全 `pull_request` run の `Send notification` step 成功で見る、と解釈した。全イベントに dedup を掛ける解釈は、Draft で開いたあと push せず ready にした場合（同じ head sha）に `ready_for_review` が落ちてレビュー開始の合図が消えるため選ばなかった。dedup の手段は `uses:` の Action を足せない（`actions.lock` の再生成は人間名義の `gh actions-lock` が必要・この workflow は意図的に uses ゼロ）ため、`permissions: actions: read` と `gh api` で過去 run を照合する方式にした（QA の「permissions が最小」= この目的に必要な最小の読み取り権限と解釈）。同じ head の run の競合は job レベルの concurrency（PR+sha 単位・直列・cancel しない）で潰す。照合失敗時は送る側に倒す（重複は既存の許容、取りこぼしは新たな損失のため）。
4. 足りなかった情報・検証手段: エンジニアチームの推奨の原文（どの dedup 方式を想定していたか）が GitHub 上に無く、判断の根拠はコメントの文言と制約の消去法に限定された。`pull_requests` フィールドが空になる run が多いことは実リポジトリの API 応答で確認済み（照合は `head_branch`+`head_sha` で行うが、埋まっている run では `pull_requests[].number` でも照合する。同じブランチを別 PR に再利用した稀なケースで旧 PR の送信記録に新 PR の通知を落とされないため）。dedup の API 照合そのものは実イベントが無いと手元では試せない。Codex レビューが 2 件の実害を見つけた: `gh api --jq` は jq の `--arg`/`--argjson` を受け付けず照合が必ず fail-open に倒れていた（`gh api | jq` のパイプ形＋`pipefail` に修正。実リポジトリの API 応答で照合クエリを動作確認した）、rerun は同じ run id の新 attempt で走り status=completed の一覧に自分が載らないため過去 attempt の送信を見落として再通知していた（自分の run を `jobs?filter=all` で別照合し、候補 run の job 照合にも `filter=all` を追加）。
5. 改善案: `docs/worklogs/gh-notify-samples/check.sh` の gate_decision 節に、workflow の `Send notification` step 名を変えたら dedup の照合（step 名を文字列で参照）も壊れる旨を 1 行足す（step 名依存は暗黙の結合点のため）。

使ったスキル: `implement-issue`（`.agents/skills/implement-issue`）。輸入スキル `tdd` は試行の指定がないため未使用。

## 実装者メモ（round 2 の QA 送り返し対応）

指揮役（代行）からの修正指示への対応。push しない前提は変わらない。

- must-fix（job concurrency）: `notify-<PR>-<sha>` のグループは pull_request_review など全イベントに効き、同一グループで待てるのは 1 run だけのため続く run の保留中が取り消され通知が落ちていた。QA の案どおり `github.event.action == 'synchronize'` のときだけ `notify-<PR>-<sha>` グループにし、他イベントは `github.run_id` 単位に変更。コメントも「同じ sha の synchronize が重なったとき待っていた古い run が取り消されるのは dedup 対象同士なので許容」に合わせて修正。nit として照合が `per_page=100` の 1 ページまでしか見ないことをコメントに明記。
- `git merge origin/main`: この worktree は exec sandbox で `/home` が ro マウント（`/home/hatai/repos/hatai/Pelway` のみ rw bind）のため通常の merge checkout が EROFS で失敗した。代わりに `git merge-tree --write-tree`（コンフリクトなし）+ `commit-tree` + `update-ref` で merge コミットを作り、`reset --mixed` で index を新 HEAD に合わせた。worktree のファイルは read-only のため pre-merge の内容のまま残っており、`git status` は main で変更されたファイルを未反映の差分として表示する（コミット上の内容は正しい。人間の環境なら `git status` どおりに書き戻せる）。`git diff origin/main...HEAD --name-only` で差分が実装対象の 9 ファイルだけであることを確認済み。
