//! `snapshot_hash` の計算。
//!
//! 承認者が承認する対象は、正規化した操作の中身 [`ActionPayload`]（操作の種類・
//! 宛先・本文・添付・使うアカウント・範囲。03 §5 の固定 payload）。カードの
//! コメントは承認の対象外（#87）: コメントを保存しても `snapshot_hash` は
//! 変わらず、保存済みの承認条件が無効にならない。
//!
//! 形式：`"sha256:" + 小文字 16 進 64 桁`。入力は曖昧さが出ないよう、各フィールドを
//! 8 バイトのビッグエンディアン長さ付きで連結する。同じ意味の操作は
//! [`ActionPayload::normalized`] で同じバイト列になり、同じ hash になる。

use sha2::{Digest, Sha256};

use crate::task::ActionKind;

const DOMAIN: &[u8] = b"pelway-contract/snapshot/v2";

fn put(hasher: &mut Sha256, field: &[u8]) {
    hasher.update((field.len() as u64).to_be_bytes());
    hasher.update(field);
}

/// 正規化した操作の中身（03 §5.1 の固定 payload）。承認の対象はこれだけで、
/// カードのコメントは含まない（#87）。
///
/// `snapshot_hash` は [`compute_snapshot_hash`] が [`ActionPayload::normalized`] の
/// 結果から計算する。同じ意味の操作は同じ hash になる:
///
/// - 本文（`body`）: 改行を LF に揃え、末尾の空白を落とす。
/// - 添付（`attachments`）と範囲（`scope`）: 並べ替えて重複を除く（集合として扱う）。
/// - 種類・宛先・アカウントはそのまま。表現が違えば別の操作として別の hash になる。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ActionPayload {
    /// 操作の種類（04 §9 の 4 値）。
    pub action_kind: ActionKind,
    /// 宛先。
    pub recipient: String,
    /// 本文。
    pub body: String,
    /// 添付の内容 hash（03 §5.1: 添付名だけでは固定しない）。
    pub attachments: Vec<String>,
    /// 使うアカウント（送信アカウント）。
    pub account: String,
    /// 許可する範囲（scope）。
    pub scope: Vec<String>,
}

impl ActionPayload {
    /// hash 計算に使う正規化した複製。
    ///
    /// - 本文: 改行を LF に揃え、末尾の空白（改行を含む）を落とす。
    /// - 添付・scope: 辞書順に並べ替えて重複を除く（集合として扱う）。
    /// - 種類・宛先・アカウント: そのまま。
    #[must_use]
    pub fn normalized(&self) -> Self {
        Self {
            action_kind: self.action_kind,
            recipient: self.recipient.clone(),
            body: normalize_text(&self.body),
            attachments: normalize_set(&self.attachments),
            account: self.account.clone(),
            scope: normalize_set(&self.scope),
        }
    }
}

/// 文章の正規化: CRLF と単独 CR を LF に揃え、末尾の空白を落とす。
fn normalize_text(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .trim_end()
        .to_owned()
}

/// 集合の正規化: 辞書順に並べ替えて重複を除く。
fn normalize_set(items: &[String]) -> Vec<String> {
    let mut sorted = items.to_vec();
    sorted.sort();
    sorted.dedup();
    sorted
}

/// 正規化した操作の中身（[`ActionPayload`]）から `snapshot_hash` を計算する。
/// コメントは hash の対象外（#87）。
#[must_use]
pub fn compute_snapshot_hash(payload: &ActionPayload) -> String {
    let p = payload.normalized();
    let mut hasher = Sha256::new();
    put(&mut hasher, DOMAIN);
    // 種類のタグは ActionKind::as_str（serde のワイヤ値と同じ 4 値）。
    put(&mut hasher, p.action_kind.as_str().as_bytes());
    put(&mut hasher, p.recipient.as_bytes());
    put(&mut hasher, p.body.as_bytes());
    hasher.update((p.attachments.len() as u64).to_be_bytes());
    for a in &p.attachments {
        put(&mut hasher, a.as_bytes());
    }
    put(&mut hasher, p.account.as_bytes());
    hasher.update((p.scope.len() as u64).to_be_bytes());
    for s in &p.scope {
        put(&mut hasher, s.as_bytes());
    }
    finish(hasher)
}

