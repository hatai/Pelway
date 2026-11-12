# Pelway 技術調査：RustのLLM APIクライアント比較

## 採否

この調査の結論（案4）に対し、hataiは `rig-core` を選びました（[ADR 0008](../adr/0008-llm-provider-and-byok.md)）。使う範囲はOpenAI互換エンドポイントのchat completionsとembeddingsだけです。以下の結論と比較は調査時点の筆者の整理で、rigを使う際の注意（MSRV、HTTPクライアントの差込み、再試行判定を使わないこと、Z.aiへの `json_schema`）の根拠として残します。

## 結論

LLM gatewayの外部API呼出しは、自前の薄いクライアントを `reqwest` と serde で書く案（案4）を第一候補にします。判定は「MVP前に検証」です。リクエストとレスポンスの型だけは `async-openai` の型専用feature（`chat-completion-types` と `embedding-types`、通信部分を含まない）を借りる案（案4'）も同じ枠で比べます。`async-openai` の通信部分まで使う案は「将来の候補」、genai・rig-core・openai-api-rs は「見送り」とします。

- `async-openai` は既定のクライアントが 429・5xx・接続エラーを最大3回自動で再試行します。この再試行を外す方法は `middleware` featureを有効にして `with_http_service` で自前の送信サービスを差し込むことだけです。
- `genai` には自動再試行がありません。ただし Z.ai アダプタはチャットの送信先を `https://api.z.ai/...` に毎回上書きするので、gateway経由のURLを指定できません。
- `rig-core` 0.46 は MSRV が 1.97.1 で、Pelwayが固定する Rust 1.96.0 ではビルドできません。
- Z.ai の公式APIリファレンスは `response_format` の型に `text` と `json_object` しか載せていません。Vertex AI の OpenAI互換エンドポイントは `json_schema` に対応すると書いています。どのライブラリを使っても、構造化出力はクライアント側でスキーマ検証する必要があります。
- Vertex AI の公式文書には「エンドポイントはデータの所在や地域内でのML処理を保証しない」という注意書きがあります。asia-northeast1 を選べば国内処理になる、という前提は別途確認が必要です（下の「Vertex AI」を参照）。

## この文書の前提と読み方

- 調査日は 2026-11-12 です。crateの版と公開日は crates.io API、スター数・最終push・主な貢献者は GitHub API から、この日に取得しました。
- 各crateのソースは crates.io から該当版を取得し、ファイル名と行番号で引用しています（例：［async-openai 0.43.0, src/executor.rs:144-150］）。行番号はその版のものです。
- 依存数は、リポジトリ外の作業ディレクトリで `cargo tree -e normal --prefix none` を実行し、重複を除いた行数を数えたものです。数にはダミーのルートcrate 1件を含みます。ツールチェーンは Rust 1.99.1 で、Pelwayが固定する 1.96.0 ではありません。依存の解決だけを行い、ビルドはしていません。
- 「事実」は一次情報で確かめた内容、「Pelwayへの当てはめ（筆者の整理）」は筆者の判断です。一次情報で確かめられなかった点には「未確認」と付けます。
- 実際にAPIを呼び出す検証はしていません。
- 提供元の文書は、取得したHTMLを要約して読んだものが中心です。Vertex AI のURL形式・トークン有効期限・`json_schema` 対応・データ所在の注意書きは、原文のHTMLからも該当箇所を確認しました。OpenAI の構造化出力ガイドは、ページの先頭約10万字だけを読んでいます。

### 判定に効くPelway側の制約（仕様と依頼から）

- 外部LLMへの自動fallbackは禁止です［Pelway 10 (2026/11), 本体LLMの入出力行］［Pelway 08 (2026/11), A8］。
- D3「dispatch中のtimeout/retry/runner crash」の期待値は「許可外送信0、結果不明の自動再送0」です［Pelway 10 (2026/11), D3］。タイムアウトは結果の確定ではなく、再送を許可しません［Pelway 03 (2026/11), 通信目的と根拠・開始資格・結果不明の追加契約］。
- 外向き通信は既定で拒否します［Pelway 10 (2026/11), core_domestic_policy］。このため、HTTPクライアント・proxy・timeoutを自前で差し込めることが必要です。
- LLM adapterでrigを使うのは「必要なら小範囲」で、全runtimeは組み込みません［Pelway 02 (2026/11), LLM adapter行］。
- 依頼時点の方針：開発・MVP段階は利用者自身が契約したAPI（Z.ai GLM または OpenAI）をアクセストークンで使うBYOKです。一般利用者の段階では、日本リージョンのある提供元（例：Vertex AI の asia-northeast1）を既定にします。必要な機能はチャット補完（JSONスキーマによる構造化出力を含む）と埋め込みだけです。Pelwayは自前のtraitを持ち、どの提供元を使うかは設定で選びます。テレメトリは送らず、秘密情報はログに残さず、TLSはrustlsを優先します。
- Pelwayのツールチェーンは Rust 1.96.0 に固定されています［Pelway mise.toml (2026/11)］。現時点のワークスペースは `reqwest` に依存していません［Pelway Cargo.lock (2026/11)］。

