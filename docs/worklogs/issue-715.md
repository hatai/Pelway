# Issue #715 の作業ログ

## 実装者メモ（round 1）

対象 head: c23746982e68eb95e29a9fc164d6cd1c25cb1f87
PR: #722（この Issue の実装 PR。レビューで確かめる実装の head はこの「対象 head」。メモのコミットはその後ろに乗るため PR の headRefOid と 1 つずれる）

1. 詰まった箇所: `.agents/skills/command-team/SKILL.md`「実装のモデルと effort」: 条件ごとの effort の値（GLM `low`/variant なし/`high`、Devin CLI `thought_level=high`/`max`）は担当で段名が違うが、`effort.raise` の位置では揃っていた（小規模=先頭、通常=2 番目、コア=3 番目、上限=最後）。段名を写さず「`effort.raise` の先頭から N 番目の値」で表した
2. 曖昧だった文: 「`docs/agents/grokbot-runbook.md`: 「3. 同時数と順番待ち」の表を、上限は `agent-team.toml`、使用数は `SLOTS` の行とする。空きが出たときの順番の規則は残す。」（Issue #715「変更の内容」）
3. 仮定と解釈: 「使用数は SLOTS の行」は実装・調査の枠の行だけに掛かると解釈した（SLOTS は #714 が出す実装・調査の枠の使用数の行。レビューの行の数え方（指揮役が立てた分だけ）は残した）。SLOTS の行がまだ出ていない環境のため、無いときは従来どおり「枠の数え方」で自分で数える形にした。レビュー委託の JSON ブロックの `providerInstanceId` は、インスタンス名を列挙するプレースホルダ自体が禁止語の grep に引っかかるため、`<agent-team.toml の instance.review>` に変え、文言ピン試験を同じ PR で直した。Codex のモデル名（`gpt-6.x`）は禁止語の一覧に無く、選択の正本「Codex のモデル」の表は skill に残す決まりのため、そのままにした
4. 足りなかった情報・検証手段: 自分のスレッドの起動時の `Assigned:` の行の effort の値（`Implementer:` 行への転記に使う。起動コマンドの記録は巡回記録側にあり、実装スレッドからは読めない）。devin の通常割り当ての effort は `effort.raise` の 2 番目（`high`）と読んで書いた
5. 改善案: `docs/agents/grokbot-runbook.md` の「MCP の `t3_thread_launch` と `t3_thread_send` の引数」に「`modelSelection.model` には `agent-team.toml` の `[agents.<担当>].models` のモデルを入れる」の 1 文を足す（今回の改訂で `instanceId` は登録情報参照と明記したが、`model` は例のプレースホルダに留まる）

使ったスキル: .agents/skills/implement-issue

## 実装者メモ（round 2）

対象 head: 063595b74cb567dcec5b419ec93cc663090cb0e4
PR: #722

レビュー指摘（run 7164371558 codex request-changes、head 5e5d6e1）への対応:

1. 詰まった箇所: なし
2. must-fix の対応:
   - implement-issue の委託の表が作業ツリーの `agent-team.toml` を指していた → 「レビューの優先順位と登録情報は main の承認済みの版から読み、読めなければ委託・起動を止める」を implement-issue・command-team・runbook の指摘箇所すべてに書いた（レビューの隔離を担うインスタンスを審査中のブランチの設定で選べてしまう穴）
   - 修正指示 2 回超の移行先が「`normal` の先頭からまだ使っていない担当」だった → 移行先を `normal` の先頭の担当に限定し、すでにその担当のときは替えない形に戻した（変更前の Devin CLI 限定の条件と同等）
   - command-team-details の調査の数え方が「実行中の調査スレッド（Issue を持たない入口B のものを含む）」で全スレッドを数えていた → 正本どおり「Issue を持たない入口B の調査の稼働中スレッド」だけに絞った（Issue 付きの調査は `agent:working` で数えるため二重計上になる）
   - runbook の使用数の見方が `SLOTS` の行が無い場合しか触れていなかった → `SLOTS over`（上限超過の行）と `SLOTS error`（toml 検証の失敗）の扱いを足し、使用数か上限を確認できなければ割り当てない形にした
