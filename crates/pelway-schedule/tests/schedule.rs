//! ルーティンの cron 解釈の境界ケース試験（#213、ADR 0010 のテスト観点）。
//! 前提: Pelway は日本国内利用で、評価タイムゾーンは Asia/Tokyo（DST なし、通年 UTC+9）。

use chrono::TimeZone as _;
use chrono_tz::{Asia::Tokyo, Tz};
use pelway_schedule::Schedule;

fn jst(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
) -> chrono::DateTime<Tz> {
    Tokyo
        .with_ymd_and_hms(year, month, day, hour, minute, second)
        .unwrap()
}

// Asia/Tokyo での解釈: 時刻はローカル時刻（JST）で評価され、UTC 基準にずれない。
#[test]
fn fires_at_local_time_in_asia_tokyo() {
    let schedule = Schedule::parse("0 9 * * *").unwrap();
    assert_eq!(
        schedule.next_after(&jst(2026, 11, 10, 0, 0, 0)),
        Some(jst(2026, 11, 10, 9, 0, 0))
    );
    // from 自身が発火時刻に一致するとき、次は翌日を返す（二重発火しない）。
    assert_eq!(
        schedule.next_after(&jst(2026, 11, 10, 9, 0, 0)),
        Some(jst(2026, 11, 11, 9, 0, 0))
    );
}

// 月末: L は「その月の最終日」。2 月は平年 28 日・うるう年 29 日を正しく返す。
#[test]
fn last_day_of_month_via_l() {
    let schedule = Schedule::parse("0 9 L * *").unwrap();
    assert_eq!(
        schedule.next_after(&jst(2026, 1, 15, 0, 0, 0)),
        Some(jst(2026, 1, 31, 9, 0, 0))
    );
    assert_eq!(
        schedule.next_after(&jst(2027, 2, 1, 0, 0, 0)),
        Some(jst(2027, 2, 28, 9, 0, 0))
    );
    assert_eq!(
        schedule.next_after(&jst(2028, 2, 1, 0, 0, 0)),
        Some(jst(2028, 2, 29, 9, 0, 0))
    );
}

// うるう年: 2/29 指定は平年を飛ばして次のうるう年に進む。
#[test]
fn feb_29_skips_non_leap_years() {
    let schedule = Schedule::parse("0 0 29 2 *").unwrap();
    assert_eq!(
        schedule.next_after(&jst(2027, 3, 1, 0, 0, 0)),
        Some(jst(2028, 2, 29, 0, 0, 0))
    );
}

// 曜日と日付の併記: 既定は POSIX の OR 意味（「13 日または金曜」）。AND ではない。
#[test]
fn dom_and_dow_default_is_posix_or() {
    let schedule = Schedule::parse("0 9 13 * 5").unwrap();
    // 2026-11-16（月）以降: 金曜の 11-20 が次の 13 日（12-13）より先に来る。
    assert_eq!(
        schedule.next_after(&jst(2026, 11, 16, 0, 0, 0)),
        Some(jst(2026, 11, 20, 9, 0, 0))
    );
    // 12-12（土）以降: 12-13 は日曜だが、日付条件だけで発火する（AND なら翌年以降になる）。
    assert_eq!(
        schedule.next_after(&jst(2026, 12, 12, 0, 0, 0)),
        Some(jst(2026, 12, 13, 9, 0, 0))
    );
}

// 秒を含む 6 フィールド記法も受け付ける（秒は任意フィールド）。
#[test]
fn six_field_pattern_with_seconds_is_accepted() {
    let schedule = Schedule::parse("30 0 9 * * *").unwrap();
    assert_eq!(
        schedule.next_after(&jst(2026, 11, 10, 0, 0, 0)),
        Some(jst(2026, 11, 10, 9, 0, 30))
    );
}

// 解釈不能な式は panic ではなく Err で返す。
#[test]
fn invalid_pattern_is_rejected() {
    for bad in ["", "not a cron", "0 9 32 * *", "* * * *"] {
        assert!(Schedule::parse(bad).is_err(), "{bad:?}");
    }
}
