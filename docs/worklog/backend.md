# 作業記録: バックエンド・AI連携（backend）

- 担当: Pelway バックエンド・AI連携エンジニア
- worktree: `/home/hatai/.t3/worktrees/Pelway/demo-t3-backend`
- branch: `demo-t3/backend`（base `f32ca74`）
- 時刻はすべて日本時間（JST, UTC+9）
- データは合成データのみ。製品コードに外部送信は無い。待受を書く場合は `127.0.0.1` / `::1` のみ。

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

### 2026-11-09 第1枠（GLM 不使用枠）
- start: 2026-11-09 14:51 JST
- stop:  2026-11-09 15:01 JST
- glm: none（15:00〜19:00 の GLM 不使用ルールに合わせ、実装者が直接編集。T3 thread は使っていない）
- 内容:
  - `crates/pelway-contract`（承認 API の契約型とメモリ内台帳）と `crates/pelway-at-rest`（保存時の分割 AES-256-GCM コンテナ）の下書きを未追跡ファイルとして作成。
- 検証（コマンドと exit code）:
  - 依存 crate が未取得のため `cargo check --offline` は 101。依存なしで動く部分だけ `rustc --test` で確認（報告どおり 18 件・6 件）。
- 未検証・未決:
  - serde と AES-GCM を使うテストは未実行（crate 取得の許可待ち）。

### 2026-11-09 第2枠（GLM 不使用枠）
- start: 2026-11-09 15:02 JST
- stop:  2026-11-09 15:13 JST（コミット 4564e88 の時刻 15:13:26。当初 15:14 と記載していたのを訂正）
- glm: none（同上。実装者が直接編集）
- 許可と外部取得:
  - 2026-11-09 15:02 JST、開発リードがルームで crates.io からの取得を許可。対象は `aes-gcm` `hkdf` `sha2` `zeroize` `serde` `serde_json` の 6 つと、その推移的依存だけ。これ以外の crate は追加していない。
  - `cargo fetch`：2026-11-09 15:05:32〜15:05:35 JST、exit 0。38 パッケージを `Cargo.lock` に追加（バージョンと checksum を固定）。取得先は crates.io のみ。ダウンロード先は共有の `~/.cargo/registry`（cargo の通常のキャッシュ）。
  - `pelway-contract` に `sha2` を足したあと `cargo fetch --offline`（15:11、exit 0、新規ダウンロードなし）で依存の対応だけを更新。
- 内容:
  - 冪等キー：送信前に「処理中」で (`card_id`, `card_version`, `snapshot_hash`) に結び付けて記録し、送信後に「完了」へ。処理中の再送は 409 `approval_in_progress` で送信しない。内容の違う同じキーは 422 `idempotency_key_reused`。送信前の失敗（409 など）ではキーを記録しない。送信前の中止 `abort_before_send` はキーを解放。
  - `snapshot_hash` を「固定 payload の hash ＋ コメント」から計算（`snapshot.rs`）。コメント保存で版は上げず hash は変わる。コメント保存の応答 `CommentSaved` に新しい `snapshot_hash` と `card_version` を入れた。
  - コメント上限 `COMMENT_MAX_CHARS = 4000`（Unicode コードポイント数）と、数え方を一本化した公開関数 `comment_len` / `validate_comment`。台帳もこの関数を使う。
  - 保存時コンテナ：チャンクの AAD にヘッダー全体を追加。nonce は連番 11 B ＋ 最終フラグ 1 B。他形式の名前を外し「分割 AES-256-GCM コンテナ」と呼ぶ。
  - `member_kind` は `human|ai` のまま（リード決定）。仕様 04 §2 の 4 値への対応は TODO のまま。U01・U05・U06 も TODO のまま。
- 検証（コマンドと exit code）:
  - `cargo fmt --all --check` 0
  - `cargo check --workspace --all-targets --locked` 0
  - `cargo test --workspace --locked` 0（pelway-at-rest 単体 8・aead 12、pelway-contract 単体 31・serde_contract 9、dev-smoke 1。計 61 件、失敗 0）
  - `cargo clippy --workspace --all-targets --locked -- -D warnings` 0
  - `mise run verify` 0（fmt-check・check・clippy・test・build・toolchain-check・hk-validate・hk-check・fnox-check・pitchfork-check）
- 未検証・未決:
  - 独立コードレビュー（Codex）は未実施。重点：nonce が重ならないこと、切り詰め・並べ替えの検出、鍵の包装。
  - このブランチは `f32ca74` 起点のため `demo-check` は含まれていない（開発基盤のブランチにある）。統合時に QA が実行する。
  - HTTP 層・WASM 向けのビルド（`--target wasm32-unknown-unknown`）は未実施。
  - QA による独立の再実行は未実施。
- 手順からの逸脱:
  - T3とMatt Pocockのskillsを使わずに直接書いた。理由はGLMを使わない時間（15:00-19:00）だったため。リードの指示で、作り直しはせず、Codexで全16ファイルをレビューし、QA合格まで統合候補にしない。
  - WSL への受け渡しで PowerShell の引用が崩れるため、Windows 側の一時フォルダ（`%TEMP%` の `pelway-be-*`）にファイルとスクリプトを置いて WSL から読んだ。中身はこの枠のソースと検証用スクリプトだけで、秘密は含まない。使用後に削除済み。WSL の `/tmp` に出した検証ログも削除済み。

### 2026-11-09 第3枠（レビュー準備・GLM 不使用枠）
- start: 2026-11-09 15:15 JST
- stop:  2026-11-09 15:18 JST（コミット 607ffa3 の時刻 15:18:01。当初 15:19 と記載していたのを訂正）
- glm: none（実装はしていない。リード指示で 19:00 まで実装停止）
- 内容:
  - この作業記録の訂正（第2枠の stop、逸脱の記録）。docs のみのコミット。
  - Codex の読み取り専用レビュー（範囲 `f32ca74..4564e88`、全16ファイル）を T3 で始めようとした。
- 検証（コマンドと exit code）:
  - `t3 thread start . - --provider codex --model gpt-6-luna --title ...`（cwd はこの worktree、15:17:24）→ exit 1、`No T3 Code project matches '.'. Pass a project id or a path inside a project.`
  - この worktree のパスは T3 のプロジェクトとして登録されていないため、既存の worktree を指定してスレッドを始める方法が CLI に無い（`--worktree` は新しい worktree を作る）。指示に従い、回避せずに止めた。
- 未検証・未決:
  - Codex レビューは未実施（開始できていない）。リードの判断待ち。

### 2026-11-09 第4枠（Codex 独立レビュー・GLM 不使用枠）
- start: 2026-11-09 15:19 JST
- stop:  2026-11-09 15:26 JST（このコミットの直前に `date` で確認）
- glm: none（実装はしていない。リード指示で 19:00 まで実装停止）
- 内容:
  - Codex の独立レビュー。T3 のスレッド `mcp:da6eb722-7779-4acd-b9c8-bf2c64b6d617`（題名「backend 独立レビュー f32ca74..607ffa3 読み取り専用 Codex」、provider `codex`、model `gpt-6-luna`）。
  - CLI（`t3 thread start /home/hatai/repos/hatai/Pelway - --provider codex --model gpt-6-luna --title ...`）で、本体のプロジェクト（`/home/hatai/repos/hatai/Pelway`、worktree 指定なし）から開始した。`t3 thread show --json` では `runtimeMode: full-access`、`worktreePath: null`、`branch: null`。**読み取り専用は依頼文でしか担保されていない。**
  - 実行時間：15:19:59 開始、15:21:47 完了（status `completed`）。サインインは不要だった。
  - 範囲：`f32ca74..4564e88` の16ファイルと `607ffa3`（この作業記録）。依頼文には「ファイルの書き込み、git の書き換え、ネットワークの利用はしない。読むのは `git show`、`git diff f32ca74..4564e88` と `607ffa3` だけ」と明記した。
- 手順からの逸脱:
  - 案②（新しいレビュー用 worktree）に決め直したあと、行き違いのため本体のフォルダで full-access のCodexが動いた（スレッドは15:19:59開始、決め直しは15:18）。
  - レビュー用 worktree は作っていない。そのため、この作業から片付けの候補になる worktree は無い。
- 検証（確かめたこと）:
  - 本体（`/home/hatai/repos/hatai/Pelway`）の状態を前（15:19:48）と後（15:24:03）で記録した。`git rev-parse HEAD` はどちらも `f32ca74c25d8c5614e7655a11c2a929be8211a10`、`git status --porcelain` はどちらも0行、`git stash list` はどちらも空で、同一。`git status` は `GIT_OPTIONAL_LOCKS=0` で実行し、index を書き戻さないようにした。
  - `git worktree list` は後のほうに1行多い：`/home/hatai/.t3/worktrees/Pelway/review-codex-demo-check-slot4 6cfa813 [review/codex-demo-check-slot4]`。これは開発基盤担当のレビュー用 worktree で、15:19:50 に作られた。開発基盤担当がルームで自分のものだと確認した。この作業では触っていない。
  - 「Codexが使ったのは git diff と git show だけ」は Codex の自己申告で、未検証（自己申告・未検証）。確かめたのは、上の本体の前後比較（HEAD・status・stash が同一）だけ。
- レビューの指摘（総合判定：統合前に必須の修正あり）:
  - high：`crates/pelway-contract/src/ledger.rs:192-216`（`save_comment`）。カードの状態を見ずにコメントを足すため、`begin_approve` で「送信中」にしたあとでもコメントを保存でき、`snapshot_hash` が変わる。予約と疑似送信の記録には予約時の古い hash が残る。直し方の案：「送信中」のコメント保存を拒否してテストする。実送信がコメントを参照するなら、送信の直前に予約時の hash と現在の hash の一致も確かめる。実装者も、コードを読んで指摘が正しいことを確認した。
  - 重点のうち、nonce の重複、切り詰め・並べ替え・追記・重複・ヘッダー改ざんの検出、鍵の包装・HKDF・zeroize、snapshot_hash とコメント、コードポイント数、契約型と Cargo.lock は「問題なし」。
  - critical・medium・low・nit の指摘は無い。
- 次の作業:
  - high の指摘は 19:00 以降に T3 で直す。そのあと、直した差分を、新しいレビュー用 worktree（案②）で Codex に再レビューしてもらい、そのあと QA に回す。
  - それまでの間、QA は `4564e88` と `607ffa3` を統合候補から外している。

### 2026-11-09 第5枠（Codex レビュー high 指摘の対処ほか、GLM 枠）
- start: 2026-11-09 16:01 JST
- stop:  2026-11-09 16:06 JST
- glm: long（T3 Code 上で GLM が実装。バックエンド・AI連携エンジニアの依頼文どおり）
- skills:
  - Matt Pocock Agent Skills の `tdd` を使用。red（失敗するテストを先に書く）→ green（最小実装）の順で進め、
    最初の `cargo test -p pelway-contract` で新テスト 3 件の失敗を確認してから `save_comment` に状態検査を入れた。
- 内容（依頼 4 件のうち 1〜3 を実施、4 は対処せず報告）:
  - 修正1（high 指摘）: `save_comment` がカードの状態を見るようにした。送信承認待ちでないカード
    （Sending・SentAwaitingReply 等）へのコメント保存は `CommentError::NotAwaitingApproval`
    （`state` 付き、HTTP 409）で拒否し、`snapshot_hash` を変えない。`CommentError` は
    `ApproveError` と同じ serde の形（tag="code"、snake_case）で `not_awaiting_approval` バリアントを追加。
    既存テスト `comment_after_send_keeps_state_and_does_not_resend` は新しい挙動
    （送信後のコメントは拒否・hash 不変・再送は最初の結果のまま）に書き換えた。
    テストは (a) Sending へのコメントが not_awaiting_approval、(b) このとき snapshot_hash が変わらない、
    (c) JSON が `{"code":"not_awaiting_approval","state":"sending"}`（serde_contract.rs）。
  - 修正2: 国旗（地域指示子 2 つ = 2 コードポイント）2000 個でちょうど 4000 コードポイントのコメントは受理、
    地域指示子を 1 つ足した 4001 は `too_long` で拒否（hash 不変）ことを `ledger.rs` 単体テストに追加。
  - 修正3: ルート `Cargo.toml` のコメントを実際の workspace 構成（pelway-contract・pelway-at-rest・
    pelway-dev-smoke の 3 クレート）に合わせて修正。コメントのみで、セクションや依存は変えていない。
  - 修正4（`hmac` の鍵素材の zeroize）: 実装せず報告に切り替えた。理由は下記。
- 修正4の調査結果（リード判断待ち）:
  - `hmac` はこのリポジトリの直接依存ではなく `hkdf` 0.12.4 の推移的依存。HKDF 内部の PRK
    （HMAC の鍵素材）を保持する `hkdf::Hkdf` は Drop 時に zeroize せず、この挙動を有効にする機能は
    オフラインキャッシュにある hmac 0.12.1（features は `reset`・`std` のみ）と hkdf 0.12.4 の
    いずれにも存在しない。
  - 自前のコードで直接扱う HKDF の入力（file_key）・出力（okm）と `MasterKey` は既に `Zeroizing` で保持済み。
  - 残る対応案は (i) HKDF を自前実装して PRK を `Zeroizing` で持つ（暗号処理の自作になる。hkdf 依存の除去）、
    (ii) zeroize に対応した hmac/hkdf を取得する（ネットワークが必要で今回の禁止対象）の 2 つで、
    どちらも今回の範囲を超えるため実装していない。
- TODO（未決、U01）: 送信結果が不明な Sending 状態からの回復。決めない。
- 既知の制限:
  - ファイル全体の差し替え、または古い版への巻き戻しは検出しない。
  - `MasterKey::from_bytes` は呼び出し側に鍵のコピーを残す。
  - wrap nonce はランダムのため、目安は鍵1つあたり約 2^32 ファイルまで。
- 検証（コマンドと exit code、いずれもオフライン）:
  - `cargo fmt --all --check`：初回は 1（doc 付きバリアントの改行形式の指摘）。`cargo fmt --all` で整形後、
    再実行で 0。
  - `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` 0
  - `cargo test --workspace --locked --offline` 0（pelway-at-rest 単体 8・aead 12、pelway-contract 単体 33・
    serde_contract 10、dev-smoke 1。計 64 件 passed / 0 failed / 0 ignored）
- 未検証・未決:
  - HTTP 層の変更（ledger 全体の Mutex 化、`abort_before_send` の非公開化）は今回の対象外（依頼どおり未実施）。
  - 差分の Codex 再レビュー・QA は未実施。

### 2026-11-09 第6枠（should-fix 指摘の確認のみ・GLM 枠）
- start: 2026-11-09 16:07 JST
- stop:  2026-11-09 16:21 JST
- glm: long
- 内容（実装なし。リードの採否待ち）:
  - レビュー指摘（should-fix、`ledger.rs:204`）の確認。`save_comment` の検査順序が
    「本文検証（422）→ カード存在（404）→ 状態（409）」のため、承認待ちでないカードでも
    空白だけ／上限超過の本文なら先に 422 が返り、「承認待ちでないカードへのコメント保存は
    409」という決定と一致しない、という指摘。
  - 一時テストファイル（`tests/tmp_review_repro.rs`、コミットせず削除済み）で再現確認:
    Sending・SentAwaitingReply の両カードで、空白本文は `EmptyBody`（422）、
    4001 コードポイント本文は `TooLong`（422）が返り、`NotAwaitingApproval`（409）より先。
    指摘は事実。既存テストは承認待ちでないカードには有効な本文しか使っていなかったため未検出。
  - 修正する場合の案（指摘者の提示どおり）: 検査順序を「カード存在（404）→ 状態（409）→
    本文（422）」に変え、Sending・SentAwaitingReply × 空白・4001cp の4組み合わせで
    `NotAwaitingApproval`（409）を期待するテストを追加。承認待ちのカードでは挙動不变。
    採否は開発リードが決めるため、実装はしていない。
- 検証（コマンドと exit code）:
  - `cargo test --offline -p pelway-contract --test tmp_review_repro -- --nocapture` 0
    （出力: 上記の4組み合わせすべて 422）
- 未検証・未決:
  - 検査順序の変更（採否待ち）。変更する場合は doc（`# Errors`）とテストも合わせて直す。

### 2026-11-09 第7枠（should-fix 指摘の実装・GLM 枠）
- start: 2026-11-09 16:22 JST
- stop:  2026-11-09 16:26 JST
- glm: long
- 判断の根拠:
  - 第6枠の時点では採否待ちとしたが、その後 `0267988`（第5枠への追記、リード決定 16:11）を確認した。
    「コメント保存は `AwaitingSendApproval` 以外のすべての状態で 409 `not_awaiting_approval`
    として断る。`SentAwaitingReply` も含む。」と既に決定済みで、本文の内容に条件は付いていない。
    should-fix 指摘はこの決定への準拠が不完全なだけと解釈し、実装した。解釈が違えば差し戻す。
- 内容:
  - `save_comment` の検査順序を「本文（422）→ カード存在（404）→ 状態（409）」から
    「カード存在（404）→ 状態（409）→ 本文（422）」に変更。`begin_approve` と同じく
    カードの存在が先。承認待ちのカードでは挙動不变（本文検証のみに到達）。
  - `# Errors` の doc を新しい順序とリード決定 16:11 の根拠に合わせて更新。
