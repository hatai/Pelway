# 市場レビュー：Pelway技術文書 v0.2

確認日：2026-11-08（日本時間）。市場・既存チャット/AI OSS調査担当の差分レビュー。
対象は `v0.2-snapshot/pelway-technical-docs/`。以下の本文行番号は、この固定版内のファイルを指す。
`v0.1-to-v0.2.diff`、`disposition-v0.1.md`、前回市場レビュー、技術ライセンス台帳、Codex/Claudeのv0.2レビューも照合した。

## 賛否

**条件付き賛成。** 市場担当のM-01〜M-04は文書上解消。市場担当範囲で、新たな必須修正M2-xxは確認しなかった。
条件は、Codex/Claudeが残した必須指摘の採否・理由・修正先を企画と技術が記録し、必要修正後の差分を担当者が確認すること。これらを本レビューで解消扱いにしない。
文書への賛成は実装、採用、法的適合、国内限定適合、性能、実データ利用の承認ではない。

## M-01〜M-04の判定

| ID | 判定 | 固定版の根拠と限界 |
|---|---|---|
| M-01 | 解消 | `02-stack-comparison.md:99〜101`、`09-sources.md:28〜32`がCTO投稿によるRust移行方針・GPUI採用と、未確認の移行完了・一般配布・同条件メモリ比較を分離。`02:7,19,44`、`08-decisions-open.md:9`でGPUIの初版主候補除外を維持。自己訂正は同梱`reviews/market-v0.1.md:23〜33`に保存。原稿本文の書換えを解消条件とはしない。 |
| M-02 | 解消 | `README.md:50〜52`、`09-sources.md:7〜10,24,32,36`、`11-integrations.md:58`の原稿・入力・レビューに固定パッケージ内から到達できる。本文manifestとは別に根拠依存manifestがあり、下記の再計算も一致。 |
| M-03 | 解消 | `02:95`、`09:34〜57`、`08:35`に技術台帳15件と比較候補を反映。rmcpは移行中のMIT部分を保持し、Apache-2.0のみと断定しない。Slintは3許諾からの選択を要確認とする。固定版、推移依存、NOTICE、SBOM、適合確認は未了と明記。 |
| M-04 | 解消 | `02:64,77`がUI入力の§3a・§5と統合先07へ明示リンク。`07-performance-plan.md:5,9`の「比較の順番」「負荷と記録」が実在。粗い仮定で、係数や実績による裏付け済みとはしない。 |

### Discordの自己訂正と証拠

原稿 `research/existing-chat-and-agent-oss.md:16,60` には、移行範囲まで未確認に読める旧記述が残る。原稿は保全対象のため変更しない。
この旧記述は、移行方針・GPUI採用まで一括して未確認とした点を訂正する。完了・一般配布・比較数値の未確認は維持する。
同梱 `reviews/market-v0.1.md:27〜33` が既存の自己訂正記録で、採否表 `reviews/disposition-v0.1.md:24`、現本文 `09:32` から追跡できる。
優先順位は `README.md:50` が本統合本文を担当入力・旧下書きより上に置く。したがって、原稿が未改稿という理由だけでM-01を未解消へ戻さない。

