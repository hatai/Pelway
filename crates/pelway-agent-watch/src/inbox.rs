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
    // serde_json の to_string は改行を含まない（文字列中の改行は \n にエスケープされる）
    payload.to_string()
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
    let inbox_offset = obj.get("inbox_offset").and_then(Value::as_u64).unwrap_or(0);
    Ok(PatrolInfo {
        conductor_thread_id: None,
        inbox_offset,
    })
}

/// 起こしの宛先に使ってよいスレッド ID か。
/// 補助記録は sandbox の中の指揮役が書けるファイルで、値はホスト側のコマンドの引数になるため、
/// 先頭の `-`（オプションに読まれる）を拒み、使える文字と長さを絞る。
pub fn valid_thread_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_THREAD_ID_LEN
        && !id.starts_with('-')
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b':' | b'_' | b'-' | b'.' | b'%'))
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
    /// 前回の起こしの後に、指揮役がまだ受信箱を読んでいない。
    NotYetRead,
}

/// 起こすかを決める。`last_wake_lines` は前回起こしたときの受信箱の行数（一度も起こしていなければ None）。
/// 指揮役の既読（`inbox_offset`）が前回の起こしの時点まで進んでいなければ、重ねて起こさない。
pub fn decide_wake(info: &PatrolInfo, last_wake_lines: Option<u64>) -> WakeDecision {
    let Some(id) = &info.conductor_thread_id else {
        return WakeDecision::NoConductor;
    };
    if !valid_thread_id(id) {
        return WakeDecision::InvalidThreadId;
    }
    if last_wake_lines.is_some_and(|lines| info.inbox_offset < lines) {
        return WakeDecision::NotYetRead;
    }
    WakeDecision::Send(id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn info(id: Option<&str>, offset: u64) -> PatrolInfo {
        PatrolInfo {
            conductor_thread_id: id.map(str::to_string),
            inbox_offset: offset,
        }
    }

    #[test]
    fn inbox_line_is_one_line_json() {
        let p = json!({"event":"finished","report":"issue: #1\nstate: done"});
        let line = inbox_line(&p);
        assert!(!line.contains('\n'));
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
        assert_eq!(
            decide_wake(&info(Some("mcp:a"), 0), None),
            WakeDecision::Send("mcp:a".to_string())
        );
    }

    #[test]
    fn wake_needs_conductor() {
        assert_eq!(decide_wake(&info(None, 0), None), WakeDecision::NoConductor);
    }

    #[test]
    fn wake_rejects_invalid_id() {
        assert_eq!(
            decide_wake(&info(Some("--version"), 0), None),
            WakeDecision::InvalidThreadId
        );
    }

    #[test]
    fn wake_waits_until_read() {
        assert_eq!(
            decide_wake(&info(Some("mcp:a"), 3), Some(5)),
            WakeDecision::NotYetRead
        );
        assert_eq!(
            decide_wake(&info(Some("mcp:a"), 5), Some(5)),
            WakeDecision::Send("mcp:a".to_string())
        );
    }
}
