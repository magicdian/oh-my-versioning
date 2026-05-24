pub mod ntp;

use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::date::LogicalDate;
use crate::core::schema::{OmvConfig, OmvState};
use crate::errors::{OmvError, TimeError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LastTimeSource {
    #[default]
    Ntp,
    System,
    ManualConfirmed,
}

impl LastTimeSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ntp => "ntp",
            Self::System => "system",
            Self::ManualConfirmed => "manual-confirmed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "ntp" => Some(Self::Ntp),
            "system" => Some(Self::System),
            "manual-confirmed" => Some(Self::ManualConfirmed),
            _ => None,
        }
    }
}

pub trait TimeSource {
    fn source(&self) -> LastTimeSource;
    fn today(&self) -> Result<LogicalDate, OmvError>;
    fn unix_seconds(&self) -> Result<i64, OmvError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidatedDate {
    pub date: LogicalDate,
    pub source: LastTimeSource,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemTimeSource;

impl TimeSource for SystemTimeSource {
    fn source(&self) -> LastTimeSource {
        LastTimeSource::System
    }

    fn today(&self) -> Result<LogicalDate, OmvError> {
        LogicalDate::today_from_system()
    }

    fn unix_seconds(&self) -> Result<i64, OmvError> {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| {
                crate::errors::TimeError::InvalidDateFormat(format!(
                    "system clock before unix epoch: {err}"
                ))
            })?;
        Ok(duration.as_secs() as i64)
    }
}

pub fn parse_timezone_offset_hours(raw: &str) -> Option<i32> {
    let trimmed = raw.trim();
    if !trimmed.starts_with("UTC") {
        return None;
    }
    let rest = &trimmed[3..];
    if rest.is_empty() {
        return Some(0);
    }
    // Accept "UTC+8", "UTC+08", "UTC-5", "UTC-05", "UTC+0", "UTC-0"
    let (sign, digits) = if let Some(d) = rest.strip_prefix('+') {
        (1, d)
    } else if let Some(d) = rest.strip_prefix('-') {
        (-1, d)
    } else {
        return None;
    };

    let value: i32 = digits.parse().ok()?;
    Some(sign * value)
}

/// Returns the current date from system time, applying the configured timezone offset.
pub fn offset_aware_system_today(config: &OmvConfig) -> Result<LogicalDate, OmvError> {
    let offset_hours = parse_timezone_offset_hours(&config.timezone).unwrap_or(0);
    LogicalDate::today_from_system_with_offset(offset_hours)
}

pub fn validate_current_date(
    config: &OmvConfig,
    state: &OmvState,
    ntp_source: &dyn TimeSource,
    system_source: &dyn TimeSource,
) -> Result<ValidatedDate, OmvError> {
    let offset_hours = parse_timezone_offset_hours(&config.timezone).unwrap_or(0);

    let source: &dyn TimeSource = if config.ntp_enabled {
        ntp_source
    } else {
        system_source
    };

    let today = if offset_hours == 0 {
        source.today()?
    } else {
        let unix_seconds = source.unix_seconds()?;
        LogicalDate::from_unix_seconds_with_offset(unix_seconds, offset_hours)
    };

    let stored = LogicalDate::parse_iso(&state.logical_date)?;
    if stored > today {
        return Err(TimeError::FutureStoredDate {
            stored: stored.to_iso_string(),
            validated: today.to_iso_string(),
        }
        .into());
    }

    Ok(ValidatedDate {
        date: today,
        source: source.source(),
    })
}

#[cfg(test)]
mod tests {
    use crate::core::date::LogicalDate;
    use crate::core::schema::{OmvConfig, OmvState};
    use crate::core::time::{LastTimeSource, TimeSource, validate_current_date};
    use crate::errors::{NtpError, OmvError};

    struct FixedTimeSource {
        source: LastTimeSource,
        date: LogicalDate,
    }

    impl TimeSource for FixedTimeSource {
        fn source(&self) -> LastTimeSource {
            self.source
        }

        fn today(&self) -> Result<LogicalDate, OmvError> {
            Ok(self.date)
        }

        fn unix_seconds(&self) -> Result<i64, OmvError> {
            // FixedTimeSource is date-based; return a conservative noon-UTC value
            // for the stored date for offset-aware testing.
            let days = self.date.to_unix_days();
            Ok((days * 86_400) + (12 * 3600))
        }
    }

    struct FailingNtpSource;

    impl TimeSource for FailingNtpSource {
        fn source(&self) -> LastTimeSource {
            LastTimeSource::Ntp
        }

        fn today(&self) -> Result<LogicalDate, OmvError> {
            Err(OmvError::Ntp(NtpError::Unavailable(
                "ntp endpoint unreachable".to_owned(),
            )))
        }

