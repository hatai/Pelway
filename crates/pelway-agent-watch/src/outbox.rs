//! webhook に送れなかった payload をためておき、古い順に送り直す。
//! 直らない失敗の payload は列から外す。送り直せる失敗が続く間は、次に送るまでの間隔を空ける。

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// ためておく payload の上限。超えたら古いものから捨てる。
pub const OUTBOX_LIMIT: usize = 100;
/// 送り直せる失敗の後に空ける最初の間隔（秒）。見張りの周回の間隔と同じで、最初の送り直しは次の周回になる。
pub const BACKOFF_INITIAL_SECS: i64 = 30;
/// 空ける間隔の上限（秒）。
pub const BACKOFF_MAX_SECS: i64 = 15 * 60;

/// 1 件の送信の結果を、列の扱いで分けたもの。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// 届いた。列から外し、間隔を初期値に戻す。
    Sent,
    /// 受け手が直らない失敗を返した。列から外し、同じ周回で次を送る。
    Rejected,
    /// 送り直せる失敗。列に残して間隔を空け、この周回の送信をやめる。
    Retry,
}

/// 送り直せる失敗が続いている間の、次に送るまでの間隔。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Backoff {
    /// 直近に空けた間隔（秒）。次の失敗ではこの倍（上限 `BACKOFF_MAX_SECS`）を空ける。
    pub interval_secs: i64,
    /// この時刻（UNIX 秒）より前の周回では送らない。
    pub next_send_at: i64,
}

/// payload を列の末尾に足す。上限を超えたら古いものから捨てる。
pub fn push(outbox: &mut Vec<Value>, payload: Value) {
    outbox.push(payload);
    if outbox.len() > OUTBOX_LIMIT {
        let excess = outbox.len() - OUTBOX_LIMIT;
        outbox.drain(..excess);
    }
}

/// 古い順に送る。`backoff` の次に送る時刻（`now` は UNIX 秒）より前なら何も送らない。
/// 1 件送る前に毎回 `has_time` を呼び、偽ならその周回の送信をやめる（列も間隔もそのまま）。
pub fn flush(
    outbox: &mut Vec<Value>,
    backoff: &mut Option<Backoff>,
    now: i64,
    mut has_time: impl FnMut() -> bool,
    mut send: impl FnMut(&Value) -> Outcome,
) {
    if backoff.is_some_and(|b| now < b.next_send_at) {
        return;
    }
    while let Some(head) = outbox.first() {
        if !has_time() {
            return;
        }
        match send(head) {
            Outcome::Sent => {
                outbox.remove(0);
                *backoff = None;
            }
            Outcome::Rejected => {
                outbox.remove(0);
            }
            Outcome::Retry => {
                let interval_secs = backoff.map_or(BACKOFF_INITIAL_SECS, |b| {
                    (b.interval_secs * 2).min(BACKOFF_MAX_SECS)
                });
                *backoff = Some(Backoff {
                    interval_secs,
                    next_send_at: now + interval_secs,
                });
                return;
            }
        }
    }
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
        flush(&mut o, &mut None, 0, || true, |_| Outcome::Sent);
        assert!(o.is_empty());
    }

    #[test]
    fn flush_keeps_failed_and_later() {
        let mut o = vec![json!(1), json!(2), json!(3)];
        flush(
            &mut o,
            &mut None,
            0,
            || true,
            |v| {
                if *v == json!(1) {
                    Outcome::Sent
                } else {
                    Outcome::Retry
                }
            },
        );
        assert_eq!(o, vec![json!(2), json!(3)]);
    }

    #[test]
    fn flush_drops_rejected_and_continues() {
        let mut o = vec![json!(1), json!(2), json!(3)];
        let mut b = None;
        flush(
            &mut o,
            &mut b,
            0,
            || true,
            |v| match v.as_i64() {
                Some(1) => Outcome::Rejected,
                Some(2) => Outcome::Sent,
                _ => Outcome::Retry,
            },
        );
        assert_eq!(o, vec![json!(3)]);
        assert_eq!(b.map(|b| b.interval_secs), Some(BACKOFF_INITIAL_SECS));
    }

    #[test]
    fn flush_stops_when_out_of_time() {
        let mut o = vec![json!(1), json!(2)];
        let mut b = None;
        let mut starts = 0;
        let has_time = || {
            starts += 1;
            starts == 1
        };
        flush(&mut o, &mut b, 0, has_time, |_| Outcome::Sent);
        // 時間が尽きたら次を送り始めず、列に残す。失敗ではないので間隔は空けない
        assert_eq!(o, vec![json!(2)]);
        assert_eq!(b, None);
    }

    #[test]
    fn backoff_doubles_up_to_cap() {
        let mut o = vec![json!(1)];
        let mut b = None;
        let mut now = 1_000;
        let mut intervals = Vec::new();
        for _ in 0..7 {
            flush(&mut o, &mut b, now, || true, |_| Outcome::Retry);
            let set = b.expect("a retryable failure sets the backoff");
            assert_eq!(set.next_send_at, now + set.interval_secs);
            intervals.push(set.interval_secs);
            // 次に送る時刻より前の周回では送らない
            let mut tried = false;
            flush(
                &mut o,
                &mut b,
                set.next_send_at - 1,
                || true,
                |_| {
                    tried = true;
                    Outcome::Sent
                },
            );
            assert!(!tried);
            assert_eq!(b, Some(set));
            now = set.next_send_at;
        }
        assert_eq!(intervals, [30, 60, 120, 240, 480, 900, 900]);
        assert_eq!(o, vec![json!(1)]);
    }

    #[test]
    fn backoff_resets_after_success() {
        let mut o = vec![json!(1), json!(2)];
        let mut b = None;
        flush(&mut o, &mut b, 0, || true, |_| Outcome::Retry);
        flush(&mut o, &mut b, 30, || true, |_| Outcome::Retry);
        assert_eq!(b.map(|b| b.interval_secs), Some(60));
        // 1 件目が届いた後、2 件目で送り直せる失敗が出ると、空ける間隔は初期値から数え直す
        flush(
            &mut o,
            &mut b,
            90,
            || true,
            |v| {
                if *v == json!(1) {
                    Outcome::Sent
                } else {
                    Outcome::Retry
                }
            },
        );
        assert_eq!(o, vec![json!(2)]);
        let initial = Backoff {
            interval_secs: BACKOFF_INITIAL_SECS,
            next_send_at: 90 + BACKOFF_INITIAL_SECS,
        };
        assert_eq!(b, Some(initial));
        flush(&mut o, &mut b, 120, || true, |_| Outcome::Sent);
        assert!(o.is_empty());
        assert_eq!(b, None);
    }

    #[test]
    fn queued_order_is_kept_during_backoff() {
        let mut o = vec![json!(1)];
        let mut b = None;
        flush(&mut o, &mut b, 0, || true, |_| Outcome::Retry);
        // 間隔を空けている間に増えた知らせは後ろに並び、まだ送らない
        let mut sent = Vec::new();
        push(&mut o, json!(2));
        flush(
            &mut o,
            &mut b,
            10,
            || true,
            |v| {
                sent.push(v.clone());
                Outcome::Sent
            },
        );
        push(&mut o, json!(3));
        assert!(sent.is_empty());
        assert_eq!(o, vec![json!(1), json!(2), json!(3)]);
        // 空ける時刻が来たら古い順に送る
        flush(
            &mut o,
            &mut b,
            30,
            || true,
            |v| {
                sent.push(v.clone());
                Outcome::Sent
            },
        );
        assert_eq!(sent, vec![json!(1), json!(2), json!(3)]);
        assert!(o.is_empty());
    }
}
