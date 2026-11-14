# AI社員の実行基盤への標準プロトコル適用の評価と ACP v2 採用方針

## 結論（採用決定）

**ACP v2 を採用する**。これは評価の結果としてここで選んだものではなく、hatai の製品判断として 2026-11-14 に Issue #200 のコメントへ記録された決定である。本節以降の評価・比較は、採用の根拠（標準プロトコルとの適合性の記録）と、独自 wire プロトコル・プロトコル化しない案を採らなかった理由として残す。採用に必要な要件（境界の定義・移行・未決の実装論点）は「採用に必要な要件」の節に集約する。

適用する境界は Pelway 本体（server/gateway）↔ AI 社員ランタイムで、Pelway 本体が ACP の Client、AI 社員ランタイムが Agent の役割を取る。v2 は本稿の確認時点で全体として draft ラベルであり、採用要件の「移行」で扱うリスクがある。

## この文書の前提と読み方

- 評価日は 2026-11-14（JST）。一次資料は GitHub 上の公式リポジトリ内の仕様文書に限定した（`ag-ui-protocol/ag-ui` の `docs/`、`agentclientprotocol/agent-client-protocol` の `docs/protocol/v2/`）。v1 との差分は同リポジトリの `docs/protocol/v2/migration.mdx` に基づく。ドキュメントサイト独自の補足は参照していない。
- 「確認」は仕様文書に記載があるという意味。「未確認」は今回の資料で裏付けられなかった項目で、機能がないという意味ではない。
- 評価対象の境界は、質問票 R1-Q1 の確定回答に従い **Pelway 本体（server/gateway）と AI 社員ランタイムの間** とする。ランタイム↔利用者 UI の境界は A2（独自 UI）・A4（独自同期）の決定と衝突するため主対象から除外した。
- 表現可否の採点は質問票 R1-Q2 の確定回答に従い 3 段とする：第一級（仕様明記の機能で表せる）／拡張点（`_meta`・カスタムイベント等で運べる）／表せない（Pelway 側に残る）。
- 比較の土台は質問票 R1-Q3 の確定回答に従い 4 案：AG-UI 準拠・ACP＋MCP 準拠・独自 wire プロトコル・プロトコル化しない。「将来の外部エージェント受け入れ」は比較軸のみで、標準プロトコル側の加点としない（R1-Q4）。

## 評価対象の境界

(a) Pelway 本体とランタイムの間を採用する。ACP の本来のスロット（クライアント↔エージェント）であり、OpenBot も「任意の AG-UI エンドポイントを Bot として登録する」形で同じ境界に AG-UI を置いている。(b) ランタイム↔利用者 UI は AG-UI の本来のスロットだが、A2（TypeScript/Octane/DOM）と A4（seq/cursor/snapshot の独自同期）が決定済みで、置き換える根拠がないため除外する。(c) の両面案は (b) を含むため同じ理由で除外する。

## Pelway 側の要件（正本から）

- **承認**：ActionSnapshot の hash と version への指名承認者による期限付き承認。payload が変われば再承認。承認の決定は server 側で行い、一回限りの開始資格を gateway が消費して外部 I/O を一回試みる。結果が確定しない操作は outcome_unknown として保持する（ADR 0002、仕様 03「固定承認と外部副作用」）。
- **段階宣言**：操作の 4 段階は連携ごとの宣言と管理者の確定で決める。MCP annotations は手がかり表示のみ。段階が決まっていない操作は止める（ADR 0004）。
- **監査**：追記型の監査ログと hash 鎖、署名付きチェックポイント。actor・承認 snapshot・policy 版・操作・結果・時刻・相関 ID を保存する（ADR 0007、仕様 03「監査とローカル推論」）。
- **ADR 0003**：AI の実行は依頼チャンネルの読者に結び付け、より広い範囲への投稿は投稿直前の範囲検査を経て承認を要する。強制点は投稿経路に置く。

## 要件の表現可否