## 1. 比較表

| 観点 | async-openai | genai | rig-core | reqwest自前（案4） | openai-api-rs |
|---|---|---|---|---|---|
| 版・公開日 | 0.43.0（2026-11-01） | 0.7.4（2026-11-11） | 0.46.0（2026-10-31） | reqwest 0.13.5 | 10.0.2（2026-10-03） |
| リリース頻度（直近） | 2026-09〜11に5版 | 2026-10〜11に0.7.0〜0.7.4（rc含め多数） | 2026-08〜10に5版 | — | 2026年に3版 |
| ライセンス | MIT | MIT OR Apache-2.0 | MIT | MIT OR Apache-2.0（reqwest） | MIT |
| 保守体制 | 64bit 1人が263コミットで大半 | jeremychone 1人が979コミットで大半 | 上位3人が685/249/160コミット、スター約9.1k | 自分たち | dongri 1人が大半 |
| MSRV | 1.75 | 記載なし（edition 2024） | **1.97.1** | — | 記載なし |
| 依存数（下記feature） | 127（rustls+chat+embedding）。型だけなら24 | 146（rustls-tls） | 141（rustls+reqwest） | 86（rustls+json） | 132（rustls） |
| TLS | 既定rustls。native-tlsも選べる | 既定rustls。native-tlsも選べる | 既定rustls（rig-reqwest経由） | 選べる | **既定native-tls**。rustlsは任意 |
| base_url変更 | `with_api_base` | `ServiceTargetResolver`。ただし**Zaiアダプタはチャットで上書き** | `with_base_url` | 自由 | `with_endpoint` |
| 独自ヘッダ | `with_header`、呼出しごとのオプション | `WebConfig::with_default_headers` | トランスポートのmiddleware | 自由 | `with_header` |
| reqwest::Client差込 | `with_http_client` / `build`（再試行層付き） | `with_reqwest` | `ReqwestClient::from(reqwest::Client)` | 自由 | **不可**（呼出しごとに内部で作り直す） |
| ライブラリ独自の自動再試行 | **既定で有効**（429/5xx/接続、最大3回） | なし | なし（再試行の要否を判定する関数だけ） | なし | なし |
| 再試行の無効化 | `middleware` featureで `with_http_service` | 不要 | 不要 | 不要 | 不要 |
| `response_format: json_schema` | 型あり | あり（OpenAI系アダプタ） | あり（OpenAI chat wire） | 自前で書く | 未確認 |
| 埋め込み | あり | あり | あり | 自前で書く | あり |
| 更新されるトークン | `Config` traitの自前実装、またはtowerサービスで毎回設定 | 非同期 `AuthResolver` | 非同期のヘッダ用middleware | 自由 | 不可（固定値） |
| 送信したかどうかの判別 | `OpenAIError::Reqwest(reqwest::Error)` から判定できる | `webc::Error::Reqwest` から判定できる | `Error::Instance` に包まれる（下記） | `reqwest::Error` を直接扱える | `APIError` 文字列中心（未精査） |

依存数・MSRV・版の出典は［crates.io API (2026/11)］と各crateの `Cargo.toml`、保守体制は［GitHub API (2026/11)］です。各行の根拠は次の節で示します。

## 2. 全候補に共通する下層：reqwestとhyper-utilの再送

自前実装を含め、どの候補もHTTPには `reqwest` 0.13 を使っています［async-openai 0.43.0, Cargo.toml］［genai 0.7.4, Cargo.toml］［rig-reqwest 0.46.0, Cargo.toml］。openai-api-rs だけは reqwest 0.12 です［openai-api-rs 10.0.2, Cargo.toml］。このため、下層での再送の仕組みは全候補に共通します。

