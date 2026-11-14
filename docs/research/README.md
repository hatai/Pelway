# 技術調査

v0.7の技術選定を、既存のOSSと競合の構成と比べた調査です。採否は[02 技術比較と依存採用案](../technical/v0.7/body/02-stack-comparison.md#調査を受けた採否)に反映しています。競合サービスごとの比較は[競合調査](../competitors.md)を参照します。

| 文書 | 内容 |
|---|---|
| [日本語の全文検索](japanese-full-text-search.md) | PGroonga、pg_bigm、Tantivy＋Lindera、ParadeDB、Meilisearch、RAGFlowの比較と、Zulip・Misskey・Synapse・Mastodonの事例 |
| [エージェントの実行と安全](oss-agent-execution-and-safety.md) | サンドボックス、ローカルLLMと日本語モデル、埋め込みモデル、認可エンジン、秘密情報の仲介、MCPとエージェント間プロトコル、プロンプトインジェクション対策 |
| [データと共同作業](oss-data-and-collaboration.md) | ジョブキュー、同期、共同編集、ベクトル検索、日本語OCR（YomiTokuを含む）、改ざん検出の監査ログ、バックアップ |
| [基盤とクライアント](oss-platform-and-client.md) | 認証（Oryを含む）、エンドツーエンド暗号化、音声・ビデオ、通知、観測、クライアントの部品、相互接続 |
| [RustのLLM APIクライアント比較](rust-llm-client-comparison.md) | async-openai、genai、rig-core、openai-api-rs、reqwest自前実装の比較。自動再試行、HTTPクライアント差込、構造化出力、Z.ai・OpenAI・Vertex AIの互換仕様 |
| [Slint Royalty-free License 2.0 の利用条件](slint-royalty-free-license.md) | Slint を無償ライセンスで使う条件（製品種類・attribution・Embedded Systems・API 公開・notice）と Pelway クライアントの現状 |
| [AI社員の実行基盤への標準プロトコル適用の評価](agent-runtime-standard-protocols.md) | AG-UI・ACP＋MCP・独自wireプロトコル・プロトコル化しないの4案で、本体↔ランタイム境界の標準化を評価。要件（承認・段階宣言・監査・ADR 0003）の表現可否と推奨 |

測定報告（実機検証）:

| 文書 | 内容 |
|---|---|
| [サンドボックス隔離の測定](sandbox-isolation-measurements.md) | WSL2+rootless PodmanでのgVisor（runsc）とcrunの比較。起動時間、syscall互換性、資源使用量、egress拒否、KVM可否 |
| [Slint と Tauri＋React のクライアント比較](slint-tauri-client-measurements.md) | Slint 1.18.1 の読み上げ時の ListView の重さ（#3867）、可変高の仮想リスト、#10861 の再現可否、wry の Linux のインライン変換、型生成（ts-rs/specta）、wasm-opt の WASM サイズ |

各文書の判定は「MVP前に検証」「将来の候補」「見送り」の3つです。記載は公式ドキュメントと公開リポジトリにもとづき、実機での検証はしていません（測定報告を除く）。
