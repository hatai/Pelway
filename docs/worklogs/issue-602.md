# Issue #602 の作業ログ

## 実装者メモ（round 1）

対象 head: 25438552f84caee3dcc5d32450092d2366384037

1. 詰まった箇所: `scripts/conductor-watch.sh` の `"$T3_BIN" thread list --json > threads.json || true`: t3 が失敗しても空の一覧と同じに読まれ、登録があると `thread-missing` の誤った格上げになる。`|| rm -f` に替えて取得失敗を区別した。`gh issue edit --add-label needs-human --remove-label needs-decision` が 1 回の不可分な更新かを確かめられなかったため、`needs-human` を付けてから `needs-decision` を外す 2 回の呼び出しに分けた（途中で失敗してもどちらのラベルも無い状態にしない）。
2. 曖昧だった文: 「`t3 thread list` の取得に失敗した周は判定せず、次の周に回す（指揮役がいるのかいないのか分からないまま替えない）」（Issue #602 の「背景・前提」）。登録ファイルが無ければ、一覧が無くても T3 の指揮役がいないことは分かる。
3. 仮定と解釈: 受け入れ条件の無条件の文（「取得に失敗した周では格上げせず」）に合わせ、登録ファイルが無くても一覧の取得に失敗した周は格上げしないと解釈した。選ばなかった解釈は「登録が無ければ一覧なしでも格上げする」。この解釈のままだと、t3 が失敗し続けるホストでは格上げが起きない。目印の名前は未決事項の起票者の案で進めた。`grokbot` の目安 24 時間は指揮役が書く待ちに残し、`needs-decision` から導く待ちだけを `labeled_at` + 15 分にした。
4. 足りなかった情報・検証手段: `needs-decision` のラベルが GitHub にまだ無いため、実際の `issue edit --remove-label needs-decision` と、`pelway-dev[bot]` 名義のコメントの `@hatai` で通知が届くことは、モックの `agent-gh` でしか確かめていない（通しの訓練 #605 で確かめる）。
5. 改善案: `docs/agents/triage-labels.md` の `needs-human` の行の Who adds に「conductor-watch, when `needs-decision` stays 15 minutes and no conductor is running」を足す（#603 で `needs-decision` の行を足すときに一緒に）。

使ったスキル: `implement-issue`、`write-pull-request`
