# Issue #86 の作業ログ

## 実装者メモ（round 1）

1. 詰まった箇所: crates/pelway-contract/src/ledger.rs の `non_decider_cannot_replay_completed_key`: 承認後のカードは版 2 なので、リプレイ要求を `current_req`（現在版）で作ると `IdempotencyKeyReused` になる。リプレイは完了済みキーの binding（版 1）そのもので送るのが正しい形だった。ほか scripts/pr-size.sh: 全差分が 836/800 行。重複の除去（台帳と inbox に重複していた承認者解決の approval.rs 自由関数化、(a0) と重複する dec2 の承認成功テスト、状態不変 assert の統合）で 868→836 まで減らしたが、残りはテスト・説明の削減になるため分割案を Issue に書いて止まった。
2. 曖昧だった文: 「承認と却下の操作で、操作した人の身元を受け取り、カードの承認者でなければ拒否する」（Issue #86 本文「やること」）と「承認者は、指名した1人か、役割（例：代表）のどちらかにする」（docs/adr/0002-designated-approver-and-expiry.md の本文）
3. 仮定と解釈: 却下 API は現在システムに存在しない（契約・台帳・HTTP に経路が無い）ため、この Issue では承認 API に身元検査を実装し、台帳の検査を「操作者の身元を引数に取る」形にして却下 API ができたとき同じ形で適用できるようにした。役割「代表」の解決先はワークスペース設定（`ApprovalPolicy.representative`）とし、解決できないカードは作成時に `UnresolvedDeciderRole` で断る形（承認時に毎回解決する形は選ばなかった。ADR の「承認者を承認カードに固定し」に合わせ、作成時点で誰の判断待ちか定まらないカードを作らない）。承認者検査は冪等キー検査より前に置いた（承認者でない人による完了済みキーのリプレイ結果の再取得を防ぐ。キー検査が先だと 409/422 で内容を漏らす）。既定の `allow_requester_as_decider` は true（既存の合成データ・デモ・テストの requester == decider のカードを壊さない。ADR は既定値を決めていない）。
4. 足りなかった情報・検証手段: 却下 API の予定される経路・状態遷移（`TaskState::Stopped` への遷移操作は契約に無い）。承認者が承認した事実の記録（誰が承認したかの監査ログ項目。今回はログは既存の `approved` 出来事のまま）。
5. 改善案: docs/superpowers/specs/agent-team.md の「実装者メモ」の節に「Issue に『採用する方式』の見出しや却下・承認など未実装操作への言及があるとき、実装者がどこまで実装し何を先送りしたかを PR 本文のどこに書くか」を足す。

## 実装者メモ（round 3）

1. 詰まった箇所: crates/pelway-contract/src/handoff.rs のテスト置換: `Decider::Member(member("m-bot-1"))` を機械置換で `member("m-bot-1")` に戻したとき `Decider::Member(` の閉じカッコが残り、区切りの壊れたテストが一度できた（cargo check の mismatched closing delimiter で発見）。
2. 曖昧だった文: 「作成時の解決先をカードに固定し、承認者の変更は明示的な再割当てとして新しい承認を要求してください」（PR #119 のレビュー2、pelway-review による must-fix）
3. 仮定と解釈: レビュー2のとおり、`ApprovalCard.decider` を作成時に解決した `MemberId`（固定）に戻し、`Decider`（指名/役割）は `NewCard`（カード作成時の指定）だけが持つ形にした。`inbox` はカードの承認者との比較だけ（policy 引数を削除）、handoff も `MemberId` 比較に戻る。`set_policy` は「以降のカード作成の解決にだけ効く」ことを doc に明記した（既存カードに影響しなくなるため残す）。レビュー1（PR 単体で Issue の受け入れ条件が未達）は指揮役の分割判断（Issue コメント参照）により後続 PR #121 が満たすため、PR #119 にコメントして案内する（コード変更の対象外）。round 1 の仮定「承認時に毎回解決する形は選ばなかった」は本 round で完全に実現した。
4. 足りなかった情報・検証手段: 「承認者の明示的な再割当て」の操作（handoff は担当の受け渡し用で、承認カードの承認者変更に使えるかは #89 の範囲の解釈。今回は既存の handoff をそのまま）。
5. 改善案: docs/superpowers/specs/agent-team.md のレビュー担当の節に「スタック PR の子が Issue を閉じるとき、親 PR の must-fix を子への案内として処理してよい条件」を足す。


## 実装者メモ（round 4）

1. 詰まった箇所: crates/pelway-contract/src/ledger.rs: `with_policy` を workspace 引数付きに変えたとき、テストの `ApprovalLedger::with_policy(ApprovalPolicy {...})` のような複数行リテラル引数の置換漏れが cargo check の E0061 で 4 件出た（順に直して解消）。
2. 曖昧だった文: 「ADR と Issue の要件どおり workspace ごとの設定を保持し、`create_card` では `new.workspace_id` に対応するポリシーを使ってください」（PR #119 のレビュー3、pelway-review による must-fix。Reviewed: 6898287）
3. 仮定と解釈: `ApprovalLedger.policies` を `HashMap<WorkspaceId, ApprovalPolicy>` にし、未設定のワークスペースは既定（兼任可・代表なし）を返す `policy(workspace_id)` を用意した。`with_policy` / `set_policy` も workspace 引数付きに変えた（台帳全体への単一設定はレビュー指摘の原因なので残さない）。解決はレビュー2どおりカード作成時の一度だけなので、承認・inbox・handoff は影響しない。
4. 足りなかった情報・検証手段: ワークスペースごとのポリシーを HTTP で設定する経路（まだ無い。台帳の組立でのみ設定）。
5. 改善案: docs/adr/0002-designated-approver-and-expiry.md の「ワークスペースの設定で決める」に「設定はワークスペースごと（台帳で共有しない）」を明記する。
