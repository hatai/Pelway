# 09 出典（確認日はすべて 2026-11-08 日本時間。単価・機能は変わるので実装前に再確認）

## OS・コンテナ（一次情報）
- [Install WSL（Microsoft Learn）](https://learn.microsoft.com/en-us/windows/wsl/install)：Windows 10 2004（ビルド19041）以降またはWindows 11
- [Docker Desktop on Windows](https://docs.docker.com/desktop/setup/install/windows-install)：WSL 2.1.5以上、SLAT対応CPU、RAM 8GB、仮想化の有効化
- [Docker Desktop license](https://docs.docker.com/subscription-billing/desktop-license)：従業員250人未満かつ年商1,000万ドル未満は無料
- [apple/container](https://github.com/apple/container)、[BUILDING.md](https://github.com/apple/container/blob/main/BUILDING.md)：Apple silicon＋macOS 26、vmnetの不具合でDocuments/Desktop配下だとネットワーク作成に失敗

## UI
- [zed#41138](https://github.com/zed-industries/zed/issues/41138)（GPUI：Windowsでスクリーンリーダーが使えない）
- [slint#3867](https://github.com/slint-ui/slint/issues/3867)（Slint：大きいListViewでアクセシビリティの性能影響）
- Slint・egui・iced・GPUI・Freyaのライセンス・版・issueは UI入力 `input/ui.md` §1・§1a に一覧（同日確認）
- [GoldStrikeArch「The Rust desktop GUI ecosystem, measured」](https://goldstrikearch.github.io/rust-gui-desktop-ecosystem-state/)（第三者の調査。参考のみ）

## 同期・Bot（参考）
- [Zulip get-events](https://zulip.com/api/get-events)、[Zulip](https://github.com/zulip/zulip)、[Matrix クライアント・サーバー仕様](https://spec.matrix.org/)
- [rmcp（MCP公式Rust SDK）](https://github.com/modelcontextprotocol/rust-sdk)（依存として採用の候補。ライセンスは実装前に再確認）
- [OpenAI Codex CLI](https://github.com/openai/codex)、[docs/sandbox.md](https://github.com/openai/codex/blob/13c42a077c88a0d04ae7680a9891d2daf4558577/docs/sandbox.md)、[OpenHands](https://github.com/All-Hands-AI/OpenHands)、[goose](https://github.com/block/goose)（参考）

## Discord（根拠には使わない）
- [CTOの投稿 2026-11-04 UTC](https://x.com/svishnevskiy/status/2118081920194203799)：Rustへの移行とGPUIの採用は本人の投稿で確認。スクリーンショットの約290MB（2アカウント）は条件不明
- 報道：[The Verge](https://www.theverge.com/news/1027379/discord-ditching-electron-web-app)、[Windows Latest](https://www.windowslatest.com/2026/11/05/discord-is-killing-its-electron-app-that-can-eat-4gb-of-ram-on-windows-11-and-the-rust-rebuild-uses-just-290mb/)、[Wccftech](https://wccftech.com/discord-is-ditching-electron-for-rust-with-early-windows-build-using-just-290mb-of-ram/)。1.3GBなどの比較値は一次情報で未確認
