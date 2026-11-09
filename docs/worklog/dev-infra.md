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

### 2026-11-09 第3枠（GLM 不使用枠）
- start: 2026-11-09 15:02 JST
- stop:  2026-11-09 15:09 JST
- glm: none（T3 thread は使っていない）
- 内容（QA の 2ef065a レビュー指摘とリード確定 15:01/15:03 を反映。コミット d707dfc）:
  - 必須修正: `run.sh` は段階名の完全一致（loopback / leak-phrases / external-refs / canary）だけを受け付け、それ以外の引数（パス・未知の名前・空文字）が 1 つでもあれば何も実行せず exit 2。
  - 段階を指定した実行は `PARTIAL RUN: not valid for acceptance` と表示し、まとめに「受入は引数なしの全段階実行だけ」と出す。
  - loopback: PELWAY_BIND_ADDR は 127.0.0.1:<port> と [::1]:<port> の完全一致だけ（localhost・0.0.0.0・127.0.0.10・括弧なし ::1 は不可）。
  - allowed-hosts.txt: 127.0.0.1 と ::1 だけ（localhost を削除）。
  - forbidden-phrases.txt: 英語の secure / safe / encrypted / fully deleted / permanently deleted（語単位・大文字小文字を区別しない）、24/72 時間・hours の表記そのもの（以前の「近くに通知」より厳しい）、秘密値の形（PRIVATE KEY ブロック・Bearer トークン・sk- トークン）を追加。冒頭に対象ファイル範囲（利用者向け文言と外向け文書だけ）を明記。状態はリード確定 2026-11-09 15:01/15:03。
  - 外向け文書として docs/public/ を走査対象に追加（docs/ のそれ以外は対象外のまま）。
  - Rust の文字列リテラル先頭にある語（例 "24時間"）を見落とす不具合を修正（自己テストで発見）。
  - 新しい段階 canary と canaries.txt（ダミーの目印 3 つ）。target/・logs/・log/・*.log・DEMO_CHECK_EXTRA_PATHS を走査し、走査対象が無ければ "no artifacts to scan" と明示して合格扱い。
  - 自己テストを 68 件に拡充。
- 検証（コマンドと exit code）:
  - `bash -n scripts/demo-check/*.sh` すべて 0
  - `mise run demo-check` 0（full run、4 段階。loopback static 0 件・runtime PENDING、leak-phrases 0 件、external-refs 0 件、canary 0 件。走査はソース 43 ファイル、canary 49 ファイル＝target/ の中身）
  - `bash scripts/demo-check/run.sh canary` 0（PARTIAL RUN 表示を確認）
  - `bash scripts/demo-check/run.sh ../../x/ok` 2（何も実行しない）
  - `mise run demo-check-selftest` 0（68 passed, 0 failed）
  - `mise run fmt-check` 0 / `mise run check` 0 / `mise run hk-check` 0
- 未検証・未決:
  - loopback の runtime 検査は未実装（デモの起動方法の決定待ち）
  - canary は今の target/ が開発確認用クレートの中身だけで、合成データを使ったデモのビルド・ログではまだ未確認
  - 英語の safe / secure は語単位でも、否定文や技術的な説明文（例: thread-safe の safe）で誤検出しうる
  - shellcheck は未導入のため未実施
  - 独立コードレビュー（Claude または Codex）は未実施
