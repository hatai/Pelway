<a id="ch09-s01"></a>
# 09 根拠と確認範囲

担当調査の確認日は2026-11-08（日本時間）。動く版の一次資料を読んだこと、固定版と同一であること、実機で動いたこと、契約/法的条件を満たすことは別である。原稿中の調査を統合し、すべてを実機検証したとはしない。

<a id="ch09-s02"></a>
## 詳細な出典台帳

- [市場・既存チャット/AI OSS調査](../sources/v0.6/fixed/pelway-technical-docs/research/existing-chat-and-agent-oss.md)は公式仕様、repo、release、license、通信、連携APIの出典を識別子で保持する。各表の事実・設計提案・未確認を区別する。
- [UI入力](../sources/v0.6/fixed/pelway-technical-docs/input/ui.md)はSlint/egui/iced/GPUI等の確認版、確認日、licenseとIME/読み上げissueを保持する。master条文とrelease同梱licenseの同一性は未確認。
- [技術入力](../sources/v0.6/fixed/pelway-technical-docs/input/technical.md)と[旧技術下書き](../sources/v0.6/fixed/pelway-technical-docs/input/technical-draft/README.md)は構成・OS・実行境界の素材。古い要件や撤回案は本統合の採用判断ではない。
- [プロダクト入力](../sources/v0.6/fixed/pelway-technical-docs/input/product.md)は既存PRDに対する整合要求と試験。既存文書は変更しない。

<a id="ch09-s03"></a>
## 主な一次資料

