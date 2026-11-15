# Issue #342 の作業ログ

## 実装者メモ（round 1）

対象 head: a7a72be98f33636b8c165823b4e2b33797561717（この round の実装の実体を含むコミット。このメモを含むコミットの親）

1. 詰まった箇所: 環境: この exec sandbox は `/home` が読み取り専用で、`agent-sandbox.test.sh` の opencode-plugin 節（`$HOME/.t3/worktrees/sbx-plug-test` の mkdir）と、mise 経由の mbx cargo（`couldn't create a temp dir`）が動かない（issue-308・issue-315・issue-316 のメモと同じ）。node を PATH から外して（`MISE_DISABLE_TOOLS=node`＋node を含む PATH ディレクトリのリンク集化）plugin 節を仕様どおり skip にし、`RUSTUP_TOOLCHAIN=1.99.1-x86_64-unknown-linux-gnu`＋`MISE_DISABLE_TOOLS=node,rust`＋toolchain bin 先頭の PATH で `mise run verify` 全体（exit 0）を通した。plugin 節の実走は CI が担う
2. 曖昧だった文: 「`scripts/demo-check/selftest.sh` が収集経路に含まれる（glob 対象外なら明示のまま残る）」（Issue #342 本文「受け入れ条件」）
3. 仮定と解釈: 「明示のまま残る」は収集スクリプト内の明示列入れで満たされると解釈した（selftest.sh のパスは変えない。`mise run demo-check-selftest` タスクと docs/dev-environment.md の参照がそのまま生きる。scripts/tests/ への移動は選ばなかった: demo-check の内部構成の変更はこの Issue の対象外）。未決事項の収集方式は収集用スクリプト 1 本（`scripts/script-test.sh`）を選んだ: `pr-size` タスクが `scripts/pr-size.sh`＋`scripts/tests/pr-size.test.sh` の形を持つのと同じにすると、収集の振る舞い自体を `scripts/tests/script-test.test.sh` で検査できる。mise.toml の run へのシェルループ埋め込みは選ばなかった（テストで確かめられないため）。スキル同梱テストの glob は `.agents/skills/*/*.test.sh` とし、外部スキルのディレクトリも収集対象に含めると解釈した（現在の同梱テストは pr-watch の 1 つのみ）
4. 足りなかった情報・検証手段: plugin 節（`command -v node` で走る）はローカルでは skip にせざるを得ず、`mise run script-test` の全テスト実行の確認は CI に委ねる。`.agents/skills/improve-skills/SKILL.md` 手順 6 の「登録漏れでも script-test 自体は成功してしまう」は今回の変更で前提が消えたため、同じテーマの変更として本 PR で直した
5. 改善案: `docs/agents/skill-helpers.md` の「テストの置き場と実行方法」に「glob 収集は `.agents/skills/` 配下の外部スキル固定ディレクトリも走査する。外部スキルが同梱テストを更新で持ち込んだときの扱い（実行するか無視するか）は未決なので、持ち込みが起きた時点で判断が必要」を足す

使ったスキル: implement-issue（`.agents/skills/implement-issue`）、improve-skills（`.agents/skills/improve-skills`。入口B: 手順 3「改善する」と 6「検証する」を適用）

## 実装者メモ（round 2）

対象 head: 54b4ee15f776261fef6319573730c97a26841b9b（この round の実装の実体を含むコミット。このメモを含むコミットの親）

1. 詰まった箇所: PR #351 の CI（run 43704519836・head 4b05e5e）で selftest の「run.sh: partial run under ACCEPTANCE」1 ケースだけが失敗。同一のコードで走った 1 回目の CI（run 43702511912・head 0417e71）は同一ケースを含め全合格で、失敗時の出力には期待値（exit 1・PARTIAL RUN・ACCEPTANCE MODE の両マーク）がすべて揃い、ログに fork・OOM 等のエラーも無い。ローカルでは切り出し 300 回・実物 selftest 6 回のいずれでも再現せず。ケース内の `printf '%s\n' "${OUT}" | grep -q …`（`set -o pipefail` 下・マッチが先頭行付近で grep が早期 exit する形）が唯一非決定的になり得る要素のため、この形式を selftest 内の全 27 箇所（`grep -q` 26 箇所＋`grep -c` 1 箇所）で here-string（`grep -q … <<<"${OUT}"`）に置き換えてパイプ自体を消した
2. 曖昧だった文: 「glob 収集で selftest が CI の script-test に入ったことで、CI 環境（`DEMO_CHECK_RUNTIME` 未設定）での失敗が表面化したと見られます」（指揮役の修正依頼・PR #351 のコメント）
3. 仮定と解釈: 指揮役の見立て（glob 収集の入れ替えが原因）に対し、旧列挙にも selftest は最初から含まれており、かつ同一コードの 1 回目の CI が合格していたことから、収集の変更は失敗の原因ではなくまれな揺れの偶然の表面化と解釈した。修正は期待値の変更ではなくアサーション形式の堅牢化のみにとどめた（`DEMO_CHECK_RUNTIME` 未設定時の PENDING 挙動は仕様どおりで、テストもそれを期待しているため変えない）
4. 足りなかった情報・検証手段: 失敗の正確なトリガーの直接観測（CI ログにはパイプ内 grep の終了コードが記録されず、再現もしなかったため類推。here-string 化でこのクラスは構造的に消えるが、再発した場合は別原因として調査し直す）
5. 改善案: `docs/agents/skill-helpers.md` の「テストの置き場と実行方法」のテスト形式の箇所に「コマンド置換の結果の検査は `grep -q … <<<"$OUT"`（here-string）を使い、`printf | grep -q` パイプは `set -o pipefail` 下で grep の早期 exit と組み合わさると競合になり得るため使わない」を足す

使ったスキル: implement-issue、improve-skills（入口B）、pr-watch（pr-digest による CI 状態の把握と失敗ログの取得）
