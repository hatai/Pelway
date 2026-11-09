# 作業記録: クライアント（client）

- 担当: Pelway Rust クライアント担当
- worktree: `/home/hatai/.t3/worktrees/Pelway/demo-t3-client`
- branch: `demo-t3/client`（base `f32ca74`）
- 時刻はすべて日本時間（JST, UTC+9）
- データは合成データのみ。実データ・個人情報・実メッセージ本文は使わない。

## 記録の書式

各作業枠ごとに 1 項目を追加する。`glm:` には GLM（OpenCode + zai-coding-plan/glm-5.3）の使用有無を書く（`none` / `short` / `long`）。

```
### YYYY-MM-DD 枠名
- start: YYYY-MM-DD HH:MM JST
- stop:  YYYY-MM-DD HH:MM JST
- glm: none | short | long（T3 thread ID）
- 内容:
- 検証（コマンドと exit code）:
- 未検証・未決:
```

## 記録

### 2026-11-09 第1枠（UI なしの中核ロジック）
- start: 2026-11-09 16:04 JST
- stop:  2026-11-09 16:20 JST
- glm: long（T3 Code スレッドで OpenCode + zai-coding-plan/glm-5.3 が実装）
- 使った skills:
  - `~/.agents/skills/tdd/SKILL.md`（赤→緑。モジュールごとに縦スライスでテストを先に書き、実装前の失敗を確認してから実装）
  - `~/.agents/skills/domain-modeling/SKILL.md`（状態を型で表す: `Phase`・`Effect`・`Notice`・`ReloadReason`・`BugReason`。GLOSSARY.md はこの枠では新規確定した用語が「確認ラウンド」程度のため未作成）
  - `~/.agents/skills/verification-before-completion/SKILL.md`（完了報告の前に全コマンドを再実行して exit code を確認）
- 内容:
  - 新クレート `crates/pelway-client-core`（ライブラリ、外部依存なし・標準ライブラリのみ、Slint 非依存）。
  - `src/contract.rs`: `4564e88`（`demo-t3/backend` の `crates/pelway-contract`）からの写し。写した範囲はコメント本文の検査（`COMMENT_MAX_CHARS = 4000`、`comment_len`＝コードポイント数、`validate_comment`、`CommentError`）、承認 API の要求・応答型（`SaveCommentRequest`・`CommentSaved`・`ApproveRequest`・`ApproveOutcome`）、`ApproveError`（エラーコード `version_conflict`+`current_version` / `snapshot_mismatch` / `not_awaiting_approval`+`state` / `approval_in_progress` / `idempotency_key_reused` / `card_id_mismatch` / `card_not_found`）、ID 型（`CardId`・`CommentId`・`SimulatedSendId`・`IdempotencyKey`）、`TaskState`。値と数え方は不変。このクレートは serde を使わないため、ワイヤのタグ名（snake_case）は `ApproveError::code()` が同じ値を返す（`http_status()` も写しのまま）。ファイル先頭に「4564e88 の写し。統合のときに差し替える」を置いた。`ApprovalCard`・`CardComment`・`ActionKind`・`MemberKind`・`snapshot.rs`・`ledger.rs` はこの段階で不要のため写していない（統合時に本体ごと差し替え）。
    - 注記: この初回記録の写し元 `4564e88` は、のちに第2枠で `99ee013` に差し替えた。現在の写し元は `99ee013`（第5枠で git により確認済み）。
  - `src/input.rs`: キー操作の判定。`Focus`（コメント欄・承認ボタン・メッセージ入力欄）×`Key`（Enter・Space・Esc）×`Modifiers`（Ctrl/Cmd・Shift）×`ImeState`（変換中・確定済み）×候補一覧の開閉から `Action`（改行 1 つ・何もしない・承認 1 回・候補を 1 つ入れて一覧を閉じる・一覧を閉じるだけ・送信 1 回）を返す純粋関数 `on_key`。QA-APR-01・QA-MEN-01 をそのまま仕様表にしたテストを `tests/input.rs` に置いた。
  - `src/approval.rs`: 承認カードの状態機械。通信は書かず `Effect`（`Approve`・`SaveComment`・`Notice`）を返す。冪等キー生成は `KeySource` trait、時計は `Clock` trait（`now_ms(&self)`・ミリ秒）で注入。内部状態 `Phase`（`Idle` / `AwaitingResponse` / `RecheckScheduled` / `CannotCheck` / `NeedsReload` / `ClientBug` / `Approved`）。
    - 押すたびに新しいキーを 1 つ作る。同じキーの再利用は（a）`approval_in_progress` の確認、（b）確認あきらめ後の再押下での確認、のときだけ。
    - `approval_in_progress` は同じキーで確認を最大 3 回、1 秒・2 秒・4 秒の間隔（`poll()` が期限を見て確認要求を返す）。3 回目の応答でも決まらなければ `Notice::CannotCheckApproval`（処理中のため確認できません）を出してボタンを押せる状態へ。再押下は同じキーで確認（二重送信しない）。
    - 409 の `version_conflict` / `snapshot_mismatch` / `not_awaiting_approval`（と 404 `card_not_found`）は `NeedsReload` へ。自動再送なし。`reload(card)` で `Idle` に戻り、次の承認は新しいキー。
    - 422 の `idempotency_key_reused` / `card_id_mismatch` は `ClientBug`。再送しない（`reload` での復帰のみ）。
    - コメント保存は `validate_comment` を先に通し（空白・4001 コードポイント以上は API を呼ばない `Reject`）、成功応答 `CommentSaved` の `snapshot_hash` だけを差し替える。`card_version` は表示中の値を承認に送る（コメント保存では版は変わらない）。
    - 承認の通信中（応答待ち・確認スケジュール中・確認あきらめ後）は `comment_editable() == false` とし、コメント API を呼ばない。コメント欄はコメント API だけ、承認は承認ボタンからだけ。
  - TDD の赤→緑の記録:
    - contract: テスト（12 件）を先に書き `cargo test` は E0432（未実装）で exit 101 → 実装 → 12 件緑。ただし国旗 4000 cp テストの補助アサーション（バイト数の期待値）が実装前に誤っていた（地域指示子は 4 バイト/文字のため国旗 2000 個は 16000 バイト）ので、テスト側を 8000→16000 に修正して緑。
    - input: テスト（13 件）を先に書き exit 101（E0432）→ 実装 → 13 件緑。
    - approval: テスト（18 件）を先に書き exit 101（E0432）→ 実装 → 借用の E0499（`Clock` を `&mut` で機械に持たせるとテストから時間を進められない）→ `Clock::now_ms` を `&self` に変更し FakeClock を `Cell<u64>` に → 18 件緑。
    - clippy: `doc_nested_refdefs`（lib.rs のモジュール一覧リンク）と `wrong_self_convention`（`to_needs_reload` → `enter_needs_reload`）を修正。
- 作ったファイル:
  - `crates/pelway-client-core/Cargo.toml`
  - `crates/pelway-client-core/src/lib.rs`
  - `crates/pelway-client-core/src/contract.rs`
  - `crates/pelway-client-core/src/input.rs`
  - `crates/pelway-client-core/src/approval.rs`
  - `crates/pelway-client-core/tests/contract.rs`（12 件）
  - `crates/pelway-client-core/tests/input.rs`（13 件）
  - `crates/pelway-client-core/tests/approval.rs`（18 件）
  - `docs/worklog/client.md`（本ファイル）
  - `Cargo.lock`（ワークスペースに `pelway-client-core` が加わった分のみ。依存の追加なし）
- 検証（コマンドと exit code）:
  - `cargo test -p pelway-client-core` 0（43 件: approval 18・contract 12・input 13、失敗 0）
  - `cargo clippy -p pelway-client-core -- -D warnings` 0（`--all-targets` 付きでも 0）
  - `cargo fmt --check` 0（`cargo fmt --all --check` でも 0）
  - `cargo test --workspace --locked` 0（dev-smoke 含む全体、失敗 0）
  - `cargo clippy --workspace --all-targets --locked -- -D warnings` 0（pre-commit と同じ形）
  - `cargo check --workspace --all-targets --locked` 0
