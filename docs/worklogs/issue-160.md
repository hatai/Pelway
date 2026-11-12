# Issue #160 の作業ログ

## 対象と方針

`cargo llvm-cov -p pelway-client-ui --no-cfg-coverage` の LCOV から未カバー行を洗い出し、到達可能な分岐をテストで潰す。正本 JSON・正本 Fluent リソースに対する防御分岐（注入不能）は対象外とした。

| ファイル | 計測前 | 計測後 |
|---|---|---|
| pelway-client-ui/src/bridge.rs | 92/103（89.3%） | 101/103（98.1%） |
| pelway-client-ui/src/i18n.rs | 104/107（97.2%） | 104/107（97.2%） |
| pelway-client-ui/src/state.rs | 173/192（90.1%） | 192/192（100%） |
| pelway-client-ui/src/testkit.rs | 14/17（82.4%） | 17/17（100%） |
| pelway-client-ui/src/theme.rs | 220/235（93.6%） | 220/235（93.6%） |

crate 全体で 603/654（92.2%）→ 634/654（97.0%）見込み。

## 追加したテスト

- `crates/pelway-client-ui/tests/edges.rs`（新規）: `AvatarKind::initial`/`is_ai`、`ai_presence_key` 3 値、`TodaySectionKind::heading_key` 3 種、`FixedKeySource::default` が `new` と同じ連番を振ること、`ShellBridge` の `set_focus`/`focus`/`set_ime`/`approval_ref`、MessageInput フォーカスの Enter が `Action::Send` を適用して送信回数を増やすこと

## 残した未カバー（注入不能経路）

- `bridge.rs` 79: `on_approve_response` で `Effect::Approve` が返ったときの要求回数加算 — 中核の `ApprovalMachine::on_approve_response` は `Ok` では `None`、各 `Err` では `Notice` または `RecheckScheduled` を返し、`Approve` は `poll()` からしか出ない。公開経路では到達不能
- `bridge.rs` 193: llvm-cov の行領域の計上差分（`Action::Send` 腕は `enter_on_message_input_counts_a_send` で実行済みだが lib 側インスタンスとして未カウントのまま残る）
- `i18n.rs` 182, 201-202: Fluent の整形エラー → `None` → `apply_translations` の missing 経路 — 正本リソースの全キーは提供済みの変数で整形できるため、公開経路では発生しない（`Messages` の bundle は非公開で差し替えられない）
- `theme.rs` 260, 268, 294, 297, 300, 303, 308, 314, 398, 402, 419-420, 456, 487: 正本 `TOKENS_JSON`（`include_str!`）に対する防御分岐（`$` メタキー・非 srgb・成分不足・文字列エスケープ・パース panic 腕・`object_get` の非オブジェクト腕）— 入力が定数で差し替えられず到達不能

## 実装者メモ（round 1）

1. 詰まった箇所: 手順 4（テスト追加）: `no_color_values` 試験がテストファイル内の `(#160)` を 16 進カラー値と誤検出して失敗した（`(Issue 160)` に書き換えて解消）
2. 曖昧だった文: 「未カバー行を実質的に潰す」（Issue #160「受け入れ条件」）
3. 仮定と解釈: `include_str!`・正本 Fluent リソースに対する防御分岐と、中核が返さない `Effect` への備えは「実質的に潰す」の対象外と解釈した。注入用に private API を公開する変更は入れない方針とした
4. 足りなかった情報・検証手段: `no_color_values` がテストファイル内の `#` + 16 進 3 桁も色値として走査することは失敗して初めて分かった。Issue 番号を `(#N)` と書く既存ファイルは pelway-client-ui には無かった（他 crate にはある）
5. 改善案: `no_color_values` 試験の走査を「16 進の前後が単語境界」の条件に絞る改善の候補（`#160` や `#abc` のような Issue 参照・文字列中の短い # を許す）