| 要件 | AG-UI | ACP＋MCP（v2） |
|---|---|---|
| 承認 | 拡張点：interrupt が第一級の「実行を止めて人の回答を待つ」構造（`responseSchema`、`expiresAt`、`metadata` を持ち、run を interrupt 結果で終えて resume で再開）。承認者の指名・snapshot hash との結合・再承認の強制・開始資格・outcome_unknown の区別は `metadata` と `resume.payload` の独自意味づけになる | 拡張点：`session/request_permission` が第一級（v2 で `title` 必須・`subject` で toolCallId/terminalId への構造化参照が追加）。前景状態 `requires_action` が v2 で第一級化され「判断待ち」を仕様上の状態として表せる。`elicitation/create`（form/URL mode）も構造化入力の第一級器。ただし v2 の security 文書が「permission request は advisory で、問い合わせずに Agent が実行した操作を Client は防げない」と明記しており、承認の強制は Pelway 側に必須で残る。`allow_always`/`reject_always` の記憶はエージェント所有のモデルで ADR 0002/0004 と向きが違い、options を once 系に制限する等の調整が要る。指名承認者・期限・snapshot hash・開始資格・outcome_unknown は `subject`/`_meta`/`_` 拡張の独自意味づけ、または Pelway 側完結 |
| 段階宣言 | 拡張点：tool 定義は `RunAgentInput.tools` でクライアントが供給する。段階の概念はなく、`metadata` か `CUSTOM` イベントで運ぶ | 拡張点：`ToolKind`（read/edit/delete/…、カスタムは `_` 始まり）は表示用の粗い分類で認可ではないと仕様が明記。段階の運搬は `tool_call_update._meta`・permission の `subject`・session config option・MCP annotations が候補。MCP 仕様自身が annotations を untrusted と定めており、ADR 0004 の「手がかり表示＋管理者確定」と一致する。宣言レジストリ本体は Pelway 側に残る |
| 監査 | 表せない：監査は Pelway 側の責任（仕様 03）。境界を越えるのは監査の入力（イベント列・相関 ID）だけ。`metadata` は自由形 | 表せない：同上。ただし `_meta` の `traceparent`/`tracestate`/`baggage` は W3C trace context 用に予約されており、監査の相関 ID に使える |
| ADR 0003 | 表せない：読者集合の概念がプロトコルにない。強制点は投稿経路（Pelway gateway）にあり、ランタイム境界を通す必要がない | 表せない：同上 |

4 要件はすべて Pelway 側に強制点があり、プロトコルが運ぶのは「判断を求める・判断を受け取る」面だけになる。よって「表せない」は即不適合ではない。v2 の security 文書は permission の advisory 性と仲介者（intermediary）モデルを明記しており、「ガバナンスの強制点を Pelway 側に置いたまま ACP を通す」設計と矛盾しない。一方で、準拠してもガバナンスの主要部は自作のままで、標準準拠が買うのはライフサイクルと対話面の枠組みの再利用に限られる。

## 各案の整理

### AG-UI 準拠

- 形：`run(RunAgentInput) -> BaseEvent` のストリーム。`threadId`/`runId`/`parentRunId`、標準のイベント群（lifecycle・text message・tool call・state snapshot/delta・reasoning・subagent）、interrupt→resume の HITL モデル。transport は HTTP POST＋SSE またはバイナリで、仕様は transport 非依存。
- 合う点：`thread`/`run`/`parentRunId` と subagent イベントは Pelway の task/run/子 task 案と対応しやすい。interrupt モデルは判断待ち（awaiting_decision）と構造的に近い。
- 合わない点：run の取消経路が仕様文書で確認できない（interrupt の resume status `cancelled` は「利用者が interrupt を放棄した」の意味で別物。HTTP 切断で中断する想定と推測するが未確認）。イベントの中心は UI 面（text message・state delta・generative UI 系）で、Pelway では本体側が解釈する構成になる。エージェント側は HTTP サーバーを立てる形で、task コンテナ内の短命実行には ACP の stdio 型より構成が重い。

### ACP＋MCP 準拠（v2）