| 論点 | 一次資料 | 限界 |
|---|---|---|
| Slint | [license](https://github.com/slint-ui/slint/blob/master/LICENSE.md)、[大規模list/a11y issue](https://github.com/slint-ui/slint/issues/3867) | issueの存在は再現/未修正の永久保証ではない。固定配布版で再確認 |
| egui | [公式repo](https://github.com/emilk/egui) | IME/selection/a11yは対象OSとbackend次第 |
| iced/GPUI | [iced a11y PR](https://github.com/iced-rs/iced/pull/3281)、[GPUI Windows読み上げissue](https://github.com/zed-industries/zed/issues/41138) | 現状制約の調査根拠。版更新時は再確認 |
| Windows | [WSL install](https://learn.microsoft.com/en-us/windows/wsl/install)、[Docker Desktop条件](https://docs.docker.com/desktop/setup/install/windows-install/) | Desktop条件をWSL内Engine/Podmanへそのまま外挿しない |
| macOS | [Apple container](https://github.com/apple/container)、[build](https://github.com/apple/container/blob/main/BUILDING.md) | OS/CPU条件、network、build手順を確認。コンテナ内でMac GUI/署名ができる意味ではない |
| 同期 | [Matrix client-server](https://spec.matrix.org/latest/client-server-api/)、[Zulip events](https://zulip.com/api/get-events) | 参考で、server/UIのコードを採用しない |
| MCP | [公式Rust SDK](https://github.com/modelcontextprotocol/rust-sdk/) | 固定版license、全通信、接続toolの審査が条件 |
| local inference | [llama.cpp server](https://github.com/ggml-org/llama.cpp/blob/46dba9fc/tools/server/README.md)、[vLLM usage stats](https://docs.vllm.ai/en/stable/usage/usage_stats/) | model/配布物のlicenseと全通信は別。offline optionだけで適合とはしない |
| 外部連携 | [11の公式手段表](11-integrations.md)、[市場原稿のE1〜E19](../sources/v0.6/fixed/pelway-technical-docs/research/existing-chat-and-agent-oss.md) | API公開、プラン、beta、地域/処理/保管、実機接続は別 |

<a id="ch09-s04"></a>
## Discord情報の扱い

[Discord CTO本人の投稿](https://x.com/svishnevskiy/status/2118081920194203799)は保存済み公開投稿の取得記録に基づき、移行方針/GPUI採用/今後の試験予定を確認した。移行完了や一般配布とはしない。「ElectronからRustで1.3GBが300MBになった」という同条件比較は一次資料で未検証で、UI選定・性能目標・Rust非WebView要件の根拠に使わない。[DiscordのGo→Rust記事](https://discord.com/blog/why-discord-is-switching-from-go-to-rust)は別の話であり、desktopの数値の証明にならない。SNSの本文で確認できる範囲と、画像/報道の数値・条件不明を分ける。

性能目標や工数は仮定として02/07に限定する。公式単価をこの文書で追加せず、既存の原価表や段階計画も書き換えない。

Discord役職の追加確認は市場レビューM-01の[公式companyページ](https://discord.com/company/)に基づく。投稿は移行方針であり、GPUIのWindows読み上げ阻害が解消した証拠ではない。詳細出典は[市場レビュー](../sources/v0.6/fixed/pelway-technical-docs/reviews/market-v0.1.md)。

<a id="ch09-s05"></a>
## 直接依存と許諾の対応（M-03）

技術担当の[照合記録](../sources/v0.6/fixed/pelway-technical-docs/reviews/technical-license-ledger-v0.1.md)を統合した。確認日は2026-11-08（日本時間）。15件と比較候補を区別する。下表の版は観測対象で、採用版の決定ではない。可変ブランチ/最新版のLICENSEやCargo.tomlに基づく情報であり、固定配布物の条文・全推移依存・NOTICE・SBOMは未照合/未作成。法的適合や検証完了とはしない。

| 対象 | 区分（02の分類） | 確認した版 | 公式の許諾 | 出典 | 同梱・表示 | 未確認条件 |
|---|---|---|---|---|---|---|
| tokio | 条件付き依存採用候補 | repo master | MIT | [LICENSE](https://github.com/tokio-rs/tokio/blob/master/LICENSE) | 著作権表示と許諾文 | 固定版の条文 |
| axum | 同上 | crates.io 0.8.9（2026-04-14） | MIT（crate）。workspace定義は `MIT OR Apache-2.0` | [LICENSE](https://github.com/tokio-rs/axum/blob/main/LICENSE) | 著作権表示と許諾文 | crateとworkspaceで表記が違う理由。固定版で再確認 |
| tower | 同上 | repo master | MIT | [LICENSE](https://github.com/tower-rs/tower/blob/master/LICENSE) | 同上 | 固定版の条文 |
| tracing | 同上 | repo main | MIT | [LICENSE](https://github.com/tokio-rs/tracing/blob/main/LICENSE) | 同上 | 外部exporterは使わない（02:93） |
| sqlx | 同上 | crates.io 0.9.0（2026-05-21） | MIT OR Apache-2.0 | [LICENSE-APACHE](https://github.com/transact-rs/sqlx/blob/main/LICENSE-APACHE)（旧 launchbadge/sqlx からリダイレクト） | 選んだ方の条文。Apacheを選ぶならNOTICEの有無を確認 | repo移管の経緯、固定版の条文 |
| PostgreSQL | 同上（サーバー部品、同梱するかは配布形態次第） | repo master | PostgreSQL License（GitHub上はNOASSERTION表示） | [COPYRIGHT](https://github.com/postgres/postgres/blob/master/COPYRIGHT) | 同梱するなら著作権表示 | 同梱か利用者環境か。拡張（pg_bigm、pgvector）は別台帳 |
| rusqlite | 同上（client保存） | repo master | MIT | [LICENSE](https://github.com/rusqlite/rusqlite/blob/master/LICENSE) | 同上 | bundled SQLiteを使う場合のSQLite本体（パブリックドメイン宣言）は別確認 |
| rmcp（MCP公式Rust SDK） | 条件付き依存採用案 | crates.io 3.9.0（2026-11-05） | crate表記はApache-2.0。ただしLICENSE本文は「MITからApache-2.0へ移行中。再許諾の同意がない旧寄稿はMITのまま」 | [LICENSE](https://github.com/modelcontextprotocol/rust-sdk/blob/main/LICENSE)、[Cargo.toml](https://github.com/modelcontextprotocol/rust-sdk/blob/main/Cargo.toml) | Apache-2.0とMITの両方の条文を同梱する前提で考える | 固定版でMIT部分がどこか。単純に「Apache-2.0のみ」と書かない |
| llama.cpp | LLM adapterの第一候補 | repo master | MIT | [LICENSE](https://github.com/ggml-org/llama.cpp/blob/master/LICENSE) | 著作権表示と許諾文 | モデル・tokenizerの許諾は別台帳（08:A7）。同梱する依存（ggml等）の条文 |
| rig | 必要なら小範囲（02:92） | repo main | MIT | [LICENSE](https://github.com/0xPlaygrounds/rig/blob/main/LICENSE) | 同上 | 使う範囲が決まるまで未判定 |
| Slint | 採用案（実機試験と固定版ライセンス確認まで未確定） | crates.io 1.18.1（2026-09-21） | `GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0 OR LicenseRef-Slint-Software-3.0` | [LICENSE.md](https://github.com/slint-ui/slint/blob/master/LICENSE.md) | Royalty-freeならAboutSlint等の表示条件（02:24〜34） | **要確認（濱田さん判断・法務確認）**：どの許諾を選ぶか、固定版同梱の条文、UI拡張を外に開かない設計の維持 |
| egui | 対照 | repo main | MIT OR Apache-2.0 | [LICENSE-APACHE](https://github.com/emilk/egui/blob/main/LICENSE-APACHE)、[Cargo.toml](https://github.com/emilk/egui/blob/main/Cargo.toml) | 選んだ方の条文。default fontsは別条件 | フォントの条文 |
| wasm-bindgen | Web同期核のWASM化 | repo main | MIT OR Apache-2.0（Cargo.toml） | [LICENSE-APACHE](https://github.com/wasm-bindgen/wasm-bindgen/blob/main/LICENSE-APACHE)（旧 rustwasm/wasm-bindgen からリダイレクト） | 同上 | 固定版の条文 |
| TypeScript | Web（ビルド時） | repo main | Apache-2.0 | [LICENSE.txt](https://github.com/microsoft/TypeScript/blob/main/LICENSE.txt) | 配布物に含めるならNOTICE確認 | 実行時には同梱しない想定（仮定） |
| Octane | Web DOM UI | repo main | MIT | [LICENSE](https://github.com/octanejs/octane/blob/main/LICENSE) | 著作権表示と許諾文 | 固定版の条文 |
| pgvector、pg_bigm、Tantivy/Lindera | 比較候補のみ（02:89〜90） | 未調査 | 未確認 | — | — | 導入を決めたときに台帳へ追加 |

rmcpは移行中のMIT部分を残すため「Apache-2.0のみ」としない。Slintの許諾は濱田さんの選択と法務確認待ち。sqlx、wasm-bindgenの出典は移管後URLであり、旧URLのリダイレクト到達は固定版の真正性を保証しない。

<a id="ch09-s06"></a>
## レビューへ提示する証拠の範囲

Discordの独立読取は[保存済み公開投稿の証拠](../sources/v0.6/fixed/pelway-technical-docs/reviews/discord-primary-post-evidence.txt)と[担当レビュー](../sources/v0.6/fixed/pelway-technical-docs/reviews/market-v0.1.md)による。直接依存台帳の[技術原稿](../sources/v0.6/fixed/pelway-technical-docs/reviews/technical-license-ledger-v0.1.md)も固定snapshotへ同梱する。v0.2のCLIレビューは読取ツール無効のため、snapshotにあってもpromptに全文添付しなかったファイルを確認できなかった。次の差分レビューではこの証拠/製品入力/status/台帳を明示的に添付する。未取得の事実を確認済みへ格上げしない。
