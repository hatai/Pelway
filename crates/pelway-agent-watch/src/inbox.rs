//! 指揮役の受信箱（`<git common dir>/pelway/conductor-inbox.jsonl`）に書く行と、
//! いまの指揮役を起こすかの判定。ファイルと `t3` の呼び出しは `main.rs` 側（`io.rs`）に置く。

use serde_json::Value;

/// 受信箱のファイル名（共有の git dir の `pelway/` の下）。
pub const INBOX_FILE: &str = "conductor-inbox.jsonl";

/// 指揮役の補助記録のファイル名（同じく `pelway/` の下。書くのは指揮役と conductor-watch）。
pub const PATROL_STATE_FILE: &str = "patrol-state.json";

/// 起こしの文。決まった文だけを送り、知らせの中身（エージェントが書いた `report` など）は含めない。
pub const WAKE_MESSAGE: &str = "受信箱に新着があります。docs/agents/grokbot-runbook.md の「指揮役の定期巡回（スケジュール起床）」に従って巡回を始めてください。";

/// スレッド ID の長さの上限。
const MAX_THREAD_ID_LEN: usize = 256;

/// 受信箱の 1 行。webhook に送る payload と同じ JSON を改行なしで書く。
pub fn inbox_line(payload: &Value) -> String {
    // serde_json の to_string は改行を含まない（文字列中の改行は \n にエスケープされる）。
    // 非 ASCII の文字は、JSON として同じ値の `\uXXXX`（U+FFFF を超えるものはサロゲートの組）に
    // 置き換え、行を ASCII だけにする。追記が文字の途中で切れた断片が残っても UTF-8 として
    // 壊れた行にならず、U+0085・U+2028・U+2029 で行を分ける読み手（Python の str.splitlines など）
    // でも 1 行のまま読める。非 ASCII は JSON の文字列の中にしか現れない（構造の記号は ASCII）
    let mut out = String::new();
    for c in payload.to_string().chars() {
        if c.is_ascii() {
            out.push(c);
        } else {
            let mut units = [0u16; 2];
            for u in c.encode_utf16(&mut units) {
                out.push_str(&format!("\\u{u:04x}"));
            }
        }
    }
    out
}

/// 起こしに要る値。宛先はホスト側の登録ファイル（sandbox から書けない）から、
/// 既読の位置は補助記録（sandbox の中の指揮役が書く）から読む。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PatrolInfo {
    /// いまの指揮役の T3 スレッド ID（ホスト側の登録ファイルの値）。空なら起こさない。
    pub conductor_thread_id: Option<String>,
    /// 指揮役が読んだ受信箱の行数。
    pub inbox_offset: u64,
}

/// ホスト側の登録ファイルの中身から、指揮役のスレッド ID を読む（最初の行。空なら None）。
pub fn parse_conductor_file(text: &str) -> Option<String> {
    text.lines()
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// `patrol-state.json` の中身から、既読の位置（`inbox_offset`）を読む。JSON として読めなければ Err。
/// 補助記録は sandbox の中のエージェントも書けるので、宛先（`conductor_thread_id`）はここから読まない。
pub fn parse_patrol_state(text: &str) -> Result<PatrolInfo, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("patrol-state: {e}"))?;
    let obj = v
        .as_object()
        .ok_or_else(|| "patrol-state: not an object".to_string())?;
    // 項目が無ければ 0（受信箱を一度も読んでいない）。あるのに 0 以上の整数でなければ読めない扱い
    let inbox_offset = match obj.get("inbox_offset") {
        None => 0,
        Some(v) => v.as_u64().ok_or_else(|| {
            "patrol-state: inbox_offset is not a non-negative integer".to_string()
        })?,
    };
    Ok(PatrolInfo {
        conductor_thread_id: None,
        inbox_offset,
    })
}

/// 起こしの宛先に使ってよいスレッド ID か。
/// 値（ホストの登録ファイルの 1 行目、`--register` の引数）は `t3 thread send` の引数になるため、
/// 先頭の `-`（オプションに読まれる）を拒み、使える文字と長さを絞る。
pub fn valid_thread_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_THREAD_ID_LEN
        && !id.starts_with('-')
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b':' | b'_' | b'-' | b'.' | b'%'))
}

