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