- 未検証・未決:
  - QA-MEN-01 に明記のないキーの解釈（実装は済ませてあるが、QA とのすり合わせ待ち）:
    - メッセージ入力欄の修飾キー付き Enter（Ctrl/Cmd+Enter・Shift+Enter、一覧閉）は「何もしない」。誤送信防止で送信しない側に倒した。
    - 候補一覧が開いていても IME 変換中の Enter・Esc は IME 操作（確定・キャンセル）を優先し、候補の挿入も一覧のクローズもしない。
    - コメント欄の修飾キー付き Enter（IME 確定後）は改行 1 つとして扱う（承認も送信もしない）。承認ボタンでの修飾キー付き Enter/Space は承認しない。
  - 仕様に明記がない動作の実装判断:
    - `CannotCheck`（確認 3 回で決まらず）のあと、ボタンは押せるがコメント欄は承認未確定のため読み取り専用のままにした。
    - `NeedsReload`・`ClientBug` でもコメント欄を読み取り専用にした（コメント保存で snapshot がさらに変わるのを避ける安全側）。
    - `CannotCheck` で再押下した場合、同じキーで即座に承認要求（確認）を出し、以後の `approval_in_progress` には 1・2・4 秒を再適用する解釈。
    - コメント保存の応答待ち中に承認ボタンを押した場合、その時点の `snapshot_hash` で承認要求を出す（特別扱いなし。サーバー側の 409 判定に任せる）。コメント保存の応答待ち状態は持たず、`on_comment_saved` が来るまで複数回保存要求を出せる。
    - `ApproveError::CardNotFound`（404）は依頼のリストにないが写し元の enum にあるため `NeedsReload` 扱いにした。
    - コメント API のエラー応答（`CommentError::CardNotFound` 等）の表示は呼び出し側（UI）の仕事とし、状態機械には成功応答（`on_comment_saved`）だけを用意した。
  - Slint UI（第2段階以降）、実時間 `Clock`・UUID `KeySource` の実装、HTTP 通信層は未実装（この枠の対象外）。
  - U01〜U07 の未決事項はこの枠では決めていない（`ui-pro-draft-v0.1/tokens/pelway.tokens.json` は不使用）。

### 2026-11-09 第2枠（写し元を 99ee013 へ差し替え）
- start: 2026-11-09 16:22 JST
- stop:  2026-11-09 16:31 JST
- glm: long（T3 Code スレッドで OpenCode + zai-coding-plan/glm-5.3 が実装）
- 使った skills: tdd / domain-modeling / verification-before-completion（第1枠と同じ使い方）
- 内容:
  - **写し元を `4564e88` から `99ee013`（ブランチ `demo-t3/backend`）に替えた。理由: 99ee013 でコメント保存のエラー `CommentError` に `not_awaiting_approval`（409、`state` 付き）が入ったため。** `git show 99ee013:...` で読み直しただけで、checkout・merge はしていない。
  - `src/contract.rs`: 先頭行を `// 99ee013 の写し。統合のときに差し替える` に更新。`CommentError` に `NotAwaitingApproval { state: TaskState }` を追加（原本のドキュメントコメントも写し）、`http_status()` に 409 を反映。serde のタグ名は `ApproveError::code()` と同じ手法で `CommentError::code()` も新設し `not_awaiting_approval` / `empty_body` / `too_long` / `card_not_found` を返す。`COMMENT_MAX_CHARS = 4000` と `comment_len` の数え方（コードポイント数）は不変。
  - `src/approval.rs`: `on_comment_error(err)` を追加。409 `not_awaiting_approval` は自動で再送せず `NeedsReload` へ移し、`snapshot_hash` は差し替えない。404 `card_not_found` も同じ扱い。クライアント側で先に弾く `empty_body`・`too_long` は状態を変えない。`request_save_comment` の事前検査に新しいバリアントは影響しない（`validate_comment` は空白・長すぎだけを返す）。
  - テスト追加（赤→緑: 追加テストのコンパイルエラー E0599/E0433/E0004 を確認してから実装）:
    - `tests/contract.rs`（+2）: `CommentError` のコード名と HTTP ステータス（`not_awaiting_approval` は 409）。
    - `tests/approval.rs`（+2）: コメント保存が 409 `not_awaiting_approval` を返したら読み直しを求める状態へ（`snapshot_hash` 不変・自動再送なし・reload で承認待ちに戻る）、404 `card_not_found` も読み直し。
- 検証（コマンドと exit code）:
  - `cargo test -p pelway-client-core` 0（47 件: approval 20・contract 14・input 13、失敗 0）
  - `cargo clippy -p pelway-client-core --all-targets -- -D warnings` 0
  - `cargo fmt --all --check` 0
  - `cargo test --workspace --locked` 0
- 未検証・未決:
  - 第1枠の未決（メッセージ欄の修飾キー付き Enter 等）は変わらず未決。
  - コメント保存の 409 で読み直したあと、未保存のコメント本文を入力欄に残すか破棄するかは UI 層の判断（この状態機械の対象外）。

### 2026-11-09 第3枠（リード決定: コメント欄は承認待ちのときだけ書ける）
- start: 2026-11-09 16:35 JST
- stop:  2026-11-09 16:44 JST
- glm: long（T3 Code スレッドで OpenCode + zai-coding-plan/glm-5.3 が実装）
- 使った skills: tdd / domain-modeling / verification-before-completion（第1枠と同じ使い方）
- **これはリードの決定（2026-11-09 16:12ごろ）の反映**:
  1. コメント欄はカードが承認待ちのときだけ書ける。承認待ち以外の状態（送信中・送信後など）ではいつも読み取り専用にし、コメント API を呼ばない。前の依頼の「承認の通信中だけ読み取り専用」はこれに置き換えた。
  2. 送ったあとに 409 `not_awaiting_approval` が返ったとき（承認でもコメント保存でも）は「再読み込みして、次の承認は新しいキー」の扱いにする。自動では送り直さない。
- 内容:
  - `CardView` に `state: TaskState` を追加。`reload` でカードごと差し替わり、承認成功（`ApproveOutcome.state`）で送信側の状態に移る。コメント保存では状態は変わらない。
  - `comment_editable()`: カードが `TaskState::AwaitingSendApproval` のときだけ true。承認の通信中かどうかは見ない（カードが承認待ちなら、承認要求の応答待ち・`approval_in_progress` の確認待ち・確認あきらめ後も書ける）。承認待ち以外のカード状態ではいつも false。加えて、読み直し待ち（`NeedsReload`）・クライアント不具合（`ClientBug`）・承認済み（`Approved`）の各フェーズでは、手持ちのカード状態に関係なく読み取り専用にする（409 の直後に同じコメント API を再送させないため。私の追加判断）。
  - `approve_button_enabled()`: 承認待ちのカードで、かつ待機中か確認あきらめ後のときだけ true（承認待ちでないカードで押させない。指示にはないが一貫性のための最小の追加）。
  - 409 `not_awaiting_approval`（承認・コメント保存の両方）: `NeedsReload` へ移し、自動再送なし。`reload` で承認待ちに戻り、次の承認は新しいキー。`snapshot_hash` は差し替えない。
  - テスト（+4、書き換え1、削除1）:
    - 追加: `comment_box_is_editable_only_when_the_card_awaits_send_approval`（9 状態ごとに書けるか・コメント API を呼ぶかを検査）、`comment_box_stays_editable_while_approving_as_long_as_the_card_awaits_approval`（承認の通信中も書ける。「承認の通信中だけ読み取り専用」の撤廃）、`approve_success_moves_the_card_to_the_send_side_and_locks_the_comment_box`、`reloaded_card_not_awaiting_approval_keeps_the_comment_box_read_only`。
    - 拡張: `not_awaiting_approval_requires_reload`（reload → 次の承認は新しいキー）、`comment_not_awaiting_approval...`（reload → 新しいキーでの承認要求を確認）。
    - 削除: `comment_cannot_be_saved_while_approving`（新仕様の逆。上の置き換えテストが担当）。
    - 修正: 通信中・確認あきらめ後のコメント欄の期待を「書ける」に更新。
  - TDD: テスト追加→赤（`CardView` に `state` なし E0560/E0609、旧期待の4件失敗）→ 実装→緑。
- 検証（コマンドと exit code）:
  - `cargo test -p pelway-client-core` 0（50 件: approval 23・contract 14・input 13、失敗 0）
  - `cargo clippy -p pelway-client-core --all-targets -- -D warnings` 0
  - `cargo fmt --all --check` 0
  - `cargo test --workspace --locked` 0、`cargo clippy --workspace --all-targets --locked -- -D warnings` 0
- 未検証・未決:
  - 承認の通信中にコメントを保存すると `snapshot_hash` が変わり、進行中の承認要求と食い違う可能性がある。その場合はサーバーが 409 `snapshot_mismatch` を返し、「再読み込みして次の承認は新しいキー」で処理する（リード決定の運用）。UI 側で保存ボタン等の制御をするかは第2段階（Slint）で決める。
  - `NeedsReload`・`ClientBug` 中もコメント欄を読み取り専用にする条件は指示に明記がなく私の追加判断。異論があれば外す。

