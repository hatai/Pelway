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

## 実装者メモ（round 3）

対象 head: 03f2e6a944704286c4e907e093040a87d7cea450

1. 詰まった箇所: なし（must-fix 1 件の対応。`mise run verify` は conductor-watch-ensure.test.sh の既知の間欠失敗で 1 回だけ落ち、再実行で合格）
2. 曖昧だった文: 「手順 3 を、書き直した旨のコメントを残した上で Grok Bot 経由で hatai に解除を依頼し、解除後に `ready-for-agent` を付け直す形に書き換える」（PR #407 のレビュー 2f56a7c の must-fix）
3. 仮定と解釈: レビューの直し方をそのまま手順 3〜4 に割り当てた（コメントを残すのを手順 3、付け直しを手順 4）。`ready-for-agent` を付け直す主体は同スキル 24 行目の決まり（付けるのは人間。指揮役は人間に委任されたときだけ）をそのまま引用した。must-fix とされていない nit（テストファイル名、per_page=100 の 1 ページ限りの既知の限界の明記、hatai 本人の UI 操作の文言）は、指示が無いため直していない
4. 足りなかった情報・検証手段: なし（must-fix の指摘は対象行と直し方が明示されていた）
5. 改善案: scripts/tests/conductor-watch-ensure.test.sh の間欠失敗の調査（round 1・2 と同一。別 Issue にすべき）

使ったスキル: implement-issue（round 3 はスキル本文 1 箇所の修正のため、テストの追加・変更は無し）
