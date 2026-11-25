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

## 実装者メモ（round 5）

対象 head: 6edab97509a7f6f634a81b3e558654f983431f6a
PR: #722

レビュー指摘（run 7165693838 codex request-changes、head 2ced301f）への対応。hatai の例外（ラウンド上限の免除）で継続:

1. 詰まった箇所: なし
2. must-fix の対応:
   - runbook「枠の数え方」・正本の「制約」・command-team-details の要約の 3 箇所を、PR #721 の `stop_verified`（`scripts/conductor-watch.sh:711`、head f080bd1 時点）と同じ判定に揃えた。round 5 で runbook に記載した条件: 補助記録の作業状態の印が停止中・再開処理中・再開待ちのもののうち、(a) 巡回記録の現在の割り当てに結び付いたスレッド（`implement_thread` の組の `thread_id`）が T3 の一覧にあってどれも動いていない（完了・失敗・中断・取消のどれか、またはアーカイブ済み）こと、(b) 一覧にある同じ Issue の実装スレッドがどれも動いていないこと、の両方を確かめられるものだけを除き、`stop_verified` が偽を返す側の条件（割り当ての記録が無い・結び付いたスレッドが一覧に無い・`rollback` に起動結果の照合が残る）は使用中として数える
   - 照合と round 5 の残差: `stop_verified` は `it.get("rollback") is not None`・`bound_ids` が `None`・結び付いた thread_id が一覧に無い・結び付いたか題名のスレッドに動いているものがある・一覧に題名一致の実装スレッドが 1 件も無い（`if not ts: return False`）、のどれかで偽を返し使用中と数える。round 5 では command-team-details の要約に `rollback` の照合と「一覧に実装スレッドが 1 件以上ある」の条件が抜け、3 文書とも題名のスレッドが 0 件の場合の扱いが抜けていた（round 6 で修正: runbook に 1 件以上を明記し、要約と正本は runbook の手順を参照する形に集約）
3. 印の nit（`[文書のずれ]`）の対応: メモに実際に記載した条件と照合結果をこの節に残した
4. 仮定と解釈: なし

## 実装者メモ（round 6）

対象 head: 58247475782206bb3c75acb6c0e244134c69a573
PR: #722

レビュー指摘（run 7169605014 codex request-changes、head 908983de）への対応。hatai の例外（ラウンド上限の免除）で継続:

1. 詰まった箇所: なし
2. must-fix の対応:
   - 停止の判定を runbook「枠の数え方」に集約し、正本の「制約」と command-team-details の要約は runbook の手順を参照する形に替えた。runbook 側には「一覧に同じ Issue の実装スレッドが 1 件以上あってどれも動いていないこと」を明記（PR #721 の `stop_verified` の `if not ts: return False` と同じ。題名で 1 件も確認できないときは除外できない）。要約には、起動結果の照合が残るもの（`rollback`）と一覧に実装スレッドが無いものを使用中として数えることを明記
3. 印の nit（`[文書のずれ]`）の対応:
   - round 5 のメモの照合を、`stop_verified` が偽を返す条件の全列挙と round 5 時点の残差（要約側の欠落と題名 0 件の扱い）を書く形に直した
   - PR 本文の「確かめ方を弱める変更」を差分に合わせた（board-patrol-docs.test.sh は存在確認の追加のみ、既存の文言を置き換えたのは review-delegation-docs.test.sh）
4. 仮定と解釈: なし

## 実装者メモ（round 7）

対象 head: 55c86f2c4ec36202d22f029272b989fbe595dd21
PR: #722

レビュー指摘（run 7170056854 claude request-changes、head 6a59e981）への対応。高リスク PR の必須実行（codex+claude）のうち、claude の 1 回目:

1. 詰まった箇所: なし
2. must-fix の対応:
   - 指摘: `instance.review` の正本を `agent-team.toml` に移した結果、`[agents.*].instance` だけを変える PR が高リスク・`security=yes` のパス条件のどれにも当たらず、通常のレビューで通りうる。マージ後の toml はそのまま委託・起動に読まれ、隔離の無いインスタンスへ替えられうる
   - 直し方の (1)・(2) の両方を適用:
     - (1) command-team「レビュー担当と effort」の高リスクの条件と implement-issue 手順 8 の `security=yes` の条件に「`agent-team.toml` の `[agents.*].instance` の変更」を追加
     - (2) 委託・起動の直前に、読んだ `instance.review` が `docs/dev-environment.md` の「レビュー担当の Codex」「レビュー担当の Claude」「レビュー担当の Devin CLI」の節が名指すインスタンスと一致することを確かめ、一致しなければ止める、を command-team の表・implement-issue 手順 8・runbook（`t3_thread_launch` の引数と「レビューを始める」の 2 箇所）に追加。正本の「レビュー担当の権限」にも同じ条件を 1 行追加
3. 印の nit（`[文書のずれ]`）の対応: implement-issue の `target.options` の列でコードスパン内にバッククォートが入れ子になり表示が崩れていた箇所（codex・claude の 2 行）を、`{"<[agents.<担当>].effort.option>": "<effort>"}` の形に直した
4. 仮定と解釈:
   - runbook:214（MCP `t3_thread_launch` の引数の項）は指摘の 3 箇所に含まれないが、同じ「main の版から読む」規則を並記している箇所のため、同じ照合の条件を足した