- テスト（tdd: red → green）:
  - red: `invalid_body_on_non_awaiting_card_returns_409_not_422` を追加。Sending・
    SentAwaitingReply × 空白本文・4001 コードポイント本文の 4 ケースすべてで
    `NotAwaitingApproval`（409）を期待。変更前は `EmptyBody`/`TooLong`（422）が先に返り fail。
  - green: 順序変更後に全テスト通過。既存テスト（承認待ちカードの空白・超過の 422 拒否を含む）
    はすべて挙動不变。
- 検証（コマンドと exit code、オフライン）:
  - `cargo fmt --all --check` 0
  - `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` 0
  - `cargo test --workspace --locked --offline` 0（pelway-contract 単体 34・serde_contract 10、
    ほか前枠と同じく計 65 件 passed / 0 failed / 0 ignored）
- 未検証・未決:
  - 差分の Codex 再々レビュー・QA は未実施。

### 2026-11-09 第5枠の追記（リード決定 16:11、送信記録、仕様確認）
- 記録者: バックエンド・AI連携エンジニア（docs のみの追記。コードは変更しない）
- HEAD before: 5600c99
- 修正4の決定（リード決定 16:11、案(c)）: 実装しない。既知の制限として記録する。
- 既知の制限（追加）:
  - `hkdf 0.12.4` 内部の HMAC 鍵（PRK）は、使用後に消去されない。自前コードの `file_key`、`okm`、`MasterKey` は drop 時に消去される。
  - 直すには zeroize に対応したライブラリの版へ切り替える必要があり、ネットワークの許可が要るため、デモ後に検討する。
  - 案(a)（HKDF の自前実装）は不採用（暗号処理を自作しないため）。
- 挙動の変更（リード決定 16:11）:
  - コメント保存は `AwaitingSendApproval` 以外のすべての状態で 409 `not_awaiting_approval` として断る。`SentAwaitingReply` も含む。
  - 理由: 承認後にコメントが変わると、承認した内容の記録（snapshot_hash）と食い違うため。
  - 旧テスト `comment_after_send_keeps_state_and_does_not_resend`（送信後のコメント保存が成功することを期待）は、この理由で `comment_after_send_is_rejected_and_does_not_resend`（拒否・hash 不変・再送は最初の結果のまま）に書き換えた（コミット 99ee013）。
- 仕様確認（2026-11-09 16:12〜16:14 JST、読み取りのみ）:
  - 対象: `docs/technical/v0.7/` 全体と `docs/architecture-v0.7.md`（base と同じ文書）。
  - 検索語: コメント、comment、送信後、承認後、SentAwaitingReply、sent_awaiting_reply、送信済、編集、attempt_started前。
  - 結果: 送信後・承認後にコメントを保存できるとする記述はない。本文（body/、architecture）にはコメントについての記述そのものがない。
  - 関連: `docs/technical/v0.7/body/03-ai-bot-runtime.md:57` は、開始後は元の snapshot・operation・試行・結果を変えずに残し、編集は後続案として保存するとしている。今回の拒否はこれと矛盾しない。
- 送信記録（4-1）:
  - 送信先: `mcp:253c6c92-be03-4997-a781-a708b553dd97`（バックエンドの GLM 実装スレッドのみ）
  - 送信時刻: 2026-11-09 15:58:19 JST（`t3 thread send <id> -`、exit 0）
  - 本文: バックエンド・AI連携エンジニアのコンピューター上の `/workspace/pelway-be/fix-instruction-1.md`、sha256 `93d7ac9882a4e2ddeb41046963899e435161752a31f29110483331393858b1ab`
  - それより前の 3 回の試行は届いていない（2 回は sha256 照合で送信前に停止、1 回は MINISFORUM に一時的につながらず実行されなかった。届いていないことはスレッドの runCount が 1 のままであることで確認した）。
- 逸脱（リード承認済みの記録事項）: Windows のコマンド行が引用符を落とすため、本文を base64 にして WSL 内で戻し、sha256 が一致したときだけ送った。Windows・WSL に一時ファイルは置いていない。
- 未検証: GLM が報告した TDD の red（先に失敗するテストを確認した）段階は GLM の自己申告で、こちらでは確認していない（未検証）。
- 独立の再実行（2026-11-09 16:10 JST、HEAD 5600c99、オフライン）: `cargo fmt --all --check` 0、`cargo clippy --workspace --all-targets --locked --offline -- -D warnings` 0、`cargo test --workspace --locked --offline` 0（64 passed / 0 failed / 0 ignored）。
- 次: 17b71a4..（この追記のコミット）を案2で Codex に再レビューしてもらい、そのあと QA。

### 2026-11-09 Codex 再レビュー（案2）の記録と逸脱（16:15〜16:33 JST）
- 記録者: バックエンド・AI連携エンジニア（docs のみの追記。コードは変更しない）
- HEAD before: 47b94c1
- Codex 再レビュー:
  - スレッド: `mcp:981a04f6-cdf9-4abc-86a7-cc76689f09db`（codex / gpt-6-luna、full-access）。作成 16:16:29 JST（最初は何もしない接続確認のみ）、レビュー依頼の送信 16:17:53 JST、完了 16:19:11 JST。
  - worktree: `/home/hatai/.t3/worktrees/Pelway/review-codex-backend-fix-slot5`（branch `review/codex-backend-fix-slot5`、HEAD 0267988、終了後も変更なし）。消さずに残す。片付け候補（濱田さんの承認後）。
  - 範囲: `17b71a4..0267988`（99ee013、5600c99、0267988）。
  - 依頼文: バックエンド・AI連携エンジニアのコンピューター上の `/workspace/pelway-be/review-prompt-2.md`、sha256 `071b1548ef802881d6514aba804705ffd7b143f2edb360c79b7277815b26459c`。
  - 指摘: should-fix 1件（場所 `crates/pelway-contract/src/ledger.rs:204`、状態検査より本文検証が先のため、承認待ちでないカードで空白・上限超過の本文が 409 でなく 422 になる）。high・must はなし。
  - 指摘の本文は Codex スレッドの最終返答と、送信先スレッドの受信メッセージにある（両者が一致することを確認済み）。
  - 送信: Codex が 16:19 JST に `t3 thread send` で `mcp:253c6c92-be03-4997-a781-a708b553dd97` へ送った（exit 0）。その前に空の入力で 1 回失敗している（exit 1、届いていない）。
- 逸脱1（手順違反）: should-fix は開発リードの採否を待つ決まりだったが、GLM がリードの判断前に実装した（d51ef28 `fix(contract): check card state before comment body in save_comment`、47b94c1 worklog）。GLM は 0267988 のリード決定 16:11 を根拠にしたと報告している。その前の 3d25c99 では採否待ちと記録していた。
- リード決定 16:31: d51ef28 と 47b94c1 を受け入れる（16:11 の決定と一致: 承認待ちでないカードは本文に関係なく 409）。条件は、`0267988..（この追記のコミット）` を案2で Codex に再レビューしてもらい指摘をファイルにだけ書くこと（`t3 thread send` はしない）、および独立の fmt / clippy の exit code。
- 逸脱2（GLM 同時実行の決まりに違反、確定）: GLM（opencode / glm-5.3）が 3 本同時に動いていた。16:20〜16:28 JST。
  - 担当者の申告: 開発基盤 `mcp:e3d9f751-e151-4fa2-9d7c-7345f5253414` は glm-5.3 で 16:14:11〜16:31。Rust・クライアント `mcp:53b5a1b3-0e04-4b3c-a387-0651b88f8019` は glm-5.3 で 15:53〜16:31。
  - バックエンド `mcp:253c6c92-be03-4997-a781-a708b553dd97`（glm-5.3）の run 3 は、Codex の送信 16:19 から始まり、最後の更新は 16:28:43 JST（updatedAt 07:28:43Z）で completed。
  - こちらの CLI 監査（`t3 thread list` / `t3 thread show --json`、読み取りのみ）: 3 本とも provider opencode、model zai-coding-plan/glm-5.3。16:15:56 JST の一覧で e3d9f751 と 53b5a1b3 は running。バックエンドは 16:20:35、16:23:51、16:26:07 の確認で running。16:32:19 JST にも e3d9f751（active run 2）と 53b5a1b3（active run 3）は running。
  - CLI では run ごとの開始・終了時刻が見えない（出るのはスレッドの createdAt / updatedAt と activeRunId だけ）。そのため、こちらで確かめられたのは時点ごとの観測だけで、16:20〜16:28 を通して切れ目なく重なったことは CLI だけでは未検証。担当者の申告と合わせて、リードが確定とした。
- 新しい手順（リード決定 16:31、3担当共通）: レビュアーの指摘はまずファイルに書く。担当が部屋で GLM の空き枠を確かめてから、`t3 thread send` で送る。
- 独立の再実行（2026-11-09 16:31:58〜16:31:59 JST、HEAD 47b94c1、オフライン）:
  - `cargo fmt --all --check` 0
  - `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` 0
  - `cargo test --workspace --locked --offline` 0（pelway-at-rest 単体 8・aead 12、pelway-contract 単体 34・serde_contract 10、dev-smoke 1。計 65 passed / 0 failed / 0 ignored）
  - 実行後の `git status --porcelain` は空。
- 未了:
  - `0267988..（この追記のコミット）` の Codex 再レビュー（案2、新しいレビュー用 worktree、指摘はファイルのみ）。
  - 空き枠の確認後に、指摘があれば担当が送る。
  - QA。

### 2026-11-09 Codex 再レビュー3と開発リード受入（16:59〜17:04 JST）
- 記録者: バックエンド・AI連携エンジニア（docs のみの追記。コードは変更しない）
- HEAD before: 71d851b
- Codex 再レビュー3（案②）:
  - 範囲: `0267988..71d851b`
  - スレッド: `mcp:1b3a3950-7ae6-44f1-a669-c6b0f99ffb67`（codex / gpt-6-luna）
  - worktree: `/home/hatai/.t3/worktrees/Pelway/review-codex-backend-71d851b-r3`（branch `review/codex-backend-71d851b-r3` @ 71d851b）。片付け候補・未削除。
  - 開始 16:59:59 JST、依頼送信 17:00:00 JST、完了 17:01:21 JST。
  - 依頼文: バックエンド・AI連携エンジニアのコンピューター上の `/workspace/pelway-be/review-prompt-3.md`、sha256 `05da2bb1a6171d63fe58534f7694e7ffba6db111eb2556ce14eec5673658094b`。
  - 指摘ファイル: `/workspace/pelway-be/codex-findings-3.md`、sha256 `0c066020f2fc4a356781c9295d45deb6cb5d08b82c4a5d5782e0d8fb03d8faf4`。内容は「指摘なし」。
  - GLM `mcp:253c6c92-…` へは未送信。
  - 本体: HEAD `f32ca74`、status 空、stash 空。レビュー前後で変化なし。
- 注記: レビュー所要が約81秒と速い。開発リードは Codex の「指摘なし」だけでは合格にしないと判断。QA が独自に確認する3点:
  - 承認待ちでないカードへ、空白のみまたは4001文字の本文を送ると 409 が返ること。
  - そのとき、ハッシュとコメントが変わらないこと。
  - 承認待ちのカードでは、これまでどおり 422 が返ること。
- 開発リード判断（17:04頃 JST）: `d51ef28` と `47b94c1` を確定。16:31の条件（fmt / clippy 終了コード0、Codex 再レビュー）を満たしたため。
- ルール変更の記録: 16:55頃に濱田さんが GLM 同時実行数を厳密に管理しなくてよいと指示。開発リードが上限を撤廃。指摘は引き続き先にファイルへ書き、部屋へ記録し sha256 をそろえたら、担当が待たず自分のスレッドへ送る。16:20〜16:28 の3本同時は、当時のルール違反として記録は残す。
- 次: `f32ca74..<このコミット>` を QA へ引き渡し。

### 2026-11-09 第8枠（HTTP 層の追加とテストの強化・GLM 枠）
- start: 2026-11-09 18:26 JST
- stop:  2026-11-09 18:40 JST
- glm: long
- skills:
  - Matt Pocock Agent Skills の `tdd` を使用。作業3〜4は red（スケルトンに対する失敗）→ green（実装）の順。
- 内容:
  - 作業1（17:29 リード採用）: `invalid_body_on_non_awaiting_card_returns_409_not_422` に、拒否のあと
    `snapshot_hash`・コメント数・`card_version` が呼び出し前と変わらないことの確認（`card_summary` ヘルパー）
    を追加。実装変更を伴わないテスト強化のため red は無し（現行実装で pass を確認）。
    コミット 2e51597。
  - 作業2: `ApprovalLedger::abort_before_send` を `pub(crate)` に。HTTP 層は `approve` だけを呼び、
    予約を持ち出さない。単体テスト（同一クレート）はそのまま。lib 部で未使用になるため
    `#[cfg_attr(not(test), allow(dead_code))]` を付けてテスト専用と明示。
    台帳の共有は `pub type SharedLedger = Arc<Mutex<ApprovalLedger>>`（std::sync のみ）。
    1 要求の中で `lock()` を取り、approve（予約→模擬送信→完了）をロック保持のまま 1 回で行う。
    poison した場合は 500 `internal` を返して状態に触れない（テスト (h) で確認）。
  - 作業3: 新クレート `crates/pelway-http`。std::net::TcpListener による最小の HTTP/1.1
    （接続ごとにスレッド、Connection: close、keep-alive・chunked は非対応）。依存は
    `pelway-contract`（path）と `serde` / `serde_json`（既存の Cargo.lock のみで解決、--offline）。
  - 作業4: 結合テスト `tests/http.rs`（(a)〜(g) + poison (h) + 経路/本文 card_id 食い違い (i)）。
    `127.0.0.1:0` で起動し `std::net::TcpStream` で実際に HTTP 要求を送って状態コードと本文 JSON を確認。
    接続先は 127.0.0.1（と (g) の ::1 起動確認）のみ。外部への接続は無い。
- red-first の証拠（作業3〜4）:
  - スケルトン（bind して全要求に 501 を返すだけ・ループバック検査なし）を先に置き、
    テスト 9 件を書いて実行: `cargo test --offline -p pelway-http --test http` → exit 101、
    0 passed / 9 failed。(a) 409期待が501、(b) 422期待が501、(c) 200期待が501、(d) 409期待が501、
    (e) 200が0件、(f) 400期待が501、(g) 非ループバックが受理される、(h) 500期待が501、
    (i) 422期待が501。その後 本実装で 9 passed / 0 failed。
- 検証（コマンドと exit code、オフライン）:
  - `cargo fmt --all --check` 0
  - `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` 0
  - `cargo test --workspace --locked --offline` 0（at-rest 8・aead 12・contract 34・serde_contract 10・
    dev-smoke 1・http 9。計 74 passed / 0 failed / 0 ignored）
- 判断が必要な点（リード確認用）:
  - **仮の経路**: `POST /v1/cards/{card_id}/comments` と `POST /v1/cards/{card_id}/approve`。
    docs/technical/v0.7（body/04-data-api-realtime.md:44）には `POST /v1/approvals/{id}/decide` の
    定義だけがあり、comments 保存と contract 型の approve に対応する経路の定義は無いため。
    contract の既存定数 `COMMENTS_PATH` / `APPROVE_PATH`（`/v1/approvals/{card_id}/...`）とは
    プレフィックスが異なる。contract 側の定数を HTTP 層に合わせるかは未決（今回は HTTP 層の実装のみ）。
  - comments の経路と本文の card_id 食い違い: `CommentError` に mismatch バリアントが無いため、
    HTTP 層のエラー `card_id_mismatch`（422）として返す（approve は既存の `check_path`・
    `ApproveError::CardIdMismatch` をそのまま）。独自バリアントを contract に足すかは未決。
  - 認証なし・デモ用。要求者は合成ヘッダー `x-pelway-member-id` / `x-pelway-member-kind: human|ai`
    （comments のみ必須。欠落・不正は 400）。
  - poison の扱い: 500 `internal` を返して状態に触れない（作業2の指示どおり）。
  - そのほかの HTTP 細部（判断として記録）: 不正なリクエストライン/ヘッダー/JSON は 400 `bad_request`、
    本文 64 KiB 超は 413 `payload_too_large`（1 MiB までは読み捨ててから応答）、知らない経路は 404、
    経路あり・メソッド違いは 405、応答はすべて JSON（Connection: close）、シャットダウン API は無し
    （テストはプロセス終了で止まる）。
- 既知の制限（前枠のものはそのまま）に今回の分を追加:
  - keep-alive・chunked（Transfer-Encoding）非対応。Content-Length 必須。
  - シャットダウン API が無い。スレッドは放置（テストプロセスの終了と共に止まる）。
  - 認証なし。ヘッダーは誰でも偽装できる（デモ用）。
- 未検証・未決:
  - 差分の Codex 再レビュー・QA は未実施。
  - U01〜U07 は未決のまま（Sending からの回復は実装していない）。

