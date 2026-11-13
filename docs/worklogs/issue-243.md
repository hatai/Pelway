# Issue #243 の作業ログ

## hkdf 0.12.4 → 0.13.0 の変更点（使っている箇所に関わるもの）

出典: hkdf の CHANGELOG（0.13.0 項）と、`hkdf-v0.13.0`・`hkdf-v0.12.4` タグの
src/lib.rs・src/hmac_impl.rs。本文鍵の一致は `payload_key_matches_hkdf_0_12_fixed_value`、
コンテナ形式の一致は `container_format_is_compatible_with_aes_gcm_0_10` で確認する。

| 変更点（0.12.4 → 0.13.0） | 使っている箇所への影響 |
|---|---|
| `Hkdf<H, I = Hmac<H>>`（struct）が `GenericHkdf<H: HmacImpl>` に改名され、`Hkdf<H>` は型エイリアス `GenericHkdf<Hmac<H>>` に | `use hkdf::Hkdf` と `Hkdf::<Sha256>` の書き方はそのまま解決できる。use 行・型名の変更不要 |
| `new(salt: Option<&[u8]>, ikm: &[u8])`・`expand(&self, info: &[u8], okm: &mut [u8]) -> Result<(), InvalidLength>` | シグネチャは同じ。`payload_key`（src/lib.rs）の呼び出しは変更不要 |
| `HmacImpl` の実装条件が `H: EagerHash`（digest 0.11 の特性）になり、sealed が外れた | sha2 0.10 の `Sha256` は digest 0.10 の特性しか実装しないため `Hmac<Sha256>: HmacImpl` を満たさず、E0277 でコンパイルできない（PR #238 が失敗した原因）。pelway-at-rest の `sha2` を 0.11 に上げて対処 |
| HKDF-SHA256 の演算（RFC 5869） | 変更なし。同じ ikm・salt・info から同じバイト列が出ることを固定値試験で確認 |
| `std` 機能の削除・`kdf` 機能の追加 | 当クレートは hkdf の機能を何も有効にしていないため影響なし |
| MSRV 1.85・edition 2024 | ワークスペースの Rust 1.99.1（mise.toml）が満たすため変更不要 |
| `HkdfExtract` の分割型・`SimpleHkdf` 別実装 | 使っていない（`Hkdf::new` の一括 extract+expand のみ）。影響なし |
| Cargo.lock の統一 | hkdf 0.12.4 と hmac 0.12.1 がロックから消え、ワークスペースは hkdf 0.13.0（hmac 0.13.0）だけになる |

### sha2 を 0.11 に上げた理由（Issue の「依存の都合」にあたる）

hkdf 0.13 の `HmacImpl` は `Hmac<H>` に `H: EagerHash`（digest 0.11）を要求する。
`sha2` 0.10 の `Sha256` は digest 0.10 系の特性しか持たないため、hkdf 0.13 と
sha2 0.10 の組み合わせはコンパイルできない（上表 3 行目）。sha2 の更新自体は
PR #173 の範囲だが、このクレートでは切り離せないため pelway-at-rest に限って
0.11 に上げた。pelway-audit-checkpoint・pelway-contract の `sha2 = "0.10"` は
そのまま残し、Cargo.lock には sha2 0.10.9 と 0.11.1 が並存する（残りは PR #173 で扱う）。

### 固定値の作り方

`PAYLOAD_KEY_0_12` は更新前の版（hkdf 0.12.4 + sha2 0.10.9）で
`cargo test -p pelway-at-rest payload_key_matches_hkdf_0_12_fixed_value` を実行し、
失敗した assert の `left`（実際の導出値 32 バイト）を定数に写したもの。
ikm = `[0x33; 32]`、salt = `[0x44; 32]`、info = `b"pelway-at-rest/v1 payload"`。

## 実装者メモ（round 1）

1. 詰まった箇所: crates/pelway-at-rest/Cargo.toml: hkdf を 0.13 に上げただけでは `Hkdf::<Sha256>::new` が E0277（`Hmac<Sha256>: HmacImpl` 不成立）でコンパイルできない。hkdf 0.13 の `HmacImpl` が digest 0.11 の `EagerHash` を要し、sha2 0.10 の `Sha256` では満たせないため、sha2 を 0.11 に上げて解消した。
2. 曖昧だった文: 「sha2 の更新（PR #173 で扱う。依存の都合で同時に上げる必要がある場合は、理由を worklog に書いて同じ PR で上げてよい）」（Issue #243 の対象外）— 「同じ PR で上げる」範囲が、sha2 が必要なクレートだけか、ワークスペース全体かが決まっていない。
3. 仮定と解釈: 仮定: 切り離せない範囲は hkdf を使う pelway-at-rest だけ。解釈: sha2 の 0.11 化は pelway-at-rest に限定し、pelway-audit-checkpoint・pelway-contract の `sha2 = "0.10"` は残した（ロックに 0.10.9 と 0.11.1 が並存）。選ばなかった: 全クレート一括の 0.11 化（PR #173 の範囲を先取りするため）。
4. 足りなかった情報・検証手段: `Hkdf` が struct から型エイリアスに変わった影響と、`Hmac<Sha256>` が使える条件。CHANGELOG には特性名が書かれていないため、GitHub の `hkdf-v0.13.0` タグの src/hmac_impl.rs を読んで `H: EagerHash` が条件だと確かめた。
5. 改善案: `.github/ISSUE_TEMPLATE/agent-task.yml` の「対象外」の説明に「依存の都合で同時更新が必要なとき、その範囲（同一クレートに限るかワークスペース全体か）をここに書く」という一文を足す。
