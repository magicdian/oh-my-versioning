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

    pub fn today_from_system() -> Result<Self, OmvError> {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| {
                TimeError::InvalidDateFormat(format!("system clock before unix epoch: {err}"))
            })?;
        let days_since_epoch = (duration.as_secs() / 86_400) as i64;
        Ok(Self::from_unix_days(days_since_epoch))
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
}