### 2026-11-09 第4枠（第1段階レビューの指摘1・2の修正）
- start: 2026-11-09 17:30 JST
- stop:  2026-11-09 17:39 JST
- glm: long（T3 Code スレッドで OpenCode + zai-coding-plan/glm-5.3 が実装）
- 使った skills: tdd（テストを先に書き、実装前の失敗を見た） / domain-modeling / verification-before-completion。テストが先に失敗するのを見たか: **見た**（input 1 件・approval 1 件の失敗を確認してから実装。下記）。
- レビュー対応（レビュー: Codex gpt-6-luna、範囲 `f32ca74..8f331be`、指摘ファイルの sha256 `f832289272aed3c5bbc99e930a368425e18895e778571d19bf192cc1a4e9a640`）:
  - **指摘1（must-fix）→ 修正**: `on_key` は IME 変換中でも、候補一覧が開いている間（メッセージ入力欄）は Enter で `AcceptMention`、Esc で `DismissMentionList` を返すようにした（QA-MEN-01 は IME 状態を理由に例外を設けない）。いずれも送信しない。コメント欄の QA-APR-01（変換中は IME 操作を優先）は従来どおり。テスト: `composing_keys_go_to_ime_even_with_mention_list_open` を削除し、`open_mention_list_enter_and_esc_work_even_while_composing`（Composing/NotComposing 両方で Enter=AcceptMention・Esc=DismissMentionList・送信なし）と `composing_keys_still_go_to_ime_in_the_comment_box` に置き換え。赤: 修正前に新テストが失敗するのを確認。
  - **指摘2（high）→ 修正**: `press_approve` は表示中カードが `AwaitingSendApproval` でなければ `None` を返し、冪等キーを消費しない（`KeySource` を呼ばない）。テスト: `press_approve_rejects_cards_not_awaiting_approval_and_consumes_no_key`（初期カードと reload 後カード×承認待ち以外の 8 状態で `None`、その後承認待ちに戻して押すと 1 個目のキーが出ることで非消費を検証）。赤: 修正前に失敗するのを確認（`press_approve` が要求を作ってしまっていた）。
  - **指摘3（should-fix）・指摘4（nit）→ 今回は不対応**: 開発リードの判断待ちのため触っていない。
- 運用記録:
  - 2026-11-09 16:20〜16:28（JST）に GLM が 3 つ同時に動いた。当時のルール（GLM は同時に 2 つまで）では違反だった。16:55 にユーザーが「同時実行数は厳密に管理しなくてよい」と決定したため、それ以降は違反扱いとしない。
  - 色の値は画面のコードに直接書かず、トークン名で参照する方針。テーマは light / dark / light-accessible / dark-accessible の 4 つになる前提。現時点のコミットに色の値は入っていない（第1段階は UI なしのため）。
- 検証（コマンドと exit code）:
  - `cargo fmt --all --check` 0
  - `cargo clippy -p pelway-client-core --all-targets --locked --offline -- -D warnings` 0
  - `cargo test -p pelway-client-core --locked --offline` 0（52 件: approval 24・contract 14・input 14、失敗 0）

### 2026-11-09 第5枠（レビュー指摘3・4の修正。開発リード判断 17:5x JST）
- start: 2026-11-09 17:58 JST
- stop:  2026-11-09 18:08 JST
- glm: long（T3 Code スレッドで OpenCode + zai-coding-plan/glm-5.3 が実装）
- 使った skills: verification-before-completion（原本との照合をコマンドで確認してから書き直した）。テストが先に失敗するのを見たか: 未確認（この枠はドキュメントコメントとコメントの書き直しが中心で、振る舞いを変えないため既存 52 件の通過確認のみ）。
- 指摘3・4は開発リードの判断（2026-11-09 17:5x JST）で直すことになった（前枠では判断待ちとして保留していたもの）。
- 写し元を確かめたコマンドと結果:
  - `git diff 4564e88 99ee013 -- crates/pelway-contract/src/approval.rs` → 差分は (a) API 表の doc 1 行、(b) `CommentError` への `NotAwaitingApproval { state }` 追加、(c) `http_status` への 409 行追加、(d) `TooLong` の整形のみ。
  - `git diff --stat 4564e88 99ee013 -- crates/pelway-contract/src/ids.rs crates/pelway-contract/src/task.rs crates/pelway-contract/src/member.rs` → 出力なし（ids.rs・task.rs は両コミットで同一）。
  - `crates/pelway-client-core/src/contract.rs` は `CommentError::NotAwaitingApproval` を含む（99ee013 版）ため、**写し元は 99ee013**。
- 内容:
  - 指摘3: `src/contract.rs` の先頭説明を事実どおりに書き直した。99ee013 の `approval.rs` から写したもの（`COMMENT_MAX_CHARS`・`comment_len`・`validate_comment`・`SaveCommentRequest`・`CommentSaved`・`ApproveRequest`（`check_path` 含む）・`ApproveOutcome`・`ApproveError`・`CommentError`、各 `http_status` 含む）、`approval.rs` から省いたもの（`COMMENTS_PATH`・`APPROVE_PATH`・`ApprovalCard`・`CardComment`）、`ids.rs` から足したもの（`string_id!` マクロ・`CardId`・`CommentId`・`SimulatedSendId`・`IdempotencyKey`・`InvalidIdempotencyKey`）、`task.rs` から足したもの（`TaskState`。`ActionKind` は省略）、写していないファイル（`member.rs`・`message.rs`・`snapshot.rs`・`ledger.rs`）、serde 属性の代わりに `ApproveError::code()` / `CommentError::code()` がワイヤのタグ値（`snake_case`）を返すこと、を明記。
  - 指摘4: 確かめた値（99ee013）に 3 か所をそろえた。`crates/pelway-client-core/Cargo.toml`（4564e88 → 99ee013）、`docs/worklog/client.md`（第2枠の記録は 99ee013 で正しく、この枠に確かめた過程を追記）、`src/contract.rs`（先頭行は第2枠から 99ee013 のまま）。あわせて `src/lib.rs` のモジュール一覧に残っていた `4564e88` の記載も 99ee013 に直した（記載不整合の解消）。
  - ついでに `cargo doc` の `redundant explicit link target` 警告 4 件（`lib.rs` 3 件・`approval.rs` 1 件）を解消（明示リンクターゲットの削除。前回の `doc_nested_refdefs` 回避と両立する書式に）。
- 検証（コマンドと exit code）:
  - `cargo fmt --all --check` 0
  - `cargo clippy -p pelway-client-core --all-targets --locked --offline -- -D warnings` 0
  - `cargo test -p pelway-client-core --locked --offline` 0（52 件、失敗 0）
  - `cargo doc -p pelway-client-core --no-deps --locked --offline` 0（警告 0）

### 2026-11-09 第6枠（Codex 第2回レビューの指摘への対処）
- start: 2026-11-09 18:18 JST
- stop:  2026-11-09 18:28 JST
- glm: long（T3 Code スレッドで OpenCode + zai-coding-plan/glm-5.3 が実装）
- レビュー: Codex 第2回（review-findings-client-2.md、sha256 `0eab5d310c22d92ee213282c20ce2351e204643e30be1f48d5ed783722b3cd0f`）。前回指摘1〜3は「直った」、指摘4（nit）が新指摘1（should-fix）として継承。
- 使った skills: tdd（連続操作のテストを先に書き、実装変更前に実行して結果を記録）/ verification-before-completion。
- 対処:
  - 前回指摘4＋新指摘1（写し元の記載）: `tests/contract.rs` の先頭説明を「期待値は原本 99ee013 の契約と同じ値（`CommentError::NotAwaitingApproval` は 99ee013 で追加）」に直した。`docs/worklog/client.md` 第1枠の初回記録（写し元 4564e88）は過去の記録としてそのまま残し、「のちに第2枠で 99ee013 に差し替えた。現在の写し元は 99ee013（第5枠で確認済み）」の注記行を足した。
  - 新指摘2（候補一覧の連続操作のテスト）: `tests/input.rs` に2件追加。
    - `mention_accept_closes_list_then_next_enter_sends_exactly_once`: 一覧が開いた状態の Enter → `AcceptMention`（≠Send）、UI が一覧を閉じる遷移（`mention_list_open = false`）を挟み、次の Enter → `Send` ちょうど 1 回、続けて Enter しても毎回送信で候補操作に戻らない、を状態の移り変わり込みで検証。
    - `closed_mention_list_composing_enter_does_not_send`: 一覧が閉じていて変換中の Enter は `None`（IME の確定を優先・送信しない）、確定後（NotComposing）の Enter は `Send`。
  - テストが先に失敗するか: **最初から通った**（実装変更前の `cargo test --test input` で 16 件中 2 件の追加テストを含め全部合格、exit 0）。実装（`on_key`）はすでに仕様どおりで、振る舞いの修正は不要だった。
