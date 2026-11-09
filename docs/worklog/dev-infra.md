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
- stop:  2026-11-09 14:56 JST
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