- 事実：reqwest 0.13 は、既定で「安全に再試行できると分かっているエラーや低レベルのプロトコルNACK」だけを再試行します［reqwest 0.13.5, src/retry.rs:9-14］。既定のポリシーは1回の要求につき最大2回まで再試行します［reqwest 0.13.5, src/retry.rs:195-202］。対象は、HTTP/2で相手がGOAWAY（NO_ERROR）を送った場合とREFUSED_STREAMを返した場合です［reqwest 0.13.5, src/retry.rs:273-317］。`retry::never()` を `ClientBuilder::retry` に渡すとこの再試行を止められます［reqwest 0.13.5, src/retry.rs:84-90］［reqwest 0.13.5, src/async_impl/client.rs:1411］。
- 事実：hyper-util 0.1.21 の legacy client は、プール済みの接続を再利用した要求が書き込みを始める前に失敗した場合、その要求を送り直します。この設定 `retry_canceled_requests` は既定で有効です［hyper-util 0.1.21, src/client/legacy/client.rs:248-268, 1583-1593］。reqwest 0.13.5 のソースにはこの設定を変えるコードがありません（`retry_canceled` で検索して0件）。
- 事実：`reqwest::Error::is_connect()` は、エラーの原因をたどってhyper-utilの接続エラーがあるときにtrueを返します。`is_timeout()` は、原因のどこかにタイムアウトがあればtrueを返します［reqwest 0.13.5, src/error.rs:119-170］。どちらの判定も、要求が送信前か後かを直接は示しません。接続確立中のタイムアウトで両方がtrueになるかは、接続処理のコードを読んでいないため未確認です。
- Pelwayへの当てはめ（筆者の整理）：上の2つの再送は、どちらも相手が処理していないと分かっている要求を送り直すものです。D3がいう「結果不明の自動再送」には当たらないと整理します。それでも残したくない場合は、reqwest側の再試行を `retry::never()` で止められます。hyper-util側の再送はreqwestの公開APIでは止められないとみられます（未確認）。送信状態は「`is_connect()` がtrueなら未送信が確定」「それ以外の送信エラーとタイムアウトは結果不明」と保守的に分類するのが安全です。

## 3. 候補ごとの記録

### async-openai（64bit/async-openai）

- 事実：既定のfeatureは `rustls` です。APIのグループごとにfeatureが分かれており（`chat-completion`、`embedding` など）、`*-types` は型だけを有効にします［async-openai 0.43.0, Cargo.toml, [features]］。
- 事実：`Client::build`、`with_config`、`with_http_client` はどれも、内部で `ReqwestExecutor::new` を呼びます［async-openai 0.43.0, src/client.rs:130-155］。`ReqwestExecutor::new` は送信サービスを必ず `OpenAIRetryLayer::default()` で包みます［async-openai 0.43.0, src/executor.rs:144-150］。既定の再試行回数は3回です［async-openai 0.43.0, src/middleware/retry/openai.rs:52-55］。
- 事実：`retry_request` が再試行するのは次の3つです。429（ただし `insufficient_quota` は除外）［同 openai.rs:167-200］、5xx［同 openai.rs:202-205］、接続エラー（`reqwest::Error::is_connect()`）［同 openai.rs:241-249］。待ち時間は `Retry-After` があればその値、なければ100msから倍々に増やし、上限は8秒です［同 openai.rs:218-224］。
- 事実：この再試行層を外す公開手段は `with_http_service` だけで、`middleware` featureが必要です［async-openai 0.43.0, src/client.rs:157-170］。モジュール文書も「既定のクライアントはこの層を内部で使う」と書いています［async-openai 0.43.0, src/middleware/mod.rs:84-90］。
- 事実：ヘッダは呼出しごとに `self.config.headers()` から組み立てます［async-openai 0.43.0, src/client.rs:340-354］。`Config` traitは同期の `headers()` と `api_key() -> &SecretString` を持ち、自前で実装できます［async-openai 0.43.0, src/config.rs:18-28］。`OpenAIConfig` はAPIキーを `secrecy::SecretString` で持ちます［async-openai 0.43.0, src/config.rs:113-122］。
- 事実：ログの出力箇所は次のとおりです。5xx応答の本文は `tracing::warn!` で出力されます［async-openai 0.43.0, src/client.rs:742-745］。JSONの解釈に失敗したときは応答本文全体が `tracing::error!` で出力されます［async-openai 0.43.0, src/error.rs:146-148］。レート制限のヘッダも出力されます［async-openai 0.43.0, src/middleware/retry/mod.rs:33-45］。リクエスト本文やキーを出力するコードは見つかりませんでした。
- 事実：`OpenAIError` には `Reqwest(reqwest::Error)`、`ApiError`、`JSONDeserialize`、`InvalidArgument` などがあります［async-openai 0.43.0, src/error.rs:11-41］。
- 事実：型専用feature（`chat-completion-types`、`embedding-types`、`default-features = false`）だけなら依存数は24で、依存ツリーに reqwest と tokio は現れません。reqwest 0.13（rustls+json）と組み合わせると98です（どちらも筆者が `cargo tree` で計測）。
- 未確認：Z.ai や Vertex の応答に、async-openai の型が必須とするフィールドが欠けていないか。欠けていると解釈に失敗します。`byot` featureは任意の型で送受信するためのものです［async-openai 0.43.0, src/middleware/mod.rs:115-120］。
- Pelwayへの当てはめ（筆者の整理）：使うなら `middleware` featureと `with_http_service` が必須です。差し込む自前のtowerサービスで、再試行なし・timeout・毎回のbearer付与をまとめて実装することになります。5xxは相手が処理したかどうか分からないので、既定の再試行はD3と衝突すると判断します。通信部分を借りないなら、型専用featureの24依存だけを使う案4'が軽くて済みます。