- 確かめたコマンドと結果:
  - `cargo fmt --all --check` 0
  - `cargo clippy -p pelway-client-core --all-targets --locked --offline -- -D warnings` 0
  - `cargo test -p pelway-client-core --locked --offline` 0（54 件: approval 24・contract 14・input 16、失敗 0）
- 未検証・未決:
  - `on_key` は入力状況を引数で受ける純粋関数で、一覧の開閉状態の管理（`AcceptMention` 後に閉じる等）は呼び出し側（UI）の責任。テストはその遷移をシミュレートして検証した。実 UI での結合は第2段階（Slint）で確かめる。

### 2026-11-09 第7枠（Codex 第3回レビューの指摘への対処）
- start: 2026-11-09 18:55 JST
- stop:  2026-11-09 18:59 JST
- glm: long（T3 Code スレッドで OpenCode + zai-coding-plan/glm-5.3 が実装）
- レビュー: Codex 第3回（review-findings-client-3.md、sha256 `ca569dc6c6db34ccf87ae0c8c337a8493955a327ff5ee9f6520e353615b805d8`）。前回指摘1・2は「直った」確認済み。新指摘は1件（nit）のみ。
- 使った skills: verification-before-completion（修正後に指定 3 コマンドを再実行して exit code を確認）。
- 対処:
  - 新指摘1（nit）: `tests/contract.rs` 先頭 2〜3 行目の説明で「（コードポイント数・エラーコード。」の括弧が閉じられていなかったのを、「（コードポイント数・エラーコード）。」で閉じ、次行の `CommentError::NotAwaitingApproval` は 99ee013 で追加された、を独立した文にした。前回の2件（写し元の記載・連続操作テスト）は直っているのでやり直していない。
- 確かめたコマンドと結果:
  - `cargo fmt --all --check` 0
  - `cargo clippy -p pelway-client-core --all-targets --locked --offline -- -D warnings` 0
  - `cargo test -p pelway-client-core --locked --offline` 0（54 件: approval 24・contract 14・input 16、失敗 0）
- 未検証・未決: なし（この枠の範囲で新たな未決は無し）。

### 2026-11-09 第8枠（Slint の画面の骨組み 段階A・やり直し）
- start: 2026-11-09 20:31 JST
- stop:  2026-11-09 21:05 JST
- glm: long（T3 Code スレッドで OpenCode + zai-coding-plan/glm-5.3 が実装）
- 使った skills: tdd と verification-before-completion の考え方に従った（ファイルは worktree 外のため読んでいない）。テストを先に書き、実装前の失敗を見て、完了の前に13章のコマンドを全部動かし直して出力で報告した。
- この枠は段階Aだけ（外部の crate を足さず、標準ライブラリと `pelway-client-core` だけ）。**Slint で窓を開く段階Bは手を付けていない（別指示）。**
- 経緯: 前の回のスレッド（`mcp:d1770ef8`）が誤って `demo-t3-client` に書いたため、この回（スレッド `demo-t3-client-ui-r2`、起点 `bd4cb78`）でやり直した。`demo-t3-client`・`demo-t3-client-ui` の worktree と `demo-t3/client`・`demo-t3/client-ui` ブランチには読み書き含めて一切触れていない（前の回がそこに書いた未追跡ファイルも見ていない・参考にしていない）。
- ネットワーク: 19:12 JST に crates.io からの取得が1回あったのは、ユーザー本人がそのとき1回だけ出した許可によるもの（前の回の文脈）。この回では許可を得ておらず、取得していない。cargo はすべて `CARGO_NET_OFFLINE=true` + `--offline` で動かし、lock 更新の1回だけ `--locked` を外した（下記）。
- Prior art first（AGENTS.md）について: Slint は仕様（docs/technical/v0.7/body/02-stack-comparison.md）で採用案として決まっており、WebView・Electron・Tauri を使わないことも要件なので、今回の「骨組み」作業には事例調査を当てはめなかった。GitHub・Web の検索はしていない。
- 内容:
  - 新クレート `crates/pelway-client-ui`（`publish = false`、`edition = "2024"`、`#![forbid(unsafe_code)]`、依存は path の `pelway-client-core` だけ）。`pelway-client-core` は変更していない。
  - `src/theme.rs`: テーマの enum（`Theme::Light` / `Dark` / `LightHighContrast` / `DarkHighContrast`）。**テーマ名は `light`・`dark`・`light-high-contrast`・`dark-high-contrast` の 4 つで確定**（`Theme::name()` が唯一の定義元。Slint 側 `ThemeState.current-name` は Rust から渡される文字列を受けるだけ）。第4枠（17:30 の記録）にあった `light-accessible`・`dark-accessible` は古い名前であり、`from_name` は受け付けない（テストで確認）。トークン名の一覧 `TOKEN_NAMES`（6章の指示書の一覧どおり 50 名）。**トークンの値は入れていない**。テーマごとの値の表 `token_value()` は「TODO: トークン確定後に入れる」の空の形で常に `None`（値を入れていないことをテストで確認）。
  - `src/i18n.rs`: Rust が使う Fluent キーの一覧 `USED_KEYS`（49 キー）。段階Aでは Fluent の crate を使えないため、キーの一覧だけ。**`@tr()`（gettext）は使わない判断**（仕様が Fluent のため。docs/technical/v0.7/body/06-i18n.md）。段階Bで Fluent から引いた文字列を Slint の `tr-*` プロパティへ渡す。
  - `src/testkit.rs`: 決まった値を返す `FixedKeySource`（`fixed-key-000001` 形の連番・16 文字ちょうど）と `FixedClock`（固定ミリ秒）。本物の UUID・実時間は段階B。
  - `src/state.rs`: 表示用の状態（Slint 非依存）と合成データ `sample()`。モックの架空の会社名・人名（株式会社サンプル開発・濱田・田中・佐藤・ユイ・リク・ハル・B社）だけを使い、実在の情報は入れていない。`AvatarKind`（人=円・AI=四角+印）、`AiPresence`、`TodaySection`（3区切り）、`TimelineEntry`（日付区切り・メッセージ・未読・決定の印・引き継ぎ・あなただけに表示・社外送信カード）、`ExternalSendState`（待ち/送信後）。
  - `src/bridge.rs`: 中核へのつなぎ。`ShellBridge`（フォーカス・IME・候補一覧の開閉を持ち、キー操作をすべて `input::on_key` に渡し、返ってきた `Action` だけで状態を変える。UI 側で判定を書き直していない）と `ApprovalCardBridge`（「送る」= `press_approve`、押せるか = `approve_button_enabled()`）。**`Effect` は未接続**: 通信を書かず、ログにも出さず、`last_effect` として保持するだけ。**コメント欄はモックに見た目がないため画面には置かない**。`comment_editable()` は表示用の状態としてだけ持つ（「モックにコメント欄がない」は未決）。
  - `i18n/ja/main.ftl`・`i18n/en/main.ftl`: 日英 49 キー。キーは `rail-today` 型の安定した名前。**英語は意味の分かる仮の訳**（最終文言は未決）。◆・✓・⌕・🔒 の記号は文言側（.ftl）に置き、`.slint`・Rust に利用者に見える文言を直接書かないようにした。
  - `ui/main.slint`: 画面の骨組み（rail・上の帯・今日の3区切り・チャンネル画面・右の枠・入力欄。モックにない部品と台本操作は除く）。**この回では slint に依存していないため未コンパイル・未検証**。色は `DesignTokens` global のトークン名だけで参照（値の初期値なし）。文言は `tr-*` プロパティで受ける。accessible-role/accessible-label は段階Bのため未実装。
  - モック（読むだけ）: `docs/ui-mocks/csv.html`（sha256 `b5a540ece91abc7cd0741d9daf90ec40d427cada12ad5add99862b2ee83d3220`）と `docs/ui-mocks/csv_dark.html`（sha256 `f72603d7465157723a6818d9e99b68cb831f1d0c5edb45918a940473f77e0aac`）。どちらも「試作モック、暫定承認（11/8）、最終承認は未」のため、骨組みの配置の参考にだけ使った。色の値とフォントの URL は写していない（ダーク版はライト版にダーク用スタイル上書きを足しただけの同じ構成）。
  - `Cargo.lock`: `CARGO_NET_OFFLINE=true cargo check --workspace --offline` を1回だけ（`--locked` なし）動かして更新。`git diff Cargo.lock` は `pelway-client-ui` の項目（依存 `pelway-client-core` のみ）の追加だけで、他の行は変わっていない。
