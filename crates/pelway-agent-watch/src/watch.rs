//! 前回の状態と今回のスナップショットを比べて、通知するイベントを決める。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::parse::parse_utc_epoch;

/// 実行中のまま更新がないとき `stalled` にするまでの秒数。
pub const NO_ACTIVITY_SECS: i64 = 45 * 60;

/// `t3 thread list --json` の 1 要素。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ListedThread {
    #[serde(rename = "threadId")]
    pub id: String,
    pub status: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

/// `t3 thread show --json` から読んだスレッドの詳細。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ThreadDetail {
    pub id: String,
    pub status: String,
    pub updated_at: String,
    pub branch: Option<String>,
    pub provider: String,
    pub model: String,
    pub link: String,
    pub run_count: u32,
    pub pending_requests: u32,
    pub last_assistant_text: Option<String>,
}

/// スレッドごとに覚えておく前回の値。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ThreadMemo {
    pub status: String,
    pub updated_at: String,
    pub run_count: u32,
    pub pending_requests: u32,
    /// `no-activity` を送ったときの `updated_at`。同じ停滞を 2 回送らないため。
    pub stalled_for: Option<String>,
}

/// 状態ファイルに保存する内容。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct State {
    pub initialized: bool,
    pub threads: BTreeMap<String, ThreadMemo>,
    /// ブランチごとに、`ci-failed-twice` を送ったときの最新の head sha。
    pub ci_notified: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StallReason {
    NoActivity,
    CiFailedTwice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Finished,
    Failed,
    Cancelled,
    Waiting,
    Stalled(StallReason),
}

/// 実行が終わった状態か。
fn is_terminal(status: &str) -> bool {
    matches!(status, "completed" | "failed" | "cancelled")
}

fn terminal_event(status: &str) -> Option<Event> {
    match status {
        "completed" => Some(Event::Finished),
        "failed" => Some(Event::Failed),
        "cancelled" => Some(Event::Cancelled),
        _ => None,
    }
}

/// このスレッドを `t3 thread show` で詳しく見る必要があるか。
/// 初回（状態の記録だけを行う回）は見ない。
pub fn needs_detail(state: &State, listed: &ListedThread) -> bool {
    if !state.initialized {
        return false;
    }
    match state.threads.get(&listed.id) {
        None => true,
        Some(memo) => {
            memo.status != listed.status
                || memo.updated_at != listed.updated_at
                || listed.status == "running"
        }
    }
}

