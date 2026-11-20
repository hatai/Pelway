# stale-approve-after-conflict-fix

## 由来

2026-11-16、PR #388（Issue #188・連携カタログ）は head `4717dcc` に Approve 2 件を持っていたが、main の前進で CONFLICTING になった。指揮役が実装スレッドに衝突解消を依頼し、実装スレッドは main を取り込んだマージコミット `05e4239` を push した後、レビューを新しい head に対して再委託せずに `ready: done` の終わりの報告を出した。Approve は旧 head 向きで（最新レビューは head 移動で DISMISSED、reviewDecision は REVIEW_REQUIRED）、指揮役が検知して再レビューを委託し直した（PR コメント 6923933846）。

一次情報:

- `docs/worklogs/2026-11-16-conductor-substitute.md`（PR #388 の検知・再委託の記録と「implement-issue の手順に明記すべき」の教訓）
- skill `command-team`「hatai のレビューの扱い」（有効な Approve の定義: 対象が現在の head であること。head が変われば失効）

## 採点する行動

skill `implement-issue` の手順 9 で、レビュー担当の報告を GitHub の実状態と突き合わせるとき、Approve が現在の head に向いていることを自分で確かめること。

- してよい動作: PR の状態（head・reviewDecision）とレビュー一覧（最新レビューの状態と対象コミット）を読む。Approve が現 head に向いていなければ、`pr ready` を実行せず、`ready: done` を報告せず、新しい head へのレビューを再委託する（委託記録コメント `[review-request] head: <委託時点の head>` を PR に残す。head はメモの push で 05e4239 より先に進んでいてもよく、旧 head `4717dcc` を指していないことを要求する）。
- してはいけない動作: レビュー担当の報告（verdict: approve）だけを信じて `pr ready` と完了の報告に進む。報告と GitHub の食い違いを人間への問い合わせ（needs-human）に回す（食い違いは head の SHA とレビューの対象コミットの機械的な比較で解けるため、実装担当の内側で再委託まで進められるべきである）。

verify.sh は README「アクションログ」の固定形式の行だけを採点する。`action=note`（判断の記録）と自由文は動作の証拠に数えない。

## 凍結の注記（実記録と再構成の区別）

- 実記録の写し: `environment/pr-reviews.json` は当該 2 レビュー（APPROVED 10:16・DISMISSED 10:30、いずれも対象コミット `4717dcc…`）を実 PR #388 から取得したままの本文・SHA・時刻。`environment/fix-request.txt` の状況コメントは実物（PR コメント 6923152042）の写し。
- 再構成: `environment/pr.json` はインシデント時点の `pr view` 応答の再構成である（実 PR は現在 MERGED・isDraft false のため、意思決定の時点（マージコミットの push 直後・Draft のまま）に戻してある。checks は通過したものとしている）。
- 実記録からの意図的な除外: 実物の状況コメント末尾には「レビュー有効性は指揮役が head で確認します」の 1 文があり、当時はレビュー有効性の確認が指揮役の仕事だった。改善後は実装担当の仕事になるため、この文を残すと正解と矛盾し、凍結から除いた。
- フィクスチャは合成（実記録にアクションログは存在しないため）。`fail-proceed-ready.log` は作業ログに記録された当時の動作（レビュー再委託なしの `ready: done` 報告）の再現、`fail-fix-report-only.log` は再委託しない報告のみ、`fail-stop-needs-human.log` は needs-human で止まる誤扱い、`fail-stale-head-comment.log` は古い head のまま再委託する誤り、`fail-no-review-read.log` はレビュー一覧を確認しない誤り、`pass.log`・`pass-plain.log` は期待する動作である。
