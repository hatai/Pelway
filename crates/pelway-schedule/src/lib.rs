//! ルーティンの定期実行のための cron 解釈（#213、ADR 0010）。
//! cron 式の解釈は `croner` に任せ、自前の解釈実装は置かない。
//! スケジュール実行基盤（キューとの接続）はこのクレートの対象外。

#![forbid(unsafe_code)]

use chrono::{DateTime, TimeZone};
use croner::parser::{CronParser, Seconds};
use std::fmt;

/// 解析済みの cron スケジュール。発火時刻の計算は croner に委譲する。
pub struct Schedule {
    cron: croner::Cron,
}

/// cron 式の解析に失敗した。
#[derive(Debug)]
pub struct ScheduleParseError {
    expression: String,
    reason: String,
}

impl fmt::Display for ScheduleParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid cron expression {:?}: {}",
            self.expression, self.reason
        )
    }
}

impl std::error::Error for ScheduleParseError {}

impl Schedule {
    /// cron 式を解析する。5 フィールド（分単位）と、先頭に秒を置く
    /// 6 フィールドの両方を受け付ける。日付と曜日の併記は POSIX の OR 意味
    /// （どちらかに合えば発火）とする（ADR 0010）。
    pub fn parse(expression: &str) -> Result<Self, ScheduleParseError> {
        let cron = CronParser::builder()
            .seconds(Seconds::Optional)
            .build()
            .parse(expression)
            .map_err(|reason| ScheduleParseError {
                expression: expression.to_owned(),
                reason: reason.to_string(),
            })?;
        Ok(Self { cron })
    }

    /// `from` より後（`from` 自身は含めない）の最初の発火時刻を返す。
    /// タイムゾーンは `from` のもので評価する（製品では Asia/Tokyo を渡す）。
    /// 発火時刻が求まらないときは `None`。
    pub fn next_after<Tz: TimeZone>(&self, from: &DateTime<Tz>) -> Option<DateTime<Tz>> {
        self.cron.find_next_occurrence(from, false).ok()
    }
}
