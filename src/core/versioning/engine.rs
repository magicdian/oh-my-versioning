use crate::core::date::LogicalDate;
use crate::core::schema::{OmvConfig, OmvState};
use crate::core::versioning::{BuildPolicy, VersionOutput};
use crate::errors::{OmvError, TimeError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextVersion {
    pub logical_date: LogicalDate,
    pub build_number: u32,
    pub value: String,
}

pub fn compute_next_version(
    config: &OmvConfig,
    state: &OmvState,
    validated_today: LogicalDate,
) -> Result<NextVersion, OmvError> {
    let stored = LogicalDate::parse_iso(&state.logical_date)?;

    if stored > validated_today {
        return Err(TimeError::FutureStoredDate {
            stored: stored.to_iso_string(),
            validated: validated_today.to_iso_string(),
        }
        .into());
    }

    let build_number = match config.build_policy {
        BuildPolicy::DailyReset => {
            if stored == validated_today {
                state.build_number.saturating_add(1)
            } else {
                1
            }
        }
        BuildPolicy::Continuous => state.build_number.saturating_add(1),
    };

    let value = format_version(validated_today, build_number, config.version_output);

    Ok(NextVersion {
        logical_date: validated_today,
        build_number,
        value,
    })
}

pub fn format_version(date: LogicalDate, build_number: u32, output: VersionOutput) -> String {
    let major = ((date.year % 100) as u32) * 100 + date.month as u32;
    let minor = date.day as u32;

    match output {
        VersionOutput::DateTriplet | VersionOutput::Semver => {
            format!("{major}.{minor}.{build_number}")
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::date::LogicalDate;
    use crate::core::schema::{OmvConfig, OmvState};
    use crate::core::versioning::engine::{compute_next_version, format_version};
    use crate::core::versioning::{BuildPolicy, VersionOutput};
    use crate::errors::OmvError;

    #[test]
    fn daily_reset_increments_when_same_day() {
        let config = OmvConfig {
            build_policy: BuildPolicy::DailyReset,
            ..OmvConfig::default()
        };
        let state = OmvState {
            logical_date: "2026-04-13".to_owned(),
            build_number: 1,
            ..OmvState::default()
        };
        let today = LogicalDate::parse_iso("2026-04-13").expect("today should parse");

        let next = compute_next_version(&config, &state, today).expect("version should compute");
        assert_eq!(next.build_number, 2);
        assert_eq!(next.value, "2604.13.2");
    }

    #[test]
    fn daily_reset_resets_when_day_changes() {
        let config = OmvConfig {
            build_policy: BuildPolicy::DailyReset,
            ..OmvConfig::default()
        };
        let state = OmvState {
            logical_date: "2026-04-12".to_owned(),
            build_number: 7,
            ..OmvState::default()
        };
        let today = LogicalDate::parse_iso("2026-04-13").expect("today should parse");

        let next = compute_next_version(&config, &state, today).expect("version should compute");
        assert_eq!(next.build_number, 1);
        assert_eq!(next.value, "2604.13.1");
    }

    #[test]
    fn continuous_policy_keeps_incrementing_across_days() {
        let config = OmvConfig {
            build_policy: BuildPolicy::Continuous,
            ..OmvConfig::default()
        };
        let state = OmvState {
            logical_date: "2026-04-12".to_owned(),
            build_number: 7,
            ..OmvState::default()
        };
        let today = LogicalDate::parse_iso("2026-04-13").expect("today should parse");

        let next = compute_next_version(&config, &state, today).expect("version should compute");
        assert_eq!(next.build_number, 8);
        assert_eq!(next.value, "2604.13.8");
    }

    #[test]
    fn blocks_future_stored_date_conflict() {
        let config = OmvConfig::default();
        let state = OmvState {
            logical_date: "2026-04-15".to_owned(),
            build_number: 1,
            ..OmvState::default()
        };
        let today = LogicalDate::parse_iso("2026-04-13").expect("today should parse");

        let err = compute_next_version(&config, &state, today).expect_err("future date must fail");
        assert!(matches!(err, OmvError::Time(_)));
    }

    #[test]
    fn formats_date_triplet_version() {
        let date = LogicalDate::parse_iso("2026-04-13").expect("date should parse");
        let value = format_version(date, 1, VersionOutput::DateTriplet);
        assert_eq!(value, "2604.13.1");
    }
}