### genai（jeremychone/rust-genai）

- 事実：既定のfeatureは `rustls-tls` と `system-proxy` です。`otel` featureは既定で無効です［genai 0.7.4, Cargo.toml:41-46］［genai 0.7.4, src/lib.rs:24-26］。
- 事実：`ClientBuilder::with_reqwest` で自前の `reqwest::Client` を渡せます［genai 0.7.4, src/client/builder.rs:26-29］。`WebConfig` でtimeout・proxy・既定ヘッダも指定できます［genai 0.7.4, src/client/web_config.rs:12-80］。
- 事実：ソース全体を `retry` と `backoff` で検索すると、該当するのはエラー型の説明だけです。説明には「再試行の方針は呼出し側に任せる」とあります［genai 0.7.4, src/error.rs:193-207］。自動再試行の実装は見つかりませんでした。
- 事実：`AuthResolver` には非同期版（`ResolverAsyncFn`）があります［genai 0.7.4, src/resolver/auth_resolver.rs:17-41］。`exec_chat` は呼出しのたびに `resolve_model_spec` を実行し、その結果の認証値を使います［genai 0.7.4, src/client/client_impl.rs:100-102］。設定側では `resolve_model_spec` の中で認証の `resolve` が呼ばれます［genai 0.7.4, src/client/config.rs:234］。
- 事実：**Z.ai アダプタの `to_web_request_data` は `target.endpoint` を `ZaiModelEndpoint::from_model` の結果で必ず上書きします。** 結果は `https://api.z.ai/api/coding/paas/v4/` か既定の `https://api.z.ai/api/paas/v4/` のどちらかです［genai 0.7.4, src/adapter/adapters/zai/adapter_impl.rs:17-30, 83-94］。チャットのリクエスト組み立てはOpenAIアダプタの処理を流用しています。
- 事実：モデル名が `glm` で始まると Zai アダプタを選び、どの接頭辞にも当たらないモデル名は Ollama に振り分けます［genai 0.7.4, src/adapter/adapter_kind.rs:292-305］。
- 事実：Vertex アダプタは Gemini ネイティブの `:generateContent` と `:predict` を呼びます。OpenAI互換エンドポイントは使いません。リージョンは環境変数 `VERTEX_LOCATION` から決め、未設定なら global です［genai 0.7.4, src/adapter/adapters/vertex/adapter_impl.rs:57-71, 96-118］。認証値はOAuthのbearerトークンとして送ります［同 adapter_impl.rs:136-138］。
- 事実：応答本文は `tracing::trace!` で出力されます［genai 0.7.4, src/webc/web_client.rs:99, 116］。
- 事実：エラーは `Error::WebAdapterCall` / `WebModelCall` の中に `webc::Error::Reqwest(reqwest::Error)` を持ちます［genai 0.7.4, src/error.rs:74-85］［genai 0.7.4, src/webc/error.rs:10-33］。
- Pelwayへの当てはめ（筆者の整理）：再試行がない点はD3に合います。一方で、Z.ai をgateway経由のURLで呼ぶにはOpenAIアダプタに切り替えてURLを指定し直す必要があります。モデル名による自動振り分け（不明な名前はOllama行き）も、Pelway側で必ず固定しなければなりません。提供元を設定で選ぶPelwayのtraitと、genai自身の振り分けが二重になります。

### rig-core（0xPlaygrounds/rig）

