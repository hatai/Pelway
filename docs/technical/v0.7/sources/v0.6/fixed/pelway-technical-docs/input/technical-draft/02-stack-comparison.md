# 02 言語・フレームワーク・ライブラリの比較と推奨

## 1. ネイティブGUI（Rust・非WebView）

条件（確定）：Rustベース、Electron/Tauri等のブラウザエンジン依存を避ける、WebViewを既定にしない。外観の再デザインは対象外（UIは既存方針のまま、ここでは描画基盤だけ選ぶ）。

評価の出典：各公式リポジトリ・crates.io と、第三者の比較調査（[GoldStrikeArch「The Rust desktop GUI ecosystem, measured」](https://goldstrikearch.github.io/rust-gui-desktop-ecosystem-state/)）。第三者調査の評価は**要実機確認**として扱う。

| 観点 | GPUI + gpui-component | Slint | egui | iced | Xilem/Masonry | Freya |
|---|---|---|---|---|---|---|
| 実績 | Zedエディタ。Longbridge Pro（gpui-component）。Discordが新クライアントに採用と表明（§4） | 組込み・デスクトップで商用実績 | ツール類で広く使用 | COSMICデスクトップ | 開発初期 | 小規模 |
| 長文・大量履歴 | 可変高さの仮想リスト、20万行級のエディタをうたう（gpui-component公式） | ListViewあり。可変高さ大量件数は要確認 | 毎フレーム描画する方式。大量履歴は自前の仮想化が必要 | 仮想リストは自前 | 要確認 | 要確認 |
| 日本語IME | プラットフォームAPI。Windowsは「荒い」との第三者評価 → **要実機確認** | winit経由 | winit経由 | winit 0.14以降 | winit経由 | winit経由 |
| 選択・コピー | TextView で選択可（gpui-component比較表） | 要確認 | ラベル選択可 | 不可（比較表） | 要確認 | 入力は1行のみ（第三者調査） |
| アクセシビリティ | AccessKit：GPUI本体は2026-05に統合されたが未リリース・Zedでは無効（第三者調査）。gpui-componentはAccessKit対応をうたう → **最大のリスク** | AccessKit既定 | AccessKit（eframe既定） | **なし**（PR草案のみ） | AccessKit | AccessKit |
| 日本語表示・フォント代替 | OSのCoreText/DirectWrite | fontique | 同梱フォント。CJKは「Bad」（gpui-component比較表） | fontdb | fontique | Skia |
| 3OS | ○ | ○ | ○ | ○ | ○ | ○ |
| Web（WASM） | GPUI本体に基本的なWeb対応がマージ（2026-02、Zed PR #50228）。キャンバス描画 | WASM可（Rustのみ） | 可 | 可 | — | — |
| ライセンス | Apache-2.0（gpui・gpui-component） | GPLv3 / 帰属表示つきロイヤリティフリー / 商用 | MIT/Apache-2.0 | MIT | Apache-2.0 | MIT |
| 保守 | gpui は crates.io 0.2.2 で、Zed本体のmainが先行。gpui-component は週次リリース（0.9.0、2026-11-02） | 活発 | 活発 | 活発 | 活発 | 活発 |

### 推奨（UI担当の入力 input/ui.md に合わせて更新）：採用案 Slint、比べる相手 egui
- 初稿ではGPUIを条件付きで推したが、UI担当が示した一次情報で、GPUIは**Windowsでスクリーンリーダーが使えない状態が2025-10から未解決**（[zed#41138](https://github.com/zed-industries/zed/issues/41138)）、Zed専用で外部利用の安定性を保証していない。業務アプリのアクセシビリティ要件に対し阻害が大きいので、**技術担当もSlintを採用案とする**ことに合わせる。GPUIはこの課題が解決したら再評価。
- Slintの阻害候補：大きいListViewでアクセシビリティ有効時に遅い（[slint#3867](https://github.com/slint-ui/slint/issues/3867)）。ライセンスは帰属表示が条件で、アプリからSlintのAPIを外に出すと禁止に当たる恐れ（UI入力 §1a）。
- 技術担当とUI担当の差として残すもの：技術担当は**メモリ・描画性能の比較にGPUIも計測対象として残す**（07）ことを推す。UI担当は比較相手をeguiとする。採否はアクセシビリティとIMEの実機試験を先に行い、両方合格した候補の中で性能を比べる、という順なら矛盾しない。
- 確定採用は、実機試験（UI入力 §5、本書07）とライセンス判断の後。
- 不採用：iced（アクセシビリティなし）、Xilem/Freya（時期尚早）。

## 2. ブラウザ版

| 案 | 内容 | 長所 | 短所 |
|---|---|---|---|
| **A（推奨）** TS + DOM UI ＋ Rust核をWASM | UIはReactなど（DOM）。同期・再接続・キャッシュ・型はRustの `pelway-client-core` をWASMで共有 | ブラウザのIME・アクセシビリティ・ページ内検索・選択をそのまま使える。公開済みの technical.md（Web は TypeScript）と合う | **UIは二重実装** |
| B ネイティブUIをWASMで（GPUI/Slint） | 同じUIをキャンバス描画 | UIが1つ | Web対応は限定的。キャンバス描画ではブラウザのIME・スクリーンリーダー・Ctrl+F・翻訳機能が効きにくい |
| C Leptos（Rust・DOM） | RustでDOMを書く | 言語がRustで揃う | UIはやはり二重。周辺部品が少ない |

### 共有できるもの / できないもの

| 共有できる | 共有できない（二重実装） |
|---|---|
| APIとイベントの型（Rustから TypeScript の型を生成） | 画面部品、レイアウト、フォーカス管理 |
| 同期・再接続・欠番の補完・送信の再試行（`pelway-client-core`） | IME・キーボード操作の細部 |
| 状態の規則（タスク状態、承認の有効/無効の判定） | 通知（OS通知とブラウザ通知） |
| i18nの文言ファイル（Fluent `.ftl`） | アクセシビリティの属性付け |
| デザイントークン（色・余白の値。値はUI担当が決める） | ファイル選択・ドラッグ&ドロップ |

**二重実装の負担（仮定）**：画面数が同じなら、UI層の作業量はおおむね2倍。軽くするため、**最初はWeb版を先に作って仕様を固め、ネイティブは検証ゲート後に同じ画面を後追い**にする案を08に置く（順番は未決）。

## 3. サーバー・共通ライブラリ

| 用途 | 推奨 | 代替 | 理由 |
|---|---|---|---|
| 非同期実行 | tokio | — | 事実上の標準 |
| HTTP/WebSocket | axum（+ tower） | actix-web | tokioと同じ系列、WebSocket標準対応 |
| DB | PostgreSQL + sqlx | SeaORM | SQLを直接書けて、権限と監査が素直に書ける（公開 technical.md と同じ） |
| クライアントキャッシュ | rusqlite（ネイティブ）/ IndexedDB（Web） | — | オフライン表示と再接続後の差分取得 |
| 全文検索 | PostgreSQL＋日本語向け拡張（pg_bigm など）か Tantivy＋Lindera | — | 日本語の分かち書きが必要。**比較は未実施＝要確認** |
| ベクトル検索（RAG） | pgvector | — | DBを増やさない。採用は検索要件が固まってから |
| MCP | rmcp（公式Rust SDK、Apache-2.0） | — | ツール連携の標準 |
| i18n | fluent-rs / @fluent/bundle | — | 06 |
| シリアライズ | serde | — | 標準 |
| ログ・計測 | tracing + OpenTelemetry | — | 監査ログとは別に扱う |

## 4. 「DiscordがElectronからRustへ、メモリ1.3GB→300MB」の確認結果

| 項目 | 確認できたこと | 出典 |
|---|---|---|
| 対象製品 | Discordのデスクトップクライアントを Rust に移す。Electronをやめ、React Native もやめ、モバイルはネイティブUIのままアプリのコードをRustで共有 | DiscordのCTO Stanislav Vishnevskiy のXへの投稿（[2026-11-04 UTC](https://x.com/svishnevskiy/status/2118081920194203799)） |
| UI基盤 | 「TauriではなくGPUI」 | 同上 |
| 移行範囲・時期 | 「近いうちにユーザーテストへ」。完全移行の時期は未発表 | 同上、[The Verge](https://www.theverge.com/news/1027379/discord-ditching-electron-web-app) |
| 約300MB | CTOの投稿の画像で「Discord Phoenix」が約290MB（報道）。報道では2アカウント同時の状態とされる。**比較条件を揃えた測定ではない** | [Windows Latest](https://www.windowslatest.com/2026/11/05/discord-is-killing-its-electron-app-that-can-eat-4gb-of-ram-on-windows-11-and-the-rust-rebuild-uses-just-290mb/)、[Wccftech](https://wccftech.com/discord-is-ditching-electron-for-rust-with-early-windows-build-using-just-290mb-of-ram/) |
| 1.3GB | Discordの一次資料では**確認できなかった**。報道の推定値 | — |

**結論**：製品・移行の方向・GPUIの採用はCTO本人の発信で確認できた。数値は一次資料の比較測定ではないので、**採用根拠にしない**。GPUIを推す理由は§1の部品と実績で、この数値ではない。Rust・非WebViewの方針は要件どおり維持する。
