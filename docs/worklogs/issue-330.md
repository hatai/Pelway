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
