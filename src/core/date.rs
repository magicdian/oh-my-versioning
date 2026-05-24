use std::time::{SystemTime, UNIX_EPOCH};

use crate::errors::{OmvError, TimeError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LogicalDate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

impl LogicalDate {
    pub fn parse_iso(value: &str) -> Result<Self, OmvError> {
        let mut parts = value.trim().split('-');
        let year = parts
            .next()
            .ok_or_else(|| TimeError::InvalidDateFormat(value.to_owned()))?
            .parse::<u16>()
            .map_err(|_| TimeError::InvalidDateFormat(value.to_owned()))?;
        let month = parts
            .next()
            .ok_or_else(|| TimeError::InvalidDateFormat(value.to_owned()))?
            .parse::<u8>()
            .map_err(|_| TimeError::InvalidDateFormat(value.to_owned()))?;
        let day = parts
            .next()
            .ok_or_else(|| TimeError::InvalidDateFormat(value.to_owned()))?
            .parse::<u8>()
            .map_err(|_| TimeError::InvalidDateFormat(value.to_owned()))?;

        if parts.next().is_some() {
            return Err(TimeError::InvalidDateFormat(value.to_owned()).into());
        }

        Self::from_ymd(year, month, day)
            .ok_or_else(|| TimeError::InvalidDateFormat(value.to_owned()).into())
    }

    pub fn from_ymd(year: u16, month: u8, day: u8) -> Option<Self> {
        if !(1..=12).contains(&month) {
            return None;
        }

        let max_day = Self::days_in_month(year, month);
        if day == 0 || day > max_day {
            return None;
        }

        Some(Self { year, month, day })
    }

    pub fn to_iso_string(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    /// Reverse of Howard Hinnant's civil date algorithm: return days since Unix epoch.
    pub fn to_unix_days(self) -> i64 {
        let year = self.year as i64;
        let month = self.month as i64;
        let day = self.day as i64;

        let y = year - if month <= 2 { 1 } else { 0 };
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400;
        let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    pub fn from_unix_seconds_with_offset(unix_seconds: i64, offset_hours: i32) -> Self {
        let adjusted_seconds = unix_seconds + (offset_hours as i64) * 3600;
        let days_since_epoch = adjusted_seconds / 86_400;
        Self::from_unix_days(days_since_epoch)
    }

    pub fn today_from_system() -> Result<Self, OmvError> {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| {
                TimeError::InvalidDateFormat(format!("system clock before unix epoch: {err}"))
            })?;
        let days_since_epoch = (duration.as_secs() / 86_400) as i64;
        Ok(Self::from_unix_days(days_since_epoch))
    }

    pub fn today_from_system_with_offset(offset_hours: i32) -> Result<Self, OmvError> {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| {
                TimeError::InvalidDateFormat(format!("system clock before unix epoch: {err}"))
            })?;
        let unix_seconds = duration.as_secs() as i64;
        Ok(Self::from_unix_seconds_with_offset(
            unix_seconds,
            offset_hours,
        ))
    }

    pub fn from_unix_days(days_since_epoch: i64) -> Self {
        // Howard Hinnant's civil date algorithm.
        let z = days_since_epoch + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = mp + if mp < 10 { 3 } else { -9 };
        let year = y + if m <= 2 { 1 } else { 0 };

        Self {
            year: year as u16,
            month: m as u8,
            day: d as u8,
        }
    }

    fn days_in_month(year: u16, month: u8) -> u8 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if Self::is_leap_year(year) {
                    29
                } else {
                    28
                }
            }
            _ => 0,
        }
    }

    fn is_leap_year(year: u16) -> bool {
        (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
    }
}

#[cfg(test)]
mod tests {
    use super::LogicalDate;

    #[test]
    fn parse_iso_accepts_valid_date() {
        let date = LogicalDate::parse_iso("2026-04-13").expect("date should parse");
        assert_eq!(date.year, 2026);
        assert_eq!(date.month, 4);
        assert_eq!(date.day, 13);
    }

    #[test]
    fn parse_iso_rejects_invalid_date() {
        assert!(LogicalDate::parse_iso("2026-02-30").is_err());
        assert!(LogicalDate::parse_iso("bad-date").is_err());
    }

    #[test]
    fn from_unix_days_matches_epoch_start() {
        let date = LogicalDate::from_unix_days(0);
        assert_eq!(date.to_iso_string(), "1970-01-01");
    }

    #[test]
    fn to_unix_days_roundtrips() {
        let date = LogicalDate::parse_iso("2026-05-24").expect("date should parse");
        let days = date.to_unix_days();
        let roundtripped = LogicalDate::from_unix_days(days);
        assert_eq!(roundtripped.to_iso_string(), "2026-05-24");
    }

    #[test]
    fn to_unix_days_matches_epoch() {
        let epoch = LogicalDate::from_unix_days(0);
        assert_eq!(epoch.to_unix_days(), 0);
    }

    #[test]
    fn from_unix_seconds_with_offset_utc_plus_8_late_evening() {
        // 2026-05-23 18:28 UTC = some unix seconds
        // We need the unix days for 2026-05-23 then add 18h28m in seconds
        let may23_days = LogicalDate::parse_iso("2026-05-23")
            .expect("date should parse")
            .to_unix_days();
        let unix_seconds = may23_days * 86_400 + 18 * 3600 + 28 * 60;
        // With UTC+8, 18:28 UTC + 8h = 02:28 next day → 2026-05-24
        let date = LogicalDate::from_unix_seconds_with_offset(unix_seconds, 8);
        assert_eq!(date.to_iso_string(), "2026-05-24");
    }

    #[test]
    fn from_unix_seconds_with_offset_utc_plus_8_early_morning() {
        // 2026-05-24 02:00 UTC (still May 23 in UTC-5, but we're testing +8)
        let may24_days = LogicalDate::parse_iso("2026-05-24")
            .expect("date should parse")
            .to_unix_days();
        let unix_seconds = may24_days * 86_400 + 2 * 3600;
        // With UTC+8, 02:00 UTC + 8h = 10:00 same day → 2026-05-24
        let date = LogicalDate::from_unix_seconds_with_offset(unix_seconds, 8);
        assert_eq!(date.to_iso_string(), "2026-05-24");
    }

    #[test]
    fn from_unix_seconds_with_offset_zero_is_unchanged() {
        let may24_days = LogicalDate::parse_iso("2026-05-24")
            .expect("date should parse")
            .to_unix_days();
        let unix_seconds = may24_days * 86_400 + 18 * 3600;
        let date = LogicalDate::from_unix_seconds_with_offset(unix_seconds, 0);
        assert_eq!(date.to_iso_string(), "2026-05-24");
    }

    #[test]
    fn from_unix_seconds_with_offset_utc_minus_5() {
        // 2026-05-24 03:00 UTC → with UTC-5 → 2026-05-23 22:00
        let may24_days = LogicalDate::parse_iso("2026-05-24")
            .expect("date should parse")
            .to_unix_days();
        let unix_seconds = may24_days * 86_400 + 3 * 3600;
        let date = LogicalDate::from_unix_seconds_with_offset(unix_seconds, -5);
        assert_eq!(date.to_iso_string(), "2026-05-23");
    }
}
