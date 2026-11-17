# Issue #405 の作業ログ

## 実装者メモ（round 1）

対象 head: 797baf44feedf28cee2aea3d96cc583d1e0f875b

1. 詰まった箇所: scripts/tests/conductor-watch-ensure.test.sh: 手順 6 の前に回帰確認したら `behavior-fallback`（`watch is running`）が間欠失敗した。変更を stash して main で 6 回実行したところ 3 回同じ失敗があり、本変更と無関係な既存のフライルティと判断した（Issue の対象外「進行中の別障害」に該当する可能性。今回は触っていない）
2. 曖昧だった文: 「conductor-watch が PR の `automerge` ラベルの labeled/unlabeled イベントの主体を読み、許可リスト（`hatai` と `pelway-dev[bot]`）外なら警告行を watch.log に出す」（Issue #405 の受け入れ条件）
3. 仮定と解釈: 許可リストは Issue 本文のとおり hatai と pelway-dev[bot] の 2 つだけとし、Kodiak（実アカウント `kodiakhq[bot]`、コンフリクト時に automerge を外す）や Grok Bot の分は足さなかった。ラベルイベントの取得元は `scripts/agent-gh dev api /repos/hatai/Pelway/issues/events`（PR のラベルイベントも含む。実データで応答形式を確認済み）と仮定した。選ばなかった解釈: kodiakhq[bot] を当初から許可リストに含める（受け入れ条件の記述と食い違うため却下。`kodiakhq[bot]` の追加は指揮役の判断に回す）
4. 足りなかった情報・検証手段: Grok Bot が automerge を外すときの GitHub アカウント名（Issue の未決事項どおり未確認。runbook の「マージキュー待ちの PR」に追記した）。実稼働の conductor-watch での LABEL bad-actor 行の確認（本 PR マージ後の実運用で観察する）
5. 改善案: scripts/tests/conductor-watch-ensure.test.sh の「behavior-fallback」の間欠失敗（count の race）を別 Issue として調査すること。`docs/agents/grokbot-runbook.md` の「マージキュー待ちの PR」に「Kodiak（`kodiakhq[bot]`）が外した automerge は現状警告行に出る。許可リストに足すかは指揮役の判断」を、判断後に追記する

使ったスキル: implement-issue（輸入スキル tdd は未使用の通常割り当てのため。テストは scripts/tests/conductor-watch-label-actor.test.sh として先に書き、レッド→グリーンを確認）

## 実装者メモ（round 2）

対象 head: 0ddffc6775b3f49579052a94b3a69d41cee1875e

1. 詰まった箇所: scripts/tests/conductor-watch-label-actor.test.sh: 裏付けコメントのモックファイル（comments-<n>.json）を書き忘れたイベントは「comments 取得失敗 → 判定延期」の仕様どおり警告が出ず、レッドの原因がテスト側の設定漏れだった。ファイルを置いて解決
2. 曖昧だった文: 「`needs-human` や automerge が外れた・付いた時刻を、次の 2 つの記録と突き合わせる」（Issue #405 のコメント「【チーム判断による要件変更】」）
3. 仮定と解釈: needs-human は「外れた」だけ（付与はエスカレーションで日常的なため検査しない）、automerge は付け外し両方を検査すると解釈した。watch が機械判定できるのは記録 1（hatai 本人のコメント。イベントの前 24 時間以内、同じ Issue・PR）だけで、記録 2（指揮役と T3 スレッドの操作記録）は watch から読めないため巡回の仕事として runbook に書いた。コメントの窓を 24 時間にしたのは人間待ちのフォローアップタイマーの目安と揃えたため。選ばなかった解釈: actor ベースの許可リスト検査を併存させる（要件変更が「actor だけで判断せず」への置き換えのため、LABEL bad-actor は廃止して LABEL unbacked に統一）。#188 の実イベントの actor は pelway-dev[bot] だったが、決定を実装側の解釈で差し替えない
4. 足りなかった情報・検証手段: Grok Bot が了承を中継するときの GitHub アカウント名（記録 1 の「Grok Bot が中継した了承」を watch が機械判定するには必要。未確認のため巡回の照合対象として runbook に書いた）。実稼働 conductor-watch での LABEL unbacked 行の確認（マージ後の実運用）
5. 改善案: scripts/tests/conductor-watch-ensure.test.sh の間欠失敗（`watch is running`・`foreign watch present`。main でも発生、本 PR と無関係）を別 Issue で調査すること

使ったスキル: implement-issue（テストは scripts/tests/conductor-watch-label-actor.test.sh を書き直してレッド→グリーンを確認）
