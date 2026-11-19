# implement-issue skill の Approve 有効性判定の改訂と回帰 eval（2026-11-19）

出どころ: 人間からの直接依頼（T3 セッション `$skill-creator implement-issue`）。輸入スキル skill-creator の eval 駆動改善ループ（下書き → テスト → 評価 → 改訂）を、リポジトリ既存の回帰 eval 形式（`evals/agent-team/`）と「アクションログを演じる」被験実行に合わせて実施した。Issue を経由していない判断は人間の選択（起票して通常運用を待つ代わりに直接 PR を作る指示）。

## 根拠の対応表

| 変更 | 根拠（一次記録） | 期待する変化 |
|---|---|---|
| implement-issue 手順 9 の突き合わせコマンドを `--json headRefOid,reviewDecision,reviews` に拡張し、approve の有効性（対象コミット＝現在の head・`reviewDecision` が APPROVED）と、無効な場合に `pr ready`・終わりの報告に進まず手順 8 で再委託する経路を明記 | PR #388 インシデント（`docs/worklogs/2026-11-16-conductor-substitute.md` の PR #388 の記録: 旧 head 4717dcc の Approve のまま `ready: done` を報告）と同ログの教訓「implement-issue の手順に明記すべき」。有効な Approve の定義は skill command-team「hatai のレビューの扱い」に既にある | head が動いたあとの旧 Approve で `pr ready` する再発の防止。レビュー有効性の確認が指揮役の介入なしに実装担当の内側で完結する |
| 「よくある失敗」に「approve 報告だけで head を確かめずに `pr ready` する」の1行を追加 | 同上。skill-creator 反復2の被験が改訂文を判定根拠として引用し、手順として機能した | 実装担当が突き合わせの際に head の比較まで辿り着く頻度の向上 |
| 回帰 eval ケース `evals/agent-team/stale-approve-after-conflict-fix/` の追加（実レビュー2件は実 PR #388 から取得した写し） | `evals/agent-team/README.md` のケース化の手順。インシデントの一次記録が残っている | 改訂後の振る舞いの固定。将来の回帰（スキルや運用変更で旧 Approve 扱いに戻ること）の機械的検出 |
| `.gitignore` に `.eval-workspace/` を追加 | skill-creator の被験実行・採点結果の置き場（sandbox が worktree 外書き込みを拒否するため worktree 内に置く） | 実行成果物の誤コミット防止 |
| `typos.toml` の除外に `evals/agent-team/*/environment/**` を追加 | `mise run verify` の typos 検査が凍結した実レビュー ID（`PRR_kwDOVm0izM8…`）を誤検出した（実行ログ）。environment/ は定義上「観察できた状態の写し」で、書き換えると凍結記録が壊れる | 凍結データを検査が書き換え・妨害しない |

## 反復の結果（要約）

- 反復1: 現行スキル（当時）3/3 合格、スキルなし 2/3（委託記録コメントの証跡形式のみ失点）。インシデントの主失敗（旧 Approve の有効扱い）はどちらの構成でも再現せず。
- 反復2: 改訂版 3/3、旧版スナップショット 3/3。改訂の差は GLM 被験の合否に現れない。改訂版被験は判定根拠に改訂文そのものを引用（決定論性の確保として機能）。
- 採点器は被験実行後に2箇所修正した（`src=` のパス接頭辞の許容、委託記録コメントの head の一般化）。いずれも正当な被験動作を採点器が落とすバグで、ケース README に明記。

詳細な benchmark とレビューアは実行時のワークスペース（`.eval-workspace/`、コミット対象外）にあった。上記の数値は `verify.sh` の終了コードと grading.json による。

## 限界と残る候補

- 被験モデルは GLM（zai-coding-plan/glm-5.3）1 種・各構成1回。改訂の効果は GLM の合否差として観測できていない（上記のとおり）。非 GLム実装担当（Devin・Codex）での被験は未実施。
- ラウンドの数え方（判定が割れたときの上限の数え方）は #436 の範囲のため本改訂では触れていない。
- 反復2被験の実装者メモから: 手順 7 の PR 本文の sha は「その round の実装 head（メモコミットの親）」と明記する候補。
- 当初分析で未対応のままの候補: `mise run verify` の無関係失敗の切り分け手順（#404・#405・#411・#440・#470）、実測の無い記録欄を捏造しない1行（#449）、docs 変更のみの Issue の手順 4（#468）、修正依頼の対象明示（#385）。
- write-pull-request 手順 3 の `api PATCH /repos/...` 記法は現行 gh で拒否される（`docs/worklogs/issue-404.md` round 2 の実測。`api --method PATCH` が正）。本 PR のタイトル設定では動作する形を使い、skill 側の修正は別立ての候補。

## 実装者メモ（round 1）

対象 head: 5e08f66dfa23e5be2b184aeb94e7ca97bfa0f0e3（新規実行ファイル verify.sh の mode 100755 を含むため agent-gh-resign の署名付け替え対象外。SHA は push 後も不変）

1. 詰まった箇所: `.eval-workspace/` の置き場。sandbox が worktree 外（/tmp/opencode）への書き込みを拒否したため、worktree 内に `.eval-workspace/` を作り .gitignore に追加した。
2. 曖昧だった文: 「レポジトリ内の一次記録には確認できない。そのため本ケースは実インシデントの凍結ではない。」（`evals/agent-team/single-conductor-handoff/README.md`「由来」）— 実インシデントと想定シナリオのmixed扱いの書き方の先例として参照した。本ケースは実記録（レビュー2件・状況コメント）と再構成（pr.json）と意図的除外（指揮役の確認文）が混ざるため、README の「凍結の注記」に全て明記した。
3. 仮定と解釈: 人間の「PR を作成」の指示は Issue を経由しない直接 PR を認めるものと解釈した（起票する選択肢を示した上での選択のため）。ラウンドの数え方には触れない範囲で最小の改訂とした。
4. 足りなかった情報・検証手段: 被 OnePlus あたりのトークン計測（ハーネスが通知しないため timing.json は所要時間の近似のみ）。Devin・Codex での被験実行（T3 の delegate_task で可能だが本セッションでは実施していない）。
5. 改善案: `evals/agent-team/README.md` の「ケース一覧」に実装担当向けケースの区分（conductor/implementer）を tags で示す運用は既存どおり tags に任せているが、ケース数が増えたら層の列を足すことを検討する。

使ったスキル: skill-creator（輸入元 `~/.claude/skills/synced/…/skill-creator`。本作業の主手順）、write-pull-request（`.agents/skills/write-pull-request/SKILL.md`）、ai-memory-retrieval・ja-text-communication（ユーザーレベル）
