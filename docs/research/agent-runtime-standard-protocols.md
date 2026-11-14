# AI社員の実行基盤への標準プロトコル（AG-UI／ACP＋MCP）適用の評価

## 結論（推奨）

推奨は 1 つ：**Pelway 本体と AI 社員ランタイムの境界を独自契約で維持し、現段階で wire プロトコルの標準化を行わない**。当面はプロセス内の型付き契約（trait）とし、ランタイムを別プロセスに切る判断が固まった時点で Pelway 独自の wire 契約として明文化する。理由は「推奨と前提条件」の節に述べる。前提が変わって標準化を選ぶ場合の第一候補は **ACP＋MCP** で、AG-UI は将来の候補として記録する。

## この文書の前提と読み方

- 評価日は 2026-11-14（JST）。一次資料は GitHub 上の公式リポジトリ内の仕様文書に限定した（`ag-ui-protocol/ag-ui` の `docs/`、`agentclientprotocol/agent-client-protocol` の `docs/protocol/v1/`）。ドキュメントサイト独自の補足は参照していない。
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

| 要件 | AG-UI | ACP＋MCP |
|---|---|---|
| 承認 | 拡張点：interrupt が第一級の「実行を止めて人の回答を待つ」構造（`responseSchema`、`expiresAt`、`metadata` を持ち、run を interrupt 結果で終えて resume で再開）。承認者の指名・snapshot hash との結合・再承認の強制・開始資格・outcome_unknown の区別は `metadata` と `resume.payload` の独自意味づけになる | 拡張点：`session/request_permission` が第一級（toolCall の詳細と options を往復）。ただし `allow_always`/`reject_always` は「記憶した決定をエージェントが所有する」モデルで、ADR 0002/0004 の管理者確定・fail-closed と向きが違い、options を once 系に制限する運用で回避する。snapshot hash・承認期限は `toolCall._meta` 等の独自意味づけ |
| 段階宣言 | 拡張点：tool 定義は `RunAgentInput.tools` でクライアントが供給する。段階の概念はなく、`metadata` か `CUSTOM` イベントで運ぶ | 拡張点：`ToolKind`（read/edit/delete/…）は表示用の粗い分類で、認可ではないと仕様が明記。MCP 側は tool annotations が第一級だが、MCP 仕様自身が untrusted と定めており、ADR 0004 の「手がかり表示＋管理者確定」と一致する。宣言レジストリ本体はどちらにもなく Pelway 側に残る |
| 監査 | 表せない：監査は Pelway 側の責任（仕様 03）。境界を越えるのは監査の入力（イベント列・相関 ID）だけ。`metadata` は自由形 | 表せない：同上。ただし `_meta` の `traceparent`/`tracestate`/`baggage` は W3C trace context 用に予約されており、監査の相関 ID に使える |
| ADR 0003 | 表せない：読者集合の概念がプロトコルにない。強制点は投稿経路（Pelway gateway）にあり、ランタイム境界を通す必要がない | 表せない：同上 |

4 要件はすべて Pelway 側に強制点があり、プロトコルが運ぶのは「判断を求める・判断を受け取る」面だけになる。よって「表せない」は即不適合ではない。一方で、準拠してもガバナンスの主要部は自作のままで、標準準拠が買うのはライフサイクルと対話面の枠組みの再利用に限られる。

## 各案の整理

### AG-UI 準拠

- 形：`run(RunAgentInput) -> BaseEvent` のストリーム。`threadId`/`runId`/`parentRunId`、標準のイベント群（lifecycle・text message・tool call・state snapshot/delta・reasoning・subagent）、interrupt→resume の HITL モデル。transport は HTTP POST＋SSE またはバイナリで、仕様は transport 非依存。
- 合う点：`thread`/`run`/`parentRunId` と subagent イベントは Pelway の task/run/子 task 案と対応しやすい。interrupt モデルは判断待ち（awaiting_decision）と構造的に近い。
- 合わない点：run の取消経路が仕様文書で確認できない（interrupt の resume status `cancelled` は「利用者が interrupt を放棄した」の意味で別物。HTTP 切断で中断する想定と推測するが未確認）。イベントの中心は UI 面（text message・state delta・generative UI 系）で、Pelway では本体側が解釈する構成になる。エージェント側は HTTP サーバーを立てる形で、task コンテナ内の短命実行には ACP の stdio 型より構成が重い。

