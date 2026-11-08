# 市場担当の最終差分確認：Pelway技術文書 v0.3

確認日：2026-11-08（日本時間）。対象は `v0.3-snapshot/pelway-technical-docs/`。
市場担当範囲の02・08・09・11、許諾・証拠・参照と、前回の賛成条件を確認した。
`v0.2-to-v0.3.diff`、`disposition-v0.2.md`、`market-v0.2.md`、`technical-v0.2.md`を照合した。
以下の本文行番号はv0.3固定版を指す。採否表の行番号は同梱 `reviews/disposition-v0.2.md` を指す。

## 判定

**賛成。当該固定版の文書内容に対する市場担当の賛成であり、実装承認ではない。**
残る文書必須修正：なし。新しい市場指摘IDは発行しない。
前回条件の採否記録と必要修正は文書上反映された。正式仕様判断・実機試験・固定版許諾などは、下記の実装前未決として残る。
4担当全員の最終賛同や、外部レビュアー本人のv0.3再判定まで取得済みとはしない。全体の完成判定は企画が行う。

## M-01〜M-04は後退していない

| ID | 判定 | 根拠 |
|---|---|---|
| M-01 | 解消を維持 | `02-stack-comparison.md:99〜101`、`09-sources.md:28〜32,61`。DiscordのRust移行方針・GPUI採用と、移行完了・一般配布・同条件メモリ比較の未確認を区別。`02:7,19,44` と `08-decisions-open.md:9` はGPUIを初版主候補にしない。 |
| M-02 | 解消を維持 | `README.md:50〜52`、`09:7〜10,24,32,36,61`から原稿・入力・証拠・台帳へ到達する。下記の相対リンク検査と依存manifest照合も一致。 |
| M-03 | 解消を維持 | `02:95`、`09:36〜57`、`08:35`。rmcpのMIT残存、Slintの3許諾選択と固定版確認待ち、推移依存・NOTICE・SBOM未了を保持。 |
| M-04 | 解消を維持 | `02:64,77` は入力§3a・§5と統合先07を明示。`07-performance-plan.md:5,9`に「比較の順番」「負荷と記録」が実在する。 |

### 許諾

- 前回の補正提案は `02:26` で `GPLv3` から `GPL-3.0-only` へ反映された。採否表16行も採用を記録する。
- Slintは `02:16,24,34,48`、`09:50`、`08:35` により、GPL-3.0-only、Royalty-free 2.0、Software 3.0の選択を濱田さん判断・法務確認へ残す。Royalty-freeを標準的なpermissive OSSと同一視しない。
- rmcpは `09:47,57` と `02:95` でApache-2.0移行中、同意のない旧寄稿はMITのままと記す。「Apache-2.0のみ」は禁止する文脈であり、単独許諾の断定ではない。
- `09:3,36` は観測版、固定配布物、実機、法的適合を分ける。台帳をライセンス検証済みとする記述は重点確認箇所で確認しなかった。
- 今回は全許諾本文やリリース情報を再調査したわけではない。許諾内容の最終判断は未確認で、URLの200応答はその代替にならない。

### Discordと同梱証拠

