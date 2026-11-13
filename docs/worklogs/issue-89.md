# Issue #89 の作業ログ

## 実装者メモ（round 1）

1. 詰まった箇所: crates/pelway-contract/src/handoff.rs: `HandoffNotification.reason_key` を最初 `&'static str` で定義したら `Deserialize` のライフタイム制約（`'de` が `'static` を要求）でコンパイルできず、`String` に変えた。scripts/pr-size.sh ではなく、素の `cargo clippy` が mbx (mr_boxington) の環境変数なしでは `target/check` 配下の依存ビルドに失敗するため、検査は常に `mise run` 経由に直した。
2. 曖昧だった文: 「上限を超えたときや失敗したときは、チャンネルに知らせ、タスクを依頼者に戻す」（Issue #89 本文の「やること」）
3. 仮定と解釈: 「失敗」を受け渡しの拒否（段数上限・同一 2 者間の行き来）と解釈し、拒否は HTTP 200 の業務結果（`result: "refused"` + チャンネル通知 + 依頼者への担当戻し）として応答した。検査エラー（カード無し・現在の担当でない・自己受け渡し）は 4xx でカードも記録も変えないと読んだ。選ばなかった解釈: 拒否も 4xx にする案（「依頼者に戻す」という副作用が 4xx 応答から読み取れないため不採用）。「段数」はカードごとの受け渡し回数（1 始まり）と数え、3 回目まで受理。「同じ相手の間を行き来」は過去の受け渡しに同一組（向きは問わない）があることと定義した。
4. 足りなかった情報・検証手段: 依頼先メンバー（`to`）がチャンネルに存在するかの検証は台帳 (`ApprovalLedger`) が名簿を持たないため実装できていない（`DataStore` の `ChannelMember` との連携は未実装）。拒否時の「チャンネルに知らせる」を `DataStore` のシステムメッセージとして保存する経路も無く、refusal 応答 JSON の `notification` で表している。OpenBot の「1 回あたりの件数上限」は Issue に無いため対象外。
5. 改善案: docs/technical/v0.7/body/03-ai-bot-runtime.md の「実行所有者、再開、引継ぎ、取消」にある「BotがBotへ依頼する子タスクは親taskへ関連付ける。」の文に「受け渡しは引き継ぎ 5 項目を持つ記録としてカードに紐付き、段数は 3 段、同一 2 者間の往復は拒否、拒否時は担当を依頼者へ戻す（#89）」を足す。

## 実装者メモ（round 2）

1. 詰まった箇所: scripts/pr-size.sh: 全変更（941 行）が 800 行を超えたため停止し、Issue に分割案を出した（停止時のメモは Issue コメント https://github.com/hatai/Pelway/issues/89#issuecomment-6810739081 ）。指揮役の判断で 2 本に分割（PR 1: 契約・台帳、PR 2: HTTP 経路）。
2. 曖昧だった文: 「PR 2 は PR 1 のブランチに積んで出してください（PR 2 の本文に PR 1 への依存を書く）」（Issue #89 の指揮役コメント）
3. 仮定と解釈: 「積む」を stacked PR と解釈した。PR 2 の head ブランチ（`agent/89-ai-handoff-http`）は PR 1 の head ブランチ（`agent/89-ai-handoff-record`）の先端から切り、PR 2 の base を `agent/89-ai-handoff-record` にする。選ばなかった解釈: PR 2 を main から切り両 PR を独立させる案（HTTP 層は contract の handoff 型に依存するため、マージ順を制御できない）。
4. 足りなかった情報・検証手段: なし（分割の範囲と行数は Issue の分割案コメントのとおり）。
5. 改善案: .agents/skills/implement-issue/SKILL.md の「止まる条件」の表に「分割が承認されたときの再開手順（後続 PR の base は前段 PR の head ブランチにする）」を足す。

## 実装者メモ（round 3）

1. 詰まった箇所: crates/pelway-http/src/lib.rs: `Ok(decision @ HandoffDecision::Accepted { ref record })` のパターンが E0505（move と借用の衝突）になり、ログを出してから `HandoffDecision` を作り直す形に直した。
2. 曖昧だった文: 「PR 2 は PR 1 のブランチに積んで出してください（PR 2 の本文に PR 1 への依存を書く）」（Issue #89 の指揮役コメント）
3. 仮定と解釈: `mise run pr-size` の base は既定の `origin/main` ではなく `--base agent/89-ai-handoff-record` で数える（PR 2 の差分は HTTP 分だけが対象）。GitHub 側も base ブランチを `agent/89-ai-handoff-record` にする。選ばなかった解釈: base を main にして PR 1 の差分を重複して表示する案（差分行数も 800 制限を二重に食うため）。
4. 足りなかった情報・検証手段: ブランチ保護で PR の base に main 以外を指定できるかは本番確認前（作成時のエラー有無で判断する）。
5. 改善案: なし（round 2 の改善案に同じ）。

## 実装者メモ（round 4）

1. 詰まった箇所: crates/pelway-contract/src/handoff.rs: PR #113 のマージ時に PR #111（`NewCard` が `payload: ActionPayload` に集約）を取り込んでいなかったため、main の `cargo test` が `E0560`（`payload_hash`・`action_kind`・`recipient`・`body` は無い）で落ちた。`handoff_card` ヘルパー（contract・http 両方）を `ActionPayload` に載せ替えた。
2. 曖昧だった文: 「PR #114 の修正に ledger.rs のテストの修正も含め、mise run verify が通ることを確かめてから push する」（本スレッドへの修正依頼）
3. 仮定と解釈: 「ledger.rs のテスト」とは該当ヘルパーの実体が置いてある `handoff.rs`（contract の `#[cfg(test)]`）を含むと解釈した（エラー箇所は handoff.rs:278-281。ledger.rs 自体のテストは PR #111 で更新済み）。`hand_off` の実装コードは `ApprovalCard` の `channel_id`・`decider`・`requester` しか触らないため変更不要と判断した。
4. 足りなかった情報・検証手段: main の壊れを CI で検出できなかった原因（#113 マージ時点の CI が通った理由）は本スレッドからは確認できない。
5. 改善案: マージ前の「他 PR との競合で落ちるテスト」を防ぐ手段（マージキュー等）の検討を docs/decisions.md に記録する案。


