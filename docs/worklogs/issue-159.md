# Issue #159 の作業ログ

## 対象と方針

`cargo llvm-cov -p pelway-http -p pelway-gateway --no-cfg-coverage` の LCOV から未カバー行を洗い出し、到達可能な分岐をテストで潰す。到達不能な分岐（Mutex poison・listener 失敗・テスト内の失敗時メッセージ行）は対象外とした。

| ファイル | 計測前 | 計測後 |
|---|---|---|
| pelway-http/src/lib.rs | 721/822（87.7%） | 796/822（96.8% 見込み） |
| pelway-http/src/log.rs | 172/186（92.5%） | 183/186（98.4%） |
| pelway-http/src/trace.rs | 79/92（85.9%） | 94/101（93.1%） |
| pelway-gateway/src/mcp.rs | 60/74（81.1%） | 70/74（94.6%） |

## 追加したテスト

- `crates/pelway-http/tests/edge.rs`（新規）: `ServeError`/`HttpError` の Display・`http_status` 全バリアント、`serve_with_store`・`serve_with_logger_urandom`・`serve_with_store_and_logger_urandom` の公開入口（ループバック受理・非ループバック拒否）、生 TCP でのプロトコルエラー（ヘッダー途中 EOF=400・行数超過=431・コロン無し行=400・Content-Length 無し=400・本文途中 EOF=400・未知経路=404・メソッド違い=405・空 {card_id}=404・全体期限経過=408・上限超過本文=413+drain）、comments/approve/handoffs の必須ヘッダー欠落・JSON 破損=400、card_not_found/not_awaiting_approval の結果コード経路、根拠記録一覧の actor 欠落・撤回 JSON 破損=400、`FileSink` の Debug/オープン失敗時の無視、`Pseudonym` の Display、`Logger` の Debug
- `crates/pelway-gateway/tests/mcp_edges.rs`（新規）: `GatewayError` 全バリアントの Display、tools/call 結果にテキスト以外（画像）ブロックが混じったときの絞り込み
- `crates/pelway-http/src/trace.rs`（unit test 2 件）: `init_local_tracing` の 2 回目は false、`MemWriter` の flush

## 残した未カバー（到達不能・非現実経路）

- `lib.rs` 425: `listener.incoming()` の Err で `continue` — 実ソケットで accept 失敗を安定して起こせない
- `lib.rs` 507, 543, 550: `read_timeout_for` の「残りちょうど 0」、read の WouldBlock/TimedOut 以外のエラー — `std::net::TcpStream` では RST（SO_LINGER=0）を送れない（`set_linger` は unstable）
- `lib.rs` 620: ヘッダー内の空行 `continue` — 終端 `\r\n\r\n` が先に切れるため構造上到達不能
- `lib.rs` 984, 1039, 1059, 1070, 1095, 1122, 1162, 1172, 1196, 1222: `Mutex` poison・`create_*` で「起こらない」`StoreError::NotFound` の安全側 — poison の注入は他テストを汚染する
- `lib.rs` 1297: 既存テストの panic 腕（失敗時にだけ評価される行）
- `log.rs` 141: `ensure_open` 末尾の `guard.is_some()` — llvm-cov の行領域が失敗経路にだけ残る計上特性
- `trace.rs` 145, 159, 166-167: 既存テストの assert 失敗メッセージ評価・manifest が無いエントリの `continue` — 失敗時にだけ評価される行
- `mcp.rs` の残り 4 行: `DA` レコードに現れない関数領域の計上差分

## 実装者メモ（round 1）

1. 詰まった箇所: 手順 4（テスト追加）: `ServeError` の `expect_err` が `Server: Debug` を要求してコンパイルエラー（`.err().expect()` で回避）、`TcpStream::set_linger` が unstable で RST による read エラー注入を諦めた（550 の `Err(_) => bad_request` は残置）
2. 曖昧だった文: 「未カバー行を実質的に潰す」（Issue #159「受け入れ条件」）
3. 仮定と解釈: Mutex poison・listener 失敗・テストの失敗時メッセージ行は「実質的に潰す」の対象外と解釈した。全体期限の 408 は `Duration::ZERO` 注入で決定的にした（1ms では残り>0 で Some 経路になり `read_timeout_for` の None 腕を踏めない）
4. 足りなかった情報・検証手段: `cargo llvm-cov` の `LF`/`LH` がライブラリ側とテストバイナリ側の二重インスタンス化で分母を膨らませる特性は、Issue #158 で確認済みだったため今回は行レベル（`DA`）だけを見た
5. 改善案: `AGENTS.md` のテスト方針に「到達不能分岐（Mutex poison・失敗時メッセージ）はカバレッジ目標の対象外として作業ログに残す」と足す文の候補
