# Issue #745 の作業ログ

## 実装者メモ（round 1）

対象 head: ece03f8c2c5926067746ed2e30bf18c86c7ea07e

1. 詰まった箇所: `client/scripts/canary-bun.mjs` は起動と判定が一体で、試験から呼べなかった。判定を `findCanaryBinary(clientDir, target, platform)` に切り出し、型付きで試験から読めるように `.ts` に替えた（Node 26.13.1 は `.ts` をそのまま動かし、`import.meta.main` も使える。手元で確かめた）。`package.json` の `lint`、`README.md`、`bunfig.toml` のコメントの参照も `.ts` に直した。
2. 曖昧だった文: 「client/node_modules の外（親の node_modules）にしか無いとき」（#745 の受け入れ条件）。`bun` の包みからの解決が親の `node_modules` に届く木を作り、`@oven/bun-<OS>` をそこにだけ置いて確かめた。
3. 仮定と解釈: #745 はもともと文書だけの Issue だったが、hatai の指示で PR #744 のレビューの nit 2 件を足した（本文を書き直し、コメントで残した）。ADR の「依存の版」「lint と format」「単体の試験」「配布とライセンス表示」の 4 か所を、#734 の実装に合わせて書き直した。
4. 足りなかった情報・検証手段: macOS・Windows・musl の本体の名前（`currentTarget`）は確かめていない。試験は `linux-x64` の名前で木を作って判定だけを確かめる。
5. 改善案: `client/README.md` の「道具」に「`client/scripts/` のスクリプトは mise の Node で `.ts` のまま動かす（Node 26 の型の除去）」を足す。

使ったスキル: `.agents/skills/implement-issue`

## 実装者メモ（round 2）

対象 head: 703c4d061c59b314d94e3365bdbc06122c0eb737

1. 詰まった箇所: `client/scripts/canary-bun.test.ts` の外を指すリンクの試験: `symlinkSync(..., "dir")` は Windows で開発者モードが無く権限も無いと失敗し、試験が判定を呼ぶ前に落ちる（Codex の must-fix）。包みは `client/node_modules` の中に実体で置き、その中の `bin` だけを `"junction"` で外へ向けた（POSIX では type が無視されシンボリックリンクになる）。
2. 曖昧だった文: 「例外は `octane`・`@octanejs/*`・`@tsrx/*`」（ADR 0014 の「依存の版」）。設定（`client/bunfig.toml`）はスコープ全体ではなく 3 つの包みの名前を並べているので、同じ名前で書いた（Codex の `[文書のずれ]`）。
3. 仮定と解釈: 試験が判定を確かめていることを、判定（`realpathSync(binary).startsWith(modules)`）を外すと外を指す 2 つの試験が落ちることで確かめた。印の無い nit（`import.meta.main` が無い処理系で閉じる側に倒すこと、`bun` が無いときの試験の木）は直していない。
4. 足りなかった情報・検証手段: Windows ではジャンクションの試験を動かしていない（Codex の出典の Bun の実装に頼っている）。
5. 改善案: `client/README.md` に「試験でリンクを作るときは `"junction"` を使う（Windows で権限が要らない）」を足す。

使ったスキル: `.agents/skills/implement-issue`