- 事実：MSRVは1.97.1、editionは2024です［rig-core 0.46.0, Cargo.toml:13-14］。Pelwayの固定版 1.96.0 より新しい版を要求します。
- 事実：HTTPは `rig-http` のトランスポート抽象を通ります。`rig-reqwest` の `ReqwestClient` は `From<reqwest::Client>` で自前のクライアントを包めます［rig-reqwest 0.46.0, src/lib.rs:30-48］。rustls・native-tlsどちらのfeatureでも `reqwest/system-proxy` を有効にします［rig-reqwest 0.46.0, Cargo.toml］。
- 事実：`HttpMiddleware::before_request_headers` は非同期の関数で、送信前にヘッダを書き換えられます。ここでエラーを返すと要求は送られません［rig-http 0.46.0, src/http_client/middleware.rs:33-58］。
- 事実：rig-core にあるのは、再試行してよいかを判定する関数（`retryable_status`：408/425/429/5xx、`transient_transport`）と、`retry_after` の解析だけです［rig-core 0.46.0, src/error.rs:225-251］［rig-core 0.46.0, src/error/retry.rs:1-24］。再試行の実行は別のcrateが担うと書かれています［rig-core 0.46.0, src/error/retry.rs:1-3］。rig-core内に再試行のループは見つかりませんでした。
- 事実：`transient_transport` は、応答を伴わない `Instance` エラーを一律に「再試行可」と判定します。説明には「サーバが要求を処理していないことは保証しない」とあります［rig-core 0.46.0, src/error.rs:239-252］。
- 事実：Z.ai 用の設定（`ZAI` dialect）はOpenAI既定の動作を引き継ぎます［rig-core 0.46.0, src/providers/openai/wire/dialects.rs:396-416］。構造化出力は `{"type":"json_schema", ..., "strict": true}` として送ります［rig-core 0.46.0, src/providers/openai/wire/chat.rs:317-326］。
- 事実：テレメトリはtracingのspanで記録します。会話内容の記録は明示的に有効にした場合だけです［rig-core 0.46.0, src/telemetry/mod.rs:1］。秘密情報を伏せる `redact` モジュールもあります［rig-core 0.46.0, src/redact.rs:1-15］。
- 事実：トランスポートのエラーは `Error::Instance(Box<dyn Error>)` に包まれます［rig-http 0.46.0, src/http_client/mod.rs:56-59］［rig-reqwest 0.46.0, src/lib.rs:188-194］。`ProviderError::source` は中のエラーを返します［rig-core 0.46.0, src/error.rs:413-424］。
- 未確認：`ProviderError` から `reqwest::Error` まで実際に取り出せるか（コードの構造上は取り出せそうですが、試していません）。
- Pelwayへの当てはめ（筆者の整理）：rigはエージェント・ツール・観測・ストリーミングまで含む大きな部品です。Pelwayが使うのはチャット補完と埋め込みだけで、「小範囲」の用途に対して大きすぎます。MSRVを満たすにはツールチェーンを上げる必要もあります。

### openai-api-rs（dongri/openai-api-rs）

- 事実：既定のTLSは `default-tls`（native-tls）です［openai-api-rs 10.0.2, Cargo.toml］。`build_request` は呼出しのたびに `reqwest::Client::builder()` から新しいクライアントを作り、`build().unwrap()` と proxy設定の `unwrap()` で失敗時にpanicします［openai-api-rs 10.0.2, src/v1/api.rs:149-170］。
- 事実：`OpenAIClient` は `#[derive(Debug)]` で、`api_key: Option<String>` をそのまま持ちます［openai-api-rs 10.0.2, src/v1/api.rs:78-86］。Debug出力にキーが含まれます。
- 判定：見送り。自前の `reqwest::Client` を差し込めず、Debug出力からキーが漏れ得るためです。

### reqwest＋serdeの自前実装（案4、案4'）

- 事実：reqwest 0.13.5（rustls+json）の依存数は86で、候補の中で最も少ないです（筆者が `cargo tree` で計測）。下層の再送は第2節のとおりです。
- Pelwayへの当てはめ（筆者の整理）：必要なのは `POST {base}/chat/completions` と `POST {base}/embeddings` の2本だけです。Z.ai・OpenAI・Vertex のOpenAI互換エンドポイントのどれも、同じ形の要求で呼べます（次節）。書く量は、型を含めても数百行程度と見積もります。型には `async-openai` の型専用featureを借りる案（案4'）と、使うフィールドだけ自前で定義する案があります。自前で書けば、エラーを「未送信確定」「結果不明」「API拒否」に分ける型をPelwayの状態遷移（03の送信中→結果不明）に直接合わせられます。

## 4. 提供元ごとの公式仕様

### Z.ai

