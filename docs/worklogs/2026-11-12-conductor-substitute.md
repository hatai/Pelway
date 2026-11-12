# 指揮役の代行の作業ログ

Grok Bot がセッションのレート制限で動かない間、Claude Code が指揮役を代行した記録と、運用中に見つけた改善点。改善点の「反映先」が PR #110 のものは、spec・runbook・skill `command-team` に反映した。

## 改善点

| # | 見つけたこと | 対応 |
|---|---|---|
| 1 | T3 の Pelway プロジェクトの既定 runtimeMode が `full-access`。`t3 thread start` には権限のフラグが無く、MCP を持たない指揮役はレビュースレッドを `auto-accept-edits` で立てられない | Claude Code に T3 Connect の MCP（`https://prod-adff7db412d92078.t3coderelay.com/mcp`、OAuth）を local スコープで追加して解決。手順書に代行時の接続方法を書く |
| 2 | 手順書に Grok Bot が止まったときの代行手順が無い（通知の受け口、見張りの方法、「部屋」がどこか） | 手順書に代行の節を足す |
| 3 | `pelway-agent-watch` は webhook にしか出せない。代行者は `t3 thread list` と `gh pr list` を自分で見張る必要があり、`report`・`round`・`stall_reason` を同じ形で受け取れない | 標準出力（JSON Lines）へ出す口を足す Issue を立てる |
| 4 | CI の apt 取得が runner の名前解決の一時失敗（`azure.archive.ubuntu.com`）で落ち、PR #104 の verify と octocov が変更と無関係に失敗した | `apt-get -o Acquire::Retries=5` を入れるコミットを `ci/apt-retries` に用意。`pelway-dev` App に `workflows` 権限が無いため push できず、人の push 待ち |
| 5 | 実装スレッドは pr-size 超過で `needs-human` を付けて止まるが、超過の大半が削除（手写しの解消など）の場合も同じ扱いになる | pr-size の数え方（削除を含むか）と、削除が主な超過の扱いを決める |
| 6 | Claude Code のセッションが切り替わると scratchpad が消え、見張りのスクリプトと状態が失われる | 見張りの仕組みをリポジトリのスクリプトにする（3 と合わせる） |

| 7 | 指揮役が自分で書いた PR（#106）を `pelway-review` で Approve しようとして、Claude Code の自己承認の検査で止められた | 指揮役が作った PR も Codex のレビュースレッドに回す。T3 Connect の MCP の `t3_thread_launch` で `runtimeMode: auto-accept-edits` を指定すれば、手順書どおりに立てられることを確かめた |

| 8 | CODEOWNERS（`* @hatai`）により、`pelway-review` の Approve だけではマージできない | hatai の承認により、`pelway-review` の Approve と必須チェックの通過を確かめた PR は指揮役が `--admin` でマージする。手順書と仕様に反映する |
| 9 | pr-size の超過を指揮役が認めても、レビュー担当は「pr-size が失敗」を must-fix に数える決まり | レビューの指示に例外を書き足して回避した（手順書は「指示は Issue 番号と skill 名だけ」）。例外を示すラベル（例：`pr-size-exception`）を作り、review-pr skill が読むようにする |
| 13 | GLM（zai-coding-plan）の 5 時間あたりの利用上限に達すると、実装スレッドは理由を残さずに `failed` になる（T3 のスレッドの activity にだけ「Usage limit reached」が出る）。runbook の「2 回目の `failed` でエスカレーション」をそのまま当てはめると、上限による失敗まで人間に上げてしまう | 見張り（`scripts/conductor-watch.sh`）が `failed` のときに activity の error の title を読んで出力に含める。利用上限による失敗は、リセット時刻まで新しい実装スレッドを止め、リセット後にやり直しを送る手順を command-team に足す |
| 14 | main と衝突している PR では、GitHub が `pull_request` の workflow を動かさず、チェックが 1 つも付かない（#126）。見張りの出力は「チェックなし」になり、待ち続けてしまう | PR ができたら mergeable を確かめ、`CONFLICTING` なら実装担当に main の取り込みと衝突の解消を送る手順を command-team に足す |
| 15 | 分割することになった Issue が続いた（#86、#89、#93、#96、#102）。Issue の「規模の見積もり」が小・中でも、テストと文書を含めると追加 800 行を超える | Issue を書く側（指揮役・人間）が、成果物の置き場所（crate・テスト・文書）と想定の行数を Issue に書く（#96 の実装者メモの改善案）。agent-task の Issue Form の説明に足す |
| 10 | 指揮役が PR #113 を、先にマージした PR #111（`NewCard` を `payload` に変更）を取り込まずにマージし、main のビルドを壊した。ファイルが重ならないため取り込みを省いた | skill `command-team` の「マージ」を、マージの直前に PR が今の main を含むかを必ず確かめ、含まなければ取り込んで CI を通す形に直した。main の修正は PR #114 に含めた |
| 11 | 指揮役と実装担当は、どちらも `pelway-dev` の名義で Issue にコメントするので、`<!-- pr-size-exception -->` のコメントを誰が書いたかをレビュー担当が区別できない（実装担当には skill で禁じているだけ） | 指揮役用の名義（別の GitHub App、または hatai の名義）を分けるか、ラベルを付けた者をタイムラインの API で確かめる方式を検討する |
| 12 | 再起動の後に T3 がスレッドを自動で再開すると、その分も実行回数（`round`）に数えられる（#100 は差し戻し 1 回目で ordinal 5） | `round` をスレッドの実行回数ではなく、指揮役が送った修正の指示の回数で数える方式を検討する |

