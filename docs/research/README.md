# 技術調査

v0.7の技術選定を、既存のOSSと競合の構成と比べた調査です。採否は[02 技術比較と依存採用案](../technical/v0.7/body/02-stack-comparison.md#調査を受けた採否)に反映しています。競合サービスごとの比較は[競合調査](../competitors.md)を参照します。

| 文書 | 内容 |
|---|---|
| [日本語の全文検索](japanese-full-text-search.md) | PGroonga、pg_bigm、Tantivy＋Lindera、ParadeDB、Meilisearch、RAGFlowの比較と、Zulip・Misskey・Synapse・Mastodonの事例 |
| [エージェントの実行と安全](oss-agent-execution-and-safety.md) | サンドボックス、ローカルLLMと日本語モデル、埋め込みモデル、認可エンジン、秘密情報の仲介、MCPとエージェント間プロトコル、プロンプトインジェクション対策 |
| [データと共同作業](oss-data-and-collaboration.md) | ジョブキュー、同期、共同編集、ベクトル検索、日本語OCR（YomiTokuを含む）、改ざん検出の監査ログ、バックアップ |
| [基盤とクライアント](oss-platform-and-client.md) | 認証（Oryを含む）、エンドツーエンド暗号化、音声・ビデオ、通知、観測、クライアントの部品、相互接続 |

各文書の判定は「MVP前に検証」「将来の候補」「見送り」の3つです。記載は公式ドキュメントと公開リポジトリにもとづき、実機での検証はしていません。
