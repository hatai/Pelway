# Issue #226 の作業ログ

## ed25519-dalek 2.2.0 → 3.0.0 の変更点（使っている箇所に関わるもの）

出典: ed25519-dalek の CHANGELOG（3.0.0 項）と、レジストリキャッシュの
ed25519-dalek-3.0.0・rand_core-0.10.1・getrandom-0.4.3 の Cargo.toml と src。
本文の確認は `tests/compat.rs` の固定値試験（2.2.0 で生成した署名付き
チェックポイントの検証・バイト列までの再現・改ざん拒否）。

| 変更点（2.2.0 → 3.0.0） | 使っている箇所への影響 |
|---|---|
| 依存の `rand_core` が 0.6 系 → 0.10 系 | `SigningKey::generate(&mut rand_core::OsRng)`（src/note.rs）がコンパイルできなくなる（E0433）。rand_core 0.10 には `OsRng` が無い |
| OS 乱数源の調達 | `getrandom` 0.4 の `SysRng` を `getrandom::rand_core::UnwrapErr` で包んで `SigningKey::generate` に渡す形に置換（ed25519-dalek 3 の doc 例と同じ）。`UnwrapErr` は失敗時に panics で、2.x の `rand_core::OsRng` と同じ挙動。クレートの依存は `rand_core 0.6`（getrandom 機能）から `getrandom 0.4`（sys_rng 機能）に変更 |
| `signature` 2 → 3・`ed25519` 2 → 3（依存の更新） | `ed25519_dalek::{Signature, Signer, Verifier}` の再エクスポートは 3.x にも同じ形であるため、use 行の変更は不要 |
| `Signature::from_bytes` の引数 | 3.x も参照 `&[u8; 64]`（`&SignatureBytes`）のまま。呼び出し側の変更不要 |
| 署名・検証の形式（RFC 8032 Ed25519） | 変更なし。2.x で作った鍵・署名がそのまま検証でき、署名は決定的なので同一鍵・同一本文でバイト列まで再現される（compat.rs で確認） |
| `zeroize` 機能が既定に含まれる点 | 3.x も同じ。`SigningKey` の破棄時 0 埋め（src/note.rs の doc コメント）は変わらず有効 |
| MSRV 1.60 → 1.85・edition 2024 | ワークスペースの Rust 1.99.1（mise.toml）が満たすため、変更不要 |
| `std` 機能の削除（`error::Error` が core に） | 当クレートは `std` 機能を明示的に使っていないため影響なし |
| 内部の `sha2` が 0.11 系に | 当クレート自身の `sha2 = "0.10"`（行ハッシュ・鍵 ID）とは独立。両者が Cargo.lock に並存するだけで、コードへの影響なし |
| Cargo.lock の統一 | ed25519-dalek 2.2.0・curve25519-dalek 4.1.3・ed25519 2.2.3・fiat-crypto 0.2.9 がロックから消え、3 系（curve25519-dalek 5.0.0・ed25519 3.0.0・fiat-crypto 0.3.0）に統一 |

`tests/compat.rs` の固定値 `V2_SIGNED` は、更新前のコミット（a37bad5・
ed25519-dalek 2.2.0）で `NoteSigner::from_bytes(ORIGIN, [0x42; 32])` が
`tamper.rs` と同じ合成イベント 5 行に署したもの。再生成は旧コミットで
`checkpoint::sign_checkpoint(&rows_of(5), ORIGIN, &signer)` を実行する。

## 実装者メモ（round 1）

1. 詰まった箇所: crates/pelway-audit-checkpoint/src/note.rs: ed25519-dalek 3 への更新で `rand_core::OsRng`（rand_core 0.6）が使えず E0433。rand_core 0.10 に `OsRng` はもう無い（0.9 で外れた）。ed25519-dalek 3 の doc 例どおり `getrandom::{SysRng, rand_core::UnwrapErr}` に置き換えた。
2. 曖昧だった文: 「2 系で作った署名と鍵（試験の固定値）が、3 系でも同じように検証できる → `cargo test -p pelway-audit-checkpoint` に固定値の互換の試験がある」（Issue #226 の受け入れ条件）— 「固定値」が署名の 64 バイトだけか、検証鍵・チェックポイント本文（origin・行数・最新ハッシュ）も含むかが決まっていない。
3. 仮定と解釈: 仮定: 互換性が要る単位は保管先（CheckpointLog）に残る署名付きチェックポイント全体。解釈: 2.2.0 で生成した署名付きチェックポイント全体を文字列定数 `V2_SIGNED` に埋め、検証が通ること・同じ鍵で再署名するとバイト列まで一致すること・改ざんを拒否することの 3 試験にした。選ばなかった: 署名バイトだけの定数（運用の保存単位が note 全体であるため）と鍵生成（`generate`）の互換試験（乱数由来で固定値にできないため）。
4. 足りなかった情報・検証手段: rand_core 0.10 に `OsRng` が有るかと getrandom 0.4 の機能名（`sys_rng`）— crates のドキュメントではなく、レジストリキャッシュの rand_core-0.10.1・getrandom-0.4.3・ed25519-dalek-3.0.0 の Cargo.toml と src を読んで確かめた。
5. 改善案: docs/worklogs/issue-226.md の「変更点」の節に固定値の生成手順と生成元コミット（a37bad5）を書く（tests/compat.rs のコメントだけでは 2.x 環境での再生成手順が分からないため）。この worklog に反映済み。
