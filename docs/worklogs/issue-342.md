# Issue #342 の作業ログ

## 実装者メモ（round 1）

対象 head: a7a72be98f33636b8c165823b4e2b33797561717（この round の実装の実体を含むコミット。このメモを含むコミットの親）

1. 詰まった箇所: 環境: この exec sandbox は `/home` が読み取り専用で、`agent-sandbox.test.sh` の opencode-plugin 節（`$HOME/.t3/worktrees/sbx-plug-test` の mkdir）と、mise 経由の mbx cargo（`couldn't create a temp dir`）が動かない（issue-308・issue-315・issue-316 のメモと同じ）。node を PATH から外して（`MISE_DISABLE_TOOLS=node`＋node を含む PATH ディレクトリのリンク集化）plugin 節を仕様どおり skip にし、`RUSTUP_TOOLCHAIN=1.99.1-x86_64-unknown-linux-gnu`＋`MISE_DISABLE_TOOLS=node,rust`＋toolchain bin 先頭の PATH で `mise run verify` 全体（exit 0）を通した。plugin 節の実走は CI が担う
2. 曖昧だった文: 「`scripts/demo-check/selftest.sh` が収集経路に含まれる（glob 対象外なら明示のまま残る）」（Issue #342 本文「受け入れ条件」）
3. 仮定と解釈: 「明示のまま残る」は収集スクリプト内の明示列入れで満たされると解釈した（selftest.sh のパスは変えない。`mise run demo-check-selftest` タスクと docs/dev-environment.md の参照がそのまま生きる。scripts/tests/ への移動は選ばなかった: demo-check の内部構成の変更はこの Issue の対象外）。未決事項の収集方式は収集用スクリプト 1 本（`scripts/script-test.sh`）を選んだ: `pr-size` タスクが `scripts/pr-size.sh`＋`scripts/tests/pr-size.test.sh` の形を持つのと同じにすると、収集の振る舞い自体を `scripts/tests/script-test.test.sh` で検査できる。mise.toml の run へのシェルループ埋め込みは選ばなかった（テストで確かめられないため）。スキル同梱テストの glob は `.agents/skills/*/*.test.sh` とし、外部スキルのディレクトリも収集対象に含めると解釈した（現在の同梱テストは pr-watch の 1 つのみ）
4. 足りなかった情報・検証手段: plugin 節（`command -v node` で走る）はローカルでは skip にせざるを得ず、`mise run script-test` の全テスト実行の確認は CI に委ねる。`.agents/skills/improve-skills/SKILL.md` 手順 6 の「登録漏れでも script-test 自体は成功してしまう」は今回の変更で前提が消えたため、同じテーマの変更として本 PR で直した
5. 改善案: `docs/agents/skill-helpers.md` の「テストの置き場と実行方法」に「glob 収集は `.agents/skills/` 配下の外部スキル固定ディレクトリも走査する。外部スキルが同梱テストを更新で持ち込んだときの扱い（実行するか無視するか）は未決なので、持ち込みが起きた時点で判断が必要」を足す

使ったスキル: implement-issue（`.agents/skills/implement-issue`）、improve-skills（`.agents/skills/improve-skills`。入口B: 手順 3「改善する」と 6「検証する」を適用）