- 形：JSON-RPC 2.0。transport は stdio（クライアントがエージェントを子プロセスとして起動し改行区切りで往復）が唯一の安定定義。Streamable HTTP は RFD 段階、custom transport は仕様上許容される。ライフサイクルは `session/new`・`session/resume`（`replayFrom` 付き）・`session/list`・`session/close`（いずれも v2 で必須）・`session/delete`（optional capability）。prompt 応答は受理確認に変わり、進捗と完了は `state_update`（`running`/`requires_action`/`idle`＋`stopReason`）の upsert 通知で届く。tool 呼び出しは `tool_call_update` の upsert。`session/request_permission`、`elicitation/create`（form/URL mode）、`session/cancel`・`$/cancel_request`。拡張は `_meta`（upsert の metadata patch を含む）・`_` 始まりの拡張メソッド・capability 広告・未知 enum 値の保持で仕様化されている。v1 の `session/load`・fs/terminal API・session modes は廃止され、mode は config option 化された。agent の tool 面は `session/new`・`session/resume` で渡すクライアント提供の MCP サーバーで賄う構成が仕様に組み込まれている。
- 合う点：`requires_action` が承認待ちの前景状態に第一級対応する（v1 にはなかった改善）。`session/resume`/`list`/`close` が task の再開・一覧・中断と対応しやすい。stdio が task コンテナ内でランタイムを子プロセス起動する形と整合する。security 文書が permission の advisory 性と仲介者モデルを明記し、Pelway 側に強制点を残す設計と矛盾しない。MCP は既に Pelway の条件付き依存候補（rmcp）で、tool 面を MCP に寄せれば接続面が二重にならない。Buzz が ACP で外部 agent を部屋のメンバーにする先行例。Rust の schema crate（`agent-client-protocol`）がある（v2 対応状況は未確認）。
- 合わない点：v2 は全体として draft ラベルで、仕様が変わりうる（移行節で扱う）。task/run/step/operation の 4 層・logical operation・冪等 key・outcome_unknown は `_` 拡張として自作になる。permission は advisory で強制できないため、承認の強制点は必ず Pelway 側に置く設計が要る。stdio 以外の transport は未仕様化。

### 独自 wire プロトコル

- 合う点：9 状態・ActionSnapshot・logical operation・outcome_unknown・引継ぎ欄・取消の集約表示をそのまま型にできる。v0.7 の内部契約（03 の runner/gateway、11 の IntegrationPort 能力定義）を写す形になる。
- 合わない点：スキーマ・版管理・シリアライズを自前で持つ。相互運用はゼロ。内部契約を wire 形式に翻訳する作業量は小さくない。

### プロトコル化しない（プロセス内 trait 契約）

- 合う点：実装量が最小。Pelway 製ランタイムだけが対象なら「差し替え」は trait 実装の差し替えで足り、wire 化の変換誤りが出ない。
- 合わない点：Pelway にリンクしない別プロセス・別言語のランタイムは接続できない。ランタイムを task コンテナ内の別プロセスにする判断が固まった時点で wire 契約が別途必要になる。

## 比較

| 観点 | AG-UI 準拠 | ACP＋MCP 準拠（v2） | 独自 wire | プロトコル化しない |
|---|---|---|---|---|
| 相互運用 | AG-UI を話す agent/クライアントと理論上接続できる。外部接続の実需要は未確認 | ACP を話す agent/クライアントと理論上接続できる。Buzz の先行例あり | なし | なし |
| 実装量 | エージェント側 HTTP サーバー＋イベント生成＋独自意味づけの二重管理 | JSON-RPC over stdio＋`_` 拡張の独自意味づけ。schema crate あり（v2 対応は未確認） | 契約の明文化＋シリアライズ＋版管理 | 最小（既存 trait の維持） |
| 将来の外部エージェント受け入れ | AG-UI を話す agent を受けられる（ガバナンスは Pelway 側実装が前提） | ACP を話す agent を受けられる（同上） | 受けられない（外部 agent は 11 の IntegrationPort/Adapter 経路が担当） | 同左 |
| Pelway 要件との適合 | 承認＝拡張、段階＝拡張、監査・ADR-0003＝Pelway 側 | 承認＝拡張（第一級の器＋`requires_action`。advisory のため強制は Pelway 側）、段階＝拡張（MCP annotations と一致）、監査・ADR-0003＝Pelway 側 | 完全一致 | 完全一致（要件が境界を越えない） |

## 採用決定の位置づけ