- 事実：汎用のAPIエンドポイントは `https://api.z.ai/api/paas/v4` で、認証はHTTP Bearerです［Z.ai API Introduction (2026/11)］。OpenAI Python SDKで `base_url="https://api.z.ai/api/paas/v4/"` を指定する例があります［Z.ai API Introduction (2026/11)］。
- 事実：Chat Completion のAPIリファレンスでは、`response_format.type` のenumが `text` と `json_object` だけです［Z.ai Chat Completion (2026/11), response_format］。構造化出力のガイドも `{"type": "json_object"}` だけを説明しています。JSONスキーマはシステムプロンプトに書き、検証はクライアント側で行う例になっています［Z.ai Structured Output (2026/11)］。
- 未確認：Z.ai に `json_schema` を送った場合の挙動（無視されるか、エラーになるか）。genai と rig の Z.ai 向けの経路はどちらも `json_schema` を送るつくりです（第3節）。
- 未確認：埋め込み。docs.z.ai の索引（llms.txt）に埋め込みのページはありません［Z.ai llms.txt (2026/11)］。中国向けの docs.bigmodel.cn には「文本嵌入」（embedding-3）のページがあります［BigModel llms.txt (2026/11)］。`api.z.ai` に埋め込みエンドポイントがあるかは確認できませんでした。genai の Zai アダプタは埋め込みのURLを `{base}embeddings` としています［genai 0.7.4, src/adapter/adapters/zai/adapter_impl.rs:71-80］が、APIが存在する根拠にはなりません。

### OpenAI

- 事実：Chat Completions で `response_format` に `json_schema` を指定する構造化出力は、`gpt-4o-mini`、`gpt-4o-2024-08-06` 以降のスナップショットで使えます。JSONモードは有効なJSONを返しますが、スキーマには従わせません［OpenAI Structured Outputs (2026/11)］。
- 事実：埋め込みのエンドポイントは `https://api.openai.com/v1/embeddings` です。`text-embedding-3-small` は1536次元、`text-embedding-3-large` は3072次元です［OpenAI Embeddings (2026/11)］。

### Vertex AI（Gemini）

- 事実：OpenAI互換エンドポイントのベースURLとして、2つの形が示されています。`https://aiplatform.googleapis.com/v1/projects/{project_id}/locations/{location}/endpoints/openapi`（例ではlocationをglobalに設定）と、リージョン付きホストの `https://${LOCATION}-aiplatform.googleapis.com/v1beta1/projects/${PROJECT_ID}/locations/${LOCATION}/endpoints/openapi` です［Vertex OpenAI auth (2026/11)］［Vertex OpenAI start (2026/11)］。
- 事実：認証は短命のOAuthアクセストークンです。OpenAIライブラリの `api_key` にトークンを渡します。サービスアカウントのトークンは既定で1時間有効で、定期的に更新して差し替えるよう案内しています［Vertex OpenAI auth (2026/11)］。
- 事実：OpenAI互換の対応パラメータとして `response_format` の `json_schema` が挙がっています。完全に再帰的なスキーマは非対応です［Vertex OpenAI overview (2026/11)］。
- 未確認：OpenAI互換エンドポイントでの埋め込み。OpenAI互換の各ページは埋め込みに触れていません［Vertex OpenAI start (2026/11)］［Vertex OpenAI overview (2026/11)］。埋め込みはネイティブの埋め込みAPI（`gemini-embedding-001` など）で呼ぶ前提にするのが安全です。
- 事実：ロケーションのページは、地域ごとの表の列に Tokyo（asia-northeast1）を載せています［Vertex locations (2026/11)］。ただし、モデルごとの提供可否はアイコンで表示されており、本調査では読み取れませんでした（未確認）。
- 事実：同じページに次の注意書きがあります。

```text
Important: Endpoints don't guarantee data residency or in-region ML processing.
（重要：エンドポイントは、データの所在や地域内でのML処理を保証しません。）
```

［Vertex locations (2026/11), Agent Platform endpoints］

```text
Don't use the global endpoint if you have ML processing requirements, because you can't control or know which region your ML processing requests are sent to when a request is made.
（ML処理の要件がある場合はglobalエンドポイントを使わないでください。要求したときにML処理がどのリージョンへ送られるかを制御も把握もできないためです。）
```

［Vertex locations (2026/11), Global endpoint］

- 未確認：国内でのML処理を保証する条件。上の注意書きは、データ所在（Data residency）について別のページを参照するよう案内しています。そのページは本調査で読んでいません。

### 各候補で更新されるトークンを扱えるか

