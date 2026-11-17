# Issue #196 の作業ログ

## 実装者メモ（round 1）

対象 head: 6d0ba7a03b7a2bd17ee5346cf9115fec72fcd618

1. 詰まった箇所: docs/technical.md: Issue 本文は置き場所を「技術仕様の該当節」としか書いていない。#193 の置き場所の了承（PR #409 の指揮役コメント、2026-11-17 12:18 JST。docs/technical.md、前例 #377）と同じく、docs/technical.md §4 への節追加とした。
2. 曖昧だった文: 「検出時の扱い（投稿を止める、AI に汎用的に差し戻す、人へ知らせる）が決まっている → 該当箇所を目視で確認（#192 の却下理由の規則と矛盾しないこと）」（Issue #196 受け入れ条件2）。CircleChat の reply-guard は却下のたびに Bot へ教示的な手がかり（teaching hint）を返すため、「汎用的に差し戻す」と hint が両立するのかに迷った。
3. 仮定と解釈: 差し戻しは hint 無しの汎用の文面のみと解釈した（#192 の「詳細は人間の画面と監査のみ、AI へは汎用のみ」と同じ規則形状で矛盾しないようにし、検出パターンのすり抜け学習を許さない）。選ばなかった解釈: CircleChat 同様の teaching hint 付き差し戻し。秘密については CircleChat のマスクして通す扱い（scrub-and-post）も不採用とした（部分マスクの取りこぼしリスク、fail-closed）。
4. 足りなかった情報・検証手段: CircleChat の reply-guard は現行 main のファイルツリーからは消えており（api/ から server/ への再構成）、`scripts/agent-gh dev api` でコミット 4fac7d8 の api/src/agents/reply-guard.ts とそのテストを取得して分類の実体を確認した。
5. 改善案: .github/ISSUE_TEMPLATE/agent-task.yml の「関連情報」欄の説明に「外部リポジトリを参照するときはファイルパス（可能ならコミット sha も）まで書く」を足す。

使ったスキル: implement-issue, write-pull-request

## 実装者メモ（round 2）

対象 head: 3491861526fa8330f555b2f0af09688b7f76d074

1. 詰まった箇所: docs/technical.md の対象パターンと緩和規則: round 1 のレビュー（Devin・request-changes）で「コード・差分・実行スクリプトの大量貼り付け」を漏洩系に置いたまま「形式の異常系のうちコード貼り付けを緩和可」と書いた分類矛盾を指摘された。
2. 曖昧だった文: 「形式の異常系のうちコード・差分・ログの大量貼り付けの観点だけを、チャンネル設定として緩和・除外できます」（round 1 の docs/technical.md「コードやログを扱う仕事」節。どの分類の項目を指すのかが分類の一覧と噛み合っていなかった）。
3. 仮定と解釈: Devin の提示した2案のうち「コード・差分・ログの大量貼り付けを形式の異常系へ移し、漏洩系は実行スクリプトの貼り付けに限定する」方を採った。仮定: 実行スクリプト（curl・API を叩くスクリプト）は資格情報や内部の宛先を含み得るので常時適用が要るが、通常のコード・差分・ログのダンプは形式の問題で、開発チャンネルでは正当な仕事になり得る。選ばなかった解釈: コード貼り付け全体を漏洩系に留め緩和不可にする案（誤検出で正当な仕事を止めるため）。
4. 足りなかった情報・検証手段: 分類の境界（実行スクリプトと通常のコードの区別）を一文で書く根拠となる CircleChat 側の実装。api/src/agents/reply-guard.ts では API_SCRIPT_RE（CC_BOT_TOKEN・urllib 等）と looksLikeCodeDiffDump が別パターンであることを再確認した。
5. 改善案: .agents/skills/write-issue/SKILL.md の「採用する方式」に「出典の分類を複数の軸（深刻度・緩和可否など）に割り振るときは、割り振りの軸も Issue 本文に書く」を足す。

使ったスキル: implement-issue, write-pull-request

## 実装者メモ（round 3）

対象 head: 998234df7909167f3beb2dda645c347cf07955df

1. 詰まった箇所: scripts/tests/reply-guard-docs.test.sh: round 2 のレビュー（Codex・request-changes）で「自然言語の受け入れ条件を grep の必須語句の有無だけで判定するのは AGENTS.md『LLM judges』違反」と指摘された。
2. 曖昧だった文: 「never by scoring the text with keyword lists or regular expressions. Deterministic assertions are limited to observable effects and structured output contracts.」（AGENTS.md「LLM judges」）。「文言の存在」が観測可能な効果と数えられるか、意味の評価に当たるかに迷った。
3. 仮定と解釈: must-fix の提示した第2案（スクリプトを受け入れ条件テストと扱わない形）を採り、冒頭コメントを「必須文言の固定（wording pin）であり、意味や整合性は評価しない。意味の確認は Issue の受け入れ条件どおり目視」と書き換えた。選ばなかった解釈: rubric 付き LLM judge の追加（CI が LLM 呼び出しに依存する形になり、evals/agent-team の枠外）。
4. 足りなかった情報・検証手段: 同種の grep 文言テスト（review-isolation-docs.test.sh（PR #409・2026-11-17 マージ）、egress-host-docs.test.sh（PR #377））が LLM judges 規則（2026-11-16 導入）の後に同じ形で承認・マージされている事実と、今回の指摘の関係を正本側で確定する場。本次では指摘のとおり直し、不一致の指摘に留める（AGENTS.md「Code over docs」の報告義務）。
5. 改善案: .agents/skills/implement-issue/SKILL.md の手順 4 に「文書の受け入れ条件のテストは wording pin（文言の存在だけの決定的検査）と位置づけ、意味の確認は目視とレビューで行う旨を PR 本文に書く」を足す。

使ったスキル: implement-issue, write-pull-request