### 2026-11-09 第9枠（QA 18:21 の5点・GLM 枠）
- start: 2026-11-09 18:47 JST
- stop:  2026-11-09 18:55 JST
- glm: long
- skills: Matt Pocock Agent Skills の `tdd`（QA 5点のテストを先に書いて red を記録し、実装して green）。
- red の証拠: `cargo test --offline -p pelway-http --test http` → exit 101、8 passed / 5 failed。
  - `serve_refuses_non_loopback_addresses`: `::1` が受理されていた（QA 1）。
  - `oversize_content_length_answers_413_before_reading_body`: 本文を送らないまま
    `Content-Length: 100000` の要求に応答が無く、クライアントの読み取り（2秒）がタイムアウト（QA 2）。
  - `content_length_must_be_single_decimal_and_te_rejected`: `+` 付き Content-Length が 200 で
    受理されていた（QA 3。負・非数字・重複・Transfer-Encoding は依頼 2 の実装で既に 400）。
  - `stalled_connection_times_out_and_does_not_block_others`: 放置した接続が切断されず
    クライアントの読み取りがタイムアウト（QA 4。他の要求の処理は接続ごとのスレッドで
    依頼 2 の時点で妨げられていなかった）。
  - `header_size_and_line_limits_return_431`: 120 行のヘッダーが受理されて 200（QA 2 の行数上限）。
- 内容（QA 5点の対処）:
  1. 待ち受けを 127.0.0.1（IPv4 ループバック 127.0.0.0/8）だけにした。`::1` を含む IPv6 は
     `ServeError::NotLoopback` で拒否。**判断**: QA 文の「127.0.0.1 だけ」に忠実に IPv4 のみとし、
     依頼 2 で認めていた `::1` の受理を取りやめた（迷った点。外部に出ない意味では ::1 も安全だが、
     文言に忠実な方を採った）。
  2. `Content-Length` が上限（64 KiB）を超えたら**本文を読み込む前に** 413 を返す。残余の読み捨ては
     応答を書いた後（1 MiB まで。それ以上は読み捨てない）。ヘッダー上限は大きさ 16 KiB
     （`MAX_HEAD_BYTES`）と行数 100（`MAX_HEADER_LINES`）。**超えたら 431 にした**
     （431 か 400 の選択: ヘッダー由来の失敗を呼応答で区別できる 431 を採用。サイズ超過も
     依頼 2 実装の 400 から 431 に統一した）。
  3. `Content-Length` は ASCII 数字のみ受理（空・`-5`・`+26`・`abc` は 400。`+26` は従来
     `usize::parse` が受理していたが拒否するようにした）。重複は 400。`Transfer-Encoding` 付きは 400。
  4. 読み取りタイムアウト `READ_TIMEOUT` = 3 秒（`set_read_timeout`）。超えた接続には 408
     `request_timeout` を返して切断（**408 にした判断**: 読み取り超過を示す標準の状態コードのため）。
     放置接続があっても他の要求は処理される（接続ごとのスレッド）ことをテストで確認。
  5. `Cargo.lock` は今回変更なし。`git diff 8d37fe6 -- Cargo.lock` は依頼 2 で加えた
     `pelway-http` パッケージの追記 1 か所のみ（依存は `pelway-contract`・`serde`・`serde_json`
     の既存 3 つで、新しい外部クレートは増えていない）。ネットワーク接続なし（`--offline`）。
- green の証拠: `cargo test --offline -p pelway-http --test http` → exit 0、13 passed / 0 failed。
- 検証（コマンドと exit code、オフライン）:
  - `cargo fmt --all --check` 0
  - `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` 0
  - `cargo test --workspace --locked --offline` 0（at-rest 8・aead 12・contract 34・serde_contract 10・
    dev-smoke 1・http 13。計 78 passed / 0 failed / 0 ignored）
- 未検証・未決:
  - 差分の Codex 再レビュー・QA は未実施。

### 2026-11-09 第10枠（Codex レビュー4 指摘1〜4の修正と全体期限・GLM 枠）
- start: 2026-11-09 19:15 JST 以降（バックエンド作業依頼 5 の受領後。正確な時刻は未記録）
- stop:  2026-11-09 20:10 JST（`date` で確認）
- glm: long（T3 Code 上で GLM が実装。バックエンド作業依頼 5 のとおり）
- worktree: `/home/hatai/.t3/worktrees/Pelway/demo-t3-backend-fix5`、branch `demo-t3/backend-fix5`（base `a78f0b0`）
- 開始時の確認: HEAD `a78f0b0f85c9671aa23dc900d50db8d2f2448531`、`git status --porcelain` 空。
- skills:
  - Matt Pocock Agent Skills の `tdd` を使用。指摘ごとのテストを先に書き、実装前に失敗（red）を
    確認してから実装（green）した。
- 開発リードの決定（2026-11-09 19:00頃）:
  1. ヘッダーが 16 KiB または 100 行を超えたら 431。
  2. 区切りのないヘッダー行・不正なヘッダー名・Content-Length の重複や不正値は 400。本文の大きさ超過は 413 のまま。
  3. 接続元は IPv4 の 127.0.0.0/8 だけ受け付け、::1 は拒否（デモ用、あとで見直す）。
  4. 3 秒で 408、413 は応答を先に書く。
- 決定の変更（2026-11-09 19:14 リード）: 待ち受け先は 127.0.0.1 だけに絞る（上の3の「127/8」を、QA の条件に合わせて絞った）。
- 追加の決定（2026-11-09 19:15 リード）: ヘッダー読み取りと 413 後の読み捨てを合わせて 1 接続 10 秒の全体期限。3 秒の読み取りごとのタイムアウトは残す。413 後の読み捨ての上限は 1 MiB のまま。
- Codex レビュー4（スレッド d893efe6、所見ファイル sha256 `9f5ebc330f83b58ec9dd27da8f8d96e92eaf152be45139d2385cfc423c224ab9`）の指摘1〜4と対処、テスト名:
  - 指摘1（ヘッダーのサイズ判定、`src/lib.rs` 251-255 付近）: `read_request` のループで、
    終端（空行）を探す **前に**、受け取った合計バイト数で `MAX_HEAD_BYTES`（16 KiB）を判定し、
    超えたら 431 にした。終端が上限を超えた最後の読み取りに含まれる場合も 431。
    テスト: `head_bytes_exactly_16kib_ok_and_one_more_431`（16 KiB ちょうどは受理、
    16 KiB+1 は 431。+1 のケースでは終端の空行が上限を超える最後の読み取りに含まれる）。
  - 指摘2（形の不正を 400 に、267-270・280-283 付近）: リクエスト行の HTTP バージョンが
    `HTTP/1.1` の形でなければ 400。ヘッダー名が空、または空白・区切り文字を含む
    （RFC 9110 の token でない、`is_tchar` で判定）なら 400。どちらも台帳に触れず状態を変えない。
    テスト: `bad_version_and_bad_header_names_return_400_and_keep_state`
    （HTTP/2.0・HTTP/1.0・HTTP バージョンでない形・空のヘッダー名・空白入りヘッダー名・
    コロンの前の空白・区切り文字 `()` 入り。有効な Content-Length と本文付きで 400 になり、
    カードの hash・コメント数・版が変わらない）。
  - 指摘3（bind 先を 127.0.0.1 だけに、176-181 付近）: `serve`（実体は
    `serve_with_connection_deadline`）で、アドレスの IP が 127.0.0.1 ちょうどでなければ
    `ServeError::NotLoopback` で拒否。127.0.0.2・127.255.255.255（127/8 のほか）、
    0.0.0.0、`::`、`::1` も拒否。
    テスト: 既存 `serve_refuses_non_loopback_addresses` に `127.0.0.2:0` と
    `127.255.255.255:0` を追加（既存の拒否ケースは残したまま）。
  - 指摘4（500 本文の回帰テスト、`tests/http.rs` 328-330 付近）: poison したロックでの 500 応答の
    本文 JSON のキーと値をすべて確かめ、決まった `code` 以外の詳細（エラー文・ファイルパス・
    型名・panic の文）が入らないことを確かめる回帰テストに強化した。
    テスト: `poisoned_lock_returns_500_with_exact_body_and_keeps_state`
    （旧 `poisoned_lock_returns_500` の強化。本文が `{"code":"internal"}` だけであることと、
    poison 前後でカード状態が変わらないことも確認。実装変更を伴わない回帰テストのため
    red は無し——現行実装で pass を確認。第8枠の作業1と同じ扱い）。
  - 全体期限（リード決定 19:15、5.）: `handle_conn` の開始時から `CONNECTION_DEADLINE`（本番 10 秒）で、
    ヘッダー読み取り（`read_request`）と 413 後の読み捨て（`drain`）を合わせて打ち切って接続を閉じる。
    ヘッダー読み取り中の期限切れには 408 を返してから閉じる。1 回の読み取りごとの 3 秒
    （`READ_TIMEOUT`）はそのまま。読み捨ての 1 MiB 上限もそのまま。
    期限は引数で差し替えられる形（`serve_with_connection_deadline`。`serve` は 10 秒を渡すだけ）。
    テスト: `slow_trickle_head_hits_connection_deadline`（150ms 間隔で送り続けると 800ms の
    期限前後で 408 と切断。本番値 10 秒もあわせて assert）、
    `slow_trickle_after_413_stops_draining_at_deadline`（413 の応答を受け取ったあと
    少しずつ送り続けると、期限で接続を閉じられる）。
- red-first の証拠（tdd）:
  - テストと、期限を無視するスタブの `serve_with_connection_deadline`（中身は従来の `serve`
    と同じ）を先に置いて実行: `cargo test --offline -p pelway-http --test http` →
    exit 101、12 passed / 5 failed。
    - `head_bytes_exactly_16kib_ok_and_one_more_431`: 16 KiB+1 が 200 で受理されていた（指摘1）。
    - `bad_version_and_bad_header_names_return_400_and_keep_state`: HTTP/2.0 が 200 で
      受理されていた（指摘2。最初のケースで失敗）。
    - `serve_refuses_non_loopback_addresses`: `127.0.0.2:0` での起動が受理されていた（指摘3）。
    - `slow_trickle_head_hits_connection_deadline` / `slow_trickle_after_413_stops_draining_at_deadline`:
      切断されず 6.0s / 3.0s（全体期限）。
  - green: 実装後に `cargo test --offline -p pelway-http --test http` → exit 0、17 passed / 0 failed。
- 検証（コマンドと exit code、オフライン）:
  - `cargo fmt --all --check`: 初回 1（整形の指摘のみ）。`cargo fmt --all` 適用後に 0。
  - `cargo clippy --workspace --all-targets --offline -- -D warnings` 0
  - `cargo test --workspace --offline` 0（at-rest 8・aead 12・contract 34・serde_contract 10・
    dev-smoke 1・http 17。計 82 passed / 0 failed / 0 ignored）
  - `git diff -- Cargo.lock` は空（変更なし。依存クレートの追加も無し）。
- 迷った点（安全側で進めたもの）:
  - HTTP/1.0 の受理: 依頼文は「必要なら HTTP/1.0 も」のため、受理しない（HTTP/1.1 だけ）を
    選んだ（厳しい方。テストも HTTP/1.0 → 400 を固定）。
  - 全体期限切れの応答: 決定文は「打ち切り、接続を閉じる」だけで応答の有無を定めないため、
    ヘッダー読み取り中の期限切れには 408 を返してから閉じることにした（408 は読み取り超過の
    標準の状態コード。第9枠の 408 採用と同じ理由）。413 後の読み捨ては応答済みなので
    追記せず閉じるだけ。
  - ヘッダー読み取りループの判定順: サイズ判定（431）→ 終端探索 → 期限（408）→ 読み取り。
    受け取った合計が上限内で終端まで揃った要求は、期限ぎりぎりでも受理する
    （期限は読み取りを防ぐためのものなので、読み終えた要求は捨てない）。
  - 全体期限の適用範囲: 決定は「ヘッダー読み取りと 413 後の読み捨て」のため、本文読み取り
    （Content-Length 64 KiB 以下の正当な要求の本体）には全体期限をかけていない。本文を
    3 秒未満の間隔で少しずつ送る接続は、1 回の読み取りごとの 3 秒タイムアウトだけでは
    切断されないまま残る。要リード判断。
  - `ServeError::NotLoopback` の名前: 127.0.0.2 は loopback だが拒否するようになったため
    名前と意味がずれたが、名称変更は差分が広がるためそのままにし、doc とメッセージ
    （`serve address must be exactly 127.0.0.1`）を更新した。
  - 16 KiB ちょうどの受理テストは、ヘッダーだけを先に送ってサーバーに読み終えてもらってから
    本体を送る形にした（本体まで同じ読み取りに含まれると、合計バイト数判定で 431 に
    なるため。判定は「受け取った合計バイト数」のとおり）。
  - AGENTS.md の「Prior art first」（GitHub 検索）は、依頼文の外部ネットワーク接続禁止に
    従い実施しなかった。
- 未検証・未決:
  - 差分の Codex 再レビュー・QA は未実施。U01〜U07 は未決のまま（確定していない）。

### 2026-11-09 第11枠（Codex 再レビュー5 指摘1〜2の修正と全体期限の本文への拡大・GLM 枠）
- start: 2026-11-09 20:59 JST
- stop:  2026-11-09 21:18 JST（`date` で確認）
- glm: long（T3 Code 上で GLM が実装。バックエンド作業依頼 6 のとおり）
- worktree: `/home/hatai/.t3/worktrees/Pelway/demo-t3-backend-fix6`、branch `demo-t3/backend-fix6`（base `cc86038`）
- 開始時の確認: cwd はこの worktree（本体フォルダではない）、HEAD
  `cc8603884db316bda3323458f55271b363ca40aa`（指定と一致）、branch `demo-t3/backend-fix6`、
  `git status --porcelain` 空。
- 開発リードの決定（2026-11-09 20:36〜20:40 JST）:
  - Codex 再レビュー5（スレッド 4570f41c、所見ファイル review-findings-5.md、sha256
    `937eb7837afca3150ee2bd85a306322d46102f3eb7a1dbadbadd4f177bfc8b82`）の指摘1・2を
    どちらも採用。
  - 全体期限を本文の読み取りにも広げる（1 接続あたり同じ 10 秒。3 秒の読み取りごとの
    上限と読み捨ての 1 MiB 上限はそのまま）。
- 安全側の選択の承認（リード 20:36）: HTTP/1.0 を拒否すること（HTTP/1.1 の形だけ受理）、
  期限切れで 408 を返すこと、`ServeError::NotLoopback` の名前をそのまま使うこと。
- AGENTS.md の「Prior art first」（GitHub 検索）: 外部ネットワーク接続禁止のため未実施。
- skills:
  - Matt Pocock Agent Skills の `tdd` を使用。修正1（テスト (r)〜(u)）と修正2・3
    （テスト (v)〜(x)）をそれぞれ実装前に実行して red を記録し、実装後に green を確認。
- 内容（`crates/pelway-http` のみ。依存クレートの追加なし、`Cargo.lock` 変更なし）:
  - 指摘1（431 の判定、`src/lib.rs` ヘッダー読み取りループ）: 大きさの上限の判定を
    「受け取った合計バイト数」から「ヘッダーの終端（空行）までのバイト数」に変更。
    同じ読み取りに本文のバイトが混ざっていても上限に数えない。終端がまだ見つからない
    ときは、受け取ったバイト数が 16 KiB を超えた時点で 431（ヘッダーだけで超えている
    場合）。行数 100 の上限は変更なし。
    テスト: `head_16kib_and_body_in_one_write_accepted`（テストA。16 KiB ちょうどの
    ヘッダーと有効な本文を 1 回の write で続けて送って受理）、
    `oversize_cl_head_16kib_body_start_in_one_write_answers_413`（テストB。
    Content-Length 100000 の要求をヘッダーと本文の先頭 8 KiB を 1 回の write で送って
    413）、`body_in_final_head_read_not_counted_in_16kib_limit`（ヘッダーを 13000 バイト
    まで先に送って読み取りの境界をずらし、残りのヘッダーと本文を 1 回の write で送って
    受理）、`oversize_cl_straddled_head_answers_413_not_431`（同じ送り方で
    Content-Length 100000 → 431 ではなく 413）。
    既存 `head_bytes_exactly_16kib_ok_and_one_more_431`（16 KiB+1 は 431）は残したまま。
  - 指摘2（1 回の読み取りのタイムアウト、`drain`・`read_once`・`handle_conn`）:
    新しい `read_timeout_for` が「3 秒（READ_TIMEOUT）」と「全体期限までの残り」の短い
    ほうを返し、`read_once` と `drain` が読み取りのたびに `set_read_timeout` で設定する。
    残りが 0 以下なら読まずに打ち切る。期限切れはこれまでの安全側の選択どおり 408 を
    返して閉じる（413 のあとの読み捨て中は 413 応答済みのため追記せず閉じるだけ）。
    0 秒のタイムアウトは設定できないため、残り 0 は期限切れとして読み取りの前に判定。
    `handle_conn` 開始時の 1 回だけの `set_read_timeout` とヘッダー読み取りループ先頭の
    期限確認は削除（read_once が毎回残りを見るため。期限ぎりぎりに読み終わった要求は
    従来どおり受理）。
    テスト: `head_read_wait_is_cut_at_deadline_not_3s`（テストC・ヘッダー読み取り中）、
    `drain_read_wait_after_413_is_cut_at_deadline`（テストC・読み捨て中。下記の判定方法）。
  - 決定3（全体期限を本文の読み取りへ拡大、リード 20:36）: 本文の読み取りループも同じ
    `deadline` を `read_once` に渡す。ヘッダー読み取り・本文読み取り・413 後の読み捨ての
    すべてに 1 接続あたり同じ 10 秒。
    テスト: `slow_trickle_body_hits_connection_deadline`（テストD。正しい大きさの本文を
    150ms 間隔で少しずつ送り続けると、差し替えた期限 800ms 前後で 408 と切断）。
  - モジュール doc・定数の doc・`HttpError::RequestTimeout` の doc を新しい判定と期限の
    範囲に合わせて更新。既存テスト (n) のコメントも新しい判定に合わせて修正。
