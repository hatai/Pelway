//! 送れなかった payload をためておき、次の周回で送り直す。

use serde_json::Value;

/// ためておく payload の上限。超えたら古いものから捨てる。
pub const OUTBOX_LIMIT: usize = 100;

/// payload を列の末尾に足す。上限を超えたら古いものから捨てる。
pub fn push(outbox: &mut Vec<Value>, payload: Value) {
    outbox.push(payload);
    if outbox.len() > OUTBOX_LIMIT {
        let excess = outbox.len() - OUTBOX_LIMIT;
        outbox.drain(..excess);
    }
}

/// 古い順に送る。送れたものは列から外し、送れなかったものとそれより後ろは残す（順番を保つため）。
/// 送れた数を返す。
pub fn flush(outbox: &mut Vec<Value>, mut send: impl FnMut(&Value) -> bool) -> usize {
    let sent = outbox.iter().take_while(|v| send(v)).count();
    outbox.drain(..sent);
    sent
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn push_caps_and_drops_oldest() {
        let mut o = Vec::new();
        for i in 0..(OUTBOX_LIMIT + 2) {
            push(&mut o, json!(i));
        }
        assert_eq!(o.len(), OUTBOX_LIMIT);
        assert_eq!(o[0], json!(2));
    }

    #[test]
    fn flush_removes_sent() {
        let mut o = vec![json!(1), json!(2)];
        assert_eq!(flush(&mut o, |_| true), 2);
        assert!(o.is_empty());
    }

    #[test]
    fn flush_keeps_failed_and_later() {
        let mut o = vec![json!(1), json!(2), json!(3)];
        let sent = flush(&mut o, |v| *v == json!(1));
        assert_eq!(sent, 1);
        assert_eq!(o, vec![json!(2), json!(3)]);
    }
}
