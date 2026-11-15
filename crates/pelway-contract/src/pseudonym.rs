//! ID の仮名化（ログ用）。
//!
//! R04・R09・R10・R11（`docs/technical/v0.7/body/13`）と S17
//! （`12-communication-secrecy` §9）: log・監査に本文・秘密値を出さない。
//! ログに出る ID も生の値ではなく、プロセスごとの salt 付き hash にする。
//!
//! 形式: `"pid:" + 小文字 16 進 64 桁`（SHA-256）。入力は [`crate::snapshot`] と
//! 同じく 8 バイトビッグエンディアン長さ前置きで連結し、salt と ID の境界に
//! 曖昧さを出さない。
//!
//! salt は呼び出し側（HTTP 層）が作って引数で渡す。環境変数から読まない。
//! salt 自体をログに出さないこと（この戻り値には含まれない）。

use sha2::{Digest, Sha256};

const DOMAIN: &[u8] = b"pelway-contract/pseudonym/v1";

/// ID をプロセスごとの salt で仮名化する。ログに出せるのはこの戻り値だけ。
#[must_use]
pub fn pseudonymize(salt: &[u8], id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN);
    hasher.update((salt.len() as u64).to_be_bytes());
    hasher.update(salt);
    hasher.update((id.len() as u64).to_be_bytes());
    hasher.update(id.as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(4 + 64);
    out.push_str("pid:");
    for b in digest {
        out.push(char::from(HEX[usize::from(b >> 4)]));
        out.push(char::from(HEX[usize::from(b & 0x0f)]));
    }
    out
}

const HEX: &[u8; 16] = b"0123456789abcdef";

#[cfg(test)]
mod tests {
    use super::*;

    // mod 内の b"salt" 等はすべてテスト用の固定 salt で実在の秘密ではない。本番の
    // salt は pseudonymize の引数で受け取る設計で、OS の乱数源から作る。行末の
    // codeql 抑制（Issue #301）はこの理由（salt と入力の組で仮名の挙動を確かめる
    // ために値を固定している）。

    #[test]
    fn format_is_pid_prefix_and_64_lowercase_hex() {
        let p = pseudonymize(b"salt", "card-1"); // codeql[rust/hard-coded-cryptographic-value]
        assert!(p.starts_with("pid:"));
        assert_eq!(p.len(), 4 + 64);
        assert!(p[4..].bytes().all(|b| HEX.contains(&b)));
    }

    #[test]
    fn same_inputs_give_same_output_salt_or_id_changes_it() {
        let base = pseudonymize(b"salt-a", "m-human-1"); // codeql[rust/hard-coded-cryptographic-value]
        assert_eq!(base, pseudonymize(b"salt-a", "m-human-1")); // codeql[rust/hard-coded-cryptographic-value]
        assert_ne!(base, pseudonymize(b"salt-b", "m-human-1")); // codeql[rust/hard-coded-cryptographic-value]
        assert_ne!(base, pseudonymize(b"salt-a", "m-human-2")); // codeql[rust/hard-coded-cryptographic-value]
    }

    #[test]
    fn salt_and_id_boundary_is_unambiguous() {
        // 単純連結だと同じバイト列になる組でも、長さ前置きで別の hash になる。
        assert_ne!(pseudonymize(b"ab", "c"), pseudonymize(b"a", "bc")); // codeql[rust/hard-coded-cryptographic-value]
        assert_ne!(pseudonymize(b"", "abc"), pseudonymize(b"abc", "")); // codeql[rust/hard-coded-cryptographic-value]
    }

    #[test]
    fn empty_inputs_are_usable_and_distinct() {
        let e = pseudonymize(b"", ""); // codeql[rust/hard-coded-cryptographic-value]
        assert!(e.starts_with("pid:"));
        assert_ne!(e, pseudonymize(b"", "x")); // codeql[rust/hard-coded-cryptographic-value]
        assert_ne!(e, pseudonymize(b"x", "")); // codeql[rust/hard-coded-cryptographic-value]
    }
}
