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
- stop:  2026-11-09 15:19 JST
- glm: none（実装はしていない。リード指示で 19:00 まで実装停止）
- 内容:
  - この作業記録の訂正（第2枠の stop、逸脱の記録）。docs のみのコミット。
  - Codex の読み取り専用レビュー（範囲 `f32ca74..4564e88`、全16ファイル）を T3 で始めようとした。
- 検証（コマンドと exit code）:
  - `t3 thread start . - --provider codex --model gpt-6-luna --title ...`（cwd はこの worktree、15:17:24）→ exit 1、`No T3 Code project matches '.'. Pass a project id or a path inside a project.`
  - この worktree のパスは T3 のプロジェクトとして登録されていないため、既存の worktree を指定してスレッドを始める方法が CLI に無い（`--worktree` は新しい worktree を作る）。指示に従い、回避せずに止めた。
- 未検証・未決:
  - Codex レビューは未実施（開始できていない）。リードの判断待ち。
