# Issue #734 の作業ログ

## 実装者メモ（PR-1 round 1）

対象 head: d4f337af56a7f0aede0c1de80ac7edc8ee8ebe81

1. 詰まった箇所: `mise run pr-size`: 1 本にまとめると lock を除いて +849 行で上限を超え、`scripts/agent-gh-pr-size-gate` が push を止めた。hatai の判断で、道具と包み（この PR）と `client/apps/desktop`（PR-2。この PR の上に積む）に分けた。
2. 曖昧だった文: 「`client/bunfig.toml` に `exact = true` と、`minimumReleaseAge = 604800`（7 日）があり、例外が `octane`・`@octanejs/*`・`@tsrx/*`・`bun` だけ」（Issue #734 の受け入れ条件）。`minimumReleaseAgeExcludes` は glob を受け付けない（bun の `docs/runtime/bunfig.mdx`）。名前で並べ、canary の本体 `@oven/bun-<OS>` の 12 個も足した（足さないと canary の本体が年齢の制限で落ち、postinstall の取得が 404 になる）。
3. 仮定と解釈: 「Octane の Vite プラグイン」は `octane` 本体の `octane/compiler/vite` と解した。`@octanejs/vite-plugin` はルーターと SSR を持つ fullstack 用で、Octane 公式の Electron の例も `octane/compiler/vite` を使う。Octane の部品の試験のプラグインは、画面のコード（`packages/ui` と `apps/desktop/src/renderer`）の `.ts`・`.tsx` だけを通す。外すと `react/jsx-dev-runtime` が見つからずに失敗する（確かめた）。
4. 足りなかった情報・検証手段: `bun check` が Octane の JSX の型を扱えるかは、わざと誤り（props の型違い、存在しない属性）を入れて 2 件のエラーが出ることで確かめた。`octane-tsc` には替えていない。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「4. 規模を見積もり、分ける」に「新しいディレクトリの土台は、設定ファイル（package.json・tsconfig・ツールの設定）を 1 つ 10〜40 行で数え、試験は実装の 1.2 倍で見込む」を足す。

使ったスキル: `.agents/skills/implement-issue`

## 実装者メモ（PR-1 round 2）

対象 head: a47f0a5dbba04924032db6355080f2cbb83828e6

1. 詰まった箇所: `client/package.json` の `trustedDependencies: ["bun"]`: Codex の must-fix（canary の postinstall は、OS ごとの本体が無いと registry から取り直し、lock の sha512 と照合しない）。新しいキャッシュ（`BUN_INSTALL_CACHE_DIR`）で postinstall を止めて入れ直すと、`bun install` が `bun/bin/bun.exe` に本体を置き、`1.4.6-canary.1+0966e99d8` で動いた。`trustedDependencies` を外した。
2. 曖昧だった文: 「分け方は hatai が決めました（#734 のコメントの分割案）」（PR #743 の本文）。分割案のコメントは `pelway-dev` 名義で、レビュー担当は hatai 名義の `<!-- split-plan -->` だけを分割の根拠に数える。hatai が文面を選び、hatai 名義で #734 に投稿した。
3. 仮定と解釈: `@oven/bun-<OS>` の例外は hatai の split-plan で認められたので、#734 の受け入れ条件をそれに合わせて直した。ADR 0014 の文は #745 で直す（split-plan の「後続の Issue」）。
4. 足りなかった情報・検証手段: 手元の bun のキャッシュは、前の postinstall が置き換えた本体を持っていた（ハードリンク）。キャッシュを分けないと、postinstall なしで本体が置かれるかを確かめられなかった。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「止まる条件」の pr-size の行に「分けるときは、hatai 名義の `<!-- split-plan -->` が Issue に入るまでレビューを委託しない（レビュー担当は実装担当の分割案を根拠に数えない）」を足す。

使ったスキル: `.agents/skills/implement-issue`
