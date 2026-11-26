# Issue #745 の作業ログ

## 実装者メモ（round 1）

対象 head: ece03f8c2c5926067746ed2e30bf18c86c7ea07e

1. 詰まった箇所: `client/scripts/canary-bun.mjs` は起動と判定が一体で、試験から呼べなかった。判定を `findCanaryBinary(clientDir, target, platform)` に切り出し、型付きで試験から読めるように `.ts` に替えた（Node 26.13.1 は `.ts` をそのまま動かし、`import.meta.main` も使える。手元で確かめた）。`package.json` の `lint`、`README.md`、`bunfig.toml` のコメントの参照も `.ts` に直した。
2. 曖昧だった文: 「client/node_modules の外（親の node_modules）にしか無いとき」（#745 の受け入れ条件）。`bun` の包みからの解決が親の `node_modules` に届く木を作り、`@oven/bun-<OS>` をそこにだけ置いて確かめた。
3. 仮定と解釈: #745 はもともと文書だけの Issue だったが、hatai の指示で PR #744 のレビューの nit 2 件を足した（本文を書き直し、コメントで残した）。ADR の「依存の版」「lint と format」「単体の試験」「配布とライセンス表示」の 4 か所を、#734 の実装に合わせて書き直した。
4. 足りなかった情報・検証手段: macOS・Windows・musl の本体の名前（`currentTarget`）は確かめていない。試験は `linux-x64` の名前で木を作って判定だけを確かめる。
5. 改善案: `client/README.md` の「道具」に「`client/scripts/` のスクリプトは mise の Node で `.ts` のまま動かす（Node 26 の型の除去）」を足す。

使ったスキル: `.agents/skills/implement-issue`
