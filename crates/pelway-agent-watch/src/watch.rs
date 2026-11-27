//! 前回の状態と今回のスナップショットを比べて、通知するイベントを決める。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::parse::parse_utc_epoch;

/// 実行中のまま更新がないとき `stalled` にするまでの秒数。
pub const NO_ACTIVITY_SECS: i64 = 45 * 60;

/// JSON の値を、文字列なら `Some`、それ以外（欠落・null・数値など）は `None` にする。
/// `createdBy` の型が変わっても一覧・詳細の読み取り自体は失敗させず、立て手不明に倒すため。
fn as_opt_string<'de, D>(d: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<serde_json::Value>::deserialize(d)?.and_then(|v| v.as_str().map(str::to_string)))
}

/// `t3 thread list --json` の 1 要素。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ListedThread {
    #[serde(rename = "threadId")]
    pub id: String,
    pub status: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    /// スレッドの立て手（`createdBy`。`"agent"` はエージェント、それ以外は人間として扱う。
    /// `docs/agents/grokbot-runbook.md` の shared-worktree と同じ見分け）。項目が無い・
    /// 文字列でないときは `None`（立て手不明）。
    #[serde(default, rename = "createdBy", deserialize_with = "as_opt_string")]
    pub created_by: Option<String>,
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
    /// `ListedThread::created_by` と同じ項目を詳細からも読む（無い・読めないときは `None`）。
    pub created_by: Option<String>,
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
    /// 初回の記録をした時刻（UNIX 秒）。これより前に更新された未知のスレッドは通知しない。
    #[serde(default)]
    pub baseline_at: i64,
    pub threads: BTreeMap<String, ThreadMemo>,
    /// ブランチごとに、`ci-failed-twice` を送った（または初回に記録した）ときの最新の head sha。
    pub ci_notified: BTreeMap<String, String>,
    /// webhook に送れなかった payload（古い順）。`outbox::flush` が送り直す。
    #[serde(default)]
    pub outbox: Vec<serde_json::Value>,
    /// 送り直せる失敗が続いている間の、送信待ちの列の送り直しの間隔。送れたら `None` に戻る。
    #[serde(default)]
    pub outbox_backoff: Option<crate::outbox::Backoff>,
    /// 指揮役の受信箱に書けなかった payload（古い順）。次の周回で、新しい知らせより先に書き直す。
    /// `None` はこの項目の無い更新前の版の状態で、送信待ちの列の知らせを受信箱に書けたかが分からない。
    /// 要素は payload そのものに留める。別の形に包むと、更新前の版に戻したときに
    /// 包んだ形のまま受信箱に書かれて、読み手が拾えない行になる。
    #[serde(default)]
    pub inbox_pending: Option<Vec<serde_json::Value>>,
    /// `inbox_pending` に残した知らせの元のスレッドの立て手（thread_id → `createdBy`）。
    /// 追記に失敗した時点で判明していた立て手を残す。残さないと、追記をやり直す回では
    /// 完了済みスレッドの詳細を取り直さないため、立て手不明として起こす側に倒れてしまう。
    /// 立て手は payload に入れずこの別項目に持つ（上の `inbox_pending` の形の約束）。
    /// キューから外れたスレッドの項目（全部書けた・上限で外れた）は消し、キューに残る
    /// 知らせの元のスレッドの分だけを残す。
    #[serde(default)]
    pub inbox_pending_creators: BTreeMap<String, String>,
    /// 前回指揮役を起こしたときの、指揮役の既読の位置（補助記録の `inbox_offset`）。
    /// 既読の位置がここから進むまで、重ねて起こさない。
    #[serde(default)]
    pub last_wake_inbox_offset: Option<u64>,
    /// 前回起こした指揮役のスレッド ID。登録が替わったら、`last_wake_inbox_offset` と
    /// `last_wake_at` は使わない。
    #[serde(default)]
    pub last_wake_thread_id: Option<String>,
    /// 前回指揮役を起こした時刻（UNIX 秒）。ここから起こしの最短間隔の間は起こさない（Issue #839）。
    #[serde(default)]
    pub last_wake_at: Option<i64>,
    /// 起こすに足る知らせを起こせず、後に回している（起こし待ちの印。Issue #839。最短間隔の内・
    /// 既読が進んでいない・補助記録が読めない・既読が範囲外のときに付ける）。値は、後に回した
    /// 知らせのうち最後のものまでの受信箱の行数（数えきれなかったとき・受信箱が自分の追記以外で変わって印の行番号を確かめられないときは
    /// `u64::MAX`）。起こせる周回で、
    /// 新しい知らせが無くても 1 回起こす（指揮役の既読の位置がこの行数に追いついていれば取り消す）。
    #[serde(default)]
    pub wake_pending: Option<u64>,
    /// 最後に数えた受信箱の行数（Issue #839）。知らせを受信箱に書いた周と、印だけの周で受信箱の
    /// 同一性が変わっていたときに数える。数えている間に受信箱が変わった・数え直せないときは None。
    /// 印だけの周で、補助記録の既読の位置が範囲内かを確かめるのに使う。
    #[serde(default)]
    pub inbox_lines: Option<u64>,
    /// `inbox_lines` を数えたときの受信箱の同一性（デバイス・inode・大きさ・mtime・ctime）。
    /// 印だけの周で受信箱がこれと違えば、数え直す（書き換えられた受信箱で古い行数を使わない）。
    /// 知らせを書く周でも、書く前の同一性と比べ、違えば（自分の追記以外で受信箱が変わった）
    /// 印と新しい知らせの行番号を不明にする。
    #[serde(default)]
    pub inbox_identity: Option<[u64; 5]>,
    /// 受信箱が自分の追記以外で変わったのを見分けた（Issue #839）。その間の指揮役の既読の位置は、
    /// いまの受信箱の同じ中身を指すと確かめられないので、新しい知らせと印の行番号を不明にし、
    /// 既読による取り消しをせず、印が立っている間は前回の起こしの基準を使わない（印の間に
    /// 送った起こしの位置も含む。重ねての起こしは最短間隔だけで絞る）。
    /// 起こしを送った後に、指揮役の既読の位置がそのときから変わったら消す。
    #[serde(default)]
    pub inbox_rewritten: bool,
    /// `inbox_rewritten` の間に起こしを送ったときの、指揮役の既読の位置。
    #[serde(default)]
    pub rewritten_wake_offset: Option<u64>,
    /// スレッドごとに、起こしに数えた最後の承認待ち（`waiting`）の round（Issue #839）。
    /// 同じスレッドの同じ round の承認待ちは、2 件目から起こしに数えない。
    /// 一覧の記録（`threads`）から外れたスレッドの項目は消す。
    #[serde(default)]
    pub wake_waiting_rounds: BTreeMap<String, u32>,
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
        // 空の一覧（T3 の起動直後など）では記録しない。空のまま記録すると、次の周回で全スレッドを新規と誤解する
        if listed.is_empty() {
            return Vec::new();
        }
        state.baseline_at = now;
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
                // 初回の記録より前に更新されたもの（初回の一覧に欠けていた古いスレッド）は通知しない
                None => parse_utc_epoch(&d.updated_at).is_some_and(|t| t >= state.baseline_at),
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
            created_by: None,
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
    fn first_run_empty_list_does_not_baseline() {
        let mut state = State::default();
        let out = step(&mut state, &[], &[], now_after(T0, 5));
        assert!(out.is_empty());
        assert!(!state.initialized);
    }

    #[test]
    fn baseline_records_time() {
        let mut state = State::default();
        step(
            &mut state,
            &[],
            &[listed("a", "completed", T0)],
            now_after(T1, 0),
        );
        assert_eq!(state.baseline_at, now_after(T1, 0));
    }

    #[test]
    fn unknown_thread_updated_before_baseline_is_silent() {
        // 初回の一覧に欠けていた古いスレッドは、あとで見つかっても通知しない
        let mut state = initialized(&[]);
        state.baseline_at = now_after(T1, 0);
        let out = step(
            &mut state,
            &[detail("old", "completed", T0, 1, 0)],
            &[listed("old", "completed", T0)],
            now_after(T1, 30),
        );
        assert!(out.is_empty());
        assert_eq!(state.threads["old"].status, "completed");
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

    #[test]
    fn old_state_without_backoff_loads() {
        let old =
            r#"{"initialized":true,"threads":{},"ci_notified":{},"outbox":[{"event":"finished"}]}"#;
        let mut state: State = serde_json::from_str(old).unwrap();
        assert_eq!(state.outbox.len(), 1);
        assert_eq!(state.outbox_backoff, None);
        // 間隔と次に送る時刻は状態ファイルに書かれ、読み直しても同じ値になる
        state.outbox_backoff = Some(crate::outbox::Backoff {
            interval_secs: 60,
            next_send_at: 1_000,
        });
        let saved = serde_json::to_string(&state).unwrap();
        assert!(saved.contains(r#""next_send_at":1000"#), "{saved}");
        assert_eq!(serde_json::from_str::<State>(&saved).unwrap(), state);
    }
}
