# Issue #196 の作業ログ

## 実装者メモ（round 1）

対象 head: 6d0ba7a03b7a2bd17ee5346cf9115fec72fcd618

1. 詰まった箇所: docs/technical.md: Issue 本文は置き場所を「技術仕様の該当節」としか書いていない。#193 の置き場所の了承（PR #409 の指揮役コメント、2026-11-17 12:18 JST。docs/technical.md、前例 #377）と同じく、docs/technical.md §4 への節追加とした。
2. 曖昧だった文: 「検出時の扱い（投稿を止める、AI に汎用的に差し戻す、人へ知らせる）が決まっている → 該当箇所を目視で確認（#192 の却下理由の規則と矛盾しないこと）」（Issue #196 受け入れ条件2）。CircleChat の reply-guard は却下のたびに Bot へ教示的な手がかり（teaching hint）を返すため、「汎用的に差し戻す」と hint が両立するのかに迷った。
3. 仮定と解釈: 差し戻しは hint 無しの汎用の文面のみと解釈した（#192 の「詳細は人間の画面と監査のみ、AI へは汎用のみ」と同じ規則形状で矛盾しないようにし、検出パターンのすり抜け学習を許さない）。選ばなかった解釈: CircleChat 同様の teaching hint 付き差し戻し。秘密については CircleChat のマスクして通す扱い（scrub-and-post）も不採用とした（部分マスクの取りこぼしリスク、fail-closed）。
4. 足りなかった情報・検証手段: CircleChat の reply-guard は現行 main のファイルツリーからは消えており（api/ から server/ への再構成）、`scripts/agent-gh dev api` でコミット 4fac7d8 の api/src/agents/reply-guard.ts とそのテストを取得して分類の実体を確認した。
5. 改善案: .github/ISSUE_TEMPLATE/agent-task.yml の「関連情報」欄の説明に「外部リポジトリを参照するときはファイルパス（可能ならコミット sha も）まで書く」を足す。

使ったスキル: implement-issue, write-pull-request