### ACP＋MCP 準拠

- 形：JSON-RPC 2.0 over stdio（v1 で定義された唯一の transport。Streamable HTTP は RFD 段階）。session/new・load・resume・close・list・delete のライフサイクル、prompt turn＋`session/update` 通知、tool_call、plan、elicitation、`session/request_permission`、`session/cancel`。拡張は `_meta`・`_` 始まりの拡張メソッド・capability 広告で仕様化されている。agent の tool 面は session ごとに設定する MCP サーバーで賄う構成が仕様に組み込まれている。
- 合う点：session→prompt→update→stopReason の流れは run モデルに近い。`session/cancel` は「要求であり停止の保証ではない」意味づけで、暫定採用の取消 P 方針と形が似る。Rust の schema crate（`agent-client-protocol`）がある。MCP は既に Pelway の条件付き依存候補（rmcp）で、tool 面を MCP に寄せれば接続面が二重にならない。Buzz が ACP で外部 agent を部屋のメンバーにする先行例。
- 合わない点：session/prompt の turn モデルで、task/run/step/operation の 4 層・logical operation・冪等 key・outcome_unknown は `_` 拡張として自作になる。エージェントが権限の記憶（allow_always）と mode の自己提示を持つモデルで、管理者確定型の ADR 0002/0004 と向きが違う。stdio 以外の transport は仕様化途中。

### 独自 wire プロトコル

- 合う点：9 状態・ActionSnapshot・logical operation・outcome_unknown・引継ぎ欄・取消の集約表示をそのまま型にできる。v0.7 の内部契約（03 の runner/gateway、11 の IntegrationPort 能力定義）を写す形になる。
- 合わない点：スキーマ・版管理・シリアライズを自前で持つ。相互運用はゼロ。内部契約を wire 形式に翻訳する作業量は小さくない。

### プロトコル化しない（プロセス内 trait 契約）

- 合う点：実装量が最小。Pelway 製ランタイムだけが対象なら「差し替え」は trait 実装の差し替えで足り、wire 化の変換誤りが出ない。
- 合わない点：Pelway にリンクしない別プロセス・別言語のランタイムは接続できない。ランタイムを task コンテナ内の別プロセスにする判断が固まった時点で wire 契約が別途必要になる。

## 比較

| 観点 | AG-UI 準拠 | ACP＋MCP 準拠 | 独自 wire | プロトコル化しない |
|---|---|---|---|---|
| 相互運用 | AG-UI を話す agent/クライアントと理論上接続できる。外部接続の実需要は未確認 | ACP を話す agent/クライアントと理論上接続できる。Buzz の先行例あり | なし | なし |
| 実装量 | エージェント側 HTTP サーバー＋イベント生成＋独自意味づけの二重管理 | JSON-RPC over stdio＋`_` 拡張の独自意味づけ。schema crate あり | 契約の明文化＋シリアライズ＋版管理 | 最小（既存 trait の維持） |
| 将来の外部エージェント受け入れ | AG-UI を話す agent を受けられる（ガバナンスは Pelway 側実装が前提） | ACP を話す agent を受けられる（同上） | 受けられない（外部 agent は 11 の IntegrationPort/Adapter 経路が担当） | 同左 |
| Pelway 要件との適合 | 承認＝拡張、段階＝拡張、監査・ADR-0003＝Pelway 側 | 承認＝拡張（第一級の器あり）、段階＝拡張（MCP annotations と一致）、監査・ADR-0003＝Pelway 側 | 完全一致 | 完全一致（要件が境界を越えない） |

