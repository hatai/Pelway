# Issue #197 の作業ログ

## 実装者メモ（round 1）

対象 head: fa7d06aececd0c446fa4898c30a5d9516254fb40

1. 詰まった箇所: Issue #197 の関連情報「tashfeenahmed/circlechat の Model routing and real usage」: 現行リポジトリに見つからない（2026-11-16 の 2.0 で README が全面書き換え）。Issue 起票時点（2026-11-12、commit ca7dada）の README と docs/automation-platform.md を取得して確認し、出典として ca7dada 時点へのリンクを技術.md §9.8 に貼った。
2. 曖昧だった文: 「技術.md §9 または v0.7 の該当節を目視で確認」（Issue #197 の受け入れ条件）——prose の追加は製品テストで確かめられないため、scripts/tests/egress-host-docs.test.sh（Issue #205）の先例に倣い、必須文言の grep による構造確認に読み替えた。
3. 仮定と解釈: ルーティングの「階層」は §5 の3区分（振り分けや要約向け／本作業向け／難しい作業向け）と解釈した。CircleChat の4階層（economy/balanced/frontier/advisor）の英名を Pelway に導入する解釈は取らなかった（§5 の既存の振り分けが階層に対応するため）。実測の受け取り手は LLMゲートウェイ（§9.7・ADR 0008）と前提した。
4. 足りなかった情報・検証手段: `mise run verify` の scripts/tests/conductor-watch-ensure.test.sh が環境によりフレークする（本変更と無関係。クリーンなツリーでも run1 失敗・run2 成功を確認済み）。再実行で全体が通ったことを確認した。
5. 改善案: `.github/ISSUE_TEMPLATE/agent-task.yml` の「関連情報」の description に「外部リンクは確認日かピン留めした版（commit 等）も併記する（本 Issue では CircleChat の README が 2.0 で書き換えられ、関連情報の記述が現行版から消えていた）」を足す。

使ったスキル: implement-issue