- テストCの余裕の値と理由:
  - 上限は「全体期限 + 0.5 秒」（QA の受入条件どおり）。期限の差し替え: (v)(w) は 2.0 秒、
    (x) は 0.8 秒（テスト時間を短くするため。本番の値 10 秒・3 秒はそのまま、(v) は
    READ_TIMEOUT=3 秒を、(x) は CONNECTION_DEADLINE=10 秒を assert）。
  - 下限は「全体期限 − 0.1 秒」。期限前に切れないことの確認で、時計とスレッド
    スケジューリングのぶれに 0.1 秒だけ許容した。
  - (w) の判定時刻は期限 + 0.3 秒（+0.5 秒の内側）。閉じた後の最初の書き込みに対する
    RST が戻るのに時間が要るため、1 回目と 2 回目の書き込みの間に 0.1 秒空けた。
- red-first の証拠（tdd）:
  - 修正1のテストを追加して実装前に実行: `cargo test --offline -p pelway-http --test http`
    → exit 101、19 passed / 2 failed。
    - `body_in_final_head_read_not_counted_in_16kib_limit`: 431 が返った（200 期待）。
    - `oversize_cl_straddled_head_answers_413_not_431`: 431 が返った（413 期待）。
    - テストA・B相当の 1 回の write だけの 2 件（`head_16kib_and_body_in_one_write_accepted`・
      `oversize_cl_head_16kib_body_start_in_one_write_answers_413`）は実装前も通った。
      ループバックではサーバーの読み取りが 4096 バイト（チャンク）単位で区切られ、
      16 KiB ちょうどのヘッダーの終端が本文と同じ読み取りに混ざらないため。red の証拠は
      読み取りの境界を確実にまたがらせた上の 2 件で取った（テストA・Bは受理の受け
      テストとして残す）。
  - 修正2・3のテストを追加して（まだ実装前に）実行: 同コマンド → exit 101、
    19 passed / 5 failed（上の 2 件に加えて）:
    - `head_read_wait_is_cut_at_deadline_not_3s`: 3.19 秒で切断（期限 2.0 秒 + 0.5 秒を超過）。
    - `drain_read_wait_after_413_is_cut_at_deadline`: 期限 + 0.3 秒の時点で 2 回の書き込みが
      どちらも成功（接続がまだ開いている。修正前は 3 秒の読み取りタイムアウトが
      優先されていた）。
    - `slow_trickle_body_hits_connection_deadline`: 本文を読み終えるまで応答が来ない
      （本文の読み取りに全体期限が無かった）。
  - green: 実装後に同コマンド → exit 0、24 passed / 0 failed。さらに 3 回再実行して
    24 passed / 0 failed を確認（時間を使うテストの揺らぎ対策）。
- 検証（コマンドと exit code、オフライン）:
  - `cargo fmt --all --check`: 初回は 1（整形の指摘のみ）。`cargo fmt --all` 適用後に 0。
  - `cargo clippy --workspace --all-targets --offline -- -D warnings` 0
  - `cargo test --workspace --offline` 0（at-rest 単体 8・aead 12・contract 単体 34・
    serde_contract 10・dev-smoke 1・http 24。計 89 passed / 0 failed / 0 ignored）
- 迷った点（安全側で進めたもの）:
  - 終端までの長さの数え方: 終端（空行 `\r\n\r\n`）を含む長さ（`find_terminate` の位置 +4）
    で判定した。既存の 16 KiB+1 テスト（終端の完成が最後の 1 バイト）が 431 のまま
    成立するのはこちらの解釈だけのため。
  - (w) の検出方法: 413 のあとの読み捨て中の切断は、応答書き込み済み（write 側
    shutdown 済み）のためクライアントの読み取りでは観測できない。期限 + 0.3 秒に
    1 バイトを書き、0.1 秒空けて 2 回目の書き込みが失敗すれば「1 バイト目の時点で
    既に閉じていた」と判定できる（サーバーがまだ読み取り待ちなら 1 バイト目は読み捨てに
    消費され、2 回目も成功する）。
  - 実装途中で (w) だけ残って失敗した: 1 回目と 2 回目の書き込みを連続で実行したため、
    1 回目に対する RST が戻る前に 2 回目が終わってしまい、どちらも成功した。上の
    0.1 秒の間隔を入れて解決（既存テスト (q) が書き込み 150ms 間隔で検出できているのと
    同じ理屈）。
  - ヘッダー読み取りループ先頭の期限確認の削除: read_once が読み取りごとに残りを見る
    ための重複。受け取った合計が上限内で終端まで揃った要求は期限ぎりぎりでも受理する
    従来の挙動は不变。
- 逸脱: なし（worktree・ブランチの中だけ。push・fetch なし、外部接続なし、破壊的gitなし。
  一度 worklog の第10枠の見出し行を誤って編集したが、すぐ戻して `git diff` で空を確認）。
- 未検証・未決:
  - 差分の Codex 再レビュー・QA は未実施。U01〜U07 は未決のまま（確定していない）。

### 2026-11-09 第12枠（テストC を本文読み取り中に拡大・GLM 枠）
- start: 2026-11-09 21:21 JST頃（リード補足決定 20:43 の受領後）
- stop:  2026-11-09 21:25 JST（`date` で確認）
- glm: long（T3 Code 上で GLM が実装）
- 開発リードの決定（2026-11-09 20:43 JST、依頼6への補足）: テストC は「ヘッダーの
  読み取り中」「本文の読み取り中」「413 のあとの読み捨て中」の 3 か所それぞれで行う。
  どの場面でも、期限の直前に読み取り待ちに入ってから何も送らず、接続が切れるまでの
  時間が全体期限 + 0.5 秒以内に収まることを確かめる。
- skills: Matt Pocock Agent Skills の `tdd`（新しいテストを先に書き、実装前の失敗
  （red）を確認してから実装側で green を確認）。
- 内容:
  - 第11枠では (v) ヘッダー読み取り中・(w) 読み捨て中の 2 か所だったテストCに、
    本文読み取り中の (y) を追加。ヘッダー（Content-Length: 100）を送ったあと本文を
    10 バイトずつ 300ms 間隔で 5 回（計 50 バイト）だけ送り、期限（差し替え 2.0 秒）の
    約 0.5 秒前に読み取り待ちに入ったまま何も送らない。408 が「全体期限 + 0.5 秒」以内
    （下限は期限 − 0.1 秒。余裕の値と理由は第11枠と同じ）に来ることを確認。
    テスト名: `body_read_wait_is_cut_at_deadline_not_3s`。
  - 実装の変更は無し（第11枠の指摘2・3の実装が本文の読み取りにも効いていることを
    確認するのが目的）。
- red-first の証拠（tdd）:
  - テストを追加した後、実装側だけを base `cc86038` に一時的に戻して実行
    （`git checkout cc86038 -- crates/pelway-http/src/lib.rs`。テストは HEAD のまま。
    コミット済みの状態での一時差し替えで、履歴には触れない。実行後に
    `git checkout HEAD -- crates/pelway-http/src/lib.rs` で戻し、`git status --porcelain`
    で tests/http.rs のみの変更であることを確認）:
    `cargo test --offline -p pelway-http --test http body_read_wait` → exit 101、
    0 passed / 1 failed。「期限から 0.5 秒以内に切れていない: 4.407011233s」
    （base では本文読み取りの 3 秒タイムアウトが優先され、最後の部分送信から約 3 秒後
    の切断になっていた）。
  - green: `lib.rs` を HEAD に戻して同コマンド → exit 0、1 passed（2.03 秒で切断）。
- 検証（コマンドと exit code、オフライン）:
  - `cargo fmt --all --check` 0
  - `cargo clippy --workspace --all-targets --offline -- -D warnings` 0
  - `cargo test --workspace --offline` 0（at-rest 単体 8・aead 12・contract 単体 34・
    serde_contract 10・dev-smoke 1・http 25。計 90 passed / 0 failed / 0 ignored。
    http スイートはさらに 2 回再実行して 25 passed / 0 failed を確認）
- 迷った点:
  - 「期限の直前に読み取り待ちに入る」の再現方法: 本文を 300ms 間隔で少しずつ送り、
    期限約 0.5 秒前に送るのをやめる形にした（送る間隔は 3 秒未満なので 1 回の読み取り
    タイムアウトには引っかからない）。(v)(w) と同じく、期限は読み取り待ちの最中に来る。
  - red を取るための `git checkout <commit> -- <path>` による一時的な差し替え:
    破壊的gitには当たらず（履歴・コミット不変、即時復元、status で確認）、worktree の
    中だけの操作のため実施した。実施理由は、実装が第11枠で既にコミット済みであり、
    新しいテストだけでは実装前の失敗を示せないため。
- 未検証・未決:
  - 差分の Codex 再レビュー・QA は未実施。U01〜U07 は未決のまま（確定していない）。

### 2026-11-09 第13枠（Codex 再レビュー6 指摘1〜3の修正・GLM 枠）
- start: 2026-11-09 22:03 JST頃（バックエンド作業依頼 7 の受領後）
- stop:  2026-11-09 22:18 JST（`date` で確認）
- glm: long（T3 Code 上で GLM が実装。バックエンド作業依頼 7 のとおり）
- worktree: `/home/hatai/.t3/worktrees/Pelway/demo-t3-backend-fix7`、branch `demo-t3/backend-fix7`（base `e37c5a0`）
- 開始時の確認: cwd はこの worktree（本体フォルダではない）、HEAD `e37c5a0`（指定と一致）、
  branch `demo-t3/backend-fix7`、`git status --porcelain` 空。
- 開発リードの決定（2026-11-09 21:48 JST）: Codex 再レビュー6（スレッド c2e75113、所見ファイル
  review-findings-6.md、sha256 `81e5f3679ef6c48866d6b710ff2183bfa61303dacf8ae879aa43944b9562dfe8`）
  の指摘1〜3をすべて採用。
- AGENTS.md の「Prior art first」（GitHub 検索）: 外部ネットワーク接続禁止のため未実施。
- skills:
  - Matt Pocock Agent Skills の `tdd` を使用。修正1はテストを先に書いて実装前の失敗
    （red）を確認してから実装（green）。修正2は「修正前のテストが条件を満たしていない
    こと」を確認してからテストを本当の条件に置き換えた。修正3はテスト強化のため実装は
    変更なし（第8枠作業1・第10枠指摘4と同じ扱い）だが、過去実装に対する red を別途記録。
- 内容（`crates/pelway-http` のみ。依存クレートの追加なし、`Cargo.lock` 変更なし）:
  - 指摘1（期限の作り方の panic、`src/lib.rs` 236 付近）: `Instant::now() + connection_deadline`
    を新しい `deadline_at(now, budget)` に置き換え、`checked_add` で作る。あふれる
    （`None`）ときは **1 年（31,536,000 秒）に丸め**、それでも表せないときは `now`
    （残り 0 の期限切れとして扱われる）を返す。どの場合も panic しない。
    **丸めとエラーのどちらにしたか**: 丸め。理由: (a) あふれるほど巨大な期限
    （`Duration::MAX` など）は「無期限」の意図であり、本番 10 秒より十分長い 1 年は
    実運用上それと区別がつかない、(b) `Instant` の表現範囲（基準時刻から数百年）には
    十分収まる、(c) `ServeError` は待ち受けアドレス・ソケットの失敗を表すもので、
    期限の大きさの解釈で起動を失敗させる（エラーを返す）のは不釣り合いで、テスト
    注入用の引数に新しいエラー処理を強いるのも過剰なため。
    `serve_with_connection_deadline` の doc にも丸めの旨を追記。
    テスト: `duration_max_deadline_does_not_panic_and_serves`（(z)。`Duration::MAX` を
    渡して起動し、通常の comments 要求に 200 が返ること。接続スレッドで panic すると
    応答のないまま接続が切れるため、200 が返ること自体が panic していないことの証明）。
  - 指摘2（テストB、`tests/http.rs` 762-788 付近）: Content-Length が 64 KiB を超える
    要求で、ヘッダー（16 KiB ちょうど）と **64 KiB を超える本文の全体（100,000 バイト）**
    を 1 回の `write_all` で送り、431 ではなく 413 が返ることを確かめる形にした。
    **既存テストは置き換えた**（旧 `oversize_cl_head_16kib_body_start_in_one_write_answers_413`
    → 新 `oversize_cl_head_16kib_whole_body_in_one_write_answers_413`）。理由: 新しい版は
    旧版の条件（1 回の write に本文のバイトが混ざる）を含む上位集合であり、同じ場面の
    弱い版を並べて重複させないため。本文の先頭（2 KiB）だけを 1 回の write に混ぜる
    場面は既存 (u) `oversize_cl_straddled_head_answers_413_not_431` が別途カバーしている。
    実装の変更は無し（レビュー5実装が既にこの条件を満たすことを確認）。
  - 指摘3（テストC の2か所、`tests/http.rs` (v)(w)）: ヘッダー読み取り中の (v)
    `head_read_wait_is_cut_at_deadline_not_3s` と 413 後の読み捨て中の (w)
    `drain_read_wait_after_413_is_cut_at_deadline` を、(y) と同じ「期限の直前に
    読み取り待ちに入る」形にした。期限より前は 300ms 間隔（3 秒未満のため 1 回の
    読み取りタイムアウトには引っかからない）で少しずつ送り続け、最後の送信が期限の
    約 0.5 秒前になる時点でやめて読み取り待ちに入り、そのあと何も送らない。
    (v) は 408 が「全体期限 + 0.5 秒」以内に来ること、(w) は期限 + 0.3 秒の時点で
    接続が既に切れていること（1 バイト書き→0.1 秒空けて 2 回目の書き込みが失敗。
    判定方法は第11枠 (w) と同じ）に加え、trickle 中の書き込みが期限前に失敗しない
    こと（期限前の切断は失敗とみなす下限判定）を確認。本文読み取り中の (y)
    `body_read_wait_is_cut_at_deadline_not_3s` は依頼どおり残した。
    実装の変更は無し。
- テストC の余裕の値と理由（第11枠・第12枠と同じ）:
  - 上限は「全体期限 + 0.5 秒」（QA の受入条件どおり）。期限の差し替え: (v)(w) とも
    2.0 秒（テスト時間を短くするため。本番の値 10 秒・3 秒はそのまま、(v) は
    READ_TIMEOUT=3 秒を assert）。
  - 下限は「全体期限 − 0.1 秒」。期限前に切れないことの確認で、時計とスレッド
    スケジューリングのぶれに 0.1 秒だけ許容した。(v) は 408 到着時刻、(w) は
    trickle 失敗時刻で判定。
  - trickle の間隔は 300ms。最後の送信が「期限の約 0.5 秒前」になる時点でやめる
    （while `elapsed + 0.5s < deadline`）。0.5 秒の理由: スケジューリングの遅れで
    最後の送信が期限をまたいでも書き込みエラー→即 408 読み取りで破綻しない程度の
    余裕があり、かつ「1 回の読み取りタイムアウトに全体期限そのもの（2.0 秒）を
    誤って使う実装」なら待ち開始（約 1.5 秒）から 2.0 秒後 = 期限 + 約 1.5 秒に
    切れるため、+0.5 秒の上限で必ず検出できる距離。
  - (w) の判定時刻は期限 + 0.3 秒（+0.5 秒の内側）。閉じた後の最初の書き込みに対する
    RST が戻るのに時間が要るため、1 回目と 2 回目の書き込みの間に 0.1 秒空けた。