## 推奨と前提条件

**独自契約を維持する。当面はプロセス内 trait 契約とし、ランタイムを別プロセスに切る判断が固まった時点で Pelway 独自の wire 契約として明文化する。標準プロトコル準拠は行わない。** 理由：

1. 要件 4 点はすべて Pelway 側に強制点があり、標準準拠でもその実装は消えない。標準が買うのは対話面とライフサイクルの枠組みだけで、差し替え可能性の中核（承認・段階・監査・範囲の強制）は protocol 非依存である。
2. 独自意味づけが支配的な準拠は「独自プロトコルに標準の外被を着せたもの」になる。ACP の permission モデル（エージェントが所有する allow_always の記憶、自己提示の mode）は ADR 0002/0004 と向きが違い、仕様上の機能を運用で囲う必要が増える。
3. 「将来の外部エージェント受け入れ」は正本 11 の IntegrationPort/Adapter 経路が担当する決定済みの論点で、標準準拠の加点になる製品目標は現状ない（R1-Q4）。

推奨が覆る前提（再評価の引き金）：

- 「第三者のランタイムを AI 社員の基盤として受け入れる」が製品目標になった → **ACP＋MCP** を第一候補として再評価する（session/permission/cancel の面が最も近く、Buzz の先行例と Rust crate がある）。
- ランタイムを task コンテナ内の別プロセスとして切る実装判断が固まった → 独自 wire 契約か ACP＋拡張かを実装量で再比較する。
- AG-UI に取消・承認の第一級の仕組みが加わった → AG-UI を再評価する。

## 未確認事項

- AG-UI に run の取消経路があるか（仕様文書では見つからず、HTTP 切断で扱う想定と推測）。
- ACP の Streamable HTTP transport の仕様化状況（RFD 段階。v2 draft の告知あり）。
- ACP v2・AG-UI 1.0 への移行で本評価の表現可否が変わるか。
- OpenBot・Buzz が各プロトコルのどこまでを実装しているか（本評価はプロトコル仕様のみを対象とし、実装の準拠度は見ていない）。

## 出典

- [AG-UI：ag_ui.md](https://github.com/ag-ui-protocol/ag-ui/blob/main/docs/ag_ui.md)
- [AG-UI：Core architecture](https://github.com/ag-ui-protocol/ag-ui/blob/main/docs/concepts/architecture.mdx)
- [AG-UI：Events](https://github.com/ag-ui-protocol/ag-ui/blob/main/docs/concepts/events.mdx)
- [AG-UI：Interrupts](https://github.com/ag-ui-protocol/ag-ui/blob/main/docs/concepts/interrupts.mdx)
- [AG-UI：Core types](https://github.com/ag-ui-protocol/ag-ui/blob/main/docs/sdk/js/core/types.mdx)
- [ACP：Overview](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/docs/protocol/v1/overview.mdx)
- [ACP：Session Setup](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/docs/protocol/v1/session-setup.mdx)
- [ACP：Prompt Turn（stop reasons・cancellation）](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/docs/protocol/v1/prompt-turn.mdx)
- [ACP：Tool Calls（request_permission）](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/docs/protocol/v1/tool-calls.mdx)
- [ACP：Session Modes](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/docs/protocol/v1/session-modes.mdx)
- [ACP：Extensibility](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/docs/protocol/v1/extensibility.mdx)
- [ACP：Transports](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/docs/protocol/v1/transports.mdx)
- Pelway 正本：`docs/technical/v0.7/body/03-ai-bot-runtime.md`、`docs/technical/v0.7/body/11-integrations.md`、`docs/adr/0002`〜`0004`、`0007`
- 既存調査：`docs/research/oss-agent-execution-and-safety.md` §5（AG-UI の UI スロットでの見送り、ACP の将来の候補）、`docs/competitors/openbot.md`、`docs/competitors/buzz.md`
