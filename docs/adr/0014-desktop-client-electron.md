---
status: accepted
---

# デスクトップアプリは Electron で Web UI を包み、サーバーは Rust のままにする

> 2026-11-26 改訂: hatai が Slint をやめ、D1（友人デモ通し）から Electron で作ると決めた。この改訂は、その決定を切り替えの時期・Slint の廃止順・配布・通信の記述へ反映する。

hatai が、デスクトップアプリを Electron にすると決めた。画面は Web の技術（ブラウザ版と同じもの）で作り、Electron はそれを包む外枠だけを担う。server・runner・gateway などのサーバー側は、これまでどおり Rust で作る。理由は開発の速さで、Rust だけで画面まで作り切るのは時間がかかりすぎると判断した。この ADR は、[技術仕様 v0.7](../technical/v0.7/README.md) の A1（Rust非WebViewのSlint GUI。[08 の「継承する技術選定」](../technical/v0.7/body/08-decisions-open.md)）と、[02](../technical/v0.7/body/02-stack-comparison.md) 冒頭の「Rustベースの…ネイティブ、非WebViewを既定」の要件を置き換える。

## 決定

- デスクトップアプリは Electron で作る。画面は Web UI（TypeScript）で作り、ブラウザ版とデスクトップアプリで同じ画面のコードを使う。
- Web UI は Octane で作る。React も UI ライブラリ（shadcn・Radix・Kobalte など）も使わず、UI 部品は自前で実装する。
- Windows・macOS・Linux の 3 つの OS に配る。
- Electron は外枠（窓、メニュー、OS との連携、資格情報の保存）だけを担い、業務のロジックや承認の判定は持たない。承認の確定はこれまでどおりオンラインの server だけで行う（[04](../technical/v0.7/body/04-data-api-realtime.md)）。
- サーバー側（server: axum、runner、gateway、DB 層）と `pelway-contract` の契約は変えない。
- 切り替えの時期：D1（友人デモ通し）から Electron で作る。`client/` の土台（ディレクトリ、道具の設定、最小の窓、検査）は既にある。Electron 版の画面・サーバーとの通信・E2E はまだ完成していないので、D1 に向けた実装 Issue で完成させる。Slint の `pelway-client-ui` は凍結し、新しい機能は足さず、D1 の中で #766 の順で消す。D3（社内で毎日使える最小版）も Electron で出す。

## 検討した案

| 案 | 採否 | 理由 |
|---|---|---|
| Electron | 採る | 情報量が多く、安定している。Chromium を同梱するので、3 つの OS で表示と日本語入力の挙動がそろいやすい（同梱の効果は未実測） |
| Tauri v2 | 採らない | 軽く、外枠を Rust で書けるが、OS ごとの WebView（WebView2・WKWebView・WebKitGTK）で表示と挙動が分かれる。Linux の日本語のインライン変換は wry が既定で無効にしている（[slint-tauri-client-measurements.md](../research/slint-tauri-client-measurements.md) 5 節）。情報量も Electron より少ない |
| Slint の継続（v0.7 の A1） | 採らない（2026-11-26 にやめた） | 画面をブラウザ版と別に作る二重実装になり、開発が遅い |
| GPUI・Dioxus・egui | 採らない | 比較の Issue（#423・#428・#430・#431・#434）は、この決定で目的がなくなったので閉じた |

## 捨てる要件と代わり

| 捨てる要件 | 出所 | 代わり |
|---|---|---|
| Rustベースのネイティブ、非WebViewを既定 | [02](../technical/v0.7/body/02-stack-comparison.md) 冒頭と「Discordの事例」の「Rust・非WebViewはユーザー要件として維持」 | Electron（Chromium 同梱）。日本語入力・読み上げ（NVDA/VoiceOver/Orca）・文字選択はブラウザの機能を使い、3 つの OS の実機で確かめる。v0.7 [05](../technical/v0.7/body/05-local-dev.md) の「headless試験だけでIME/読み上げ合格にしない」は残す |
| ネイティブの初期メモリ目標 300MiB 以下（仮置き） | [07](../technical/v0.7/body/07-performance-plan.md) | ひとまず 800MiB 以下。対象は Electron の main・renderer・GPU・utility の子プロセスを含めたプロセス木の合計（コンテナ・DB・LLM は含まない）。OS ごとの測り方は 07 の「GUIメモリ」の行のまま |

