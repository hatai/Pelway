# Issue #225 の作業ログ

## 乱数 API の対応表（旧 API → 新 API）

rand を使う箇所は `crates/pelway-auth` のテスト内 RSA 鍵生成 1 箇所だけ。

| 箇所 | 旧 API | 新 API | 乱数源 |
|---|---|---|---|
| `crates/pelway-auth/tests/auth_flow.rs` の `FakeIdp::new` | `rand::thread_rng()`（rand 0.8、OS 乱数でシードされる ThreadRng = CSPRNG） | `rsa::rand_core::OsRng` | OS の乱数源（getrandom 経由） |

`rand::thread_rng()` を `rand::rng()`（rand 0.9 以降の新名）に書き換えるだけでは足りない。rand 0.10 の `ThreadRng` は rand_core 0.10 系のトレイト実装で、`rsa` 0.9 の `RsaPrivateKey::new` が要求する `CryptoRngCore`（rand_core 0.6 系）と互換がない。rsa 0.9.10 は `pub use rand_core` しており、`rsa` の `getrandom` feature を有効にすると `rsa::rand_core::OsRng` が使えるため、鍵生成の乱数源は OsRng（OS 乱数）に置き換えた。rsa のバージョン自体は「rand 以外の依存の更新」を対象外とする Issue の範囲に合わせて 0.9 のまま。

`rand = "0.10"` は dev-dependency として残す。直接使用箇所は無くなったが、受け入れ条件「ワークスペースの rand が 0.10 系」の確認先（`Cargo.lock` の pelway-auth の依存エントリが `rand 0.10.3` を指すこと）を、このクレートの依存として明示するため。

src/ 側（`oidc.rs` の `CsrfToken::new_random`・`Nonce::new_random`・`PkceCodeChallenge::new_random_sha256`）は openidconnect クレート内部の乱数生成で、pelway-auth の rand 依存とは無関係。openidconnect 4.1.0 は内部で rand 0.8.8 を使い続けるため、`Cargo.lock` には rand 0.8.8 と 0.10.3 が併存する（openidconnect の更新は対象外）。

## 実装者メモ（round 1）

1. 詰まった箇所: 手順 4。`rand::thread_rng()` → `rand::rng()` の単純な改名ではコンパイルが通らず、rsa 0.9.10 の `CryptoRngCore` が rand_core 0.6 系で rand 0.10（rand_core 0.10 系）と非互換だと判明するまで一手間かかった。
2. 曖昧だった文: 「乱数を使う箇所が、暗号論的に安全な乱数源（OS の乱数か、それを種にした CSPRNG）を使い続けている」（Issue #225「受け入れ条件」）。
3. 仮定と解釈: 「rand 0.10 の API を使い続ける」ことを要求していないと解釈し、rsa 0.9 が受け付ける OS 乱数源 `rsa::rand_core::OsRng` を採った。`rand::rng()` を使う案は rsa を 0.10.0-rc.19（crypto-glue 経由で lock に既存）へ上げる必要があり、「rand 以外の依存の更新」に触れるため捨てた。
4. 足りなかった情報・検証手段: rand 0.10 と rsa 0.9 の rand_core 世代差は Cargo.toml だけでは読めず、`~/.cargo/registry/src` の rsa 0.9.10 ソース（`pub use rand_core`）と `Cargo.lock` の依存エントリを突き合わせて確認した。
5. 改善案: `docs/agents/` か `AGENTS.md` の依存更新の注意書きに「rand 系の bump は rand_core の世代差で暗号系クレート（rsa・signature 系）と衝突しやすい。lockfile で依存先の rand_core 版を確認してから書き換える」を足す。