- TDD の赤→緑の記録:
  - テスト4ファイル（bridge 10・i18n_keys 6・no_color_values 4・theme_tokens 6 = 26 件）を先に書き、実装前に `cargo test -p pelway-client-ui --locked --offline` を実行: **exit 101、E0432**（`pelway_client_ui::bridge` / `i18n` / `testkit` / `theme` が未実装）で4テストともコンパイル失敗。テスト側の誤り1件（`&sample4` に参照を渡し忘れた E0308）を直したうえで、改めて未実装による E0432 の赤を確認してから実装した。
  - 実装中に一度 `no_color_values_anywhere_in_the_crate` が失敗: テストファイル自身のドキュメントコメントに `rgb(` 等のリテラルが書いてあり自己検出した。コメントの書き方を直して解消（スキャナは文字列リテラルを解釈しないため、コメントにも書かないのが安全側）。
  - 色の値の混入確認: `ui/main.slint` に一時的に `#c0ffee` を1つ入れて `cargo test -p pelway-client-ui --test no_color_values` が **exit 101 で失敗**（`main.slint: ["#c0ffee"]` を検出）することを確認し、入れた値は消した。消したあとは緑（exit 0）。
- 検証（コマンドと exit code。すべて `CARGO_NET_OFFLINE=true` 付き）:
  - `cargo fmt --all --check` 0
  - `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` 0
  - `cargo test --workspace --locked --offline` 0（pelway-client-core 54 件: approval 24・contract 14・input 16、失敗 0 ／ pelway-client-ui 26 件: bridge 10・i18n_keys 6・no_color_values 4・theme_tokens 6、失敗 0 ／ pelway-dev-smoke 1 件、失敗 0）
  - `cargo build -p pelway-client-ui --locked --offline` 0（**段階Aの範囲＝Slint なしのライブラリとしてのビルド**。`ui/main.slint` は含まない）
  - `git status --short`: `M Cargo.lock`、`?? crates/pelway-client-ui/`（ほかに変わったものなし）
- 未検証・未決:
  - **`.slint` は未コンパイル・未検証**（段階Bで slint を依存に足して初めてコンパイルする。構文ミスの可能性は残る）。
  - **トークンの値は未確定**。テーマごとの値の表は「TODO: トークン確定後に入れる」の空の形。`ui-pro-draft-v0.1/tokens/pelway.tokens.json` とその生成物は不使用。
  - **足りないトークン**: 「今日」の区切りの件数バッジ（地と文字のペア）が一覧にない。`.slint` では数字だけを並記し、見た目はトークン確定後に整える。この名前は作っていない（指示どおり新規の名前は作らない）。
  - **コメント欄はモックに見た目がないため画面に置かない**（未決）。つなぎの `comment_editable()` は表示用の状態としてだけ保持。
  - **`Effect` は未接続**（通信を書かない。保持するだけ）。
  - **フォントは未決**（06-i18n.md は同梱を推奨。Noto Sans JP 等の配布ファイルとライセンスは実装前に確認する。モックの Web フォント URL は写していない）。
  - **ライセンスは未決**: Slint のライセンス許諾は U05 で未決。Royalty-free の表示義務（AboutSlint または「Made with Slint」）の要否・置き場所も未決のため、この骨組みには About 画面等を入れていない。
  - **英語の文言は仮訳**（最終文言は未決）。
  - 読み上げ（AccessKit）は段階B。NVDA・VoiceOver・Orca での確認は未検証。
  - 迷って安全側にした点: （1）検索の ⌕・決定の ◆・送信後の ✓・鍵の 🔒 を .slint に直書きせず .ftl の文言に含めた（文言の直書き禁止を記号にも広げた）。（2）`no_color_values` のスキャナは文字列リテラルを解釈しないため、コメントにも色の値のパターンを書かない方針にした。（3）承認ボタンのキー操作（Enter/Space）は中核が `Action::Approve` を返したら橋経由で `press_approve` に結びつけたが、カードが押せなければ何も起こさない（UI の無効表示だけに頼らない中核の保証をそのまま活かす）。

### 2026-11-09 第9枠（段階B-1：Codex 指摘の修正と Slint のコンパイル）
- start: 2026-11-09 21:35 JST
- stop:  2026-11-09 22:18 JST
- glm: long（T3 Code スレッドで OpenCode + zai-coding-plan/glm-5.3 が実装。起点 `1971708` の worktree `demo-t3-client-ui-r3` で実施）
- 使った skills: tdd と verification-before-completion の考え方に従った（ファイルは worktree 外のため読んでいない）。テストを先に書いて落ちることを確かめてから直し、最後に指定のコマンドを全部動かして exit code を記録した。
- 対象: Codex 第4回レビューの should-fix 1件（トークン照合が正本 JSON を読んでいない）と、Slint 追加による `ui/main.slint` の初コンパイル。指示書「Slint の画面の骨組み 段階B-1」の §2-1・§2-2 のみ。§2-3（Fluent・accessible・トークン値・KeySource・Clock・Effect・About）は手を付けていない。
- 2-1（Codex 指摘の修正）:
  - 正本の写しを `git show 97fb70d:docs/design-system/tokens/pelway.tokens.json` で読み（checkout・merge・cherry-pick は不使用）、そのまま `crates/pelway-client-ui/tokens/pelway.tokens.json` に置いた。sha256 は `4f523ce19145250df391975dea3924aa7b5bdcbd417c0c31e910f3b17c543953`（期待値と一致）。
  - `tests/theme_tokens.rs` に正本との照合テストを追加: std だけの最小の JSON 構造読み取り（オブジェクト・配列の構造だけ読み、文字列・数値の中身は読まない）で `color.<theme>` の直下のトークン名（`$` 始まりのメタキーと `mobile` などの子グループを除く、`$value` を持つもの）を取り出し、4テーマ分すべてと `TOKEN_NAMES` の一致を確かめる。**読み方の限界**: キーのエスケープは解かない（正本のキー名にエスケープなし）、`color.<theme>` の直下（デスクトップが使う普通の名前）だけが対象で `mobile` 上書き（10名・すべて直下の名前の部分集合）・`brand`/`font`/`typography`/`space`/`size`/`radius`/`border`/`shadow`/`themed`/`motion` の各グループは対象外。JSON が壊れていればパニックで落ちる。
  - 差分: 正本（62名）にあって `TOKEN_NAMES`（50名）になかった 12 名を `TOKEN_NAMES` に追加した —— `avatar-human-bg-section`・`ext-option-bg`・`ext-option-hover`・`grabber`・`line-tabbar`・`menu-bg`・`scrim`・`sheet-bg`・`sheet-inset-bg`・`toast-bg`・`toast-fg`・`track`。逆（`TOKEN_NAMES` にあって正本にない名前）は 0 件。`.slint` が参照する 37 名はすべて正本にあるため「足りないトークン」はなし（件数バッジのトークン不足は第8枠どおり未決のまま）。
  - `src/theme.rs` の説明が `ui-pro-draft-v0.1` を指していたのを、正本の写し（97fb70d 由来の `tokens/pelway.tokens.json`）を指すように直した。
  - `tests/no_color_values.rs` の走査対象に `.json` は入れていない（拡張子は rs/slint/ftl のまま。トークンの正本だけが色の値を持つ）。
  - TDD の赤→緑: テストを先に書き `cargo test -p pelway-client-ui --test theme_tokens --locked --offline` が exit 101（`token_names_match_canonical_json` が不一致で失敗、差分 12 名を表示）するのを確認してから `theme.rs` を直し、exit 0（8件合格）を確認した。
