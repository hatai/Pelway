# 作業記録: 開発基盤（dev-infra）

- 担当: Pelway 開発基盤エンジニア
- worktree: `/home/hatai/.t3/worktrees/Pelway/demo-t3-platform`
- branch: `demo-t3/platform`（base `f32ca74c25d8c5614e7655a11c2a929be8211a10`）
- 時刻はすべて日本時間（JST, UTC+9）
- データは合成データのみ。外部送信なし。待受は 127.0.0.1 のみ。

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
- stop:  2026-11-09 14:55 JST
- glm: none（T3 thread は使っていない）
- 内容:
  - `mise.toml` の rust に `targets = ["wasm32-unknown-unknown"]` を追加。`mise install` で `mise.lock` の rust options に targets が追記された。
  - `demo-check` の土台: `mise run demo-check` → `scripts/demo-check/run.sh` が 3 段階（loopback / leak-phrases / external-refs）を全部実行する。各段階は未実装のため `NOT IMPLEMENTED` を出して exit 3、全体は exit 1。
  - この作業記録を追加。
- 検証（コマンドと exit code）:
  - `mise install` 0（mr-boxington の sigstore 取得で一時的な 504 警告あり、再試行後に完了）
  - `mise exec -- rustc --version` → rustc 1.96.0、0
  - `mise exec -- rustup target list --installed` → wasm32-unknown-unknown, x86_64-unknown-linux-gnu、0
  - `mise exec -- cargo check --workspace --locked --target wasm32-unknown-unknown` 0
  - `mise run demo-check` 1（3 段階とも NOT IMPLEMENTED。想定どおり）
  - `mise run fmt-check` 0 / `mise run check` 0 / `mise run hk-validate` 0 / `mise run hk-check` 0 / `mise run toolchain-check` 0
- 未検証・未決:
  - demo-check の 3 段階の中身（禁止語リスト・外部参照の許可リストは QA と合意してから）
  - `hk install` はこの枠では実行していない。hk の pre-commit は既にリポジトリ共有の `.git/config`（`hook.hk-pre-commit`）に登録済みで、コミット 45cd1e6 の時に実行され通過した（trailing_whitespace / newlines。Rust ファイルの変更が無いため cargo 系は対象外）
  - shellcheck は未導入のため未実施（`bash -n` の構文確認のみ 0）
  - 独立コードレビュー（Claude または Codex）は未実施
- 共有ツールチェーンへの追加（事後記録）:
  - `mise install` で `rustup target add wasm32-unknown-unknown` 相当が、共有の `~/.rustup` のツールチェーン 1.96.0 に入った（追加のみ・削除なし）。mise の rust は `~/.cargo/bin` への symlink のため、main と他の worktree からも見える。
  - 開発リードが事後に了承。今後、共有・グローバルな場所（`~/.rustup` `~/.cargo` `~/.local` apt など）への追加は、事前にルームでリードへ知らせてから行う。

### 2026-11-09 第2枠（GLM 不使用枠）
- start: 2026-11-09 14:56 JST
- stop:  2026-11-09 15:01 JST
- glm: none（T3 thread は使っていない）
- 内容:
  - demo-check の 3 段階を実装（コミット「feat(dev-infra): implement demo-check stages…」）。走査範囲・除外は `scripts/demo-check/lib.sh` の冒頭に記載。
  - loopback: static 検査（PELWAY_BIND_ADDR が 127.0.0.1:<port> か、全アドレス待受の記述が無いか）は失敗で exit 1。runtime（待受ソケットの列挙）は未実装で「runtime: PENDING」と表示し、合格扱いにしない（static の合格のみ exit 0）。
  - leak-phrases: `forbidden-phrases.txt`（QA未合意/暫定）。出力はファイル:行と規則だけで、行の中身は出さない。
  - external-refs: `allowed-hosts.txt`（QA未合意/暫定。localhost / 127.0.0.1 / ::1）。xmlns 属性の値は対象外。
  - 自己テスト `scripts/demo-check/selftest.sh` と mise タスク `demo-check-selftest`（29 件）。
  - 第1枠の stop を 14:55 に訂正、共有ツールチェーン追加の事後記録を追記。
- 検証（コマンドと exit code）:
  - `bash -n scripts/demo-check/*.sh` すべて 0
  - `mise run demo-check` 0（loopback static 0 件・runtime PENDING、leak-phrases 0 件、external-refs 0 件。走査 43 ファイル）
  - `mise run demo-check-selftest` 0（29 passed, 0 failed）
  - `mise run fmt-check` 0 / `mise run check` 0 / `mise run hk-check` 0
- 未検証・未決:
  - 禁止語・許可ホストのリストは QA未合意/暫定
  - loopback の runtime 検査は未実装（デモの起動方法の決定待ち）
  - 「安全」「安心」は語が広く、UI 文言の範囲（資産・UI 系ファイル全体と Rust の文字列リテラル）に絞っても、否定文や内部メッセージで誤検出しうる
  - shellcheck は未導入のため未実施
  - 独立コードレビュー（Claude または Codex）は未実施