- async-openai：`Config::headers()` は同期関数なので、トークンの更新は外で行い、共有された値を読む形になります（筆者の整理）。`with_http_service` のtowerサービスなら、要求ごとに非同期でbearerを付けられます［async-openai 0.43.0, src/client.rs:340-354］［async-openai 0.43.0, src/middleware/mod.rs:57-80］。
- genai：非同期の `AuthResolver` で扱えます［genai 0.7.4, src/resolver/auth_resolver.rs:17-41］。ただし Vertex アダプタはネイティブAPI用です。OpenAI互換で呼ぶ場合は、OpenAIアダプタとURLの指定し直しが必要です。
- rig：非同期の `before_request_headers` で扱えます［rig-http 0.46.0, src/http_client/middleware.rs:46-58］。
- 自前実装：毎回の送信直前に、トークンを管理する部品から取得します。秘密を扱う部品（gatewayの資格情報broker）と直接つなげられます。
- openai-api-rs：固定値だけです。

## 5. Pelway への当てはめ（筆者の整理）

この節は全て筆者の判断です。

1. **第一候補（MVP前に検証）：案4（reqwest＋serdeの自前実装）、または案4'（型だけasync-openai）。** 依存が最も少なく（案4は86、案4'は98）、ライブラリ独自の再試行はありません。下層の再送は第2節のとおり、未送信が確定した要求に限られます。MSRVの問題もありません。エラーを、03の状態（送信中・結果不明）にそのまま対応する型で返せます。
   - `reqwest::ClientBuilder` に、`.retry(reqwest::retry::never())`、gatewayの限定proxy（`.proxy(...)` で明示的に指定。proxyを追加するとシステムのproxyの自動利用は無効になります［reqwest 0.13.5, src/async_impl/client.rs:1418-1423］。`.no_proxy()` は指定したproxyまで全て消すので併用しません［同 client.rs:1429-1436］）、connect/read/全体のtimeout、`https_only(true)`、rustls を設定します。
   - Pelwayのtraitは `chat(req, schema) -> Result<Chat, LlmError>` と `embed(texts) -> Result<Vec<Vec<f32>>, LlmError>` 程度にします。`LlmError` は少なくとも `NotSent`（`is_connect` か送信前の検査で失敗）、`OutcomeUnknown`（送信後のtimeoutや切断）、`Rejected{status, body}`（4xx/5xx）、`Invalid`（解釈失敗やスキーマ違反）に分けます。5xxも「相手が処理したか不明」なので自動再送しません。
   - 構造化出力：提供元ごとの能力値（Z.ai は `json_object`、OpenAI と Vertex は `json_schema`）を設定に持たせます。結果は常にクライアント側でJSON Schema検証します。Z.ai の公式仕様が `json_object` だけなので、この検証層はどの候補を選んでも必要です。
   - 秘密：トークンは `secrecy` などの型に入れ、`Debug` に出ないようにします。応答本文をログに出す場合も、本文ではなく長さやハッシュにとどめます。
2. **次点（将来の候補）：async-openai を `middleware` featureと `with_http_service` 付きで使う案。** 型の網羅性と保守頻度は高いです。ただし、既定経路が再試行付きなので、設定を誤ると5xxで自動再送が起きます。CIで「`with_http_client` や `build` を使っていない」ことを検査する仕掛けが要ります。応答本文をwarn/errorで出力する箇所があり、ログの設定で抑える必要もあります。
3. **見送り：genai、rig-core、openai-api-rs。**
   - genai：Z.ai アダプタがURLを固定し、モデル名でOllamaへ振り分けるなど、提供元の自動選択がPelwayの方針（設定で明示的に選ぶ、fallbackなし）と重なり、管理する箇所が二重になります。
   - rig-core：MSRV 1.97.1 で現行ツールチェーンに合いません。規模も「小範囲」の用途に対して大きすぎます。将来エージェント機能が要る場合に改めて比べます。
   - openai-api-rs：第3節の理由で見送ります。
4. **一般利用者段階のVertexについて。** 公式文書が「エンドポイントは地域内でのML処理を保証しない」と明記しています。asia-northeast1 を指定するだけでは、10の国内限定方針を満たす根拠になりません。Data residency のページと、契約上の地域内処理の条件を、選定の前に確認する必要があります。技術面では、OAuthトークンを要求ごとに付け替える仕組みがあれば、案4のクライアントをそのまま使えます。埋め込みはOpenAI互換では確認できなかったので、ネイティブAPIの呼出しを1本追加する前提で見積もります。

### 未確認事項の一覧

