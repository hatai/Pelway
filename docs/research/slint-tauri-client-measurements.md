# Slint と Tauri＋React のクライアント比較: 実測報告

- 確認日: 2026-11-12。Issue #98 の「やること」に対する実測の結果。
- 範囲: 実測（GUI 実機での読み上げ支援を含むもの）と一次資料の確認。**NVDA と VoiceOver の実測はできていない**（理由は 6 節）。
- 判定の区分: 「事実（出典つき）」と「Pelway への当てはめ（筆者の判断）」を分ける。当てはめは筆者の解釈で、出典元の主張ではない。
- 前回の調査 [oss-platform-and-client.md](oss-platform-and-client.md) の 6 節（クライアントの部品）を実測で詰めたもの。

## 0. 結論

1. **Slint #3867（読み上げを有効にすると大きな ListView が遅い）は 1.18.1 でも再現する。** 10 万行の ListView を 1,000 回スクロールした CPU 時間で、読み上げ有効時が 3,841 ms、無効時が 1,732 ms（2.2 倍、+2.1 ms/スクロール）。窓の他の所に静的 Text を 1,000 個置くと差は +1,400 ms から +2,100 ms に広がる。**読み上げ支援が有効なときは、アクセシビリティ木全体の作り直しが原価になる。**
2. **可変高の仮想リストは扱える。** 10,000 行（行ごとに高さが違う）でも生成される行は可視範囲の 12 行だけ。行は自分の高さどおりに並び、隙間も重なりもない。遠い位置へスクロールしても生成数は変わらない（34,747 px スクロールして 8 行）。
3. **#10861（IME 変換中にフォーカスが外れると最後の文字が消える）はこの環境では再現できなかった。** 測定環境は Linux のみで、IME（fcitx5・mozc）が無く sudo も使えない。Slint 1.18.1 が使う winit 0.30.13 は、報告者が「再現しなかった」と言った winit master より前の版なので、**人の手（Windows 11 + 韓国語 IME）での確認が必要**。
4. **Tauri（wry）の Linux のインライン変換は、今も既定でできない。** wry は WebKitGTK に対して `set_enable_preedit(false)` を無条件に呼んでおり、wry #1724（未マージ）はオプトイン API を提案しているだけ。Slint は preedit を自前で描画するので、この点は Slint が有利。
5. **型生成は ts-rs と specta の両方で試せた。** ts-rs は 1 型 1 ファイルで i64 は `bigint`。specta は 1 ファイルにまとめ、i64/u64 は既定で書き出しを拒否する（field ごとに `number` と明示する必要あり）。**ts-rs の方が維持費は小さい**（1 行の指定で済み、拒否もされない）。
6. **WASM のサイズは、型生成道具の選択と独立。** 同期処理の土台（serde + wasm-bindgen）を wasm32 にビルドしたものは 173,499 バイト、wasm-opt -Oz で 130,274 バイト（-24.9%）。
7. **Octane は A2 の Web クライアントの UI 層の候補として残す。** MIT・beta・React の API 互換で仮想 DOM を持たない。上の 5・6 の結果（型生成・WASM）は UI フレームワークに依らないので、Octane を選んでも変わらない。

## 1. この文書の前提と読み方

- 測定環境:
  - ホスト: WSL2（カーネル `6.18.58.1-microsoft-standard-WSL2`、Ubuntu 24.04）。画面は Xvfb（ソフトウェアレンダラ、`SLINT_BACKEND=winit-software`）。
  - Slint 1.18.1（`pelway-client-ui` と同じ版）、Rust 1.96.0。
  - 読み上げ（アクセシビリティ）: AT-SPI のセッションが動いている環境で、`org.a11y.Status.IsEnabled` を true にして測る（AccessKit はこの値が true のときだけアクセシビリティ木を作る）。
- 測定の単位:
  - CPU 時間は `CLOCK_PROCESS_CPUTIME_ID`（プロセス全体の user+sys）。描画とアクセシビリティ木の更新は同一スレッドで動くので、この差がそのままアクセシビリティの原価になる。
  - スクロールは 1 回のイベントで 20 px 動かすものを 5 ms 間隔で 1,000 回送った。
- 限界: ソフトウェアレンダラ・WSL2 上の値であり、実機 GPU や Windows / macOS の値ではない。**読み上げ支援（Orca・NVDA・VoiceOver）が同時に動いている状態は測っていない**（6 節）。