- 決定：hatai の製品判断「ACP v2 を採用する」。Issue #200 のコメントに 2026-11-14 記録。比較節は採用の根拠と却下理由の記録として残す指示が同コメントで確認されている。
- 分析上の所見との関係：上の比較だけなら「当面 wire プロトコル化しない」が最有力だった（要件の強制点が全て Pelway 側にあり、標準準拠が買うのは対話面の枠組みに限られるため）。比較の時点で「標準化を選ぶ場合の第一候補」は ACP＋MCP と記録しており、「第三者のランタイムを AI 社員の基盤として受け入れる」が製品目標になった場合の再評価の引き金を置いていた。今回の製品判断はこの前提を成立させたものと読む（評価者の解釈であり、判断者の確認は取っていない）。
- 採用を前提に変わる設計上の論点は次節に集約する。採用決定の正式な記録形式（ADR 化の要否）は未決の実装論点に含める。

## 採用に必要な要件

### 境界の定義（要確定）

- 役割：Pelway 本体（server/gateway）＝ACP の Client、AI 社員ランタイム＝Agent。ACP 本来のスロットで、Buzz の先行例と同じ配置。
- 概念写像（案。採用設計で確定する）：
  - task ↔ `session`（`session/new`・`resume`・`list`・`close`・`delete`）
  - run/依頼 ↔ `session/prompt`（受理と完了が分離した v2 のモデル）
  - operation/step ↔ `tool_call_update` の `toolCallId` 単位
  - 承認待ち ↔ 前景状態 `requires_action` ＋ `session/request_permission`（または `elicitation/create`）
  - 9 状態 ↔ `state_update`（`running`/`requires_action`/`idle`＋`stopReason`）と Pelway 側の状態機械の写像表が要る
- transport：stdio（ランタイムを子プロセス起動）は task コンテナ内の短命実行と整合する。リモートランタイム要件があるなら Streamable HTTP は RFD 段階のため custom transport 検討か仕様化待ちになる — 要決定。
- 初期化と認証：`initialize` で `protocolVersion: 2` をネゴシエートし、capability（elicitation・session.delete・systemPrompt 等）を確定する。`auth/login` と Pelway 自身の認証の関係は未決。
- gateway の位置：ACP の security 文書は Client↔Agent 間の仲介者（intermediary）を仕様上想定している。Pelway gateway をその仲介位置に置くか、Client 役を gateway 自身が兼ねるかは未決。

### 移行

- v2 は全体として draft ラベル。`schema/v2/schema.json`（安定 baseline）に固定し、`schema.unstable.json` の experimental 機能は採用しない。ACP 側の実装は feature flag で囲い、spec 追従の担当と手順を決める。
- v1 併存：Pelway 主導の新規接続は `protocolVersion: 2` 要求でよいが、v1-only の外部ランタイムを将来受け入れるかは未決（ACP の migration 文書は v1/v2 併存サポートを推奨している）。
- 段階導入（案）：内部 trait 契約の上に ACP adapter 層を置く → Pelway 製ランタイムを ACP Agent 化して内部で検証 → 外部ランタイムの受け入れ可否を別途判断。
- `docs/research/oss-agent-execution-and-safety.md` §5 が置いた「ACP＝将来の候補」の位置づけは今回の採用で更新される。採用決定を `docs/adr/` に記録するかは未決。

### 未決の実装論点

