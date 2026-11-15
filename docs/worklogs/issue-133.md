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

## 実装者メモ（round 5）

対象 head: b7a30a8（この round の実装の実体を含むコミット。このメモを含むコミットの親。署名付け替え前は ab0f41c）

1. 詰まった箇所: `src/acp.rs`: round 4 のレビュー must-fix。`run_stdio` が gateway の環境変数・cwd を継承した子プロセスを隔離なしに起動でき、ExportEnvelope の境界を迂回する経路だった。R1-Q6 案a「隔離は task container 側」の確定回答に沿い、`run_stdio` を削除して `run_prompt` が隔離済み transport のみを受け取る形にした
2. 曖昧だった文: 「Run the agent in a sandbox with an explicit environment, cwd, filesystem mounts, and network policy before exposing this path, or make this API accept only an already-isolated transport.」（PR #304 round 4 レビュー本文）— adapter で sandbox を実装するか API 形状で防ぐか分かれたが、container 隔離は別層の責務なので後者を選んだ
3. 仮定と解釈: `build_session_cwd()` が session/new に送る cwd は、隔離済み transport 前提では情報漏えいに当たらないと解釈した（ACP schema が session cwd を要求するため残置）
4. 足りなかった情報・検証手段: 起動側の隔離強制は task container（別 Issue）側で確かめる経路であり、この PR の double 試験では検証できない。型で起動経路を消したことが静的な担保
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 6 に「push 後は `agent-gh-resign` が head を付け替えるため、メモの対象 head は署名付き SHA で書き直してから push する」を足す（round 3・4 で 2 回踏んだ）

使ったスキル: `.agents/skills/implement-issue`

## 実装者メモ（round 6）

対象 head: 0ec217d（この round の実装の実体を含むコミット。このメモを含むコミットの親。署名付け替え前は 3502718）

1. 詰まった箇所: `src/acp.rs`: round 5 のレビュー must-fix 2 件。(a) grant と接続先の同一性が無かったため `initialize.info.name` と grant の連携を照合し、不一致なら session 開始前に `IntegrationMismatch` で拒否（監査 `identity_rejected`）。(b) 操作名申告を信頼する構造は、client capability・client 提供 MCP server を一切 advertise しないことを試験で固定し、residual risk を module doc に明記
2. 曖昧だった文: 「Route protected actions through a gateway-controlled executor and derive the operation from that trusted call, or ensure the agent cannot perform those actions outside that executor.」（PR #304 round 5 レビュー本文）— gateway executor は R1-Q7(a) で後続 Issue 扱いと確定済みのため、この PR では「Pelway 仲介の副作用経路を持たない」ことを確かめる方向に読み替えた
3. 仮定と解釈: `initialize.info.name` を連携の識別子とするのは名前ベースの暫定 binding と解釈した（偽装は transport を作る呼び出し元の真正性に依存し、誤配送の防止が目的）。grant の integration 値は接続先の申告名に合わせる契約とした
4. 足りなかった情報・検証手段: 「不届きなエージェントが拒否後も実行する」動的検証は adapter 層では原理的に不可能（permission は仕様上 advisory）。ゼロ capability・ゼロ MCP server を double 側で観測する試験で代替した
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 9 に「レビューが『実装不能・後続 Issue の責務』を must-fix にしたときは、同じ文を引用して代替の担保（試験・doc・型制約）を書いたうえで stop 条件かどうかを示す」を足す

使ったスキル: `.agents/skills/implement-issue`

## 実装者メモ（round 7）

対象 head: d232bda（この round の実装の実体を含むコミット（origin/main の取り込み）。このメモを含むコミットの親。マージは署名付け替えの対象外）

1. 詰まった箇所: 指揮役の指示で main（#302・#297・#299 込み）を取り込み。`git merge origin/main` は衝突なく完了し、`mise run verify` も緑だったため追加修正は無し
2. 曖昧だった文: 「まず main を取り込んで head 9268d75 で fail している verify/test を緑に戻し」（指揮役の指示）— ローカル・CI とも 9268d75 時点で verify/test は緑（fail は例外承認の pr-size のみ）だったため、「取り込み後に緑を維持する」と解釈してマージを行った
3. 仮定と解釈: 取り込みは merge commit とした（rebase はブランチ書き換え＝force push を要し、agent-gh の署名付け替えはマージを API で表現できず unsigned のまま残る）。round 4 の委託は指揮役の明示承認として止まる条件に抵触しないと解釈した
4. 足りなかった情報・検証手段: 指揮役の言う「9268d75 で fail している verify/test」は GitHub 側の最新 check suite の再実行失敗を指す可能性があるが、`pr checks` の確認では該当する fail が見つからなかった
5. 改善案: `.agents/skills/command-team/SKILL.md` の「レビュー担当と effort」に「止まる条件に達した後の追加ラウンドは、指揮役が明示承認した回数だけ委託できる」を足す（round 4 許可の根拠を明文化）

使ったスキル: `.agents/skills/implement-issue`

### round 8