        fn unix_seconds(&self) -> Result<i64, OmvError> {
            Err(OmvError::Ntp(NtpError::Unavailable(
                "ntp endpoint unreachable".to_owned(),
            )))
        }
    }

    #[test]
    fn uses_ntp_by_default() {
        let config = OmvConfig {
            ntp_enabled: true,
            ..OmvConfig::default()
        };
        let state = OmvState {
            logical_date: "2026-04-12".to_owned(),
            ..OmvState::default()
        };
        let ntp = FixedTimeSource {
            source: LastTimeSource::Ntp,
            date: LogicalDate::parse_iso("2026-04-13").expect("date should parse"),
        };
        let system = FixedTimeSource {
            source: LastTimeSource::System,
            date: LogicalDate::parse_iso("2026-04-11").expect("date should parse"),
        };

        let validated =
            validate_current_date(&config, &state, &ntp, &system).expect("validation should pass");
        assert_eq!(validated.source, LastTimeSource::Ntp);
        assert_eq!(validated.date.to_iso_string(), "2026-04-13");
    }

    #[test]
    fn uses_system_time_when_ntp_disabled() {
        let config = OmvConfig {
            ntp_enabled: false,
            ..OmvConfig::default()
        };
        let state = OmvState {
            logical_date: "2026-04-12".to_owned(),
            ..OmvState::default()
        };
        let ntp = FailingNtpSource;
        let system = FixedTimeSource {
            source: LastTimeSource::System,
            date: LogicalDate::parse_iso("2026-04-13").expect("date should parse"),
        };

        let validated =
            validate_current_date(&config, &state, &ntp, &system).expect("validation should pass");
        assert_eq!(validated.source, LastTimeSource::System);
    }

    #[test]
    fn blocks_future_stored_date_conflict() {
        let config = OmvConfig::default();
        let state = OmvState {
            logical_date: "2026-04-15".to_owned(),
            ..OmvState::default()
        };
        let ntp = FixedTimeSource {
            source: LastTimeSource::Ntp,
            date: LogicalDate::parse_iso("2026-04-13").expect("date should parse"),
        };
        let system = FixedTimeSource {
            source: LastTimeSource::System,
            date: LogicalDate::parse_iso("2026-04-13").expect("date should parse"),
        };

        let err = validate_current_date(&config, &state, &ntp, &system)
            .expect_err("future stored date must fail");
        assert!(matches!(err, OmvError::Time(_)));
    }

    #[test]
    fn ntp_failure_propagates_when_ntp_is_enabled() {
        let config = OmvConfig::default();
        let state = OmvState {
            logical_date: "2026-04-13".to_owned(),
            ..OmvState::default()
        };
        let ntp = FailingNtpSource;
        let system = FixedTimeSource {
            source: LastTimeSource::System,
            date: LogicalDate::parse_iso("2026-04-13").expect("date should parse"),
        };

        let err = validate_current_date(&config, &state, &ntp, &system)
            .expect_err("ntp failure should propagate");
        assert!(matches!(err, OmvError::Ntp(_)));
    }

    #[test]
    fn parse_timezone_utc_plus_8() {
        assert_eq!(super::parse_timezone_offset_hours("UTC+8"), Some(8));
        assert_eq!(super::parse_timezone_offset_hours("UTC+08"), Some(8));
    }

    #[test]
    fn parse_timezone_utc_plus_0() {
        assert_eq!(super::parse_timezone_offset_hours("UTC+0"), Some(0));
        assert_eq!(super::parse_timezone_offset_hours("UTC"), Some(0));
        assert_eq!(super::parse_timezone_offset_hours("UTC+00"), Some(0));
    }

    #[test]
    fn parse_timezone_utc_minus_5() {
        assert_eq!(super::parse_timezone_offset_hours("UTC-5"), Some(-5));
        assert_eq!(super::parse_timezone_offset_hours("UTC-05"), Some(-5));
    }

    #[test]
    fn parse_timezone_invalid() {
        assert_eq!(super::parse_timezone_offset_hours("EST"), None);
        assert_eq!(super::parse_timezone_offset_hours(""), None);
        assert_eq!(super::parse_timezone_offset_hours("UTC+"), None);
    }