/// 今回の詳細で状態を更新し、通知するイベントを返す。
///
/// - 初回は `listed` をすべて記録するだけで、イベントは返さない
/// - `details` に無いスレッドの記録は変えない（`t3` の一時的な欠落や詳細の取得失敗で再通知しないため）
pub fn step(
    state: &mut State,
    details: &[ThreadDetail],
    listed: &[ListedThread],
    now: i64,
) -> Vec<(Event, ThreadDetail)> {
    if !state.initialized {
        for l in listed {
            state.threads.insert(
                l.id.clone(),
                ThreadMemo {
                    status: l.status.clone(),
                    updated_at: l.updated_at.clone(),
                    ..ThreadMemo::default()
                },
            );
        }
        state.initialized = true;
        return Vec::new();
    }

    let mut out = Vec::new();
    for d in details {
        let prev = state.threads.get(&d.id).cloned();

        if let Some(event) = terminal_event(&d.status) {
            let emit = match &prev {
                None => true,
                Some(p) if !is_terminal(&p.status) => true,
                // 前回も終わっていたが、その間に 1 往復した（実行回数が増えた）。
                // 実行回数が分からない記録（初回に記録したもの）では判断しない。
                Some(p) => p.run_count > 0 && d.run_count > p.run_count,
            };
            if emit {
                out.push((event, d.clone()));
            }
        }

        let prev_pending = prev.as_ref().map_or(0, |p| p.pending_requests);
        if d.pending_requests > 0 && prev_pending == 0 {
            out.push((Event::Waiting, d.clone()));
        }

        let mut stalled_for = prev.as_ref().and_then(|p| p.stalled_for.clone());
        if d.status == "running"
            && parse_utc_epoch(&d.updated_at).is_some_and(|t| now - t >= NO_ACTIVITY_SECS)
            && stalled_for.as_deref() != Some(d.updated_at.as_str())
        {
            out.push((Event::Stalled(StallReason::NoActivity), d.clone()));
            stalled_for = Some(d.updated_at.clone());
        }

        state.threads.insert(
            d.id.clone(),
            ThreadMemo {
                status: d.status.clone(),
                updated_at: d.updated_at.clone(),
                run_count: d.run_count,
                pending_requests: d.pending_requests,
                stalled_for,
            },
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: &str = "2026-11-10T16:00:00Z";
    const T1: &str = "2026-11-10T16:10:00Z";

    fn listed(id: &str, status: &str, updated_at: &str) -> ListedThread {
        ListedThread {
            id: id.into(),
            status: status.into(),
            updated_at: updated_at.into(),
        }
    }

    fn detail(
        id: &str,
        status: &str,
        updated_at: &str,
        run_count: u32,
        pending: u32,
    ) -> ThreadDetail {
        ThreadDetail {
            id: id.into(),
            status: status.into(),
            updated_at: updated_at.into(),
            run_count,
            pending_requests: pending,
            ..ThreadDetail::default()
        }
    }

    fn memo(status: &str, updated_at: &str, run_count: u32) -> ThreadMemo {
        ThreadMemo {
            status: status.into(),
            updated_at: updated_at.into(),
            run_count,
            ..ThreadMemo::default()
        }
    }

    fn initialized(threads: &[(&str, ThreadMemo)]) -> State {
        State {
            initialized: true,
            threads: threads
                .iter()
                .map(|(id, m)| (id.to_string(), m.clone()))
                .collect(),
            ..State::default()
        }
    }

    fn events(out: &[(Event, ThreadDetail)]) -> Vec<(Event, String)> {
        out.iter().map(|(e, d)| (*e, d.id.clone())).collect()
    }

    fn now_after(ts: &str, secs: i64) -> i64 {
        parse_utc_epoch(ts).unwrap() + secs
    }

    #[test]
    fn first_run_is_baseline() {
        let mut state = State::default();
        let listed = vec![
            listed("a", "completed", T0),
            listed("b", "completed", T0),
            listed("c", "failed", T0),
        ];
        let details: Vec<ThreadDetail> = vec![];
        let out = step(&mut state, &details, &listed, now_after(T0, 60));
        assert!(out.is_empty());
        assert!(state.initialized);
        assert_eq!(state.threads.len(), 3);
        assert_eq!(state.threads["c"].status, "failed");
    }

    #[test]
    fn first_run_needs_no_detail() {
        assert!(!needs_detail(
            &State::default(),
            &listed("a", "running", T0)
        ));
    }

    #[test]
    fn needs_detail_rules() {
        let state = initialized(&[
            ("a", memo("completed", T0, 1)),
            ("r", memo("running", T0, 1)),
        ]);
        assert!(!needs_detail(&state, &listed("a", "completed", T0)));
        assert!(needs_detail(&state, &listed("a", "completed", T1)));
        assert!(needs_detail(&state, &listed("r", "running", T0)));
        assert!(needs_detail(&state, &listed("new", "completed", T0)));
    }

    #[test]
    fn running_to_completed_emits_finished() {
        let mut state = initialized(&[("a", memo("running", T0, 1))]);
        let out = step(
            &mut state,
            &[detail("a", "completed", T1, 1, 0)],
            &[listed("a", "completed", T1)],
            now_after(T1, 5),
        );
        assert_eq!(events(&out), vec![(Event::Finished, "a".to_string())]);
        assert_eq!(state.threads["a"].status, "completed");
    }

    #[test]
    fn completed_stays_completed_is_silent() {
        let mut state = initialized(&[("a", memo("completed", T0, 1))]);
        let out = step(
            &mut state,
            &[],
            &[listed("a", "completed", T0)],
            now_after(T0, 5),
        );
        assert!(out.is_empty());
    }

    #[test]
    fn missed_completion_after_restart() {
        let mut state = initialized(&[("a", memo("completed", T0, 1))]);
        let out = step(
            &mut state,
            &[detail("a", "completed", T1, 2, 0)],
            &[listed("a", "completed", T1)],
            now_after(T1, 5),
        );
        assert_eq!(events(&out), vec![(Event::Finished, "a".to_string())]);
        assert_eq!(state.threads["a"].run_count, 2);
    }

    #[test]
    fn failed_and_cancelled() {
        let mut state =
            initialized(&[("f", memo("running", T0, 1)), ("c", memo("running", T0, 1))]);
        let out = step(
            &mut state,
            &[
                detail("f", "failed", T1, 1, 0),
                detail("c", "cancelled", T1, 1, 0),
            ],
            &[listed("f", "failed", T1), listed("c", "cancelled", T1)],
            now_after(T1, 5),
        );
        assert_eq!(
            events(&out),
            vec![
                (Event::Failed, "f".to_string()),
                (Event::Cancelled, "c".to_string())
            ]
        );
    }

    #[test]
    fn pending_request_emits_waiting_once() {
        let mut state = initialized(&[("a", memo("running", T0, 1))]);
        let l = [listed("a", "running", T0)];
        let first = step(
            &mut state,
            &[detail("a", "running", T0, 1, 1)],
            &l,
            now_after(T0, 5),
        );
        let second = step(
            &mut state,
            &[detail("a", "running", T0, 1, 1)],
            &l,
            now_after(T0, 35),
        );
        assert_eq!(events(&first), vec![(Event::Waiting, "a".to_string())]);
        assert!(second.is_empty());
    }

    #[test]
    fn no_activity_stall_once() {
        let mut state = initialized(&[("a", memo("running", T0, 1))]);
        let l = [listed("a", "running", T0)];
        let d = [detail("a", "running", T0, 1, 0)];
        let first = step(&mut state, &d, &l, now_after(T0, 46 * 60));
        let second = step(&mut state, &d, &l, now_after(T0, 47 * 60));
        assert_eq!(
            events(&first),
            vec![(Event::Stalled(StallReason::NoActivity), "a".to_string())]
        );
        assert!(second.is_empty());
    }

    #[test]
    fn no_activity_not_before_threshold() {
        let mut state = initialized(&[("a", memo("running", T0, 1))]);
        let out = step(
            &mut state,
            &[detail("a", "running", T0, 1, 0)],
            &[listed("a", "running", T0)],
            now_after(T0, 44 * 60),
        );
        assert!(out.is_empty());
    }

    #[test]
    fn no_activity_rearms_after_update() {
        let mut state = initialized(&[("a", memo("running", T0, 1))]);
        step(
            &mut state,
            &[detail("a", "running", T0, 1, 0)],
            &[listed("a", "running", T0)],
            now_after(T0, 46 * 60),
        );
        let out = step(
            &mut state,
            &[detail("a", "running", T1, 1, 0)],
            &[listed("a", "running", T1)],
            now_after(T1, 46 * 60),
        );
        assert_eq!(
            events(&out),
            vec![(Event::Stalled(StallReason::NoActivity), "a".to_string())]
        );
    }

    #[test]
    fn missing_from_list_is_kept() {
        let mut state = initialized(&[("a", memo("completed", T0, 1))]);
        let out = step(&mut state, &[], &[], now_after(T0, 5));
        assert!(out.is_empty());
        assert!(state.threads.contains_key("a"));
    }

    #[test]
    fn new_thread_finished_between_polls() {
        let mut state = initialized(&[]);
        let out = step(
            &mut state,
            &[detail("n", "completed", T1, 1, 0)],
            &[listed("n", "completed", T1)],
            now_after(T1, 5),
        );
        assert_eq!(events(&out), vec![(Event::Finished, "n".to_string())]);
    }

    #[test]
    fn baseline_thread_metadata_change_is_silent() {
        // 初回に記録したスレッド（実行回数が分からない）は、更新時刻だけ変わっても通知しない
        let mut state = State::default();
        step(
            &mut state,
            &[],
            &[listed("a", "completed", T0)],
            now_after(T0, 5),
        );
        let out = step(
            &mut state,
            &[detail("a", "completed", T1, 3, 0)],
            &[listed("a", "completed", T1)],
            now_after(T1, 5),
        );
        assert!(out.is_empty());
        assert_eq!(state.threads["a"].run_count, 3);
    }

    #[test]
    fn new_running_thread_is_silent() {
        let mut state = initialized(&[]);
        let out = step(
            &mut state,
            &[detail("n", "running", T1, 1, 0)],
            &[listed("n", "running", T1)],
            now_after(T1, 5),
        );
        assert!(out.is_empty());
        assert_eq!(state.threads["n"].status, "running");
    }
}