- Z.ai に `json_schema` を送ったときの挙動と、`api.z.ai` での埋め込みAPIの有無。
- Vertex の OpenAI互換エンドポイントでの埋め込み対応、asia-northeast1 でのモデルごとの提供可否、地域内でのML処理を保証する条件。
- hyper-util の `retry_canceled_requests` を reqwest の公開APIから無効にできるか。
- rig の `ProviderError` から `reqwest::Error` を取り出せるか。
- async-openai の型で Z.ai と Vertex の応答を解釈できるか。
- openai-api-rs の構造化出力の型とエラー型の詳細（見送りのため精査していません）。

## 出典

[Pelway 02, 2026/11] Pelway. 「02 技術比較と依存採用案」. docs/technical/v0.7/body/02-stack-comparison.md
[Pelway 03, 2026/11] Pelway. 「03 Bot実行・権限・承認・隔離」. docs/technical/v0.7/body/03-ai-bot-runtime.md
[Pelway 08, 2026/11] Pelway. 「08 採用記録・未決の正本・受入試験」. docs/technical/v0.7/body/08-decisions-open.md
[Pelway 10, 2026/11] Pelway. 「10 Pelway本体の国内限定と外部連携の責任分界」. docs/technical/v0.7/body/10-domestic-only.md
[Pelway mise.toml, 2026/11] Pelway. "mise.toml." mise.toml
[Pelway Cargo.lock, 2026/11] Pelway. "Cargo.lock." Cargo.lock
[crates.io API, 2026/11] crates.io. "async-openai / genai / rig-core / openai-api-rs." https://crates.io/api/v1/crates/
[GitHub API, 2026/11] GitHub. "Repository and contributors API." https://api.github.com/repos/
[async-openai 0.43.0, 2026/11] 64bit. "async-openai 0.43.0." https://crates.io/crates/async-openai/0.43.0 ／ https://github.com/64bit/async-openai
[genai 0.7.4, 2026/11] jeremychone. "genai 0.7.4." https://crates.io/crates/genai/0.7.4 ／ https://github.com/jeremychone/rust-genai
[rig-core 0.46.0, 2026/11] 0xPlaygrounds. "rig-core 0.46.0." https://crates.io/crates/rig-core/0.46.0 ／ https://github.com/0xPlaygrounds/rig
[rig-http 0.46.0, 2026/11] 0xPlaygrounds. "rig-http 0.46.0." https://crates.io/crates/rig-http/0.46.0
[rig-reqwest 0.46.0, 2026/11] 0xPlaygrounds. "rig-reqwest 0.46.0." https://crates.io/crates/rig-reqwest/0.46.0
[openai-api-rs 10.0.2, 2026/11] dongri. "openai-api-rs 10.0.2." https://crates.io/crates/openai-api-rs/10.0.2 ／ https://github.com/dongri/openai-api-rs
[reqwest 0.13.5, 2026/11] seanmonstar. "reqwest 0.13.5." https://crates.io/crates/reqwest/0.13.5
[hyper-util 0.1.21, 2026/11] hyperium. "hyper-util 0.1.21." https://crates.io/crates/hyper-util/0.1.21
[Z.ai API Introduction, 2026/11] Z.ai. "Introduction." https://docs.z.ai/api-reference/introduction
[Z.ai Chat Completion, 2026/11] Z.ai. "Chat Completion." https://docs.z.ai/api-reference/llm/chat-completion
[Z.ai Structured Output, 2026/11] Z.ai. "Structured Output." https://docs.z.ai/guides/capabilities/struct-output
[Z.ai llms.txt, 2026/11] Z.ai. "Documentation index." https://docs.z.ai/llms.txt
[BigModel llms.txt, 2026/11] Zhipu AI. "Documentation index." https://docs.bigmodel.cn/llms.txt
[OpenAI Structured Outputs, 2026/11] OpenAI. "Structured Outputs." https://developers.openai.com/api/docs/guides/structured-outputs
[OpenAI Embeddings, 2026/11] OpenAI. "Embeddings." https://developers.openai.com/api/docs/guides/embeddings
[Vertex OpenAI start, 2026/11] Google Cloud. "Call Gemini by using the OpenAI library." https://docs.cloud.google.com/vertex-ai/generative-ai/docs/start/openai
[Vertex OpenAI auth, 2026/11] Google Cloud. "Authenticate (OpenAI compatibility)." https://docs.cloud.google.com/vertex-ai/generative-ai/docs/migrate/openai/auth-and-credentials
[Vertex OpenAI overview, 2026/11] Google Cloud. "OpenAI compatibility overview." https://docs.cloud.google.com/vertex-ai/generative-ai/docs/migrate/openai/overview
[Vertex locations, 2026/11] Google Cloud. "Deployments and endpoints / locations." https://docs.cloud.google.com/vertex-ai/generative-ai/docs/learn/locations
