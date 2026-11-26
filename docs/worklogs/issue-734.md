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

## 実装者メモ（PR-1 round 3）

対象 head: cb59e1fb2c1fc0f794e69e307b3acc9caab678fb

1. 詰まった箇所: `client/package.json`: round 2 で `trustedDependencies` を外したが、bun 1.4.5 は `trustedDependencies` が無いと既定の許可リスト（npm の `bun` を含む）を使い、canary の postinstall が走り続けていた（Codex の round 2 の must-fix）。round 2 の「postinstall なしでも本体が置かれた」は、この postinstall が走った結果を読み違えていた。
2. 曖昧だった文: 「依存の postinstall を走らせない」（round 2 で #734 の受け入れ条件に足した文）。`trustedDependencies` を置かないことでは満たせない。`bunfig.toml` の `ignoreScripts = true` で満たす。
3. 仮定と解釈: `ignoreScripts = true` では npm の `bun` の bin が置き換え前（450 バイト）のままになり、本物の実行ファイルは lock の sha512 で照合して入った `@oven/bun-<OS>/bin/bun` にだけ残る。`client/scripts/canary-bun.mjs` がそれを直接起動し、見つからなければ exit 1 にする。`bun run` の中では `node_modules/.bin/bun`（置き換え前）が先に見つかるので、起動は mise の Node で行う。
4. 足りなかった情報・検証手段: 手元の bun のキャッシュは前の postinstall が書き換えていた（本体を `bun/bin` へ移していた）。空のキャッシュ（`BUN_INSTALL_CACHE_DIR`）で、`bun install --frozen-lockfile` から `bun run lint`（`1.4.6-canary.1+0966e99d8`）・誤りを入れたファイルで exit 1・`bun` が無い場所で exit 1 まで確かめた。macOS・Windows・musl の本体の名前は確かめていない。
5. 改善案: `client/README.md` の「道具」に「依存の postinstall は走らせない（`bunfig.toml` の `ignoreScripts`）。postinstall が要る依存を足すときは、その理由と照合の仕方を ADR 0014 に書いてから許す」を足す（#745 で ADR に書く）。

使ったスキル: `.agents/skills/implement-issue`

## 実装者メモ（PR-1 round 4）

対象 head: a06ddced9e4885a727974aedcbcf7398a8b7bad9

1. 詰まった箇所: `docs/dev-environment.md`: Claude の round 3 の `[文書のずれ]` の nit。ツールの表に Node と bun が無く、pre-commit の列挙に `client_fmt` が無く、CI の表の `verify-static` の中身に `client:verify` が無かった。3 か所に足した。
2. 曖昧だった文: 「`verify-static`（hk / fnox / pitchfork / cargo-deny / shellcheck）」（`docs/dev-environment.md` の CI の表）。`mise.toml` の `verify-static` を変えたとき、どの文書が中身を列挙しているかを探す手段が無く、レビューで見つかった。
3. 仮定と解釈: `[文書のずれ]` の印の nit だけを直した。印の無い nit（`canary-bun.mjs` が `client/node_modules` の外の同名の包みを解決しうること、`"trustedDependencies": []` も置くこと）は直していない。Codex の round 3 は利用の上限で失敗し、hatai の判断で Claude の結果だけで進めた。
4. 足りなかった情報・検証手段: `mise run verify` の 1 回目は `shellcheck` が `Terminated` で落ちた（指摘は 0 件）。単独で回すと 0 findings、2 回目の `mise run verify` は exit 0。
5. 改善案: `mise.toml` の `[tasks.verify-static]` の上のコメントに「中身を変えたら `docs/dev-environment.md` の CI の表も直す」を足す。

使ったスキル: `.agents/skills/implement-issue`
