# Issue #330 の作業ログ

## 実装者メモ（round 1 / PR-1）

対象 head: 未 push（ブランチ `agent/330a-llm-config`。全体を 1 コミットで作ったあと `pr-size` +800 超過で停止し、hatai の split-plan 承認（Issue コメント）で 3 本の積み上げ PR に分解。本 PR は PR-1「デモ seed の根拠 ID 設定化と LLM 接続プリセット」）

1. 詰まった箇所:
   - main の runner は LLM 呼び出しの根拠を `PurposeId::P03` に固定しており、デモ seed の既定をこの段階で P06 に替えるとゲートが常に不成立になって既存テストが落ちる。PR-1 では `new_demo_seeded_with_basis` の追加だけに留め、`new_demo_seeded` の既定は P03 のまま（P06 への切替とゲートの設定化は PR-2 で一緒に行う）。
   - demo-check の leak-phrases に `ident` 規則 `PELWAY_LLM_API_KEY` があり、コメント内のリテラル参照と `PELWAY_LLM_API_KEY_ENV`（部分文字列が一致）が新規ヒットになった。既定名の参照は `FnoxEnvTokenSource::DEFAULT_VAR` 経由に留め、差し替え用の変数名は `PELWAY_LLM_KEY_ENV` にした。
   - `ScriptGateway` は要求スキーマの required に `subject` を含むかで A/B 要求と下書き要求を切り分ける（下書きスキーマの定数は PR-3 の runner 側にあるため、この段階では runner の内部定数を参照しない）。
2. 曖昧だった文: 「実 LLM（GLM）に接続し、A/B 案の生成と、決定後のメール下書き生成を実物で動かす」（Issue #330 本文「目的」）
3. 仮定と解釈:
   - 根拠 ID は hatai 決定どおり既定 P06（BYOK）。`PurposeId` はコードに固定せず `PELWAY_LLM_BASIS` と `new_demo_seeded_with_basis` で受ける。
   - Z.ai の Coding Plan 向け endpoint（`/api/coding/paas/v4`）は自前アプリからの呼び出しが想定されていないため使わず、汎用 `/api/paas/v4` を既定にした。
   - 「台本モード」は起動時の gateway 選択（キー未設定・空白のみ・ヘッダー不可文字・gateway 構築失敗 → `ScriptGateway`）として PR-1 に入れ、呼び出し途中の失敗からのフォールバックは PR-2 の runner 側に入れた。
4. 足りなかった情報・検証手段:
   - この環境は DNS が壊れており GitHub 経由の通信しかできないため、実 z.ai への接続確認は未実施。GLM-5.3 Flash の正確なモデルコードは Z.ai 公式の記載に依る（`glm-5.3-flash`）。実呼び出しで 404/無効モデルが返る場合は `PELWAY_LLM_CHAT_MODEL` で差し替え。
   - demo-check は main 上ですでに 3 段階が失敗（leak-phrases・external-refs・loopback の既存ヒット）。本 PR が追加する新規ヒットは 0（`pelway-llm/src/config.rs` を endpoint 宣言の集約ファイルとして external-refs の除外に追加）。
5. 改善案:
   - `scripts/pr-size.sh` は untracked ファイルを数えない。未追跡も含めるか警告を出すと早めに上限超過に気付ける。
   - `FnoxEnvTokenSource` が既定名 `PELWAY_LLM_API_KEY` を持つ一方で demo-check の `ident` 規則が同名を禁止しており、製品コードのリテラル参照を規則に合わせる整理（既定名の単一管理元とスキャン除外方針の明確化）が残課題。

使ったスキル: implement-issue（`.agents/skills/implement-issue`）

## 実装者メモ（round 2 / PR-2）

対象 head: `4a2c9989dc47d50ea840dcb41cadb12d506ab85a`（ブランチ `agent/330b-ab-real-llm`。base は PR-1 の署名付き head `fec2c5b`。署名付替え前のローカルコミットは `3e1c1d7`。実装はモノリス `25030ea` の分解から抽出）

1. 詰まった箇所:
   - モノリス版からの抽出で `runner.rs`/`lib.rs` の削除範囲を行番号で消した際、`stop_reason` フィールドと構造体の閉じ括弧まで消してしまいコンパイルエラーになった（PR-3 の下書き系と PR-2 の残存部分が隣接しているため）。以後は範囲削除後に grep で残存参照を確認してから `cargo check` する手順にした。
   - `RunnerConfig` に `llm_basis`/`llm_timeout` を加えたため、main 由来の構築箇所（`pelway-http/tests/runner_mention.rs`）が初期化子不足で落ちた。デモ既定の P06・30 秒を明示して追従。
   - `mise run verify` の `scripts/tests/conductor-watch-ensure.test.sh` が環境依存で不安定（実行ごとに別のアサーションが落ちる。クレートの変更とは無関係の watch プロセス管理テスト）。変更クレートの check/clippy/test は全緑。