/// 起こすかの判定に使う、知らせ 1 件の要約。
/// `Watcher::cycle` が、受信箱に書いた payload（`thread_id`・`event`・`round`・`report`）と、
/// その周の `t3 thread list`/`show` の `createdBy` から作る。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WakeNotice {
    /// 知らせの元のスレッド ID（`thread_id`。知らせの元が指揮役自身かの照合にも使う）。
    pub thread_id: String,
    /// payload の `event`（`finished`・`failed`・`cancelled`・`waiting`・`stalled`）。
    pub event: String,
    /// payload の `round`（スレッドの実行回数）。無い・読めないときは `None`。
    pub round: Option<u32>,
    /// payload の `report` に報告の行があるか。
    pub has_report: bool,
    /// 元のスレッドの立て手（`createdBy` の値）。項目が無い・読めないときは `None`。
    pub created_by: Option<String>,
}

/// 人間が立てたスレッドか。`createdBy` が `agent` 以外の値（`user` など）を持つものを
/// 人間が立てたスレッドと見る（`docs/agents/grokbot-runbook.md` の shared-worktree と同じ見分け）。
/// 項目が無い・読めない（`None`）ときは、起こし漏れを避けるため人間ではない側
/// （従来どおり起こす）に倒す。
pub fn human_created(created_by: Option<&str>) -> bool {
    matches!(created_by, Some(v) if v != "agent")
}

/// その知らせの種類と立て手が、指揮役を起こすに足るか（Issue #727）。
/// 人間が立てたスレッドの知らせは、`failed`・`waiting`・`stalled`、または報告の行のある
/// `finished` のときだけ起こす。それ以外（報告の無い `finished`・`cancelled` など）は起こさず、
/// 受信箱に残して次の定期巡回で読ませる。人間以外（エージェント・立て手不明）の知らせは
/// 従来どおり起こす。同じ承認待ちの繰り返しは `classify_notices` が別に除く。
pub fn wake_worthy(notice: &WakeNotice) -> bool {
    if !human_created(notice.created_by.as_deref()) {
        return true;
    }
    match notice.event.as_str() {
        "failed" | "waiting" | "stalled" => true,
        "finished" => notice.has_report,
        _ => false,
    }
}

/// その周に受信箱へ書いた知らせの束の分類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeBatch {
    /// 知らせが無い（起こし待ちの印だけを確かめる周）。
    Empty,
    /// 起こすに足る新しい知らせがある。
    Worthy,
    /// 知らせの元がすべて、登録された指揮役自身（巡回の終わりなど）。
    OwnOnly,
    /// 指揮役自身のものを除くと、起こすに足るのは、すでに起こしに数えた承認待ちの繰り返しだけ（Issue #839）。
    RepeatWaiting,
    /// 指揮役自身のものを除くと、人間が立てたスレッドの起こさない種類のものだけ（Issue #727）。
    HumanNonUrgent,
}

/// 知らせの束を分類する。`conductor` は登録された指揮役のスレッド ID で、その知らせは数えない
/// （数えると、巡回の終わり → 知らせ → 起こし → 巡回、を繰り返すため）。
///
/// 承認待ち（`waiting`）は、スレッドと round の組ごとに最初の 1 件だけを起こしに数える（Issue #839。
/// 承認待ちの件数が 0 から増えるたびに知らせが出るため、同じ round で何度も来る）。`waiting_rounds` は
/// スレッドごとに、起こしに数えた最後の承認待ちの round で、数えた知らせの分をここで書き足す。
/// round の無い承認待ちは組を作れないので、毎回数える（起こし漏れの側に倒さない）。
pub fn classify_notices(
    notices: &[WakeNotice],
    conductor: &str,
    waiting_rounds: &mut std::collections::BTreeMap<String, u32>,
) -> NoticeBatch {
    if notices.is_empty() {
        return NoticeBatch::Empty;
    }
    let mut any_other = false;
    let mut worthy = false;
    let mut repeat = false;
    for n in notices.iter().filter(|n| n.thread_id != conductor) {
        any_other = true;
        if !wake_worthy(n) {
            continue;
        }
        match (n.event.as_str(), n.round) {
            ("waiting", Some(round)) => {
                if waiting_rounds.get(&n.thread_id) == Some(&round) {
                    repeat = true;
                } else {
                    waiting_rounds.insert(n.thread_id.clone(), round);
                    worthy = true;
                }
            }
            _ => worthy = true,
        }
    }
    match (any_other, worthy, repeat) {
        (false, _, _) => NoticeBatch::OwnOnly,
        (_, true, _) => NoticeBatch::Worthy,
        (_, false, true) => NoticeBatch::RepeatWaiting,
        _ => NoticeBatch::HumanNonUrgent,
    }
}

