# client

Pelway のデスクトップアプリ（Electron）と、その画面（Octane）を置く。決定と構成の正本は [ADR 0014](../docs/adr/0014-desktop-client-electron.md)。

## 構成

| パス                                 | 中身                                                                                                             |
| ------------------------------------ | ---------------------------------------------------------------------------------------------------------------- |
| `apps/desktop/src/main/`             | Electron の main。窓、セキュリティの既定（`security.ts`）、同梱の資産を出す `pelway://app/`（`app-protocol.ts`） |
| `apps/desktop/src/preload/`          | 画面に出す IPC の橋（`bridge.ts`）。`packages/contracts` で宣言したチャンネルだけを通す                          |
| `apps/desktop/src/renderer/`         | 画面の殻。CSP は `index.html` に書く                                                                             |
| `packages/contracts/`                | IPC のチャンネルの一覧（`src/ipc.ts`）                                                                           |
| `packages/core/`                     | `pelway-client-core` を TypeScript に移したもの（今は空）                                                        |
| `packages/ui/`                       | 自前の UI 部品                                                                                                   |
| `packages/tokens/`・`packages/i18n/` | デザイントークンの CSS 変数と、画面の文言（今は空）                                                              |
| `test/`                              | `bun test` の前に読むもの（happy-dom と、Octane のコンパイラを通す bun のプラグイン）                            |

## 道具

- Node と bun の版は、リポジトリの直下の `mise.toml` で固定する。mise の Node は開発の道具（electron-vite・Vite・Octane のコンパイラ）を動かすためのもので、アプリの中で動く Node は Electron に同梱のもの。
- 依存の版は `package.json` の `workspaces.catalog` にまとめる。`bunfig.toml` で、公開から 7 日以上経った版だけを入れる（例外は `bunfig.toml` の `minimumReleaseAgeExcludes`）。
- lint は canary 版の bun の `bun lint`（devDependency の `bun`。依存の postinstall は走らせず、`scripts/canary-bun.ts` が lock で照合した OS ごとの本体を起動する）、整形は `bun fmt`、型検査は `bun check`、試験は `bun test`。

## 使い方

リポジトリの直下で次を実行する。

```bash
mise run client:bootstrap   # 依存を入れる（Electron の本体は初めて起動したときに取ってくる）
mise run client:dev         # Electron の開発版を起動する
mise run client:verify      # 整形・lint・型検査・試験（mise run verify からも回る）
```

`client/` の中で個別に回すときは次のとおり。

```bash
bun fmt            # 整形する（検査だけなら bun fmt --check）
bun run lint       # canary の bun lint
bun check          # 型検査
bun test           # 試験
```

## TypeScript の型エラーラチェット

まだ tsconfig で有効化できない TypeScript の厳格化オプションは、`client:type-ratchet` でエラー件数を `type-ratchet-baseline.json` 以下に縛る。普段は `mise run client:verify`（したがって `mise run verify`）から回る。エラーが増えたときは失敗し、減ったときも基準値を更新するまでは失敗する。

```bash
mise run client:type-ratchet                  # 基準値どおりか検査する
mise run client:type-ratchet-update           # 減った件数だけを基準値へ反映する
mise run client:type-ratchet-add <option>     # 新しい compiler option を現在の件数で追加する
```

更新は悪化した件数を自動で基準値へ入れない。悪化と改善が混ざったときは改善分だけが書き込まれ、悪化が残る限り終了コードは失敗になる。ファイルを rename したときは、基準値のパスを同じ変更で手動で移す。オプションのエラーが 0 件になったら、ラチェットで縛り続けず、tsconfig で有効化して基準値から外す。0 件のオプションは `client:type-ratchet-add` に拒否される。

## IPC のチャンネルを足すとき

`packages/contracts/src/ipc.ts` に宣言し、main のハンドラ（送り元を `isTrustedSender` で確かめる）、preload、画面の模擬（`@octanejs/electron` の `installElectronBridge`）を同じ変更で直す。`@octanejs/electron` から使うのは、`invoke`/`on` だけで動くフック（`useInvoke`・`useInvokeState`・`useIpcEvent`）と試験用の模擬だけで、組み込みの `octane:*` のチャンネルに頼るフックとヘルパー（アプリの情報と終了・窓の状態と操作・テーマ・ダイアログ・シェル・クリップボード・ディスプレイの情報）は使わない。
