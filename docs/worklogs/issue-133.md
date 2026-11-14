# Issue #133 の作業ログ

## 実装者メモ（round 3）

対象 head: 4ab442d（この round の実装の実体を含むコミット。このメモを含むコミットの親。署名付け替え前は d60592f）

1. 詰まった箇所: `src/acp.rs`: `connect_with` のコールバックが `Result<_, acp::Error>` 固定のため、`NoSessionCapability`/`Disconnected` の種別を保つ `Result<Result<SessionReport, AcpError>, Error>` の二重 Result にした。`tests/inprocess_gateway.rs`: feature `acp` で agent-client-protocol が `serde_json/preserve_order` を有効化し、引数の key 順が変わって既存試験が壊れた（順序仮定を内容含有の表明に直した）
2. 曖昧だった文: 「task ごとの export 対象の固定」（Issue #133「決めること」）— export 対象が prompt 本文の構造までを指すか、コンテキスト一式の粒度を指すか分かれた。MVP では `ExportEnvelope`＝prompt 本文 1 本とし、追加データは「後続 Issue の限定 MCP サーバー」に読み替えた
3. 仮定と解釈: `session/request_permission` の操作名は `subject.tool_call.name`、無ければ先行 `tool_call_update` の `name` を toolCallId で引く相関とした（R1-Q5 案a）。subject が ToolCall でない要求は操作名なし＝fail closed と解釈した。`integration_grant` の未実装項目（data分類・宛先・有効化版）は `GrantRegistry` の (連携,アカウント,操作) キーに畳み込み、拡張は別 Issue とした
4. 足りなかった情報・検証手段: `session/cancel` の送り方は SDK の `ActiveSession` に cancel ヘルパが見当たらず `CancelSessionNotification` の送信で行った。agent 側の permission 送信例が公式例に無く、`send_request` の対称使用は SDK の API 面から類推した
5. 改善案: `docs/research/agent-runtime-standard-protocols.md` の「未確認事項」に「Rust schema crate（`agent-client-protocol`）の v2 対応状況」があるが、rust-sdk v5.2.0 が `unstable_protocol_v2` で `Client.v2()`/`Agent.v2()` を提供済みと確かめたので、この項を「確認済み: rust-sdk 5.x が draft-v2 API を `unstable_protocol_v2` 後ろに提供」と更新する

使ったスキル: `.agents/skills/implement-issue`、`.agents/skills/grilling`（frontier の切り方）、`.agents/skills/write-pull-request`（PR 本文の形）

## 実装者メモ（round 4）

対象 head: 65dfcaa（この round の実装の実体を含むコミット。このメモを含むコミットの親。署名付け替え前は 206ccda）

1. 詰まった箇所: `src/acp.rs`: round 3 のレビュー must-fix。`AgentGrant` のフィールドが公開で、`GrantRegistry::require` を通さず任意の grant を構築して `run_prompt` に渡せていた。フィールドを非公開化し、`ConnectAttempt` 監査に検証済みの連携×アカウント×操作を載せて dead_code も解消
2. 曖昧だった文: 「grant を registry の検証でのみ生成できる型にし、その検証済み値を使う API にしてください」（PR #304 レビュー本文）— `run_prompt` 内で再検証する実装か、型で偽造不能にするか分かれた。`AcpClient` が registry を持たない設計のため、型レベルの偽造不能化（フィールド非公開）を選んだ
3. 仮定と解釈: `AgentGrant` の `Clone` 派生は許容と解釈した（検証済み値の複製は偽造ではない）。`grant()` を呼べる側＝管理者の設定面と同じ信頼度とみなし、in-memory registry 自体の改ざん防止は対象外とした
4. 足りなかった情報・検証手段: 「外部から構築できないこと」の機械的検証は trybuild 等のコンパイル失敗試験が必要だが、リポジトリにその基盤が無い。doc comment とレビュー観点で担保した
5. 改善案: `docs/technical/v0.7/body/10-domestic-only.md` の `integration_grant` 節に「grant の実体は検証関数でのみ生成できる型（非公開フィールド）で表し、外部エージェント接続 API はその検証済み値だけを受け取る」を足す

使ったスキル: `.agents/skills/implement-issue`
