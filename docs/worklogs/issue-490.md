# Issue #490 の作業ログ

## 実装者メモ（round 1）

対象 head: 599f6264fddfc82b0dca464c517fdf1d01928a4f

1. 詰まった箇所: mise run verify: 依存タスク並列実行で clippy と test が aws-lc-sys の build script の out/*.o を競合させ ar が失敗。cargo clean -p aws-lc-sys 後、verify-lint → verify-test → verify-static をシリアルに実行して全部通過。あわせて scripts/tests/audit-approval-docs.test.sh の cond3 初版（`action_snapshot`/`approval_decision` の語単独）が既存本文 03 ch03-s11 にあり最初から通ったため、追記文でだけ成立する文言（主体の行への外部キーで参照せず）に書き直した。
2. 曖昧だった文: 「docs/technical/v0.7 は保全対象のため、更新は v0.7 の検証（verification）の手順に従うこと。」（Issue #490 本文「背景・前提」）
3. 仮定と解釈: 「削除された主体を表す形が pelway-contract に置かれる」は仕様への記述であって実装は「対象外: 監査の表の実装と移行」に含まれると解釈した。選ばなかった解釈: 本 PR で crates/pelway-contract に契約型を追加する。
4. 足りなかった情報・検証手段: verify.py が書き換える verification/relative-links.json と static-checks.json をコミットするかの規則。git log（5f2ad1cb の導入時のみで以後の更新なし）からコミットしない運用と推定した。
5. 改善案: mise.toml の「[tasks.verify]」の説明に「並列で cargo を二重実行すると aws-lc-sys 等の build script out/ の競合で失敗することがある。失敗時は cargo clean -p aws-lc-sys 後にグループをシリアルに再実行」を足す。

使ったスキル: implement-issue

## 実装者メモ（round 2）

対象 head: b2e71fbb5a98daf954ba1a840e5a77c6b2af996a

1. 詰まった箇所: docs/technical/v0.7/body/03-ai-bot-runtime.md:121: codex レビュー（round 2）の must-fix。追記文の「〜だけを残す」の列挙が節冒頭の保存項目の定義と別になり、保存項目が2つに割れていた。両者を「節冒頭の定義（actor、承認snapshot、policy版、操作、結果、時刻、correlation ID）＋事象の種類」の一つの出力に統合し、actor→仮名識別子・snapshot→版と内容hash・操作→logical operation ID の保存形を明示して解消した。
2. 曖昧だった文: 「監査はDBの専用書込権限による追記型とし、actor、承認snapshot、policy版、操作、結果、時刻、correlation IDを保存する。」（docs/technical/v0.7/body/03-ai-bot-runtime.md ch03-s08 の既存文）— round 1 でこの列挙を前提にしたまま別の列挙を書いたことが must-fix の原因だった。
3. 仮定と解釈: claude_review への委託が 2 回連続で failed（sandbox の read-deny が ~/.config/pelway-agent と ~/.config/fnox を拒否しレビュー App トークンを発行できない）のため、must-fix の修正・push・ラウンドサマリーまでは自分で進め、その後のレビュー再委託の段階で止まる条件としてエスカレーションすると解釈した。選ばなかった解釈: codex_review だけへの再委託（指揮役記録の「2 担当への再委託」を実装側の解釈で差し替えることになるため）。
4. 足りなかった情報・検証手段: claude_review のレビュースレッドの sandbox で ~/.config/pelway-agent・~/.config/fnox を読めるようにする設定手段。レビュー環境側の設定で実装側からは変更できない。
5. 改善案: docs/dev-environment.md の「レビュー担当の Claude」に「レビュースレッドの sandbox が ~/.config/pelway-agent と ~/.config/fnox を読めること」の確認項目を足す。

使ったスキル: implement-issue

## 実装者メモ（round 3）

対象 head: bd5fc991967a06f58ddfc38c1640f4567c91c92a

1. 詰まった箇所: docs/technical/v0.7/body/03-ai-bot-runtime.md:124: codex レビュー（round 3）の must-fix。round 2 の修正で保存項目を「節冒頭の定義＋事象の種類」に統合したが、04 の `audit_events`（actor、subject、操作、結果、snapshot hash、policy版、日時、prev_hash）と ADR 0007 の輸出項目（workspace_id、event ID、`hash`・`prev_hash`）が含まれず、正本との参照関係が切れていた。保存項目の正本を 04 と ADR 0007 へ明示参照させ、correlation ID と事象の種類をこの節の追加、仮名化・最小化を既存項目への適用として定義し直した。
2. 曖昧だった文: 「監査はDBの専用書込権限による追記型とし、actor、承認snapshot、policy版、操作、結果、時刻、correlation IDを保存する。」（docs/technical/v0.7/body/03-ai-bot-runtime.md ch03-s08 の既存文）— 03 の節冒頭の列挙だけを保存項目の定義に使うと 04・ADR 0007 の正本（subject・hash・prev_hash 等）と食い違う構造になっていた。
3. 仮定と解釈: レビューの実効回数は round 2・round 3 の 2 回（round 1 の devin は指揮役により無効）のため「3 回目のレビュー結果に must-fix が残った」の停止条件には該当せず、開発リードの「修正→再レビュー→QA へ」の明示指示により round 4 の委託まで進めると解釈した。effort は round 3 と同じ max、モデルは main #563 の新規則（command-team「Codex のモデル」）で高リスクの gpt-6.2-sol とした。
4. 足りなかった情報・検証手段: #563 導入前に読んだ implement-issue 手順 8 の担当地表には codex_review のモデルが gpt-6-luna 固定で書かれていたため、main 取り込み後に command-team 側の新規則（追加行・高リスク・回数によるモデル選択）を読み直す必要があった。
5. 改善案: .agents/skills/implement-issue/SKILL.md の「手順 8」の担当地表に「codex のモデルは command-team『Codex のモデル』の表から選ぶ」の参照を足す。

使ったスキル: implement-issue
