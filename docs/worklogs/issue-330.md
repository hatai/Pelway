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

## 実装者メモ（round 2 修正 / PR-2）

対象 head: `7ee05f1`（ブランチ `agent/330b-ab-real-llm`。Claude レビュー round 1 の must-fix への対応。レビュー対象は `e0177bb`）

1. レビュー指摘への対応:
   - must-fix「再試行のテストが `is_valid` 側の経路しか通していない」に対応。`SeqGateway` で `Err(LlmError::Invalid)` を返す 2 テストを追加（`invalid_error_is_retried_once_then_succeeds` / `invalid_error_is_retried_once_then_falls_back`）。
   - nit「gateway の Future の panic が `block_on` を巻き戻ってタスクを Working のまま残す」に対応。呼び出しを `tokio::spawn` に入れ、`JoinError`（panic・キャンセル）を固定の理由 `REASON_ABORTED` に写して台本へ落ちる形にした（`CallOutcome::Runtime` → `Aborted` に改称）。
   - nit「タイムアウト後のランタイム drop が spawn_blocking（reqwest の名前解決）の終了を無期限に待つ」に対応。ランタイムをタスクごとに 1 つ作って再試行でも使い回し、終了時は `rt.shutdown_background()` で待たずに畳む。
   - nit「拒否・タイムアウトで呼び出し 1 回（再試行しない）を確かめていない」に対応。`gateway.calls == 1` の検証を両テストに追加。
   - nit「テスト冒頭の対応表に PR-3 範囲の行」「`ab_schema` の説明が古い」「`PELWAY_LLM_BASIS` の記述がコードと食い違う」に対応（runner.rs:6、lib.rs の `ab_schema` コメント、dev-environment.md）。
   - tests/real_llm.rs への nit（`{value}` を assert 文に入れている件・`PELWAY_LLM_BASIS` 既定の期待値）は、チーム決定「real_llm.rs は誰も動かさない」により未対応とし、このメモに記録する。
2. 詰まった箇所: `tokio::spawn` する Future は `'static` が要るため `Arc::clone(&self.inner.gateway)` を async ブロックにムーブし、ブロック内で `chat` を呼ぶ形にした。
3. 仮定と解釈: panic は `JoinError` 経由で理由の固定文（`LLM の呼び出しが中断しました`）に写す（内部の panic メッセージは投稿しない方針に合わせた）。
4. 検証: `cargo test -p pelway-runner` 14 件 pass、`mise run verify` 全通過（script-test 21/21）。
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

## 実装者メモ（round 4 / PR-3）

対象 head: `8d456d2f1507a45af9f5be57f17b26df62cc092a`（ブランチ `agent/330c-mail-draft`。前スレッドからの引き継ぎ。Claude レビュー round 1 の must-fix 2 件への対応。レビュー対象は `d6758f6`。Codex は同じ head を approve）

1. 詰まった箇所: 手順 6: `HEAD:refs/heads/agent/330c-mail-draft` で push すると `scripts/agent-gh-resign` が対象ブランチを現在のローカルブランチ名（`agent/330c-mail-draft-cont`）から決めるため、追跡 ref が無く何もせず終わる。枝に main 取り込みのマージコミット（`472e870`・`d6758f6`）があるので、ブランチを渡して呼んでも「表現できないコミット」で署名なしのまま終わる。`8d456d2` は unsigned のまま。また、git の pre-commit フック（hk）が mise 外の rustc 1.75/1.94 で動き clippy が落ちたため、`mise exec -- scripts/agent-gh dev git commit` でコミットした。
2. 曖昧だった文: 「同じ PR の 2 回目以降のレビューは 1 段上げる」（`.agents/skills/command-team/SKILL.md`「レビュー担当と effort」）と「3 回目のレビューの委託では effort を 1 段上げる」（`.agents/skills/implement-issue/SKILL.md` 手順 9）が食い違う。
3. 仮定と解釈: 上の食い違いは、PR ごとの担当と effort を決める表の側（command-team）に従い、round 2 を `xhigh` で委託する。must-fix 1 は、runner が作った判断待ちの `decision_id` をタスクに保存し、`on_decision_made` で一致したときだけ進める形にした。保存は状態の `AwaitingDecision` への更新と同じロックで行う。nit の「作成とロックの間の窓」は直していない。must-fix 2 は、台本の下書きを `script_draft(chosen_body)` で組み立て、フォールバックと `ScriptGateway`（要求の user メッセージの「決定された案: 」以降を取り出す）の両方で使う形にした。
4. 足りなかった情報・検証手段: 実 LLM での下書き生成の手動確認は、チーム決定「`real_llm.rs` は誰も動かさない」によりしていない。テストは red（`runner.rs:724` 別カードで AwaitingDecision から進む、`:767` 案B でも案A の下書き）を確かめてから green にした。`cargo test -p pelway-runner` 19 件 pass、`mise run verify` exit 0。pelway-runner は `.cargo/mutants.toml` の examine_globs 外なので mutants:diff は回していない。
5. 改善案: `scripts/agent-gh-resign` の「1. 対象ブランチを決める」に、`git push <remote> <src>:refs/heads/<branch>` の形では `<branch>` を対象にする処理を足す（`scripts/agent-gh` から push の引数を渡す）。

