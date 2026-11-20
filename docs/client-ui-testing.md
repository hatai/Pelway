# クライアント UI の試験で押さえる Slint の挙動

`crates/pelway-client-ui` の試験（`crates/pelway-client-ui/tests/*.rs`）と `.slint` のレイアウトを書くときに要る、Slint の挙動の事実をまとめる。内容は Slint 1.18.1（`crates/pelway-client-ui/Cargo.toml` の `slint = "=1.18.1"`、`i-slint-backend-testing = "=1.18.1"`）に対するもので、版を上げたら出典のソースで確かめ直す。

出典の `i-slint-core/…`・`i-slint-compiler/…`・`i-slint-backend-testing/…` は、各 crate の 1.18.1 の crate 直下からのパス（`~/.cargo/registry/src/*/<crate>-1.18.1/` で読める）。試験の挙動の仮説は、まずこの文書の該当節で確かめ、ここに無い挙動は crate のソースを読んで確かめる。

## 1. 要素の探索は見えている要素だけを返す

- テスト用 backend の要素の探索（`ElementHandle::find_by_*`・`query_descendants`・`visit_descendants`）は、見えていない要素を結果に入れない。木の下へは降りるが、要素ごとに `ItemRc::is_visible()` を確かめ、偽なら結果に入れない。
  - 出典: `i-slint-backend-testing/search_api.rs` の `visit_descendants_impl`（`if !item_rc.is_visible() { return ControlFlow::Continue(()); }`）、`i-slint-core/item_tree.rs` の `visit_descendants_impl`（`Continue` のとき子へ降りる）。
- `is_visible()` は、祖先の clip を重ねた矩形と要素の矩形が重なるときだけ真になる。Flickable（ListView を含む）の見えている範囲の外にある要素と、窓の外にある要素は見つからない。
  - 出典: `i-slint-core/item_tree.rs` の `ItemRc::is_visible`。
- 窓の下の方や Flickable の中の要素を探すときは、探す前に窓を大きくするか、スクロールして見えている範囲に入れる。窓は `window.window().set_size(LogicalSize::new(w, h))` で変え、変えた後に `i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(0))` でレイアウトを反映させてから探す。
  - 例: `crates/pelway-client-ui/tests/demo_banner.rs` の `find_growing`（見つかるまで窓の高さを伸ばす）。窓の大きさは前の検査の値が残るので、探す前に必要な大きさへ戻す。

## 2. HorizontalLayout の余りの配り方

- レイアウトは、伸びられる要素の stretch の合計に比例して余りを配る。合計が 0（伸びられる要素の stretch がどれも 0）のときは、全員を stretch 1 とみなして余りを等分に配る。stretch 0 の要素も伸びる。
  - 出典: `i-slint-core/layout.rs` の `adjust_items`（`let actual_stretch = |s: f32| if total_stretch <= 0. { 1. } else { s };`）。
- 既定の stretch は要素によって違う。Text と Image は 0、Rectangle などの多くの要素は 1。
  - 出典: `i-slint-core/items/text.rs` の `text_layout_info` の直前のコメント（text と image は既定の stretch 0）、`i-slint-core/items.rs` の各要素の `layout_info`（`LayoutInfo { stretch: 1., .. }`）、`i-slint-core/layout.rs` の `LayoutInfo::default`（`stretch: 0`）。
- 要素を内側のレイアウトで包むと、その stretch は内側の値と自分の値の小さい方になる。Rectangle（1）で Text（0）を包んだ部品は stretch 0 になる。
  - 出典: `i-slint-core/layout.rs` の `LayoutInfo::merge`（`stretch: self.stretch.min(other.stretch)`）。
- 要素が横に伸びるのを止めるには、伸びる側に stretch を足さず、その行で余りを受ける要素を 1 つ決めて `horizontal-stretch: 1` を付ける。行の末尾に受け手が無いときは、末尾に空の `Rectangle { horizontal-stretch: 1; }` を置く。伸ばしたくない要素に `horizontal-stretch: 0` を付けるだけでは、行の全員が 0 のとき等分の配りに戻るので止まらない。
- 試験で「要素 A が要素 B の直後にある」を確かめるときは、A と B のそれぞれの幅が中身相当に収まっていること（余りを分け取っていないこと）を先に確かめる。HorizontalLayout の隣どうしのセルの間は常に spacing になるので、B のセルの右端と A の左端の差だけでは、B のセルが余りで広がっていても通る。
  - 例: `crates/pelway-client-ui/tests/demo_banner.rs` の `assert_badge_packed`（バッジの幅が中身相当であることを確かめてから、直前の要素との差を見る）。

## 3. 文字の大きさと折り返しの試験

- テスト用 backend の文字の計測は、`font-family` が `FixedTestFont` のときだけ固定の計測（1 バイトあたりフォントの大きさ分の幅）になる。それ以外のフォントは、実行環境に入っているフォントで計測する。このリポジトリの UI は `FixedTestFont` を使わないので、文字の幅と折り返しの位置は CI とローカルで違う。
  - 出典: `i-slint-backend-testing/testing_backend.rs` の `FIXED_TEST_FONT`・`is_fixed_test_font` と `RendererSealed for TestingWindow` の `text_size`（固定フォント以外は `sharedparley::text_size`）。