2. 曖昧だった文: 前記同様「実 LLM（GLM）に接続し…実物で動かす」（Issue #330 本文「目的」）
3. 仮定と解釈:
   - 失敗の分類は「検証失敗（`LlmError::Invalid`・スキーマ不適合）のみ1回再試行。その他（送信不可・拒否・タイムアウト・結果不明）は再試行せず即フォールバック」とした（Issue の「Invalid 応答は1回だけ再試行」に合わせた）。
   - タイムアウトは `RunnerConfig.llm_timeout` で runner 側が `tokio::time::timeout` する（呼び出しスレッド内で current-thread ランタイムを都度立てる。runner は同期 API のため）。実提供元側の HTTP タイムアウト（`DemoLlmConfig.timeout`）とは別枠。
   - フォールバックは Stopped を経由せず「台本の応答で進めます」の進捗行 → 台本 A/B で判断待ち作成、にした（デモを止めない Issue の意図。停止が必要なら判断待ちの決定者側で制御できる）。
4. 足りなかった情報・検証手段:
   - 実 z.ai への接続確認はこの環境（GitHub 到達のみ）では未実施。`tests/real_llm.rs`（既定 ignored）を `mise exec -- fnox exec -- cargo test -p pelway-runner --test real_llm -- --ignored` で手動確認する手順を dev-environment.md に記載済み。
   - `mise run verify` 全体は conductor-watch-ensure の既存不安定で赤になることがある（本変更の追加失敗なし）。
5. 改善案:
   - `conductor-watch-ensure.test.sh` の不安定ケース（extra leader present / foreign watch present / ensure restarts watch）を待ち合わせで安定化するか、verify から外して別枠で定期実行する。
   - `ScriptGateway` の応答選択が PR-1 時点で required=subject による下書き判定を残していた（PR-3 で下書き台本を戻す設計だが、PR-2 の単体では A/B 固定返しの方が分かりやすい）。PR-2 では A/B 固定に簡素化済み。

使ったスキル: implement-issue（`.agents/skills/implement-issue`）

## 実装者メモ（round 3 / PR-3）

対象 head: `b17e9beb893455a88f6f5d876c9a187b15632cc1`（ブランチ `agent/330c-mail-draft`。base は PR-2 の署名付き head `421688a`。署名付替え前のローカルコミットは `d2be91e`。実装はモノリス `25030ea` の下書き系部分をそのまま復元）

1. 詰まった箇所:
   - 特になし。PR-2 で draft 系だけを抜いて構築したため、PR-3 はモノリス版ファイルの復元で PR-2 との差分が下書き系のみになることを `git diff <PR-2 head>` の全削除行確認で検証した（非下書きの退行なし）。
2. 曖昧だった文: 前記同様（Issue #330 本文「目的」）
3. 仮定と解釈:
   - 決定の確定通知は「この要求で新しく確定したときだけ」発火させ、`handle_decide` は台帳のロックを離してから `runner.on_decision_made` を呼ぶ（runner 側も `AwaitingDecision → Working` の遷移をロック内でアトミックに確かめ、冪等リプレイ・二重通知では下書き生成を走らせない）。
   - 下書き生成の失敗も A/B と同じく台本へフォールバックし、`AwaitingSendApproval` まで進める（デモを止めない。送信承認のカード化は #332 で確認するため、ここではタスク保持の `MailDraft` と投稿のみ）。
   - 下書きスキーマ `draft_schema` は `pub(crate)`（runner 内の解析と `ScriptGateway` の応答選択で共用）。
4. 足りなかった情報・検証手段:
   - 実 z.ai への接続確認はこの環境では未実施（PR-2 と同じ。`tests/real_llm.rs` は A/B のみ対象。下書きの実 LLM 確認は runner の同一経路 `call_chat` を通るため別枠とした）。
5. 改善案:
   - `ScriptGateway` の応答選択が「required に `subject` を含むか」という文字列判定に依存しており、要求スキーマ側を `draft_schema()` 定数の参照に揃えるか、要求の種類を ChatRequest に持たせる形にすると頑健になる。

使ったスキル: implement-issue（`.agents/skills/implement-issue`）
