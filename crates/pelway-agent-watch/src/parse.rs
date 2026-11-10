//! 時刻・ブランチ名・報告行の読み取り。

/// 報告行の 1 行の上限（文字数）。
const MAX_REPORT_LINE_CHARS: usize = 200;
/// 報告行の行数の上限。
const MAX_REPORT_LINES: usize = 5;

/// `2026-11-10T16:57:09.187Z` / `2026-11-10T16:57:09Z` を UNIX 秒にする。
/// UTC（末尾 `Z`）以外の形式は `None`。
pub fn parse_utc_epoch(s: &str) -> Option<i64> {
    let s = s.strip_suffix('Z')?;
    let (date, time) = s.split_once('T')?;
    let time = time.split_once('.').map_or(time, |(whole, _frac)| whole);
    let mut d = date.split('-');
    let (year, month, day) = (num(d.next()?)?, num(d.next()?)?, num(d.next()?)?);
    let mut t = time.split(':');
    let (hour, minute, second) = (num(t.next()?)?, num(t.next()?)?, num(t.next()?)?);
    if d.next().is_some() || t.next().is_some() {
        return None;
    }
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return None;
    }
    Some(days_from_civil(year, month, day) * 86_400 + hour * 3_600 + minute * 60 + second)
}

fn num(s: &str) -> Option<i64> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// UNIX 秒を `2026-11-10T16:57:09Z` 形式にする。
pub fn format_utc(epoch: i64) -> String {
    let days = epoch.div_euclid(86_400);
    let secs = epoch.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        secs / 3_600,
        secs % 3_600 / 60,
        secs % 60
    )
}

/// 1970-01-01 からの日数を年月日にする（Howard Hinnant の civil_from_days）。
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// 1970-01-01 からの日数（Howard Hinnant の days_from_civil）。
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `agent/<番号>-<slug>` から Issue 番号を取り出す。
pub fn issue_number(branch: &str) -> Option<u64> {
    let rest = branch.strip_prefix("agent/")?;
    let digits: &str = rest.split('-').next()?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// 最後の返答が `キー: 値` の行だけで 5 行以内なら、前後の空白を除いた本文を返す。
/// キーは英小文字・数字・`_`・`-`、1 行は 200 文字まで。
pub fn extract_report(text: &str) -> Option<String> {
    let text = text.trim();
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() || lines.len() > MAX_REPORT_LINES {
        return None;
    }
    let ok = lines.iter().all(|line| {
        if line.chars().count() > MAX_REPORT_LINE_CHARS {
            return false;
        }
        let Some((key, value)) = line.split_once(": ") else {
            return false;
        };
        !key.is_empty()
            && key
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
            && !value.trim().is_empty()
    });
    ok.then(|| text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_with_millis() {
        assert_eq!(
            parse_utc_epoch("2026-11-10T16:57:09.187Z"),
            Some(1_794_329_829)
        );
    }

    #[test]
    fn epoch_without_millis() {
        assert_eq!(parse_utc_epoch("1970-01-01T00:00:00Z"), Some(0));
    }

    #[test]
    fn epoch_rejects_offset() {
        assert_eq!(parse_utc_epoch("2026-11-10T16:57:09+09:00"), None);
    }

    #[test]
    fn format_round_trips() {
        assert_eq!(format_utc(1_794_329_829), "2026-11-10T16:57:09Z");
        assert_eq!(format_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_utc(951_782_400), "2000-02-29T00:00:00Z");
    }

    #[test]
    fn issue_from_agent_branch() {
        assert_eq!(issue_number("agent/80-agent-gh-help"), Some(80));
    }

    #[test]
    fn issue_none_for_other_branch() {
        assert_eq!(issue_number("review/81"), None);
        assert_eq!(issue_number("agent/x-1"), None);
    }

    #[test]
    fn report_four_lines() {
        let text = "issue: #80\npr: https://github.com/hatai/Pelway/pull/81\nverify: pass\npr-size: 66 / 800";
        assert_eq!(extract_report(text), Some(text.to_string()));
    }

    #[test]
    fn report_rejects_prose() {
        assert_eq!(extract_report("レビューを確認します。"), None);
    }

    #[test]
    fn report_rejects_six_lines() {
        let text = ["a: 1"; 6].join("\n");
        assert_eq!(extract_report(&text), None);
    }

    #[test]
    fn report_rejects_long_value() {
        let text = format!("note: {}", "x".repeat(300));
        assert_eq!(extract_report(&text), None);
    }
}