/// hasher を確定させ、`"sha256:" + 小文字 16 進 64 桁` の形式にする。
fn finish(hasher: Sha256) -> String {
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

    fn payload() -> ActionPayload {
        ActionPayload {
            action_kind: ActionKind::CustomerSend,
            recipient: "customer@example.invalid".to_owned(),
            body: "1行目\n2行目\n".to_owned(),
            attachments: vec!["sha256:att-1".to_owned(), "sha256:att-2".to_owned()],
            account: "acct-cw-1".to_owned(),
            scope: vec!["tickets:read".to_owned(), "tickets:write".to_owned()],
        }
    }

    fn owned(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|p| (*p).to_owned()).collect()
    }

    #[test]
    fn hash_format_is_sha256_hex() {
        let h = compute_snapshot_hash(&payload());
        assert!(h.starts_with("sha256:"));
        assert_eq!(h.len(), 7 + 64);
        assert!(h[7..].bytes().all(|b| HEX.contains(&b)));
    }

    // 受け入れ条件: 同じ意味の操作は同じ hash になる（正規化）。
    #[test]
    fn equivalent_payloads_hash_the_same() {
        let base = compute_snapshot_hash(&payload());
        // 本文の改行コード（CRLF）と末尾の空白・改行は同じ意味。
        for body in ["1行目\r\n2行目\r\n", "1行目\n2行目", "1行目\n2行目  \n\n"] {
            let mut p = payload();
            p.body = body.to_owned();
            assert_eq!(compute_snapshot_hash(&p), base, "body={body:?}");
        }
        // 添付・scope の順序と重複は同じ意味（集合として扱う）。
        for (attachments, scope) in [
            (
                vec!["sha256:att-2", "sha256:att-1"],
                vec!["tickets:write", "tickets:read"],
            ),
            (
                vec!["sha256:att-1", "sha256:att-2", "sha256:att-1"],
                vec!["tickets:read", "tickets:read", "tickets:write"],
            ),
        ] {
            let mut p = payload();
            p.attachments = owned(&attachments);
            p.scope = owned(&scope);
            assert_eq!(
                compute_snapshot_hash(&p),
                base,
                "attachments={attachments:?}"
            );
        }
    }

    // 受け入れ条件: 宛先や本文など、操作の中身が変われば hash が変わる。
    #[test]
    fn each_payload_field_changes_the_hash() {
        let base = compute_snapshot_hash(&payload());
        let variants = [
            ActionPayload {
                action_kind: ActionKind::ToolWrite,
                ..payload()
            },
            ActionPayload {
                recipient: "other@example.invalid".to_owned(),
                ..payload()
            },
            ActionPayload {
                body: "別の本文\n".to_owned(),
                ..payload()
            },
            ActionPayload {
                body: "1行目\n2行目 あ".to_owned(),
                ..payload()
            },
            ActionPayload {
                attachments: vec![],
                ..payload()
            },
            ActionPayload {
                attachments: owned(&["sha256:att-1"]),
                ..payload()
            },
            ActionPayload {
                attachments: owned(&["sha256:att-1", "sha256:att-2", "sha256:att-3"]),
                ..payload()
            },
            ActionPayload {
                account: "acct-cw-2".to_owned(),
                ..payload()
            },
            ActionPayload {
                scope: owned(&["tickets:read"]),
                ..payload()
            },
            ActionPayload {
                scope: vec![],
                ..payload()
            },
        ];
        for (i, p) in variants.into_iter().enumerate() {
            assert_ne!(compute_snapshot_hash(&p), base, "variant #{i}");
        }
    }

    #[test]
    fn payload_field_boundaries_are_unambiguous() {
        // 連結だけだと同じバイト列になる組でも、長さ前置で hash は別。
        let mut a = payload();
        a.recipient = "customer@ab.example".to_owned();
        a.body = "c".to_owned();
        let mut b = payload();
        b.recipient = "customer@a".to_owned();
        b.body = "b.examplec".to_owned();
        assert_ne!(compute_snapshot_hash(&a), compute_snapshot_hash(&b));
    }

    // 種類のタグ（as_str）は serde のワイヤ値と同じ 4 値であること。
    #[cfg(feature = "serde")]
    #[test]
    fn action_kind_tag_matches_serde_wire_value() {
        for kind in [
            ActionKind::CustomerSend,
            ActionKind::AgentHandoff,
            ActionKind::ToolRead,
            ActionKind::ToolWrite,
        ] {
            let wire = serde_json::to_string(&kind).expect("serialize");
            assert_eq!(wire, format!("\"{}\"", kind.as_str()));
        }
    }
}
