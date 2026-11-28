# Issue #774 の作業ログ

## 失敗の筋（実装前）

- 失敗の筋: `bun check` の出力から診断でない行を拾い、エラー件数を多くまたは少なく数える。
  確かめ方: `client/scripts/type-ratchet.test.ts` の診断行の解析のテスト。`bun test` で、stdout の tsc 形式の行だけを数え、同一行を重複除去し、形式外の stdout を失敗にすることを確かめる。
- 失敗の筋: 既存の基準値よりエラーが増えてもラチェットが成功する。
  確かめ方: `client/scripts/type-ratchet.test.ts` の増加時の失敗のテストと、受け入れ条件の確認で実際の client コードへ一時的に TS4111 を 1 件足して `mise run client:type-ratchet` が失敗することを確かめる。
- 失敗の筋: エラーが減ったときに自動で基準値が下がり、減少のレビューができない。
  確かめ方: `client/scripts/type-ratchet.test.ts` の未更新の減少で失敗するテスト。更新コマンドだけが基準値を書き換えることを確かめる。
- 失敗の筋: 更新コマンドが改善と悪化の両方を書き込み、悪化を自動で承認する。
  確かめ方: `client/scripts/type-ratchet.test.ts` の混合時の更新のテスト。改善分だけが書き込まれ、悪化が残る限り終了コードが失敗であることを確かめる。
- 失敗の筋: 将来のオプション追加で、現在エラー 0 件のオプションまで基準値に入り、tsconfig で直接有効化する運用と衝突する。
  確かめ方: `client/scripts/type-ratchet.test.ts` の `--add` のテスト。1 件以上では追加され、0 件では拒否されることを確かめる。
- 失敗の筋: mise の依存が順序どおりに配線されず、verify からラチェットが回らない、または bootstrap 前に実行される。
  確かめ方: `client/scripts/type-ratchet.test.ts` の mise の依存構造のテストと、`mise tasks deps verify-static` の出力、`mise run verify` の終了コード。
- 失敗の筋: `bun check` 自体の失敗や出力形式の変化をエラー 0 件と読み違え、ラチェットが黙って通る。
  確かめ方: `client/scripts/type-ratchet.test.ts` の形式外 stdout と検査器失敗のテスト。診断が無い非ゼロ終了と、診断行に見えない stdout を失敗にすることを確かめる。

## 失敗の筋（最終）

- 失敗の筋: `bun check` の stdout の形式が変わったとき、診断を読み落としてエラー 0 件と扱う。
  確かめ方: `client/scripts/type-ratchet.test.ts` の「stderr の要約のような形式外の stdout は受け付けない」「検査器が診断を出さずに失敗したときは fail closed にする」。形式外行と診断なしの非ゼロ終了を失敗にすることを確かめる。
- 失敗の筋: project references をたどった出力で同じ診断行が複数回出て、負債を実際より多く数える。
  確かめ方: `client/scripts/type-ratchet.test.ts` の「stdout の診断行だけを数え、同一行は 1 件にする」。
- 失敗の筋: 増えたエラーを検出できず、ラチェットが成功する。
  確かめ方: `client/scripts/type-ratchet.test.ts` の「検査で増えていたら失敗し、基準値を書かない」と、`apps/desktop/src/main/index.ts` に一時的な TS4111 を足して実行した `mise run client:type-ratchet` の終了コード 1。
- 失敗の筋: 減ったエラーを検査時に自動で承認し、レビューで減少が見えない。
  確かめ方: `client/scripts/type-ratchet.test.ts` の「検査で減っていても更新するまでは失敗し、基準値を書かない」。
- 失敗の筋: 更新時に悪化分まで基準値へ書き込む。
  確かめ方: `client/scripts/type-ratchet.test.ts` の「更新は悪化分を書かず、改善を書いても悪化が残れば失敗する」。
- 失敗の筋: verify の依存配線が外れ、ラチェットが検査経路から回らない。
  確かめ方: `client/scripts/type-ratchet.test.ts` の「client:type-ratchet が bootstrap と client:verify の間に入り、verify-static から回る」と `mise tasks deps verify-static` の出力。
- 失敗の筋: GitHub Actions で `bun check` が同じ診断を workflow command としても出し、形式外行としてラチェットが止まる、または注釈行を別件として数える。
  確かめ方: `client/scripts/type-ratchet.test.ts` の「GitHub Actions の注釈行は tsc 形式の重複として読み飛ばす」「GitHub Actions の注釈行だけでは診断件数にしない」と、`GITHUB_ACTIONS=true node scripts/type-ratchet.ts` の終了コード 0。

## 実装者メモ（round 1）

対象 head: 2a7e8468daa55ce0732a48388bbc8f882d473ca7

1. 詰まった箇所: 手順 4 / `client/scripts/type-ratchet.ts`: tsc の複数行診断の字下げ行を形式外として扱いすぎ、初回の実機確認が fail closed で止まった。頭の行だけを数え、字下げの続き行は件数に入れない形にした。
2. 曖昧だった文: 「`mise run verify` から回る → `mise tasks deps verify`」（Issue #774 の「受け入れ条件」）
3. 仮定と解釈: verify の構造は変えず、hatai の決定どおり `verify-static` の依存と `mise run verify` の成功を証拠にした。`bun check` の診断は client/ からの相対パスで出る前提にし、診断の続き行は字下げ行として読み飛ばし、頭の行だけを数える解釈にした。
4. 足りなかった情報・検証手段: 複数行診断を安定して出す実機入力は既存コードに無かったため、`TS2322` の実出力で字下げ行の形を確かめた。
5. 改善案: `docs/superpowers/specs/agent-team.md` の「実装担当」に「テストを先に書くときは、検査対象の出力が複数行になる既存例を実機で先に確かめる」と足す。
6. 詰まった箇所: CI の static ジョブ / `client:type-ratchet`: GitHub Actions では `bun check` が同じ診断を workflow command（`::error file=...`）としても出し、最初の実装が形式外行として fail closed で止まった。tsc 形式の行だけを件数にし、注釈行は重複として読み飛ばす形にした。注釈行だけでは件数にしない。

使ったスキル: implement-issue、write-pull-request、brainstorming、grilling
