//! 承認カードからの一連の処理をつなぐ trace id（02「観測」・#103）。
//!
//! 仕様 02 の「承認・開始資格・結果照会の各段階に同じ trace idを最初から付ける」に
//! 従い、trace id は **カード ID だけから決まる**（決定論的に導出する）。カードの
//! 作成時から変わらないので、段階が増えても（開始資格・結果照会は TODO(U01) で未実装）
//! 同じカードの処理は最初から同じ trace id を共有できる。
//!
//! 形式は W3C Trace Context の trace-id と同じ 32 桁の小文字 hex。
//! 導出は SHA-256（[`crate::snapshot`] の hash 計算と同じ crate）で、
//! 入力には固定のドメイン分離文字列を混ぜる（ほかの用途の hash との衝突を避ける）。
//!
//! 台帳（[`crate::ledger::ApprovalLedger`]）の承認まわりの操作は、この trace id を
//! 持つ [`tracing::info_span!`] で囲む。コメント保存・承認の予約・疑似送信が同じ
//! カードなら同じ trace id の span に載る。

#![forbid(unsafe_code)]

use std::fmt;

use crate::ids::CardId;
use sha2::{Digest, Sha256};

/// trace id の導出に混ぜる固定値（ドメイン分離）。
const TRACE_DOMAIN: &[u8] = b"pelway-contract/trace/v1";

/// trace id（W3C Trace Context の trace-id と同じ 32 桁の小文字 hex）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraceId(String);

impl TraceId {
    /// 32 桁の小文字 hex。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TraceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// カード ID から trace id を導出する。同じカードなら常に同じ値。
///
/// SHA-256 の先頭 16 バイトを hex（32 桁）にする。カード ID だけで決まるため、
/// 状態を持たず、どの段階（コメント保存・承認・将来の開始資格・結果照会）でも
/// 同じ値を最初から作れる。
#[must_use]
pub fn trace_id_for_card(card_id: &CardId) -> TraceId {
    let mut hasher = Sha256::new();
    hasher.update(TRACE_DOMAIN);
    hasher.update(card_id.as_str());
    let digest = hasher.finalize();
    let hex: String = digest[..16].iter().map(|b| format!("{b:02x}")).collect();
    TraceId(hex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_id_is_32_lowercase_hex_chars() {
        let tid = trace_id_for_card(&CardId::from("card-1"));
        assert_eq!(tid.as_str().len(), 32);
        assert!(
            tid.as_str()
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()),
            "{}",
            tid
        );
    }

    #[test]
    fn trace_id_is_stable_for_same_card() {
        let a1 = trace_id_for_card(&CardId::from("card-1"));
        let a2 = trace_id_for_card(&CardId::from("card-1"));
        assert_eq!(a1, a2);
        // 違うカード（1 文字違い・前置き違い）は全部違う値。
        for other in ["card-2", "xcard-1", ""] {
            assert_ne!(a1, trace_id_for_card(&CardId::from(other)), "{other:?}");
        }
    }

    #[test]
    fn trace_id_display_is_the_hex_string() {
        let tid = trace_id_for_card(&CardId::from("card-1"));
        assert_eq!(tid.to_string(), tid.as_str().to_owned());
    }
}
