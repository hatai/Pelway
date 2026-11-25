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