/// 前回までの起こしの記録（いまの宛先に対するもの。宛先が替わったら呼び出し側が空にして渡す）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WakeHistory {
    /// 前回起こしたときの指揮役の既読の位置（`inbox_offset`）。一度も起こしていなければ None。
    pub last_offset: Option<u64>,
    /// 前回起こした時刻（UNIX 秒）。一度も起こしていなければ None。
    pub last_at: Option<i64>,
    /// 最短間隔の間に起こすに足る知らせが来て、起こしを後に回している（起こし待ちの印）。
    pub pending: bool,
}

/// 起こすかの判定の結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WakeDecision {
    /// このスレッドを起こす。
    Send(String),
    /// 指揮役が登録されていない（Grok Bot が指揮役のときなど）。
    NoConductor,
    /// 登録された値がスレッド ID の形をしていない。
    InvalidThreadId,
    /// 起こすに足る新しい知らせも、起こし待ちの印も無い。
    Nothing,
    /// 前回の起こしから最短間隔がたっていない。起こし待ちの印を付けて後に回す（Issue #839）。
    Cooldown,
    /// 前回の起こしの後に、指揮役の既読の位置が進んでいない。
    NotYetRead,
    /// 指揮役が受信箱を末尾まで読み終えている。起こし待ちの印は取り消す（Issue #839）。
    AlreadyRead,
}