[CTO投稿](https://x.com/svishnevskiy/status/2118081920194203799)の保存証拠、前回市場レビュー、技術台帳は固定版に同梱され、依存manifestにも照合できた。
`09:61` は保存済み独立読取と今回のレビューへの提示範囲を明確化している。今回新しくX APIで独立取得したとはしない。
`research/existing-chat-and-agent-oss.md`、`reviews/discord-primary-post-evidence.txt`、`reviews/market-v0.1.md`、`reviews/technical-license-ledger-v0.1.md` はv0.2同梱物とバイト一致。
市場原稿の旧説明は保全されたままだが、自己訂正は同梱 `reviews/market-v0.1.md:23〜33` に記録済みで、`09:32,61`から追跡できる。旧説明を新しい本文より優先しない。
他社採用をPelwayの性能や読み上げ合格の根拠へ転用していない。

## 前回の賛成条件

| 元指摘と技術担当ID | 採否記録 | v0.3で確認した反映 |
|---|---|---|
| Codex V2-01 / T2-01 | 採否表5行、採用 | `03-ai-bot-runtime.md:64〜66` は検査と開始更新を同一transactionにし、永続commit後の一つのgateway実行だけに開始資格を一回渡す。重複・再起動・応答不明時の再発行とHTTP自動再試行を禁止。`04-data-api-realtime.md:20〜22,96`、`01-architecture.md:85〜88`にも対応。 |
| Codex V2-02 / T2-02 | 採否表6行、採用 | `03:53` は編集による承認待ち復帰を開始前に限定。開始後は元操作と結果を保持し、unknown中の別案送信も保留。追加指示の承認は `11-integrations.md:21,87` と `08:49` に既定必須と明記。 |
| Codex V2-03 / T2-03 | 採否表7行、採用 | `03:65,136`、`04:96,98` が永続commitと復旧後dispatch停止、独立照合、照合不能時の停止継続を規定。`08:102` にDB耐久・backup巻戻り試験を追加し、具体方式は実装前の阻害とする。 |
| Codex V2-04 / 企画判断 | 採否表8行、採用 | `08:61〜64,72,98,101〜102` が局所部分試験と全構成試験を分離。既存局所試作への自動追加を否定し、別見積もり・別承認後へ配置。 |

これにより、市場担当が前回求めた「採否・理由・修正先の記録と必要修正」は文書上満たされた。
技術担当本人やCodexの最終合格を代行した判断ではない。実装で成立することは未検証。

## 参照先と範囲

- `11:87` の「08の未決」は `08:49` に実在する。未決は将来の承認省略条件だけで、今は新payloadの承認を省略できない。
- `02:64` の入力§3a・§5は `input/ui.md:47,78`、`11:91` の入力§8は同143行に実在する。
- `08:74` のプロダクト入力§3・§7・§10・§11は `input/product.md:65,109,150,193` に実在する。
- `08:89` の既存計画の出典は同梱計画と `references/plan-gates.md` に到達する。今回すべての§参照本文の意味まで網羅再審査したわけではない。
- `02:90` はwiki/Q&A自動生成を範囲外のまま維持する。`08:3` は既存計画を変更しない。`11:3` は4製品を例示としており限定しない。
- 状態遷移、T6、自然返信の決定方法は未承認部分を提案として残す。表示文言・形状・色・配置はUI担当の判断であり、本レビューで決定しない。名称・Aロゴも変更しない。

## ハッシュ・リンク・URL検査

Python 3で `hashlib.sha256(path.read_bytes()).hexdigest()` を再計算した。
本文manifest `v0.3-manifest.json` のcanonical全12件一致。不一致0。
依存manifest `v0.3-dependencies-manifest.json` のunchanged_source_copies全78件とgenerated_navigation全4件も、固定版内の対象ファイルについて一致。不一致0。
manifest自体の真正性や、元の可変ファイルがその後変わっていないことまで保証しない。

README・01〜11・statusの通常Markdown相対ファイルリンク44件をリンク元基準で解決し、欠落0。
全同梱下書き・全参照形式・全見出しアンカーの網羅検査ではない。主要な§参照は上記のとおり別途確認した。

02・09・11のHTTPリンクを重複除去して68件をcurl GETで検査した。全68件が終了コード0・最終HTTP 200。
今回の対象では403・404・timeoutは0。200は内容・能力・適合の証明ではなく、本文の事実をすべて再取得した結果でもない。
使用コマンドは `curl --location --silent --show-error --max-time 25 --connect-timeout 5 --output /dev/null --write-out '%{http_code}\t%{url_effective}' URL`。
検査したURLを以下に残す。いずれも最終200。

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

- Slintの3許諾選択、固定版条文、帰属表示・配布形態、rmcpのMIT部分、全推移依存・NOTICE・SBOM。
- モデル・tokenizer・量子化・配布権、国内host・機材・品質、実行時の通信遮断と観測。国内保存と国外送出禁止を別々に確認する。
- DB耐久設定・復旧照合方式、gateway/server認証、CLI専用区画、権限・冪等保持期限など提供元ごとの能力検証。
- 3OSのIME・読み上げ・選択・大量履歴・性能。すべて実機未確認。
- T6、自然返信、追加遷移、承認省略の将来条件、国外端末・持出し・外部メール・将来地域変更の正式判断。
- 他担当と外部の最終差分結果、完成判定、安全ゲートと実データ利用の別許可。文書賛成を実行許可としない。

## 作業範囲

新規作成は本ファイルだけ。固定版・本文・research・既存計画・既存Git文書は変更していない。
公開URLへのGETとローカル読取り検査のみ。非公開本文・会話・鍵を外部へ送信していない。
clone、実装、サービス接続設定、登録、購入、公開、ユーザーPC操作は行っていない。