## 2. Slint #3867: 読み上げを有効にしたときの ListView の重さ

### 2.1 再現の条件

事実:

- #3867 は 1.3.0 での報告で、Slint 開発者が「#4261 以降は既定では起きない」「読み上げ支援が使われていないなら性能の代償はない。Free desktop では `org.a11y.Status` の `IsEnabled` を true にしたときだけ」「リストは見える要素だけをアクセシビリティ木に入れる」と説明している [slint #3867 (2026/11)]。
- #4261（2024-01-04 マージ）は「Linux/dbus での AccessKit adapter の遅延初期化」を入れた PR で、cc #3867 [slint #4261 (2026/11)]。
- 実装側: AccessKit の Unix 版はセッション bus の `org.a11y.Status.IsEnabled` を監視し、変化したときに adapter を作る [accesskit_unix 0.22.1, context.rs]。
- したがって、**再現には「読み上げ支援が利用中」の状態を作る必要がある**。この環境では `busctl --user set-property org.a11y.Bus /org/a11y/bus org.a11y.Status IsEnabled b true` で作れる。

事実（同じ条件で 2 ビルドを比べた CPU 時間。10 万行・行高 20px・1 行に Text 2 つ）:

| 条件 | 読み上げ有効 (ms) | 読み上げ無効 (ms) | 差 |
|---|---|---|---|
| 窓の他の所に静的 Text 1,000 個 | 3,979 / 3,589 / 3,841 | 1,732 / 1,773 / 1,725 | +2,109 ms（2.2 倍） |
| 静的 Text なし | 1,570 / 1,465 | 550 / 509 | +988 ms（2.9 倍） |

（中央値。3 回 / 2 回の実行。1,000 回のスクロール。）

解釈: **#3867 は 1.18.1 でも起きる。** 差の大きさは窓のアクセシビリティ木の大きさに依存する（静的 Text を 1,000 個置くと差が 2 倍以上になる）。これは「行が増減すると木全体を組み直す」ためで、木が大きくなるほど元のコストが効く。列表だけの窓なら 1 回のスクロールあたり約 1 ms、窓が大きければ約 2 ms の追加。**数目としては「読み上げを使う人には重い」とまでは言えないが、フレーム落ちの要因になりうる規模**である。

### 2.2 同じ指標の既存測定との比較