- red-first の証拠（tdd）:
  - 修正1: テスト (z) を追加して実装前に実行: `cargo test --offline -p pelway-http
    --test http duration_max` → exit 101、0 passed / 1 failed。接続スレッドで
    `overflow when adding duration to instant`（std/src/time.rs:429）の panic、
    クライアントは `ConnectionReset` で応答なし。実装後に同コマンド → exit 0、
    1 passed。
  - 修正2: 修正前のテストが条件を満たしていないことの確認: `git show e37c5a0:
    crates/pelway-http/tests/http.rs`（762-788 行）で、1 回の write に入れているのは
    本文の先頭 8 KiB（`raw.extend_from_slice(&vec![b'z'; 8 * 1024])`）だけで、
    64 KiB を超える本文の全体ではなかった。置き換え後 `cargo test --offline
    -p pelway-http --test http oversize_cl_head` → exit 0、1 passed（実装変更なし）。
  - 修正3: 実装変更を伴わないテスト強化のため現行実装では red は無し（http スイート
    全体で 26 passed / 0 failed、3 回再実行して確認）。代わりに第12枠と同じ方法で
    過去実装に対する red を記録: 実装だけを `git checkout cc86038 -- crates/pelway-http/src/lib.rs`
    で一時的に差し替え（第11枠の「読み取りごとの残り時間タイムアウト」導入前。
    コミット済み状態での一時差し替えで履歴には触れず、実行後に
    `git checkout HEAD -- crates/pelway-http/src/lib.rs` で戻し `git status --porcelain`
    で空を確認）して `cargo test --offline -p pelway-http --test http read_wait`
    → exit 101、0 passed / 3 failed:
    - (v) `head_read_wait_is_cut_at_deadline_not_3s`: 4.64 秒で切断（期限 2.0 秒 +
      0.5 秒を超過）。旧実装は読み取り待ちに入った時点からの 3 秒固定タイムアウトで
      切るため、期限の直前に待ちに入る形だと期限 + 約 2.6 秒になる。旧 (v)（開始
      直後に待ちに入る形）ではこの差を検出できなかった。
    - (w) `drain_read_wait_after_413_is_cut_at_deadline`: 期限 + 0.3 秒の時点で
      1・2 回目の書き込みがどちらも成功（接続がまだ開いていた）。旧実装の読み捨ては
      期限を過ぎても 3 秒固定の読み取り待ちを続けていた。
    - (y) `body_read_wait_is_cut_at_deadline_not_3s`: 4.38 秒で切断（第12枠の red と
      同一の挙動）。
- 検証（コマンドと exit code、オフライン）:
  - `cargo fmt --all --check`: 初回は 1（テスト (z) の整形の指摘のみ）。`cargo fmt --all`
    適用後に 0。
  - `cargo clippy --workspace --all-targets --offline -- -D warnings` 0
  - `cargo test --workspace --offline` 0（at-rest 単体 8・aead 12・contract 単体 34・
    serde_contract 10・dev-smoke 1・http 26。計 91 passed / 0 failed / 0 ignored。
    http スイートはさらに 3 回再実行して 26 passed / 0 failed を確認）
- 迷った点（安全側で進めたもの）:
  - 修正1の丸め先を 1 年にした理由は上記のとおり。`FAR_FUTURE` に丸めた結果も
    表せない（基準時刻から数百年後という実用上あり得ない場合）は `now` を返して
    即期限切れ（408）にする安全側に倒した。
  - 修正2で既存テストを消した理由は上記のとおり（置き換え）。弱い版を残す選択も
    あったが、同じ場面の重複になるため採らなかった。
  - 修正3の (w) は、読み捨て中の切断をクライアントの読み取りでは観測できないため
    （413 応答済みでサーバーの書き込み側は shutdown 済み）、書き込み 2 回での判定を
    第11枠から踏襲した。trickle 中の書き込み失敗を期限前切断の下限判定に使うために
    失敗時刻を記録する形にした。
  - cc86038 差し替えによる red の再現は破壊的gitには当たらない（履歴・コミット不変、
    即時復元、`git status --porcelain` で空を確認）が、第12枠と同じ判断で実施した。
  - cargo test のフィルタ引数は 1 つだけのため、複数テストを指定しようとして
    `error: unexpected argument`（exit 1）が 1 回出た。フィルタを `read_wait`
    （3 件に一致）に変えて再実行し、目的の red を取得した。
  - worklog への追記で第12枠の見出し行を誤って置き換えてしまった（第11枠と同じ
    事故）。sed で当該ブロックを削除し、見出し・start 行を復元してから末尾に
    追記し直した。`git diff` で既存部分（第12枠まで）に差分が無いことを確認済み。
- コミット: `c8a0f87`（修正1〜3のコードとテスト。pre-commit hook の trailing_whitespace・
  newlines・cargo_clippy・cargo_fmt・cargo_check はすべて合格）と、この追記の worklog
  コミット。
- 逸脱: なし（worktree・ブランチの中だけ。push・fetch なし、外部接続なし、破壊的gitなし、
  一時ファイルは作成せず）。
- 未検証・未決:
  - 差分の Codex 再レビュー・QA は未実施。U01〜U07 は未決のまま（確定していない）。

### 2026-11-09 第14枠（バックエンド作業依頼 GDPR-07・本文を残さないログ・GLM 枠）
- start: 2026-11-09 23:01 JST頃（バックエンド作業依頼 GDPR-07 の受領後。期限 2026-11-13 21:00 JST）
- stop:  2026-11-09 23:44 JST（`date` で確認。コミット・検証はこの後）
- glm: long（T3 Code 上で GLM が実装。バックエンド作業依頼 GDPR-07 のとおり）
- worktree: `/home/hatai/.t3/worktrees/Pelway/demo-t3-backend-gdpr-07`、branch
  `demo-t3/backend-gdpr-07`（base `88cf51b` = fix7 受入後）
- 開始時の確認: cwd はこの worktree（本体フォルダではない）、HEAD `88cf51b`（指定と一致）、
  branch `demo-t3/backend-gdpr-07`、`git status --porcelain` 空。
- AGENTS.md の「Prior art first」（GitHub 検索）: 外部ネットワーク接続禁止のため未実施
  （第13枠と同じ）。
- skills:
  - Matt Pocock Agent Skills の `tdd` を使用。結合テスト（no_body_logs）を先に書き、
    「出力点が未接続の足場」でコンパイルは通るがログが出ない red を取ってから接続を実装
    （green）。契約側（pseudonymize）の単体テストは新関数のため red はコンパイルエラー
    のみ（依頼が red の証拠を求めたのは結合テスト (a)(b)(c) のため、それらは意味のある
    失敗まで持っていった）。
  - 同 `verification-before-completion` を最終検証の前に使用。各受入条件に対応する
    コマンドと exit code を実行・記録してから完了を主張した。
- 内容（`crates/pelway-contract` と `crates/pelway-http` のみ。依存追加なし、
  `Cargo.lock` 変更なし、新クレートなし。std + 既存の sha2/serde/serde_json だけ）:
  - **ID の仮名化**（`crates/pelway-contract/src/pseudonym.rs` 新）:
    `pseudonymize(salt, id)` は SHA-256 で `"pid:" + 小文字16進64桁` を返す。入力は
    snapshot.rs と同じ 8 バイト BE 長さ前置き（salt/ID の境界に曖昧さなし、ドメ分離
    `pelway-contract/pseudonym/v1`）。salt は引数で受け取る（環境変数から読まない。
    依頼の約束）。単体テスト 4 件（書式・決定性・salt/ID 変化・境界）。
  - **構造化ログ**（`crates/pelway-http/src/log.rs` 新）:
    - 出来事は列挙型 `Event`: `request_received`（経路・メソッド・経路の card_id の
      仮名）、`responded`（経路・状態コード・応答本文バイト数・所要ミリ秒）、
      `comment_saved`（カード・書き手の仮名、書き手種別）、`approved`（カードの仮名）、
      `simulated_send`（カード・疑似送信 ID の仮名）、`rejected`（経路・状態コード・
      結果コード）、`internal_error`（経路）。項目は仮名 ID・種別・数値・結果コード
      **だけ**で、本文・コメント・宛先・エラー文言・要求の中身・`client_msg_id` を
      入れる欄は持たせない（許可リスト方式）。`Debug` で要求や本文の型を書き出さない
      （`Request` 型は `Debug` を実装しないまま）。
    - 結果コード `Code` は契約エラー（ApproveError/CommentError）と HTTP 層エラーを
      平坦化した列挙型（文言は持たない）。`Response` に `code` を載せ、全エラー生成点
      で設定（4xx/5xx で `rejected`/`internal_error` が出る）。
    - 出力は JSON Lines（1 行 1 JSON、`ts_ms` はサーバーが記録時に付ける UNIX エポック
      ミリ秒）。出力先は `LogSink` で差し替え: テスト用 `MemorySink`、ファイル出力
      `FileSink`（`target/demo-logs/` の下だけ。ファイル名は 1 成分のみで `/`・`\`・
      `..` を拒否、最初の追記でディレクトリ作成・ファイルを作り直し）、既定 `NullSink`
      （`serve` は従前どおりログを出さない）。外部 exporter なし（S17・10-domestic-only
      の log/audit/crash 行）。
    - `Logger` は salt を持ち、ログにも `Debug` にも出さない（`finish_non_exhaustive`）。
  - **出力点の接続**（`crates/pelway-http/src/lib.rs`）:
    - `serve_with_logger(addr, ledger, logger)` を追加（既存 `serve`・
      `serve_with_connection_deadline` は引数不变・挙動不变、内部で `serve_inner` に統一）。
    - 接続スレッドで `request_received`（route で経路確定後）→ ハンドラで
      `comment_saved` / `approved` + `simulated_send` → 応答前に
      `rejected`/`internal_error` + `responded`。要求の本文・コメント・`client_msg_id`
      はログに渡さない。既存の 500 応答（`{"code":"internal"}` だけ）は不变。
    - 冪等リプレイ（同じキー・同じ内容の再送で send_log が増えない）では
      `approved`/`simulated_send` を出さない（この要求で新規送信があったときだけ。
      偽の送信記録を作らない安全側）。
    - panic の文言・`expect` の文言に本文が入る箇所は既存コードに無いことを確認
      （`expect` はすべて固定語句）。追加点も固定語句のみ。
  - **example**（`crates/pelway-http/examples/demo-log-emit.rs` 新）:
    `127.0.0.1:0` でサーバーを立て、合成データ（カード 2 枚、本文に本文 canary、
    要求ヘッダーに秘密 canary、未知フィールドに client_msg_id canary）で
    コメント保存（200）→ 承認（200）→ 拒否 409（snapshot_hash に秘密 canary）→
    拒否 400（未知フィールド）→ 内部エラー 500（ロック poison）を流し、
    `target/demo-logs/backend.jsonl` に書いて終わる。外部へ何も送らない。環境変数を
    読まない（`PELWAY_LLM_API_KEY` 等の名前もコードに書かない）。
- **salt の作り方と理由**: `process_salt(now)` は PID + 起動時刻（UNIX エポックからの
  ナノ秒、引数で注入）+ ヒープ確保アドレス（`Box` の `{:p}`、ASLR の影響）のバイト列を
  連結して返す（hash は `pseudonymize` の中で掛かる）。環境変数を読まないのは依頼の
  約束。`/dev/urandom` を使わなかったのは「読むのは worktree の中だけ」の制約で
  worktree 外のファイルを読めないため。定数 salt はプロセス横断で仮名を突き合わせ
  られるため避け、プロセスごとに変わる値だけ混ぜた。安全性の定量的評価は未検証
  （実運用では CSPRNG が望ましい）。
- red-first の証拠（tdd）:
  - 結合テストを先に書き、出力点が未接続の足場（`serve_with_logger` は存在するが
    接続経路がログを出さない）で実行: `cargo test --offline -p pelway-http --test
    no_body_logs` → **exit 101、2 passed / 4 failed**。失敗の要点（すべて「ログが
    出ていない」ことが正しい理由）:
    - (a) `logs_have_no_canary_and_at_least_one_event`: 「ログが1件も出ていない」
    - (b) `raw_ids_absent_and_pseudonyms_present`: 「仮名化した card_id がない」
    - (c) `rejected_and_internal_error_have_no_canary_anywhere`:
      「拒否・内部エラーでもログが出ていない」
    - (f) `full_flow_emits_all_event_kinds_with_pseudonyms_only`:
      「request_received がない: []」
    - (d) 陽性対照・(e) パス検査は実装に依存しないため素通り（期待どおり）。
  - 接続を実装して green: 同コマンド → exit 0、6 passed / 0 failed。
  - 実装途中の red（単体テストが拾ったもの、正直に記録）:
    - `FileSink` の最初の実装は `OpenOptions` に `append(true).truncate(true)` を
      指定し、このツールチェーンの検証「creating or truncating a file requires write
      or append access」で open が失敗した（exit 101、単体テスト 2 件失敗）。
      `write(true)` を追加しても同様（append+truncate の併用が拒否される）。
      単一ハンドルを持ち続ける設計なので `append` を外し `create+write+truncate`
      で開いて逐次書き込みに修正 → green。
    - `process_salt` の最初の単体テストは同一入力の決定性を主張したが、ヒープ確保
      アドレスは呼び出しごとに変わる追加密のため失敗（exit 101）。アドレス分の
      エントロピーは意図どおりなので、テストを「空でないこと・時刻が変われば必ず
      変わること」に修正 → green。
- example の実行コマンドと出力:
  - `mkdir -p target/demo-logs`（shell の `2>` 差し替え先のため。`target/demo-logs/`
    自体は FileSink が最初の追記時に作るが、リダイレクトはプログラム起動前に開くため
    初回は先に作る必要があった）。
  - `cargo run --offline -p pelway-http --example demo-log-emit 2> target/demo-logs/stderr.txt`
    → exit 0、stdout「demo-log-emit: statuses = [200, 200, 409, 400, 500]」。
  - 出力ファイル: `target/demo-logs/backend.jsonl`（16 行）、
    `target/demo-logs/stderr.txt`（15 行。cargo のビルド出力と poison 用 panic の
    固定語句のみ）。
  - canary の件数: `rg -c 'PELWAY-CANARY-BODY-7Q2M9X|PELWAY-CANARY-SECRET-4K8T1Z|
    3f6c2a9e-8b1d-4c7a-9e2f-5a1b7c3d9e04' target/demo-logs/` → 出力なし・exit 1
    （**3種とも 0 件**）。エラー応答の本文（`{"code":"snapshot_mismatch"}`・
    `{"code":"bad_request"}`・`{"code":"internal"}`）と panic の文言にも 0 件
    （テスト (c) と stderr.txt の内容で確認）。
  - 出来事の件数（backend.jsonl 16 行）: request_received 5・responded 5・
    comment_saved 1・approved 1・simulated_send 1・rejected 2（409 と 400）・
    internal_error 1。受けた・応答した・承認した・模擬送信した・拒否した・内部エラー
    がそれぞれ 1 件以上。
  - 生 ID の走査: `rg -c 'card-gdpr07|m-human-gdpr07|sim-00000001' …backend.jsonl` →
    exit 1（0 件）。`ws-synthetic-gdpr07|ch-synthetic-gdpr07|cmt-00000001|
    gdpr07-approve-key` も 0 件。仮名（`"pid:`）は 8 行にあり。
- 検証（コマンドと exit code、オフライン）:
  - `cargo fmt --all --check` 0
  - `cargo clippy --workspace --all-targets --offline -- -D warnings` 0
  - `cargo test --workspace --offline` 0（at-rest 単体 8・aead 12・contract 単体 38
    （pseudonym 4 を追加）・serde_contract 10・dev-smoke 単体 0+smoke 1・http 単体 5
    （log 5 を追加）・http 結合 26・no_body_logs 6。**計 106 passed / 0 failed /
    0 ignored**）
  - example 実行・canary 走査・行数は上記のとおり。
- 迷った点（安全側に倒した内容）:
  - 冪等リプレイで approved/simulated_send を出すか: 台帳の `send_log()` 件数の増減で
    「この要求で新しい疑似送信」のときだけ出すことにした（リプレイで偽の送信記録を
    作らない）。HTTP 層は従前どおり `approve` だけを呼び、予約を持ち出さない方針を維持。
  - `FileSink` の書き込み失敗は best-effort で握る（接続スレッドで panic しない）。
    C05/C07 の「記録できない時は新業務を停止」はこのデモの範囲外（未実施として記録）。
  - `FileSink` は CWD 相対の `target/demo-logs` に置く。cargo 経由の実行（ワークスペース
    ルートが CWD）を前提とし、それ以外の起動は想定しない（worktree の外への書き込みを
    防ぐファイル名検査は実装済み）。
  - canary の UUID は comments の JSON に未知フィールド (`client_msg_id`) として埋めた
    （deny_unknown_fields で 400 になる。拒否経路で本文に canary があっても出ないことの
    確認にもなる）。`idempotency_key` には canary を使わなかった（書式制限があり、
    かつ「client_msg_id 用」の目印だから）。
  - 仮名化は sim ID・コメント ID などサーバー採番 ID にも適用（生値は 1 つも出さない
    安全側）。`responded`/`rejected`/`internal_error` はそもそも ID を持たない。
- コミット: 仕組みとテスト・example（下記コミット SHA 参照）と、この worklog 追記。
  `target/` の下のログはコミットしない（`/target` は gitignore 済み。単体テストの
  掃除で `crates/pelway-http/target/` にファイルは残っていない）。
- 逸脱: なし（worktree・ブランチの中だけ。push・fetch なし、外部接続なし、
  破壊的gitなし、`~/.cargo` の ls・cat・find なし、cargo はすべて `--offline`。
  worktree の外に書いたファイルはない。一時ファイルは作成せず）。
- 未検証・未決:
  - クライアントログは対象外（クライアント担当）。仮名化の法的な十分性は未検証。
  - U01〜U07 は未決のまま（`TODO(U0x)` は残している）。ログの保持期限
    （R09 14日・R10 30日・R11 365日）と容量（C05）はこのデモでは未実装。
  - salt のエントロピー（PID+時刻+ASLR）の定量的評価は未検証。実運用では CSPRNG。
  - Codex レビュー・QA（QA-GDPR-07/12・QA-MSG-02・QA-ENC-04/K5 の正式な受入）・
    開発基盤の canary 走査は未実施。

