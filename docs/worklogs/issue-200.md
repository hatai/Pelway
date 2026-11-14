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
