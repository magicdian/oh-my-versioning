pub mod capability;
pub mod engine;

use crate::generated::version::OMV_VERSION;

use capability::UnsupportedCapability;

/// First release that understands `release_mode = "pipeline"`.
///
/// Kept as published documentation of the value's introduction. It is *not* used
/// to decide whether a config is readable: a binary older than this constant has
/// no such constant at all, so gating on it would only ever protect binaries that
/// already understand the value. See [`capability`] for the mechanism that does
/// work -- the config declaring `requires_omv` itself.
pub const RELEASE_MODE_PIPELINE_SINCE: &str = "2611.2.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReleaseMode {
    #[default]
    Local,
    Pipeline,
}

impl ReleaseMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Pipeline => "pipeline",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "local" => Some(Self::Local),
            "pipeline" => Some(Self::Pipeline),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BuildPolicy {
    #[default]
    DailyReset,
    Continuous,
}

impl BuildPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DailyReset => "daily-reset",
            Self::Continuous => "continuous",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "daily-reset" => Some(Self::DailyReset),
            "continuous" => Some(Self::Continuous),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VersionOutput {
    #[default]
    DateTriplet,
    Semver,
}

impl VersionOutput {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DateTriplet => "date-triplet",
            Self::Semver => "semver",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "date-triplet" => Some(Self::DateTriplet),
            "semver" => Some(Self::Semver),
            _ => None,
        }
    }
}

/// Outcome of checking a config file against the running binary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateOutcome {
    /// This binary is new enough for the config.
    Supported,
    /// The config declares a minimum release this binary predates.
    UpgradeRequired(UnsupportedCapability),
}

/// Checks the `requires_omv` declaration in a config file.
///
/// `declared` is the raw value of `requires_omv`, if the file carries one. An
/// absent declaration is treated as compatible so every config written before
/// this key existed keeps loading. `field` names the config key whose semantics
/// forced the requirement, purely for the error message.
pub fn gate_declared_requirement(
    field: &'static str,
    value: Option<&str>,
    declared: Option<&str>,
) -> GateOutcome {
    let Some(required) = declared.map(str::trim).filter(|raw| !raw.is_empty()) else {
        return GateOutcome::Supported;
    };

    if capability::requires_upgrade(OMV_VERSION, required) {
        return GateOutcome::UpgradeRequired(capability::unsupported(
            field,
            value,
            required,
            OMV_VERSION,
        ));
    }

    GateOutcome::Supported
}

/// The `requires_omv` value a project should declare for its current config.
///
/// `release_mode = "pipeline"` only means what it says from the release that
/// introduced it; every other setting in `.omv/config.toml` predates this
/// mechanism, so the baseline applies.
pub fn required_omv_for(release_mode: ReleaseMode) -> &'static str {
    match release_mode {
        ReleaseMode::Local => capability::BASELINE_VERSION,
        ReleaseMode::Pipeline => RELEASE_MODE_PIPELINE_SINCE,
    }
}

#[cfg(test)]
mod tests {
    use super::{GateOutcome, ReleaseMode, gate_declared_requirement, required_omv_for};

    #[test]
    fn absent_declaration_keeps_legacy_configs_readable() {
        assert_eq!(
            gate_declared_requirement("release_mode", Some("pipeline"), None),
            GateOutcome::Supported
        );
    }

    #[test]
    fn blank_declaration_counts_as_absent() {
        assert_eq!(
            gate_declared_requirement("release_mode", None, Some("   ")),
            GateOutcome::Supported
        );
    }

    #[test]
    fn requirement_at_or_below_this_binary_passes() {
        // This binary is 2611.2.1; requiring that or anything older is fine.
        assert_eq!(
            gate_declared_requirement("release_mode", None, Some(super::OMV_VERSION)),
            GateOutcome::Supported
        );
        assert_eq!(
            gate_declared_requirement("release_mode", None, Some("2610.27.2")),
            GateOutcome::Supported
        );
    }

    #[test]
    fn requirement_above_this_binary_asks_for_an_upgrade() {
        match gate_declared_requirement("release_mode", Some("pipeline"), Some("9999.1.1")) {
            GateOutcome::UpgradeRequired(detail) => {
                assert_eq!(detail.field, "release_mode");
                assert_eq!(detail.value.as_deref(), Some("pipeline"));
                assert_eq!(detail.required_version, "9999.1.1");
                assert_eq!(detail.current_version, super::OMV_VERSION);
            }
            other => panic!("expected upgrade prompt, got {other:?}"),
        }
    }

    #[test]
    fn malformed_requirement_is_not_treated_as_an_upgrade() {
        assert_eq!(
            gate_declared_requirement("release_mode", None, Some("newest")),
            GateOutcome::Supported
        );
    }

    #[test]
    fn pipeline_projects_declare_the_release_that_added_pipeline() {
        assert_eq!(required_omv_for(ReleaseMode::Pipeline), "2611.2.1");
    }

    #[test]
    fn local_projects_declare_only_the_baseline() {
        assert_eq!(
            required_omv_for(ReleaseMode::Local),
            super::capability::BASELINE_VERSION
        );
    }
}