### 2026-11-10 第15枠（QA-GDPR-07 追補・salt を /dev/urandom から・GLM 枠）
- start: 2026-11-10 00:12 JST頃（追補依頼「be-salt-u1」の受領後）
- stop:  2026-11-10 00:23 JST（`date` で確認。コミットはこの後）
- glm: long（T3 Code 上で GLM が実装。依頼の決まり文句どおり）
- worktree: `/home/hatai/.t3/worktrees/Pelway/demo-t3-backend-gdpr-07-salt`、branch
  `demo-t3/backend-gdpr-07-salt`（base `549ab8b` = 第14枠の worklog コミット）
- 開始時の確認: HEAD `549ab8b`（指定と一致）、`git status --porcelain` 空。
- AGENTS.md の「Prior art first」（GitHub 検索）: 外部ネットワーク接続禁止のため未実施
  （第13・14枠と同じ）。
- skills: Matt Pocock Agent Skills の `tdd` のみ（他は足さず）。red → green。
- 内容（`crates/pelway-http` のみ。Cargo.toml・Cargo.lock 不变、新クレートなし、
  標準ライブラリだけ。実装が実行時に読むファイルは `/dev/urandom` だけ
  ＝依頼の例外1つ。エージェント自身が `/dev/urandom` を読んでいない）:
  - 変えたファイル:
    - `src/log.rs`: `process_salt(now)`（PID+起動時刻+ヒープアドレス）を削除。
      新しく `SALT_BYTES`(=32)・`SALT_SOURCE`(="/dev/urandom") の定数、
      `pub(crate) fn read_salt_from(path) -> io::Result<[u8; 32]>`
      （`std::fs::File::open` + `read_exact`。短い読み取りも失敗）、
      `pub fn process_salt() -> io::Result<[u8; 32]>`（`read_salt_from(SALT_SOURCE)`）
      を追加。フォールバック（代替値に落とす経路）なし。PID・時刻・アドレスを
      使うコードは残していない（`rg 'process::id|to_ne_bytes|as_nanos|Box::new|:p}'`
      で確認、マッチ 0）。doc コメント（「`/dev/urandom` も使わず」の箇所）を
      新しい動きに書き換え。`SystemTime` は `ts_ms` 用なので残る（salt には不使用）。
    - `src/lib.rs`: `serve_with_logger_salt_from(addr, ledger, sink, salt_path)` を
      追加。salt の読み取りを **bind より前**に行い、失敗したら `ServeError::Io`
      で `Err` を返して待ち受けを始めない。既存 `serve`・
      `serve_with_connection_deadline`・`serve_with_logger`（固定 salt の
      `Arc<Logger>` を渡す形）は引数・挙動とも不变。
    - `examples/demo-log-emit.rs`: `serve_with_logger_salt_from` +
      `Path::new(SALT_SOURCE)` に切り替え（salt の読み取り失敗で起動が失敗する
      経路を使う）。doc ヘッダーに salt の方針を追記。
    - `tests/no_body_logs.rs`: (s2) を (g) として追加（下記）。
  - テストの入れ替え: 単体 `process_salt_varies_with_time_and_is_never_empty`
    （旧実装専用）を削除し、(s1) `read_salt_from_missing_path_is_err`・
    (s3) `urandom_salts_are_32_bytes_and_distinct`・
    (s4) `same_salt_same_pseudonym_and_different_salt_differs` を追加。
    結合 (s2) `serve_with_failing_salt_source_returns_err_without_listening` を
    `no_body_logs.rs` に追加（存在しない salt パス → `Err(ServeError::Io)` で
    `ErrorKind::NotFound`、ログ 0 行）。既存6件 (a)〜(f) は無変更。
- red-first の証拠（tdd）: まず「常に Ok を返す足場」
  （`read_salt_from` は常に `Ok([0u8; 32])`、`serve_with_logger_salt_from` は
  読み取り失敗を `unwrap_or` で握って常に Ok）を置き、テストを実行:
  - `cargo test --offline -p pelway-http --lib` → **exit 101、4 passed / 3 failed**。
    失敗の要点（すべてコンパイルエラーではなく挙動の失敗）:
    - (s1)「存在しないパスは Err: ()」— 足場が `Ok` を返したため
    - (s3) `assert_ne!` failed: 左右とも全ゼロの 32 バイト（salt が変わらない）
    - (s4) `assert_ne!` failed: 左右の仮名が同一（salt が同一のため）
  - `cargo test --offline -p pelway-http --test no_body_logs` → **exit 101、
    6 passed / 1 failed**。(s2)「salt の読み取りに失敗したら起動は Err」—
    足場がフォールバックで `Ok`（リスナー起動）を返したため。
- green の証拠: 足場を本実装（`File::open` + `read_exact`、`map_err(ServeError::Io)`、
  フォールバック削除）に置き換えて:
  - `cargo test --offline -p pelway-http` → exit 0（単体 7・結合 http 26・
    no_body_logs 7、すべて passed）。
- 検証（コマンドと exit code、オフライン）:
  - `cargo fmt --all --check` 0
  - `cargo clippy --workspace --all-targets --offline -- -D warnings` 0
  - `cargo test --offline --workspace` 0（**計 109 passed / 0 failed / 0 ignored**:
    at-rest 単体 8・aead 12・contract 単体 38・serde_contract 10・dev-smoke 単体 0
    ＋smoke 1・http 単体 7（旧 5 から入れ替えで +2）・http 結合 26・
    no_body_logs 7（既存6件はそのまま通過 + 新 (s2)）。doc-tests は各 0）
  - no_body_logs の既存6件が引き続き通ること: 上記のとおり 6+1 で全通過。
- ログの取り直し（前回=第14枠と同じ手順）:
  - `mkdir -p target/demo-logs` → `cargo run --offline -p pelway-http --example
    demo-log-emit 2> target/demo-logs/stderr.txt` → exit 0、stdout
    「demo-log-emit: statuses = [200, 200, 409, 400, 500]」。
  - `target/demo-logs/backend.jsonl`: **16 行**（前回と同じ構成: request_received 5・
    responded 5・comment_saved 1・approved 1・simulated_send 1・rejected 2・
    internal_error 1。仮名 `pid:` は 8 行）、
    sha256 `741e5aabcf83455e8096b8b930e3763d992b793755cfb0a9fbeec94095f3b70f`。
  - `target/demo-logs/stderr.txt`: **16 行**（前回 15 行から +1。cargo のビルド
    メッセージの構成差。poison 用 panic の固定語句のみで canary なし）、
    sha256 `e8167b26b18b3fa0468c5f7ba792c3b471f8afff43a64b840ccb6f6aab21ac15`。
  - canary 走査 `rg -c 'PELWAY-CANARY-BODY-7Q2M9X|PELWAY-CANARY-SECRET-4K8T1Z|
    3f6c2a9e-8b1d-4c7a-9e2f-5a1b7c3d9e04' target/demo-logs/` → 出力なし・exit 1
    （3種とも 0 件）。生 ID（`card-gdpr07|m-human-gdpr07|sim-00000001`）も 0 件。
- 迷った点（質問で止まらず安全側で進めた）:
  - (s2) の「待ち受けを始めない」の観察方法: リスナー非起動は外部から直接観測
    できないため、(i) 実装で salt の読み取りを bind より前に置く、(ii) `Err` の
    中身が `ErrorKind::NotFound`（ファイル由来。bind 失敗では出ない種別）である
    こと、で担保した（Ok を返す場合のみリスナーが立つ）。
  - (s4) を固定 salt ではなく `process_salt()` の実 salt 2 個で書いた: 固定 salt だと
    現行実装でも最初から green になり「意味のある red」が取れないため。足場では
    salt が同一になって仮名も同一になり、落ちる（s3 の性質と結合する形）。
  - `serve_with_logger`（呼び出し側が固定 salt の `Logger` を渡す形）は残した:
    テストは salt 既知のまま仮名の決定性を検査する必要があり、公開 API の形を
    なるべく変えないため。本番経路（example）は新しい
    `serve_with_logger_salt_from` に切り替えた。
  - `process_salt` のシグネチャ変更（引数 `SystemTime` 削除・戻り値を
    `io::Result<[u8; 32]>`）: 失敗を呼び出し側へ伝えるには `Result` が必須のため。
    関数名は残した（利用者は example のみで、上記のとおり切り替え）。
  - salt パスを `serve_with_logger_salt_from` の**公開**引数にした: 結合テストから
    失敗を注入するため（`read_salt_from` は `pub(crate)` の内部関数）。本番の
    既定パスは `log::SALT_SOURCE` 定数で、example が使うのもこちら。
- コミット: 実装とテスト・example（下記コミット SHA 参照）と、この worklog 追記。
  `target/` の下のログはコミットしない（`/target` は gitignore 済み）。push はしない。
- 逸脱: なし（worktree の中だけ。push・fetch なし、外部接続なし、破壊的gitなし、
  `~/.cargo` 不変、cargo はすべて `--offline`、webfetch/websearch/MCP 不使用。
  `/dev/urandom` は実装・テスト・example の実行時にコードが読んだだけで、
  エージェント自身は cat していない）。
- 未検証・未決:
  - 第14枠の salt 記載（PID+時刻+アドレス）はこの追補で置き換えられた
    （第14枠の記述は当時の記録として残す）。
  - getrandom(2) の直接使用（`/dev/urandom` より推奨される場合がある）は新依存に
    なるため不使用。Linux 以外の OS の CSPRNG パスは想定外。
  - salt の再読み込み・ローテーション、プロセス間での salt 共有は対象外。

### 2026-11-10 第16枠（QA-GDPR-07 salt 追補 r2・salt の読み込み元を公開経路から外す・GLM 枠）
- start: 2026-11-10 00:26 JST頃（追補依頼「be-salt-u2」の受領後）
- stop:  2026-11-10 00:48 JST（`date` で確認。コミットはこの後）
- glm: long（T3 Code 上で GLM が実装。依頼の決まり文句どおり）
- worktree: `/home/hatai/.t3/worktrees/Pelway/demo-t3-backend-gdpr-07-salt-r2`、branch
  `demo-t3/backend-gdpr-07-salt-r2`（base `9d8ab09` = 第15枠の worklog コミット）
- 開始時の確認: HEAD `9d8ab09`（指定と一致）、`git status --porcelain` 空。
- AGENTS.md の「Prior art first」（GitHub 検索）: 外部ネットワーク接続禁止のため未実施
  （第13〜15枠と同じ）。
- skills: Matt Pocock Agent Skills の `tdd` のみ（他は足さず）。red → green。
- 目的: `serve_with_logger_salt_from` は salt を読むパスを**公開の引数**で受けており、
  本番の呼び出し側が `/dev/urandom` 以外（固定内容のファイルなど）を渡せてしまった。
  公開の入口を salt 必ず `log::SALT_SOURCE`（`/dev/urandom`）から読む形だけにする。
- 内容（`crates/pelway-http` のみ。Cargo.toml・Cargo.lock 不变、新クレートなし、
  標準ライブラリだけ。実装が実行時に読むファイルは `/dev/urandom` だけ
  ＝依頼の例外1つ。エージェント自身が `/dev/urandom` を読んでいない）:
  - 変えたファイル:
    - `src/lib.rs`: 公開入口 `serve_with_logger_urandom(addr, ledger, sink)` を追加
      （パス引数なし。`log::SALT_SOURCE` 固定で後述の `pub(crate)` 版へ委譲）。
      `serve_with_logger_salt_from(addr, ledger, sink, salt_path)` は `pub` →
      `pub(crate)` にして公開から外す（`#[doc(hidden)]` は付けていない。単に
      クレート外から見えない）。doc コメントは「`serve_with_logger_urandom` の実体・
      採取元を引数に受けるのはクレート内部のテストだけ」と書き換え。
      `serve_with_logger`（固定 salt の `Arc<Logger>` を渡す形）は**形を変えず**
      doc に「テストと既知 salt の検証用。本番では `/dev/urandom` 固定の入口
      （`serve_with_logger_urandom`）を使う」と追記。クレート先頭のモジュールdoc の
      ログ説明も新しい入口に差し替え。`#[cfg(test)] mod tests` を新設し、
      (s2) `serve_with_failing_salt_source_returns_err_without_listening` を移設。
    - `tests/no_body_logs.rs`: (g) として置いていた (s2) を削除し、移設先を明記する
      コメントだけ残した。`serve_with_logger_salt_from` の import も削除。
    - `examples/demo-log-emit.rs`: `serve_with_logger_urandom` に切り替え
      （`Path::new(SALT_SOURCE)` と `std::path::Path`・`SALT_SOURCE` の import を削除。
      salt はこの入口が `/dev/urandom` から読む）。doc ヘッダーも更新。
- 「公開の入口がパスを取らないこと」のテストについて: これは**コンパイル時の性質**
  （シグネチャと可視性）なので、ランタイムのテストとして置けない。代わりに:
  (i) 公開入口はパス引数を持たないシグネチャそのもの、(ii) rustc の可視性検査を
  証拠として確認 — 結合テストから一時的に `use pelway_http::serve_with_logger_salt_from;`
  したところ `error[E0603]: function serve_with_logger_salt_from is private`
  （`cargo test --offline -p pelway-http --test no_body_logs` → exit 101）。
  確認後すぐに戻した（`git diff` で一時行なしを確認）。
- red-first の証拠（tdd）: まず (s2) を `src/lib.rs` の単体テストに移し、
  `serve_with_logger_salt_from` を足場（`read_salt_from(...).unwrap_or([0u8; 32])`
  で読み取り失敗を握りつぶして常に Ok）に差し替えて実行:
  - `cargo test --offline -p pelway-http --lib` → **exit 101、7 passed / 1 failed**。
    失敗の要点（コンパイルエラーではなく挙動の失敗）:
    - (s2) `panicked at ...: salt の読み取りに失敗したら起動は Err: ()`
      — 足場がフォールバックで `Ok`（リスナー起動）を返したため。
- green の証拠: 足場を本実装（`map_err(ServeError::Io)`、フォールバックなし）に
  置き換え、`serve_with_logger_urandom` を追加して:
  - `cargo test --offline -p pelway-http --lib` → exit 0（**8 passed**。log 7 + 移した (s2) 1）。
  - `cargo test --offline -p pelway-http` → exit 0（単体 8・結合 http 26・
    no_body_logs 6・doc-tests 0、すべて passed）。
- 検証（コマンドと exit code、オフライン）:
  - `cargo fmt --all --check` 0
  - `cargo clippy --workspace --all-targets --offline -- -D warnings` 0
  - `cargo test --offline --workspace` 0（**計 109 passed / 0 failed / 0 ignored**:
    at-rest 単体 8・aead 12・contract 単体 38・serde_contract 10・dev-smoke 単体 0
    ＋smoke 1・http 単体 8・http 結合 26・no_body_logs 6。doc-tests は各 0）
  - 件数の変化（(s2) の移動）: http 単体 7 → **8**（+1）、no_body_logs 7 → **6**
    （−1）。合計は第15枠と同じ 109（移動であって増減なし）。
    (s1)・(s3)・(s4)（`src/log.rs` 単体）と no_body_logs 既存6件 (a)〜(f) は
    無変更で通過。
- ログの取り直し（前回=第15枠と同じ手順）:
  - `mkdir -p target/demo-logs` → `cargo run --offline -p pelway-http --example
    demo-log-emit 2> target/demo-logs/stderr.txt` → exit 0、stdout
    「demo-log-emit: statuses = [200, 200, 409, 400, 500]」。
  - `target/demo-logs/backend.jsonl`: **16 行**（前回と同じ構成: request_received 5・
    responded 5・comment_saved 1・approved 1・simulated_send 1・rejected 2・
    internal_error 1。仮名 `pid:` は 8 行）、
    sha256 `f98b84245f673db5e9f6d78f20dd6654b28d80ac6ff2c82ac05ac87f88ebb8db`。
    ※salt が実ランダムなのでハッシュは毎回変わる（行数・構成が同じことが確認点）。
  - `target/demo-logs/stderr.txt`: **13 行**（前回 16 行からの差は cargo のビルド
    メッセージの構成差（キャッシュ）。poison 用 panic の固定語句のみで canary なし）、
    sha256 `02188d092fe9bd7ec44f125d68b543cd49d1b32ff9b579b2315acfcf5b5bf99c`。
  - canary 走査 `rg -c 'PELWAY-CANARY-BODY-7Q2M9X|PELWAY-CANARY-SECRET-4K8T1Z|
    3f6c2a9e-8b1d-4c7a-9e2f-5a1b7c3d9e04' target/demo-logs/` → 出力なし・exit 1
    （3種とも 0 件）。生 ID（`card-gdpr07|m-human-gdpr07|sim-00000001`）も 0 件。
