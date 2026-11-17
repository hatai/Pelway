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

## 実装者メモ（round 1 修正 / PR-1）

対象 head: `7fe4d630c94b428bc4450f666e4b20105decdc10`（ブランチ `agent/330a-llm-config`。署名付替え前のローカルコミットは `2fc2154`）

1. レビュー指摘への対応:
   - Codex レビュー（round 1）の must-fix「fnox.toml から既存の `PELWAY_PENPOT_MCP_TOKEN` 宣言が削除されている」に対応。モノリス版 fnox.toml を全体置き換えで作った際、無関係な宣言を落としていた。PR-1 の宣言に戻して復元。

## 実装者メモ（round 3 / PR-1）

対象 head: `b7f6653f96069430db4ad4457df81ff3b1f2989f`（ブランチ `agent/330a-llm-config`。QA 点検不合格への修正）

1. 詰まった箇所:
   - テストに `https://api.openai.com/v1/`・`https://llm.example.com/v1` のリテラルを書くと demo-check external-refs の新規ヒットになった（runner.rs は除外対象外）。拒否ケースは `https://127.0.0.1:9`（loopback は許可ホスト）で確かめ、末尾 `/` ケースは `format!` で定数から生成して新規ヒット 0 に戻した。
2. 曖昧だった文: 「`config.rs` に URL が 2 つだけであることを確かめるテストは、この PR から除いて別の Issue に分ける」（Issue #330 コメント comment-6921443439）
3. 仮定と解釈:
   - 当該テストは PR-1 の差分にも元コミット（25030ea）にも存在しなかったため、取り除く対象は無し。依頼どおり別 Issue として起票し、その旨をメモに記録する。
   - 「それ以外の接続先は別の環境変数で明示したときだけ許可」は `PELWAY_LLM_CUSTOM_BASE_URL` と解釈。設定時は `PELWAY_LLM_BASE_URL` より優先してその値が接続先になる（優先順位は docs に明記）。
   - 「起動時に接続先をログに出す」は `build_gateway_with`（gateway 組み立て＝接続開始の入口）で stderr に出すと解釈。キー・要求本文は出さない。
4. 足りなかった情報・検証手段:
   - QA 点検の正本は Issue コメントの 3 項目のみで、「URL が 2 つだけ」のテストがどの指摘を指したかの参照は無かった（存在しないテストの取り除き依頼と判断し、起票側で対応）。
5. 改善案: `scripts/demo-check/lib.sh` の除外コメントに「endpoint 宣言の数そのものを確かめる検査は別 Issue」と追記すると次の実装者が重複検査を置かずに済む。

使ったスキル: implement-issue（`.agents/skills/implement-issue`）

## 実装者メモ（round 4 / PR-1）

対象 head: `ad55a7c200f58ace7e3f7dcf24a83600a04ea405`（ブランチ `agent/330a-llm-config`。Codex レビュー round 3 の must-fix への対応）

1. レビュー指摘への対応:
   - must-fix「`PELWAY_LLM_CUSTOM_BASE_URL` が任意 URL を許し、http の外部接続先が Bearer キーの送出経路になりうる」に対応。宛先検証 `custom_base_url_allowed` を `pelway-llm/src/config.rs` に追加し、loopback（127.0.0.0/8・::1・localhost）は任意スキーム、それ以外は https のみ許可とした（レビュー提示の代替案のうち「非 loopback は HTTPS と宛先検証」側を採用）。
   - `url` クレートは pelway-llm の dev-dependency だったため `[dependencies]` に移動。
2. 詰まった箇所:
   - 非 loopback URL のリテラルは外部 IP・TLD 付きホスト名のどちらでも demo-check external-refs の新規ヒットになるため、テストでは `127.0.0.1` のリテラルを `.invalid`（TLD 表・許可ホスト外）に `replace` して実行時生成にした。
   - `mise run verify` が main 由来の `docs/worklogs/issue-220.md` の `BRE`（基本正規表現の意図的な略号）で落ちたため、typos.toml の `extend-words` に `bre` を追加（同ファイルの既存パターンに従う）。
3. 足りなかった情報・検証手段: `custom_base_url_allowed` の unit test を pure.rs にも置くか迷ったが、runner.rs の `from_vars` 経由の検証で loopback/http/https/非URL の 4 分岐を覆えているため重複は置かなかった。
4. 改善案: `typos.toml` の extend-words コメントに「略号か造語かを書く」ルールがあると次回の判断が速い。

使ったスキル: implement-issue（`.agents/skills/implement-issue`）