## opencode＋GLM 側の材料

実装セッションの記録（T3 スレッド）と PR レビューから集める。レビューで繰り返し指摘される問題は、GLM 側の指示・スキルの不足として扱う。

| # | Issue・PR | GLM の挙動 | 共有層での対応案 | 効果の測り方 |
|---|---|---|---|---|
| G1 | #91（PR #105）、#99（PR #107） | pr-size が 800 を超えた後、コメントの削減、テストの統合・削除、ループ化と巻き戻しを繰り返してから止まった（#91 は 939→868、#99 は 1018→977）。#91 では重複とみなしたテストを 2 つ削除した | implement-issue の「止まる条件」に、行数を合わせるためにテスト・受け入れ条件の確認・説明を削らないこと、超えた時点で分割案を書いて止まることを足す | 以降の実装スレッドで、pr-size 超過の後に圧縮を試みた手順の数と、テストの削除の有無 |
| G2 | #88（PR #104） | 受け入れ条件・TDD・Issue への判断の記録まで手順どおり。指摘は nit のみ（一覧の対象メンバーをクエリで受け、他の経路のヘッダーと方式がそろわない） | 既存の経路の慣習（身元はヘッダーで受ける）に合わせることを skill に書くかは、同じ指摘が繰り返されたら判断する | 同種の nit の再発 |

| G3 | #99（PR #107）round 1 | DB キューで、lease 切れの `running` 行を再取得するときに `max_attempts` を確かめず、上限なく再実行しうる実装にした。テストは `fail` 経由の再試行の上限だけを確かめていた（Codex の must-fix） | 再試行の上限など「上限・回数」の仕様は、すべての経路（正常・失敗・回収）で確かめるテストを書くことを、implement-issue の TDD の節に足すか検討する | 同種の「一部の経路だけ上限を確かめる」指摘の再発 |
| G4 | #89 round 1〜2（新しい implement-issue） | pr-size 超過（1037）の後、テストの重複の除去だけで 941 まで減らし、テスト・受け入れ条件の確認・説明は削らずに止まった。実装者メモを round 1・2 とも 5 項目で残し、分割案は追加行の境目（契約・台帳／HTTP）で出した | G1 の対応（PR #108）が GLM に効いた例として記録する | 次の超過でも同じ動きか |
| G5 | #87（PR #111）の実装者メモ | 「正規化」の範囲が Issue に無く、本文の改行・末尾空白、添付と scope の集合化だけを正規化と解釈した。仕様（03「固定承認と外部副作用」）はハッシュの対象に参照版・integration 版も挙げるが、Issue の列挙に無いので外した、と自分で書いた | 仕様と Issue の列挙の食い違いを、Issue を書く側（指揮役・人間）が仕様と照らしてから ready-for-agent にする。03 に、デモ契約が使う項目の注記を足す | 実装者メモの 2・4 に、仕様と Issue の食い違いが書かれる回数 |

## Codex（レビュー担当）側の材料

| # | PR | 挙動 | 対応案 |
|---|---|---|---|
| R2 | #110 round 1 | 見張りスクリプトとスキルが素の `gh` を使っていることを must-fix にした（AGENTS.md の `scripts/agent-gh` の決まり）。妥当な指摘 | 読み取りを `scripts/agent-gh dev` に直し、`--admin` のマージだけを例外として正本に書いた |
| R1 | #108 round 1 | 差分の削除行（旧「4 行」）を、残っている記述と読んで must-fix にした。head には「4 行」の記述は無かった | review-pr に「must-fix は head の版のファイルで該当行を確かめてから書く（差分の `-` 行は消えた記述）」を足すか、同じ誤りが繰り返されたら判断する |

## 割り当ての記録

指揮役が実装・レビューを割り当てた記録。改善ループで、レビュー結果（`Reviewer:` の行）・実装者メモ（`Implementer:` の行）と対照する。下の表より前の割り当ては、T3 のスレッドの設定と Issue のコメントに残っている。

| 日時（JST） | Issue・PR | 役割 | 担当 | モデル | effort | 理由 |
|---|---|---|---|---|---|---|
| 2026-11-12 | #102 | implement | opencode | zai-coding-plan/glm-5.3 | high | 監査（セキュリティの境界） |
| 2026-11-12 | #103 | implement | opencode | zai-coding-plan/glm-5.3 | default | 通常のコード |
| 2026-11-12 | PR #123 | review | codex | gpt-6-luna | medium | 文書が主の PR |
| 2026-11-12 | PR #125 | review | codex | gpt-6-luna | medium | 運用文書の小さな変更 |
| 2026-11-12 | #96 | implement | opencode | zai-coding-plan/glm-5.3 | default | 通常のコード |
| 2026-11-12 | PR #125（2 回目） | review | codex | gpt-6-luna | high | 範囲が広がったため |
| 2026-11-12 | PR #127 | review | codex | gpt-6-luna | high | 監査（セキュリティの境界） |

## 判断の記録

- #91：pr-size 868/800 の超過は分割せずに進めた。超過の大半は client-core の契約型の手写し（331 行）の削除。hatai から、diff の上限を守れないときは `--admin` で迂回してマージしてよいと委任を受けている。
