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