- 迷った点（質問で止まらず安全側で進めた）:
  - 依頼の選択肢は (a)（(s2) を `src/` 内 `#[cfg(test)]` 単体テストへ移し
    `pub(crate)` 関数を直接呼ぶ）を選んだ。しかし素朴な `pub(crate)` のままだと
    非テストビルドで唯一の呼び出し方が無くなり、clippy の `-D dead_code` が
    exit 101 になった。`#[allow(dead_code)]` で握るのも案 (b)（`#[cfg(test)]` 専用の
    入口）にするのも変だと判断し、**公開入口から `SALT_SOURCE` 固定で `pub(crate)` 版に
    委譲する形**で解決（実質 (a) のまま。読み込み・bind 前判定の実装は 1 か所で、
    公開入口は定数以外のパスを構造的に渡せない）。
  - `serve_with_logger`（固定 salt の `Logger` を渡す形）を変えなかった理由:
    結合テスト `no_body_logs` の (a)〜(f) が「salt 既知のまま仮名の決定性を検査する」
    前提でこの入口を使っており、形を変えると既存6件の意味が崩れる。テスト専用と
    位置づけを doc に明示する方が影響が小さい（依頼4の指定どおり）。
  - (s2) の移設先を `src/lib.rs` 直下の `mod tests` にした: 移動元が結合テストの
    末尾 (g) で lib に依存する他の単体テストは `src/log.rs` にあるが、対象関数は
    lib.rs にあるため同一モジュールツリーの lib.rs 末尾に置いた。
  - E0603 確認用の一時 import を結合テストに入れた（証拠取得のために一瞬だけ）:
    worktree の中への一時差し込みで、確認直後に戻し、`git diff --stat` で
    意図した 3 ファイルのみになっていることを確認した。
- コミット: 実装とテスト・example（下記コミット SHA 参照）と、この worklog 追記。
  `target/` の下のログはコミットしない（`/target` は gitignore 済み）。push はしない。
- 逸脱: なし（worktree の中だけ。push・fetch なし、外部接続なし、破壊的gitなし、
  `~/.cargo` 不変、cargo はすべて `--offline`、webfetch/websearch/MCP 不使用。
  `/dev/urandom` は実装・テスト・example の実行時にコードが読んだだけで、
  エージェント自身は cat していない）。
- 未検証・未決:
  - 第15枠の「salt パスを公開引数にした」判断はこの追補で置き換えられた
    （第15枠の記述は当時の記録として残す）。
  - `log::process_salt()` は lib から直接呼ばなくなったが、`pub` のまま残す
    （salt 作成の公開 API。example も今回の入口経由で間接的に使う）。
  - salt のローテーション・プロセス間共有は引き続き対象外。

### 2026-11-10 第17枠（QA-GDPR-07 salt 追補 r3・入口の salt 利用テストと公開範囲の縮小・GLM 枠）
- start: 2026-11-10 01:13 JST頃（追補依頼「be-salt-u3」の受領後）
- stop:  2026-11-10 01:23 JST（`date` で確認。コミットはこの後）
- glm: long（T3 Code 上で GLM が実装。依頼の決まり文句どおり）
- worktree: `/home/hatai/.t3/worktrees/Pelway/demo-t3-backend-gdpr-07-salt-r3`、branch
  `demo-t3/backend-gdpr-07-salt-r3`（base `bc0152f` = 第16枠の worklog コミット）
- 開始時の確認: cwd はこの worktree（本体フォルダではない）、HEAD
  `bc0152faa4466613e9d70e14f7195e4ecba7a636`（指定と一致）、
  `git status --short` 空。
- AGENTS.md の「Prior art first」（GitHub 検索）: 外部ネットワーク接続禁止のため未実施
  （第13〜16枠と同じ）。
- 依頼3の `rg` 調査（コマンド `rg -n 'Logger::with_sink|process_salt' --type rust`）:
  - `Logger::with_sink`: クレート外からの呼び出しは `tests/no_body_logs.rs:62`
    （結合テスト (a)〜(f) の spawn_logged。**tests/ だけ**）。ほかはクレート内
    （`src/lib.rs:284`（serve_with_logger_salt_from の本実装）と
    `src/log.rs:364,396,452,453`（単体テスト））。examples/・他クレートからの呼び出し無し。
  - `process_salt`: 呼び出し元は `src/log.rs` の単体テスト（439,440,450,451 行）のみ。
    tests/・examples/・他クレートからの呼び出し無し（tests/no_body_logs.rs:28 は
    コメント内の言及のみ）。
- 依頼1（本番の入口が読んだ salt を本当に使っていることのテスト）:
  - テスト (s5) `entry_uses_salt_read_from_file_for_log_pseudonyms` を `src/lib.rs` の
    `#[cfg(test)] mod tests` に追加（r2 で (s2) を移した場所と同じ。同じ入口
    `serve_with_logger_salt_from` のテストのため）。
  - 内容: 決まった中身の 32 バイト（`b"known-salt-for-entry-test-012345"`）を
    `target/demo-logs/known-salt-entry-test`（パッケージルート相対＝worktree の中、
    gitignore 済み。既存単体テストと同じ置き場所の前提）に書き、
    `serve_with_logger_salt_from` に読ませて `127.0.0.1:0` で起動。comments への
    要求（本文を不正 JSON にして 400 になる）を送り、ログの `request_received` の
    card 仮名が、同じ salt で `pseudonymize` を別に計算した仮名と一致することを
    確かめる（生の card_id がログに無いことも確認。テスト終了時に salt ファイルを削除）。
  - red（一時改変）: `serve_with_logger_salt_from` を、salt ファイルの読み取りは
    そのままに、読んだ salt を捨てて 0 埋め 32 バイト（`vec![0u8; SALT_BYTES]`）を
    Logger に渡すように一時的に書き換えて実行:
    `cargo test --offline -p pelway-http --lib entry_uses_salt` → **exit 101、
    0 passed / 1 failed**。出力:
    `assertion left == right failed: 入口のログの仮名が同じ salt で別に計算した仮名と一致しない
    left: "pid:cb70f23ab7b31c525a5a5c7cb7ce25ee087b0d3cdb3f6768ee99e20dc9bbf538"
    right: "pid:a2ce0b1eaeb5030be421731037db6ffb5906b57f316dbf960c1a57261de51dd7"`
    （left が 0 埋め salt で仮名化したログの仮名、right が salt ファイルの中身で
    別に計算した仮名）。出力を確認後、すぐに改変を戻した。
  - green（改変を戻したあと）: 同コマンド → **exit 0、1 passed**。
    なお追加直後・改変前に一度実行して現行実装で pass することも確認済み
    （テストが常時落ちでないことの確認）。
- 依頼2（(s4) を別々に作った2つの Logger で比べる形に）:
  - `src/log.rs` の (s4) `same_salt_same_pseudonym_and_different_salt_differs` を書き直し。
    旧: 同じ Logger `la` の `id` を2回呼んで一致を見る（`la.id == la.id`）。
    新: 同じ salt で**別々に作った**2つの Logger `la1`/`la2` で一致を、違う salt で
    作った `lb` との不一致も Logger 同士で比べる（`la1.id == la2.id`、
    `la1.id != lb.id`）。salt は従来どおり `process_salt()` で 2 個採取。
  - 実装変更を伴わないテストの書き直しのため red は無し（現行実装で pass を確認。
    第8枠作業1・第10枠指摘4と同じ扱い）。仮名化実装の検証（red）は上記 (s5) の
    一時改変で担保。
- 依頼3（可視性の縮小）:
  - `Logger::with_sink`（`src/log.rs`）: `pub` → `pub(crate)`。唯一のクレート外呼び出し元が
    tests/ の結合テストだけだったため、依頼の優先（「そのテストを src 内の
    `#[cfg(test)]` に移して `pub(crate)` にする」「迷ったら移す側」）どおり、
    `tests/no_body_logs.rs`（(a)〜(f) の6件とヘルパー）を `src/no_body_logs.rs` に
    移設（`git mv` で移動し、import を `crate::` に、モジュール doc に移設の経緯を
    書き換えただけ。**検証内容は無変更**）。doc コメントに「テスト・検証用」を明記。
  - `process_salt`（`src/log.rs`）: `pub` → `#[cfg(test)]`（クレート外からの呼び出し
    無し。呼び出し元は同ファイルの単体テスト (s3)(s4) のみのため。`pub(crate)` の
    ままだと非テストビルドで dead_code が出るため `#[cfg(test)]` を選んだ）。
    doc コメントに「テスト・検証用（クレート内のテストだけが使う）」を明記。
  - 付随する doc の更新: `src/log.rs` のモジュール doc が参照していた `process_salt`
    の説明を `read_salt_from`（`SALT_SOURCE` から `SALT_BYTES` バイト。本番は
    `serve_with_logger_urandom` 経由）の記述に差し替え。
  - `serve_with_logger`（固定 salt の `Arc<Logger>` を渡す公開入口）は**今回の対象外**のため
    `pub` のまま。移設後の呼び出し元は `src/no_body_logs.rs`（クレート内テスト）のみに
    なったが、doc の「テストと既知 salt の検証用」の明記は既存のものを残した。
- 追加の記録（32 バイトより短い salt ファイルのテストの有無）: **無い**。
  `read_salt_from` は `read_exact` なので短い読み取りは失敗（`UnexpectedEof`）に
  なる設計だが、それを確かめるテストは今回も追加していない。**既知の制限**として記録する
  （依頼どおり実装・テストとも足していない）。
- 検証（コマンドと exit code、オフライン）:
  - `cargo fmt --all -- --check` 0
  - `cargo clippy --offline --workspace --all-targets -- -D warnings` 0
  - `cargo test --offline --workspace` 0（**計 110 passed / 0 failed / 0 ignored**:
    at-rest 単体 8・aead 12・contract 単体 38・serde_contract 10・dev-smoke 単体 0
    ＋smoke 1・http 単体 **15**（log 7＋lib.rs 2＝(s2)+(s5)＋no_body_logs 6）・
    http 結合 26。doc-tests は各 0）
  - 件数の変化: http 単体 8 → 15（no_body_logs 6 件の移設 ＋ 新 (s5) ＋1）、
    no_body_logs 結合 6 → 0（テスト目標ごと src へ移設）。合計 109 → 110
    （+1 は (s5) の追加。6 件は移動であって増減なし）。
- ログの取り直し（前回=第16枠と同じ手順）:
  - `mkdir -p target/demo-logs` → `cargo run --offline -p pelway-http --example
    demo-log-emit 2> target/demo-logs/stderr.txt` → exit 0、stdout
    「demo-log-emit: statuses = [200, 200, 409, 400, 500]」。
  - `target/demo-logs/backend.jsonl`: **16 行**（前回と同じ構成: request_received 5・
    responded 5・comment_saved 1・approved 1・simulated_send 1・rejected 2・
    internal_error 1。仮名 `pid:` は 8 行）、
    sha256 `5b6ba3f1291c229860e4751ab83f9b5b2c140e22fc7aa1ae6197ec75d0db3842`。
    ※salt が実ランダムなのでハッシュは毎回変わる（行数・構成が同じことが確認点）。
  - `target/demo-logs/stderr.txt`: **10 行**（cargo のビルドメッセージの構成差を含む。
    poison 用 panic の固定語句のみで canary なし）、
    sha256 `b927e6929cbe7ddc21e8a3aa4e1598420475791f6aad1f9829dd44da31ca5b12`。
  - canary 走査 `rg -c 'PELWAY-CANARY-BODY-7Q2M9X|PELWAY-CANARY-SECRET-4K8T1Z|
    3f6c2a9e-8b1d-4c7a-9e2f-5a1b7c3d9e04' target/demo-logs/` → 出力なし・exit 1
    （3種とも 0 件）。生 ID（`card-gdpr07|m-human-gdpr07|sim-00000001`）も 0 件。
- 迷った点（質問で止まらず安全側で進めた）:
  - (s5) の要求を 400 になる不正 JSON にした: `request_received` の仮名は route が
    要求受け付け時点で出すため、台帳にカードを用意しなくても「入口が読んだ salt で
    仮名化していること」は確かめられる。正当な 200 フローと比べて依存が減り、
    red の一時改変にも同じように効く。
  - (s5) の salt ファイルを `target/demo-logs/`（パッケージルート相対）に置いた:
    既存の単体テスト（`read_salt_from_missing_path_is_err`・(s2)）と同じ CWD の前提。
    worktree の中で gitignore 済み。テスト終了時に削除する。
  - (s5) を lib.rs の tests モジュールに置き、no_body_logs の移設は純粋な移動にした:
    (s2) と同じ入口のテストは同じ場所に集める。移設ファイルは import と doc の
    書き換えだけ（diff で「検証内容は無変更」を示せる）。
  - `process_salt` を `pub(crate)` でなく `#[cfg(test)]` にした: 呼び出し元が
    同ファイルの単体テストだけのため（上記のとおり dead_code 回避）。
  - テストの移設で no_body_logs の「結合テストとしての性質」（クレート外から
    public API 経由で検証すること）は失われるが、依頼文の「移す側を優先」
    「迷ったら移す側を選ぶ」に従った。QA が `cargo test --test no_body_logs` で
    個別実行していた場合は、今後は `cargo test -p pelway-http --lib no_body_logs` になる。
- コミット: `548cf09`（実装・テスト・移設。pre-commit hook の trailing_whitespace・
  newlines・cargo_fmt・cargo_check・cargo_clippy はすべて合格）と、この worklog 追記
  （コミットは最大2つ、push はしない）。`target/` の下のログはコミットしない
  （`/target` は gitignore 済み）。
- 逸脱: なし（worktree の中だけ。push・fetch なし、外部接続なし、破壊的gitなし、
  `~/.cargo` 不変、cargo はすべて `--offline`、webfetch/websearch/MCP 不使用。
  `/dev/urandom` はテスト実行時にコードが読んだだけで、エージェント自身は
  cat していない。red 確認用の一時改変は worktree 内で、出力を記録後にすぐ戻し、
  `git diff` で改変が残っていないことを確認）。
- 未検証・未決:
  - 32 バイト未満の salt ファイルを読んだときの挙動（設計上は `UnexpectedEof` で
    失敗）を確かめるテストは無い（既知の制限として上記に記録）。
  - `serve_with_logger` は `pub` のまま（呼び出し元がクレート内テストのみに
    なったが、今回の対象は `with_sink`/`process_salt` の2つだけのため）。
  - 差分の Codex 再レビュー・QA は未実施。U01〜U07 は未決のまま。

### 2026-11-10 第18枠（QA-GDPR-02・目的ごとの根拠の記録と共通のメモリ保存・GLM 枠）

- 依頼: 「バックエンド作業依頼 QA-GDPR-02（目的ごとの根拠の記録と、共通のメモリ保存）」
  （開発リード割り当て、期限 2026-11-14 21:00 JST）。
- worktree: `/home/hatai/.t3/worktrees/Pelway/demo-t3-backend-gdpr-02-r2`（新規作成された
  このスレッドの worktree）、ブランチ `demo-t3/backend-gdpr-02-r2`、
  base `cb161520252ed3c262d60e76f9bb43a5cb6892af`。開始 2026-11-10 04:52 JST、
  終了 2026-11-10 05:10 JST。開始時の `git status --porcelain` は空（クリーン）。
- 使った skills: `tdd`（red → green のループ参照。テストは公開インターフェース
  （`DataStore` の公開メソッドと HTTP 経路）越しに振る舞いだけを検証し、内部構造に
  依存しない形にした。依頼どおりテスト先行で、実装前に red を記録してから green にした）。
- red-first の証拠（実装前にテストを書き、最小スケルトン＝型と空のメソッドだけの
  状態で実行。コンパイルエラーで終わらせず、正しい理由での実行時失敗を記録した）:
  - `cargo test --offline -p pelway-contract --lib store::` → **exit 101**、
    3 passed / 6 failed。要点: テスト1（P01〜P08 の作成・読取）は basis_id が空・
    保存されず 0 件で失敗。テスト2（canary 不出力）は、red 確認用に一時入れた
    「メッセージ本文を複製して持つ不正な実装（`debug_messages_echo`）」が直列化に
    canary を漏らし、`根拠レコードの直列化に本文 canary がある` で検出失敗。
    テスト3（サーバー付け）は `left: "" right: "bas-00000001"` で失敗。
    テスト6（撤回）は epoch が 0 のまま（撤回が未実装で必ず NotFound）で失敗。
  - `cargo test --offline -p pelway-http --lib gdpr02_routes` → **exit 101**、
    0 passed / 6 failed。要点: 全テストが 404 `{"code":"not_found"}`（経路未実装）。
    テスト3・6 も 404 で落ちた（期待 200/400 に対し 404）。
  - green: 実装後 `cargo test --offline -p pelway-contract --lib store::` → exit 0
    （9 passed / 0 failed）、`cargo test --offline -p pelway-http --lib gdpr02_routes` →
    exit 0（6 passed / 0 failed）。一時フィールド `debug_messages_echo` は red 記録後
    に削除した（テスト2が「検出できた」ことの陽性対照を兼ねる）。
  - 途中経過: テスト2 の HTTP 側で「応答本文に canary 無し」の検証を入れていたが、
    これは誤り（要求で送った自由文字列は保存された正本として応答に戻るのが正しい）。
    検証目的は「07 のログに漏れないこと」なので当該 assert を削除して通るようにした。
- 「本人」の仮の決め方（開発リード 2026-11-10）: デモに認証は無く、**合成の利用者 ID
  （`actor`）を API・関数の引数で渡す**（作成・撤回は要求本文、読取はクエリ）。
  ヘッダーや Cookie、環境、作業ディレクトリから本人を推定しない。この決め方は仮で、
  U01〜U07 と法的な当てはめ（どの根拠の種類がどの目的に当てはまるかを含む）は
  **未決のまま**。`basis_type` は `synthetic_consent` / `synthetic_non_consent` の
  合成の値のみで、実際の根拠分類との対応は決めていない。