/// 起こすかを決める。`fresh` はその周に起こすに足る新しい知らせがあったか（`classify_notices` が
/// `Worthy`）。`inbox_lines` は受信箱のいまの行数、`now` はいまの時刻（UNIX 秒）、`cooldown_secs` は
/// 起こしの最短間隔（秒）。
///
/// 1. 新しい知らせも起こし待ちの印も無ければ起こさない
/// 2. 前回の起こしから最短間隔がたっていなければ、起こし待ちの印を付けて後に回す。どの知らせも
///    例外にしない（間隔の後に、溜まった分で 1 回起こす）
/// 3. 前回の起こしの後に既読の位置が進んでいなければ、重ねて起こさない（前回の起こしの後に 1 行でも
///    読めば、次の機会に起こす）
/// 4. 既読の位置が受信箱の末尾に追いついていれば、起こさない（定期巡回などで読み終えている）
pub fn decide_wake(
    info: &PatrolInfo,
    inbox_lines: u64,
    history: &WakeHistory,
    fresh: bool,
    now: i64,
    cooldown_secs: i64,
) -> WakeDecision {
    let Some(id) = &info.conductor_thread_id else {
        return WakeDecision::NoConductor;
    };
    if !valid_thread_id(id) {
        return WakeDecision::InvalidThreadId;
    }
    if !fresh && !history.pending {
        return WakeDecision::Nothing;
    }
    // 時計が戻った（前回の起こしが未来にある）ときは、間隔がたったものとして扱う（起こしを止め続けない）
    if history
        .last_at
        .is_some_and(|at| (0..cooldown_secs).contains(&(now - at)))
    {
        return WakeDecision::Cooldown;
    }
    if history
        .last_offset
        .is_some_and(|at| info.inbox_offset <= at)
    {
        return WakeDecision::NotYetRead;
    }
    if info.inbox_offset >= inbox_lines {
        return WakeDecision::AlreadyRead;
    }
    WakeDecision::Send(id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeMap;

    fn info(id: Option<&str>, offset: u64) -> PatrolInfo {
        PatrolInfo {
            conductor_thread_id: id.map(str::to_string),
            inbox_offset: offset,
        }
    }

    fn notice(created_by: Option<&str>, event: &str, has_report: bool) -> WakeNotice {
        notice_on("mcp:t", created_by, event, has_report)
    }

    fn notice_on(
        thread_id: &str,
        created_by: Option<&str>,
        event: &str,
        has_report: bool,
    ) -> WakeNotice {
        WakeNotice {
            thread_id: thread_id.to_string(),
            event: event.to_string(),
            round: None,
            has_report,
            created_by: created_by.map(str::to_string),
        }
    }

    fn waiting_on(thread_id: &str, round: u32) -> WakeNotice {
        WakeNotice {
            round: Some(round),
            ..notice_on(thread_id, Some("agent"), "waiting", false)
        }
    }

    /// 指揮役 `mcp:a` に対する分類（承認待ちの記録は毎回空から）。
    fn classify(notices: &[WakeNotice]) -> NoticeBatch {
        classify_notices(notices, "mcp:a", &mut BTreeMap::new())
    }

    const NOW: i64 = 1_000_000;

    /// 最短間隔 300 秒で、指揮役 `mcp:a`、受信箱 10 行のときの判定。
    fn decide(offset: u64, history: WakeHistory, fresh: bool) -> WakeDecision {
        decide_wake(&info(Some("mcp:a"), offset), 10, &history, fresh, NOW, 300)
    }

    fn woke(offset: u64, ago: i64) -> WakeHistory {
        WakeHistory {
            last_offset: Some(offset),
            last_at: Some(NOW - ago),
            pending: false,
        }
    }

    fn send() -> WakeDecision {
        WakeDecision::Send("mcp:a".to_string())
    }

    #[test]
    fn inbox_line_is_one_line_json() {
        let p = json!({"event":"finished","report":"issue: #1\nstate: done"});
        let line = inbox_line(&p);
        assert!(!line.contains('\n'));
        assert_eq!(serde_json::from_str::<Value>(&line).unwrap(), p);
    }

    #[test]
    fn inbox_line_escapes_other_line_separators() {
        let p = json!({"event":"finished","report":"a\u{0085}b\u{2028}c\u{2029}d"});
        let line = inbox_line(&p);
        assert_eq!(line.lines().count(), 1);
        assert!(
            !line.contains(['\u{0085}', '\u{2028}', '\u{2029}']),
            "{line}"
        );
        assert_eq!(serde_json::from_str::<Value>(&line).unwrap(), p);
    }

    #[test]
    fn inbox_line_is_ascii_only() {
        let p = json!({"event":"finished","report":"state: 実装中\nnote: 😀"});
        let line = inbox_line(&p);
        assert!(line.is_ascii(), "{line}");
        assert!(line.contains("\\u5b9f"), "{line}");
        assert!(line.contains("\\ud83d\\ude00"), "{line}");
        assert_eq!(serde_json::from_str::<Value>(&line).unwrap(), p);
    }

    #[test]
    fn patrol_state_reads_offset_and_ignores_thread_id() {
        let got = parse_patrol_state(
            r#"{"conductor_thread_id":"mcp:abc","inbox_offset":7,"subjects":{}}"#,
        )
        .unwrap();
        assert_eq!(got, info(None, 7));
    }

    #[test]
    fn conductor_file_first_line() {
        assert_eq!(
            parse_conductor_file("mcp:abc\n"),
            Some("mcp:abc".to_string())
        );
        assert_eq!(parse_conductor_file("  \n"), None);
        assert_eq!(parse_conductor_file(""), None);
    }

    #[test]
    fn patrol_state_offset_missing_is_zero_and_odd_is_error() {
        assert_eq!(
            parse_patrol_state(r#"{"subjects":{}}"#)
                .unwrap()
                .inbox_offset,
            0
        );
        for bad in [
            r#"{"inbox_offset":"3"}"#,
            r#"{"inbox_offset":-1}"#,
            r#"{"inbox_offset":1.5}"#,
            r#"{"inbox_offset":null}"#,
        ] {
            assert!(parse_patrol_state(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn patrol_state_broken_is_error() {
        assert!(parse_patrol_state("{\"conductor_thread_id\":").is_err());
        assert!(parse_patrol_state("[]").is_err());
    }

    #[test]
    fn thread_id_rejects_option_and_odd_chars() {
        assert!(valid_thread_id("mcp:fe379cab-84fc-4bdd-841e-aeea82129d66"));
        assert!(valid_thread_id(
            "thread:delegated-task:command%3Amcp%3A1c7f63d3"
        ));
        assert!(!valid_thread_id("--help"));
        assert!(!valid_thread_id("-x"));
        assert!(!valid_thread_id("a b"));
        assert!(!valid_thread_id("a;rm"));
        assert!(!valid_thread_id(""));
        assert!(!valid_thread_id(&"a".repeat(MAX_THREAD_ID_LEN + 1)));
    }

    #[test]
    fn wake_first_time_sends() {
        assert_eq!(decide(0, WakeHistory::default(), true), send());
    }

    #[test]
    fn wake_needs_conductor() {
        assert_eq!(
            decide_wake(&info(None, 0), 10, &WakeHistory::default(), true, NOW, 300),
            WakeDecision::NoConductor
        );
    }

    #[test]
    fn wake_rejects_invalid_id() {
        assert_eq!(
            decide_wake(
                &info(Some("--version"), 0),
                10,
                &WakeHistory::default(),
                true,
                NOW,
                300
            ),
            WakeDecision::InvalidThreadId
        );
    }

    #[test]
    fn wake_waits_until_read() {
        // 前回の起こしのときの既読は 3。まだ進んでいない
        assert_eq!(decide(3, woke(3, 1000), true), WakeDecision::NotYetRead);
        // 前回の起こしの時点の未読のうち 1 件だけ読んだ: 進んだので起こす
        assert_eq!(decide(4, woke(3, 1000), true), send());
    }

    #[test]
    fn nothing_new_and_no_pending_does_not_wake() {
        assert_eq!(decide(4, woke(3, 1000), false), WakeDecision::Nothing);
    }

    #[test]
    fn notice_within_cooldown_is_deferred() {
        // 前回の起こしから 299 秒: 既読が進んでいても後に回す
        assert_eq!(decide(4, woke(3, 299), true), WakeDecision::Cooldown);
        // 300 秒たてば起こす
        assert_eq!(decide(4, woke(3, 300), true), send());
    }

    #[test]
    fn pending_wake_fires_after_cooldown_without_new_notice() {
        let pending = WakeHistory {
            pending: true,
            ..woke(3, 1000)
        };
        assert_eq!(decide(4, pending, false), send());
        // 間隔の内なら、まだ待つ
        let pending = WakeHistory {
            pending: true,
            ..woke(3, 10)
        };
        assert_eq!(decide(4, pending, false), WakeDecision::Cooldown);
    }

    #[test]
    fn pending_wake_is_dropped_when_all_read() {
        let pending = WakeHistory {
            pending: true,
            ..woke(3, 1000)
        };
        // 受信箱 10 行を末尾まで読み終えている
        assert_eq!(decide(10, pending, false), WakeDecision::AlreadyRead);
    }

    #[test]
    fn clock_going_back_does_not_hold_wakes() {
        // 前回の起こしが未来にある（時計が戻った）: 間隔がたったものとして扱う
        assert_eq!(decide(4, woke(3, -5000), true), send());
    }

    #[test]
    fn empty_batch_is_empty() {
        assert_eq!(classify(&[]), NoticeBatch::Empty);
    }

    #[test]
    fn own_notices_only_are_own() {
        // 自分の finished を何件入れても、起こすに足るものにならない
        let own: Vec<WakeNotice> = (0..5)
            .map(|_| notice_on("mcp:a", Some("agent"), "finished", true))
            .collect();
        assert_eq!(classify(&own), NoticeBatch::OwnOnly);
    }

    #[test]
    fn human_finished_without_report_does_not_wake() {
        assert_eq!(
            classify(&[notice(Some("user"), "finished", false)]),
            NoticeBatch::HumanNonUrgent
        );
    }

    #[test]
    fn human_finished_with_report_wakes() {
        assert_eq!(
            classify(&[notice(Some("user"), "finished", true)]),
            NoticeBatch::Worthy
        );
    }

    #[test]
    fn human_urgent_events_wake() {
        for event in ["failed", "waiting", "stalled"] {
            assert_eq!(
                classify(&[notice(Some("user"), event, false)]),
                NoticeBatch::Worthy,
                "{event}"
            );
        }
    }

    #[test]
    fn human_cancelled_does_not_wake() {
        // 起こすのは failed・waiting・stalled・報告のある finished だけ。cancelled は次の定期巡回で読ませる
        assert_eq!(
            classify(&[notice(Some("user"), "cancelled", false)]),
            NoticeBatch::HumanNonUrgent
        );
    }

    #[test]
    fn agent_thread_notices_wake_unchanged() {
        // エージェントが立てたスレッドの知らせの扱いは変えない（報告の無い finished・cancelled でも起こす）
        for event in ["finished", "failed", "cancelled", "waiting", "stalled"] {
            assert_eq!(
                classify(&[notice(Some("agent"), event, false)]),
                NoticeBatch::Worthy,
                "{event}"
            );
        }
    }

    #[test]
    fn unknown_creator_wakes() {
        // 立て手が分からない（createdBy が無い・読めない）ときは、起こし漏れを避けるため従来どおり起こす
        for event in ["finished", "cancelled"] {
            assert_eq!(
                classify(&[notice(None, event, false)]),
                NoticeBatch::Worthy,
                "{event}"
            );
        }
    }

    #[test]
    fn mixed_batch_wakes_for_agent_notice() {
        // 人間の報告の無い finished とエージェントの知らせが同じ回なら、後者で起こす
        let notices = vec![
            notice(Some("user"), "finished", false),
            notice(Some("agent"), "finished", true),
        ];
        assert_eq!(classify(&notices), NoticeBatch::Worthy);
    }

    #[test]
    fn own_notice_does_not_make_quiet_batch_worthy() {
        // 指揮役自身（agent で立てたスレッド）の finished と、人間の報告の無い finished が
        // 同じ回なら、起こすに足るものは無い（自身の知らせは起こす側に数えない）
        let notices = vec![
            notice_on("mcp:a", Some("agent"), "finished", false),
            notice_on("mcp:t", Some("user"), "finished", false),
        ];
        assert_eq!(classify(&notices), NoticeBatch::HumanNonUrgent);
    }

    #[test]
    fn own_notice_plus_agent_notice_wakes() {
        // 指揮役自身の finished と、エージェントが立てたスレッドの finished が同じ回なら、後者で起こす
        let notices = vec![
            notice_on("mcp:a", Some("agent"), "finished", false),
            notice_on("mcp:t", Some("agent"), "finished", true),
        ];
        assert_eq!(classify(&notices), NoticeBatch::Worthy);
    }

    #[test]
    fn waiting_counts_once_per_thread_and_round() {
        let mut rounds = BTreeMap::new();
        let first = [waiting_on("mcp:t", 1)];
        assert_eq!(
            classify_notices(&first, "mcp:a", &mut rounds),
            NoticeBatch::Worthy
        );
        // 同じスレッドの同じ round の承認待ちは、何回来ても数えない（自分の finished と一緒でも）
        for _ in 0..3 {
            let again = [
                notice_on("mcp:a", Some("agent"), "finished", false),
                waiting_on("mcp:t", 1),
            ];
            assert_eq!(
                classify_notices(&again, "mcp:a", &mut rounds),
                NoticeBatch::RepeatWaiting
            );
        }
        // 別のスレッドの承認待ちと、次の round の承認待ちは数える
        assert_eq!(
            classify_notices(&[waiting_on("mcp:u", 1)], "mcp:a", &mut rounds),
            NoticeBatch::Worthy
        );
        assert_eq!(
            classify_notices(&[waiting_on("mcp:t", 2)], "mcp:a", &mut rounds),
            NoticeBatch::Worthy
        );
    }

    #[test]
    fn repeat_waiting_does_not_hide_other_urgent_notices() {
        let mut rounds = BTreeMap::from([("mcp:t".to_string(), 1)]);
        let notices = [
            waiting_on("mcp:t", 1),
            notice_on("mcp:u", Some("agent"), "failed", false),
        ];
        assert_eq!(
            classify_notices(&notices, "mcp:a", &mut rounds),
            NoticeBatch::Worthy
        );
    }

    #[test]
    fn waiting_without_round_always_counts() {
        let mut rounds = BTreeMap::new();
        let w = notice_on("mcp:t", Some("agent"), "waiting", false);
        for _ in 0..2 {
            assert_eq!(
                classify_notices(std::slice::from_ref(&w), "mcp:a", &mut rounds),
                NoticeBatch::Worthy
            );
        }
    }

    #[test]
    fn quiet_notice_is_not_recorded_as_waiting() {
        // 起こすに足らない知らせ（人間の cancelled）は承認待ちの記録に影響しない
        let mut rounds = BTreeMap::new();
        classify_notices(
            &[notice(Some("user"), "cancelled", false)],
            "mcp:a",
            &mut rounds,
        );
        assert!(rounds.is_empty());
    }
}