- 2-2（Slint の追加）:
  - `Cargo.toml`: `slint = "=1.18.1"`（dependencies）・`slint-build = "=1.18.1"`（build-dependencies）・`i-slint-backend-testing = "=1.18.1"`（dev-dependencies）。**features は既定のままで解決・コンパイルできた**（減らす試行は不要だった）。
  - `Cargo.lock`: `CARGO_NET_OFFLINE=true cargo check -p pelway-client-ui --offline` を `--locked` なしで 1 回だけ実行して更新。追加された package は **560**（`+name` 562 件のうち 2 件は `pelway-client-ui`・`pelway-dev-smoke` の項目の再配置＝同じ内容で場所が動いただけ）。既存 package の版が変わったものは **0 件**（`-version` 行は再配置 2 件の `0.0.0` のみ）。
  - `build.rs`: `slint_build::compile("ui/main.slint")`。`src/lib.rs`: `slint::include_modules!()` で `MainWindow` などを読み込み。**Slint は `pelway-client-ui` の中だけで使い、`pelway-client-core` には入れていない。**
  - `ui/main.slint` の修正は構文と型の誤りだけ（並び・見た目は変えない方針）:
    1. `out property <callback> x();`（19箇所）→ `callback x();`。`out callback` も Slint の構文にないため修飾子なし（callback の既定の向きが「部品が呼ぶ・外が受ける」）。
    2. `Text` に `underline` プロパティはないため `LinkText` の `underline: true;` を削除（**下線は付かない**。リンクは色だけで示す）。
    3. `LinkText` に `callback touched();` を宣言（4箇所の `touched => {...}` が通るように）。**touched を発火する入力はまだ置いていない**（押せるようにするのは結合の段階）。
    4. 未読の横線の `vertical-alignment: center;`（`Rectangle` に無いプロパティ）を削除。HorizontalLayout の交差軸で固定サイズの子は中央に置かれる前提（Qt 流）。**実際の見た目は未検証**。
    5. 入力欄の `composer-ta`・`reply-ta` を `TouchArea` から `FocusScope` に（`has-focus` は `TouchArea` に無く `FocusScope` にある。この2つの TouchArea はクリック処理に使っていなかったため機能の喪失なし）。
    6. `AiBadgeView` の `padding-left/right` は layout 要素にしか効かないため、中に `HorizontalLayout` を置いて同じ余白を実現（警告 2 件の解消）。
    7. `ThemeState.current-name` を `out` から `in-out` に変更。**理由**: 生成物で確認したところ `out` のグローバルプロパティは Rust 側の setter が空のスタブになり値を渡せない（第8枠の設計「Rust の `Theme::name()` から渡す」と矛盾するため）。
  - unsafe の取扱い: 生成物の `ItemTreeVTable_static` 内に `#[allow(unsafe_code)]` があり、クレートの `#![forbid(unsafe_code)]` と衝突して E0453 になるため、**`forbid` を `deny` に緩めた**（生成物は自分自身を許可している。手書きのコードは unsafe 禁止のまま）。
  - `tests/slint_window.rs`: `i-slint-backend-testing` と `SLINT_BACKEND=testing`（テスト関数の先頭で 1 回だけ設定。並列実行の競合を避けるため検査は 1 関数にまとめた）で、`MainWindow` が作れること・テーマ名 4 つ（`light`・`dark`・`light-high-contrast`・`dark-high-contrast`）を `ThemeState.current-name` に渡せること・`tr-*` と切替プロパティを渡せることを確認（**窓は開いていない**）。グローバルへのアクセスは `slint::Global` トレイトの `ThemeState::get(&window)` 経由。
  - トークンの値（色）は `.slint` に入れていない（`DesignTokens` は名前だけ・初期値なしのまま。値の入れ方は段階B-2で決める）。
- hk.pkl の変更（この枠で迷って安全側にした点）: 正本の写しは sha256 固定のため末尾改行を付けることができないが、pre-commit の newlines チェック（end-of-file-fixer）がこれを弾いた。`docs/**`（凍結文書）と同じ理由で、`crates/pelway-client-ui/tokens/**` を trailing whitespace / newlines の対象外に 1 行追加した（--no-verify は不使用）。
- 検証（コマンドと exit code。すべて `CARGO_NET_OFFLINE=true` 付き）:
  - `cargo fmt --all --check` 0
  - `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` 0
  - `cargo test --workspace --locked --offline` 0（pelway-client-core 54 件: approval 24・contract 14・input 16、失敗 0 ／ pelway-client-ui 29 件: bridge 10・i18n_keys 6・no_color_values 4・slint_window 1・theme_tokens 8、失敗 0 ／ pelway-dev-smoke 1 件、失敗 0）
  - `cargo build -p pelway-client-ui --locked --offline` 0（build.rs 経由で `ui/main.slint` を含むビルド）
  - `git status --short`（最終確認時）: 空
- 未検証・未決:
  - **窓を開いての確認はしていない**（デザイン担当・QA が別に行う）。`FocusScope` 化・下線の削除・未読横線の中央寄せ前提・`AiBadgeView` の余白の実際の見た目は未検証。
  - **U01〜U07 は未決のまま**。とくに U05（Slint のライセンス）: Royalty-free かどうか・表示義務の有無・置き場所は決めていないため About 画面等は作っていない。**Slint を依存に足したことと、ライセンス表示をどこにどう出すかは未決のまま残す。**
  - `LinkText.touched` は宣言だけで発火する入力なし（結合の段階で付ける）。
  - `ThemeState.current-name` を in-out にした結果、.slint 側からも書けるようになっているが、書き方はしていない（テーマ名の定義元は `Theme::name()` のまま）。
  - `DesignTokens` は `out property` のまま。段階B-2で Rust から値を渡す際には `ThemeState` と同じ問題（out は Rust から渡せない）が予想されるため、そのとき `in-out` 化と `theme_tokens.rs` の `out property <brush>` を読むテストの更新が必要。
  - 17:30 の記録にある `light-accessible`・`dark-accessible` は古い名前のまま使っていない（`from_name` は引き続き拒否。テストで確認）。
  - 正本 JSON の `mobile` 上書き・`color` 以外のグループ（`space`・`size`・`radius` 等）は `TOKEN_NAMES` の対象外のまま（デスクトップの色トークンだけが今の対象）。
- ネットワーク・書き込み範囲: cargo はすべて `CARGO_NET_OFFLINE=true` + `--offline`（lock 更新の 1 回だけ `--locked` を外した）。`cargo fetch`・`cargo update`・`cargo add` は不使用。worktree の外への書き込みなし（一時ファイルは worktree 内の `target/` のみ。前回のような `/tmp/opencode` への出力はしていない）。git の push・fetch なし。`crates/pelway-client-core` は変更なし。

### 2026-11-09 第10枠（段階B-2：Codex 第5回指摘の修正・トークンの値・Fluent）
- start: 2026-11-09 22:50 JST
- stop:  2026-11-09 23:50 JST
- glm: long（T3 Code スレッドで OpenCode + zai-coding-plan/glm-5.3 が実装。起点 `f41c276` の worktree `demo-t3-client-ui-r4` で実施）
- 使った skills: tdd と verification-before-completion の考え方に従った（ファイルは worktree 外のため読んでいない）。各項目でテストを先に書いて落ちることを確かめてから直し、最後に指定のコマンドを全部動かして exit code を記録した。
- 対象: 指示書「Slint の画面の骨組み 段階B-2」の §2-1（Codex 第5回レビューの should-fix 2件）・§2-2（トークンの値）・§2-3（Fluent）。§2-4（accessible・窓を開く・LinkText.touched の発火・KeySource・Clock・Effect・About・フォント）は手を付けていない。コミットは3件＋作業ログ1件（`f41c276` の後ろへ。push はしていない）。
- 最初の確認: `pwd` = worktree `demo-t3-client-ui-r4`、ブランチ `demo-t3/client-ui-r4`、HEAD `f41c276`、`git status --short` 空（引き継ぎどおり）。
- 2-1（Codex 第5回指摘の修正。コミット `689c115`）:
  - 指摘1（写しと正本の一致）: `tests/theme_tokens.rs` に `token_copy_bytes_match_canonical_by_sha256` を追加。`tokens/pelway.tokens.json` のバイト列の SHA-256 が `4f523ce19145250df391975dea3924aa7b5bdcbd417c0c31e910f3b17c543953` に一致することを確かめる。
  - **SHA-256 の計算方法: sha2 は使わず std だけの実装をテスト内に書いた**。理由: `Cargo.lock` に sha2（とその依存一式）が入っていない（`grep '^name = "sha2"'` が 0 件）。オフラインで dev-dependencies に足しても解決できる保証がなく、lock を動かすリスクを避けた。実装の正しさは `sha256_known_answers`（FIPS 180-4 の既知の答え: 空の入力と `abc`）で確認する。
  - 赤→緑（指摘1）: 写しの1バイトを変えて（`$extensions.pelway.generated` の日付を1文字）`cargo test -p pelway-client-ui --test theme_tokens --locked --offline` が **exit 101**（`token_copy_bytes_match_canonical_by_sha256` が FAILED、ほか9件は合格）。`git checkout -- crates/pelway-client-ui/tokens/pelway.tokens.json` で戻して **exit 0**（10件合格）。`sha256sum` が上の値に戻り、`git diff --stat` から tokens JSON が消えたことも確認。
  - 指摘2（build.rs の走査漏れ）: `tests/no_color_values.rs` の `source_files()` にクレート直下の `build.rs` を加えた（`.rs` の決まり。`.json` は引き続き対象外）。`bundled_slint_and_ftl_are_part_of_the_scan` にも build.rs の存在確認を追加。
  - 赤→緑（指摘2）: build.rs に色の値のパターンを一時的に1つ入れて（パターンそのものはここにも書かない）`cargo test -p pelway-client-ui --test no_color_values --locked --offline` が **exit 101**（`no_color_values_anywhere_in_the_crate` が FAILED）。戻して **exit 0**（4件合格）。build.rs の差分が消えたことを確認。