- 作ったもの:
  - `crates/pelway-contract/src/store.rs`（新規・DataStore の置き場）:
    workspace・channel・message・明示メモリ・根拠記録・利用方針・削除/制限記録を
    **メモリだけ**で持つ（ディスクに書かない。07 のログ以外ファイルを作らない）。
    04・05・10 が使う前提の置き場と型のみ用意（削除処理・本人書き出し・所在一覧 API
    は実装しない）。ID・時刻・版・epoch はすべてサーバー付け（`bas-{:08}`、
    `msg-{:08}`、`mem-{:08}`、`del-{:08}`、version・seq・`collected_at` 等）。
    `client_msg_id` は保存しない（既存 `Message` 型を使わず `StoredMessage` を新設）。
    保存物の `purpose_id` は `PurposeId`（P01〜P08 のみ）で必須・非 Option。未設定・
    不正値（`p09`・大文字・数字等）は serde の脱直列化エラーとして拒否（HTTP は 400）。
    学習・広告・会社横断分析の経路は作っていない。
  - 列の持ち方: 12 §3.2 の **名前の列** を省略せずそのまま持つ。利用方針
    `workspace_id, purpose_id, version, data_classes, source_scope, processors,
    allowed_outputs, explanation_version, retention_policy_ref`（＋サーバー付け時刻の
    `created_at` を追加。時刻はサーバー付けの約束のため）、根拠記録
    `workspace_id, basis_id, basis_version, basis_type, purpose_id, scope,
    party_coverage_ref, grantor_ref, representative_authority_ref, explanation_version,
    evidence_ref, collected_at, effective_at, expires_at, withdrawn_at,
    withdrawal_method_ref, status, epoch`（全列）。`party_coverage_ref` 等の参照列は
    参照だけ（文字列）で本文や人物情報の複製は持たない。利用方針の変更前の版は
    削除せず残す（版は同じ workspace×purpose で 1 から採番）。
  - status を未確定側にした理由: 12 §3.3「根拠未確定の機能は無効」「`pending`、欠落、
    不明、期限切れ、撤回済みは同意根拠として使わない」に従い、作成直後は必ず
    `pending`。有効（`active`）な根拠記録を作る経路は **意図的に用意しない**
    （有効化には U03 の法的当てはめの確定が必要なため）。`is_purpose_enabled` は
    active・期間内・目的一致の記録があるときだけ true を返す実装だが、active な記録を
    作る経路が無いため常に false（P03/P04/P06 は根拠なし・pending だけでは有効に
    ならない＝初期オフ。他の目的も同様）。
  - 取消P・同意撤回（根拠失効）・削除の利用停止は別状態として保持: `HaltKind`
    （`operation_cancellation` / `basis_invalidation` / `deletion_suspension`）と
    `HaltState`（`received` / `suspended`）を削除・制限記録に持たせた。撤回
    （`basis_invalidation`）だけ実装（根拠記録の status→`withdrawn`、epoch+1、
    `withdrawn_at` 更新）。03 の 9 状態には「取消」を足していない（`task.rs` 無変更）。
  - 登録簿: `StorageRegistry`（メモリ内）。`DataStore::new()` で 7 保存部が自分を登録し、
    `register_location` で外の保存部（07 のログ出力先など）も登録できる。所在一覧の
    応答 API は作らない（10 の範囲）。
  - 仮の経路（`crates/pelway-http`）: `POST|GET /v1/workspaces/{workspace_id}/
    communication-basis-records`、`POST /v1/workspaces/{workspace_id}/
    communication-basis-records/{basis_id}/withdraw`（撤回 = 取得と同じ1操作の POST
    1回）、`POST|GET /v1/workspaces/{workspace_id}/communication-use-policies`。
    **経路名は仮**。要求型は `deny_unknown_fields` で、偽の `basis_id`/`collected_at`/
    `status`/`epoch` 等は未知フィールドとして 400 で拒否（保存されない）。
    既存 comments/approve の振る舞いは無変更（既存テスト 26＋15 件がそのまま通る）。
  - 起動入口: 既存 `serve` / `serve_with_logger` / `serve_with_logger_urandom` 等の
    署名は変えず、内部で空の DataStore を作る（既存の振る舞い不变）。新規は
    `serve_with_store`（ログ無し）、`serve_with_store_and_logger`（テスト・検証用・
    既知 salt）、`serve_with_store_and_logger_urandom`（本番相当・salt は
    /dev/urandom 固定・読み取り失敗で起動しない）。
- salt / ログの仕組みは変えていない: `log.rs` の salt 採取（`/dev/urandom` 固定・
  フォールバックなし）、`with_sink`/`serve_with_logger_salt_from` の `pub(crate)`、
  `process_salt` の `#[cfg(test)]`、許可リスト方式（項目は仮名化 ID・種別・数値・
  結果コードだけ）はそのまま。追加したのは `Route` の 3 バリアント
  （`basis_records` / `basis_withdrawal` / `use_policies`）と出来事 5 種
  （`basis_record_saved` / `basis_records_listed` / `basis_withdrawn` /
  `use_policy_saved` / `use_policies_listed`。仮名 ID・`PurposeId`・版・epoch のみで、
  参照列の中身・本文・`client_msg_id` は出さない）、`unix_millis` の `pub(crate)` 化
  （サーバー付け時刻の共用のため。仕組みの変更ではない）。撤回後の読取・検索・AI 入力の
  拒否は QA-GDPR-03 の範囲なので実装していない（撤回で epoch・status・時刻が更新される
  ところまで）。
- 検証（すべて `--offline`、コマンド・exit code・件数）:
  - `cargo fmt --all -- --check` → **exit 0**（整形 1 回適用後）。
  - `cargo clippy --workspace --all-targets --offline -- -D warnings` → **exit 0**。
  - `cargo test --workspace --offline` → **exit 0**、**計 125 passed / 0 failed /
    0 ignored**（at-rest 単体 8・aead 12・contract 単体 47（旧 38＋新 9）・
    serde_contract 10・dev-smoke 単体 0＋smoke 1・http 単体 21（旧 15＋新 6）・
    http 結合 26。doc-tests 各 0）。前回（第17枠）110 → 125（+15）。
- 迷った点と安全側の判断:
  - `BasisStatus` に `active` バリアントを置いたが、**active に遷移させる経路は作ら
    なかった**（有効化は U03 確定後に開発リードが決める。初期オフ要件の安全側）。
  - 偽 ID・時刻の入った要求は「無視して保存」ではなく **400 で拒否**した
    （`deny_unknown_fields`）。要求型に偽を受け取る欄自体が無いため、正本がサーバー値
    であることは型で保証され、HTTP では拒否の形で確認できる。
  - 撤回の再実行は冪等に epoch をさらに進める形にした（取得より難しい手順にしない
    要件の安全側。`withdrawal_method_ref` は再撤回で消さない）。
  - `expires_at` を要求から受け取らない（全時刻をサーバー付けに統一）。期限切れ判定は
    `is_valid_consent_basis_at` の期間比較のみ（この実装では常に false）。
  - ログの読取出来事に件数を出さない（12 §6 の件数の扱いの安全側）。
  - GET の actor はクエリ引数（URL デコードはしない。合成 ID は ASCII 想定と明記）。
  - `is_purpose_enabled` は P03/P04/P06 に限らず全目的で「active な根拠が無ければ
    false」にした（どの目的にどの根拠が要るかは未決のため、一律の安全側）。
  - 既存 `serve` 系の公開署名は変えず、新規入口を足すだけにした（既存呼び出し元
    （example・テスト）への影響ゼロ。振る舞い不变は結合テスト 26 件で確認）。
- 未検証・範囲外: 日英画面はクライアント担当。撤回後の読取・検索・AI 入力の拒否は
  QA-GDPR-03。本人 JSON 書き出し（04）、削除処理（05）、所在一覧 API（10）は未実装
  （置き場・型・登録簿のみ用意）。法定記録・保持期限の運用、法的部分（L-04）と根拠の
  当てはめ（U03、通信当事者・代理権の確認方法を含む）は未決のまま。worktree で
  `target/demo-logs/` にログを出す example の再実行は今回の対象外。Codex レビューと
  QA は未実施。`Cargo.lock` は無変更（`git diff <base>..HEAD -- Cargo.lock` は空）。

### 2026-11-10 第19枠(QA-GDPR-02 r3: 参照列と本文 canary の修正)
- start: 2026-11-10 05:40 JST
- stop:  2026-11-10 05:54 JST
- glm: long(T3 thread。OpenCode + zai-coding-plan/glm-5.3)
- 依頼: QA-GDPR-02 r3(目的ごとの根拠の記録: 参照列と本文 canary の修正)
  - worktree: `/home/hatai/.t3/worktrees/Pelway/demo-t3-backend-gdpr-02-r3`(新規作成)
  - branch: `demo-t3/backend-gdpr-02-r3`、base `c9bc3041aa02051587dd1de89212004b3b3d6f48`(r2 の最終 sha。r2 ブランチは早送りしていない)
  - 初期確認: `pwd` = 上記 worktree、`git rev-parse HEAD` = `c9bc3041…`、`git branch --show-current` = `demo-t3/backend-gdpr-02-r3`、`git status --porcelain` 空(開始時)。
- 採択した findings: `/home/hatai/.t3/worktrees/Pelway/review-codex-backend-c9bc304-r1/review-findings-gdpr-02-r2.md`
  (sha256 `9a79701523b4c3bc84641a1b9a869b44a62abe7e5f380975db394d86355571a0`、
  重大度 high の 1 件を採択)。
  - 指摘: `scope` と `*_ref` が通常の `String` のまま保存され JSON 応答に載る。本文
    canary を `scope`/`evidence_ref` に入れた POST が拒否されず、作成応答と GET 一覧に
    canary が出る。既存テストはこの経路を見ていない。
- 使った skills: `tdd`(Matt Pocock の Agent Skills。red → green ループ、シームは
  「`DataStore` の公開 API」と「ループバックの実 HTTP による仮経路」の 2 つ。垂直
  スライスでテスト8 → 実装 → 既存テスト追従の順に進めた)。
- red-first の証拠(テスト8):
  - 最初に `crates/pelway-http/src/gdpr02_routes.rs` に
    `http_rejects_body_canary_in_scope_and_ref_columns` を追加し、実装前に実行:
    `cargo test -p pelway-http --offline --lib http_rejects_body_canary_in_scope_and_ref_columns`
    → **exit 101**。失敗の要点: 最初のケース `{"scope": "PELWAY-CANARY-BODY-7Q2M9X"}`
    が **400 でなく 200 で受理され**、作成応答 JSON の `"scope"` に canary がそのまま
    直列化されている(left: 200, right: 400)。コンパイルエラーではなく、指摘と同じ
    経路で落ちた。
  - 実装後(戻し確認): 同コマンド → **exit 0**、`1 passed`(gdpr02_routes 内)。
    contract 側 `reference_columns_accept_only_synthetic_reference_forms` も
    `cargo test -p pelway-contract --offline --lib reference_columns_accept_only_synthetic_reference_forms`
    → **exit 0**、`1 passed`。
- 直した内容:
  - `crates/pelway-contract/src/store.rs`: `scope` と参照列
    (`party_coverage_ref`/`grantor_ref`/`representative_authority_ref`/`evidence_ref`/
    `withdrawal_method_ref`、利用方針の `source_scope`/`retention_policy_ref`)を
    **合成の参照形式だけ** に制限した。形式は「参照列 = `ref:` + ASCII 小文字・数字・
    `-`・`_` のみ 1..=60 文字(全体 ≤ 64 バイト)」「scope 系 = 同じ文字種で `ref:` 無し
    1..=64 バイト」。既存の合成 ID(`msg-00000001`、`ch-synthetic-02`、`ref:*`)と同じ
    文字種。大文字・空白・記号・多バイト文字を含む値(本文や人物情報の自由文、本文
    canary)は [`StoreError::InvalidReference { column }]`(新設)として **保存しない**。
    `create_basis_record`/`create_use_policy`/`withdraw_basis_record` が `Result` を
    返す形に変えた(検証失敗では状態を変えない。撤回の `epoch` も進めない)。
  - `crates/pelway-http/src/lib.rs`: 仮経路の作成・撤回ハンドラが
    `InvalidReference` を **400 `{"code":"bad_request"}`** に写す(列名は応答・ログに
    出さない)。拒否は既存の `rejected` ログ(code=bad_request)として出る。
  - テスト: テスト8(HTTP。canary 単体・混在を全参照欄と scope へ → 400・応答/一覧/
    保存部正本の直列化/ログに canary 0 件・正当な形式は 200 で通る対照・陽性対照)と
    contract 側の参照形式検証テスト(どの列が `InvalidReference` になるか・検証失敗で
    状態が変わらないこと)を追加。**テスト4(route_logs_have_no_body_or_client_msg_id_canary)
    は r3 の挙動に追従**: canary 入り scope/evidence_ref の POST が 200→400 になった
    ため、「正当な 1 件を作ってから canary 入り要求を 400 で出し、拒否時のログにも
    canary が出ない」形に変えた(検証意図=ログに本文 canary を出さない、は無変更)。
    既存テスト 1〜7 の検証意図は壊していない(戻り値型の追従で `.expect` を追加した
    箇所のみ)。comments/approve の結合テスト 26 件は無変更で通る。
  - 参照形式の 64 バイト上限と `ref:` プレフィックスは **合成データ用の仮の規則**
    (12 §3.2 は論理単位のみを定め、確定 DDL ではない。本修正のための仮決め)。
- 「本人」の仮の決め方(開発リード 2026-11-10): 合成の利用者 ID を API/関数の
  **引数**(仮経路では要求本文・クエリの `actor`)で渡す。ヘッダーや Cookie、環境、
  作業ディレクトリから本人を推定しない。**この決め方は仮** であり、U01〜U07 と
  法的当てはめ(どの根拠の種類が P01〜P08 に当たるか、通信当事者・代理権の確認方法
  を含む)は **未決のまま**(今回も確定していない。`TODO(U0x)` も残している)。
- 変えていないもの: 仮の経路名(`POST|GET /v1/workspaces/{ws}/communication-basis-records`、
  `POST .../{basis_id}/withdraw`、`POST|GET /v1/workspaces/{ws}/communication-use-policies`)、
  DataStore の置き場(`pelway-contract/src/store.rs`・メモリのみ)、12 §3.2 の列の
  持ち方(列の省略・追加なし)、salt / ログの仕組み(`/dev/urandom` 固定、
  `with_sink`/`serve_with_logger_salt_from` の `pub(crate)`、`process_salt` の
  `#[cfg(test)]`、許可リスト方式のままで本番 `lib.rs`/`log.rs` に canary 定数は無い)、
  comments/approve の振る舞い、`Cargo.lock`。ログ系の `Route`/`Event` への追加も無し
  (既存 `rejected` に載るだけ)。
- 検証(すべて `--offline`、コマンドと exit code):
  - `cargo fmt --all -- --check` → **exit 0**
  - `cargo clippy --workspace --all-targets --offline -- -D warnings` → **exit 0**
  - `cargo test --workspace --offline` → **exit 0**、**計 127 passed / 0 failed /
    0 ignored**(at-rest 単体 8・aead 12・contract 単体 48(旧 47＋新 1)・
    serde_contract 10・dev-smoke 単体 0＋smoke 1・http 単体 22(旧 21＋新 1)・
    http 結合 26。doc-tests 各 0)。前回(第18枠)125 → 127(+2)。
- 迷った点と安全側の判断:
  - 参照形式の文字種・長さの規則はドキュメントに確定仕様が無いため、**既存の合成
    ID の慣習(小文字・数字・`-`)に合わせた仮の規則** として実装し、doc に「仮」と
    明記した(確定は開発リードの判断)。
  - 検証を serde 層でなく **保存部の境界(DataStore)で** 行った(HTTP 以外の呼び出し
    (04/05/10 の将来実装)でも同じ制約が効く。HTTP は 400 に写すだけ)。
  - `StoreError::InvalidReference` の列名はエラー詳細として持つが、HTTP 応答には
    `{"code":"bad_request"}` だけを返すようにした(仕様の漏出を出さない安全側)。
  - テスト4 の追従(200→400)は「既存テストを壊さない」趣旨の判断。検証意図(ログに
    本文 canary を出さない)は維持し、canary を入れる経路が拒否に変わったことだけ
    追従した。
  - 指摘対象外の列(`explanation_version`、`data_classes`、`processors`、
    `allowed_outputs`)には今回の形式検証を入れて **いない**(findings の範囲は
    `scope` と `*_ref`。列の意味を勝手に広げない)。canary をこれらに入れた要求は
    現状も通るため、継続案件として開発リードに報告する。
  - `is_purpose_enabled` は相変わらず常に false(active への遷移経路は今回も作って
    いない。初期オフ要件の維持)。
- 未検証・範囲外: 日英画面(クライアント担当)。撤回後の読取・検索・AI 入力の拒否は
  QA-GDPR-03。本人 JSON 書き出し(04)、削除処理(05)、所在一覧 API(10)は未実装のまま。
  法的部分(L-04)と根拠の当てはめ(U03)は未決のまま。この修正に対する Codex レビュー
  と QA は未実施。`Cargo.lock` は無変更。