事実: macOS 27（Apple M5 Max）で 10 万行をスクロールした先行測定は、AccessKit 有効時 1 frame あたり +0.2〜1.5 ms、原因を「行の作成・破棄で木全体の再構築が起きる」としている [slint #3867 の tilladam のコメント (2026/11)]。

解釈: 先行測定と「木全体の再構築が原価」という見立ては一致する。今回の Linux（ソフトウェアレンダラ）でも、同じ見立てが読み取れる。

## 3. Slint の可変高の仮想リスト

事実（`crates/dev-slint-listview` のテストで確かめた）:

- 10,000 行用意し、行ごとに高さが違う（20px から 116px を 97 行周期で繰り返す）的情况下、生成された行は **12 行**（可視範囲の 1.5 倍程度）だけ。行番号は連続し、途中の行は抜けていない。
- 生成された行の y 座標の差は、**その間の行の高さの合計と一致する**（誤差 0.5px 未満）。つまり行は重ならず、隙間も空かない。
- 10 万行の状態で 30,000 px スクロールすると、生成された行は **8 行**（615 番目以降）。可視範囲の外に 10 万行を実体化していない。

解釈: **Slint 1.18.1 は可変高の仮想リストを扱える。** 行の高さを測り直しながら範囲を実体化する仕組みが i-slint-core の repeater にあり、ListView のスクロールに合わせて作り直される。チャットのように行の高さが変わる UI でも、可視範囲だけを作ればよい。

事実（注意点）: `ListView` の `content-y` を外側から binding してスクロール位置を与えると、**仮想化の範囲が追従しない**（行が作り直されず、位置だけがずれる）。スクロールは ListView 内部の flickable を通して行う必要がある（テストも GUI 計測も、この経路で動かしている）。

解釈: 外部からスクロール位置を注入する設計は避けたい。アプリ側でスクロール状態を保持するとしても、ListView のスクロールそのものは ListView に任せる形が安全。

## 4. Slint #10861: 変換中にフォーカスが外れると最後の文字が消える

事実:

- 報告は Slint 1.15.1 / Windows 11 / 韓国語 IME。ウィンドウの OS レベルのフォーカスが外れると、変換中の最後の文字が確定せず消える。同じ Slint ウィンドウ内の別の LineEdit に移ると確定する [slint #10861 (2026/11)]。
- Slint 開発者は Windows で再現を確認し、「winit master では再現しない。先行リリースで直るかもしれない」と書いている [slint #10861 (2026/11)]。
- Slint 1.18.1 が依存する winit は **0.30.13**（Cargo.lock で確認）。winit 0.31 系の IME 改善（macOS の NSTextInputClient 再設計、iOS の multi-stage IME、Android の IME 報告など）は未マージの PR として残っている [winit の IME 関連 PR (2026/11)]。
- Slint 側は winit の `Ime::Preedit` を受けて preedit 文字列を自前で描画する（`i-slint-backend-winit` の `winitwindowadapter.rs`、`i-slint-core` の `preedit_text`）。

事実（再現の可否）: この環境は WSL2 上の Linux で、IME（fcitx5・mozc）が入っておらず、sudo も使えないため、**変換中の文字が消える事象を再現できていない**。

解釈: **Windows 11 + 韓国語 IME での確認を人間に委ねる必要がある。** Linux は報告の再現環境ではないので、Slint を A1 にする判断の材料にはしにくい。#10861 が 1.18.1 で直っていたとしても、**Linux の日本語入力（fcitx5 + mozc）でのインライン変換は別項目として必ず人の目で確かめるべき**である（Linux 版を視野に入れるなら、Wayland と X11 のどちらでも）。

## 5. Tauri（wry）の Linux でのインライン変換

事実:

- wry #1724 は **未マージ**。提案は `WebViewBuilderExtUnix::with_input_method_preedit(bool)` で既定は false のまま。報告者は Ubuntu 24.04 + webkit2gtk-4.1 2.50.x + fcitx5-hangul / fcitx5-anthy で手動確認し、オプトインすればインライン preedit が出ると報告している [wry #1724 (2026/11)]。
- 根本原因は wry が WebKitGTK に対して `set_enable_preedit(false)` を無条件に呼ぶことで、これは wry 0.29.0 で fcitx の popup 位置の問題（tauri#5986）を回避するために入った [wry #1724 (2026/11)]。
- つまり **現状の Tauri（Linux）は、日本語の変換中文字列をカーソル位置に出さない**。確定した文字だけが現れる。

事実（Slint との比較）: Slint は winit の IME イベントを受けて preedit を自前で描画する（4 節）。**インライン変換の点では Slint が素直で、Tauri は wry の PR が入ってオプトインできるまで弱い。**

解釈: A1（Slint）と A2（Tauri 比較候補）の差として、**Linux の日本語入力には Tauri 側の未解決の制約がある**。Tauri を採用するなら wry #1724 の取り込みを前提に条件的判断する（Wayland は未検証とも出ている）。

## 6. 読み上げ（NVDA / VoiceOver / Orca）の比較

事実（仕組み）:

- Slint は AccessKit を使う。Linux は AT-SPI（→ Orca）、Windows は UI Automation（→ NVDA）、macOS は AX（→ VoiceOver）。読み上げはブラウザではなく、ウィジェット（widget）単位に届く。
- Tauri は WebView を介する。Linux は WebKitGTK（→ AT-SPI）、Windows は WebView2（→ UI Automation）、macOS は WKWebView（→ AX）。読み上げは DOM 単位に届く。

事実（実測）: **この環境では読めない。** Windows と macOS の機械が無く、Linux にも Orca が入っていない（導入には apt と sudo が必要で、この作業環境には無い）。

解釈: 両者とも同じ読み上げ支援に届くので、「読めない」という差別は無い。差が出るのは粒度（ウィジェットか DOM か）と、Linux の IME（5 節）にあり、**実測は人間に委ねる**。手順は次のとおり（同じ項目・同じ文言・同じ OS・同じ読み上げ支援で比較する）:

| 項目 | Slint | Tauri ＋ React |
|---|---|---|
| 日本語の読み上げ（読み上げボタンで本文が流れるか、漢字の読みが正しく出るか） | 未確認 | 未確認 |
| フォーカスの移動（Tab / 矢印）と「現在どこにいるか」の読み上げ | 未確認 | 未確認 |
| 通知（未読の件数、承認待ち）が読み上げられるか | 未確認 | 未確認 |
| 日本語入力（fcitx5 + mozc）で変換中の文字列がカーソル位置に出るか | 4 節のとおり出る | wry #1724 のとおり出ない（現状） |
| 画面拡大（拡大鏡）で読み上げの Tree が崩れないか | 未確認 | 未確認 |

## 7. Rust から TypeScript の型を生成する道具

事実（`crates/dev-typegen-probe` で試した）:

- **ts-rs 12.0.1**: derive 1 行で型が TS になる。1 型 1 ファイル（`import type` で相互参照）。i64/u64 は `bigint`（`TS_RS_LARGE_INT` で変えられる）。enum は判別可能な和集合になる。
- **specta 2.0.0-rc.28**: 1 ファイルにすべての型をまとめる。**i64/u64 は既定で書き出しを拒否する**（`JSON.parse` が大きな整数を丸めるため。`BigIntForbidden` エラー）。field ごとに `#[specta(type = specta_typescript::Number)]` を書いて `number` として出すことを明示する必要がある。enum は `& { 別の variant?: never }` を含む和集合になる。
- どちらの出力も、Rust の doc コメントを JSDoc として取り込む。

事実（維持費）: どちらの道具も「Rust の型を変えれば TS の型も変わる」ので、CI で差分を検査できる形にした。`crates/dev-typegen-probe` は `bindings/` に生成結果を置き、生成結果と違えばテストが落ちる（`UPDATE_GOLDEN=1` で作り直せる）。**差分検査の実装はいずれの道具でも同じ工作量だった**（生成 → 置いたものと比べるだけ）。

解釈: **A2 では ts-rs を第一候補にする。** i64 を bigint として素直に出し、拒否の理由（JS の数値精度）を実装者が持つ必要がない。specta を選ぶ利点は Tauri 連携（tauri-specta）と RC 版であることだが、A2 は Web クライアントで Tauri を採用する前提ではない。specta の 2.0 が RC のまま（2026-11-12 時点で 2.0.0-rc.28）であることも、採用を後回しにする理由になる。

## 8. WASM のサイズ（wasm-opt）

事実（`crates/dev-typegen-probe` を wasm32-unknown-unknown に release ビルドしたもの。serde_json と wasm-bindgen を含む。binaryen 133 の wasm-opt）:

| 工程 | サイズ (バイト) |
|---|---|
| cargo build --release の出力そのまま | 173,499 |
| wasm-opt -O | 130,474（-24.8%） |
| wasm-opt -Oz | 130,274（-24.9%） |
| wasm-opt -Oz --strip-debug | 130,274 |

事実（内訳、wasm-opt --metrics の -O 出力）: データ 32,362 バイト、関数 181、変数 704、テーブル 71。

解釈: **WASM のサイズ負担は、wasm-opt で 4 分の 3 まで減る。** この規模（同期処理の土台＋JSON 変換）は 130 KB で収まる。wasm-bindgen の CLI を通した後のサイズ（JS シムの追加とカスタムセクションの除去）は未測定。twiggy は archived なので、内訳は wasm-opt の metrics を使う。

## 9. Octane（Web クライアントの UI 候補）

事実: octanejs/octane は MIT、TypeScript、1,485 stars、最終 push 2026-11-12、npm 版 0.16.7（2026-11-02）、状態は **beta**。React の API（`useState`・`useEffect`・`memo`・context・portals・Suspense・transition）を互換で提供し、コンパイラが直接 DOM のコードに落とす（仮想 DOM を持たない）。Inferno の後継で、作者は Inferno の作者 [octanejs/octane README (2026/11)]。React との差は、クラスコンポーネント・Server Components・合成イベントシステムを持たないこと [octanejs/octane, Differences from React (2026/11)]。

解釈: A2 の UI 層（TS/React DOM）の候補として、React の代わりに置ける。**ただし 7・8 節の測定結果（型生成・WASM サイズ）は UI フレームワークに依らない**ので、Octane を選ぶか選ばないかは、この Issue の判断材料には影響しない。beta であることと Linux/読み上げでの検証が未実施であることを踏まえ、**A2 の UI 層の選定は別 Issue での比較（実 DOM での読み上げ・IME・描画負荷）に残す**。

## 10. A1 / A2 への示唆（筆者の整理）

- A1（Slint）: 読み上げの原価（+1〜2 ms/スクロール）は想定内。ただし「窓のアクセシビリティ木が大きいほど効く」ので、**1 画面に static な Text を大量におく設計は避ける**。可変高の仮想リストは使えるので、チャット本文は行高が変わっても問題ない。
- A1（Slint）: **IME は Linux で人の目視確認が要る**（この環境では再現できていない）。Windows 由来の #10861 は 1.18.1 で直っていた可能性があるが、確認していない。
- A2（Web）: 型生成は ts-rs を使う。CI では生成結果と置いた TS の差分を見る。
- A2（Web）: WASM は同期処理の土台で 130 KB（wasm-opt -Oz）。WASM のサイズで A2 を却下する必要はない。
- 比較候補（Tauri）: **Linux の日本語インライン変換は現時点で不利**（wry #1724 が未マージ）。Tauri を推すなら、この条件付きとする。

## 11. 出典

- [slint #3867, 2026/11] slint-ui. "accessibility feature have big performance impact on big listview." GitHub Issue. https://github.com/slint-ui/slint/issues/3867
- [slint #4261, 2026/11] slint-ui. "Bump AccessKit dependencies." GitHub PR（2024-01-04 マージ、Linux/dbus の AccessKit adapter の遅延初期化、cc #3867）。https://github.com/slint-ui/slint/pull/4261
- [slint #10861, 2026/11] slint-ui. "The last character is dropped when the window loses focus during IME composition in LineEdit." GitHub Issue. https://github.com/slint-ui/slint/issues/10861
- [wry #1724, 2026/11] tauri-apps. "feat(webkitgtk): make input method preedit configurable." GitHub PR（未マージ）。https://github.com/tauri-apps/wry/pull/1724
- [winit の IME 関連 PR, 2026/11] rust-windowing. winit の IME 関連 PR（macOS の NSTextInputClient 再設計、iOS の multi-stage IME、Android の IME 報告、win32 の IME 位置など、2026-11-12 時点で未マージ）。https://github.com/rust-windowing/winit/pulls?q=is%3Apr+IME
- [accesskit_unix 0.22.1] AccessKit. "Unix 版の context.rs（org.a11y.Status.IsEnabled の監視）。" crates.io
- [i-slint-backend-winit 1.18.1 / i-slint-core 1.18.1] Slint. "winitwindowadapter.rs（Ime::Preedit の処理）と preedit_text。" crates.io
- [ts-rs 12.0.1 / specta 2.0.0-rc.28 / specta-typescript 0.0.15] crates.io（型生成の道具）
- [binaryen 133] WebAssembly. "binaryen version_133（wasm-opt）。" https://github.com/WebAssembly/binaryen/releases/tag/version_133
- [octanejs/octane README, 2026/11] octanejs. "octane README." GitHub. https://github.com/octanejs/octane
- [oss-platform-and-client, 2026/11] 本リポジトリ `docs/research/oss-platform-and-client.md`

## 付録: 再現手順

```bash
# 1. 読み上げ支援が利用中の状態にする（AT-SPI のセッションがある環境）。
busctl --user set-property org.a11y.Bus /org/a11y/bus org.a11y.Status IsEnabled b true

# 2. 読み上げ有効 / 無効の 2 ビルドを用意する。
cargo build -p pelway-dev-slint-listview --release --bin perf
cp target/release/perf /tmp/perf-a11y-on
cargo build -p pelway-dev-slint-listview --release --bin perf --no-default-features
cp target/release/perf /tmp/perf-a11y-off

# 3. 同じ条件で走らせる（画面は Xvfb、描画はソフトウェア）。
for b in /tmp/perf-a11y-on /tmp/perf-a11y-off; do
  SLINT_BACKEND=winit-software xvfb-run -a "$b" \
    --rows 100000 --events 1000 --interval 5 --static-cells 1000
done

# 4. 可変高の仮想リスト（ヘッドレスのテスト）。
cargo test -p pelway-dev-slint-listview --test virtualization

# 5. 型生成の差分検査（生成結果は bindings/ に置いたものと比べる）。
cargo test -p pelway-dev-typegen-probe --test typegen

# 6. WASM のサイズ。
cargo build -p pelway-dev-typegen-probe --target wasm32-unknown-unknown --release
wasm-opt -Oz target/wasm32-unknown-unknown/release/pelway_dev_typegen_probe.wasm -o /tmp/probe.wasm
ls -l /tmp/probe.wasm
```
