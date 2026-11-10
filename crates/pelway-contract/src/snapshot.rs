//! `snapshot_hash` の計算。
//!
//! 承認者が承認する対象は「固定 payload（宛先・本文・添付 hash・scope 等）」と
//! 「その時点のカードのコメント」の組。コメントが後で送る中身に含まれうるため、
//! コメントを保存すると `snapshot_hash` も変わる（`card_version` は変わらない）。
//!
//! 形式：`"sha256:" + 小文字 16 進 64 桁`。入力は曖昧さが出ないよう、各フィールドを
//! 8 バイトのビッグエンディアン長さ付きで連結する。

use sha2::{Digest, Sha256};

use crate::approval::CardComment;
use crate::member::MemberKind;

const DOMAIN: &[u8] = b"pelway-contract/snapshot/v1";

fn put(hasher: &mut Sha256, field: &[u8]) {
    hasher.update((field.len() as u64).to_be_bytes());
    hasher.update(field);
}

fn kind_tag(kind: MemberKind) -> &'static [u8] {
    match kind {
        MemberKind::Human => b"human",
        MemberKind::Ai => b"ai",
    }
}

/// 固定 payload の hash とコメント列から `snapshot_hash` を計算する。
#[must_use]
pub fn compute_snapshot_hash(payload_hash: &str, comments: &[CardComment]) -> String {
    let mut hasher = Sha256::new();
    put(&mut hasher, DOMAIN);
    put(&mut hasher, payload_hash.as_bytes());
    hasher.update((comments.len() as u64).to_be_bytes());
    for c in comments {
        put(&mut hasher, c.comment_id.as_str().as_bytes());
        put(&mut hasher, c.author_id.as_str().as_bytes());
        put(&mut hasher, kind_tag(c.member_kind));
        put(&mut hasher, c.body.as_bytes());
    }
    let digest = hasher.finalize();
    let mut out = String::with_capacity(7 + 64);
    out.push_str("sha256:");
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

    fn c(id: &str, body: &str) -> CardComment {
        CardComment {
            comment_id: id.into(),
            author_id: "m-human-1".into(),
            member_kind: MemberKind::Human,
            body: body.to_owned(),
        }
    }

    #[test]
    fn format_is_sha256_hex() {
        let h = compute_snapshot_hash("payload", &[]);
        assert!(h.starts_with("sha256:"));
        assert_eq!(h.len(), 7 + 64);
        assert!(h[7..].bytes().all(|b| HEX.contains(&b)));
    }

    #[test]
    fn hash_changes_with_payload_and_each_comment_field() {
        let base = compute_snapshot_hash("p1", &[c("cmt-1", "a")]);
        assert_ne!(base, compute_snapshot_hash("p2", &[c("cmt-1", "a")]));
        assert_ne!(base, compute_snapshot_hash("p1", &[]));
        assert_ne!(base, compute_snapshot_hash("p1", &[c("cmt-1", "b")]));
        assert_ne!(base, compute_snapshot_hash("p1", &[c("cmt-2", "a")]));
        let mut ai = c("cmt-1", "a");
        ai.member_kind = MemberKind::Ai;
        assert_ne!(base, compute_snapshot_hash("p1", &[ai]));
        assert_eq!(base, compute_snapshot_hash("p1", &[c("cmt-1", "a")]));
    }

    #[test]
    fn field_boundaries_are_unambiguous() {
        // 連結だけだと同じバイト列になる組でも hash は別。
        assert_ne!(
            compute_snapshot_hash("ab", &[c("cmt-1", "c")]),
            compute_snapshot_hash("a", &[c("cmt-1", "bc")])
        );
        assert_ne!(
            compute_snapshot_hash("p", &[c("cmt-1", "x"), c("cmt-2", "y")]),
            compute_snapshot_hash("p", &[c("cmt-2", "y"), c("cmt-1", "x")])
        );
    }
}