## 共通核（pelway-client-core）

TypeScript に移す。WASM にはしない（v0.7 A2 の「状態と同期はRust/WASM」を置き換える）。置き場所は `client/packages/core/` で、Rust 版は TypeScript 版（#752）のマージ後に #827 で消す。進み具合の評価は #723 のコメント（`adr-0014-research`）にある。

契約の型の正本は `crates/pelway-contract` のままにし、TS の型は ts-rs で書き出す（[slint-tauri-client-measurements.md](../research/slint-tauri-client-measurements.md) 7 節）。

## Web UI のフレームワーク

Octane（[octanejs/octane](https://github.com/octanejs/octane)。MIT。React の書き方をコンパイルする方式。[oss-platform-and-client.md](../research/oss-platform-and-client.md) 6.5 節、[08](../technical/v0.7/body/08-decisions-open.md) の A2 の行）を使い、版は完全に固定する。React も UI ライブラリも使わない。UI 部品（ボタン、入力欄、一覧、ダイアログ、メニュー、トーストなど）は自前で作る。08 の「A2 の Web UI（Octane）で実装時に決める事項」のうち、「React の併用」は「使わない」で決まり。ビルド基盤（Vite。electron-vite の中で使う）とテスト（`@octanejs/testing-library`）は「運用の仕組み」の「決定」に書いた。それ以外は、その事項に初めて触れる Issue で決める。

自前の UI 部品にすると、次の作業を自分たちで持つ。決定は変えず、工数の見込みに入れる。

- アクセシビリティ：ダイアログのフォーカスの閉じ込めと戻し、メニューと一覧の矢印キー操作、`aria-*` の付け方。部品ごとに NVDA・VoiceOver・Orca での確認が要る。
- IME：入力欄とメンション候補で、変換中の Enter・Esc を扱う（QA-APR-01・QA-MEN-01。今は `pelway-client-core` の `on_key`）。`compositionstart`／`compositionend` と `isComposing` の扱いを、3 つの OS の差を含めて試す。
- 仮想スクロール：会話の一覧は行ごとに高さが違う。可変高の仮想リスト（高さの測り直し、下端への追従、上への読み足し）を自前で作るか、Octane の移植版（`@octanejs/tanstack-virtual` など）を使うかは、会話の一覧を作る Issue で決める。
- テーマとトークン：4 つのテーマとデザイントークン（`docs/design-system/tokens/pelway.tokens.json`）を CSS 変数に落とす仕組み。

## ディレクトリ構成

構成の考え方は [nightly-labs/openbot](https://github.com/nightly-labs/openbot) を参考にする。ライセンスが PolyForm-Noncommercial-1.0.0（商用不可）なので、考え方だけを使い、コードは写さない。実装する人も同じ決まりに従う。

Rust のワークスペースはそのままにし、`client/` の下に Web と Electron をまとめる。

```text
Cargo.toml              Rust のワークスペース（crates/*。変えない）
crates/
  pelway-contract/      契約の型の正本。ts-rs で TS の型を書き出す（feature を足す）
  pelway-http/ ほか      サーバー側（変えない）
  pelway-client-ui/     Slint。D1 の中で #826 で廃止
  pelway-client-core/   Slint 用の核。#752 の TypeScript 版のマージ後に #827 で廃止（TS 版は client/packages/core）
  dev-client-probe/     D1 の中で #825 で廃止
client/                 Web と Electron（Node。版は mise で固定）
  package.json          workspaces（apps/*・packages/*）
  apps/
    desktop/            Electron アプリ
      src/main/         窓、IPC（ドメインごと）、CSP・権限・信頼する出どころ、トークンの保存（safeStorage）、自動更新
      src/preload/      contextBridge で出す橋は 1 つ（宣言したチャンネルだけを通す invoke/on）。main から来た値を解読してから画面に渡す
      src/renderer/     画面の殻と features/<domain>/（承認、判断待ち、会話、設定など）
    web/                ブラウザ版（同じ部品を使う。`client/apps/web` は今は作らない。作る時期は別に決める）
  packages/
    contracts/          pelway-contract から書き出した TS の型（生成物）と、IPC のチャンネルの一覧と送り受けの型
    core/               pelway-client-core を TS に移したもの（承認カードの状態機械、判断待ちの盤、on_key）。DOM も Electron も使わない
    ui/                 自前の UI 部品（基本の部品は src/、機能ごとの部品は src/features/<domain>/）。preload の API も main も呼ばず、props と callback で受ける
    tokens/             docs/design-system/tokens/pelway.tokens.json から CSS 変数を作る
    i18n/               画面の文言（ja・en）
```

- workflow の Slint のビルド用 `libfontconfig1-dev` は、#825・#826・#827 の後の #828 で外す。
- 型の流れ：`crates/pelway-contract` が正本。ts-rs で `client/packages/contracts/src/generated/` に書き出し、`.gitattributes` で `linguist-generated` を付ける。CI で書き出し直し、差分が無いことを確かめる。
- IPC：チャンネルの一覧と送り受けの型は `packages/contracts` の 1 か所で宣言する。main はドメインごとのファイルで受け、引数を解読する前に送り元を確かめる。チャンネルを足すときは、宣言・main・preload・画面の模擬を同じ変更で直す。
- `@octanejs/electron`：画面の側のフック（`useInvoke`・`useIpcEvent` など）と、試験用の模擬（`installElectronBridge`）だけを使う。preload と main は自前で書く。preload は、このパッケージのフックが読む形の橋（`invoke`/`on`）を出すが、通すのは `packages/contracts` で宣言したチャンネルだけにする。`registerOctaneElectronMain` と `exposeOctaneElectron` は使わず、組み込みの `octane:*` のチャンネル（`shell.openExternal`・クリップボードなどを確認なしで呼べる）を登録しない。
- client-core：`client/packages/core/` に置き、Rust の試験（`crates/pelway-client-core/tests/*.rs`）と同じ振る舞いの試験を `*.test.ts` で持つ。
- 依存の向き：`apps/desktop/src/renderer` → `packages/ui`・`packages/core`・`packages/contracts`。`packages/ui` → `packages/tokens`・`packages/i18n`。`packages/core` → `packages/contracts` だけ。逆向きの import と、renderer から main・preload への import は `bun lint` の規則で止める。

## 運用の仕組み

ビルド・署名・配布・リリース・CI・試験・依存・ログの仕組みも、[nightly-labs/openbot](https://github.com/nightly-labs/openbot) の考え方だけを参考にする（コードは写さない）。配布は国内ホストの Caddy から行う（[ADR 0012](0012-production-topology-single-host.md) の単一ホスト）。

### 決定

- **署名**：最初は Windows・macOS・Linux のどれも署名しない。あとで署名を足せるように、置き場所だけを残す。署名の設定は electron-builder の別の設定ファイルに分け、署名なしでも作れるようにする。鍵は、タグのときだけ使える GitHub の environment（例：`desktop-release`）に置く。鍵の登録は hatai が行い、agent は鍵の値を読まず、設定もしない。macOS の notarize は、Apple Developer Program に入ったときに足す。
- **自動更新**：electron-updater を使うのは Windows（NSIS）と Linux（AppImage）だけにする。Windows では署名なしでも自動更新は動くが、入れるたびに SmartScreen の警告が出る。macOS は署名が無いと自動更新が動かないので、新しい版は利用者に手で入れ替えてもらう。macOS の自動更新は、署名を入れたあとに足す。electron-updater は generic provider で、国内ホストの Caddy の `https://<国内ホスト>/desktop/<channel>/` を見る。
- **配布経路**：国内ホストが定期的に GitHub Releases を見に行き、新しい版があれば持ってくる（pull）。持ってくるのは本体、`latest*.yml`、`SHA256SUMS`、SBOM。`SHA256SUMS` で sha256 を確かめてから、Caddy が配る場所に入れ替える。`SHA256SUMS` は同じ Release から取るので、これで防げるのは転送中の破損と取り違えだけで、Release を作り替えられる者への備えにはならない。そこで、ホストは入れ替えの前に、本体が hatai の push したタグから作られたことも確かめる。確かめるのは、本体に付いた GitHub の artifact attestation（ビルドの来歴の証明）が、このリポジトリの `desktop-release.yml` によって `refs/tags/desktop-v*` のタグから作られたと示していること。あわせて、`desktop-v*` のタグが今も残っていて、attestation の元のコミットを指していることも確かめる。確かめられない版は入れ替えず、前の版を配り続ける。`desktop-v*` のタグの作成・付け替え・削除は、hatai だけに限る（リポジトリの ruleset。設定は hatai が行う）。公開の判断はタグの push とみなし、Release の公開の操作や公開した人はホストで確かめない。hatai が意図して配布を取り下げるときは、その版のタグを消す（より新しい版を出しても、古い版の本体は取れるまま残る）。ホストは、タグが消えたときだけでなく、別のコミットを指すようになったときも、その版を配布から外す（下の取り下げの規則）。本体の来歴だけでは足りない。Windows と Linux のクライアントは更新の manifest（`latest*.yml`）に従って更新を判断し、取得先の URL も manifest から決めるので、Release から取った `latest*.yml` をそのまま置くと、Release の資産を書き換えられる者が古い版への巻き戻しや、国内ホストの外への接続を仕掛けられる。そこでホストは、Release の `latest*.yml` を配らず、検証した値だけから `latest*.yml` を作り直して置く。入れる値は、版（attestation の元のタグの版）、本体のファイル名、ホストが計算した `sha512` と `size`、公開の日時、`stagingPercentage` だけにする。本体の場所は、そのチャネルの Caddy の下の相対の名前だけで書き、別のオリジンを指す URL は作らない。差分更新に使うブロックマップも、本体と同じ場所の相対の名前で置く。版の比較にもタグの版を使い、タグの版が今配っている版より新しくなければ入れ替えない。Release の `latest*.yml` からは `stagingPercentage` だけを読む（同じ版で `stagingPercentage` だけが変わったときは、その値だけを反映する）。`stagingPercentage` は改ざんされうるが、変えられるのは公開の速さと停止だけで、別の本体を配ることも別のホストへつながせることもできないので、この範囲は許す。`releaseNotes` などそのほかの項目は作り直した manifest に入れない。`SHA256SUMS` は転送の確認にだけ使う。SBOM は来歴を確かめないので、参考の情報として配り、保証には使わない。取り下げも配布に反映する。ホストは取りに行くたびに、Caddy から取れるすべての版について、タグが残っていて attestation の元のコミットを指していることを確かめる。タグが消えた、または別のコミットを指していると確かめた版は（GitHub がタグは無い、または別のコミットを指すと答えたときだけ。取得の失敗や GitHub の障害は取り下げとみなさず、次の回に確かめ直す）、本体を Caddy の配る場所から外す。その版が `latest*.yml` の版だったときは、同じ照合を通る版のうち最も新しい版で `latest*.yml` を作り直す。通る版が無ければ `latest*.yml` を置かない。すでにその版を入れた利用者は自動では前の版に戻らないので、直した版を新しいタグで出して届ける。ホストに置くのは、このリポジトリだけを対象にした、読み取りだけの fine-grained トークン 1 つだけにする。リポジトリが公開のあいだは、公開済みの Releases とタグは認証なしでも読めるので、トークンに権限を足さずに済み、トークンは主に認証なしのときのレート制限を避けるために使う。リポジトリを非公開にしたときは、Releases とタグを読むのに Contents の read が要る。fine-grained トークンの権限は Releases だけに絞れず、Contents の read はリポジトリの中身（コードと全履歴）も読めるので、そのときはホストが乗っ取られるとこのリポジトリのソースが読まれうる（attestation の読み取りにほかの権限が要るかは、取りに行く仕組みの Issue で確かめる。書き込みの権限は足さない）。トークンの作成とホストへの登録は hatai が行う。CI からホストへ送る形（push）は採らない。ホストに入る鍵を GitHub に置かないため、また外からホストへの入り口を開けないため。GitHub Releases・タグ・attestation を取りに行く通信は、[ADR 0012](0012-production-topology-single-host.md) が外向きの通信を 2 つの経路に限っていることへの、デスクトップアプリの配布だけに使う追加の経路である。この通信はホストの取得の処理だけが行い、server・runner・gateway などの構成要素には与えない。取りに行く仕組み（ADR 0012 の systemd タイマーなど）の実装は別の Issue にする。外部への接続になるので、着手には hatai の許可が要る。
- **CI の必須チェック**：既存の `verify.yml` の必須チェック名「mise run verify」は変えない。client の検査は `mise run verify` の下に足し、同じジョブで回す。
- **workflow の変更の扱い**：`verify.yml`・`nightly.yml`・`dependabot.yml` の変更と `desktop-release.yml` の追加は、hatai の admin マージが要る別の Issue にする。
- **agent の運用とのつなぎ**：実装は worktree の T3 スレッドで行い、PR とレビューは `scripts/agent-gh` を使う。署名の鍵と environment（署名を入れるとき）、タグの push、公開、配布用の読み取り専用トークンの作成とホストへの登録は hatai が行う。
- **Node とパッケージマネージャ**：Node は 26.13.1（LTS）、パッケージマネージャは bun の安定版 1.4.5 にし、どちらも `mise.toml` で固定して `mise-bump.yml` で上げる。mise の Node は開発の道具（electron-vite・Vite・Octane のコンパイラ）を動かすためのもので、アプリの中で動く Node は Electron に同梱のもの。install・`bun fmt`・`bun check`・`bun test` は mise の bun で回す。
- **依存の版**：版は `exact` で固定し、bun の catalog で `client/package.json` の 1 か所にまとめる。`client/bunfig.toml` の `minimumReleaseAge` で、公開から 7 日以上経った版だけを入れる。例外は `octane`・`@octanejs/electron`・`@octanejs/testing-library`・`@tsrx/oxc`（Octane の一式。ほぼ毎日出る beta で、7 日待つと peer の組がそろわない）と、`bun` とその OS ごとの本体 `@oven/bun-<OS>`（下の lint の canary）だけ。`minimumReleaseAgeExcludes` は glob を受け付けないので、包みの名前で並べる。TypeScript は Octane の peer の範囲に入る 6.0.3 にする。
- **ビルド**：`client/apps/desktop` を electron-vite で作り、renderer の設定に Octane の Vite プラグインを入れる。preload は `sandbox: true` で読めるよう 1 つのファイルにまとめる。
- **lint と format**：lint は `bun lint`、format は `bun fmt` にする。`bun lint` は bun の安定版にまだ無いので、日付つきの canary 版（npm の `bun@<版>-canary.<日付>.<n>`）を `client/package.json` の devDependency にして lockfile で固定し、`bun run lint` から呼ぶ。依存の postinstall は走らせない（`client/bunfig.toml` の `ignoreScripts = true` と、`client/package.json` の `trustedDependencies: []`）。canary の postinstall は本体が無いと lockfile で照合せずに取り直すので、使わない。代わりに `client/scripts/canary-bun.ts` が、lockfile の sha512 で照合して入った本体 `@oven/bun-<OS>` を直接起動し、本体が見つからないときと `client/node_modules` の外にあるときは失敗する。`bun lint` が安定版に入ったら、mise の bun を上げて canary の依存を外す。`bun fmt` の対象は `client/` の下だけにし、`hk.pkl` の pre-commit で `bun fmt --check`（直さずに検査だけ）を回す。typos は今のまま全体にかける。
- **型検査**：`bun check` で回す。Octane の JSX の型を `bun check` が扱えないときは、Octane の `octane-tsc` に替える。
- **単体の試験**：`bun test` で回し、`*.test.ts`・`*.test.tsx` を対象のファイルの隣に置く。Octane の部品は、Octane のコンパイラを `environment: 'client'` で通す bun のプラグインと、happy-dom・`@octanejs/testing-library` で試験する。main の試験では `electron` を読み込まず、対象の関数にセッションや WebContents の代わりの物（試験に要るメソッドだけを持つ物）を渡す（`client/apps/desktop/src/main/security.test.ts`）。
- **client の検査**：`mise run client:verify`（`bun install --frozen-lockfile`・`bun fmt --check`・`bun run lint`・`bun check`・`bun test`）にまとめ、`mise run verify` から呼ぶ。

### 運用の Issue で決める案

次は案で、Electron の土台の後の運用の Issue で採るかを決める。

- **パッケージ作り**：electron-builder で包む。作るものは Windows の NSIS、macOS の dmg と zip、Linux の AppImage。OS ごとに GitHub の runner を分ける。作ったあとの検査（fuse の値、asar の完全性、ライセンスの一覧、sha256、更新の manifest、Linux は xvfb の下での起動）を `mise run desktop:verify-package` にまとめる。
- **段階的な公開**：更新の manifest の `stagingPercentage` を使う。割合だけを変え、SHA-512 と大きさが変わらないことを確かめるタスク（`mise run desktop:rollout`）を用意する。Caddy は `latest*.yml` をキャッシュさせず（`no-store`）、本体は版ごとに名前を変えて上書きしない。
- **リリースと版**：SemVer。版は `client/apps/desktop/package.json` に持ち、サーバーの crate の版とは分ける。タグは `desktop-vX.Y.Z`。変更点は PR ごとの断片（`client/changelog.d/`）に書き、リリースの準備の PR で `client/CHANGELOG.md` にまとめる。release の workflow は、最初にタグと package.json の版が合うかを確かめ、タグの実行を 1 列に並べる。配る本体ごとに artifact attestation を付ける（「決定」の配布経路で、ホストが確かめるもの）。preflight のチェックリストを `mise run desktop:preflight` にする。
- **CI の中身**：electron-vite のビルド、preload の検査、`pelway-contract` から書き出した TS の型の差分を、`client/` か `crates/pelway-contract` が変わったときだけ回す（lint・format・型検査・単体の試験は「決定」の client の検査で回す）。3 つの OS の署名なしのパッケージ作りと起動の検査は `nightly.yml` に足す。リリースは新しい `desktop-release.yml`（タグと手動の dry-run）。actions は今と同じく SHA で固定する（`actions.lock`）。
- **E2E**： Playwright の Electron 対応で作ったアプリを動かし、サーバーは `pelway-contract` の fixture を返す偽のサーバーにする。本物のサーバーにつなぐ試験は手動のときだけ回す。Windows のインストーラーと更新の E2E は、配布を始めるときに足す。
- **lint の追加の検査**：使っていないコードの検出（knip）と、型の抜けを「減る方向にだけ」縛る仕組みは、土台ができてから決める。
- **依存の更新**：`.github/dependabot.yml` に npm の項目を足し、今の cargo・actions と同じく cooldown を 7 日にする。Electron の major は別のグループにする。`osv-scanner.toml` の対象に JS の lockfile を足す。npm の依存の監査とライセンスの検査は、cargo-deny と別に足す。
- **クラッシュとログ**：main のログは秘密（トークン、Authorization など）を伏せてから、ユーザーデータのフォルダに回しながら書く。クラッシュダンプは手元に残すだけで、外へ送らない。終了の記録で「きれいに終わらなかった」ことを次の起動で見分け、画面に出す。不具合のときは、利用者が「診断情報を書き出す」でログとダンプを zip にして手で渡す。D3 では分析のイベントを外へ送らない。
- **開発環境**：`mise run client:dev` で、ローカルの Pelway サーバーと Electron の開発版を一緒に起動する（常駐させるものは `pitchfork.toml` で扱う）。`mise run client:bootstrap` で依存を入れる。画面の確認は開発版で行う。
- **リリースの通知**：`notify-grokbot.yml` の仕組みにリリースのイベントを足す。`docs/agents/notify-payloads.md` の変更になるので、手順書の変更として hatai の承認が要る。

## Electron のセキュリティの既定

Electron 公式の[セキュリティチェックリスト](https://www.electronjs.org/docs/latest/tutorial/security)に従う。画面は同梱した自前の資産だけを読み、外部のページを窓に出さない。

| # | 公式の項目 | 採否 | Pelway での形 |
|---|---|---|---|
| 1 | Only load secure content | 採る | 読むのは同梱の資産と自前の server（https、または開発時の loopback）だけ |
| 2 | Do not enable Node.js integration for remote content | 採る | すべての窓で `nodeIntegration: false` |
| 3 | Enable Context Isolation | 採る | `contextIsolation: true` |
| 4 | Enable process sandboxing | 採る | `sandbox: true` |
| 5 | Handle session permission requests from remote content | 採る | `setPermissionRequestHandler` で、使う権限（通知など）だけを許し、ほかは拒否 |
| 6 | Do not disable `webSecurity` | 採る | 変えない |
| 7 | Define a Content Security Policy | 採る | `script-src 'self'`。renderer から直接 fetch する接続先は CSP の `connect-src` で自前の server だけに絞る |
| 8 | Do not enable `allowRunningInsecureContent` | 採る | 変えない |
| 9 | Do not enable experimental features | 採る | 変えない |
| 10 | Do not use `enableBlinkFeatures` | 採る | 使わない |
| 11 | Do not use `allowpopups` for WebViews | 採る | `<webview>` を使わない |
| 12 | Verify WebView options before creation | 採る | `<webview>` を使わないので、作られようとしたら止める |
| 13 | Disable or limit navigation | 採る | `will-navigate` で同梱の資産以外への遷移を止める |
| 14 | Disable or limit creation of new windows | 採る | `setWindowOpenHandler` で新しい窓を開かせない |
| 15 | Do not use `shell.openExternal` with untrusted content | 採る | チャットのリンクを外で開くのは https だけ。AI Bot の発言のリンクは確認を挟む。`@octanejs/electron` の組み込みの `octane:*` のチャンネル（URL を確かめずに `shell.openExternal` を呼ぶ）は登録しない |
| 16 | Use a current version of Electron | 採る | サポート中の版を使い、依存の更新の流れに乗せる |
| 17 | Validate the `sender` of all IPC messages | 採る | main の IPC ですべて送り元を確かめる。送り元を確かめない `@octanejs/electron` の main の登録（`registerOctaneElectronMain`）は使わない |
| 18 | Avoid usage of the `file://` protocol | 採る | 同梱の資産は独自のスキームで出す |
| 19 | Check which fuses you can change | 採る | `RunAsNode` など使わない fuse を切り、asar の完全性の検査を有効にする |
| 20 | Do not expose Electron APIs to untrusted web content | 採る | preload の `contextBridge` で出すのは、`packages/contracts` で宣言したチャンネルだけを通す `invoke`/`on` の橋 1 つ。`ipcRenderer` をそのまま出さない |

画面と server の業務通信は、renderer から直接 fetch する。server は CORS の Origin 許可リストで Electron の renderer を限定し、renderer は CSP の `connect-src` で接続先を自前の server に絞る（#817）。現時点の土台の CSP には `connect-src` がまだ無く、通信の実装も未完成なので、この方針を実装済みとは読まない。server 側の検査（Origin、CSRF、WebSocket の認証。v0.7 [04](../technical/v0.7/body/04-data-api-realtime.md)）は、Electron から来る要求にも省かない。

## 配布とライセンス表示

- 3 つの OS の配布物は CI の同じ sha から作り、sha256 を一緒に出す（#551 の条件を Electron 向けに書き換えて引き継ぐ）。
- About から開くサードパーティライセンスの一覧には、Electron・Chromium・Node.js と npm の依存を入れる。画面側に Rust を残さない（共通核は TypeScript に移す）ので、Rust の依存（cargo-about）は含めない。
- Slint の Royalty-free の条件（AboutSlint の表示）は、Slint を廃止した後は要らない。依存が残る間は `deny.toml` の台帳を維持し、#826 で依存と一緒に外す。
- D1 は署名なしの開発版を `mise run client:dev` で動かす。D1 の配布物は作らない。D3 の配布では最初は署名なしで行い、署名付きの配布物と自動更新を D3 の対象として整える。自動更新は Windows と Linux だけで、配布経路は国内ホストが GitHub Releases を取りに行く形（「運用の仕組み」の「決定」）。署名の選択肢の調査は #723 のコメント（`adr-0014-research`）にある。

## Slint の廃止の時期

D1 の中で #766 の子 Issue の順に廃止する。それまで `pelway-client-ui`・`dev-client-probe`・`deny.toml` の slint の台帳（#229）は今のまま残す。

1. #825 で `dev-client-probe` を消す
2. #826 で `pelway-client-ui` を消し、`deny.toml` の slint の台帳とライセンスの例外、`.agents/skills/slint/` を外す
3. #752 の TypeScript 版がマージされた後、#827 で Rust 版 `pelway-client-core` を消す
4. #828 で workflow の Slint のビルド用 `libfontconfig1-dev` を外す（workflow の変更なので hatai が扱う）

## 未決

- 署名を入れる時期と方式（Windows の証明書、macOS の Apple Developer Program と notarize）。入れると決めるのは hatai。決めたら、残してある置き場所（タグのときだけ使える environment）に鍵を登録し、macOS の自動更新も足す。

## v0.7 との関係

[技術仕様 v0.7](../technical/v0.7/README.md) の本文は消さず、この ADR で置き換えた箇所（08 の U05・A1・A2、02 の冒頭・「ネイティブUI」・クライアントの行・「Discordの事例」、07 の比較とメモリ目標）に注記を入れる。v0.7 のそれ以外（サーバー側、データ、承認、国内完結）は変えない。
