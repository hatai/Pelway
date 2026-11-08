# 直接依存のライセンス対応表（M-03への技術担当の回答・差分案）

作成：AI協業チャット技術・コスト設計係　確認日：2026-11-08（日本時間）
位置づけ：`02-stack-comparison.md:84〜95` と `09-sources.md` に足す材料。固定版・本文は変更していない。企画係さんが統合する。
これは採用確定・ライセンス適合の判定ではない。全推移依存、NOTICE、SBOMの確定はlock作成後の別作業として残す。

確認方法：GitHub APIのlicense情報、各repoの公式LICENSE、Cargo.tomlの`license`、crates.ioの最新版の`license`欄を見た。いずれも可変ブランチか最新版の値で、採用する固定版の同梱条文と同じかは未確認。

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
| React | Web DOM UI | repo main | MIT | [LICENSE](https://github.com/react/react/blob/main/LICENSE)（旧 facebook/react からリダイレクト） | 著作権表示と許諾文 | 固定版の条文 |
| pgvector、pg_bigm、Tantivy/Lindera | 比較候補のみ（02:89〜90） | 未調査 | 未確認 | — | — | 導入を決めたときに台帳へ追加 |

## 本文への修正案

1. 02:95の「用途・ライセンスの一次資料は09と市場調査に集約」を「直接依存の許諾は上表（確認日2026-11-08、可変ブランチ・最新版の値）。全推移依存・NOTICE・SBOMは未作成」に直す。
2. rmcpの許諾は「Apache-2.0（移行中のためMIT部分あり）」と書き、単純にApache-2.0のみとしない。
3. Slintの許諾の選択は08の要確認に残したまま、濱田さんの判断と法務確認の対象とする。
4. repoの移管（sqlx、wasm-bindgen、React）は出典リンクを新しい場所にし、旧URLはリダイレクトで届くことだけ記す。
