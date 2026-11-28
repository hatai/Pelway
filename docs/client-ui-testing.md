# クライアント UI（Electron・Octane）の試験

`client/` の画面と Electron の main/preload に触れる実装者が、試験を書くときの決まりをまとめる。QA が利用者の道筋を起動して確かめる手順は `.agents/skills/verify-pelway/features/client-ui.md` に置き、この文書では重ねて管理しない。

## 1. 検査の入口と範囲

- 入口は `mise run client:verify`。実行内容は `bun install --frozen-lockfile`、`bun fmt --check`、`bun run lint`、`bun check`、`bun test`（`mise.toml` の `client:verify`）。`mise run verify` からも同じ検査が回る。
- `bun test` は `client/bunfig.toml` の `[test].preload` で `client/test/happy-dom.ts` と `client/test/octane-plugin.ts` を先に読む。個別のテストファイルで同じ初期化を繰り返さない。
- `client:verify` と `bun test` は Electron の実行ファイルを起動しない。ローカルの Pelway サーバーにも接続しない。起動の確認は `mise run client:dev` を人が動かす手順、端から端までの Electron E2E は #762 で扱う。
- 試験ファイルは対象のファイルの隣に置く。TypeScript は `*.test.ts`、Octane の部品は `*.test.tsx` を使う。

## 2. Octane の部品の試験

- `client/test/happy-dom.ts` は `@happy-dom/global-registrator` で DOM を `globalThis` に入れる。部品の試験はこの DOM の上で動かす。
- `client/test/octane-plugin.ts` は Octane のコンパイラを `environment: "client"` で通す bun のプラグイン。`packages/ui` と `apps/desktop/src/renderer` の `.ts`・`.tsx` を画面用に変換する。Octane が用意するサーバー向けの読み込みを部品の試験に使わない。
- 部品は `@octanejs/testing-library` の `render` で出し、`screen` の role・label・表示文字で確かめる。`afterEach(cleanup)` で DOM を片付ける。例は `client/packages/ui/src/Greeting.test.tsx`。
- 確かめるのは、props と callback の境界で見える振る舞い（表示、状態、操作の結果）にする。Octane のコンパイル後の内部表現や DOM の実装詳細を固定しない。
- happy-dom は実ブラウザではない。ピクセル単位のレイアウト、OS の IME、NVDA・VoiceOver・Orca、実際のフォーカス移動を合格にしない。これらは 5 節の実機確認と将来の E2E で扱う。

## 3. main・preload・security の単体試験

- main と preload の単体試験では Electron の実行ファイルを起動しない。対象の関数に、試験に要るメソッドだけを持つ代わりのオブジェクトを渡す。
- `security.ts` のように `electron` から型だけを読む設計にすると、試験で `electron` パッケージの実行時の中身を読まずに済む。実行時の `app`・`BrowserWindow`・`session` が要る処理は、小さい関数へ分けて値を注入する。
- `security.test.ts` の形に倣い、`WebContents`・`Session`・`IpcRenderer` の代わりのオブジェクトには試験で使うメソッドだけを持たせる。網羅的な Electron の模倣を作らない。
- preload の橋は `createBridge` の単体試験で、宣言したチャンネルだけを通すことと、listener に `IpcRendererEvent` を渡さないことを確かめる（例は `client/apps/desktop/src/preload/bridge.test.ts`）。
- main の資産解決・送信元検査・遷移の拒否は、実際の窓を作らずに確かめる（例は `client/apps/desktop/src/main/app-protocol.test.ts` と `security.test.ts`）。

## 4. CSP の試験

- renderer の CSP は `client/apps/desktop/src/renderer/index.html` の `<meta http-equiv="Content-Security-Policy">` が正本。試験では HTML を読み、directive ごとに分解して確かめる（例は `client/apps/desktop/src/renderer/csp.test.ts`）。
- `script-src 'self'` と `object-src 'none'` を外さない。外部のスクリプトや object を足す変更は、ADR 0014 のセキュリティの既定に反する。
- `frame-ancestors` は `<meta>` の CSP では効かないので、HTML に書かない。窓の遷移と新しい窓は main の `hardenWebContents` で止める。
- サーバーへ renderer から直接 fetch する通信を入れるときは、CSP に `connect-src` を足し、接続先を自前の server に絞る。具体的な値は #817 で決める。`connect-src` を足したら `csp.test.ts` も同じ変更で直す。

## 5. この文書の試験で確かめられないもの

- `mise run client:dev` での起動は、GUI がある環境で人が確かめる。現在の期待値は、窓に「Electron と Octane の土台が動いています。」と出ること。手順は verify-pelway の `features/client-ui.md`。
- Playwright の Electron E2E は #762 で作る。E2E ができるまで、実際の Chromium の描画・OS 連携・ウィンドウの挙動を `bun test` で合格にしない。
- 実機の IME・文字選択・NVDA・VoiceOver・Orca は 3 OS で確かめる。happy-dom の試験はその代わりにならない。
- サーバーとの通信は #333・#817、サーバーと開発版の同時起動は #757 で進める。通信の実装ができるまで、画面操作がサーバーへ届くことをこの文書の単体試験で合格にしない。