[CTO本人の投稿](https://x.com/svishnevskiy/status/2118081920194203799)の保存証拠 `reviews/discord-primary-post-evidence.txt:3` を今回読んだ。
長文本文はElectron/React Nativeを置き換える方針、モバイルのnative UI保持、Rustでのapp code共有、GPUI採用、今後のユーザー試験を記す。
保存日時は日本時間2026-11-05 05:44:16に相当する。引用先の非公式投稿と本人本文を混ぜていない。
今回はX APIによる独立再取得は行っていない。保存済み証拠と前回レビューの照合、およびURLのcurl到達確認である。
GPUIの他社採用はPelwayの読み上げ合格の証拠ではなく、メモリ数値も性能保証へ転用しない。

## 市場担当範囲の差分

- `02:7,24,64,77,80,95` は採用案・固定版未照合・部分工数・未見積もりを明示する。32〜62人時と58〜112人時の和は90〜174人時で、総開発工数や競合実績ではない。既存計画への加算も指示しない。
- `02:89〜93` はRAGを初版必須にせず、wiki/Q&A自動生成は範囲外。rmcpは条件付き依存、rigは必要時の小範囲、他runtimeは参考という区別を維持する。
- `08:3,20〜41,49,74〜108` は既存計画との差分と正式判断待ちを区別する。将来地域、国外端末、持出し、外部メール、T6変更、自然返信による決定を確定事項へ格上げしない。
- `09:36〜57` は直接依存と比較候補を分離する。rmcpはcrateの表示とLICENSE本文の違いを記録する。Slintの正式な許諾式は `09:50` にあり、選択は濱田さんと法務へ残す。llama.cppのMITをモデル・tokenizerへ自動適用しない。
- `11:3,31,98,106,124〜136` は4サービスを例示とし、共通契約と個別Adapter、初期オフ、明示有効化、個別承認、外部権限の制約、不変入力を分ける。MCPだけで全機能を満たすとはしていない。
- `README:24〜34` と `11:97,120,136` は本体国内限定と利用者選択連携を分ける。国内保存、国外送出禁止、法適合を同じ保証にしない。自動fallbackや将来地域移行を新しい許可にしない。

新しい市場範囲の必須修正IDは発行しない。補正提案として、`02:26` の「GPLv3」は `GPL-3.0-only` に揃えると誤読が減る。`02:16` と `09:50` は既に正確なため、これだけをM-03再開や反対理由にはしない。
UIの形状・色・配置・採用判断は評価せず、UI担当に委ねる。Pelway名称・選定済Aロゴを変更する提案もしていない。

## 外部レビューとの関係

- Codex `codex-v0.2.md:33〜61` の必須指摘は、開始の一意性、開始後編集、DB永続確定とbackup巻戻り、局所試作への試験混入。担当範囲外の技術修正として企画・技術の採否確認が必要。
- Claude `claude-v0.2.md:34〜37` の追加prompt承認は、`08:49` と `11:87` が既に自動許可を拒否するが、resume能力表へ同じ既定を明記する提案には賛成。採否は企画・技術・プロダクトが決める。
- Claude `claude-v0.2.md:59〜67` の証拠未同梱・外部URL未検査は、同レビューへ渡されたパッケージの限界である。今回の実ファイルには証拠と台帳があり、外部URLも下記の範囲で確認した。「固定版そのものに証拠が存在しない」とは採用しない。
- Claude `claude-v0.2.md:69〜72` のSlint表記統一は上記の補正提案に対応。許諾選択の未決とは別に扱う。

## ハッシュと相対リンクの再検査

Python 3の `hashlib.sha256(path.read_bytes()).hexdigest()` でmanifestの値と比較した。
`v0.2-manifest.json` の `canonical` 全12ファイルが一致。不一致0。
`v0.2-dependencies-manifest.json` の `unchanged_source_copies` 全74件と `generated_navigation` 全4件も、固定パッケージ内のコピーに対して一致。不一致0。
元の可変ファイルがその後変更されていないことまで、この一致から保証しない。

README、01〜11、statusの通常のMarkdown相対ファイルリンク39件を、リンク元ディレクトリ基準で解決した。欠落0。
ファイル存在の検査であり、全見出しアンカー・参照形式リンク・すべての同梱下書きまでの網羅検査ではない。M-04の見出しは別途目視確認した。

## 主要外部URLのcurl確認

02・09・11の通常MarkdownリンクからHTTP URLを重複除去し、68件をGETで確認した。
`curl --location --silent --show-error --max-time 25 --connect-timeout 5 --output /dev/null --write-out '%{http_code}\t%{url_effective}' URL` を使用。
全68件がcurl終了コード0・最終HTTP 200。403、404、timeoutは今回の対象では0。
HTTP 200は本文の正しさ、API能力、リリース状態、法的適合の証明ではない。全116件の市場原稿参照URLを再調査した結果でもない。
リダイレクト例はegui CHANGELOGのmaster→main。移管済sqlx・wasm-bindgen・Reactの新URLにも到達した。
検査URL一覧を以下に残す。すべて最終200。

- https://code.claude.com/docs/en/headless
- https://code.claude.com/docs/en/hooks
- https://cursor.com/docs/cloud-agent/api/endpoints
- https://cursor.com/docs/hooks
- https://cursor.com/docs/integrations/slack
- https://developers.google.com/workspace/events/guides/create-subscription
- https://developers.google.com/workspace/guides/configure-mcp-servers
- https://discord.com/blog/why-discord-is-switching-from-go-to-rust
- https://discord.com/company/
- https://docs.datadoghq.com/getting_started/site.md
- https://docs.datadoghq.com/mcp_server.md
- https://docs.docker.com/desktop/setup/install/windows-install/
- https://docs.vllm.ai/en/stable/usage/usage_stats/
- https://github.com/0xPlaygrounds/rig/blob/main/LICENSE
- https://github.com/DioxusLabs/blitz
- https://github.com/apple/container
- https://github.com/apple/container/blob/main/BUILDING.md
- https://github.com/emilk/egui
- https://github.com/emilk/egui/blob/main/Cargo.toml
- https://github.com/emilk/egui/blob/main/LICENSE-APACHE
- https://github.com/emilk/egui/blob/master/CHANGELOG.md
- https://github.com/emilk/egui/issues/2529
- https://github.com/emilk/egui/issues/7975
- https://github.com/emilk/egui/issues/8813
- https://github.com/emilk/egui/pull/3814
- https://github.com/emilk/egui/pull/7701
- https://github.com/ggml-org/llama.cpp/blob/46dba9fc/tools/server/README.md
- https://github.com/ggml-org/llama.cpp/blob/master/LICENSE
- https://github.com/iced-rs/iced
- https://github.com/iced-rs/iced/issues/552
- https://github.com/iced-rs/iced/pull/2777
- https://github.com/iced-rs/iced/pull/3281
- https://github.com/iced-rs/iced/pull/3497
- https://github.com/linebender/xilem
- https://github.com/makepad/makepad
- https://github.com/marc2332/freya
- https://github.com/microsoft/TypeScript/blob/main/LICENSE.txt
- https://github.com/modelcontextprotocol/rust-sdk/
- https://github.com/modelcontextprotocol/rust-sdk/blob/main/Cargo.toml
- https://github.com/modelcontextprotocol/rust-sdk/blob/main/LICENSE
- https://github.com/postgres/postgres/blob/master/COPYRIGHT
- https://github.com/react/react/blob/main/LICENSE
- https://github.com/rusqlite/rusqlite/blob/master/LICENSE
- https://github.com/slint-ui/slint
- https://github.com/slint-ui/slint/blob/master/FAQ.md
- https://github.com/slint-ui/slint/blob/master/LICENSE.md
- https://github.com/slint-ui/slint/blob/master/LICENSES/LicenseRef-Slint-Royalty-free-2.0.md
- https://github.com/slint-ui/slint/issues/10861
- https://github.com/slint-ui/slint/issues/3867
- https://github.com/slint-ui/slint/pull/13425
- https://github.com/tokio-rs/axum/blob/main/LICENSE
- https://github.com/tokio-rs/tokio/blob/master/LICENSE
- https://github.com/tokio-rs/tracing/blob/main/LICENSE
- https://github.com/tower-rs/tower/blob/master/LICENSE
- https://github.com/transact-rs/sqlx/blob/main/LICENSE-APACHE
- https://github.com/wasm-bindgen/wasm-bindgen/blob/main/LICENSE-APACHE
- https://github.com/zed-industries/zed/blob/main/crates/gpui/Cargo.toml
- https://github.com/zed-industries/zed/issues/41138
- https://github.com/zed-industries/zed/issues/55835
- https://github.com/zed-industries/zed/issues/61724
- https://github.com/zed-industries/zed/issues/64850
- https://github.com/zed-industries/zed/tree/main/crates/gpui
- https://knowledge.workspace.google.com/admin/compliance/choose-a-geographic-location-for-your-data
- https://learn.microsoft.com/en-us/windows/wsl/install
- https://slint.dev/pricing
- https://spec.matrix.org/latest/client-server-api/
- https://x.com/svishnevskiy/status/2118081920194203799
- https://zulip.com/api/get-events

## 実装前の未決

- Slintの許諾選択と固定版条文、rmcpのMIT部分、全推移依存・NOTICE・SBOM。台帳をライセンス検証済みとはしない。
- モデル・量子化・tokenizer・配布権、国内機材と品質。国内性と全実行経路の遮断は実機未確認。
- 外部サービスのplan・beta・API契約、送信データ・処理/保存/運用先・保持、scopeと制御可能性。公開APIの存在は接続済みを意味しない。
- 3OSのIME、長文・選択、読み上げと性能。公式対応や他社採用で代替しない。
- 国外端末、持出し、外部メール、将来地域変更。現行許可を拡張しない。
- T6、短い自然返信、追加遷移、取消・再承認・復旧の技術指摘と最終4担当確認。文書完成と各安全ゲートの通過を分ける。

## 作業範囲

新規作成は本ファイルだけ。固定版・本文・research・既存計画・Gitには変更を加えていない。
公開URLのGETとローカル読取り検査のみ。clone、実装、接続設定、登録、購入、公開、ユーザーPC操作は行っていない。
外部へ送ったのは公開URLへの通常GETだけで、非公開文書・会話・鍵は送信していない。
