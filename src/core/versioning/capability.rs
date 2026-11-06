//! Release-version plumbing for configuration compatibility.
//!
//! `.omv/config.toml` is plain text read by whatever `omv` binary happens to be
//! on the machine. A capability table compiled into the binary cannot protect
//! against an *older* binary: an old release has no knowledge of the newer value
//! and silently falls back to the enum default. That is exactly how a
//! `release_mode = "pipeline"` project accepted local version writes under
//! `omv@2610.27.2`.
//!
//! The only direction that works is the config telling the binary what it needs:
//! a `requires_omv` key states the minimum release that can honour the file, and
//! every binary -- old or new -- can compare that against its own version.

use std::cmp::Ordering;

/// Sentinel meaning "no upgrade required"; sorts below every real release.
pub const BASELINE_VERSION: &str = "0.0.1";

/// A configuration file that demands a newer `omv` than the running binary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedCapability {
    /// Config key whose semantics need the newer release, e.g. `release_mode`.
    pub field: &'static str,
    /// The value written in the config file, when the key was specific enough
    /// to name one.
    pub value: Option<String>,
    /// Version of the binary doing the read.
    pub current_version: String,
    /// Minimum release the config declares it needs.
    pub required_version: String,
}

impl std::fmt::Display for UnsupportedCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.value {
            Some(value) => write!(
                f,
                ".omv/config.toml declares {} = \"{}\", which needs omv >= {}, but this binary is {}; upgrade omv (npm install -g @magicdian/omv@latest) to read this project",
                self.field, value, self.required_version, self.current_version
            ),
            None => write!(
                f,
                ".omv/config.toml requires omv >= {}, but this binary is {}; upgrade omv (npm install -g @magicdian/omv@latest) to read this project",
                self.required_version, self.current_version
            ),
        }
    }
}

/// Compares two date-style release strings such as `2611.2.1` and `2610.27.2`.
///
/// OMV releases are not semver: the leading segments are date-derived and the
/// trailing segment is a build counter under a daily-reset policy. They compare
/// correctly as zero-padded integer tuples, which is all the gate needs.
/// Returns `None` when either side has a non-numeric segment so a malformed
/// value surfaces as a configuration error instead of a silent ordering.
pub fn compare_versions(left: &str, right: &str) -> Option<Ordering> {
    let left = parse_version_segments(left)?;
    let right = parse_version_segments(right)?;

    let width = left.len().max(right.len());
    for index in 0..width {
        let a = left.get(index).copied().unwrap_or(0);
        let b = right.get(index).copied().unwrap_or(0);
        match a.cmp(&b) {
            Ordering::Equal => continue,
            other => return Some(other),
        }
    }
    Some(Ordering::Equal)
}

fn parse_version_segments(value: &str) -> Option<Vec<u64>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed
        .split('.')
        .map(|segment| segment.parse::<u64>().ok())
        .collect()
}

/// True when `current_version` is older than `required_version`.
///
/// An unparsable version on either side returns `false`: a malformed value is a
/// config/schema error the caller reports separately, not an upgrade prompt.
pub fn requires_upgrade(current_version: &str, required_version: &str) -> bool {
    matches!(
        compare_versions(current_version, required_version),
        Some(Ordering::Less)
    )
}

/// Builds the error payload for a config that outranks the running binary.
pub fn unsupported(
    field: &'static str,
    value: Option<&str>,
    required_version: &str,
    current_version: &str,
) -> UnsupportedCapability {
    UnsupportedCapability {
        field,
        value: value.map(|value| value.trim().to_owned()),
        current_version: current_version.to_owned(),
        required_version: required_version.trim().to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::{UnsupportedCapability, compare_versions, requires_upgrade, unsupported};

    #[test]
    fn compares_date_style_releases_numerically() {
        assert_eq!(
            compare_versions("2611.2.1", "2610.27.2"),
            Some(Ordering::Greater)
        );
        assert_eq!(
            compare_versions("2610.27.2", "2611.2.1"),
            Some(Ordering::Less)
        );
        assert_eq!(
            compare_versions("2611.2.1", "2611.2.1"),
            Some(Ordering::Equal)
        );
        // Numeric, not lexicographic: "6" < "10".
        assert_eq!(
            compare_versions("2611.5.6", "2611.5.10"),
            Some(Ordering::Less)
        );
    }

    #[test]
    fn treats_missing_segments_as_zero() {
        assert_eq!(compare_versions("2611.2", "2611.2.0"), Some(Ordering::Equal));
    }

    #[test]
    fn rejects_non_numeric_versions() {
        assert_eq!(compare_versions("2611.2.beta", "2611.2.1"), None);
        assert_eq!(compare_versions("", "2611.2.1"), None);
    }

    #[test]
    fn older_binary_requires_upgrade() {
        assert!(requires_upgrade("2610.27.2", "2611.2.1"));
        assert!(!requires_upgrade("2611.2.1", "2611.2.1"));
        assert!(!requires_upgrade("2612.1.1", "2611.2.1"));
    }

    #[test]
    fn unparsable_versions_do_not_claim_an_upgrade() {
        assert!(!requires_upgrade("unknown", "2611.2.1"));
        assert!(!requires_upgrade("2611.2.1", "unknown"));
    }

    #[test]
    fn error_message_names_both_versions() {
        let error: UnsupportedCapability =
            unsupported("release_mode", Some("pipeline"), "2611.2.1", "2610.27.2");

        let message = error.to_string();
        assert!(message.contains("release_mode"));
        assert!(message.contains("pipeline"));
        assert!(message.contains("2611.2.1"));
        assert!(message.contains("2610.27.2"));
        assert!(message.contains("upgrade omv"));
    }
}