3. nit の対応: [文書のずれ] runbook の空き枠を待つ段落が上限超過のスレッド全般に読める → 「実装・調査の上限」（レビューの順番待ちは管理しない）と明記した

## 実装者メモ（round 3）

対象 head: ac0b4cce4a91a352d4edfe87f1ba1ef0ebf6ebf9
PR: #722

レビュー指摘（run 7164704743 codex request-changes、head 188f3eff）への対応:

1. 詰まった箇所: なし
2. must-fix の対応:
   - runbook §3 の使用数の見方が watch.log の `SLOTS` 行を根拠にしていた（共有ログの行はキューの要求のファイル名で偽装でき、出所を確認できない）→ 見張りが巡回ごとに公開する `slots.json` を判断の記録とする形に替え、`status` が `ok`・`updated_at` 60 秒以内・`slots.json` の更新時刻が `state.json` から 30 秒以上遅れていないという有効性と鮮度の判定（Issue #714・PR #721 の決まり）を書いた。記録を確認できなければ「枠の数え方」で自分で数える形は維持
   - 見張りの `usage` は Issue の枠だけを数えるため、Issue を持たない入口B の調査の稼働中スレッド数を指揮役が巡回記録の割り当て表から加算し、「枠の数え方」と同じ合計で割り当て判断に使うことを明記
   - `board-patrol-docs.test.sh` の cond6c の文言の存在確認を、判断に使う記録（`slots.json` と巡回記録の割り当て表）に合わせた
3. 印の nit（`[文書のずれ]`）の対応:
   - runbook §3 の「空きが出たら」の一覧が指揮役のレビューを枠待ちの対象に含めて読める形だった → 起動の優先順位と実装・調査の枠の順番待ちを分けて書いた
   - command-team と runbook の冒頭が同時数の上限の正本を引き続き `agent-team.md` と案内していた → 両方で体制の正本は `agent-team.md`、同時数の上限の正本は `agent-team.toml` と分けた
4. 仮定と解釈: なし

## 実装者メモ（round 4）

対象 head: 01cd254a4b124a8df13e7dcb27d261028175831e
PR: #722

レビュー指摘（run 7165107457 codex request-changes、head a1660428）への対応。hatai の例外の判断（2026-11-21 の「ラウンド上限は無視して must-fix がなくなるまで続ける」指示）で round 4 に進んだ:

1. 詰まった箇所: なし
2. must-fix の対応:
   - runbook §3 の `slots.json` を使う条件を PR #721 の実装と揃えた。`patrol_at` が `state.json` の `patrol_at` と一致することを必須にした（両ファイルに巡回の印を書くため、ファイルの更新時刻の差が小さくても印が異なれば最新の記録ではない）。また `usage` は直前の巡回時点のものなので、割り当て・再開の直後は新しい記録に変更が載ったと確かめられるまで自分が動かした分を足して数え、確かめられなければ現在の使用数を数え直す形にした
   - runbook「枠の数え方」・正本の「制約」（agent-team.md）・調査時の要約（command-team-details）の 3 箇所を、補助記録の印だけでは除かず、T3 のスレッド一覧で停止を確認できたもの（実装スレッドがすべて完了・失敗・中断・取消またはアーカイブ済み）だけを使用数から除く同じ規則に揃えた（PR #721 の `stop_verified` と同じ確認）
3. 印の nit（`[文書のずれ]`）の対応: PR 本文の概要と対応表を `slots.json` を使う説明に替え、`review_limits` がレビューの同時数の目安で回数の上限ではないこと（回数の上限は command-team に残る）を直した。round 3 の委託のあとに本文を更新済み
4. 仮定と解釈: なし