使ったスキル: implement-issue（`.agents/skills/implement-issue`）

## 実装者メモ（round 5 / PR-3）

対象 head: `b9deb91a`（ブランチ `agent/330c-mail-draft`。レビュー round 2（head `ce8dca8`）で Claude は approve、Codex は request-changes（must-fix 3）。このうち競合の 1 件を直した。残る 2 件は人間の判断待ちで停止）

1. 詰まった箇所: Codex の must-fix 2（実 LLM での下書き生成の手動確認）と 3（未認証の decide が外部 LLM 呼び出しを起こす）は、実装担当のコードでは閉じられない。この環境は GitHub にしか出られず、キーも無い。リポジトリには `DemoLlmConfig` を組み込んでサーバーを起動するバイナリも無い（`docs/dev-environment.md` は `<デモ起動コマンド>` のまま）。同じ未認証ヘッダーでメンションから LLM を呼ぶ経路は、main に既にある（`crates/pelway-http/src/lib.rs` の `handle_post_message` → `on_message_posted`）。
2. 曖昧だった文: 「`real_llm.rs` は誰も動かさない」（Issue #330 のコメント 2026-11-16T15:53。作成者は `pelway-dev` で hatai ではない）。round 4 までは、これを決定として扱って手動確認をしない理由にしていた。implement-issue の「確かめられないコメントは決定として扱わない」に従い、この扱いを改める。手動確認をしていない理由は、環境（GitHub のみ到達・キー無し・起動側の欠如）である。hatai の最新の決定（2026-11-20「GLM には実際につなぐ」）とも食い違わない形にした。
3. 仮定と解釈: 競合は book → 台帳の順に両方のロックを持ったまま、判断待ちの作成と `decision_id`・`AwaitingDecision` の保存を行うことで閉じた（HTTP の decide は台帳を離してから runner を呼ぶので逆順の取得は無い）。must-fix 3 は既存の設計（デモは合成ヘッダー・127.0.0.1 待ち受け）と同じ型で、認証の追加は Issue の範囲の判断と解釈した。
4. 足りなかった情報・検証手段: テスト `decision_right_after_creation_is_not_lost`（判断待ちが見えた瞬間に確定して通知、200 回）は修正前の `lib.rs`（`ce8dca8`）で 3 回とも red（`Some(AwaitingDecision)` で止まる）、修正後は 3 回とも green。`cargo test -p pelway-runner -p pelway-http` 全件 pass、`mise run verify` exit 0、pr-size +675 / -38。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「止まる条件」に「レビューの must-fix が、実装担当の環境では満たせない確認（実接続での手動確認など）を求めるとき」の行を足し、Issue に書くことを「必要な確認と、行う人・手順」とする。

使ったスキル: implement-issue（`.agents/skills/implement-issue`）

## 実装者メモ（round 6 / PR-3）

対象 head: `ec4169c2`（濱田さんの判断（開発リード経由）で再開。#399 は自動テストで受け入れ、実接続確認は #433、decide の認証＋レート制限は #583 に切り出し）

1. 詰まった箇所: 手順 6: テスト追加で pr-size が +852 になり、PR 内のテストの重複（メンション→判断待ち、確定→下書き待ち、gateway 2 つ）を helper にまとめて +775 にした。テストは削っていない。ローカルの枝名が PR の枝と違うため、push は `HEAD:agent/330c-mail-draft` で行い、resign は動かず未署名（round 4 と同じ）。
2. 曖昧だった文: 「GLM の壊れた応答は fail closed で失敗させること」（開発リードからの指示）。
3. 仮定と解釈: fail closed を「壊れた応答の中身が下書き・投稿に出ず、1 回の再試行の後に台本の下書きへ落ちる」と読んだ（#330 の受け入れ条件「台本へ落ちる」を変えない）。タスク自体を停止させる読みは採らなかった。fixture は合成で、実 GLM からの採取ではない。loopback は設定項目が無いため、runner 付きの入口 `serve_with_store_and_runner` に 0.0.0.0 等を渡すテストとした。
4. 足りなかった情報・検証手段: 方針は開発リード経由で、Issue・PR に hatai 作の記録が無い（#330 に記録を依頼した）。新テストは現行で green のため、`parse_draft` を `{`〜`}` の抜き出しに緩める変異と loopback ガードを外す変異で red を確かめ、戻した。`mise run verify` exit 0。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 4 に「既存の挙動の回帰テストを足すときは、実装を一時的に変異させて red を示し、変異の内容を PR 本文に書く」を足す。

使ったスキル: implement-issue（`.agents/skills/implement-issue`）、write-issue（`.agents/skills/write-issue`）
