# client

Pelway のデスクトップアプリ（Electron）と、その画面（Octane）を置く。決定と構成の正本は [ADR 0014](../docs/adr/0014-desktop-client-electron.md)。

## 構成

| パス                                 | 中身                                                                                  |
| ------------------------------------ | ------------------------------------------------------------------------------------- |
| `packages/contracts/`                | IPC のチャンネルの一覧（`src/ipc.ts`）                                                |
| `packages/core/`                     | `pelway-client-core` を TypeScript に移したもの（今は空）                             |
| `packages/ui/`                       | 自前の UI 部品                                                                        |
| `packages/tokens/`・`packages/i18n/` | デザイントークンの CSS 変数と、画面の文言（今は空）                                   |
| `test/`                              | `bun test` の前に読むもの（happy-dom と、Octane のコンパイラを通す bun のプラグイン） |

## 道具

- Node と bun の版は、リポジトリの直下の `mise.toml` で固定する。mise の Node は開発の道具（electron-vite・Vite・Octane のコンパイラ）を動かすためのもので、アプリの中で動く Node は Electron に同梱のもの。
- 依存の版は `package.json` の `workspaces.catalog` にまとめる。`bunfig.toml` で、公開から 7 日以上経った版だけを入れる（例外は `bunfig.toml` の `minimumReleaseAgeExcludes`）。
- lint は canary 版の bun の `bun lint`（devDependency の `bun`。依存の postinstall は走らせず、`scripts/canary-bun.mjs` が lock で照合した OS ごとの本体を起動する）、整形は `bun fmt`、型検査は `bun check`、試験は `bun test`。

## 使い方

リポジトリの直下で次を実行する。

```bash
mise run client:bootstrap   # 依存を入れる
mise run client:verify      # 整形・lint・型検査・試験（mise run verify からも回る）
```

`client/` の中で個別に回すときは次のとおり。

```bash
bun fmt            # 整形する（検査だけなら bun fmt --check）
bun run lint       # canary の bun lint
bun check          # 型検査
bun test           # 試験
```