- 2-2（トークンに値を入れる。コミット `3e41cb5`）:
  - **読み方: (b) `include_str!` で JSON を実行形式に取り込み、実行時に標準ライブラリだけで読む**。理由: build.rs は slint のコンパイルだけに保てる（読み方 a のコード生成を足さない）、JSON を読む crate を新しく足さない、テストと同じ std の最小の読み取り器で済む。`LazyLock` で初回呼び出しのときに 1 回だけパースする。`Cargo.lock`・`Cargo.toml` はこの項目では変更していない。
  - 値の出どころは `tokens/pelway.tokens.json` の `color.<theme>` の直下だけ。`$value` は DTCG 形式（`colorSpace: "srgb"`・`components: [r, g, b]`・`alpha`、すべて 0〜1）で、`slint::Color::from_argb_f32` で色にする。`mobile` の上書きは使っていない。
  - **読めないトークン: `avatar-human-bg-section`（4テーマすべて）**。`$value` が `{color.<theme>.avatar-human-bg}` の形の参照（DTCG のエイリアス）のため、自分で参照を解決せず値なし（`token_value` が `None`）のまま。**62名中 61 名に値を入れた**。テスト側に `UNREADABLE_TOKENS` として明記し、正本がこの形をやめたら落ちる検査（`unreadable_tokens_match_canonical_json_forms`）も置いた。
  - `token_value` は `Option<slint::Color>` を返す形に変更（旧: `Option<&'static str>`・常に `None`）。`apply_design_tokens(&MainWindow, Theme)` が `DesignTokens` グローバルの全プロパティ（50）に値を設定する。
  - `ui/main.slint` の `DesignTokens` を `out property` から `in-out property` に変更（第9枠の予見どおり、out のグローバルは Rust から setter 経由で値を渡せないため）。初期値は付けていない（.slint に色の値を書かない決まりは不変）。`tests/theme_tokens.rs` の `slint_does_not_define_its_own_token_names` を `in-out property <brush>` の行を読む形に更新（検査の意味は弱めていない）。
  - テスト: `token_values_match_canonical_json` —— 4テーマ × 61トークン（読めない1名を除く）で、Rust が返す値の各チャネル（alpha/red/green/blue）が JSON の成分 × 255 と **1 以内の差**で一致すること。許容を付けた理由: JSON の f64 と Slint の色の u8 チャネルの間の丸め（`from_argb_f32` の丸め方の実装差を含む）で 1 LSB ずれる余地があるため。`token_values_are_not_filled_yet`（値を入れていないことの検査）は値を入れたため削除。
  - `tests/slint_window.rs`: `apply_design_tokens` で light→dark に切り替えると `DesignTokens.bg`・`ink` の値が変わることを確認（`i-slint-backend-testing`。窓は開かない）。
  - 赤→緑: まずスタブ（`token_value` は常に `None`・`apply_design_tokens` は何もしない）で `token_values_match_canonical_json` と `slint_window` が **exit 101**。実装して **exit 0**。
- 2-3（Fluent。コミット `01a26a1`）:
  - `fluent-bundle`・`unic-langid` は `Cargo.lock` に無かったが、`Cargo.toml` に `fluent-bundle = "0.15"`・`unic-langid = "0.9"` を足して `CARGO_NET_OFFLINE=true cargo check -p pelway-client-ui --offline`（`--locked` なし・1回だけ）を動かしたら **成功**（exit 0）。icu4x 一式（icu_provider 等）は slint 経由で既に lock に入っていたため、新規のダウンロードは要らなかった（手元のキャッシュで解決）。
  - `Cargo.lock`: +97 行。**追加された package は 10**（`+name = ` が 10 件）: fluent-bundle 0.15.3・fluent-langneg 0.13.1・fluent-syntax 0.11.1・intl-memoizer 0.5.3・intl_pluralrules 7.0.2・self_cell 0.10.3 と 1.3.0（2版併存）・type-map 0.5.1・unic-langid 0.9.6・unic-langid-impl 0.9.6。**既存 package の版の変更・項目の再配置は 0 件**（`git diff Cargo.lock` の `-` 行は diff ヘッダのみ）。
  - `src/i18n.rs`: `Messages::new("ja"/"en")`（`.ftl` は `include_str!` で取り込み、実行時にファイルを読まない）・`message(key, vars)`（Fluent の整形に誤りが出たら `None`）・`apply_translations(window, messages, vars)`（`USED_KEYS` 全 49 キーを `tr-*` に渡し、渡せなかったキーのリストを返す）。`@tr()` は使っていない（前の回の判断のまま）。英語は仮訳のまま。
  - 変数（`{ $name }` 等8種）は `FluentVars::sample()` の仮の値（合成データと同じ架空の値）を全キーに渡している。**既知の限界: `.ftl` の `$to` が `mention-read-range`（読んだ範囲の終わり＝時刻）と `ext-card-question`（メールの宛先＝人）の二役のため、1 組の仮値では両方の意味は同時に満たせない**。画面のデータとの結合は B-3 以降のためそのままにし、ここに記録する（`.ftl` は承認済みの文言なので書き換えない）。
  - テスト: `tests/i18n_fluent.rs`（3件）—— ja/en の bundle が作れること・他の言語（fr・空文字列）は拒むこと、日英それぞれで `USED_KEYS` 全キーが `Some` かつ空でないこと（Fluent の誤りが出たら `None` になるので、これで誤りなしを検査）、無いキーは `None`。`tests/slint_window.rs` —— ja を適用して `tr-rail-today`・`tr-unread-divider` が ja の文言、en に切り替えると `tr-rail-today` が en の文言になること（窓は開かない。文言の本文は出力していない）。
  - 赤→緑: スタブ（`new` は常に `None`）で `i18n_fluent` 3件と `slint_window` 1件が **exit 101**。実装して **exit 0**。
- 検証（コマンドと exit code。すべて `CARGO_NET_OFFLINE=true` 付き。HEAD `01a26a1` で）:
  - `cargo fmt --all --check` 0
  - `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` 0
  - `cargo test --workspace --locked --offline` 0（pelway-client-core 54 件: approval 24・contract 14・input 16 ／ pelway-client-ui 35 件: bridge 10・i18n_fluent 3・i18n_keys 6・no_color_values 4・slint_window 1・theme_tokens 11 ／ pelway-dev-smoke 1 件、失敗 0、合計 90）
  - `cargo build -p pelway-client-ui --locked --offline` 0（build.rs 経由で `ui/main.slint` を含むビルド）
  - `git status --short`（最終確認時）: 空（このログの追記前）
  - `tokens/pelway.tokens.json` の sha256 は最終確認時も `4f523ce19145250df391975dea3924aa7b5bdcbd417c0c31e910f3b17c543953`（変更なし）
- 未検証・未決:
  - **U01〜U07 は未決のまま**。U05（Slint のライセンス）に加え、**fluent 関連 crate のライセンス表示をどこにどう出すかも未決**（fluent-bundle 0.15.3 は MPL-2.0 / Apache-2.0 デュアルライセンス。About 画面は作っていない）。
  - **窓を開いての確認はしていない**（デザイン担当・QA が別に行う）。テーマを切り替えたときの実際の見た目も未検証（testing backend での値の検査のみ）。
  - `avatar-human-bg-section` は値なしのまま（正本が参照をやめたら `unreadable_tokens_match_canonical_json_forms` が落ちて気付く）。
  - `FluentVars` は仮の固定値。`$to` の二役（上記）は結合の段階で直す。
  - accessible-role / accessible-label・`LinkText.touched` の発火入力・本物の `KeySource`（UUID）と `Clock`（実時間）・`Effect` の通信への接続・About 画面・フォントの同梱は B-3 以降。
  - 17:30 の記録の `light-accessible`・`dark-accessible` は引き続き不使用（`from_name` が拒否するのをテストで確認）。
- 迷って安全側にした点:
  - sha2 を足さず std 実装にした（上記の理由。lock を動かさず、ダウンロード不可でも確実）。
  - 参照トークン（`avatar-human-bg-section`）を自分で解決せず `None` のまま（指示どおり「自分で値を作らない」）。
  - 値の照合に ±1/255 の許容を付けた（丸めの実装差で偽陰性を出さないため。緩すぎる検査にはならない程度の最小限）。
  - `$to` の二役にどちらかの意味を固定せず、そのまま1組の仮値を渡して結合段階に先送り（承認済みの `.ftl` を書き換えない）。
