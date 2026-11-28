# 技術調査

v0.7の技術選定を、既存のOSSと競合の構成と比べた調査です。採否は[02 技術比較と依存採用案](../technical/v0.7/body/02-stack-comparison.md#調査を受けた採否)に反映しています。競合サービスごとの比較は[競合調査](../competitors.md)を参照します。

| 文書 | 内容 |
|---|---|
| [日本語の全文検索](japanese-full-text-search.md) | PGroonga、pg_bigm、Tantivy＋Lindera、ParadeDB、Meilisearch、RAGFlowの比較と、Zulip・Misskey・Synapse・Mastodonの事例 |
| [エージェントの実行と安全](oss-agent-execution-and-safety.md) | サンドボックス、ローカルLLMと日本語モデル、埋め込みモデル、認可エンジン、秘密情報の仲介、MCPとエージェント間プロトコル、プロンプトインジェクション対策 |
| [データと共同作業](oss-data-and-collaboration.md) | ジョブキュー、同期、共同編集、ベクトル検索、日本語OCR（YomiTokuを含む）、改ざん検出の監査ログ、バックアップ |
| [基盤とクライアント](oss-platform-and-client.md) | 認証（Oryを含む）、エンドツーエンド暗号化、音声・ビデオ、通知、観測、クライアントの部品、相互接続 |
| [RustのLLM APIクライアント比較](rust-llm-client-comparison.md) | async-openai、genai、rig-core、openai-api-rs、reqwest自前実装の比較。自動再試行、HTTPクライアント差込、構造化出力、Z.ai・OpenAI・Vertex AIの互換仕様 |
| [Slint Royalty-free License 2.0 の利用条件](slint-royalty-free-license.md) | Slint を無償ライセンスで使う条件（製品種類・attribution・Embedded Systems・API 公開・notice）を調べた過去の決定根拠。Slint は廃止予定 |
| [AI Botの実行基盤への標準プロトコル適用の評価と ACP v2 採用方針](agent-runtime-standard-protocols.md) | AG-UI・ACP＋MCP・独自wireプロトコル・プロトコル化しないの4案で、本体↔ランタイム境界の標準化を評価。要件（承認・段階宣言・監査・ADR 0003）の表現可否と、ACP v2 採用（製品判断）に必要な境界・移行・未決論点 |
| [承認と資格の失効が再取り込みで復活しない世代管理の検討](revocation-generation-model.md) | Buzz の protectionClock を出発点に、承認・資格・ポリシー・AI の所属ごとの失効と再付与の意味論を整理し、世代（対象ごとの単調な版数）で失効を表す方式の採否と記録形式を推奨 |
| [連携カタログ（Composio／Pipedream）の評価と採用可否の判断材料](integration-catalog-composio-pipedream.md) | 資格情報の保管方式・データの経由地域・段階宣言（ADR 0004）との適合・費用の比較と、ADR 0008（国内完結）と両立する使い方（BYOK 的扱い）の整理。推奨案（MVPは自前実装、カタログはBYOK型例外枠で将来検討） |
| [承認ポリシーと型のwhat-ifシミュレーションの検討](what-if-approval-simulation.md) | OpenBot の what-if を出発点に、型の決裁前の過去再現試算（対象・入力・3分類の出力）と監査ログ（ADR 0007・#187 の輸出形式）との関係を整理。推奨案（作る。最小構成は機械的検査のみの CLI レポート） |
| [コーディングエージェントの並行数の上限と利用可否を設定する方式の調査](coding-agent-concurrency-config.md) | 開発環境（AI 開発チーム）の道具の調査で、v0.7 の技術選定には含めない。gptme・OmO・Claude Code・Codex CLI・GSD の並行数の上限の置き場所・検証・上限に達したときの動き、Claude Code・Codex CLI・OpenCode 2・T3 Code・Devin CLI の provider の利用可否と設定の層構造、fnox の age による秘密の共有、GitHub Projects のユーザーレベルのプロジェクトの共同作業者。採用案（リポジトリで共有する 1 つの設定ファイルに、上限と利用可否を別々の情報で置き、コードが読んで検証する） |
| [microsandbox を実装担当の sandbox に使えるかの調査](microsandbox.md) | 開発環境（AI 開発チーム）の道具の調査で、v0.7 の技術選定には含めない。libkrun＋KVM でマイクロ VM を起動するランタイム microsandbox を、現行の bwrap 方式と Docker Sandboxes（`docker-sandboxes.md`）と比べる（ファイル・git・ネットワーク・資格情報・性能・キャッシュ・条件）。導入はせず、公式リポジトリーと公開 Issue で確認。おすすめ（条件付きで試す。ホストと共有するフォルダーの不具合の修正を待って、`sbx` より先に試す） |

測定報告（実機検証）:

| 文書 | 内容 |
|---|---|
| [サンドボックス隔離の測定](sandbox-isolation-measurements.md) | WSL2+rootless PodmanでのgVisor（runsc）とcrunの比較。起動時間、syscall互換性、資源使用量、egress拒否、KVM可否 |
| [Slint と Tauri＋React のクライアント比較](slint-tauri-client-measurements.md) | Slint 1.18.1 と Tauri を比較した過去の測定記録。読み上げ時の ListView の重さ（#3867）、可変高の仮想リスト、#10861 の再現可否、wry の Linux のインライン変換、型生成（ts-rs/specta）、wasm-opt の WASM サイズ |

v0.7 の技術選定に含める文書の判定は「MVP前に検証」「将来の候補」「見送り」の3つです。開発環境（AI 開発チーム）の道具の調査は、文書ごとのおすすめに従います。記載は公式ドキュメントと公開リポジトリにもとづき、実機での検証はしていません（測定報告を除く）。
