# Issue #103 の作業ログ

## 実装者メモ（round 1）

1. 詰まった箇所: ~/.cargo/registry/src/*/object_store-0.14.2/src/{util,local}.rs: `ObjectStore` trait（0.14）には `put`/`get` が無く、拡張 trait `ObjectStoreExt` に分かれている。また `Error::kind()`/`ErrorKind` も無く、`Error::NotFound` のパターン一致で判別した。ほか crates/pelway-contract/src/ledger.rs の span テストが、テスト全体の並行実行でだけ失敗した（span の macro が大域の callsite interest キャッシュを先に見るため、subscriber 無しの別テストスレッドが先に「無効」をキャッシュすると `with_default` 側で span が消える）。テストバイナリ全体で一度だけ `set_global_default(tracing_subscriber::registry())` する関数をテストに足して直した（`register_dispatch` がキャッシュを再計算するため順序に依存しない）。
2. 曖昧だった文: 「もしローカルにサーバー立てる場合は rustfs がいいのでは」（Issue #103 の hatai のコメント）
3. 仮定と解釈: 本文が「保存先はローカルのままS3互換のインターフェース（object_store crate）で扱う」と決めているため、サーバーは立てないと解釈した（コメントは将来 S3 互換サーバーへ移る場合の指摘と読んだ）。rustfs の採用検討はこの Issue の範囲外。trace id は「カード ID から決定論的に導出」を選んだ（状態を持たず、カード作成時から同じ値。乱数で台帳に持たせる案は採らなかった: スキーマ変更が要り、開始資格・結果照会（未実装）でも同じ導出で揃う）。
4. 足りなかった情報・検証手段: 02 に tracing の出力先（stderr or ファイル）の指定が無い。`cargo info object_store`（MSRV 1.85・MIT/Apache-2.0・crates.io 到達確認）と registry 内の local.rs の `maybe_spawn_blocking` の実装（ランタイム無しではインライン実行）を確認して block_on 駆動を決めた。
5. 改善案: `docs/technical/v0.7/body/02-stack-comparison.md` の「観測」の行に「出力先は stderr・トレースの収集先は未定（U01 の開始資格・結果照会の実装時に決める）」を足す。

## 実装者メモ（round 2）

1. 詰まった箇所: crates/pelway-contract/src/ledger.rs: main（#86）が `begin_approve`/`approve` に `operator: &MemberId` を足し、この PR が同じ関数に span を足していたため 2 箇所で衝突。両方 keep（main の引数 + この PR の span）で解消し、自分のテストの `ledger.approve(req)` を `ledger.approve(req, &decider())` に直した（`reserve` ヘルパーは main 側で既に新引数になっていた）。
2. 曖昧だった文: 「PR #126 は main と衝突していて CI が動かない」（修正依頼の指示文）
3. 仮定と解釈: main との merge commit をこのブランチに積めば GitHub 側の衝突表示と CI が解消されると解釈した（rebase は履歴が already pushed のため選ばなかった）。
4. 足りなかった情報・検証手段: なし（merge → 解消 → `mise run verify` → push で足りた）。
5. 改善案: なし（同じファイルの同じ関数签名を変える変更が並走したこと自体が要因で、手順側の直す箇所は無い）。

