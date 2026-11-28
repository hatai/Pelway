# Issue #794 の作業ログ

## 実装者メモ（round 1）

対象 head: 507149f0b5f5622766890dbe4372410a036e7300

1. 詰まった箇所: 手順 6: `scripts/tests/review-perspective-docs.test.sh` の Rust 属性の wording pin が shellcheck SC2016 に当たったため、既存の方針どおり行ごとの disable で回避した。`mise run verify` では `aws-lc-sys` の並行ビルドで object file が欠ける失敗と、 `/tmp` の所有者を safe hierarchy と認めない verify-pelway の失敗が出たため、`cargo clean -p aws-lc-sys` と実行ごとの `TMPDIR=/dev/shm/...` で切り分けた。
2. 曖昧だった文: 「領域（少なくとも DB の移行と契約・試験・クライアントの UI）ごとに、対象のパス・報告するもの・報告しないもの・既定の重さを書いた観点のファイルが `docs/agents/` にある → ファイルを読む」（Issue #794「受け入れ条件」）
3. 仮定と解釈: 「契約」は backend の `/crates/pelway-contract/**` だけとし、Electron IPC の `client/packages/contracts/` は含めないと解釈した。glob の正本は各観点ファイルの front matter とし、Rust の inline unit test はパスではなく diff 内容で捕捉すると解釈した。
4. 足りなかった情報・検証手段: glob と変更パスの一致を機械的に確かめる helper が無く、レビュー担当が手で照合する必要がある。
5. 改善案: `docs/agents/review/README.md` の「領域の追加」に「glob の意味、複数一致、inline unit test の補足を書く」を足す。

使ったスキル: implement-issue, write-pull-request, improve-skills, writing-for-agents, `.agents/skills/tdd`（seam: `scripts/tests/review-perspective-docs.test.sh` の presence check）, `.agents/skills/diagnosing-bugs`

### improve-skills 手順 7 の引き渡し記録

- 候補の出どころ: `docs/worklogs/issue-794.md` round 1「改善案」
- 対象の改善点: `.agents/skills/review-pr` に領域別の「報告するもの」「報告しないもの」「既定の重さ」を読む手順が無かった（手順の欠落）
- 期待する変化: レビュー担当が領域固有の must-fix を見落とす手戻りと、範囲外・証拠の無い nit を繰り返す手戻りが減る

## 実装者メモ（round 2）

対象 head: 6b93e81bf31ba7585e4a331d50e8773da988d743

1. 詰まった箇所: 手順 9: サブエージェントへの設計入力が conflict 時の優先順と既定値変更時の理由記録を欠いていたため、最初の Spec レビューに誤報が出た。完全な設計を渡して再実行し、誤報と実指摘を分けた。
2. 曖昧だった文: 「2 回目以降の round では、前回レビューした sha（`Reviewed:` の行）から今の head までの差分と、前回の must-fix が直ったかを見る。」（`.agents/skills/review-pr/SKILL.md`「2 回目以降の round」）
3. 仮定と解釈: 観点ファイル選択の差分は、この既存の round 範囲と同じであり、main の取り込みを除く例外と `diverged` 時の全体レビューも引き継ぐと解釈した。「Rust の inline unit test」は `.rs` ファイルの差分に限定すると解釈した。
4. 足りなかった情報・検証手段: glob の一致を機械的に確かめる helper が無く、root-only 変更や `.rs` 限定の回帰は wording pin でしか確かめられない。
5. 改善案: `.agents/skills/review-pr/SKILL.md` の「2 回目以降の round」を「レビュー対象の差分」という見出しに分け、初回・再レビュー・観点ファイル選択が同じ正本を参照できるようにする。

使ったスキル: implement-issue, write-pull-request, code-review, `.agents/skills/tdd`（seam: `scripts/tests/review-perspective-docs.test.sh` の presence check）
