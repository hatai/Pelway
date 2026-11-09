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