1. **承認意味論の設計**：`request_permission` の options を once 系に限定して `allow_always` を運用で禁止するか、`_` 拡張で Pelway の承認モデル（指名承認者・期限・snapshot hash・開始資格・outcome_unknown）を載せるか、`elicitation/create` の URL mode で帯外の承認フロー（既存の承認 UI）に逃がすか。permission が advisory である以上、承認なしに外部 I/O を試みない強制は gateway 側に必須で残る。
2. **段階宣言の運搬経路**：`tool_call_update._meta`、permission の `subject`、session config option、MCP annotations のどれで tier を運ぶか。宣言レジストリと管理者確定・fail-closed は Pelway 側。
3. **監査の運搬**：署名付きチェックポイント・相関 ID を `_meta`（`traceparent` 等の予約キー含む）で運ぶか、Pelway 側チャネルに留めるか。
4. **ADR 0003**：プロトコル外。投稿経路の範囲検査で継続し、ランタイム境界の設計には影響しない。
5. **状態写像**：Pelway の 9 状態 ↔ `running`/`requires_action`/`idle`＋`stopReason`（`end_turn`/`cancelled`/`error`/`refusal`/`max_tokens`/`max_turn_requests`）の写像表。`idle` の stopReason 欠落・未知値と outcome_unknown の関係。
6. **取消**：`session/cancel`・`$/cancel_request`・未挿入 prompt の `-32800` と、Pelway の取消・タイムアウト意味づけの対応。
7. **transport の確定**：stdio 確定か、リモートランタイム要件の有無。
8. **MCP サーバー構成**：`session/new` のクライアント提供 `mcpServers` を使うか、ランタイム内蔵の MCP 構成にするか（段階宣言・依存度に影響）。
9. **永続化**：session ↔ task の永続化と `replayFrom`・`session/list`・`session/delete` の Pelway task 一覧との関係。
10. **ライブラリ**：Rust schema crate（`agent-client-protocol`）の v2 対応状況（未確認）。
11. **記録形式**：採用決定を `docs/adr/` に記録するか。

## 未確認事項

- ACP v2 の安定化時期と `schema.unstable.json` の内容。
- ACP の Streamable HTTP transport の仕様化状況（RFD 段階）。
- Rust schema crate（`agent-client-protocol`）の v2 対応状況。
- AG-UI に run の取消経路があるか（仕様文書では見つからず、HTTP 切断で扱う想定と推測）。
- OpenBot・Buzz が各プロトコルのどこまでを実装しているか（本評価はプロトコル仕様のみを対象とし、実装の準拠度は見ていない）。

## 出典

- [ACP v2：Overview](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/docs/protocol/v2/overview.mdx)
- [ACP v2：Session Setup](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/docs/protocol/v2/session-setup.mdx)
- [ACP v2：Prompt Lifecycle（state_update・stop reasons・cancellation）](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/docs/protocol/v2/prompt-lifecycle.mdx)
- [ACP v2：Tool Calls（request_permission）](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/docs/protocol/v2/tool-calls.mdx)
- [ACP v2：Elicitation](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/docs/protocol/v2/elicitation.mdx)
- [ACP v2：Session Config Options](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/docs/protocol/v2/session-config-options.mdx)
- [ACP v2：Session List](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/docs/protocol/v2/session-list.mdx)
- [ACP v2：Authentication](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/docs/protocol/v2/authentication.mdx)
- [ACP v2：Extensibility](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/docs/protocol/v2/extensibility.mdx)
- [ACP v2：Security Considerations](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/docs/protocol/v2/security.mdx)
- [ACP v2：Transports](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/docs/protocol/v2/transports.mdx)
- [ACP v2：Migrating from v1](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/docs/protocol/v2/migration.mdx)
- [AG-UI：ag_ui.md](https://github.com/ag-ui-protocol/ag-ui/blob/main/docs/ag_ui.md)
- [AG-UI：Core architecture](https://github.com/ag-ui-protocol/ag-ui/blob/main/docs/concepts/architecture.mdx)
- [AG-UI：Events](https://github.com/ag-ui-protocol/ag-ui/blob/main/docs/concepts/events.mdx)
- [AG-UI：Interrupts](https://github.com/ag-ui-protocol/ag-ui/blob/main/docs/concepts/interrupts.mdx)
- [AG-UI：Core types](https://github.com/ag-ui-protocol/ag-ui/blob/main/docs/sdk/js/core/types.mdx)
- Pelway 正本：`docs/technical/v0.7/body/03-ai-bot-runtime.md`、`docs/technical/v0.7/body/11-integrations.md`、`docs/adr/0002`〜`0004`、`0007`
- 製品判断：Issue #200 のコメント（2026-11-14、hatai の判断「ACP v2 を採用する」）
- 既存調査：`docs/research/oss-agent-execution-and-safety.md` §5（AG-UI の UI スロットでの見送り、ACP の将来の候補）、`docs/competitors/openbot.md`、`docs/competitors/buzz.md`
