# Issue #490 の作業ログ

## 実装者メモ（round 1）

対象 head: 599f6264fddfc82b0dca464c517fdf1d01928a4f

1. 詰まった箇所: mise run verify: 依存タスク並列実行で clippy と test が aws-lc-sys の build script の out/*.o を競合させ ar が失敗。cargo clean -p aws-lc-sys 後、verify-lint → verify-test → verify-static をシリアルに実行して全部通過。あわせて scripts/tests/audit-approval-docs.test.sh の cond3 初版（`action_snapshot`/`approval_decision` の語単独）が既存本文 03 ch03-s11 にあり最初から通ったため、追記文でだけ成立する文言（主体の行への外部キーで参照せず）に書き直した。
2. 曖昧だった文: 「docs/technical/v0.7 は保全対象のため、更新は v0.7 の検証（verification）の手順に従うこと。」（Issue #490 本文「背景・前提」）
3. 仮定と解釈: 「削除された主体を表す形が pelway-contract に置かれる」は仕様への記述であって実装は「対象外: 監査の表の実装と移行」に含まれると解釈した。選ばなかった解釈: 本 PR で crates/pelway-contract に契約型を追加する。
4. 足りなかった情報・検証手段: verify.py が書き換える verification/relative-links.json と static-checks.json をコミットするかの規則。git log（5f2ad1cb の導入時のみで以後の更新なし）からコミットしない運用と推定した。
5. 改善案: mise.toml の「[tasks.verify]」の説明に「並列で cargo を二重実行すると aws-lc-sys 等の build script out/ の競合で失敗することがある。失敗時は cargo clean -p aws-lc-sys 後にグループをシリアルに再実行」を足す。

使ったスキル: implement-issue
