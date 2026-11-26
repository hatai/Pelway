# Issue #734 の作業ログ

## 実装者メモ（PR-1 round 1）

対象 head: d4f337af56a7f0aede0c1de80ac7edc8ee8ebe81

1. 詰まった箇所: `mise run pr-size`: 1 本にまとめると lock を除いて +849 行で上限を超え、`scripts/agent-gh-pr-size-gate` が push を止めた。hatai の判断で、道具と包み（この PR）と `client/apps/desktop`（PR-2。この PR の上に積む）に分けた。
2. 曖昧だった文: 「`client/bunfig.toml` に `exact = true` と、`minimumReleaseAge = 604800`（7 日）があり、例外が `octane`・`@octanejs/*`・`@tsrx/*`・`bun` だけ」（Issue #734 の受け入れ条件）。`minimumReleaseAgeExcludes` は glob を受け付けない（bun の `docs/runtime/bunfig.mdx`）。名前で並べ、canary の本体 `@oven/bun-<OS>` の 12 個も足した（足さないと canary の本体が年齢の制限で落ち、postinstall の取得が 404 になる）。
3. 仮定と解釈: 「Octane の Vite プラグイン」は `octane` 本体の `octane/compiler/vite` と解した。`@octanejs/vite-plugin` はルーターと SSR を持つ fullstack 用で、Octane 公式の Electron の例も `octane/compiler/vite` を使う。Octane の部品の試験のプラグインは、画面のコード（`packages/ui` と `apps/desktop/src/renderer`）の `.ts`・`.tsx` だけを通す。外すと `react/jsx-dev-runtime` が見つからずに失敗する（確かめた）。
4. 足りなかった情報・検証手段: `bun check` が Octane の JSX の型を扱えるかは、わざと誤り（props の型違い、存在しない属性）を入れて 2 件のエラーが出ることで確かめた。`octane-tsc` には替えていない。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「4. 規模を見積もり、分ける」に「新しいディレクトリの土台は、設定ファイル（package.json・tsconfig・ツールの設定）を 1 つ 10〜40 行で数え、試験は実装の 1.2 倍で見込む」を足す。

使ったスキル: `.agents/skills/implement-issue`
