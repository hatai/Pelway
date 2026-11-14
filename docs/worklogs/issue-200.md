# Issue #200 の作業ログ

## 実装者メモ（round 1）

Issue #200 のコメント（質問票 R1）に投稿したものを転記:

1. 詰まった箇所: Issue 本文の「実行基盤と利用者とのやり取り」が一文で 2 解釈できる（実行基盤↔本体、実行基盤↔利用者 UI）。評価の全構造が変わるため R1-Q1 として質問票化した
2. 曖昧だった文: 「標準プロトコル（AG-UI、ACP＋MCP）への準拠で『実行基盤を差し替え可能にする』選択肢を評価する」（Issue #200「目的」）—「差し替え可能」の主語が、Pelway ランタイムを差し替えられるようにすることか、外部製ランタイムを受け入れられるようにすることかで分岐する
3. 仮定と解釈: 正本 11 の IntegrationPort は外部 agent を委任先として定義済みと読み、#200 の評価対象は AI 社員の実行基盤そのものの境界と解釈した。調査書 `docs/research/oss-agent-execution-and-safety.md` の AG-UI「見送り」は UI 同期スロット限定の判定と解釈し、本評価での再評価を妨げないと判断した（境界の選択自体は R1-Q1 に委ねた）
4. 足りなかった情報・検証手段: AG-UI/ACP の仕様本文は GitHub（`ag-ui-protocol/ag-ui`、`agentclientprotocol/agent-client-protocol`）経由で到達可能と確認した。protocol のドキュメントサイトは本スレッドのネットワーク制限（GitHub のみ）で届かないため、リポジトリ内の spec・schema を一次資料とする前提で評価する
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「止まる条件」表に、「grill 試行のように本ラウンドの成果物が質問票である割り当てでは、PR を作らず質問票投稿＋報告で止まる」旨の行を足す

使ったスキル: `.agents/skills/grilling/SKILL.md`、`.agents/skills/domain-modeling/SKILL.md`、`implement-issue`

## 実装者メモ（round 2）

1. 詰まった箇所: AG-UI の run 取消経路。`ag-ui-protocol/ag-ui` の `docs/` 全体を探しても interrupt の resume status `cancelled` 以外に取消の仕組みが見つからず、「HTTP 切断で扱う想定（未確認）」として文書に残した
2. 曖昧だった文: 「AI 社員の実行基盤と利用者とのやり取りについて、標準プロトコル（AG-UI、ACP＋MCP）への準拠で『実行基盤を差し替え可能にする』選択肢を評価する」（Issue #200「目的」）。round 1 の質問票 R1-Q1 で境界 (a)（本体↔ランタイム）の採用が確定し、本文はその解釈で評価した
3. 仮定と解釈: ACP の `session/request_permission` を承認要件の「第一級の器あり＝拡張」評価としたが、`allow_always` の記憶をエージェントが所有するモデルは ADR-0002/0004 と衝突するため「options を once 系に制限する運用で回避」と条件付きにした。AG-UI の interrupt は構造的に判断待ちに近いが、固定承認の意味づけは全て metadata 側になると読んだ
4. 足りなかった情報・検証手段: ACP の Streamable HTTP transport と v2 draft の内容（RFD 段階で本評価の対象外とした）。両プロトコルの最新版で表現可否が変わる可能性を未確認事項に残した
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「止まる条件」表に、「grill 試行のように本ラウンドの成果物が質問票である割り当てでは、PR を作らず質問票投稿＋報告で止まる」旨の行を足す（round 1 と同じ指摘。指揮役の回答コメントで改善ループ候補として記録済み）

使ったスキル: `.agents/skills/grilling/SKILL.md`、`.agents/skills/domain-modeling/SKILL.md`、`implement-issue`、`write-pull-request`

## 実装者メモ（round 3）

1. 詰まった箇所: hatai の製品判断「ACP v2 を採用する」（Issue #200 コメント、2026-11-14）は round 2 の評価が出した推奨（当面プロトコル化しない）と逆向き。製品判断は評価の分析範囲外の要素を含めて優先されるため、結論節を採用決定の記録に変え、比較節は採用根拠と却下理由として残す構成に作り直した。分析との関係は「比較節が置いた再評価の引き金第 1 項が成立した」読みを、判断者未確認の解釈と明記して文書に残した
2. 曖昧だった文: 「ACP v2」。round 2 時点の一次資料は `docs/protocol/v1/` だったが、リポジトリには `docs/protocol/v2/` が存在し、migration 文書によれば v2 は全体として draft ラベル。v2 の一次資料を取り直し、評価の ACP 列を v2 仕様で書き換えた（`requires_action` の第一級化、`session/request_permission` の `subject`、elicitation、`session/load` 廃止→`resume`、fs/terminal/mode API 廃止、permission が advisory である旨の security 明記など）
3. 仮定と解釈: 採用要件は「境界の定義」「移行」「未決の実装論点」の 3 分割にした。概念写像（task↔session 等）は確定事項ではなく案とし、v2 draft のリスクは schema pin＋feature flag で扱う方向を移行節に書いた。round 2 で委託したレビューは成果物が変わるため完了前に task_cancel した（レビューは未完了で、失敗としては数えない）
4. 足りなかった情報・検証手段: Rust schema crate（`agent-client-protocol`）の v2 対応状況、Streamable HTTP transport RFD の進捗は GitHub 資料から確認できず未確認事項に残した
5. 改善案: なし（round 1 の指摘は引き続き有効）

使ったスキル: `implement-issue`、`write-pull-request`
