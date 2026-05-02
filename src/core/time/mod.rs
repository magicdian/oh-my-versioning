pub mod ntp;

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
}

pub fn validate_current_date(
    config: &OmvConfig,
    state: &OmvState,
    ntp_source: &dyn TimeSource,
    system_source: &dyn TimeSource,
) -> Result<ValidatedDate, OmvError> {
    let (today, source) = if config.ntp_enabled {
        (ntp_source.today()?, ntp_source.source())
    } else {
        (system_source.today()?, system_source.source())
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
        source,
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
}