- ネットワーク・書き込み範囲: cargo はすべて `CARGO_NET_OFFLINE=true` + `--offline`（lock 更新の 1 回だけ `--locked` を外した）。`cargo fetch`・`cargo update`・`cargo add` は不使用。worktree の外への書き込みなし（`/tmp` を含む。一時ファイルは worktree 内の `target/` のみで、赤の確認後に削除済み）。git の push・fetch なし。amend・rebase・reset・stash・force・`--no-verify` も不使用。`crates/pelway-client-core` は変更なし。

### 2026-11-10 第11枠（段階B-2 r5：Codex 第6回指摘の修正）
- start: 2026-11-10 00:00 JST
- stop:  2026-11-10 00:14 JST
- glm: long（T3 Code スレッドで OpenCode + zai-coding-plan/glm-5.3 が実装。起点 `dc5e57c` の worktree `demo-t3-client-ui-r5` で実施）
- 使った skills: tdd と verification-before-completion の考え方に従った（ファイルは worktree 外のため読んでいない）。2件ともテストを先に直し、実装を一時的に壊して落ちること（赤）と戻して通ること（緑）の exit code を記録し、最後に指定のコマンドを全部動かして exit code と件数を記録した。
- 対象: 指示書「Slint の画面の骨組み 段階B-2 r5」の §2-1・§2-2（Codex 第6回レビューの should-fix 2件のみ）。§2-3 の除外項目（crate 追加・`.slint`・`.ftl`・トークン JSON・AccessKit・窓を開く・About）には手を付けていない。コミットは2件＋作業ログ1件（`dc5e57c` の後ろへ。push はしていない）。
- 最初の確認: `pwd` = worktree `demo-t3-client-ui-r5`、ブランチ `demo-t3/client-ui-r5`、HEAD `dc5e57c`、`git status --short` 空（引き継ぎどおり）。
- 2-1（色の照合の許容幅をなくす。コミット `90ee72d`）:
  - 指摘の実証: 現行テスト（±1/255 の許容）のまま `light/ink` の red を 1 LSB 下げる一時変更を入れても theme_tokens 11件が全部合格（**exit 0**）。`0.0784×255 = 19.992` のように ×255 の小数部が 0.5 超の成分は、正しい丸め（20）から 1 LSB 下がった値（19）との差が 0.992 ≤ 1.0 に収まるため。一方 `light/bg` の red（`0.902×255 = 230.01`）は 1 LSB 下げると差 1.01 > 1.0 で落ちる——許容幅は「×255 がほぼ整数の成分の、丸めと同じ向きのずれ」を見逃していた。
  - **Slint の丸めの観察**: `slint::Color::from_argb_f32` → `red()` 等の u8 変換は floor でも round でもなく **`ceil(x×255 − 0.5)`**（×255 が .5 ちょうどのときは切り下げ、それ以外は最近接）だった。観察値: `0.5×255=127.5→127`・`0.1f32×255` が f32 掛け算の丸めでちょうど `25.5` になる→`25`・`0.25×255=63.75→64`・`0.10000001f32×255=25.500002→26`・`0.09999999f32×255=25.499996→25`。一時テストファイル `tests/zz_probe_tmp.rs` で観察し、観察後に削除した。
  - **期待値の計算は f64 ではなく f32 の経路**（実装と同じ）: JSON の f64 を `as f32` して `255.0f32` を掛けてから丸める。f64 のまま `0.1×255 = 25.500000000000004` を `ceil(x−0.5)` すると 26 になり、実装（25）と一致しない（境界値で偽の失敗が出る）。
  - **テスト側で同じ決まりを独立に書いた**（共通関数化はしなかった）。理由: 実装の丸めは `slint::Color::from_argb_f32` の中で行われるため、共通化には実装を自前の丸め（`from_argb_u8`）への書き換えが要り、slint の丸めと自前の丸めが全チャネルで同じ値になる保証を新たに作ることになる（画面の見た目を変えるリスク）。テスト側の `expected_channel(json_value)`（f32 経路の `ceil(x×255−0.5)`）が正本の成分から期待値を作り、`assert_channel` を `assert_eq!` の完全一致に変えた。許容幅は廃止。
  - 赤→緑: 修正後テスト + `light/ink` の red を 1 LSB 下げる一時変更（`src/theme.rs` の `token_value`）→ **exit 101**（`token_values_match_canonical_json` が FAILED: `light/ink の red`）。一時変更を戻して **exit 0**（11件合格）。`git diff --stat` に `src/theme.rs` が残っていないことを確認。
- 2-2（`apply_translations` が全キーを渡すことの振る舞いでの確認。コミット `b0fa8ad`）:
  - `tests/slint_window.rs` に `USED_KEYS` の全キーと `MainWindow` の対応する `tr-*` getter の対応表 `TR_GETTERS` を1か所にまとめた（`USED_KEYS.len()` = **49 件**で、指示の見立てと同じ）。型は `type TrGetter = fn(&MainWindow) -> SharedString;` に切り出した（初回コミットで pre-commit の clippy が `clippy::type_complexity` を出したため。コミットは作り直さず型の切り出しを直したあとにコミット）。
  - 表の件数が `USED_KEYS.len()` と同じこと・キーの集合（昇順比較）が `USED_KEYS` と一致することをテストで確認（表から1つキーが漏れたら落ちる）。
  - 日英それぞれで `apply_translations` のあと、全キーについて `getter(&window)` の値が `messages.message(key, &vars)` の結果と一致することを確認（窓は開かない。文言の本文は出力していない）。ja→en の順に適用するため、en の適用で値が実際に切り替わることもこの照合が見る（ja の文言が残っていれば en 側で落ちる）。旧の部分照合（`tr-rail-today`・`tr-unread-divider` の2つだけ）は全キー照合に置き換えた。
  - 赤→緑: `src/i18n.rs` の `apply_translations` から `set_tr_composer_placeholder` の1行だけを一時的に外す → **exit 101**（`slint_window` が FAILED: ja 適用後の `composer-placeholder` の照合）。このとき返り値の `missing` は空のまま（指摘どおり setter が1つ消えても `missing` では検出できない）。戻して **exit 0**（1件合格）。`git diff --stat` に `src/i18n.rs` が残っていないことを確認。
- 検証（コマンドと exit code。すべて `CARGO_NET_OFFLINE=true` 付き。HEAD `b0fa8ad` で）:
  - `cargo fmt --all --check` 0
  - `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` 0
  - `cargo test --workspace --locked --offline` 0（90 件合格・失敗 0: pelway-client-core 54 = approval 24・contract 14・input 16 ／ pelway-client-ui 35 = bridge 10・i18n_fluent 3・i18n_keys 6・no_color_values 4・slint_window 1・theme_tokens 11 ／ pelway-dev-smoke 1）
  - `cargo build -p pelway-client-ui --locked --offline` 0（build.rs 経由で `ui/main.slint` を含むビルド）
  - `git status --short`（最終確認時）: 空（このログの追記前）
  - `git diff --name-only dc5e57c` は `crates/pelway-client-ui/tests/slint_window.rs`・`crates/pelway-client-ui/tests/theme_tokens.rs` の2件のみ（`Cargo.toml`・`Cargo.lock`・`ui/main.slint`・`tokens/pelway.tokens.json`・`i18n/*.ftl`・`crates/pelway-client-core` は 0 変更）
- 未検証・未決:
  - U01〜U07 は未決のまま。`$to` の二役（`mention-read-range` と `ext-card-question`）は前の回の判断のまま結合段階に先送り（`.ftl` は変えていない）。
  - **Slint の丸めの観察（`ceil(x×255−0.5)`）は slint `=1.18.1` での実測に基づく**。版を変えるときは `expected_channel` の決まりを見直す必要がある。
  - 窓を開いての確認はしていない（testing backend のまま）。
- 迷って安全側にした点:
  - 期待値の丸めを共通関数化せずテスト側に独立に書いた（実装を自前の丸めに書き換えると画面の見た目を変えるリスクがあるため。上記の理由）。
  - 期待値の計算経路を f64 でなく実装と同じ f32 に合わせた（f64 では `0.1` のような値で偽の失敗が出ることを観察で確認したため）。
  - 赤の確認に使った一時変更（`src/theme.rs`・`src/i18n.rs`）は、戻すたびに `git diff --stat` に現れないことを確認した。
- ネットワーク・書き込み範囲: cargo はすべて `CARGO_NET_OFFLINE=true` + `--offline --locked`。`cargo fetch`・`cargo update`・`cargo add` は不使用。worktree の外への書き込みなし（`/tmp` を含む。一時ファイルは `tests/zz_probe_tmp.rs` のみで観察後に削除。`target/` への出力も残していない）。git の push・fetch なし。amend・rebase・reset・stash・force・`--no-verify` も不使用。`crates/pelway-client-core` は変更なし。