- 折り返し・文字の欠けを確かめる試験は、決まった 1 つの幅だけで確かめない。条件の幅（例: 360px）で確かめたうえで、窓を段階的に狭めて折り返しが必ず起きる幅まで確かめる。
  - 例: `crates/pelway-client-ui/tests/demo_banner.rs` の `persistent_display_end_to_end`（360px の後、幅を 0.7 倍ずつ狭めて行が増えるまで確かめる）。
- `ElementHandle::size()` は、レイアウトが要素に割り当てた大きさ（`width`・`height` の値）を返す。文字の自然な幅ではない。文字が何行になったかは、割り当ての高さを 1 行の高さと比べて読む。
  - 出典: `i-slint-backend-testing/search_api.rs` の `ElementHandle::size`（`item.geometry()` の大きさ）。

## 4. ListView の仮想化

- ListView は、見えている範囲を埋める分の行だけを実体化する。実体化していない行の高さは、実体化した行の平均の高さで推定する。`content-height` は「平均の高さ × 行数」で、実体化した先頭行の一覧の先頭からの位置（アンカー）は「平均の高さ × 先頭行の番号」の推定である。実体化した行どうしの間だけが、実際の高さで並ぶ。
  - 出典: `i-slint-core/model/repeater.rs` の `ensure_updated_listview_callback` の後半（`state.cached_item_height`・`state.anchor_y = state.cached_item_height * state.offset`・`content_height_set(... * row_count)`）。
- 行の座標は 2 つある。`actual-y` は、見えている範囲の上端からの位置（実際に描かれる位置）。`y` は `actual-y - content-y` に束縛され、一覧の先頭からの位置（推定のアンカーを含む）になる。一覧の中の位置を記録するときは `y` を使い、`y + content-y` は見えている範囲の中の位置（`actual-y`）なので一覧の中の位置としては使わない。
  - 出典: `i-slint-compiler/passes/flickable.rs`（ListView の行の `y` を `actual-y - content-y` に束縛）、`i-slint-compiler/generator/rust.rs` の `listview_layout`（`actual-y` に `offset_y` を入れて行の高さ分ずつ進める）、`i-slint-core/model/repeater.rs`（`offset_y` の起点 `new_offset_y` と `new_content_y = -state.anchor_y + new_offset_y`）。
- スクロールは ListView の中の flickable を通して行う。`content-y` を外から binding すると仮想化の範囲が追従しない（`docs/research/slint-tauri-client-measurements.md` の「3. Slint の可変高の仮想リスト」）。試験では `ElementHandle::scroll` でスクロールのイベントを送る。
  - 出典: `i-slint-backend-testing/search_api.rs` の `ElementHandle::scroll`。例: `crates/pelway-client-ui/tests/timeline_virtualization.rs` の `scroll`。
- 「行に抜けが無い」を確かめるときは、実体化した行の番号が連続していることを見る。実体化した行を位置で並べて隣どうしの隙間を見るだけでは、間の行が実体化していなくても通る。
  - 例: `crates/pelway-client-ui/tests/timeline_virtualization.rs` の `timeline_materializes_only_visible_rows_and_stacks_them_at_their_heights`（記録した行番号の連続と、番号で隣り合う行の実位置の隙間の両方を確かめる）。
- 仮想化した一覧の中に出す部品は、`data-*` のプロパティの値だけでなく、要素の探索で部品が実体化していることも確かめる。値が正しくても、行が実体化していなければ画面に出ない。
  - 根拠: この節の 1 つめの項目（実体化するのは見えている範囲の行だけ）と、1 節（探索は見えている要素だけを返す）。

## 5. キー入力を送る

- キー入力は `window.window().dispatch_event(WindowEvent::KeyPressed { text })` と `WindowEvent::KeyReleased { text }` で送る。実際の backend と同じ経路を通り、修飾キーの状態も更新される。
  - 例: `crates/pelway-client-ui/tests/composer.rs` の `press_key`・`release_key`、`crates/pelway-client-ui/tests/accessibility.rs`。
- `i-slint-backend-testing` の `internal` feature の関数（`send_keyboard_string_sequence` など）は使えない。`internal` を有効にすると、公開 crate に含まれないフォント（`../../../tests/screenshots/fonts/`）を `include_bytes!` で読もうとしてビルドが通らない。
  - 出典: `i-slint-backend-testing/lib.rs` の `configure_test_fonts`（`#[cfg(feature = "internal")]`）と `internal_tests.rs`。

## 6. テスト用 backend の初期化

- `i_slint_backend_testing::init_no_event_loop()` は Slint の platform を設定する。platform はスレッドごとに持つので、テストが別のスレッドで走るときはスレッドごとに初期化する（`thread_local!` の `Once`）。プロセス全体の `Once` で 1 回だけ初期化すると、2 つめ以降のテストが別の backend に落ちる。
  - 出典: `i-slint-core/context.rs` の `GLOBAL_CONTEXT`（`thread_local!`）、`i-slint-core/platform.rs` の `set_platform`。
  - 例: `crates/pelway-client-ui/tests/timeline_virtualization.rs` の `init_testing_backend_once`（冒頭の doc コメントに理由）。
