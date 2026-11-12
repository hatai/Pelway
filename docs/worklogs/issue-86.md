# Issue #86 の作業ログ

## 実装者メモ（round 1）

1. 詰まった箇所: crates/pelway-contract/src/ledger.rs の `non_decider_cannot_replay_completed_key`: 承認後のカードは版 2 なので、リプレイ要求を `current_req`（現在版）で作ると `IdempotencyKeyReused` になる。リプレイは完了済みキーの binding（版 1）そのもので送るのが正しい形だった。ほか scripts/pr-size.sh: 全差分が 836/800 行。重複の除去（台帳と inbox に重複していた承認者解決の approval.rs 自由関数化、(a0) と重複する dec2 の承認成功テスト、状態不変 assert の統合）で 868→836 まで減らしたが、残りはテスト・説明の削減になるため分割案を Issue に書いて止まった。
2. 曖昧だった文: 「承認と却下の操作で、操作した人の身元を受け取り、カードの承認者でなければ拒否する」（Issue #86 本文「やること」）と「承認者は、指名した1人か、役割（例：代表）のどちらかにする」（docs/adr/0002-designated-approver-and-expiry.md の本文）
3. 仮定と解釈: 却下 API は現在システムに存在しない（契約・台帳・HTTP に経路が無い）ため、この Issue では承認 API に身元検査を実装し、台帳の検査を「操作者の身元を引数に取る」形にして却下 API ができたとき同じ形で適用できるようにした。役割「代表」の解決先はワークスペース設定（`ApprovalPolicy.representative`）とし、解決できないカードは作成時に `UnresolvedDeciderRole` で断る形（承認時に毎回解決する形は選ばなかった。ADR の「承認者を承認カードに固定し」に合わせ、作成時点で誰の判断待ちか定まらないカードを作らない）。承認者検査は冪等キー検査より前に置いた（承認者でない人による完了済みキーのリプレイ結果の再取得を防ぐ。キー検査が先だと 409/422 で内容を漏らす）。既定の `allow_requester_as_decider` は true（既存の合成データ・デモ・テストの requester == decider のカードを壊さない。ADR は既定値を決めていない）。
4. 足りなかった情報・検証手段: 却下 API の予定される経路・状態遷移（`TaskState::Stopped` への遷移操作は契約に無い）。承認者が承認した事実の記録（誰が承認したかの監査ログ項目。今回はログは既存の `approved` 出来事のまま）。
5. 改善案: docs/superpowers/specs/agent-team.md の「実装者メモ」の節に「Issue に『採用する方式』の見出しや却下・承認など未実装操作への言及があるとき、実装者がどこまで実装し何を先送りしたかを PR 本文のどこに書くか」を足す。
