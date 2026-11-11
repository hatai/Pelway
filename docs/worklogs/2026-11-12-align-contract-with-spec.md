# 作業ログ: コードを仕様に合わせる（承認APIのパス・メンバーの種類・client-coreの契約型）

- 日付: 2026-11-12（木）JST／担当: 実装担当エージェント（T3 Code / OpenCode + glm-5.3）／Issue: #91
- 環境: worktree `agent-91-align-contract-6195a03fd6b9c7d5`、branch `agent/91-align-contract-with-spec`、base `origin/main`（3d685e1）
- 対象外: `docs/technical/v0.7`（仕様側）の変更、`decide` 1 API への統合

## 目的と方式の判断

Issue #91 の 3 点: ①承認APIのパスを仕様（04 §3 の `/v1/approvals/`）に合わせ経路定数とサーバー判定を一致させる、②`MemberKind` を仕様 04 §2 の 4 値に、③`pelway-client-core` の手写し契約を `pelway-contract` 直接利用に置き換える。

③は Issue が「直接使う／生成する」の 2 案を提示。**直接使う（path 依存）を採用**: 同一ワークスペース内でずれが構造的に起こらず、単一 crate 向けの生成は管理コストが上回る。これに伴い `pelway-client-ui/tests/no_body_persist.rs` の「core の `[dependencies]` は空」ガードは「path 依存のみ（レジストリ依存ゼロ）」に調整（ガードの目的は維持）。

## 成果物と TDD

- `pelway-contract`: `comments_path()`/`approve_path()` 追加、`MemberKind` を 4 値に（`as_str`/`parse` 付き）、`snapshot.rs` の種類タグを `as_str` 経由に。
- `pelway-http`: 経路判定をテンプレート定数との照合（`match_path`）に、ヘッダー解析を `MemberKind::parse` に、`ActorKind` を 4 値に。
- `pelway-client-core`: `src/contract.rs` を再エクスポート化、path 依存を追加。
- テスト: 各 AC を先に書き失敗を確認（AC1 は 404≠200、AC2/AC3 はコンパイルエラー）→ 実装後 ok。経路リテラルを排除し `comments_path`/`approve_path` 経由に。

## 検証

- `cargo test --workspace` → 0／`mise run verify` → 0

## 未検証・未決

- 種類タグが `ai`→`bot` 等に変わるため旧デモデータの `snapshot_hash` とは一致しない（合成データのみ）。