- 詰まったところ: Codex round 4 の must-fix が「transport は隔離済みであることを型で表す（または明示ポリシーで sandbox 起動）」で、in-process 検証の API と両立する形の設計が要った。
- 曖昧だった表現: 「arbitrary transports」の「arbitrary」（duplex socketpair のようなクライアント側生成の transport は「接続先が任意の外部プロセス」を意味しない）。
- 仮定・解釈: in-process の duplex は client が生成する socketpair で外部プロセス境界ではないため隔離を要さないと解釈し、外部接続のみ `IsolatedTransport` で通す境界にした。in-process 検証向けに `assume_isolated` を残した。
- 足りなかった情報・検証手段: task container 側の隔離実装は本 Issue の範囲外であり、「隔離済み transport のみ」を API で強制する以上の実証はできない（doc と型名で責務を表す）。
- 改善の提案: 隔離を作る側（task container / launcher）が実装された時点で `IsolatedTransport` のコンストラクタをそこだけが呼べる形に閉じる（sealed）と、assume_isolated の濫用余地が消える。
- 対象 head: 58feab8

### round 9

- 詰まったところ: hatai の 3 決定（決1=A・決2=案a・決3=案ii）を反映。決3「_ 拡張で Pelway 承認モデルを載せる」の実装中に環境が劣化し（/home が ext4 エラーで remount-ro、DNS 不通）、mise・hk フック・agent-gh（トークン発行）・push が全て不可になった。
- 曖昧だった表現: 「_ 拡張で … を載せる」— 載せる対象が permission 応答なのか要求なのか、また「限定 MCP」（決2）が空 mcpServers で十分か。Issue 原文は DNS 不通で確認できず、指揮役の転記を根拠にした。
- 仮定・解釈: (a) 証跡は「人の承認」の内容なので `session/request_permission` への応答 `_meta.pelway` に載せると解釈し、`ApprovalEvidence`（decider・expires_at・snapshot_hash・eligibility・outcome_unknown）を `PermissionDecision` で decider から受け取り `RequestPermissionResponse._meta` に写した。(b) 「限定 MCP」は現行の「client が提供する MCP サーバー 0 件・追加ディレクトリ 0 件」が最も厳しい限定であり適合すると解釈した。
- 足りなかった情報・検証手段: Issue 原文（案a・案ii の定義）と `mise run verify`・hk pre-commit・push が環境障害で不可。代替として toolchain 直実行で ACP 試験 11 件 pass・rustfmt・clippy -D warnings を確認。commit/push は復旧待ち。
- 改善の提案: `docs/dev-environment.md` に「mise が rust 解決で network を要し、DNS 不通時は toolchain 直実行で `cargo test`/`rustfmt`/`clippy` が回せる（`~/.rustup/toolchains/<ver>/bin` を PATH 先頭に）」の退避手順を足す。
- 対象 head: 43fcbbf（決3 反映の実装コミット。このメモを含むコミットの親。署名付け替え前は 2ad1f38）

### round 10

- 詰まったところ: Codex round 5 の must-fix 4 件への対応。(1) `assume_isolated` を公開にしたままでは任意コードが隔離境界を偽造できる → `IsolatedTransport` に launcher が結び付けた連携 ID を持たせ、構築はクレート内専用の `bind`（信頼済み launcher 用、後続 Issue で公開面を決める）＋ doc(hidden) の試験用 `bind_for_test` に分けた。(2) `initialize.info.name` は自己申告で認証にならない → 接続前に transport 束縛の連携 ID と grant を照合し、不一致なら接続自体を行わず `IntegrationMismatch`。initialize 名の照合は廃止。(3) キューは件数のみ → 通知の JSON 表現サイズで合計バイトも計量（上限 4 MiB）、超過は通知を捨てて overflow、失敗時は agent へ `session/cancel` を送り close を試行。frame 層は SDK 内部のためデコード前の上限は掛けられず、通知単位の計量で代替。(4) 証跡なしでも許可できた → `AllowOnce` は完全な証跡（指名承認者・期限・snapshot hash・開始資格）必須、`outcome_unknown` では絶対に許可しない不変条件を `answer_permission` に追加（監査 `permission_denied_incomplete_evidence`）。
- 曖昧だった表現: 「信頼済み launcher のみ構築可」— launcher 実装は後続 Issue のため、本番構築経路を `pub(crate)` に閉じる形で「このクレート外の任意コードは構築できない」を担保した。
- 仮定・解釈: 証跡なし・`outcome_unknown` での拒否は once 系の `reject_once` を優先（無ければ cancel）。byte 上限の試験は 5 MiB の単発通知で overflow を決定的に再現。nit 2 件は試験として追加（`outcome_unknown`・証跡なし許可・空操作名の fail closed、計 15 試験）。
- 足りなかった情報・検証手段: JSON-RPC frame 層へのデコード前サイズ制限は SDK が口を出さないため未実施（doc に記載）。ローカル `mise run verify` は sandbox の /home ro-bind による `file-tool` プラグイン試験失敗のみ（コード非起因、CI で確認）。
- 改善の提案: launcher 実装時に `IsolatedTransport::bind` を呼ぶ唯一の本番経路を task container 起動コードとして確立し、`bind_for_test` を `cfg` で試験ビルド限定にする。
- 対象 head: このメモを含むコミットの親（round 5 must-fix 適用の実装コミット）