    #[test]
    fn validate_applies_utc_plus_8_offset() {
        // NTP source returns noon UTC on 2026-05-24 (days since epoch).
        // No offset: date stays 2026-05-24. UTC+8: date stays 2026-05-24
        // (noon + 8h = 20:00, same day).
        let may24_days = LogicalDate::parse_iso("2026-05-24")
            .expect("date should parse")
            .to_unix_days();
        let utc_noon_seconds = may24_days * 86_400 + 12 * 3600;

        let config = OmvConfig {
            ntp_enabled: true,
            timezone: "UTC+8".to_owned(),
            ..OmvConfig::default()
        };
        let state = OmvState {
            logical_date: "2026-05-24".to_owned(),
            ..OmvState::default()
        };

        // Build a time source that returns known unix_seconds and the UTC date
        struct OffsetTestSource {
            unix_seconds_val: i64,
            utc_date: LogicalDate,
        }
        impl TimeSource for OffsetTestSource {
            fn source(&self) -> LastTimeSource {
                LastTimeSource::Ntp
            }
            fn today(&self) -> Result<LogicalDate, OmvError> {
                Ok(self.utc_date)
            }
            fn unix_seconds(&self) -> Result<i64, OmvError> {
                Ok(self.unix_seconds_val)
            }
        }

        let ntp = OffsetTestSource {
            unix_seconds_val: utc_noon_seconds,
            utc_date: LogicalDate::parse_iso("2026-05-24").expect("date should parse"),
        };
        let system = FixedTimeSource {
            source: LastTimeSource::System,
            date: LogicalDate::parse_iso("2026-05-24").expect("date should parse"),
        };

        let validated =
            validate_current_date(&config, &state, &ntp, &system).expect("validation should pass");
        // Noon UTC + 8h = 20:00, same day
        assert_eq!(validated.date.to_iso_string(), "2026-05-24");
    }

    #[test]
    fn validate_applies_utc_plus_8_crosses_midnight() {
        // 2026-05-23 18:28 UTC → with +8 offset → 2026-05-24 02:28
        let may23_days = LogicalDate::parse_iso("2026-05-23")
            .expect("date should parse")
            .to_unix_days();
        let unix_seconds = may23_days * 86_400 + 18 * 3600 + 28 * 60;

        let config = OmvConfig {
            ntp_enabled: true,
            timezone: "UTC+8".to_owned(),
            ..OmvConfig::default()
        };
        // Stored date is May 23 (should be <= offset-adjusted date May 24)
        let state = OmvState {
            logical_date: "2026-05-23".to_owned(),
            ..OmvState::default()
        };

        struct OffsetTestSource2 {
            unix_seconds_val: i64,
        }
        impl TimeSource for OffsetTestSource2 {
            fn source(&self) -> LastTimeSource {
                LastTimeSource::Ntp
            }
            fn today(&self) -> Result<LogicalDate, OmvError> {
                // UTC today (without offset)
                Ok(LogicalDate::from_unix_days(self.unix_seconds_val / 86_400))
            }
            fn unix_seconds(&self) -> Result<i64, OmvError> {
                Ok(self.unix_seconds_val)
            }
        }

        let ntp = OffsetTestSource2 {
            unix_seconds_val: unix_seconds,
        };
        let system = FixedTimeSource {
            source: LastTimeSource::System,
            date: LogicalDate::parse_iso("2026-05-24").expect("date should parse"),
        };

        let validated =
            validate_current_date(&config, &state, &ntp, &system).expect("validation should pass");
        // 18:28 UTC + 8h = 02:28 next day → 2026-05-24
        assert_eq!(validated.date.to_iso_string(), "2026-05-24");
    }

    #[test]
    fn validate_utc_plus_0_unchanged() {
        let may24_days = LogicalDate::parse_iso("2026-05-24")
            .expect("date should parse")
            .to_unix_days();
        let unix_seconds = may24_days * 86_400 + 18 * 3600;

        let config = OmvConfig {
            ntp_enabled: true,
            timezone: "UTC+0".to_owned(),
            ..OmvConfig::default()
        };
        let state = OmvState {
            logical_date: "2026-05-23".to_owned(),
            ..OmvState::default()
        };

        struct UtcZeroTestSource {
            unix_seconds_val: i64,
        }
        impl TimeSource for UtcZeroTestSource {
            fn source(&self) -> LastTimeSource {
                LastTimeSource::Ntp
            }
            fn today(&self) -> Result<LogicalDate, OmvError> {
                Ok(LogicalDate::from_unix_days(self.unix_seconds_val / 86_400))
            }
            fn unix_seconds(&self) -> Result<i64, OmvError> {
                Ok(self.unix_seconds_val)
            }
        }

        let ntp = UtcZeroTestSource {
            unix_seconds_val: unix_seconds,
        };
        let system = FixedTimeSource {
            source: LastTimeSource::System,
            date: LogicalDate::parse_iso("2026-05-24").expect("date should parse"),
        };

        let validated =
            validate_current_date(&config, &state, &ntp, &system).expect("validation should pass");
        // With UTC+0, date stays May 24
        assert_eq!(validated.date.to_iso_string(), "2026-05-24");
    }
}
